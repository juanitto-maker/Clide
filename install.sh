#!/bin/bash
# ============================================
# Clide - One-Line Installer
# ============================================
# Usage: curl -fsSL https://raw.githubusercontent.com/yourusername/clide/main/install.sh | bash
# ============================================

set -e  # Exit on error

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Emojis
CHECK="✅"
CROSS="❌"
PLANE="✈️"

# ============================================
# Configuration
# ============================================

REPO_OWNER="juanitto-maker"   # <--- Change this from "yourusername"
REPO_NAME="Clide"            # <--- Ensure this matches your repo name exactly
BINARY_NAME="clide"
GITHUB_REPO="https://github.com/${REPO_OWNER}/${REPO_NAME}"

# ============================================
# Helper Functions
# ============================================

print_success() {
    echo -e "${GREEN}${CHECK} $1${NC}"
}

print_error() {
    echo -e "${RED}${CROSS} $1${NC}"
    exit 1
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_step() {
    echo -e "${CYAN}${PLANE} $1${NC}"
}

# ============================================
# Platform Detection
# ============================================

detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)
    
    # Detect OS
    case "$os" in
        linux)
            if [ -d "/data/data/com.termux" ]; then
                OS="android"
            else
                OS="linux"
            fi
            ;;
        darwin)
            OS="darwin"
            ;;
        *)
            print_error "Unsupported OS: $os"
            ;;
    esac
    
    # Detect Architecture
    case "$arch" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        armv7l|armv7)
            ARCH="armv7"
            ;;
        *)
            print_error "Unsupported architecture: $arch"
            ;;
    esac
    
    # Build target triple
    if [ "$OS" = "android" ]; then
        TARGET="${ARCH}-linux-android"
    elif [ "$OS" = "linux" ]; then
        TARGET="${ARCH}-unknown-linux-gnu"
    elif [ "$OS" = "darwin" ]; then
        TARGET="${ARCH}-apple-darwin"
    fi
}

# ============================================
# Installation
# ============================================

install_binary() {
    print_step "Downloading clide for ${TARGET}..."
    
    # Get latest release version
    LATEST_URL="${GITHUB_REPO}/releases/latest"
    VERSION=$(curl -sI "$LATEST_URL" | grep -i "location:" | sed 's/.*tag\///' | tr -d '\r\n')
    
    if [ -z "$VERSION" ]; then
        print_error "Failed to get latest version"
    fi
    
    print_info "Latest version: ${VERSION}"
    
    # Build download URL
    BINARY_FILE="${BINARY_NAME}-${TARGET}"
    DOWNLOAD_URL="${GITHUB_REPO}/releases/download/${VERSION}/${BINARY_FILE}"
    
    # Determine install location
    if [ "$OS" = "android" ]; then
        INSTALL_DIR="$HOME/.local/bin"
    else
        INSTALL_DIR="/usr/local/bin"
    fi
    
    # Create install directory if needed
    mkdir -p "$INSTALL_DIR"
    
    # Download binary
    TMP_FILE="/tmp/${BINARY_NAME}.tmp"
    
    if command -v wget &> /dev/null; then
        wget -q --show-progress "$DOWNLOAD_URL" -O "$TMP_FILE" || print_error "Download failed"
    elif command -v curl &> /dev/null; then
        curl -L --progress-bar "$DOWNLOAD_URL" -o "$TMP_FILE" || print_error "Download failed"
    else
        print_error "Neither wget nor curl found. Please install one of them."
    fi
    
    # Make executable
    chmod +x "$TMP_FILE"
    
    # Move to install location
    if [ "$OS" = "android" ]; then
        mv "$TMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"
    else
        if [ -w "$INSTALL_DIR" ]; then
            mv "$TMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"
        else
            sudo mv "$TMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"
        fi
    fi
    
    print_success "Binary installed to ${INSTALL_DIR}/${BINARY_NAME}"
}

# ============================================
# Post-Installation
# ============================================

setup_path() {
    if [ "$OS" = "android" ]; then
        # Add to PATH in Termux
        if ! grep -q "$HOME/.local/bin" "$HOME/.bashrc" 2>/dev/null; then
            echo 'export PATH=$HOME/.local/bin:$PATH' >> "$HOME/.bashrc"
            print_info "Added to PATH in ~/.bashrc"
        fi
    fi
}

verify_installation() {
    print_step "Verifying installation..."
    
    if [ "$OS" = "android" ]; then
        export PATH="$HOME/.local/bin:$PATH"
    fi
    
    if command -v clide &> /dev/null; then
        VERSION=$(clide --version 2>/dev/null | head -n1)
        print_success "Installation verified: ${VERSION}"
        return 0
    else
        print_error "Installation failed - binary not found in PATH"
    fi
}

print_next_steps() {
    echo ""
    echo -e "${CYAN}════════════════════════════════════════${NC}"
    echo -e "${GREEN}${CHECK} Installation Complete! ${CHECK}${NC}"
    echo -e "${CYAN}════════════════════════════════════════${NC}"
    echo ""
    
    if [ "$OS" = "android" ]; then
        echo -e "${YELLOW}⚠️  Termux users: Reload your shell or run:${NC}"
        echo "   source ~/.bashrc"
        echo ""
    fi
    
    echo "Next steps:"
    echo ""
    echo "1. Install Signal CLI:"
    echo "   See: ${GITHUB_REPO}#signal-cli-setup"
    echo ""
    echo "2. Create configuration:"
    echo "   mkdir -p ~/.clide"
    echo "   clide init"
    echo ""
    echo "3. Add your API key:"
    echo "   clide config set-gemini-key YOUR_KEY"
    echo ""
    echo "4. Start the bot:"
    echo "   clide start"
    echo ""
    echo "Documentation: ${GITHUB_REPO}#documentation"
    echo "Issues: ${GITHUB_REPO}/issues"
    echo ""
    echo -e "${GREEN}Happy gliding! ${PLANE}${NC}"
    echo ""
}

# ============================================
# Main
# ============================================

main() {
    echo ""
    echo -e "${CYAN}══════════════════════════════════════${NC}"
    echo -e "${CYAN}  ${PLANE} Clide Installer ${PLANE}${NC}"
    echo -e "${CYAN}══════════════════════════════════════${NC}"
    echo ""
    
    # Detect platform
    detect_platform
    print_info "Detected: ${OS} / ${ARCH}"
    print_info "Target: ${TARGET}"
    echo ""
    
    # Install binary
    install_binary
    
    # Setup PATH
    setup_path
    
    # Verify
    verify_installation
    
    # Show next steps
    print_next_steps
}

# Run
main
