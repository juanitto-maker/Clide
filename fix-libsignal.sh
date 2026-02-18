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
pkg install -y termux-elf-cleaner zip unzip 2>/dev/null \
    | grep -E "^(Unpacking|Setting up|already)" | sed 's/^/   /' || true
ok "Done"

# ── Obtain ARM64 libsignal_jni.so ────────────────────────────
# Strategy 1: download pre-built ARM64 .so from exquo/signal-libs-build
# Strategy 2: extract whatever .so is already in the JAR (then strip it)

step "Obtaining libsignal_jni.so"

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

# Fall back: extract whatever .so is already packed in the JAR
if [ -z "$SO_FILE" ]; then
    echo "   Extracting existing .so from the JAR (will strip GCC dep next)..."
    cd "$WORK_DIR"
    if unzip -o "$LIBSIGNAL_JAR" "libsignal_jni.so" 2>/dev/null; then
        SO_FILE="$WORK_DIR/libsignal_jni.so"
        ok "Extracted from JAR"
    else
        die "libsignal_jni.so not found inside $LIBSIGNAL_JAR"
    fi
fi

# ── Show ELF info ─────────────────────────────────────────────

if command -v file >/dev/null 2>&1; then
    echo "   $(file "$SO_FILE")"
fi

# ── Strip libgcc_s.so.1 dependency ───────────────────────────

step "Stripping libgcc_s.so.1 ELF dependency"
cd "$(dirname "$SO_FILE")"
SO_BASENAME="$(basename "$SO_FILE")"
STRIPPED=false

# Preferred tool: termux-elf-cleaner (removes ALL non-Android ELF deps)
if command -v termux-elf-cleaner >/dev/null 2>&1; then
    echo "   Using termux-elf-cleaner..."
    if termux-elf-cleaner "$SO_BASENAME" 2>&1; then
        STRIPPED=true
        ok "GCC dependency removed via termux-elf-cleaner"
    else
        warn "termux-elf-cleaner returned an error (may still have patched)"
        STRIPPED=true   # it often exits non-zero but still patches successfully
    fi
fi

# Alternative: patchelf --remove-needed
if ! $STRIPPED && command -v patchelf >/dev/null 2>&1; then
    echo "   Using patchelf..."
    if patchelf --remove-needed libgcc_s.so.1 "$SO_BASENAME" 2>&1; then
        STRIPPED=true
        ok "GCC dependency removed via patchelf"
    else
        warn "patchelf also failed"
    fi
fi

$STRIPPED || warn "Could not strip GCC dependency — the fix may not work."

# ── Re-inject .so into the JAR ────────────────────────────────

step "Patching libsignal JAR"

# Remove old entry (ignore error if not present)
zip -d "$LIBSIGNAL_JAR" "libsignal_jni.so" 2>/dev/null || true

# Add the (possibly stripped) .so — -j strips leading path components
if zip -uj "$LIBSIGNAL_JAR" "$SO_BASENAME" 2>/dev/null; then
    ok "Patched JAR: $LIBSIGNAL_JAR"
else
    die "zip failed — could not update the JAR"
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
