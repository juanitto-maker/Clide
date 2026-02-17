#!/data/data/com.termux/files/usr/bin/bash
set -e

echo "✨ Installing Clide..."

# --- Ensure dependencies ---
echo "📦 Checking dependencies..."
pkg install -y git rust clang

# --- Ask for configuration ---
echo ""
read -p "🔑 Enter your Gemini API key: " GEMINI_API_KEY

while [ -z "$GEMINI_API_KEY" ]; do
  echo "❌ API key cannot be empty"
  read -p "🔑 Enter your Gemini API key: " GEMINI_API_KEY
done

echo ""
read -p "📱 Enter your Signal phone number (E.164, ex: +34123456789): " SIGNAL_NUMBER

while [ -z "$SIGNAL_NUMBER" ]; do
  echo "❌ Signal number cannot be empty"
  read -p "📱 Enter your Signal phone number: " SIGNAL_NUMBER
done

# --- Paths ---
INSTALL_DIR="$HOME/Clide_Source"
BIN_DIR="$PREFIX/bin"

# --- Clean old install ---
echo "🧹 Cleaning old installation..."
rm -rf "$INSTALL_DIR"
rm -f "$BIN_DIR/clide"
rm -rf "$HOME/.clide"

# --- Clone repo ---
echo "📥 Cloning repository..."
git clone https://github.com/juanitto-maker/Clide.git "$INSTALL_DIR"

cd "$INSTALL_DIR"

# --- Write config ---
echo "⚙️  Writing configuration..."
mkdir -p "$HOME/.clide"

cat > "$HOME/.clide/config.toml" <<EOF
gemini_api_key = "$GEMINI_API_KEY"
gemini_model = "gemini-2.5-flash"
signal_number = "$SIGNAL_NUMBER"

require_confirmation = true
confirmation_timeout = 30

authorized_numbers = ["$SIGNAL_NUMBER"]

[logging]
level = "info"
EOF

# --- Build ---
echo "🦀 Building Clide..."
cargo build --release

# --- Install binary ---
echo "🚀 Installing binary..."
cp target/release/clide "$BIN_DIR/clide"
chmod +x "$BIN_DIR/clide"

echo ""
echo "✅ Clide installed successfully!"
echo "👉 Run with: clide"
echo ""
