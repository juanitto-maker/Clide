#!/bin/bash
# ============================================
# Clide - Cross-Platform Build Script
# ============================================
# Builds binaries for all supported platforms
# ============================================

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

# ============================================
# Configuration
# ============================================

BINARY_NAME="clide"
VERSION=$(grep '^version' Cargo.toml | head -n1 | cut -d'"' -f2)
DIST_DIR="dist"

# Target platforms
TARGETS=(
    "x86_64-unknown-linux-gnu"      # Linux x64
    "aarch64-unknown-linux-gnu"     # Linux ARM64
    "x86_64-apple-darwin"           # macOS Intel
    "aarch64-apple-darwin"          # macOS Apple Silicon
    "aarch64-linux-android"         # Android/Termux
)

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

print_step() {
    echo -e "${BLUE}▶ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# ============================================
# Setup
# ============================================

setup_environment() {
    print_header "Setting Up Build Environment"
    
    # Check if cargo is installed
    if ! command -v cargo &> /dev/null; then
        echo "Error: Cargo not found. Install Rust from https://rustup.rs/"
        exit 1
    fi
    
    print_step "Rust version: $(rustc --version)"
    print_step "Cargo version: $(cargo --version)"
    
    # Install cross-compilation tool
    if ! command -v cross &> /dev/null; then
        print_step "Installing cross for cross-compilation..."
        cargo install cross --git https://github.com/cross-rs/cross
    fi
    
    # Create dist directory
    mkdir -p "$DIST_DIR"
    
    print_success "Environment ready"
}

# ============================================
# Build Functions
# ============================================

build_target() {
    local target=$1
    local use_cross=$2
    
    print_step "Building for ${target}..."
    
    if [ "$use_cross" = "true" ]; then
        cross build --release --target "$target"
    else
        cargo build --release --target "$target"
    fi
    
    # Copy binary to dist directory
    local ext=""
    if [[ "$target" == *"windows"* ]]; then
        ext=".exe"
    fi
    
    local src_binary="target/${target}/release/${BINARY_NAME}${ext}"
    local dst_binary="${DIST_DIR}/${BINARY_NAME}-${target}${ext}"
    
    if [ -f "$src_binary" ]; then
        cp "$src_binary" "$dst_binary"
        
        # Strip binary to reduce size (except on macOS)
        if [[ "$target" != *"darwin"* ]] && [[ "$target" != *"windows"* ]]; then
            strip "$dst_binary" 2>/dev/null || true
        fi
        
        # Show binary size
        local size=$(du -h "$dst_binary" | cut -f1)
        print_success "Built ${target} (${size})"
    else
        print_warning "Binary not found: $src_binary"
        return 1
    fi
}

build_all() {
    print_header "Building for All Platforms"
    
    local success_count=0
    local fail_count=0
    
    for target in "${TARGETS[@]}"; do
        # Determine if we need cross-compilation
        local use_cross="false"
        local current_target=$(rustc -vV | grep host | cut -d' ' -f2)
        
        if [ "$target" != "$current_target" ]; then
            use_cross="true"
        fi
        
        if build_target "$target" "$use_cross"; then
            ((success_count++))
        else
            ((fail_count++))
        fi
        
        echo ""
    done
    
    print_header "Build Summary"
    echo "✓ Successful: $success_count"
    echo "✗ Failed: $fail_count"
    echo ""
}

# ============================================
# Packaging
# ============================================

create_checksums() {
    print_header "Creating Checksums"
    
    cd "$DIST_DIR"
    
    # Create SHA256 checksums
    sha256sum ${BINARY_NAME}-* > checksums.sha256
    
    print_success "Checksums created: checksums.sha256"
    
    cd ..
}

create_archives() {
    print_header "Creating Archives"
    
    cd "$DIST_DIR"
    
    for binary in ${BINARY_NAME}-*; do
        # Skip checksum file
        if [[ "$binary" == *.sha256 ]]; then
            continue
        fi
        
        # Create tar.gz archive
        local archive="${binary}.tar.gz"
        tar czf "$archive" "$binary"
        
        print_success "Created: $archive"
    done
    
    cd ..
}

# ============================================
# Package-Specific Builds
# ============================================

build_deb() {
    print_header "Building .deb Package"
    
    if ! command -v cargo-deb &> /dev/null; then
        print_step "Installing cargo-deb..."
        cargo install cargo-deb
    fi
    
    cargo deb --target x86_64-unknown-linux-gnu
    
    # Copy to dist
    local deb_file=$(find target/x86_64-unknown-linux-gnu/debian -name "*.deb" | head -n1)
    if [ -f "$deb_file" ]; then
        cp "$deb_file" "$DIST_DIR/"
        print_success "Created: $(basename $deb_file)"
    fi
}

build_rpm() {
    print_header "Building .rpm Package"
    
    if ! command -v cargo-generate-rpm &> /dev/null; then
        print_step "Installing cargo-generate-rpm..."
        cargo install cargo-generate-rpm
    fi
    
    cargo build --release --target x86_64-unknown-linux-gnu
    cargo generate-rpm --target x86_64-unknown-linux-gnu
    
    # Copy to dist
    local rpm_file=$(find target/x86_64-unknown-linux-gnu/generate-rpm -name "*.rpm" | head -n1)
    if [ -f "$rpm_file" ]; then
        cp "$rpm_file" "$DIST_DIR/"
        print_success "Created: $(basename $rpm_file)"
    fi
}

# ============================================
# Verification
# ============================================

verify_binaries() {
    print_header "Verifying Binaries"
    
    cd "$DIST_DIR"
    
    for binary in ${BINARY_NAME}-*; do
        # Skip archives and checksums
        if [[ "$binary" == *.tar.gz ]] || [[ "$binary" == *.sha256 ]]; then
            continue
        fi
        
        if [ -f "$binary" ] && [ -x "$binary" ]; then
            local size=$(du -h "$binary" | cut -f1)
            echo "✓ $binary ($size)"
        else
            echo "✗ $binary (not executable)"
        fi
    done
    
    cd ..
    echo ""
}

# ============================================
# Upload to GitHub Releases (optional)
# ============================================

upload_release() {
    print_header "Upload to GitHub Releases"
    
    if [ -z "$GITHUB_TOKEN" ]; then
        print_warning "GITHUB_TOKEN not set. Skipping upload."
        print_warning "Set GITHUB_TOKEN to upload releases automatically."
        return
    fi
    
    if ! command -v gh &> /dev/null; then
        print_warning "GitHub CLI (gh) not found. Skipping upload."
        return
    fi
    
    print_step "Creating release v${VERSION}..."
    
    gh release create "v${VERSION}" \
        --title "v${VERSION}" \
        --generate-notes \
        ${DIST_DIR}/${BINARY_NAME}-* \
        ${DIST_DIR}/*.tar.gz \
        ${DIST_DIR}/checksums.sha256
    
    print_success "Release created!"
}

# ============================================
# Main Menu
# ============================================

show_menu() {
    echo ""
    echo "Clide Build Script v${VERSION}"
    echo ""
    echo "Options:"
    echo "  1) Build all targets"
    echo "  2) Build current platform only"
    echo "  3) Build Linux x64"
    echo "  4) Build Linux ARM64"
    echo "  5) Build macOS (both architectures)"
    echo "  6) Build Android/Termux"
    echo "  7) Create packages (.deb, .rpm)"
    echo "  8) Create archives"
    echo "  9) Verify binaries"
    echo " 10) Upload to GitHub Releases"
    echo "  0) Exit"
    echo ""
    read -p "Choose option: " choice
    
    case $choice in
        1) build_all ;;
        2) 
            local current=$(rustc -vV | grep host | cut -d' ' -f2)
            build_target "$current" "false"
            ;;
        3) build_target "x86_64-unknown-linux-gnu" "true" ;;
        4) build_target "aarch64-unknown-linux-gnu" "true" ;;
        5) 
            build_target "x86_64-apple-darwin" "true"
            build_target "aarch64-apple-darwin" "true"
            ;;
        6) build_target "aarch64-linux-android" "true" ;;
        7)
            build_deb
            build_rpm
            ;;
        8) create_archives ;;
        9) verify_binaries ;;
        10) upload_release ;;
        0) exit 0 ;;
        *) echo "Invalid option" ;;
    esac
}

# ============================================
# Main
# ============================================

main() {
    print_header "Clide Build System v${VERSION}"
    
    # Setup environment
    setup_environment
    
    # If arguments provided, run non-interactively
    if [ $# -gt 0 ]; then
        case "$1" in
            --all)
                build_all
                create_checksums
                create_archives
                verify_binaries
                ;;
            --release)
                build_all
                create_checksums
                create_archives
                build_deb
                build_rpm
                verify_binaries
                upload_release
                ;;
            --target)
                build_target "$2" "true"
                ;;
            --verify)
                verify_binaries
                ;;
            *)
                echo "Usage: $0 [--all|--release|--target TARGET|--verify]"
                exit 1
                ;;
        esac
    else
        # Interactive mode
        while true; do
            show_menu
        done
    fi
}

# Run
main "$@"
