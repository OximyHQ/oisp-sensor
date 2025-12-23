#!/bin/bash
# Build .deb package for OISP Sensor
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VERSION="${VERSION:-0.2.0}"
ARCH="${ARCH:-amd64}"

echo "Building OISP Sensor .deb package v${VERSION} (${ARCH})"
echo ""

# Create build directory
BUILD_DIR="$SCRIPT_DIR/build"
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR/oisp-sensor_${VERSION}_${ARCH}"

PKG_DIR="$BUILD_DIR/oisp-sensor_${VERSION}_${ARCH}"

# Copy DEBIAN control files
cp -r "$SCRIPT_DIR/deb/DEBIAN" "$PKG_DIR/"

# Update version in control file
sed -i "s/^Version:.*/Version: ${VERSION}/" "$PKG_DIR/DEBIAN/control"
sed -i "s/^Architecture:.*/Architecture: ${ARCH}/" "$PKG_DIR/DEBIAN/control"

# Make scripts executable
chmod 755 "$PKG_DIR/DEBIAN/postinst"
chmod 755 "$PKG_DIR/DEBIAN/prerm"
chmod 755 "$PKG_DIR/DEBIAN/postrm"

# Create directory structure
mkdir -p "$PKG_DIR/usr/bin"
mkdir -p "$PKG_DIR/lib/systemd/system"
mkdir -p "$PKG_DIR/etc/oisp"
mkdir -p "$PKG_DIR/usr/share/doc/oisp-sensor"

# Copy binary (expects release build)
if [ -f "$PROJECT_ROOT/target/release/oisp-sensor" ]; then
    cp "$PROJECT_ROOT/target/release/oisp-sensor" "$PKG_DIR/usr/bin/"
elif [ -f "$PROJECT_ROOT/target/x86_64-unknown-linux-gnu/release/oisp-sensor" ]; then
    cp "$PROJECT_ROOT/target/x86_64-unknown-linux-gnu/release/oisp-sensor" "$PKG_DIR/usr/bin/"
elif [ -f "$PROJECT_ROOT/target/aarch64-unknown-linux-gnu/release/oisp-sensor" ]; then
    cp "$PROJECT_ROOT/target/aarch64-unknown-linux-gnu/release/oisp-sensor" "$PKG_DIR/usr/bin/"
else
    echo "Error: No binary found. Run 'cargo build --release' first."
    exit 1
fi
chmod 755 "$PKG_DIR/usr/bin/oisp-sensor"

# Copy systemd service
cp "$SCRIPT_DIR/systemd/oisp-sensor.service" "$PKG_DIR/lib/systemd/system/"
chmod 644 "$PKG_DIR/lib/systemd/system/oisp-sensor.service"

# Copy documentation
cp "$PROJECT_ROOT/README.md" "$PKG_DIR/usr/share/doc/oisp-sensor/"
cp "$PROJECT_ROOT/LICENSE" "$PKG_DIR/usr/share/doc/oisp-sensor/"

# Copy example config
cp "$PROJECT_ROOT/config.example.toml" "$PKG_DIR/usr/share/doc/oisp-sensor/"

# Calculate installed size (in KB)
INSTALLED_SIZE=$(du -sk "$PKG_DIR" | cut -f1)
sed -i "s/^Installed-Size:.*/Installed-Size: ${INSTALLED_SIZE}/" "$PKG_DIR/DEBIAN/control" || \
    echo "Installed-Size: ${INSTALLED_SIZE}" >> "$PKG_DIR/DEBIAN/control"

# Build package
cd "$BUILD_DIR"
dpkg-deb --build "oisp-sensor_${VERSION}_${ARCH}"

echo ""
echo "Package built: $BUILD_DIR/oisp-sensor_${VERSION}_${ARCH}.deb"
echo ""
echo "Install with: sudo dpkg -i oisp-sensor_${VERSION}_${ARCH}.deb"

