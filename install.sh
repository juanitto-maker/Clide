#!/usr/bin/env bash
# ============================================
# Clide Installer - Full Setup
# ============================================

set -euo pipefail

echo "✨ Installing Clide..."

# -----------------------------
# Step 1: Ask for user input
# -----------------------------
read -rp "Enter your Gemini API Key (or leave empty to skip): " GEMINI_API_KEY
read -rp "Enter your Signal number (e.g. +1234567890): " SIGNAL_NUMBER

# -----------------------------
# Step 2: Setup directories
# -----------------------------
CLIDE_DIR="$HOME/Clide_Source"
CONFIG_DIR="$HOME/.clide"
BIN_DIR="$HOME/.local/bin"

mkdir -p "$CLIDE_DIR" "$CONFIG_DIR" "$BIN_DIR"

# -----------------------------
# Step 3: Create config.yaml
# -----------------------------
CONFIG_FILE="$CONFIG_DIR/config.yaml"

cat > "$CONFIG_FILE" <<EOF
gemini_api_key: "$GEMINI_API_KEY"
gemini_model: "gemini-2.5-flash"
signal_number: "$SIGNAL_NUMBER"

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

ssh_key_path: "~/.ssh/id_rsa"
ssh_verify_host_keys: true
allowed_ssh_hosts: []
ssh_timeout: 30

logging:
  level: "info"
  file_path: "$CONFIG_DIR/logs/clide.log"
  json: false

authorized_numbers: []
EOF

echo "✅ Configuration saved to $CONFIG_FILE"

# -----------------------------
# Step 4: Check Rust/Cargo
# -----------------------------
if ! command -v cargo >/dev/null 2>&1; then
    echo "⚠️ Rust/Cargo not found. Installing Rust via pkg..."
    pkg install -y rust
fi

# -----------------------------
# Step 5: Clone or update repo
# -----------------------------
if [ ! -d "$CLIDE_DIR/.git" ]; then
    echo "📥 Cloning Clide repository..."
    git clone https://github.com/juanitto-maker/Clide.git "$CLIDE_DIR"
else
    echo "♻️ Updating Clide repository..."
    git -C "$CLIDE_DIR" pull
fi

# -----------------------------
# Step 6: Build the binary
# -----------------------------
echo "🔨 Building Clide binary..."
cd "$CLIDE_DIR"
cargo build --release

# -----------------------------
# Step 7: Install binary
# -----------------------------
echo "📂 Installing binary to $BIN_DIR"
cp "$CLIDE_DIR/target/release/clide" "$BIN_DIR/"
chmod +x "$BIN_DIR/clide"

echo "✨ Clide installed successfully!"
echo "Binary: $BIN_DIR/clide"
echo "Config: $CONFIG_FILE"

# -----------------------------
# Step 8: Final instructions
# -----------------------------
echo ""
echo "📱 Signal Bot Setup:"
echo "   1. Link device: signal-cli link -n \"clide-bot\""
echo "   2. Scan QR code with Signal app"
echo "   3. Start bot: clide start"
echo ""
echo "🎉 Happy hacking!"
