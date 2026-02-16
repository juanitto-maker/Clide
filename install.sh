#!/data/data/com.termux/files/usr/bin/bash
# ============================================
# Clide Complete Installer - One-Line Setup
# Handles everything from scratch to finish
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
pkg update -y 2>&1 | grep -v "dpkg" || true
echo ""

# ============================================
# 3. Install Rust and dependencies
# ============================================
echo "📦 Installing Rust and build dependencies..."
pkg install -y rust binutils git pkg-config openssl 2>&1 | tail -n 5
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
echo "✅ Rust: $RUST_VERSION"
echo "✅ Cargo: $CARGO_VERSION"
echo ""

# ============================================
# 5. Clone Repository
# ============================================
echo "📂 Cloning Clide..."
INSTALL_DIR="$HOME/Clide_Source"

if [ -d "$INSTALL_DIR" ]; then
    echo "⚠️  Removing old installation..."
    rm -rf "$INSTALL_DIR"
fi

git clone https://github.com/juanitto-maker/Clide.git "$INSTALL_DIR" 2>&1 | tail -n 3
cd "$INSTALL_DIR"
echo "✅ Repository cloned"
echo ""

# ============================================
# 6. Fix Cargo.toml for Android
# ============================================
echo "🔧 Applying Android compatibility fixes..."

# Backup original
cp Cargo.toml Cargo.toml.backup

# Replace rustls-tls with native-tls for Android
sed -i 's/features = \["json", "rustls-tls"\]/features = ["json", "native-tls"]/' Cargo.toml

echo "✅ TLS configuration fixed for Android"
echo ""

# ============================================
# 7. Build Clide
# ============================================
echo "🛠️  Building Clide (this takes 5-15 minutes)..."
echo "☕ Grab a coffee - this is the longest step!"
echo ""
echo "Started at: $(date '+%H:%M:%S')"
echo ""

# Build with progress indicator
if cargo build --release 2>&1 | while IFS= read -r line; do
    # Show only important lines (not all the noise)
    if echo "$line" | grep -qE "Compiling|Finished|error|warning"; then
        echo "$line"
    fi
done; then
    echo ""
    echo "✅ Build completed at: $(date '+%H:%M:%S')"
else
    echo ""
    echo "❌ Build failed!"
    echo ""
    echo "Common issues:"
    echo "  • Low memory - close other apps"
    echo "  • Low storage - need ~2GB free"
    echo ""
    echo "For detailed error, run:"
    echo "  cd $INSTALL_DIR"
    echo "  cargo build --release"
    exit 1
fi
echo ""

# ============================================
# 8. Install Binary
# ============================================
echo "🚚 Installing Clide binary..."

# Create bin directory
mkdir -p "$PREFIX/bin"

# Copy and make executable
cp target/release/clide "$PREFIX/bin/clide"
chmod +x "$PREFIX/bin/clide"

echo "✅ Installed to: $PREFIX/bin/clide"
echo ""

# ============================================
# 9. Verify Installation
# ============================================
echo "🔍 Verifying installation..."

if command -v clide >/dev/null 2>&1; then
    echo "✅ Clide is ready!"
    echo ""
    clide --version 2>&1 || echo "Clide installed (version check pending config)"
else
    echo "⚠️  Installation completed but restart Termux to use 'clide' command"
fi

echo ""

# ============================================
# 10. Setup Configuration
# ============================================
echo "═══════════════════════════════════════"
echo "✨ Installation Complete!"
echo "═══════════════════════════════════════"
echo ""
echo "📝 Next Steps:"
echo ""
echo "1️⃣  Create config directory:"
echo "   mkdir -p ~/.clide"
echo ""
echo "2️⃣  Copy example config:"
echo "   cp $INSTALL_DIR/config.example.yaml ~/.clide/config.yaml"
echo ""
echo "3️⃣  Edit config with your API key:"
echo "   nano ~/.clide/config.yaml"
echo ""
echo "4️⃣  Run Clide:"
echo "   clide --help"
echo ""
echo "💡 Tip: If 'clide' command not found, restart Termux"
echo ""
echo "🎉 Happy hacking!"
