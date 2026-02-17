#!/bin/bash

echo "✨ Starting Clide Installation..."

# 1. Install dependencies without blocking (-y is key for Termux)
if ! command -v cargo &> /dev/null; then
    echo "🦀 Installing Rust..."
    pkg install rust -y
fi

if ! command -v git &> /dev/null; then
    echo "📦 Installing Git..."
    pkg install git -y
fi

# 2. Smart Directory Handling
REPO_NAME="Clide"
if [ ! -f "Cargo.toml" ]; then
    if [ -d "$REPO_NAME" ]; then
        echo "📂 Moving into existing $REPO_NAME directory..."
        cd "$REPO_NAME" || exit 1
    else
        echo "🌐 Cloning repository..."
        git clone https://github.com/juanitto-maker/Clide.git
        cd "$REPO_NAME" || exit 1
    fi
fi

# 3. Build the project (Now that we are in the right folder)
echo "🚀 Building Clide from source..."
cargo build --release

# 4. Handle API Key (Optional)
echo ""
echo "🔑 Gemini API Key Setup"
echo "Paste your key and press Enter (or just press Enter to SKIP):"
read -r api_key

mkdir -p ~/.config/clide
if [ -n "$api_key" ]; then
    echo "GEMINI_API_KEY=$api_key" > ~/.config/clide/config.env
    echo "✅ Key saved to ~/.config/clide/config.env"
else
    echo "⚠️  Skipped. Add your key manually later."
fi

# 5. Move to Path
if [ -f "target/release/clide" ]; then
    cp target/release/clide "$PREFIX/bin/"
    chmod +x "$PREFIX/bin/clide"
    echo "✨ Done! Type 'clide' to start."
else
    echo "❌ Build failed. Cargo.toml was not found or build crashed."
fi
