#!/data/data/com.termux/files/usr/bin/bash
# ============================================================
# fix-libsignal.sh  –  Fix "libgcc_s.so.1 not found" error
#                      when running signal-cli on Termux/Android
# ============================================================
#
# Root cause:
#   signal-cli extracts libsignal_jni.so from its JAR into a temp
#   directory and loads it via Android's bionic linker.  The bionic
#   default namespace does NOT include Termux's prefix, so any
#   dependency on libgcc_s.so.1 (a GCC runtime, absent on Android)
#   causes an immediate dlopen failure.
#
#   Symlinking libgcc_s.so.1 into $PREFIX/lib does NOT help because
#   the file is loaded from a tmpfs path in the "default" namespace.
#   The only reliable fix is to strip that ELF dependency from the
#   .so itself before it is re-packaged into the JAR.
#
# Usage:
#   bash fix-libsignal.sh
#
# After running, retry:
#   signal-cli link -n "clide-bot"
# ============================================================

set -euo pipefail

SIGNAL_VERSION="${SIGNAL_VERSION:-0.12.8}"
SIGNAL_DEST="$HOME/.local/signal-cli-${SIGNAL_VERSION}"
SIGNAL_LIB_DIR="$SIGNAL_DEST/lib"

step() { echo ""; echo "── $* ─────────────────────────────────────────"; }
ok()   { echo "✅ $*"; }
warn() { echo "⚠️  $*"; }
die()  { echo "❌ $*" >&2; exit 1; }

echo ""
echo "════════════════════════════════════════════════"
echo "  fix-libsignal  –  Termux / Android ARM64 fix  "
echo "════════════════════════════════════════════════"

# ── Sanity checks ────────────────────────────────────────────

[[ "$PREFIX" =~ "com.termux" ]] || \
    die "This script is for Termux on Android only."

[ -d "$SIGNAL_DEST" ] || \
    die "signal-cli not found at $SIGNAL_DEST\nRun the Clide installer first: bash install.sh"

LIBSIGNAL_JAR=$(ls "$SIGNAL_LIB_DIR"/libsignal-client-*.jar 2>/dev/null | head -n1 || true)
[ -n "$LIBSIGNAL_JAR" ] || \
    die "libsignal JAR not found in $SIGNAL_LIB_DIR"

LIBSIGNAL_VER=$(basename "$LIBSIGNAL_JAR" \
    | sed 's/libsignal-client-//' | sed 's/\.jar//')

echo ""
echo "  signal-cli   : $SIGNAL_VERSION"
echo "  libsignal    : $LIBSIGNAL_VER"
echo "  JAR path     : $LIBSIGNAL_JAR"

# ── Work directory ───────────────────────────────────────────

WORK_DIR="$TMPDIR/libsignal_fix_$$"
mkdir -p "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

# ── Install helpers ──────────────────────────────────────────

step "Installing helper packages"
pkg install -y patchelf termux-elf-cleaner zip unzip 2>/dev/null \
    | grep -E "^(Unpacking|Setting up|already)" | sed 's/^/   /' || true
ok "Done"

# ── Obtain ARM64 libsignal_jni.so ────────────────────────────
# Strategy 1: download pre-built ARM64 .so from exquo/signal-libs-build
# Strategy 2: extract whatever .so is already in the JAR (then strip it)

step "Obtaining libsignal_jni.so"

# Find where the .so lives inside the JAR (may be at root or in a subdir
# like linux/aarch64/libsignal_jni.so depending on libsignal version).
SO_IN_JAR=$(unzip -l "$LIBSIGNAL_JAR" 2>/dev/null \
    | awk '{print $NF}' | grep 'libsignal_jni\.so$' | head -n1 || true)
echo "   .so path inside JAR: ${SO_IN_JAR:-<not found yet>}"

SO_FILE=""

ARM64_ARCHIVE="libsignal_jni.so-v${LIBSIGNAL_VER}-aarch64-unknown-linux-gnu.tar.gz"
ARM64_URL="https://github.com/exquo/signal-libs-build/releases/download/libsignal_v${LIBSIGNAL_VER}/${ARM64_ARCHIVE}"

echo "   Trying exquo ARM64 build for libsignal v${LIBSIGNAL_VER}..."
if wget -q --timeout=30 "$ARM64_URL" -O "$WORK_DIR/libsignal_arm64.tar.gz" 2>/dev/null; then
    tar xf "$WORK_DIR/libsignal_arm64.tar.gz" -C "$WORK_DIR" 2>/dev/null || true
    SO_FILE=$(find "$WORK_DIR" -name "libsignal_jni.so" | head -n1 || true)
    if [ -n "$SO_FILE" ]; then
        ok "Downloaded ARM64 lib from exquo"
    else
        warn "Archive downloaded but libsignal_jni.so not found inside it"
    fi
else
    warn "ARM64 build not available at exquo for v${LIBSIGNAL_VER}"
fi

# Fall back: extract whatever .so is already packed in the JAR.
# This works because signal-cli 0.12.x JARs already contain an ARM64 .so
# on Android; the only problem is the libgcc_s.so.1 ELF dependency.
if [ -z "$SO_FILE" ]; then
    echo "   Extracting existing .so from the JAR (will strip GCC dep next)..."
    if [ -n "$SO_IN_JAR" ]; then
        # Preserve the directory structure so we can re-pack at the same path
        unzip -o "$LIBSIGNAL_JAR" "$SO_IN_JAR" -d "$WORK_DIR" 2>/dev/null || true
    else
        # Try common locations
        for candidate in \
            "libsignal_jni.so" \
            "linux/libsignal_jni.so" \
            "linux/aarch64/libsignal_jni.so"; do
            unzip -o "$LIBSIGNAL_JAR" "$candidate" -d "$WORK_DIR" 2>/dev/null && break || true
        done
    fi
    SO_FILE=$(find "$WORK_DIR" -name "libsignal_jni.so" | head -n1 || true)
    if [ -n "$SO_FILE" ]; then
        ok "Extracted from JAR"
    else
        die "libsignal_jni.so not found inside $LIBSIGNAL_JAR — cannot proceed"
    fi
fi

# ── Show ELF info ─────────────────────────────────────────────

if command -v file >/dev/null 2>&1; then
    echo "   $(file "$SO_FILE")"
fi

# ── Strip libgcc_s.so.1 dependency ───────────────────────────

step "Stripping libgcc_s.so.1 ELF dependency"
STRIPPED=false

# Run patchelf first — it targets the exact dependency (most precise).
if command -v patchelf >/dev/null 2>&1; then
    echo "   Using patchelf --remove-needed libgcc_s.so.1..."
    if patchelf --remove-needed libgcc_s.so.1 "$SO_FILE" 2>/dev/null; then
        ok "libgcc_s.so.1 removed via patchelf"
        STRIPPED=true
    else
        warn "patchelf: dep not found or already absent (continuing)"
        STRIPPED=true  # not an error if the dep was already gone
    fi
fi

# Also run termux-elf-cleaner — it strips ALL non-Android ELF deps.
if command -v termux-elf-cleaner >/dev/null 2>&1; then
    echo "   Using termux-elf-cleaner..."
    termux-elf-cleaner "$SO_FILE" 2>/dev/null || true
    ok "ELF cleaned via termux-elf-cleaner"
    STRIPPED=true
fi

$STRIPPED || warn "No stripping tools ran — libgcc dep may still be present."

# ── Re-inject .so into the JAR at its original path ──────────

step "Patching libsignal JAR"

# Determine the in-JAR path we should use for re-insertion.
# If SO_IN_JAR is known, use it; otherwise default to root.
if [ -z "$SO_IN_JAR" ]; then
    SO_IN_JAR="libsignal_jni.so"
fi

# Remove the old (broken) entry
zip -d "$LIBSIGNAL_JAR" "$SO_IN_JAR" 2>/dev/null || true

# Re-add from the work dir, preserving the relative path inside the JAR.
# We cd to WORK_DIR so that `zip -u JAR $SO_IN_JAR` stores it at the
# correct relative path (e.g. linux/aarch64/libsignal_jni.so if needed).
#
# If the exquo strategy was used, SO_FILE is a flat file — move it to
# the expected subpath inside WORK_DIR first.
REL_SO="$SO_IN_JAR"
if [ ! -f "$WORK_DIR/$REL_SO" ]; then
    # exquo download: .so is at WORK_DIR root; copy to the right subpath
    mkdir -p "$WORK_DIR/$(dirname "$REL_SO")"
    cp "$SO_FILE" "$WORK_DIR/$REL_SO"
fi

( cd "$WORK_DIR" && zip -u "$LIBSIGNAL_JAR" "$REL_SO" 2>/dev/null ) && \
    ok "Patched JAR: $LIBSIGNAL_JAR" || \
    die "zip failed — could not update the JAR"

# ── LD_PRELOAD safety net ─────────────────────────────────────
# Compile a stub libgcc_s.so.1 and wrap signal-cli to preload it.
# This is belt-and-suspenders: even if JAR patching is incomplete,
# the bionic linker will find the stub in the preloaded library set.

step "LD_PRELOAD safety net (stub + signal-cli wrapper)"

STUB_LIB="$PREFIX/lib/libgcc_s.so.1"
SIGNAL_BIN="$SIGNAL_DEST/bin/signal-cli"
SIGNAL_BIN_REAL="$SIGNAL_DEST/bin/signal-cli.real"

if [ ! -f "$STUB_LIB" ]; then
    if command -v clang >/dev/null 2>&1; then
        printf 'void __libgcc_stub(void){}\n' \
            | clang -shared -fPIC -Wl,-soname,libgcc_s.so.1 \
                    -o "$STUB_LIB" -x c - 2>/dev/null \
            && ok "libgcc_s.so.1 stub compiled" \
            || warn "clang stub compilation failed"
    fi
    if [ ! -f "$STUB_LIB" ]; then
        for _src in "$PREFIX/lib/libgcc.so" "$PREFIX/lib/libgcc_s.so" \
                    /system/lib64/libgcc.so /system/lib/libgcc.so; do
            [ -f "$_src" ] && ln -sf "$_src" "$STUB_LIB" \
                && ok "libgcc_s.so.1 → $(basename "$_src")" && break || true
        done
    fi
fi

if [ -f "$STUB_LIB" ] && [ -f "$SIGNAL_BIN" ] && [ ! -f "$SIGNAL_BIN_REAL" ]; then
    mv "$SIGNAL_BIN" "$SIGNAL_BIN_REAL"
    printf '#!/bin/sh\n# Termux: preload libgcc_s stub so bionic resolves it for libsignal_jni.so\nexport LD_PRELOAD="%s${LD_PRELOAD:+:$LD_PRELOAD}"\nexec "%s" "$@"\n' \
        "$STUB_LIB" "$SIGNAL_BIN_REAL" > "$SIGNAL_BIN"
    chmod +x "$SIGNAL_BIN"
    ok "signal-cli wrapped with LD_PRELOAD=$STUB_LIB"
elif [ -f "$SIGNAL_BIN_REAL" ]; then
    ok "signal-cli LD_PRELOAD wrapper already in place"
elif [ ! -f "$STUB_LIB" ]; then
    warn "Could not create stub — install clang with: pkg install clang"
fi

# ── Summary ───────────────────────────────────────────────────

echo ""
echo "════════════════════════════════════════════════"
echo "  Fix complete!"
echo "════════════════════════════════════════════════"
echo ""
echo "  Next steps:"
echo "    source ~/.bashrc"
echo "    signal-cli link -n \"clide-bot\"   # scan QR with Signal app"
echo "    clide bot                         # start the bot"
echo ""
