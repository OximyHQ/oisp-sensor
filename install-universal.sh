#!/bin/sh
# OISP Sensor Universal Linux Installer
# Usage: curl -fsSL https://oisp.dev/install.sh | sh
#
# Supports: Ubuntu, Debian, RHEL, Rocky, Alma, Fedora, Arch, Alpine
#
# Environment variables:
#   INSTALL_METHOD - Package manager to use (auto-detected if not set)
#   SKIP_PRECHECK  - Skip pre-flight checks (default: false)
#   INSTALL_DIR    - Binary installation directory (default: /usr/local/bin)

set -e

REPO="oximyhq/sensor"
SKIP_PRECHECK="${SKIP_PRECHECK:-false}"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo ""
    echo "=========================================="
    echo "  OISP Sensor Universal Installer"
    echo "=========================================="
    echo ""
}

print_success() {
    echo "${GREEN}✓${NC} $1"
}

print_error() {
    echo "${RED}✗${NC} $1"
}

print_warning() {
    echo "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo "${BLUE}ℹ${NC} $1"
}

# Detect Linux distribution
detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        DISTRO_ID="$ID"
        DISTRO_VERSION="$VERSION_ID"
        DISTRO_NAME="$NAME"
    elif [ -f /etc/redhat-release ]; then
        DISTRO_ID="rhel"
        DISTRO_NAME="Red Hat Enterprise Linux"
    else
        DISTRO_ID="unknown"
        DISTRO_NAME="Unknown"
    fi
}

# Detect package manager
detect_package_manager() {
    if [ -n "$INSTALL_METHOD" ]; then
        PKG_MGR="$INSTALL_METHOD"
        return
    fi

    case "$DISTRO_ID" in
        ubuntu|debian|linuxmint|pop)
            PKG_MGR="apt"
            ;;
        rhel|centos|rocky|almalinux|ol)
            if command -v dnf > /dev/null 2>&1; then
                PKG_MGR="dnf"
            else
                PKG_MGR="yum"
            fi
            ;;
        fedora)
            PKG_MGR="dnf"
            ;;
        arch|manjaro)
            PKG_MGR="pacman"
            ;;
        alpine)
            PKG_MGR="apk"
            ;;
        opensuse*|sles)
            PKG_MGR="zypper"
            ;;
        *)
            PKG_MGR="binary"
            ;;
    esac
}

# Pre-flight checks
run_preflight_checks() {
    if [ "$SKIP_PRECHECK" = "true" ]; then
        print_warning "Skipping pre-flight checks (SKIP_PRECHECK=true)"
        return 0
    fi

    print_info "Running pre-flight checks..."

    CHECKS_PASSED=0
    CHECKS_FAILED=0
    CHECKS_WARNING=0

    # Check 1: Kernel version
    KERNEL_VERSION=$(uname -r | cut -d. -f1,2)
    KERNEL_MAJOR=$(echo "$KERNEL_VERSION" | cut -d. -f1)
    KERNEL_MINOR=$(echo "$KERNEL_VERSION" | cut -d. -f2)

    if [ "$KERNEL_MAJOR" -gt 5 ] || [ "$KERNEL_MAJOR" -eq 5 -a "$KERNEL_MINOR" -ge 8 ]; then
        print_success "Kernel version $KERNEL_VERSION (>= 5.8 required)"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
    else
        print_error "Kernel version $KERNEL_VERSION is too old (>= 5.8 required)"
        CHECKS_FAILED=$((CHECKS_FAILED + 1))
    fi

    # Check 2: BTF support
    if [ -f /sys/kernel/btf/vmlinux ]; then
        print_success "BTF (BPF Type Format) is available"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
    else
        print_warning "BTF not found - may need kernel with CONFIG_DEBUG_INFO_BTF=y"
        CHECKS_WARNING=$((CHECKS_WARNING + 1))
    fi

    # Check 3: eBPF filesystem
    if mount | grep -q bpf; then
        print_success "BPF filesystem is mounted"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
    else
        print_warning "BPF filesystem not mounted - will be auto-mounted"
        CHECKS_WARNING=$((CHECKS_WARNING + 1))
    fi

    # Check 4: libcap (for setcap)
    if command -v setcap > /dev/null 2>&1; then
        print_success "setcap is available (capabilities support)"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
    else
        print_warning "setcap not found - install libcap2-bin (Debian) or libcap (RHEL)"
        CHECKS_WARNING=$((CHECKS_WARNING + 1))
    fi

    # Check 5: OpenSSL library
    LIBSSL_PATH=""
    for path in \
        /usr/lib/x86_64-linux-gnu/libssl.so.3 \
        /usr/lib/x86_64-linux-gnu/libssl.so.1.1 \
        /usr/lib64/libssl.so.3 \
        /usr/lib64/libssl.so.1.1 \
        /lib/x86_64-linux-gnu/libssl.so.3 \
        /lib64/libssl.so.3 \
        /usr/lib/aarch64-linux-gnu/libssl.so.3 \
        /lib/aarch64-linux-gnu/libssl.so.3
    do
        if [ -f "$path" ]; then
            LIBSSL_PATH="$path"
            break
        fi
    done

    if [ -n "$LIBSSL_PATH" ]; then
        print_success "OpenSSL library found: $LIBSSL_PATH"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
    else
        print_error "OpenSSL library not found - SSL capture will not work"
        CHECKS_FAILED=$((CHECKS_FAILED + 1))
    fi

    # Check 6: Systemd
    if command -v systemctl > /dev/null 2>&1; then
        print_success "systemd is available"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
    else
        print_warning "systemd not found - manual service management required"
        CHECKS_WARNING=$((CHECKS_WARNING + 1))
    fi

    # Summary
    echo ""
    echo "Pre-flight check summary:"
    print_success "$CHECKS_PASSED checks passed"
    if [ $CHECKS_WARNING -gt 0 ]; then
        print_warning "$CHECKS_WARNING warnings"
    fi
    if [ $CHECKS_FAILED -gt 0 ]; then
        print_error "$CHECKS_FAILED checks failed"
        echo ""
        print_error "System does not meet requirements. Installation may fail."
        echo ""
        printf "Continue anyway? [y/N] "
        read -r response
        case "$response" in
            [yY][eE][sS]|[yY])
                print_warning "Continuing despite failed checks..."
                ;;
            *)
                echo "Installation aborted."
                exit 1
                ;;
        esac
    fi
    echo ""
}

# Install via APT (Debian/Ubuntu)
install_apt() {
    print_info "Installing via APT (Debian/Ubuntu)..."

    # Get latest release
    LATEST=$(curl -sSL https://api.github.com/repos/$REPO/releases/latest | grep '"tag_name"' | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')

    if [ -z "$LATEST" ]; then
        print_error "Could not determine latest version"
        return 1
    fi

    ARCH=$(dpkg --print-architecture)
    DEB_URL="https://github.com/$REPO/releases/download/$LATEST/oisp-sensor_${LATEST#v}_${ARCH}.deb"

    print_info "Downloading: $DEB_URL"

    TMP_DEB=$(mktemp)
    if curl -fsSL "$DEB_URL" -o "$TMP_DEB"; then
        print_info "Installing package..."
        sudo apt-get update -qq || true
        sudo dpkg -i "$TMP_DEB" || sudo apt-get install -f -y
        rm -f "$TMP_DEB"
        print_success "Installation complete"
        return 0
    else
        rm -f "$TMP_DEB"
        print_warning "DEB package not available, falling back to binary install"
        return 1
    fi
}

# Install via DNF/YUM (RHEL/Fedora)
install_dnf() {
    print_info "Installing via $PKG_MGR (RHEL/Fedora)..."

    # Get latest release
    LATEST=$(curl -sSL https://api.github.com/repos/$REPO/releases/latest | grep '"tag_name"' | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')

    if [ -z "$LATEST" ]; then
        print_error "Could not determine latest version"
        return 1
    fi

    ARCH=$(uname -m)
    RPM_URL="https://github.com/$REPO/releases/download/$LATEST/oisp-sensor-${LATEST#v}-1.${ARCH}.rpm"

    print_info "Downloading: $RPM_URL"

    TMP_RPM=$(mktemp)
    if curl -fsSL "$RPM_URL" -o "$TMP_RPM"; then
        print_info "Installing package..."
        sudo $PKG_MGR install -y "$TMP_RPM"
        rm -f "$TMP_RPM"
        print_success "Installation complete"
        return 0
    else
        rm -f "$TMP_RPM"
        print_warning "RPM package not available, falling back to binary install"
        return 1
    fi
}

# Install via binary (fallback)
install_binary() {
    print_info "Installing pre-built binary..."

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
            print_error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac

    TARGET="${ARCH}-unknown-linux-gnu"

    # Get latest release
    LATEST=$(curl -sSL https://api.github.com/repos/$REPO/releases/latest | grep '"tag_name"' | sed -E 's/.*"tag_name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')

    if [ -z "$LATEST" ]; then
        print_warning "Could not determine latest version, using 'main'"
        LATEST="main"
    fi

    DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST/oisp-sensor-$TARGET.tar.gz"

    TMP_DIR=$(mktemp -d)
    trap "rm -rf $TMP_DIR" EXIT

    print_info "Downloading from: $DOWNLOAD_URL"

    if curl -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/oisp-sensor.tar.gz"; then
        tar -xzf "$TMP_DIR/oisp-sensor.tar.gz" -C "$TMP_DIR"

        print_info "Installing to $INSTALL_DIR..."
        if [ -w "$INSTALL_DIR" ]; then
            cp "$TMP_DIR/oisp-sensor" "$INSTALL_DIR/"
        else
            sudo cp "$TMP_DIR/oisp-sensor" "$INSTALL_DIR/"
        fi

        chmod +x "$INSTALL_DIR/oisp-sensor"

        # Set capabilities
        if command -v setcap > /dev/null 2>&1; then
            print_info "Setting eBPF capabilities..."
            sudo setcap cap_sys_admin,cap_bpf,cap_perfmon,cap_net_admin+ep "$INSTALL_DIR/oisp-sensor" 2>/dev/null || print_warning "Could not set capabilities"
        fi

        # Create directories
        sudo mkdir -p /etc/oisp /var/log/oisp /var/lib/oisp

        # Install systemd service if available
        if command -v systemctl > /dev/null 2>&1; then
            print_info "Installing systemd service..."
            sudo tee /etc/systemd/system/oisp-sensor.service > /dev/null << 'EOF'
[Unit]
Description=OISP Sensor - Universal AI Observability
Documentation=https://sensor.oximy.com
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/oisp-sensor record --output /var/log/oisp-sensor/events.jsonl --port 7777
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5
ExecStartPre=/bin/mkdir -p /var/log/oisp-sensor /var/lib/oisp-sensor
NoNewPrivileges=no
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/var/log/oisp-sensor /var/lib/oisp-sensor
AmbientCapabilities=CAP_SYS_ADMIN CAP_BPF CAP_PERFMON CAP_NET_ADMIN
LimitMEMLOCK=infinity
StandardOutput=journal
StandardError=journal
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF
            sudo systemctl daemon-reload
        fi

        print_success "Binary installation complete"
        return 0
    else
        print_error "Could not download pre-built binary"
        return 1
    fi
}

# Main installation flow
main() {
    print_header

    # Detect system
    detect_distro
    detect_package_manager

    print_info "Detected: $DISTRO_NAME ($DISTRO_ID)"
    print_info "Package Manager: $PKG_MGR"
    echo ""

    # Run pre-flight checks
    run_preflight_checks

    # Install based on package manager
    case "$PKG_MGR" in
        apt)
            if ! install_apt; then
                install_binary
            fi
            ;;
        dnf|yum)
            if ! install_dnf; then
                install_binary
            fi
            ;;
        *)
            install_binary
            ;;
    esac

    # Success message
    echo ""
    echo "=========================================="
    echo "  Installation Complete!"
    echo "=========================================="
    echo ""
    print_success "OISP Sensor is now installed"
    echo ""
    echo "Next steps:"
    echo "  1. Start the service:"
    echo "     sudo systemctl enable oisp-sensor"
    echo "     sudo systemctl start oisp-sensor"
    echo ""
    echo "  2. Check status:"
    echo "     oisp-sensor status"
    echo "     sudo journalctl -u oisp-sensor -f"
    echo ""
    echo "  3. Access Web UI:"
    echo "     http://localhost:7777"
    echo ""
    echo "Documentation: https://sensor.oximy.com"
    echo ""
}

main
