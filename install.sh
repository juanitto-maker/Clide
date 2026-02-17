#!/usr/bin/env bash
# ============================================
# install.sh - Clide Installer
# ============================================

set -euo pipefail

echo "✨ Installing Clide..."

# --- User input: Gemini API key ---
read -rp "Enter your Gemini API key (or leave empty to skip): " GEMINI_API_KEY
if [[ -z "$GEMINI_API_KEY" ]]; then
    GEMINI_API_KEY="__SKIP__"
fi

# --- User input: Signal number ---
read -rp "Enter your Signal number (e.g., +1234567890): " SIGNAL_NUMBER
if [[ -z "$SIGNAL_NUMBER" ]]; then
    echo "❌ Signal number is required!"
    exit 1
fi

# --- Prepare directories ---
INSTALL_DIR="$HOME/Clide_Source"
CONFIG_DIR="$HOME/.clide"
BIN_DIR="$HOME/.local/bin"

mkdir -p "$INSTALL_DIR"
mkdir -p "$CONFIG_DIR"
mkdir -p "$BIN_DIR"

# --- Download source code ---
echo "📦 Downloading Clide source code..."
cd "$HOME"
if command -v git >/dev/null 2>&1; then
    git clone --depth 1 https://github.com/juanitto-maker/Clide.git Clide_Source || true
else
    echo "⚠️ Git not found, downloading ZIP..."
    curl -fsSL -o Clide.zip https://github.com/juanitto-maker/Clide/archive/refs/heads/main.zip
    unzip -o Clide.zip -d .
    mv Clide-main Clide_Source
fi

cd "$INSTALL_DIR"

# --- Write config.yaml ---
CONFIG_FILE="$CONFIG_DIR/config.yaml"
echo "📝 Writing default configuration..."
cat > "$CONFIG_FILE" <<EOF
gemini_api_key: "${GEMINI_API_KEY}"
gemini_model: "gemini-2.5-flash"

signal_number: "${SIGNAL_NUMBER}"

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

ssh_key_path: null
ssh_verify_host_keys: true
allowed_ssh_hosts: []
ssh_timeout: 30

logging:
  level: "info"
  file_path: null
  json: false
  with_timestamps: true
  with_caller: false
EOF

chmod 600 "$CONFIG_FILE"

# --- Ensure Rust toolchain ---
if ! command -v cargo >/dev/null 2>&1; then
    echo "📦 Rust not found, installing Rust..."
    pkg install -y rust
fi

# --- Build Clide ---
echo "🔨 Building Clide..."
cargo build --release

# --- Install binary ---
echo "🚀 Installing Clide binary..."
cp -f target/release/clide "$BIN_DIR/clide"
chmod +x "$BIN_DIR/clide"

# --- Success message ---
echo ""
echo "🎉 Clide installation complete!"
echo "Binary: $BIN_DIR/clide"
echo "Config: $CONFIG_FILE"
echo ""
echo "Try these commands:"
echo "  clide test-gemini 'hello'   # Test Gemini API"
echo "  clide status                # Check system"
echo "  clide start                 # Start Signal bot"
echo ""
