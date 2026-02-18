#!/data/data/com.termux/files/usr/bin/bash
# ============================================
# Clide Installer for Termux
# One-liner: curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
# ============================================

set -e

REPO="juanitto-maker/Clide"
INSTALL_DIR="$HOME/Clide_Source"
SIGNAL_VERSION="0.12.8"  # Last version requiring Java 17 (Termux-compatible)

# ─── Guards ───────────────────────────────────────────────────────────────────

if [[ ! "$PREFIX" =~ "com.termux" ]]; then
    echo "❌ This installer is for Termux on Android only."
    exit 1
fi
echo "✅ Termux detected"

# ─── Helpers ──────────────────────────────────────────────────────────────────

# Read user input from /dev/tty so it works when piped through curl | bash
ask() {
    local prompt="$1"
    local varname="$2"
    local secret="${3:-}"
    printf "%s" "$prompt" >/dev/tty
    if [ "$secret" = "secret" ]; then
        IFS= read -rs answer </dev/tty
        echo "" >/dev/tty
    else
        IFS= read -r answer </dev/tty
    fi
    eval "$varname=\"\$answer\""
}

step() { echo ""; echo "── $1 ──────────────────────────────────────────"; }

# Show a simple spinner while a background PID is running
spinner() {
    local pid=$1 msg="${2:-please wait}"
    local i=0 frames='|/-\'
    while kill -0 "$pid" 2>/dev/null; do
        printf "\r   [%s] %s..." "${frames:$((i%4)):1}" "$msg" >/dev/tty
        sleep 0.3
        i=$((i+1))
    done
    printf "\r%-70s\r" "" >/dev/tty   # clear the spinner line
}

# ─── 1. System packages ───────────────────────────────────────────────────────

step "Updating packages"
pkg update -y 2>&1 | grep -E "^(Get:|Fetched|Reading)" | tail -5 || true
echo "✅ Done"

step "Installing dependencies"
pkg install -y git wget 2>&1 | grep -E "^(Unpacking|Setting up)" | sed 's/^/   /' || true
echo "✅ Done"

# ─── 2. Signal-CLI ────────────────────────────────────────────────────────────

step "Installing Signal-CLI v${SIGNAL_VERSION}"
echo "   (Requires Java 17)"

pkg install -y openjdk-17 2>&1 | grep -E "^(Unpacking|Setting up)" | sed 's/^/   /' || true

SIGNAL_URL="https://github.com/AsamK/signal-cli/releases/download/v${SIGNAL_VERSION}/signal-cli-${SIGNAL_VERSION}.tar.gz"
SIGNAL_DEST="$HOME/.local/signal-cli-${SIGNAL_VERSION}"

if [ ! -d "$SIGNAL_DEST" ]; then
    echo "   Downloading signal-cli..."
    cd "$TMPDIR"
    wget -q --show-progress "$SIGNAL_URL" 2>&1 | tail -2
    tar xf "signal-cli-${SIGNAL_VERSION}.tar.gz"
    mkdir -p "$HOME/.local"
    rm -rf "$SIGNAL_DEST"
    mv "signal-cli-${SIGNAL_VERSION}" "$SIGNAL_DEST"
    rm -f "signal-cli-${SIGNAL_VERSION}.tar.gz"
    echo "✅ signal-cli extracted"
else
    echo "✅ signal-cli already present"
fi

# Add to PATH permanently
if ! grep -q "signal-cli-${SIGNAL_VERSION}" ~/.bashrc 2>/dev/null; then
    {
        echo ""
        echo "# Signal-CLI (added by Clide installer)"
        echo "export PATH=\"\$HOME/.local/signal-cli-${SIGNAL_VERSION}/bin:\$PATH\""
    } >>~/.bashrc
fi
export PATH="$SIGNAL_DEST/bin:$PATH"

# ─── 3. Fix libsignal for ARM64 ───────────────────────────────────────────────
# signal-cli ships an x86_64 native lib; we need an ARM64 one, and we must
# strip the libgcc_s.so.1 ELF dependency from it.
#
# Root cause of "libgcc_s.so.1 not found":
#   The JVM extracts libsignal_jni.so to a tmpdir and loads it via Android's
#   bionic linker "default" namespace, which does NOT include $PREFIX/lib.
#   A symlink in $PREFIX/lib cannot help — the dependency must be stripped
#   from the .so itself using patchelf/termux-elf-cleaner.
#
# Belt-and-suspenders: we ALSO compile a stub libgcc_s.so.1 and wrap the
# signal-cli launcher with LD_PRELOAD, so even if JAR patching is incomplete
# the library loads at runtime.

SIGNAL_LIB_DIR="$SIGNAL_DEST/lib"
LIBSIGNAL_JAR=$(ls "$SIGNAL_LIB_DIR"/libsignal-client-*.jar 2>/dev/null | head -n1 || true)

if [ -n "$LIBSIGNAL_JAR" ]; then
    LIBSIGNAL_VER=$(basename "$LIBSIGNAL_JAR" | sed 's/libsignal-client-//' | sed 's/\.jar//')
    echo "   libsignal version: $LIBSIGNAL_VER"

    # Install ALL patching/stripping tools
    pkg install -y patchelf termux-elf-cleaner zip unzip 2>/dev/null | tail -1 || true

    LIB_WORK="$TMPDIR/libsignal_fix"
    rm -rf "$LIB_WORK"; mkdir -p "$LIB_WORK"
    SO_FILE=""

    # Discover the exact in-JAR path of libsignal_jni.so upfront.
    # It can be at root ("libsignal_jni.so") or in a subdir like
    # "linux/aarch64/libsignal_jni.so" depending on libsignal version.
    # We need this path for BOTH extraction and re-packing.
    SO_IN_JAR=$(unzip -l "$LIBSIGNAL_JAR" 2>/dev/null \
        | awk '{print $NF}' | grep 'libsignal_jni\.so$' | head -n1 || true)
    echo "   .so path in JAR: ${SO_IN_JAR:-<unknown, will try common locations>}"

    # Strategy 1: download a pre-built ARM64 .so from exquo/signal-libs-build
    ARM64_URL="https://github.com/exquo/signal-libs-build/releases/download/libsignal_v${LIBSIGNAL_VER}/libsignal_jni.so-v${LIBSIGNAL_VER}-aarch64-unknown-linux-gnu.tar.gz"
    echo "   Fetching ARM64 libsignal..."
    if wget -q --timeout=30 "$ARM64_URL" -O "$LIB_WORK/libsignal_arm64.tar.gz" 2>/dev/null; then
        tar xf "$LIB_WORK/libsignal_arm64.tar.gz" -C "$LIB_WORK" 2>/dev/null || true
        SO_FILE=$(find "$LIB_WORK" -name "libsignal_jni.so" | head -n1 || true)
        [ -n "$SO_FILE" ] && echo "   ARM64 lib downloaded from exquo" || \
            echo "⚠️  Archive downloaded but .so not found inside"
    else
        echo "   ARM64 build not on exquo for v${LIBSIGNAL_VER} — extracting from JAR instead"
    fi

    # Strategy 2: extract whatever .so is already packed in the JAR.
    # (signal-cli 0.12.x JARs already carry an ARM64 .so — the only problem
    #  is the libgcc_s.so.1 ELF dep that we strip below.)
    if [ -z "$SO_FILE" ]; then
        echo "   Extracting .so from JAR (will strip GCC dep)..."
        if [ -n "$SO_IN_JAR" ]; then
            unzip -o "$LIBSIGNAL_JAR" "$SO_IN_JAR" -d "$LIB_WORK" 2>/dev/null || true
        else
            for _cand in "libsignal_jni.so" "linux/libsignal_jni.so" "linux/aarch64/libsignal_jni.so"; do
                unzip -o "$LIBSIGNAL_JAR" "$_cand" -d "$LIB_WORK" 2>/dev/null && break || true
            done
        fi
        SO_FILE=$(find "$LIB_WORK" -name "libsignal_jni.so" | head -n1 || true)
        [ -n "$SO_FILE" ] && echo "   Using .so extracted from JAR" || \
            echo "⚠️  libsignal_jni.so not found in archive or JAR"
    fi

    if [ -n "$SO_FILE" ]; then
        # Strip libgcc_s.so.1 — run patchelf (precise) AND termux-elf-cleaner (broad).
        # Both run unconditionally; patchelf first because it targets the exact dep.
        STRIPPED=false
        if command -v patchelf >/dev/null 2>&1; then
            if patchelf --remove-needed libgcc_s.so.1 "$SO_FILE" 2>/dev/null; then
                echo "   libgcc_s.so.1 removed via patchelf"
                STRIPPED=true
            fi
        fi
        if command -v termux-elf-cleaner >/dev/null 2>&1; then
            termux-elf-cleaner "$SO_FILE" 2>/dev/null || true
            echo "   ELF cleaned via termux-elf-cleaner"
            STRIPPED=true
        fi
        $STRIPPED || echo "⚠️  No stripping tools ran — libgcc dep may remain in JAR"

        # Re-pack at the original in-JAR path so the JVM loader finds it.
        REL_SO="${SO_IN_JAR:-libsignal_jni.so}"
        if [ ! -f "$LIB_WORK/$REL_SO" ]; then
            mkdir -p "$LIB_WORK/$(dirname "$REL_SO")"
            cp "$SO_FILE" "$LIB_WORK/$REL_SO"
        fi
        zip -d "$LIBSIGNAL_JAR" "$REL_SO" 2>/dev/null || true
        ( cd "$LIB_WORK" && zip -u "$LIBSIGNAL_JAR" "$REL_SO" ) 2>/dev/null && \
            echo "✅ libsignal patched and injected into JAR" || \
            echo "⚠️  Could not update JAR (bot may fail to start)"
    else
        echo "⚠️  Skipping JAR patch — could not obtain libsignal_jni.so"
    fi
else
    echo "⚠️  libsignal JAR not found in $SIGNAL_LIB_DIR"
fi

# ─── 3b. LD_PRELOAD safety net ────────────────────────────────────────────────
# Compile a minimal stub shared library with SONAME=libgcc_s.so.1 using Termux's
# clang, then wrap the signal-cli launcher to LD_PRELOAD it before the JVM starts.
# This satisfies the DT_NEEDED dependency at the bionic linker level, bypassing
# the namespace restriction entirely — no JAR modification needed.

STUB_LIB="$PREFIX/lib/libgcc_s.so.1"
SIGNAL_BIN="$SIGNAL_DEST/bin/signal-cli"
SIGNAL_BIN_REAL="$SIGNAL_DEST/bin/signal-cli.real"

if [ ! -f "$STUB_LIB" ]; then
    if command -v clang >/dev/null 2>&1; then
        printf 'void __libgcc_stub(void){}\n' \
            | clang -shared -fPIC -Wl,-soname,libgcc_s.so.1 \
                    -o "$STUB_LIB" -x c - 2>/dev/null \
            && echo "✅ libgcc_s.so.1 stub compiled" \
            || echo "⚠️  clang stub compilation failed"
    fi
    # Fallback: symlink an existing GCC lib if one is available
    if [ ! -f "$STUB_LIB" ]; then
        for _src in "$PREFIX/lib/libgcc.so" "$PREFIX/lib/libgcc_s.so" \
                    /system/lib64/libgcc.so /system/lib/libgcc.so; do
            [ -f "$_src" ] && ln -sf "$_src" "$STUB_LIB" \
                && echo "✅ libgcc_s.so.1 → $(basename "$_src")" && break || true
        done
    fi
fi

if [ -f "$STUB_LIB" ] && [ -f "$SIGNAL_BIN" ] && [ ! -f "$SIGNAL_BIN_REAL" ]; then
    mv "$SIGNAL_BIN" "$SIGNAL_BIN_REAL"
    printf '#!/bin/sh\n# Termux: preload libgcc_s stub so bionic resolves it for libsignal_jni.so\nexport LD_PRELOAD="%s${LD_PRELOAD:+:$LD_PRELOAD}"\nexec "%s" "$@"\n' \
        "$STUB_LIB" "$SIGNAL_BIN_REAL" > "$SIGNAL_BIN"
    chmod +x "$SIGNAL_BIN"
    echo "✅ signal-cli wrapped with LD_PRELOAD=$STUB_LIB"
elif [ -f "$SIGNAL_BIN_REAL" ]; then
    echo "✅ signal-cli LD_PRELOAD wrapper already in place"
fi

# Verify
if command -v signal-cli >/dev/null 2>&1; then
    echo "✅ signal-cli: $(signal-cli --version 2>/dev/null | head -1)"
else
    echo "⚠️  signal-cli not yet in PATH (will be after: source ~/.bashrc)"
fi

# ─── 4. Install Clide binary ──────────────────────────────────────────────────

step "Installing Clide binary"
mkdir -p "$PREFIX/bin"

CLIDE_INSTALLED=false

# 4a. Try pre-built binary from GitHub Releases (fast path, skips Rust build)
echo "   Checking for pre-built binary..."
LATEST_TAG=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
    | grep '"tag_name"' | sed 's/.*"v\([^"]*\)".*/\1/' | head -1 || true)

if [ -n "$LATEST_TAG" ]; then
    BIN_URL="https://github.com/${REPO}/releases/download/v${LATEST_TAG}/clide-aarch64"
    echo "   Trying pre-built binary for v${LATEST_TAG}..."
    if wget -q "$BIN_URL" -O "$PREFIX/bin/clide" 2>/dev/null; then
        chmod +x "$PREFIX/bin/clide"
        echo "✅ Pre-built binary installed (v${LATEST_TAG})"
        CLIDE_INSTALLED=true
    else
        echo "   No aarch64 binary in release v${LATEST_TAG} — will build from source."
        rm -f "$PREFIX/bin/clide"
    fi
else
    echo "   No release found — will build from source."
fi

# 4b. Build from source (fallback)
if [ "$CLIDE_INSTALLED" = false ]; then
    step "Building Clide from source"
    echo "   ⚠️  No pre-built binary — compiling from source."
    echo "   This takes 10-15 min on most devices. Keep Termux open."
    echo ""

    pkg install -y rust binutils pkg-config openssl \
        >"$TMPDIR/pkg_rust.log" 2>&1 &
    spinner $! "Installing Rust toolchain"
    wait $! || { echo "❌ Rust install failed"; cat "$TMPDIR/pkg_rust.log"; exit 1; }
    grep -E "^(Unpacking|Setting up)" "$TMPDIR/pkg_rust.log" | \
        sed 's/^/   /' | tail -3 || true

    if ! command -v cargo >/dev/null 2>&1; then
        echo "❌ Rust installation failed"
        exit 1
    fi
    echo "✅ $(rustc --version)"

    if [ -d "$INSTALL_DIR" ]; then
        echo "   Updating existing source..."
        git -C "$INSTALL_DIR" pull --ff-only origin main 2>/dev/null || true
    else
        echo "   Cloning repository..."
        git clone "https://github.com/${REPO}.git" "$INSTALL_DIR" 2>&1 | \
            grep -E "^(Cloning|Receiving|Resolving)" || true
    fi
    cd "$INSTALL_DIR"

    # Termux build environment
    export CC="$PREFIX/bin/clang"
    export AR="$PREFIX/bin/llvm-ar"
    export OPENSSL_INCLUDE_DIR="$PREFIX/include"
    export OPENSSL_LIB_DIR="$PREFIX/lib"

    echo "   Compiling Clide... (started $(date '+%H:%M:%S'), this will take a while)"
    BUILD_LOG="$TMPDIR/clide_build.log"

    cargo build --release 2>&1 | tee "$BUILD_LOG" | \
        grep -E "^(   Compiling|   Finished|  Downloaded|  Downloading|error\[)" || true

    if [ ! -f "target/release/clide" ]; then
        echo "❌ Build failed. Log: $BUILD_LOG"
        exit 1
    fi

    cp target/release/clide "$PREFIX/bin/clide"
    chmod +x "$PREFIX/bin/clide"
    echo "✅ Built and installed. Finished: $(date '+%H:%M:%S')"
    CLIDE_INSTALLED=true
fi

# ─── 5. Configuration ─────────────────────────────────────────────────────────

step "Configuration"

mkdir -p ~/.config/clide
chmod 700 ~/.config/clide

mkdir -p ~/.clide
chmod 700 ~/.clide

# Check if config.example.yaml is available (from source clone)
if [ -f "$INSTALL_DIR/config.example.yaml" ]; then
    [ ! -f ~/.clide/config.yaml ] && cp "$INSTALL_DIR/config.example.yaml" ~/.clide/config.yaml
elif [ ! -f ~/.clide/config.yaml ]; then
    # Write minimal config inline (for binary-only install)
    cat >~/.clide/config.yaml <<'YAML'
# Clide configuration - edit as needed
gemini_api_key: ""        # Set via GEMINI_API_KEY env var or enter below
gemini_model: "gemini-2.0-flash"
signal_number: ""         # Your Signal phone number e.g. +1234567890
require_confirmation: false
confirmation_timeout: 60
authorized_numbers: []    # Numbers allowed to use the bot (empty = allow all)
blocked_commands:
  - "rm -rf /"
  - "mkfs"
  - "dd if="
logging:
  level: "info"
YAML
fi
chmod 600 ~/.clide/config.yaml

# ─── 6. Interactive setup ─────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════════"
echo "  Quick Setup"
echo "═══════════════════════════════════════" >/dev/tty

echo "" >/dev/tty
echo "🔑 Gemini API Key" >/dev/tty
echo "   Get one free at: https://makersuite.google.com/app/apikey" >/dev/tty
echo "" >/dev/tty
ask "Enter API key (or press Enter to skip): " API_KEY secret

if [ -n "$API_KEY" ]; then
    # Store in env config for REPL mode
    mkdir -p ~/.config/clide
    if grep -q "GEMINI_API_KEY" ~/.config/clide/config.env 2>/dev/null; then
        sed -i "s|GEMINI_API_KEY=.*|GEMINI_API_KEY=$API_KEY|" ~/.config/clide/config.env
    else
        echo "GEMINI_API_KEY=$API_KEY" >>~/.config/clide/config.env
    fi
    chmod 600 ~/.config/clide/config.env
    export GEMINI_API_KEY="$API_KEY"

    # Also patch yaml config for bot mode
    sed -i "s|gemini_api_key:.*|gemini_api_key: \"$API_KEY\"|" ~/.clide/config.yaml

    echo "✅ API key saved" >/dev/tty
else
    echo "⚠️  Skipped. Set later: export GEMINI_API_KEY='your-key'" >/dev/tty
fi

echo "" >/dev/tty
echo "📱 Signal Phone Number" >/dev/tty
echo "   Format: +CountryCodeNumber (e.g. +12025551234)" >/dev/tty
echo "" >/dev/tty
ask "Enter your Signal number (or press Enter to skip): " SIGNAL_NUM

SIGNAL_PAIRED=false

if [ -n "$SIGNAL_NUM" ]; then
    sed -i "s|signal_number:.*|signal_number: \"$SIGNAL_NUM\"|" ~/.clide/config.yaml
    echo "✅ Signal number saved" >/dev/tty
else
    echo "⚠️  Skipped. Edit ~/.clide/config.yaml later." >/dev/tty
fi

# ── Optional: pair with Signal now ───────────────────────────────────────────
echo "" >/dev/tty
echo "🔗 Signal Device Pairing" >/dev/tty
echo "   Link this device to your Signal account right now?" >/dev/tty
echo "   (You need Signal installed on your phone.)" >/dev/tty
ask "Pair now? [Y/n]: " DO_PAIR

if [[ ! "$DO_PAIR" =~ ^[Nn] ]]; then
    pkg install -y qrencode 2>/dev/null | tail -1 || true

    echo "" >/dev/tty
    echo "   On your phone: Signal → Settings → Linked Devices → Link New Device" >/dev/tty
    echo "" >/dev/tty

    LINK_LOG="$TMPDIR/signal_link_$$.log"
    signal-cli link -n "clide-bot" > "$LINK_LOG" 2>&1 &
    LINK_PID=$!

    # Wait up to 15 s for signal-cli to emit the tsdevice:/ URI
    LINK_URI=""
    for _i in $(seq 1 30); do
        LINK_URI=$(grep -o 'tsdevice:[^[:space:]]*' "$LINK_LOG" 2>/dev/null | head -1 || true)
        [ -n "$LINK_URI" ] && break
        sleep 0.5
    done

    if [ -n "$LINK_URI" ]; then
        # Always print the raw URI first — paste it into any QR-code generator app
        # (e.g. QR & Barcode Scanner, or qr.io) if you can't scan the terminal QR.
        echo "" >/dev/tty
        echo "┌─────────────────────────────────────────────────┐" >/dev/tty
        echo "│  Pairing URI (copy → QR generator app if needed) │" >/dev/tty
        echo "└─────────────────────────────────────────────────┘" >/dev/tty
        echo "$LINK_URI" >/dev/tty
        echo "" >/dev/tty
        if command -v qrencode >/dev/null 2>&1; then
            echo "   Terminal QR code (scan directly if your camera app supports it):" >/dev/tty
            qrencode -t ansiutf8 "$LINK_URI" >/dev/tty
        fi
        echo "" >/dev/tty
        echo "   On your phone: Signal → Settings → Linked Devices → Link New Device" >/dev/tty
        echo "   Waiting for scan... (Ctrl+C to skip, pair later)" >/dev/tty
        if wait "$LINK_PID" 2>/dev/null; then
            echo "✅ Signal device linked!" >/dev/tty
            SIGNAL_PAIRED=true
        else
            echo "⚠️  Pairing timed out or was cancelled." >/dev/tty
            echo "   Run later: signal-cli link -n 'clide-bot'" >/dev/tty
        fi
    else
        kill "$LINK_PID" 2>/dev/null || true
        echo "⚠️  signal-cli link did not produce a pairing URI." >/dev/tty
        head -3 "$LINK_LOG" 2>/dev/null >/dev/tty || true
        echo "   If you see a libsignal error above, run: bash fix-libsignal.sh" >/dev/tty
        echo "   Then retry: signal-cli link -n 'clide-bot'" >/dev/tty
    fi
    rm -f "$LINK_LOG"
fi

# ─── 7. Summary ───────────────────────────────────────────────────────────────

echo ""
echo "═══════════════════════════════════════"
echo "✨ Installation Complete!"
echo "═══════════════════════════════════════"
echo ""

if command -v clide >/dev/null 2>&1; then
    echo "✅ $(clide --version)"
fi

echo ""
echo "Usage:"
echo "  clide              # Chat with Gemini (REPL)"
echo "  clide bot          # Start Signal bot"
echo "  clide --version    # Show version"
echo ""
echo "Next steps:"
echo "  source ~/.bashrc"
if $SIGNAL_PAIRED; then
    echo "  clide bot   # Start the bot (device already paired!)"
else
    echo "  signal-cli link -n \"clide-bot\"   # Scan QR: Signal → Settings → Linked Devices"
    echo "  clide bot                         # Start the bot"
fi
echo ""
echo "Config file: ~/.clide/config.yaml"
echo "API key file: ~/.config/clide/config.env"
echo ""
