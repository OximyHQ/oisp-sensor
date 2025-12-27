#!/bin/bash
# Build RPM package for OISP Sensor
# Supports: RHEL 9, Rocky Linux 9, AlmaLinux 9, Fedora 39+

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
VERSION="${VERSION:-0.2.0}"
RELEASE="${RELEASE:-1}"

# Architecture detection
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)
        RPM_ARCH="x86_64"
        RUST_TARGET="x86_64-unknown-linux-gnu"
        ;;
    aarch64)
        RPM_ARCH="aarch64"
        RUST_TARGET="aarch64-unknown-linux-gnu"
        ;;
    *)
        echo "Error: Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

echo "=========================================="
echo "OISP Sensor RPM Builder"
echo "=========================================="
echo "Version: $VERSION-$RELEASE"
echo "Architecture: $RPM_ARCH"
echo "Rust Target: $RUST_TARGET"
echo ""

# Check for rpmbuild
if ! command -v rpmbuild &> /dev/null; then
    echo "Error: rpmbuild not found. Install with:"
    echo "  RHEL/Rocky/Alma: sudo dnf install rpm-build rpmdevtools"
    echo "  Fedora: sudo dnf install rpm-build rpmdevtools"
    exit 1
fi

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo not found. Install from https://rustup.rs"
    exit 1
fi

# Setup RPM build environment
echo "Setting up RPM build environment..."
rpmdev-setuptree 2>/dev/null || mkdir -p ~/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

# Build the binary
echo "Building oisp-sensor binary..."
cd "$PROJECT_ROOT"
cargo build --release --target "$RUST_TARGET"

# Create source tarball
echo "Creating source tarball..."
TARBALL_NAME="oisp-sensor-${VERSION}.tar.gz"
BUILD_DIR="$(mktemp -d)"
PACKAGE_DIR="$BUILD_DIR/oisp-sensor-${VERSION}"

mkdir -p "$PACKAGE_DIR"

# Copy binary
cp "target/${RUST_TARGET}/release/oisp-sensor" "$PACKAGE_DIR/"

# Copy systemd service
mkdir -p "$PACKAGE_DIR/packaging/systemd"
cp packaging/systemd/oisp-sensor.service "$PACKAGE_DIR/packaging/systemd/"

# Copy spec file
mkdir -p "$PACKAGE_DIR/packaging/rpm"
cp packaging/rpm/oisp-sensor.spec "$PACKAGE_DIR/packaging/rpm/"

# Create tarball
cd "$BUILD_DIR"
tar czf "$TARBALL_NAME" "oisp-sensor-${VERSION}"
mv "$TARBALL_NAME" ~/rpmbuild/SOURCES/

# Copy spec to SPECS
cp "$PACKAGE_DIR/packaging/rpm/oisp-sensor.spec" ~/rpmbuild/SPECS/

# Clean up temp directory
rm -rf "$BUILD_DIR"

# Build RPM
echo "Building RPM package..."
cd ~/rpmbuild/SPECS
rpmbuild -ba oisp-sensor.spec \
    --define "version $VERSION" \
    --define "release $RELEASE"

# Find and display the built RPM
RPM_FILE=$(find ~/rpmbuild/RPMS -name "oisp-sensor-*.rpm" -type f | head -1)
SRPM_FILE=$(find ~/rpmbuild/SRPMS -name "oisp-sensor-*.src.rpm" -type f | head -1)

echo ""
echo "=========================================="
echo "RPM Build Complete!"
echo "=========================================="
echo ""
echo "Binary RPM: $RPM_FILE"
echo "Source RPM: $SRPM_FILE"
echo ""
echo "Install with:"
echo "  sudo dnf install $RPM_FILE"
echo ""
echo "Or for RHEL 8:"
echo "  sudo yum install $RPM_FILE"
echo ""

# Copy to project directory
OUTPUT_DIR="$PROJECT_ROOT/dist/rpm"
mkdir -p "$OUTPUT_DIR"
cp "$RPM_FILE" "$OUTPUT_DIR/" 2>/dev/null || true
cp "$SRPM_FILE" "$OUTPUT_DIR/" 2>/dev/null || true

if [ -f "$OUTPUT_DIR/$(basename "$RPM_FILE")" ]; then
    echo "RPM also copied to: $OUTPUT_DIR/$(basename "$RPM_FILE")"
fi

echo ""
