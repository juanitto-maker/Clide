#!/usr/bin/env bash
# ============================================
# Clide Installer - Single Step Installation
# ============================================

set -euo pipefail

INSTALL_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.clide"
TMP_DIR="$HOME/.clide_tmp"

BINARY_NAME="clide"
GITHUB_REPO="juanitto-maker/Clide"
ARCH="aarch64-linux-android"

mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR"
mkdir -p "$TMP_DIR"

echo "✨ Installing Clide..."

# Ask for API key and Signal number
# Ask for API key and Signal number
read -rp "Enter your Gemini API key (leave blank to skip): " GEMINI_API_KEY_INPUT
GEMINI_API_KEY="${GEMINI_API_KEY_INPUT:-}"

read -rp "Enter your Signal number (in international format, e.g., +1234567890, leave blank to skip): " SIGNAL_NUMBER_INPUT
SIGNAL_NUMBER="${SIGNAL_NUMBER_INPUT:-}"

# Prepare config.yaml
CONFIG_FILE="$CONFIG_DIR/config.yaml"
if [ ! -f "$CONFIG_FILE" ]; then
cat > "$CONFIG_FILE" <<EOL
gemini_api_key: "${GEMINI_API_KEY}"
signal_number: "${SIGNAL_NUMBER}"
authorized_numbers: []
require_confirmation: false
confirmation_timeout: 60
allow_commands: true
deny_by_default: false
allowed_commands: []
blocked_commands:
  - "rm -rf /"
  - "mkfs"
  - "dd"
dry_run: false
ssh_verify_host_keys: true
allowed_ssh_hosts: []
ssh_timeout: 30
logging:
  level: "info"
  json: false
EOL
fi

# Download latest binary from GitHub Releases
echo "📦 Downloading latest Clide binary..."

LATEST_URL=$(curl -fsSL "https://api.github.com/repos/$GITHUB_REPO/releases/latest" | \
  grep "browser_download_url" | grep "$ARCH" | cut -d '"' -f 4)

if [ -z "$LATEST_URL" ]; then
    echo "❌ Failed to fetch latest release for $ARCH"
    exit 1
fi

curl -fsSL "$LATEST_URL" -o "$TMP_DIR/$BINARY_NAME"
chmod +x "$TMP_DIR/$BINARY_NAME"
mv "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"

echo "✅ Clide installed to $INSTALL_DIR/$BINARY_NAME"
echo "📚 Config file: $CONFIG_FILE"
echo "🎉 Run 'clide start' to launch the bot"
