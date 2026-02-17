#!/bin/bash

echo "✨ Starting Clide Installation..."

# 1. Install dependencies without blocking
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust (this may take a minute)..."
    pkg install rust -y || apt install rustc cargo -y
fi

if ! command -v git &> /dev/null; then
    echo "📦 Installing Git..."
    pkg install git -y || apt install git -y
fi

# 2. Build the project
echo "🚀 Building Clide from source..."
if [ ! -d "Clide" ]; then
    git clone https://github.com/juanitto-maker/Clide.git
    cd Clide
fi

cargo build --release

# 3. Handle API Key (Optional & Non-blocking)
echo ""
echo "🔑 Gemini API Key Setup"
echo "Paste your key and press Enter, or just press Enter to SKIP:"
read -r api_key

mkdir -p ~/.config/clide

if [ -n "$api_key" ]; then
    echo "GEMINI_API_KEY=$api_key" > ~/.config/clide/config.env
    echo "✅ Key saved to ~/.config/clide/config.env"
else
    echo "⚠️  Skipped. Remember to add your key to ~/.config/clide/config.env later!"
fi

# 4. Install binary to path
if [ -f "target/release/clide" ]; then
    # For Termux
    if [ -n "$PREFIX" ]; then
        cp target/release/clide "$PREFIX/bin/"
        chmod +x "$PREFIX/bin/clide"
    # For Linux/macOS
    else
        sudo cp target/release/clide /usr/local/bin/ 2>/dev/null || cp target/release/clide ~/bin/
    fi
    echo "✨ Done! Type 'clide' to start."
else
    echo "❌ Build failed. Check cargo output above."
fi
