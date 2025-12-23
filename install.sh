#!/bin/sh
# OISP Sensor Installer
# Usage: curl -sSL https://sensor.oisp.dev/install.sh | sh
#    or: curl -sSL https://raw.githubusercontent.com/oximyHQ/oisp-sensor/main/install.sh | sh

set -e

REPO="oximyHQ/oisp-sensor"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

echo ""
echo "OISP Sensor Installer"
echo "====================="
echo ""

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in
    x86_64)
        ARCH="x86_64"
        ;;
    aarch64|arm64)
        ARCH="aarch64"
        ;;
    *)
        echo "Error: Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

case "$OS" in
    linux)
        TARGET="${ARCH}-unknown-linux-gnu"
        ;;
    darwin)
        TARGET="${ARCH}-apple-darwin"
        ;;
    *)
        echo "Error: Unsupported OS: $OS"
        echo "Please install from source: https://github.com/$REPO"
        exit 1
        ;;
esac

echo "Detected: $OS $ARCH"
echo "Target: $TARGET"
echo ""

# Get latest release - try GitHub API first, with fallback
echo "Fetching latest release..."
LATEST=""

# Try with curl, handle rate limiting gracefully
API_RESPONSE=$(curl -sSL -w "\n%{http_code}" "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null || echo "000")
HTTP_CODE=$(echo "$API_RESPONSE" | tail -n 1)
API_BODY=$(echo "$API_RESPONSE" | sed '$d')

if [ "$HTTP_CODE" = "200" ]; then
    LATEST=$(echo "$API_BODY" | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')
fi

if [ -z "$LATEST" ]; then
    echo "Warning: Could not fetch latest release from API (HTTP $HTTP_CODE)"
    echo "Trying releases list..."
    
    # Try the releases list as fallback
    RELEASES_RESPONSE=$(curl -sSL "https://api.github.com/repos/$REPO/releases" 2>/dev/null || echo "[]")
    LATEST=$(echo "$RELEASES_RESPONSE" | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')
fi

if [ -z "$LATEST" ]; then
    echo "Warning: Could not determine latest version"
    echo "Please check https://github.com/$REPO/releases for available versions"
    echo ""
    echo "Falling back to building from source..."
    LATEST="main"
fi

echo "Latest version: $LATEST"
echo ""

# Create temp directory
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

# Download URL - matches release workflow naming
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST/oisp-sensor-$TARGET.tar.gz"

echo "Downloading from: $DOWNLOAD_URL"

# Try to download pre-built binary
if curl -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/oisp-sensor.tar.gz" 2>/dev/null; then
    echo "Extracting..."
    tar -xzf "$TMP_DIR/oisp-sensor.tar.gz" -C "$TMP_DIR"

    # Install
    echo "Installing to $INSTALL_DIR..."
    if [ -w "$INSTALL_DIR" ]; then
        cp "$TMP_DIR/oisp-sensor" "$INSTALL_DIR/"
    else
        echo "(requires sudo)"
        sudo cp "$TMP_DIR/oisp-sensor" "$INSTALL_DIR/"
    fi

    chmod +x "$INSTALL_DIR/oisp-sensor"
else
    echo ""
    echo "Pre-built binary not available. Building from source..."
    echo ""

    # Check for Rust
    if ! command -v cargo > /dev/null 2>&1; then
        echo "Rust not found. Installing via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        . "$HOME/.cargo/env"
    fi

    # Clone and build
    echo "Cloning repository..."
    git clone --depth 1 --branch "$LATEST" "https://github.com/$REPO.git" "$TMP_DIR/oisp-sensor" 2>/dev/null || \
    git clone --depth 1 "https://github.com/$REPO.git" "$TMP_DIR/oisp-sensor"
    
    cd "$TMP_DIR/oisp-sensor"
    echo "Building (this may take a few minutes)..."
    cargo build --release

    # Install
    if [ -w "$INSTALL_DIR" ]; then
        cp target/release/oisp-sensor "$INSTALL_DIR/"
    else
        echo "Installing to $INSTALL_DIR (requires sudo)..."
        sudo cp target/release/oisp-sensor "$INSTALL_DIR/"
    fi
fi

echo ""
echo "OISP Sensor installed successfully!"
echo ""
echo "Get started:"
echo "  oisp-sensor --help       # Show help"
echo "  oisp-sensor status       # Check capabilities"
echo "  oisp-sensor record       # Start recording"
echo ""
