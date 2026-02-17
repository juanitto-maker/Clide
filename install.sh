#!/usr/bin/env bash
set -e

echo "✨ Installing Clide..."

# -----------------------------
# Prompt user FIRST (no vars used before this)
# -----------------------------
echo ""
echo "🔑 Gemini API Key"
printf "Enter Gemini API key: "
read -r GEMINI_API_KEY

if [ -z "$GEMINI_API_KEY" ]; then
    echo "❌ Gemini API key is required"
    exit 1
fi

echo ""
echo "📱 Signal bot number (E.164 format, e.g. +1234567890)"
printf "Enter Signal number (or press Enter to skip): "
read -r SIGNAL_NUMBER

# Make Signal number optional with a default
if [ -z "$SIGNAL_NUMBER" ]; then
    SIGNAL_NUMBER="+0000000000"
    echo "⚠️  Skipping Signal setup. You'll need to configure it manually later."
fi

# -----------------------------
# Paths
# -----------------------------
HOME_DIR="$HOME"
INSTALL_DIR="$HOME_DIR/.clide"
BIN_DIR="$HOME_DIR/.local/bin"
CONFIG_FILE="$INSTALL_DIR/config.yaml"

mkdir -p "$INSTALL_DIR"
mkdir -p "$BIN_DIR"

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
require_confirmation: false
allowed_hosts: []  # Empty = all hosts allowed

# Logging
log_level: "info"
log_file: "~/.clide/logs/clide.log"
EOF

echo "✅ Config created at $CONFIG_FILE"

# -----------------------------
# Check/Install Rust
# -----------------------------
if ! command -v cargo &> /dev/null 2>&1; then
    echo "🦀 Installing Rust..."
    pkg install -y rust
fi

# -----------------------------
# Clone + build
# -----------------------------
SRC_DIR="$HOME_DIR/Clide_Source"
rm -rf "$SRC_DIR"

echo "📦 Cloning Clide..."
git clone https://github.com/juanitto-maker/Clide.git "$SRC_DIR"

cd "$SRC_DIR"
echo "🔨 Building Clide..."
cargo build --release

# -----------------------------
# Install binary
# -----------------------------
cp target/release/clide "$BIN_DIR/clide"
chmod +x "$BIN_DIR/clide"

# Add to PATH if not already there
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$HOME_DIR/.bashrc"
    echo "📝 Added $BIN_DIR to PATH in .bashrc"
    echo "⚠️  Run 'source ~/.bashrc' or restart Termux to use 'clide' command"
fi

echo ""
echo "═══════════════════════════════════════"
echo "✨ Installation Complete!"
echo "═══════════════════════════════════════"
echo ""
echo "Try:"
echo "  clide test-gemini \"hello\""
echo "  clide start"
echo ""
echo "Config: $CONFIG_FILE"
echo ""
