#!/bin/bash
# ============================================
# clide - One-Liner Remote Installer
# ============================================
# Usage: 
#   curl -fsSL https://raw.githubusercontent.com/yourusername/clide/main/install.sh | bash
#
# Or with custom install directory:
#   curl -fsSL https://raw.githubusercontent.com/yourusername/clide/main/install.sh | bash -s -- /custom/path
# ============================================

set -e  # Exit on error

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
PURPLE='\033[0;35m'
NC='\033[0m'

# Emojis
PLANE="✈️"
CHECK="✅"
CROSS="❌"
GEAR="⚙️"

# ============================================
# Configuration
# ============================================

REPO_URL="https://github.com/juanitto-maker/Clide.git"
RAW_URL="https://raw.githubusercontent.com/juanitto-maker/Clide/main"
INSTALL_DIR="${1:-$HOME/clide}"

# ============================================
# ASCII Art
# ============================================

print_logo() {
    echo -e "${CYAN}"
    cat << "EOF"
    ✈️  ════════════════════════════════════ ✈️
    
         _____ _      _____ _____  ______ 
        /  ___| |    |_   _|  _  \ |  ___|
        | |   | |      | | | | | || |__   
        | |   | |      | | | | | ||  __|  
        \ \___| |____ _| |_| |/ / | |___  
         \____|______/\____/|___/|______|
    
        Glide through your CLI
        One-liner installer
    
    ✈️  ════════════════════════════════════ ✈️
EOF
    echo -e "${NC}"
}

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

print_step() {
    echo -e "${PURPLE}${GEAR} $1${NC}"
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
# Installation Methods
# ============================================

install_via_git() {
    print_step "Installing via Git..."
    
    if [ -d "$INSTALL_DIR" ]; then
        print_info "Directory exists, pulling latest changes..."
        cd "$INSTALL_DIR"
        git pull
    else
        git clone "$REPO_URL" "$INSTALL_DIR"
        cd "$INSTALL_DIR"
    fi
    
    print_success "Repository cloned"
}

install_via_curl() {
    print_step "Installing via direct download..."
    
    # Create directory structure
    mkdir -p "$INSTALL_DIR"/{src,docs}
    cd "$INSTALL_DIR"
    
    print_info "Downloading files..."
    
    # Root files
    FILES=(
        "README.md"
        "LICENSE"
        "CONTRIBUTING.md"
        "CHANGELOG.md"
        ".gitignore"
        "requirements.txt"
        "setup.sh"
        "config.example.yaml"
    )
    
    for file in "${FILES[@]}"; do
        curl -fsSL "$RAW_URL/$file" -o "$file" && echo "  ✓ $file" || echo "  ✗ $file (failed)"
    done
    
    # Docs files
    DOC_FILES=(
        "INSTALL.md"
        "SECURITY.md"
        "WORKFLOWS.md"
    )
    
    for file in "${DOC_FILES[@]}"; do
        curl -fsSL "$RAW_URL/docs/$file" -o "docs/$file" && echo "  ✓ docs/$file" || echo "  ✗ docs/$file (failed)"
    done
    
    # Source files
    SRC_FILES=(
        "__init__.py"
        "clide.py"
        "bot.py"
        "brain.py"
        "config.py"
        "executor.py"
        "logger.py"
        "memory.py"
        "safety.py"
    )
    
    for file in "${SRC_FILES[@]}"; do
        curl -fsSL "$RAW_URL/src/$file" -o "src/$file" && echo "  ✓ src/$file" || echo "  ✗ src/$file (failed)"
    done
    
    # Make scripts executable
    chmod +x setup.sh src/clide.py 2>/dev/null || true
    
    print_success "Files downloaded"
}

# ============================================
# Main Installation
# ============================================

main() {
    print_logo
    
    print_info "Installing to: $INSTALL_DIR"
    echo ""
    
    # Detect platform
    detect_platform
    print_info "Platform: $PLATFORM"
    echo ""
    
    # Choose installation method
    if command -v git &> /dev/null; then
        print_info "Git found - using git clone"
        install_via_git
    else
        print_info "Git not found - using direct download"
        install_via_curl
    fi
    
    # Run setup script
    print_header "Running Setup Wizard"
    
    if [ -f "$INSTALL_DIR/setup.sh" ]; then
        cd "$INSTALL_DIR"
        chmod +x setup.sh
        ./setup.sh
    else
        print_error "setup.sh not found"
    fi
    
    # Final message
    print_header "${CHECK} Installation Complete! ${CHECK}"
    
    echo -e "${GREEN}"
    echo "  🛫 clide is ready to fly!"
    echo ""
    echo "  Next steps:"
    echo "    1. cd $INSTALL_DIR"
    echo "    2. Edit config.yaml with your settings"
    echo "    3. python src/clide.py"
    echo ""
    echo "  Documentation:"
    echo "    - Installation: docs/INSTALL.md"
    echo "    - Workflows: docs/WORKFLOWS.md"
    echo "    - Security: docs/SECURITY.md"
    echo -e "${NC}"
}

# ============================================
# Run
# ============================================

main
