#!/data/data/com.termux/files/usr/bin/bash
# ============================================
# Clide Installer - Secure & User-Friendly
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
    if echo "$line" | grep -qE "Compiling|Finished|error:|warning:"; then
        echo "   $line"
    fi
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

mkdir -p "$PREFIX/bin"
cp target/release/clide "$PREFIX/bin/clide"
chmod +x "$PREFIX/bin/clide"

echo "✅ Installed to: $PREFIX/bin/clide"
echo ""

# ============================================
# 9. Secure Configuration Setup
# ============================================
echo "⚙️  Setting up configuration..."
echo ""

# Create config directory with secure permissions
mkdir -p ~/.clide/logs
chmod 700 ~/.clide

# Copy example config
cp "$INSTALL_DIR/config.example.yaml" ~/.clide/config.yaml
chmod 600 ~/.clide/config.yaml

echo "✅ Config file created at: ~/.clide/config.yaml"
echo ""

# ============================================
# 10. Secure API Key Setup
# ============================================
echo "═══════════════════════════════════════"
echo "🔑 Gemini API Key Setup (Secure)"
echo "═══════════════════════════════════════"
echo ""
echo "To use Clide, you need a Gemini API key."
echo "Get one free at: https://makersuite.google.com/app/apikey"
echo ""
echo "🔒 Security: Your key will be stored as an environment"
echo "   variable (not in the config file) for better security."
echo ""
read -sp "Enter your Gemini API key (hidden): " API_KEY
echo ""
echo ""

if [ ! -z "$API_KEY" ]; then
    # Store securely in .bashrc as environment variable
    echo "" >> ~/.bashrc
    echo "# Clide Configuration (added by installer)" >> ~/.bashrc
    echo "export GEMINI_API_KEY='$API_KEY'" >> ~/.bashrc
    
    # Set strict permissions on .bashrc
    chmod 600 ~/.bashrc
    
    # Load into current session
    export GEMINI_API_KEY="$API_KEY"
    
    # Clear from shell history
    history -d $((HISTCMD-1)) 2>/dev/null || true
    
    echo "✅ API key securely stored as environment variable"
    echo "   (stored in ~/.bashrc with 600 permissions)"
    CONFIG_READY=true
else
    echo "⚠️  Skipped API key setup"
    echo ""
    echo "To set it later, run:"
    echo "  export GEMINI_API_KEY='your-key-here'"
    echo "  echo 'export GEMINI_API_KEY=\"your-key\"' >> ~/.bashrc"
    CONFIG_READY=false
fi

echo ""

# ============================================
# 11. Optional Signal Configuration
# ============================================
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
# 12. Verify Installation
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
# 13. Final Summary
# ============================================
echo "═══════════════════════════════════════"
echo "✨ Installation Complete!"
echo "═══════════════════════════════════════"
echo ""

if [ "$CONFIG_READY" = true ]; then
    echo "🎉 Clide is ready to use immediately!"
    echo ""
    echo "Try these commands:"
    echo "   clide test-gemini 'hello'   # Test Gemini API"
    echo "   clide status                # Check system"
    echo "   clide --help                # See all commands"
    echo ""
else
    echo "📝 To finish setup:"
    echo ""
    echo "1️⃣  Get API key: https://makersuite.google.com/app/apikey"
    echo "2️⃣  Set environment: export GEMINI_API_KEY='your-key'"
    echo "3️⃣  Test: clide test-gemini 'hello'"
    echo ""
fi

echo "🔒 Security:"
echo "   • API key stored as environment variable"
echo "   • Config files have 600 permissions (only you can read)"
echo "   • API key not in config file or git history"
echo ""
echo "📚 Documentation: $INSTALL_DIR/README.md"
echo "⚙️  Config file: ~/.clide/config.yaml"
echo "🔑 API key: Set via GEMINI_API_KEY environment variable"
echo ""
echo "💡 Model: gemini-2.5-flash (fast and efficient!)"
echo ""
echo "🎉 Happy hacking!"
