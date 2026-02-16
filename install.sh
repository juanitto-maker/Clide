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
# 5. Clone Repository
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
echo "🛠️  Building Clide..."
echo "   This is the longest step (5-15 minutes)"
echo "   ☕ Grab a coffee!"
echo ""
echo "   Started at: $(date '+%H:%M:%S')"
echo ""

# Build with progress indicator
cargo build --release 2>&1 | while IFS= read -r line; do
    # Show only important lines
    if echo "$line" | grep -qE "Compiling|Finished|error:|warning:"; then
        echo "   $line"
    fi
    # Show progress dots for other lines to indicate it's working
    if echo "$line" | grep -qE "Downloading|Updating"; then
        echo -n "."
    fi
done

echo ""
echo ""
echo "✅ Build completed at: $(date '+%H:%M:%S')"
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
    clide --version 2>&1 || echo "   (Configuration needed)"
else
    echo "⚠️  Installation completed"
    echo "   Restart Termux to use 'clide' command"
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
echo "💡 If 'clide' command not found, restart Termux"
echo ""
echo "🎉 Happy hacking!"
