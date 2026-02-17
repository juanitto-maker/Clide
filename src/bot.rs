#!/usr/bin/env bash
# ============================================
# Clide Installer (Updated for full automated setup)
# ============================================

set -euo pipefail

echo "✨ Installing Clide..."

# --- Step 1: Ask for API key and Signal number ---
read -rp "Enter your Gemini API Key (required): " GEMINI_API_KEY
if [ -z "$GEMINI_API_KEY" ]; then
    echo "❌ API key is required. Exiting."
    exit 1
fi

read -rp "Enter your Signal number (e.g., +1234567890, required): " SIGNAL_NUMBER
if [ -z "$SIGNAL_NUMBER" ]; then
    echo "❌ Signal number is required. Exiting."
    exit 1
fi

# --- Step 2: Install Rust if missing ---
if ! command -v cargo &>/dev/null; then
    echo "⚙️  Rust not found. Installing Rust..."
    pkg update -y
    pkg install -y rust
fi

# --- Step 3: Clone the repo ---
CLIDE_SRC="$HOME/Clide_Source"
if [ -d "$CLIDE_SRC" ]; then
    echo "🗑️  Removing existing Clide source..."
    rm -rf "$CLIDE_SRC"
fi

echo "📦 Cloning Clide repository..."
git clone https://github.com/juanitto-maker/Clide.git "$CLIDE_SRC"

# --- Step 4: Build Clide ---
cd "$CLIDE_SRC"
echo "🔨 Building Clide..."
cargo build --release

# --- Step 5: Install binary ---
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
cp target/release/clide "$BIN_DIR/"
chmod +x "$BIN_DIR/clide"

echo "✅ Clide binary installed to $BIN_DIR/clide"

# --- Step 6: Setup configuration directory ---
CONFIG_DIR="$HOME/.clide"
mkdir -p "$CONFIG_DIR"
CONFIG_FILE="$CONFIG_DIR/config.yaml"

cat > "$CONFIG_FILE" <<EOF
gemini_api_key: "${GEMINI_API_KEY}"
gemini_model: "gemini-2.5-flash"
signal_number: "${SIGNAL_NUMBER}"

# Security
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

# Signal authorized numbers
authorized_numbers: []

# SSH
ssh_verify_host_keys: true
allowed_ssh_hosts: []
ssh_timeout: 30

# Logging
logging:
  level: "info"
  json: false
EOF

echo "✅ Configuration created at $CONFIG_FILE"

# --- Step 7: Final message ---
echo ""
echo "🎉 Clide installation complete!"
echo "You can now run:"
echo "  clide test-gemini 'hello'   # Test Gemini API"
echo "  clide status                # Check system"
echo "  clide start                 # Start Signal bot"
echo ""
echo "Config file: $CONFIG_FILE"
echo ""
