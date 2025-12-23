#!/bin/sh
# OISP Sensor Installer
# Usage: curl -sSL https://sensor.oisp.dev/install.sh | sh

set -e

REPO="oximyHQ/oisp-sensor"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

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

# Get latest release
echo "Fetching latest release..."
LATEST=$(curl -sSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST" ]; then
    echo "Warning: Could not fetch latest release, using v0.1.0"
    LATEST="v0.1.0"
fi

echo "Latest version: $LATEST"
echo ""

# Download URL
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST/oisp-sensor-$TARGET.tar.gz"

# Create temp directory
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

echo "Downloading from: $DOWNLOAD_URL"
curl -sSL "$DOWNLOAD_URL" -o "$TMP_DIR/oisp-sensor.tar.gz" || {
    echo ""
    echo "Download not available yet. Building from source..."
    echo ""
    
    # Check for Rust
    if ! command -v cargo > /dev/null 2>&1; then
        echo "Rust not found. Installing via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        . "$HOME/.cargo/env"
    fi
    
    # Clone and build
    git clone --depth 1 "https://github.com/$REPO.git" "$TMP_DIR/oisp-sensor"
    cd "$TMP_DIR/oisp-sensor"
    cargo build --release
    
    # Install
    if [ -w "$INSTALL_DIR" ]; then
        cp target/release/oisp-sensor "$INSTALL_DIR/"
    else
        echo "Installing to $INSTALL_DIR (requires sudo)..."
        sudo cp target/release/oisp-sensor "$INSTALL_DIR/"
    fi
    
    echo ""
    echo "OISP Sensor installed successfully!"
    echo "Run 'oisp-sensor --help' to get started."
    exit 0
}

# Extract
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

echo ""
echo "OISP Sensor installed successfully!"
echo ""
echo "Get started:"
echo "  oisp-sensor --help       # Show help"
echo "  oisp-sensor status       # Check capabilities"
echo "  oisp-sensor record       # Start recording"
echo ""

