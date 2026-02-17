#!/usr/bin/env bash
# ============================================
# Clide Installer - Option 2 (Build from Source)
# ============================================

set -euo pipefail

echo "✨ Installing Clide..."

# 1️⃣ Prompt for Gemini API Key
read -rp "Enter your Gemini API Key (or leave blank to skip): " GEMINI_API_KEY

# 2️⃣ Prompt for Signal Number
read -rp "Enter your Signal number (e.g., +1234567890): " SIGNAL_NUMBER

# 3️⃣ Set install paths
CLIDE_SRC="$HOME/Clide_Source"
CLIDE_BIN="$HOME/.local/bin"
CONFIG_DIR="$HOME/.clide"

mkdir -p "$CLIDE_SRC" "$CLIDE_BIN" "$CONFIG_DIR"

# 4️⃣ Clone or update repository
if [ -d "$CLIDE_SRC/.git" ]; then
    echo "🔄 Updating existing source..."
    git -C "$CLIDE_SRC" pull --rebase
else
    echo "📥 Cloning Clide repository..."
    git clone https://github.com/juanitto-maker/Clide.git "$CLIDE_SRC"
fi

# 5️⃣ Ensure Rust is installed
if ! command -v cargo >/dev/null 2>&1; then
    echo "⚠️ Rust is not installed. Installing..."
    pkg install -y rust
fi

# 6️⃣ Build Clide binary
echo "⚙️  Building Clide..."
cd "$CLIDE_SRC"
cargo build --release

# 7️⃣ Install binary
cp -f target/release/clide "$CLIDE_BIN/"
chmod +x "$CLIDE_BIN/clide"

# 8️⃣ Write default config.yaml
CONFIG_FILE="$CONFIG_DIR/config.yaml"
if [ ! -f "$CONFIG_FILE" ]; then
cat > "$CONFIG_FILE" <<EOL
gemini_api_key: "${GEMINI_API_KEY}"
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

# SSH
ssh_verify_host_keys: true
allowed_ssh_hosts: []
ssh_timeout: 30

# Logging
logging:
  level: "info"
  json: false
EOL
    echo "📝 Created default config at $CONFIG_FILE"
else
    echo "⚠️ Config already exists at $CONFIG_FILE, skipping creation"
fi

# 9️⃣ Add local bin to PATH if not already
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
    export PATH="$HOME/.local/bin:$PATH"
    echo "✅ Added $HOME/.local/bin to PATH"
fi

echo "🎉 Clide installation complete!"
echo "Try: clide test-gemini 'hello' or clide start"
