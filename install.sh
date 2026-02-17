#!/usr/bin/env bash
set -e

# Force interactive mode even when piped
exec < /dev/tty

echo "✨ Installing Clide..."

# -----------------------------
# Prompt user for API Key (required)
# -----------------------------
echo ""
echo "🔑 Gemini API Key"
printf "Enter Gemini API key: "
read -r GEMINI_API_KEY

if [ -z "$GEMINI_API_KEY" ]; then
    echo "❌ Gemini API key is required"
    exit 1
fi

# -----------------------------
# Prompt user for Signal Number (optional)
# -----------------------------
echo ""
echo "📱 Signal Bot Number (E.164 format, e.g. +1234567890)"
printf "Enter Signal number (press Enter to skip): "
read -r SIGNAL_NUMBER

if [ -z "$SIGNAL_NUMBER" ]; then
    SIGNAL_NUMBER="+0000000000"
    echo "⚠️  Signal setup skipped. Edit ~/.clide/config.yaml later to enable."
fi

# -----------------------------
# Setup directories
# -----------------------------
HOME_DIR="$HOME"
INSTALL_DIR="$HOME_DIR/.clide"
BIN_DIR="$HOME_DIR/.local/bin"
CONFIG_FILE="$INSTALL_DIR/config.yaml"

mkdir -p "$INSTALL_DIR"
mkdir -p "$BIN_DIR"
mkdir -p "$INSTALL_DIR/logs"

# -----------------------------
# Write config.yaml
# -----------------------------
cat > "$CONFIG_FILE" << EOF
# Clide Configuration
# Generated on $(date)

# Gemini API Key (get from https://makersuite.google.com/app/apikey)
gemini_api_key: "$GEMINI_API_KEY"

# Signal Number (format: +1234567890)
signal_number: "$SIGNAL_NUMBER"

# Execution Settings
allow_commands: true
require_confirmation: true
allowed_hosts: []

# Logging
log_level: "info"
log_file: "$INSTALL_DIR/logs/clide.log"
EOF

echo "✅ Config created"

# -----------------------------
# Install Rust if missing
# -----------------------------
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    pkg install -y rust || apt-get install -y cargo || (echo "❌ Failed to install Rust"; exit 1)
fi

# -----------------------------
# Clone and build
# -----------------------------
SRC_DIR="$HOME_DIR/.clide_source"
rm -rf "$SRC_DIR"

echo "📦 Downloading Clide..."
git clone --depth 1 https://github.com/juanitto-maker/Clide.git "$SRC_DIR"

cd "$SRC_DIR"
echo "🔨 Building Clide (this may take 5-10 minutes)..."
cargo build --release

# -----------------------------
# Install binary
# -----------------------------
cp "$SRC_DIR/target/release/clide" "$BIN_DIR/clide"
chmod +x "$BIN_DIR/clide"

# -----------------------------
# Setup PATH
# -----------------------------
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$HOME_DIR/.bashrc"
    echo "📝 Added to PATH. Run: source ~/.bashrc"
fi

# -----------------------------
# Final instructions
# -----------------------------
echo ""
echo "═══════════════════════════════════════════════════"
echo "✨ Clide Installation Complete!"
echo "═══════════════════════════════════════════════════"
echo ""
echo "📱 To use with Signal:"
echo "   1. Install signal-cli: pkg install signal-cli"
echo "   2. Link your device: signal-cli link -n clide-bot"
echo "   3. Start Clide: clide start"
echo ""
echo "🧪 Test Gemini: clide test-gemini \"hello world\""
echo ""
echo "⚙️  Config: $CONFIG_FILE"
echo ""
