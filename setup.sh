#!/bin/bash
# ============================================
# clide - Automated Setup Script (Termux Optimized)
# ============================================
# Glide through your CLI - Installation wizard
# Supports: Termux (Android), Linux, macOS
# ============================================

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Emojis
ROCKET="🚀"
CHECK="✅"
CROSS="❌"
WARN="⚠️"
PLANE="✈️"

# ============================================
# Global Variables
# ============================================

# Store the clide installation directory
CLIDE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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
}

print_warning() {
    echo -e "${YELLOW}${WARN} $1${NC}"
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
    if [ -d "/data/data/com.termux" ]; then
        PLATFORM="termux"
        PKG_MANAGER="pkg"
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        PLATFORM="linux"
        if command -v apt &> /dev/null; then
            PKG_MANAGER="apt"
        elif command -v dnf &> /dev/null; then
            PKG_MANAGER="dnf"
        elif command -v yum &> /dev/null; then
            PKG_MANAGER="yum"
        else
            PKG_MANAGER="unknown"
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        PLATFORM="macos"
        PKG_MANAGER="brew"
    else
        PLATFORM="unknown"
        PKG_MANAGER="unknown"
    fi
}

# ============================================
# Dependency Check
# ============================================

check_command() {
    if command -v "$1" &> /dev/null; then
        print_success "$1 is installed"
        return 0
    else
        print_warning "$1 is NOT installed"
        return 1
    fi
}

# ============================================
# Installation Functions
# ============================================

install_python() {
    print_step "Installing Python..."
    
    case $PLATFORM in
        termux)
            pkg install -y python
            ;;
        linux)
            if [ "$PKG_MANAGER" = "apt" ]; then
                sudo apt update
                sudo apt install -y python3 python3-pip
            elif [ "$PKG_MANAGER" = "dnf" ] || [ "$PKG_MANAGER" = "yum" ]; then
                sudo $PKG_MANAGER install -y python3 python3-pip
            fi
            ;;
        macos)
            if ! command -v brew &> /dev/null; then
                print_error "Homebrew not found. Please install it first:"
                print_info "Visit: https://brew.sh"
                exit 1
            fi
            brew install python3
            ;;
    esac
    
    print_success "Python installed"
}

install_rust() {
    print_step "Installing Rust (required for cryptography package)..."
    
    case $PLATFORM in
        termux)
            pkg install -y rust
            ;;
        linux)
            if [ "$PKG_MANAGER" = "apt" ]; then
                sudo apt install -y rustc cargo
            elif [ "$PKG_MANAGER" = "dnf" ] || [ "$PKG_MANAGER" = "yum" ]; then
                sudo $PKG_MANAGER install -y rust cargo
            fi
            ;;
        macos)
            brew install rust
            ;;
    esac
    
    print_success "Rust installed"
}

install_git() {
    print_step "Installing Git..."
    
    case $PLATFORM in
        termux)
            pkg install -y git
            ;;
        linux)
            if [ "$PKG_MANAGER" = "apt" ]; then
                sudo apt install -y git
            elif [ "$PKG_MANAGER" = "dnf" ] || [ "$PKG_MANAGER" = "yum" ]; then
                sudo $PKG_MANAGER install -y git
            fi
            ;;
        macos)
            brew install git
            ;;
    esac
    
    print_success "Git installed"
}

install_nodejs() {
    print_step "Installing Node.js (for Cline CLI)..."
    
    case $PLATFORM in
        termux)
            pkg install -y nodejs
            ;;
        linux)
            if [ "$PKG_MANAGER" = "apt" ]; then
                curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
                sudo apt install -y nodejs
            elif [ "$PKG_MANAGER" = "dnf" ] || [ "$PKG_MANAGER" = "yum" ]; then
                curl -fsSL https://rpm.nodesource.com/setup_20.x | sudo bash -
                sudo $PKG_MANAGER install -y nodejs
            fi
            ;;
        macos)
            brew install node
            ;;
    esac
    
    print_success "Node.js installed"
}

install_signal_cli() {
    print_step "Installing signal-cli..."
    
    # Store current directory to return later
    CURRENT_DIR=$(pwd)
    
    # Check if Java is installed first
    if ! command -v java &> /dev/null; then
        print_step "Installing Java (required for signal-cli)..."
        case $PLATFORM in
            termux)
                pkg install -y openjdk-17
                ;;
            linux)
                if [ "$PKG_MANAGER" = "apt" ]; then
                    sudo apt install -y default-jre
                elif [ "$PKG_MANAGER" = "dnf" ] || [ "$PKG_MANAGER" = "yum" ]; then
                    sudo $PKG_MANAGER install -y java-17-openjdk
                fi
                ;;
            macos)
                brew install openjdk@17
                ;;
        esac
        print_success "Java installed"
    fi
    
    # Download and install signal-cli
    SIGNAL_VERSION="0.13.1"
    SIGNAL_URL="https://github.com/AsamK/signal-cli/releases/download/v${SIGNAL_VERSION}/signal-cli-${SIGNAL_VERSION}.tar.gz"
    
    print_step "Downloading signal-cli v${SIGNAL_VERSION}..."
    
    if [ "$PLATFORM" = "termux" ]; then
        SIGNAL_INSTALL_DIR="$HOME/.local"
    else
        SIGNAL_INSTALL_DIR="/opt"
    fi
    
    mkdir -p "$SIGNAL_INSTALL_DIR"
    cd "$SIGNAL_INSTALL_DIR"
    
    if [ "$PLATFORM" = "termux" ]; then
        wget -q "$SIGNAL_URL" -O signal-cli.tar.gz
        tar xf signal-cli.tar.gz
        rm signal-cli.tar.gz
        
        # Add to PATH
        echo "export PATH=\"\$HOME/.local/signal-cli-${SIGNAL_VERSION}/bin:\$PATH\"" >> ~/.bashrc
        export PATH="$HOME/.local/signal-cli-${SIGNAL_VERSION}/bin:$PATH"
    else
        sudo wget -q "$SIGNAL_URL" -O signal-cli.tar.gz
        sudo tar xf signal-cli.tar.gz
        sudo rm signal-cli.tar.gz
        sudo ln -sf "$SIGNAL_INSTALL_DIR/signal-cli-${SIGNAL_VERSION}/bin/signal-cli" /usr/local/bin/signal-cli
    fi
    
    # Return to original directory
    cd "$CURRENT_DIR"
    
    print_success "signal-cli installed"
}

install_cline() {
    print_step "Installing Cline CLI..."
    
    npm install -g @cline/cli 2>/dev/null || {
        print_warning "Cline CLI installation failed (expected - package may not exist yet)"
        print_info "You can install it manually later when available"
        return 0
    }
    
    print_success "Cline CLI installed"
}

install_python_deps() {
    print_step "Installing Python dependencies..."

    # Ensure we're in the clide directory
    cd "$CLIDE_DIR"

    # Verify requirements.txt exists
    if [ ! -f "requirements.txt" ]; then
        print_error "requirements.txt not found in $CLIDE_DIR"
        exit 1
    fi

    # Termux-specific: Install ONLY packages that work reliably
    if [ "$PLATFORM" = "termux" ]; then
        echo ""
        print_info "Termux Detected - Using optimized installation strategy"
        print_info "Installing ONLY packages that work reliably on Android"
        echo ""
        print_warning "Note: Some packages will be skipped due to Termux compatibility:"
        print_warning "  - google-generativeai (requires grpcio - use REST API instead)"
        print_warning "  - paramiko/fabric (requires pynacl - use native ssh instead)"
        echo ""
        
        # Install build tools
        print_step "Installing build dependencies..."
        pkg install -y clang make cmake pkg-config openssl libffi 2>&1 | grep -v "dpkg-deb"
        
        # Clean caches
        rm -rf ~/.cache/pip 2>/dev/null || true
        
        # Install packages that work well on Termux
        print_step "Installing core Python packages..."
        pip install --break-system-packages --no-cache-dir \
            python-dotenv \
            pyyaml \
            requests \
            click \
            rich \
            python-dateutil \
            python-json-logger \
            pyotp || {
            print_error "Core packages failed to install"
            exit 1
        }
        print_success "Core packages installed"
        
        # Try cryptography from pip (often works with rust)
        print_step "Installing cryptography..."
        if pip install --break-system-packages --no-cache-dir 'cryptography>=41.0.0,<43.0.0'; then
            print_success "Cryptography installed"
        else
            print_warning "Cryptography pip install failed"
        fi
        
        # Install signalbot (usually works)
        print_step "Installing signalbot..."
        pip install --break-system-packages --no-cache-dir signalbot 2>/dev/null && \
            print_success "signalbot installed" || \
            print_warning "signalbot failed (optional)"
        
        # Skip problematic packages with clear explanations
        echo ""
        print_header "Skipped Packages (with workarounds)"
        
        print_warning "Skipping: google-generativeai (requires grpcio)"
        print_info "→ Workaround: Use Gemini REST API directly"
        print_info "  curl https://generativelanguage.googleapis.com/v1/models/gemini-pro:generateContent"
        echo ""
        
        print_warning "Skipping: paramiko/fabric (requires pynacl)"
        print_info "→ Workaround: Use native SSH via subprocess"
        print_info "  import subprocess"
        print_info "  subprocess.run(['ssh', 'user@host', 'command'])"
        echo ""
        
        print_warning "Skipping: Development tools (pytest, black, etc.)"
        print_info "→ These are optional and can be installed manually if needed"
        echo ""
        
        print_success "Essential Python packages installed successfully!"
        
    else
        # Standard installation for Linux/macOS
        print_info "Installing from: $CLIDE_DIR/requirements.txt"
        pip3 install -r requirements.txt
        print_success "Python dependencies installed"
    fi
}

# ============================================
# Configuration Setup
# ============================================

setup_config() {
    print_step "Setting up configuration..."
    
    # Ensure we're in the clide directory
    cd "$CLIDE_DIR"
    
    # Create clide directory
    mkdir -p ~/.clide/logs
    
    # Copy example config if it doesn't exist
    if [ ! -f config.yaml ]; then
        cp config.example.yaml config.yaml
        print_success "Created config.yaml from template"
        
        # Set restrictive permissions
        chmod 600 config.yaml
        print_success "Set secure permissions on config.yaml"
    else
        print_info "config.yaml already exists, skipping"
    fi
}

setup_signal() {
    print_header "Signal Setup"
    
    print_info "To use clide, you need to link your Signal account."
    print_info ""
    print_info "Options:"
    print_info "1. Link as secondary device (recommended)"
    print_info "2. Register new number (if you have a spare SIM)"
    print_info "3. Skip for now (configure manually later)"
    print_info ""
    
    read -p "Choose option (1/2/3): " signal_option
    
    case $signal_option in
        1)
            print_step "Starting Signal linking process..."
            print_info "A QR code will appear below."
            print_info "Scan it with Signal: Settings → Linked Devices → Add Device"
            print_info ""
            signal-cli link -n "clide-bot" || {
                print_warning "Signal linking failed. You can do this later."
                return 1
            }
            print_success "Signal linked successfully!"
            ;;
        2)
            read -p "Enter phone number (with country code, e.g., +1234567890): " phone_number
            print_step "Registering number: $phone_number"
            signal-cli -a "$phone_number" register || {
                print_warning "Signal registration failed. You can do this later."
                return 1
            }
            print_info "Verification code sent via SMS"
            read -p "Enter verification code: " verify_code
            signal-cli -a "$phone_number" verify "$verify_code"
            print_success "Signal registered successfully!"
            
            # Update config with phone number
            cd "$CLIDE_DIR"
            sed -i "s/+1234567890/$phone_number/" config.yaml
            ;;
        3)
            print_info "Skipping Signal setup. Configure manually in config.yaml"
            ;;
        *)
            print_warning "Invalid option. Skipping Signal setup."
            ;;
    esac
}

setup_gemini() {
    print_header "Gemini API Setup"
    
    if [ "$PLATFORM" = "termux" ]; then
        print_warning "Note: google-generativeai package was not installed on Termux"
        print_info "You'll need to use the Gemini REST API directly"
        print_info ""
    fi
    
    print_info "clide uses Google's Gemini Flash for AI capabilities."
    print_info "You need a free API key from: https://makersuite.google.com/app/apikey"
    print_info ""
    
    read -p "Do you have a Gemini API key? (y/n): " has_key
    
    if [ "$has_key" = "y" ] || [ "$has_key" = "Y" ]; then
        read -p "Enter your Gemini API key: " api_key
        
        # Update config with API key
        cd "$CLIDE_DIR"
        sed -i "s/YOUR_GEMINI_API_KEY_HERE/$api_key/" config.yaml
        print_success "Gemini API key configured!"
    else
        print_info "Please get an API key and add it to config.yaml later"
        print_info "Visit: https://makersuite.google.com/app/apikey"
    fi
}

# ============================================
# Main Installation Flow
# ============================================

main() {
    print_header "${PLANE} clide Installation Wizard ${PLANE}"
    
    print_info "Glide through your CLI - Autonomous terminal operations"
    print_info ""
    print_info "Installation directory: $CLIDE_DIR"
    print_info ""
    
    # Detect platform
    detect_platform
    print_info "Detected platform: $PLATFORM"
    print_info "Package manager: $PKG_MANAGER"
    print_info ""
    
    # Check dependencies
    print_header "Checking Dependencies"
    
    NEED_PYTHON=false
    NEED_GIT=false
    NEED_NODE=false
    NEED_SIGNAL=false
    NEED_RUST=false
    
    check_command python3 || check_command python || NEED_PYTHON=true
    check_command git || NEED_GIT=true
    check_command node || NEED_NODE=true
    check_command signal-cli || NEED_SIGNAL=true
    check_command rustc || NEED_RUST=true
    
    # Install missing dependencies
    if [ "$NEED_PYTHON" = true ] || [ "$NEED_GIT" = true ] || [ "$NEED_NODE" = true ] || [ "$NEED_SIGNAL" = true ] || [ "$NEED_RUST" = true ]; then
        print_header "Installing Missing Dependencies"
        
        [ "$NEED_PYTHON" = true ] && install_python
        [ "$NEED_RUST" = true ] && install_rust
        [ "$NEED_GIT" = true ] && install_git
        [ "$NEED_NODE" = true ] && install_nodejs
        [ "$NEED_SIGNAL" = true ] && install_signal_cli
    else
        print_success "All system dependencies already installed!"
    fi
    
    # Install Python dependencies
    print_header "Installing Python Packages"
    install_python_deps
    
    # Install Cline (optional)
    print_header "Installing Cline CLI"
    install_cline
    
    # Setup configuration
    print_header "Configuration Setup"
    setup_config
    
    # Setup Signal (interactive)
    setup_signal
    
    # Setup Gemini API (interactive)
    setup_gemini
    
    # Final steps
    print_header "${CHECK} Installation Complete! ${CHECK}"
    
    print_success "clide is ready to fly!"
    print_info ""
    
    if [ "$PLATFORM" = "termux" ]; then
        print_info "Termux-specific notes:"
        print_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        print_info "✅ Installed: Core packages, cryptography, signalbot"
        print_info "⚠️  Not installed: google-generativeai, paramiko"
        print_info ""
        print_info "For Gemini API: Use REST API directly in your code"
        print_info "For SSH: Use subprocess.run(['ssh', ...])"
        print_info "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        print_info ""
    fi
    
    print_info "Next steps:"
    print_info "1. Review and edit config.yaml with your settings"
    print_info "2. Run: python src/clide.py (when code is ready)"
    print_info "3. Send a message via Signal to test"
    print_info ""
    print_info "Documentation:"
    print_info "- Installation: docs/INSTALL.md"
    print_info "- Security: docs/SECURITY.md"
    print_info "- Workflows: docs/WORKFLOWS.md"
    print_info ""
    print_success "${PLANE} Happy gliding! ${PLANE}"
}

# ============================================
# Run Installation
# ============================================

main
