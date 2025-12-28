#!/bin/bash
# create-dmg.sh
# Create a DMG installer for OISP
#
# Usage: ./create-dmg.sh [app_path] [output_dir]
#
# Requirements:
# - create-dmg (brew install create-dmg) OR hdiutil

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_DIR/build"
EXPORT_DIR="$BUILD_DIR/Export"

# Arguments
APP_PATH="${1:-$EXPORT_DIR/OISP.app}"
OUTPUT_DIR="${2:-$BUILD_DIR}"

# Configuration
APP_NAME="OISP"
DMG_NAME="${APP_NAME}.dmg"
VOLUME_NAME="OISP"
VERSION=$(defaults read "$APP_PATH/Contents/Info.plist" CFBundleShortVersionString 2>/dev/null || echo "1.0.0")

echo "=== OISP DMG Creation ==="
echo "App: $APP_PATH"
echo "Version: $VERSION"
echo "Output: $OUTPUT_DIR/$DMG_NAME"
echo ""

# Verify app exists
if [ ! -d "$APP_PATH" ]; then
    echo "ERROR: App not found at $APP_PATH"
    echo "Run ./build-release.sh first"
    exit 1
fi

mkdir -p "$OUTPUT_DIR"

# Remove existing DMG
rm -f "$OUTPUT_DIR/$DMG_NAME"
rm -f "$OUTPUT_DIR/${APP_NAME}-${VERSION}.dmg"

# Check if create-dmg is available
if command -v create-dmg &> /dev/null; then
    echo "Using create-dmg for pretty DMG..."

    # Create a temporary directory for DMG contents
    DMG_TEMP="$BUILD_DIR/dmg-temp"
    rm -rf "$DMG_TEMP"
    mkdir -p "$DMG_TEMP"

    # Copy app to temp directory
    cp -R "$APP_PATH" "$DMG_TEMP/"

    # Create DMG with create-dmg
    create-dmg \
        --volname "$VOLUME_NAME" \
        --volicon "$APP_PATH/Contents/Resources/AppIcon.icns" \
        --window-pos 200 120 \
        --window-size 600 400 \
        --icon-size 100 \
        --icon "$APP_NAME.app" 150 200 \
        --hide-extension "$APP_NAME.app" \
        --app-drop-link 450 200 \
        --no-internet-enable \
        "$OUTPUT_DIR/${APP_NAME}-${VERSION}.dmg" \
        "$DMG_TEMP" \
        || {
            # If create-dmg fails (e.g., missing icon), fall back to simple DMG
            echo "create-dmg failed, falling back to simple DMG..."
            create-dmg \
                --volname "$VOLUME_NAME" \
                --window-pos 200 120 \
                --window-size 600 400 \
                --icon-size 100 \
                --icon "$APP_NAME.app" 150 200 \
                --app-drop-link 450 200 \
                --no-internet-enable \
                "$OUTPUT_DIR/${APP_NAME}-${VERSION}.dmg" \
                "$DMG_TEMP"
        }

    # Cleanup
    rm -rf "$DMG_TEMP"

else
    echo "create-dmg not found, using hdiutil..."

    # Create a temporary directory for DMG contents
    DMG_TEMP="$BUILD_DIR/dmg-temp"
    rm -rf "$DMG_TEMP"
    mkdir -p "$DMG_TEMP"

    # Copy app
    cp -R "$APP_PATH" "$DMG_TEMP/"

    # Create symlink to Applications
    ln -s /Applications "$DMG_TEMP/Applications"

    # Create a README
    cat > "$DMG_TEMP/README.txt" << EOF
OISP - Observability for Intelligent Systems Platform
Version: $VERSION

Installation:
1. Drag OISP.app to the Applications folder
2. Open OISP from Applications
3. Follow the setup wizard to enable the network extension
4. Trust the OISP CA certificate when prompted

For more information, visit: https://oisp.dev
EOF

    # Create the DMG
    hdiutil create \
        -volname "$VOLUME_NAME" \
        -srcfolder "$DMG_TEMP" \
        -ov \
        -format UDZO \
        "$OUTPUT_DIR/${APP_NAME}-${VERSION}.dmg"

    # Cleanup
    rm -rf "$DMG_TEMP"
fi

# Create a symlink without version for convenience
ln -sf "${APP_NAME}-${VERSION}.dmg" "$OUTPUT_DIR/$DMG_NAME"

echo ""
echo "=== DMG Created ==="
echo "DMG: $OUTPUT_DIR/${APP_NAME}-${VERSION}.dmg"
echo ""

# Show DMG info
echo "DMG Info:"
hdiutil imageinfo "$OUTPUT_DIR/${APP_NAME}-${VERSION}.dmg" | grep -E "(Format|Size|Partition)"
