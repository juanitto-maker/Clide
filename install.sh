#!/usr/bin/env bash
set -e

# ============================
# Clide One-Run Installer with Prompt
# ============================

INSTALL_DIR="$HOME/.clide"
BIN_DIR="$HOME/.local/bin"

echo "✨ Installing Clide..."

mkdir -p "$INSTALL_DIR"
mkdir -p "$BIN_DIR"

# ============================
# Download prebuilt binary & libs
# ============================

echo "📦 Downloading latest Clide binary..."

TMP_TAR=$(mktemp)
curl -fsSL "https://github.com/juanitto-maker/Clide/releases/latest/download/clide-static-aarch64.tar.gz" -o "$TMP_TAR"
tar -xzf "$TMP_TAR" -C "$INSTALL_DIR"
rm "$TMP_TAR"

chmod +x "$INSTALL_DIR/clide"

# Symlink binary
ln -sf "$INSTALL_DIR/clide" "$BIN_DIR/clide"

# ============================
# Prompt user for API key & Signal number
# ============================

read -p "🔑 Enter your Gemini API key: " GEMINI_API_KEY
read -p "📱 Enter your Signal number (with country code, e.g. +1234567890): " SIGNAL_NUMBER

# ============================
# Create configuration
# ============================

CONFIG_FILE="$INSTALL_DIR/config.yaml"

echo "📝 Writing configuration..."

cat > "$CONFIG_FILE" <<EOL
gemini_api_key: "$GEMINI_API_KEY"
signal_number: "$SIGNAL_NUMBER"
logging:
  level: "info"
  json: false
EOL

# ============================
# Finish
# ============================

echo
echo "✅ Clide installed successfully!"
echo "Binary: $INSTALL_DIR/clide"
echo "Config: $CONFIG_FILE"
echo
echo "Test Gemini API:"
echo "  clide test-gemini 'hello'"
echo
echo "Start the bot:"
echo "  clide start"
