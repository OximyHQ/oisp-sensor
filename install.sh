#!/bin/sh
# OISP Sensor Installer
# Usage: curl -sSL https://sensor.oisp.dev/install.sh | sh
#
# Environment variables:
#   INSTALL_DIR    - Installation directory (default: /usr/local/bin)
#   INSTALL_SERVICE - Install systemd service (default: auto-detect)
#   SKIP_CAPS      - Skip setting capabilities (default: false)

set -e

REPO="oximyHQ/oisp-sensor"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
INSTALL_SERVICE="${INSTALL_SERVICE:-auto}"
SKIP_CAPS="${SKIP_CAPS:-false}"

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

# Get latest release
echo "Fetching latest release..."
LATEST=""

API_RESPONSE=$(curl -sSL -w "\n%{http_code}" "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null || echo "000")
HTTP_CODE=$(echo "$API_RESPONSE" | tail -n 1)
API_BODY=$(echo "$API_RESPONSE" | sed '$d')

if [ "$HTTP_CODE" = "200" ]; then
    LATEST=$(echo "$API_BODY" | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')
fi

if [ -z "$LATEST" ]; then
    echo "Warning: Could not fetch latest release from API (HTTP $HTTP_CODE)"
    RELEASES_RESPONSE=$(curl -sSL "https://api.github.com/repos/$REPO/releases" 2>/dev/null || echo "[]")
    LATEST=$(echo "$RELEASES_RESPONSE" | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')
fi

if [ -z "$LATEST" ]; then
    echo "Warning: Could not determine latest version"
    echo "Falling back to building from source..."
    LATEST="main"
fi

echo "Latest version: $LATEST"
echo ""

# Create temp directory
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST/oisp-sensor-$TARGET.tar.gz"

echo "Downloading from: $DOWNLOAD_URL"

# Try to download pre-built binary
if curl -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/oisp-sensor.tar.gz" 2>/dev/null; then
    echo "Extracting..."
    tar -xzf "$TMP_DIR/oisp-sensor.tar.gz" -C "$TMP_DIR"

    # Install binary
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

# Linux-specific setup
if [ "$OS" = "linux" ]; then
    echo ""
    echo "Setting up Linux eBPF capabilities..."
    
    # Set capabilities for eBPF (allows running without root)
    if [ "$SKIP_CAPS" != "true" ]; then
        if command -v setcap > /dev/null 2>&1; then
            echo "Setting CAP_BPF and CAP_PERFMON capabilities..."
            if sudo setcap cap_sys_admin,cap_bpf,cap_perfmon,cap_net_admin+ep "$INSTALL_DIR/oisp-sensor" 2>/dev/null; then
                echo "Capabilities set successfully."
                echo ""
                echo "NOTE: You can now run 'oisp-sensor record' without sudo."
                echo "      However, the first run after capability set may still need sudo"
                echo "      to load eBPF bytecode from the expected path."
            else
                echo "Warning: Could not set capabilities. You may need to run with sudo."
            fi
        else
            echo "Warning: setcap not found. Install libcap2-bin for capability support."
        fi
    fi
    
    # Install systemd service (if systemd is available)
    if [ "$INSTALL_SERVICE" = "auto" ]; then
        if command -v systemctl > /dev/null 2>&1; then
            INSTALL_SERVICE="true"
        else
            INSTALL_SERVICE="false"
        fi
    fi
    
    if [ "$INSTALL_SERVICE" = "true" ]; then
        echo ""
        echo "Installing systemd service..."
        
        # Create service file
        SERVICE_FILE="/etc/systemd/system/oisp-sensor.service"
        sudo tee "$SERVICE_FILE" > /dev/null << 'EOF'
[Unit]
Description=OISP Sensor - Universal AI Observability
Documentation=https://sensor.oisp.dev
After=network.target

[Service]
Type=simple
User=root
Group=root

# Main command - records AI activity with web UI on default port
ExecStart=/usr/local/bin/oisp-sensor record --output /var/log/oisp-sensor/events.jsonl --port 7777

# Reload configuration on SIGHUP
ExecReload=/bin/kill -HUP $MAINPID

# Restart on failure with backoff
Restart=on-failure
RestartSec=5
RestartPreventExitStatus=SIGTERM

# Stop timeout
TimeoutStopSec=30

# Create required directories before starting
ExecStartPre=/bin/mkdir -p /var/log/oisp-sensor
ExecStartPre=/bin/mkdir -p /var/lib/oisp-sensor

# Security hardening (compatible with eBPF requirements)
NoNewPrivileges=no
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/var/log/oisp-sensor /var/lib/oisp-sensor
PrivateTmp=true

# Required capabilities for eBPF SSL capture
AmbientCapabilities=CAP_SYS_ADMIN CAP_BPF CAP_PERFMON CAP_NET_ADMIN
CapabilityBoundingSet=CAP_SYS_ADMIN CAP_BPF CAP_PERFMON CAP_NET_ADMIN

# Logging to systemd journal
StandardOutput=journal
StandardError=journal
SyslogIdentifier=oisp-sensor

# Environment
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1

# Resource limits
LimitMEMLOCK=infinity
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF

        # Update ExecStart with actual install path
        sudo sed -i "s|ExecStart=/usr/local/bin/oisp-sensor|ExecStart=$INSTALL_DIR/oisp-sensor|" "$SERVICE_FILE"
        
        sudo systemctl daemon-reload
        
        echo ""
        echo "Systemd service installed. Enable with:"
        echo "  sudo systemctl enable oisp-sensor"
        echo "  sudo systemctl start oisp-sensor"
    fi
    
    # Create config directory
    if [ ! -d /etc/oisp ]; then
        echo ""
        echo "Creating config directory /etc/oisp..."
        sudo mkdir -p /etc/oisp
    fi
fi

echo ""
echo "========================================"
echo "  OISP Sensor installed successfully!"
echo "========================================"
echo ""
echo "Get started:"
echo "  oisp-sensor --help       # Show help"
echo "  oisp-sensor status       # Check capabilities"
if [ "$OS" = "linux" ]; then
echo "  sudo oisp-sensor record  # Start recording (first run)"
echo "  oisp-sensor record       # Start recording (after caps set)"
else
echo "  sudo oisp-sensor record  # Start recording"
fi
echo ""
echo "Web UI:  http://localhost:7777"
echo "Docs:    https://sensor.oisp.dev"
echo ""
