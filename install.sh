#!/data/data/com.termux/files/usr/bin/bash
# ============================================
# Clide Complete Installer
# Installs: Clide + Signal-CLI (default) + Configuration
# ============================================
set -e

echo "🚀 Installing Clide - Complete Setup"
echo ""

# ============================================
# 1. Detect if Termux
# ============================================
if [[ ! "$PREFIX" =~ "com.termux" ]]; then
    echo "❌ This installer is for Termux on Android"
    exit 1
fi

echo "✅ Termux detected"
echo ""

# ============================================
# 2. Update packages
# ============================================
echo "📦 Updating package lists..."
pkg update -y 2>&1 | grep -E "Get:|Fetched|Reading" | tail -n 5
echo "✅ Package lists updated"
echo ""

# ============================================
# 3. Install Rust and dependencies
# ============================================
echo "📦 Installing Rust and build dependencies..."
echo "   This takes 2-3 minutes, please wait..."
echo ""

pkg install -y rust binutils git pkg-config openssl 2>&1 | while IFS= read -r line; do
    if echo "$line" | grep -qE "Unpacking|Setting up|Processing"; then
        echo "   $line"
    fi
done

echo ""
echo "✅ Packages installed"
echo ""

# ============================================
# 4. Verify Rust installation
# ============================================
echo "🦀 Verifying Rust..."
if ! command -v cargo >/dev/null 2>&1; then
    echo "❌ Rust installation failed!"
    exit 1
fi

RUST_VERSION=$(rustc --version 2>&1)
CARGO_VERSION=$(cargo --version 2>&1)
echo "✅ $RUST_VERSION"
echo "✅ $CARGO_VERSION"
echo ""

# ============================================
# 5. Install Signal-CLI (DEFAULT)
# ============================================
echo "📱 Installing Signal-CLI..."
echo "   (Required for Signal bot functionality)"
echo ""

pkg install -y openjdk-17 wget 2>&1 | tail -n 3

SIGNAL_VERSION="0.12.8"  # Java 17 compatible (0.13.x requires Java 21)
SIGNAL_URL="https://github.com/AsamK/signal-cli/releases/download/v${SIGNAL_VERSION}/signal-cli-${SIGNAL_VERSION}.tar.gz"

cd "$TMPDIR"
wget -q --show-progress "$SIGNAL_URL" 2>&1 | tail -n 2

echo "📦 Installing Signal-CLI..."
tar xf "signal-cli-${SIGNAL_VERSION}.tar.gz"

mkdir -p "$HOME/.local"
rm -rf "$HOME/.local/signal-cli-${SIGNAL_VERSION}"
mv "signal-cli-${SIGNAL_VERSION}" "$HOME/.local/"

# Add to PATH
if ! grep -q "signal-cli-${SIGNAL_VERSION}" ~/.bashrc; then
    echo "" >> ~/.bashrc
    echo "# Signal-CLI (added by Clide installer)" >> ~/.bashrc
    echo "export PATH=\$HOME/.local/signal-cli-${SIGNAL_VERSION}/bin:\$PATH" >> ~/.bashrc
fi

export PATH="$HOME/.local/signal-cli-${SIGNAL_VERSION}/bin:$PATH"

# Cleanup
rm -f "$TMPDIR/signal-cli-${SIGNAL_VERSION}.tar.gz"

# Verify signal-cli works
if command -v signal-cli >/dev/null 2>&1; then
    echo "✅ Signal-CLI installed: $(signal-cli --version | head -n1)"
else
    SIGNAL_BIN="$HOME/.local/signal-cli-${SIGNAL_VERSION}/bin/signal-cli"
    if [ -f "$SIGNAL_BIN" ]; then
        echo "✅ Signal-CLI at: $SIGNAL_BIN"
        echo "   Run: source ~/.bashrc"
    else
        echo "❌ Signal-CLI installation failed"
    fi
fi
echo ""

# ============================================
# 6. Clone Repository
# ============================================
echo "📂 Cloning Clide repository..."
INSTALL_DIR="$HOME/Clide_Source"

if [ -d "$INSTALL_DIR" ]; then
    echo "   Removing old installation..."
    rm -rf "$INSTALL_DIR"
fi

git clone https://github.com/juanitto-maker/Clide.git "$INSTALL_DIR" 2>&1 | grep -E "Cloning|Receiving|Resolving" || true
cd "$INSTALL_DIR"
echo "✅ Repository cloned"
echo ""

# ============================================
# 7. Fix Cargo.toml for Android
# ============================================
echo "🔧 Applying Android compatibility fixes..."

cp Cargo.toml Cargo.toml.backup
sed -i 's/features = \["json", "rustls-tls"\]/features = ["json", "native-tls"]/' Cargo.toml

echo "✅ TLS configuration fixed for Android"
echo ""

# ============================================
# 8. Build Clide
# ============================================
echo "🛠️  Building Clide..."
echo "   This is the longest step (5-15 minutes)"
echo "   ☕ Grab a coffee!"
echo ""
echo "   Started at: $(date '+%H:%M:%S')"
echo ""

# Set Android/Termux build environment
export CC="$PREFIX/bin/clang"
export AR="$PREFIX/bin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$PREFIX/bin/clang"
export OPENSSL_INCLUDE_DIR="$PREFIX/include"
export OPENSSL_LIB_DIR="$PREFIX/lib"

BUILD_LOG="$TMPDIR/clide_build.log"

cargo build --release 2>&1 | tee "$BUILD_LOG" | while IFS= read -r line; do
    if echo "$line" | grep -qE "Compiling|Finished|error:|warning:"; then
        echo "   $line"
    fi
    if echo "$line" | grep -qE "Downloading|Updating"; then
        echo -n "."
    fi
done

echo ""
echo ""

# Check if binary actually exists
if [ ! -f "target/release/clide" ]; then
    echo "❌ Build FAILED! Errors:"
    grep -E "^error" "$BUILD_LOG" | head -n 20
    echo ""
    echo "Full log: $BUILD_LOG"
    exit 1
fi

echo "✅ Build completed at: $(date '+%H:%M:%S')"
echo ""

# ============================================
# 9. Install Binary
# ============================================
echo "🚚 Installing Clide binary..."

mkdir -p "$PREFIX/bin"
cp target/release/clide "$PREFIX/bin/clide"
chmod +x "$PREFIX/bin/clide"

echo "✅ Installed to: $PREFIX/bin/clide"
echo ""

# ============================================
# 10. Configuration Setup
# ============================================
echo "⚙️  Setting up configuration..."
echo ""

mkdir -p ~/.clide/logs
chmod 700 ~/.clide

cp "$INSTALL_DIR/config.example.yaml" ~/.clide/config.yaml
chmod 600 ~/.clide/config.yaml

echo "✅ Config file created at: ~/.clide/config.yaml"
echo ""

# ============================================
# 11. Interactive Configuration
# ============================================

# Helper function - reads from keyboard even when piped
ask() {
    local prompt="$1"
    local varname="$2"
    local secret="$3"
    printf "%s" "$prompt" > /dev/tty
    if [ "$secret" = "secret" ]; then
        IFS= read -rs answer < /dev/tty
        echo "" > /dev/tty
    else
        IFS= read -r answer < /dev/tty
    fi
    eval "$varname=\$answer"
}

echo "═══════════════════════════════════════" > /dev/tty
echo "🔑 Gemini API Key Setup" > /dev/tty
echo "═══════════════════════════════════════" > /dev/tty
echo "" > /dev/tty
echo "To use Clide, you need a Gemini API key." > /dev/tty
echo "Get one free at: https://makersuite.google.com/app/apikey" > /dev/tty
echo "" > /dev/tty
echo "🔒 Key will be stored as environment variable (secure)" > /dev/tty
echo "" > /dev/tty

ask "Enter your Gemini API key (or press Enter to skip): " API_KEY secret
echo ""

if [ ! -z "$API_KEY" ]; then
    if ! grep -q "GEMINI_API_KEY" ~/.bashrc; then
        echo "" >> ~/.bashrc
        echo "# Clide Configuration (added by installer)" >> ~/.bashrc
        echo "export GEMINI_API_KEY='$API_KEY'" >> ~/.bashrc
    else
        sed -i "s/export GEMINI_API_KEY=.*/export GEMINI_API_KEY='$API_KEY'/" ~/.bashrc
    fi
    chmod 600 ~/.bashrc
    export GEMINI_API_KEY="$API_KEY"
    echo "✅ API key securely stored!"
    CONFIG_READY=true
else
    echo "⚠️  Skipped API key - set later: export GEMINI_API_KEY='your-key'"
    CONFIG_READY=false
fi

echo ""

# ============================================
# 12. Signal Number Setup
# ============================================
echo "═══════════════════════════════════════" > /dev/tty
echo "📱 Signal Number Configuration" > /dev/tty
echo "═══════════════════════════════════════" > /dev/tty
echo "" > /dev/tty

ask "Enter your Signal number (e.g., +1234567890) or Enter to skip: " SIGNAL_NUMBER
echo ""

if [ ! -z "$SIGNAL_NUMBER" ]; then
    sed -i "s/+1234567890/$SIGNAL_NUMBER/" ~/.clide/config.yaml
    echo "✅ Signal number configured!"
    echo "💡 Next: signal-cli link -n \"clide-bot\""
else
    echo "⚠️  Skipped - configure later: nano ~/.clide/config.yaml"
fi

echo ""

# ============================================
# 13. Verify Installation
# ============================================
echo "🔍 Verifying installation..."
echo ""

if command -v clide >/dev/null 2>&1; then
    echo "✅ Clide: $(clide --version)"
else
    echo "⚠️  Clide installed (restart Termux to use)"
fi

if command -v signal-cli >/dev/null 2>&1; then
    echo "✅ Signal-CLI: $(signal-cli --version | head -n1)"
else
    echo "⚠️  Signal-CLI not found in PATH"
fi

echo ""

# ============================================
# 14. Final Summary
# ============================================
echo "═══════════════════════════════════════"
echo "✨ Installation Complete!"
echo "═══════════════════════════════════════"
echo ""

if [ "$CONFIG_READY" = true ]; then
    echo "🎉 Clide is ready to use!"
    echo ""
    echo "Try these commands:"
    echo "   clide test-gemini 'hello'   # Test Gemini API"
    echo "   clide status                # Check system"
    echo "   clide start                 # Start Signal bot"
    echo ""
else
    echo "📝 To finish setup:"
    echo ""
    echo "1️⃣  Get API key: https://makersuite.google.com/app/apikey"
    echo "2️⃣  Set environment:"
    echo "     export GEMINI_API_KEY='your-key-here'"
    echo "     echo 'export GEMINI_API_KEY=\"your-key\"' >> ~/.bashrc"
    echo "3️⃣  Test: clide test-gemini 'hello'"
    echo ""
fi

echo "📱 Signal Bot Setup:"
echo "   1. Link device: signal-cli link -n \"clide-bot\""
echo "   2. Scan QR code with Signal app"
echo "   3. Start bot: clide start"
echo ""
echo "🔒 Security:"
echo "   • API key stored as environment variable"
echo "   • Config files have 600 permissions"
echo "   • No secrets in git history"
echo ""
echo "📚 Documentation: $INSTALL_DIR/README.md"
echo "⚙️  Config file: ~/.clide/config.yaml"
echo "💡 Model: gemini-2.5-flash"
echo ""
echo "🎉 Happy hacking!"
