#!/bin/bash
# Clide Simple Installer - Builds from Source
set -e

echo "🚀 Installing Clide directly from source..."

# 1. Install Rust (if missing)
if ! command -v cargo >/dev/null 2>&1; then
    echo "📦 Rust not found. Installing..."
    # Check if Termux (Android)
    if [[ "$PREFIX" =~ "com.termux" ]]; then
        echo "📱 Installing Rust via Termux packages..."
        pkg install -y rust binutils pkg-config openssl
    else
        # Standard installation for Linux/macOS
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source $HOME/.cargo/env
    fi
fi

# Install build dependencies for Termux
if [[ "$PREFIX" =~ "com.termux" ]]; then
    echo "📦 Installing Termux build dependencies..."
    pkg install -y binutils git pkg-config openssl
fi

# 2. Clone the repo
echo "📂 Cloning Clide..."
git clone https://github.com/juanitto-maker/Clide.git $HOME/Clide_Source
cd $HOME/Clide_Source

# 3. Build it
echo "🛠️  Building (this may take a minute)..."
cargo build --release

# 4. Install it to Termux binary directory
echo "🚚 Moving to bin..."
cp target/release/clide $PREFIX/bin/

echo "✅ DONE! Restart Termux and type 'clide'"
