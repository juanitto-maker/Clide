#!/bin/bash
# ============================================
# Clide - One-Line Installer
# ============================================
# Usage: curl -fsSL https://raw.githubusercontent.com/juanitto-maker/Clide/main/install.sh | bash
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

REPO_OWNER="juanitto-maker"
REPO_NAME="Clide"
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
    
    case "$os" in
        linux)
            if [ -d "/data/data/com.termux" ]; then
                OS="android"
            else
                OS="linux"
            fi
            ;;
        darwin)
            OS="macos"
            ;;
        *)
            print_error "Unsupported operating system: $os"
            ;;
    esac
    
    case "$arch" in
        x86_64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            print_error "Unsupported architecture: $arch"
            ;;
    esac
    
    # Set target string
    case "$OS" in
        linux)   TARGET="${ARCH}-unknown-linux-gnu" ;;
        macos)   TARGET="${ARCH}-apple-darwin" ;;
        android) TARGET="aarch64-linux-android" ;;
    esac
    
    print_info "Detected: ${OS} / ${ARCH}"
    print_info "Target: ${TARGET}"
}

# ============================================
# Installation
# ============================================

install_binary() {
    print_step "Downloading clide for ${TARGET}..."
    
    # Manually set version until first full release automation is live
    VERSION="v0.1.0" 
    
    # Fix for Termux /tmp permission error
    if [ "$OS" = "android" ]; then
        TMP_FILE="${TMPDIR:-$HOME}/clide.tmp"
    else
        TMP_FILE="/tmp/clide.tmp"
    fi

    # Construct Download URL
    DOWNLOAD_URL="${GITHUB_REPO}/releases/download/${VERSION}/${BINARY_NAME}-${TARGET}"
    
    print_info "Latest version: ${VERSION}"
    
    # Download binary
    if ! curl -fL -o "$TMP_FILE" "$DOWNLOAD_URL"; then
        echo ""
        print_error "Download failed. Please ensure you have created a Release tagged '${VERSION}' and uploaded '${BINARY_NAME}-${TARGET}' to it."
    fi
    
    # Set installation directory
    if [ "$OS" = "android" ]; then
        INSTALL_DIR="$HOME/.local/bin"
    else
        INSTALL_DIR="/usr/local/bin"
    fi
    
    mkdir -p "$INSTALL_DIR"
    
    print_step "Installing to ${INSTALL_DIR}..."
    
    # Move and make executable
    if [ "$OS" = "android" ]; then
        mv "$TMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"
        chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    else
        if [ -w "$INSTALL_DIR" ]; then
            mv "$TMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"
        else
            sudo mv "$TMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"
        fi
        sudo chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    fi
    
    print_success "Installation complete!"
}

verify_installation() {
    print_step "Verifying installation..."
    
    if command -v clide >/dev/null 2>&1; then
        print_success "Clide is ready to use!"
    else
        print_info "Note: You may need to add ${INSTALL_DIR} to your PATH"
    fi
}

show_success_message() {
    echo ""
    echo -e "${CYAN}════════════════════════════════════════${NC}"
    echo -e "${GREEN}  Installation Complete! ${CHECK}${NC}"
    echo -e "${CYAN}════════════════════════════════════════${NC}"
    echo ""
    
    if [ "$OS" = "android" ]; then
        echo -e "${YELLOW}⚠️  Termux users: Reload your shell or run:${NC}"
        echo "   source ~/.bashrc"
        echo ""
    fi
    
    echo "Next steps:"
    echo ""
    echo "1. Create configuration:"
    echo "   mkdir -p ~/.clide"
    echo "   # Copy your config.yaml here"
    echo ""
    echo "2. Start the bot:"
    echo "   clide start"
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
    
    detect_platform
    install_binary
    verify_installation
    show_success_message
}

main "$@"
