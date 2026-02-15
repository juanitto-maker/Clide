#!/bin/bash
# ============================================
# clide - One-Liner Remote Installer
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
PLANE="✈️"
CHECK="✅"
CROSS="❌"

# ============================================
# Configuration
# ============================================

REPO_URL="https://github.com/yourusername/clide.git"
INSTALL_DIR="$HOME/clide"

# ============================================
# Helper Functions
# ============================================

print_header() {
    echo ""
    echo -e "${CYAN}════════════════════════════════════════${NC}"
    echo -e "${CYAN}$1${NC}"
    echo -e "${CYAN}════════════════════════════════════════${NC}"
    echo ""
}

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

# ============================================
# Platform Detection
# ============================================

detect_platform() {
    if [ -d "/data/data/com.termux" ]; then
        PLATFORM="termux"
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        PLATFORM="linux"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        PLATFORM="macos"
    else
        print_error "Unsupported platform: $OSTYPE"
    fi
}

# ============================================
# Main Installation
# ============================================

main() {
    print_header "${PLANE} clide - One-Liner Installer ${PLANE}"
    
    print_info "Glide through your CLI - Installing..."
    echo ""
    
    # Detect platform
    detect_platform
    print_info "Platform: $PLATFORM"
    
    # Check if git is installed
    if ! command -v git &> /dev/null; then
        print_error "Git is not installed. Please install it first."
    fi
    
    # Clone repository
    print_info "Cloning repository..."
    if [ -d "$INSTALL_DIR" ]; then
        print_info "Directory exists, pulling latest changes..."
        cd "$INSTALL_DIR"
        git pull
    else
        git clone "$REPO_URL" "$INSTALL_DIR"
        cd "$INSTALL_DIR"
    fi
    print_success "Repository cloned"
    
    # Run setup script
    print_info "Running setup wizard..."
    chmod +x setup.sh
    ./setup.sh
    
    print_header "${CHECK} Installation Complete! ${CHECK}"
    print_success "${PLANE} clide is ready to fly! ${PLANE}"
    echo ""
    print_info "Next steps:"
    print_info "1. cd $INSTALL_DIR"
    print_info "2. Edit config.yaml with your settings"
    print_info "3. python src/clide.py"
    echo ""
}

# ============================================
# Run
# ============================================

main
