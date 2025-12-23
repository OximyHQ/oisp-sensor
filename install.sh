#!/bin/bash
#
# OISP Sensor Installation Script
#
# Usage:
#   curl -fsSL https://oisp.dev/install.sh | sh
#   wget -qO- https://oisp.dev/install.sh | sh
#

set -e

# Configuration
GITHUB_REPO="oximyHQ/oisp-sensor"
INSTALL_DIR="/usr/local/bin"
BIN_NAME="oisp-sensor"
VERSION="${OISP_VERSION:-latest}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[oisp]${NC} $1"
}

success() {
    echo -e "${GREEN}[oisp]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[oisp]${NC} $1"
}

error() {
    echo -e "${RED}[oisp]${NC} $1"
    exit 1
}

# Detect OS and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case "$OS" in
        linux)
            OS="linux"
            ;;
        darwin)
            OS="macos"
            ;;
        msys*|mingw*|cygwin*)
            OS="windows"
            ;;
        *)
            error "Unsupported operating system: $OS"
            ;;
    esac
    
    case "$ARCH" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        arm64|aarch64)
            ARCH="aarch64"
            ;;
        *)
            error "Unsupported architecture: $ARCH"
            ;;
    esac
    
    PLATFORM="${OS}-${ARCH}"
    log "Detected platform: $PLATFORM"
}

# Check for required tools
check_dependencies() {
    if ! command -v curl &> /dev/null && ! command -v wget &> /dev/null; then
        error "curl or wget is required for installation"
    fi
}

# Download file
download() {
    local url="$1"
    local output="$2"
    
    if command -v curl &> /dev/null; then
        curl -fsSL "$url" -o "$output"
    else
        wget -q "$url" -O "$output"
    fi
}

# Get latest version from GitHub
get_latest_version() {
    if [ "$VERSION" = "latest" ]; then
        log "Fetching latest version..."
        VERSION=$(download "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" - | \
            grep '"tag_name"' | \
            sed -E 's/.*"([^"]+)".*/\1/')
        
        if [ -z "$VERSION" ]; then
            error "Could not determine latest version"
        fi
    fi
    
    log "Version: $VERSION"
}

# Install binary
install_binary() {
    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT
    
    local archive_name="${BIN_NAME}-${VERSION}-${PLATFORM}"
    local download_url="https://github.com/${GITHUB_REPO}/releases/download/${VERSION}/${archive_name}.tar.gz"
    
    log "Downloading from: $download_url"
    
    # For now, build from source instead
    warn "Pre-built binaries not yet available. Building from source..."
    
    # Check for Rust
    if ! command -v cargo &> /dev/null; then
        log "Rust not found. Installing Rust..."
        download "https://sh.rustup.rs" - | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    # Clone and build
    log "Cloning repository..."
    git clone --depth 1 "https://github.com/${GITHUB_REPO}.git" "$tmp_dir/oisp-sensor"
    
    log "Building..."
    cd "$tmp_dir/oisp-sensor"
    cargo build --release
    
    # Install
    log "Installing to $INSTALL_DIR..."
    if [ -w "$INSTALL_DIR" ]; then
        cp "target/release/${BIN_NAME}" "$INSTALL_DIR/"
    else
        sudo cp "target/release/${BIN_NAME}" "$INSTALL_DIR/"
    fi
    
    chmod +x "${INSTALL_DIR}/${BIN_NAME}"
}

# Post-install setup
post_install() {
    success "Installation complete!"
    echo ""
    echo "  OISP Sensor has been installed to: ${INSTALL_DIR}/${BIN_NAME}"
    echo ""
    echo "  Quick start:"
    echo "    sudo oisp-sensor record"
    echo ""
    echo "  View status:"
    echo "    oisp-sensor status"
    echo ""
    echo "  For more information:"
    echo "    oisp-sensor --help"
    echo ""
    
    # Platform-specific notes
    case "$OS" in
        linux)
            echo "  Note: Full capture requires root privileges for eBPF."
            echo "  Run with: sudo oisp-sensor record"
            echo ""
            ;;
        macos)
            echo "  Note: Full capture requires System Extension approval."
            echo "  See documentation for setup instructions."
            echo ""
            ;;
    esac
}

# Main
main() {
    echo ""
    echo "  ╔═══════════════════════════════════════════╗"
    echo "  ║       OISP Sensor Installation            ║"
    echo "  ║    Universal AI Observability Sensor      ║"
    echo "  ╚═══════════════════════════════════════════╝"
    echo ""
    
    check_dependencies
    detect_platform
    get_latest_version
    install_binary
    post_install
}

main "$@"

