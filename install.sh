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
# 9. Auto-Configure Clide
# ============================================
echo "⚙️  Setting up configuration..."
echo ""

# Create config directory
mkdir -p ~/.clide/logs

# Copy example config
cp "$INSTALL_DIR/config.example.yaml" ~/.clide/config.yaml

echo "✅ Config file created at: ~/.clide/config.yaml"
echo ""

# Ask for API key
echo "═══════════════════════════════════════"
echo "🔑 Gemini API Key Setup"
echo "═══════════════════════════════════════"
echo ""
echo "To use Clide, you need a Gemini API key."
echo "Get one free at: https://makersuite.google.com/app/apikey"
echo ""
read -p "Enter your Gemini API key (or press Enter to skip): " API_KEY

if [ ! -z "$API_KEY" ]; then
    # Insert API key into config
    sed -i "s/YOUR_API_KEY_HERE/$API_KEY/" ~/.clide/config.yaml
    echo ""
    echo "✅ API key configured!"
    CONFIG_READY=true
else
    echo ""
    echo "⚠️  Skipped API key setup"
    echo "   Edit later: nano ~/.clide/config.yaml"
    CONFIG_READY=false
fi

echo ""

# Ask for Signal number (optional)
echo "═══════════════════════════════════════"
echo "📱 Signal Number (Optional)"
echo "═══════════════════════════════════════"
echo ""
read -p "Enter your Signal number (e.g., +1234567890) or press Enter to skip: " SIGNAL_NUMBER

if [ ! -z "$SIGNAL_NUMBER" ]; then
    sed -i "s/+1234567890/$SIGNAL_NUMBER/" ~/.clide/config.yaml
    echo ""
    echo "✅ Signal number configured!"
else
    echo ""
    echo "⚠️  Skipped Signal setup"
    echo "   Configure later: nano ~/.clide/config.yaml"
fi

echo ""

# ============================================
# 10. Verify Installation
# ============================================
echo "🔍 Verifying installation..."

if command -v clide >/dev/null 2>&1; then
    echo "✅ Clide is ready!"
    echo ""
    clide --version 2>&1
else
    echo "⚠️  Installation completed"
    echo "   Restart Termux to use 'clide' command"
fi

echo ""

# ============================================
# 11. Final Summary
# ============================================
echo "═══════════════════════════════════════"
echo "✨ Installation Complete!"
echo "═══════════════════════════════════════"
echo ""

if [ "$CONFIG_READY" = true ]; then
    echo "🎉 Clide is ready to use!"
    echo ""
    echo "Try these commands:"
    echo "   clide status           # Check system status"
    echo "   clide test-gemini      # Test Gemini API"
    echo "   clide start            # Start the bot"
    echo ""
else
    echo "📝 To finish setup:"
    echo ""
    echo "1️⃣  Get API key: https://makersuite.google.com/app/apikey"
    echo "2️⃣  Edit config: nano ~/.clide/config.yaml"
    echo "3️⃣  Test: clide test-gemini"
    echo ""
fi

echo "📚 Documentation: $INSTALL_DIR/README.md"
echo "⚙️  Config file: ~/.clide/config.yaml"
echo "🗂️  Source code: $INSTALL_DIR"
echo ""
echo "💡 Tip: Run 'clide --help' to see all commands"
echo ""
echo "🎉 Happy hacking!"
