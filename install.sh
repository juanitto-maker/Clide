#!/bin/bash

# --- 1. Environment Check & Fix ---
echo "✨ Starting Clide Installation..."

# Prevent freezing by using -y for Termux/Debian
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    pkg install rust -y || apt install rustc cargo -y
fi

if ! command -v git &> /dev/null; then
    echo "📦 Installing Git..."
    pkg install git -y || apt install git -y
fi

# --- 2. Build Process ---
echo "🚀 Building Clide..."
# Build the release version
cargo build --release

# --- 3. Optional API Key Setup ---
echo ""
echo "🔑 Gemini API Key Setup"
echo "Paste your key and press Enter, or just press Enter to SKIP (you can add it later):"
read -r api_key

mkdir -p ~/.config/clide

if [ -n "$api_key" ]; then
    echo "GEMINI_API_KEY=$api_key" > ~/.config/clide/config.env
    echo "✅ Key saved to ~/.config/clide/config.env"
else
    echo "⚠️  Skipped. Add your key manually to ~/.config/clide/config.env to use Clide."
fi

# --- 4. Install the Command ---
if [ -f "target/release/clide" ]; then
    # In Termux, $PREFIX is /data/data/com.termux/files/usr
    if [ -n "$PREFIX" ]; then
        cp target/release/clide "$PREFIX/bin/"
        chmod +x "$PREFIX/bin/clide"
    else
        # For standard Linux
        sudo cp target/release/clide /usr/local/bin/ 2>/dev/null || cp target/release/clide ~/bin/
    fi
    echo "✨ Installation complete! You can now type 'clide' from anywhere."
else
    echo "❌ Build failed. Please check the errors above."
fi
