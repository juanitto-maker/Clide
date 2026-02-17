#!/usr/bin/env bash
set -e

echo "✨ Installing Clide..."

# -----------------------------
# Detect platform
# -----------------------------
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

if [[ "$OS" != "linux" ]]; then
  echo "❌ Unsupported OS: $OS"
  exit 1
fi

# -----------------------------
# Ask for configuration FIRST
# -----------------------------
echo ""
echo "🔑 Gemini API Key"
read -r -p "Enter Gemini API key: " GEMINI_API_KEY
if [[ -z "$GEMINI_API_KEY" ]]; then
  echo "❌ Gemini API key is required"
  exit 1
fi

echo ""
echo "📱 Signal bot number (E.164 format, e.g. +123456789)"
read -r -p "Enter Signal number: " SIGNAL_NUMBER
if [[ -z "$SIGNAL_NUMBER" ]]; then
  echo "❌ Signal number is required"
  exit 1
fi

# -----------------------------
# Paths
# -----------------------------
INSTALL_DIR="$HOME/.clide"
BIN_DIR="$HOME/.local/bin"
CONFIG_FILE="$INSTALL_DIR/config.yaml"

mkdir -p "$INSTALL_DIR"
mkdir -p "$BIN_DIR"

# -----------------------------
# Write config.yaml
# -----------------------------
cat > "$CONFIG_FILE" <<EOF
# ============================================
# Clide Configuration File
# ============================================

gemini_api_key: "$GEMINI_API_KEY"
gemini_model: "gemini-2.5-flash"

signal_number: "$SIGNAL_NUMBER"

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

logging:
  level: "info"
  json: false
EOF

chmod 600 "$CONFIG_FILE"

# -----------------------------
# Install Rust if missing
# -----------------------------
if ! command -v cargo >/dev/null 2>&1; then
  echo "🦀 Installing Rust..."
  pkg install -y rust
fi

# -----------------------------
# Clone source
# -----------------------------
SRC_DIR="$HOME/Clide_Source"
rm -rf "$SRC_DIR"

echo "📦 Cloning Clide source..."
git clone https://github.com/juanitto-maker/Clide.git "$SRC_DIR"

# -----------------------------
# Build
# -----------------------------
echo "🔨 Building Clide..."
cd "$SRC_DIR"
cargo build --release

# -----------------------------
# Install binary
# -----------------------------
cp target/release/clide "$BIN_DIR/clide"
chmod +x "$BIN_DIR/clide"

# -----------------------------
# Final message
# -----------------------------
echo ""
echo "═══════════════════════════════════════"
echo "✨ Installation Complete!"
echo "═══════════════════════════════════════"
echo ""
echo "🎉 Clide is ready to use!"
echo ""
echo "Try:"
echo "  clide test-gemini \"hello\""
echo "  clide start"
echo ""
echo "⚙️  Config: $CONFIG_FILE"
echo ""
