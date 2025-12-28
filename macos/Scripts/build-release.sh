#!/bin/bash
# build-release.sh
# Build OISP for release distribution
#
# Usage: ./build-release.sh [--skip-sign] [--skip-notarize]
#
# Requirements:
# - Xcode 15+
# - Apple Developer ID certificate
# - DEVELOPMENT_TEAM env var or --team argument

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_DIR/build"
ARCHIVE_DIR="$BUILD_DIR/Archives"
EXPORT_DIR="$BUILD_DIR/Export"

# Configuration
SCHEME="OISP"
PROJECT="$PROJECT_DIR/OISP.xcodeproj"
CONFIGURATION="Release"

# Parse arguments
SKIP_SIGN=false
SKIP_NOTARIZE=false
DEVELOPMENT_TEAM="${DEVELOPMENT_TEAM:-}"

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-sign)
            SKIP_SIGN=true
            shift
            ;;
        --skip-notarize)
            SKIP_NOTARIZE=true
            shift
            ;;
        --team)
            DEVELOPMENT_TEAM="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "=== OISP Release Build ==="
echo "Project: $PROJECT"
echo "Scheme: $SCHEME"
echo "Configuration: $CONFIGURATION"
echo "Skip Signing: $SKIP_SIGN"
echo "Skip Notarization: $SKIP_NOTARIZE"
echo ""

# Clean build directory
echo "Cleaning build directory..."
rm -rf "$BUILD_DIR"
mkdir -p "$ARCHIVE_DIR" "$EXPORT_DIR"

# Copy oisp-spec-bundle.json to app resources
echo "Copying spec bundle to app resources..."
SPEC_BUNDLE_SRC="$PROJECT_DIR/../crates/oisp-core/data/oisp-spec-bundle.json"
SPEC_BUNDLE_DST="$PROJECT_DIR/OISPApp/Resources/oisp-spec-bundle.json"
if [ -f "$SPEC_BUNDLE_SRC" ]; then
    mkdir -p "$(dirname "$SPEC_BUNDLE_DST")"
    cp "$SPEC_BUNDLE_SRC" "$SPEC_BUNDLE_DST"
    echo "Spec bundle copied to app resources."
else
    echo "ERROR: Spec bundle not found at $SPEC_BUNDLE_SRC"
    exit 1
fi

# Regenerate Xcode project from project.yml if xcodegen is available
if command -v xcodegen &> /dev/null; then
    echo "Regenerating Xcode project..."
    cd "$PROJECT_DIR"
    xcodegen generate
    echo "Project regenerated."
fi

# Set code signing options
CODE_SIGN_OPTS=""
if [ "$SKIP_SIGN" = true ]; then
    CODE_SIGN_OPTS="CODE_SIGN_IDENTITY=- CODE_SIGNING_REQUIRED=NO"
    echo "WARNING: Building without code signing. App will not work on other machines."
elif [ -n "$DEVELOPMENT_TEAM" ]; then
    CODE_SIGN_OPTS="DEVELOPMENT_TEAM=$DEVELOPMENT_TEAM"
    echo "Using Development Team: $DEVELOPMENT_TEAM"
else
    echo "WARNING: No DEVELOPMENT_TEAM set. Attempting to use default signing."
fi

# Build the archive
echo "Building archive..."
ARCHIVE_PATH="$ARCHIVE_DIR/OISP.xcarchive"

xcodebuild archive \
    -project "$PROJECT" \
    -scheme "$SCHEME" \
    -configuration "$CONFIGURATION" \
    -archivePath "$ARCHIVE_PATH" \
    $CODE_SIGN_OPTS \
    ONLY_ACTIVE_ARCH=NO \
    SKIP_INSTALL=NO \
    BUILD_LIBRARY_FOR_DISTRIBUTION=YES \
    | tee "$BUILD_DIR/archive.log"

if [ ! -d "$ARCHIVE_PATH" ]; then
    echo "ERROR: Archive failed. Check $BUILD_DIR/archive.log for details."
    exit 1
fi

echo "Archive created: $ARCHIVE_PATH"

# Export the app
echo "Exporting app..."

# Create export options plist
EXPORT_OPTIONS="$BUILD_DIR/ExportOptions.plist"
cat > "$EXPORT_OPTIONS" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>method</key>
    <string>developer-id</string>
    <key>signingStyle</key>
    <string>automatic</string>
    <key>teamID</key>
    <string>${DEVELOPMENT_TEAM:-TEAM_ID}</string>
</dict>
</plist>
EOF

if [ "$SKIP_SIGN" = false ]; then
    xcodebuild -exportArchive \
        -archivePath "$ARCHIVE_PATH" \
        -exportPath "$EXPORT_DIR" \
        -exportOptionsPlist "$EXPORT_OPTIONS" \
        | tee "$BUILD_DIR/export.log"

    if [ ! -d "$EXPORT_DIR/OISP.app" ]; then
        echo "ERROR: Export failed. Check $BUILD_DIR/export.log for details."
        exit 1
    fi
else
    # Just copy the app from the archive for unsigned builds
    cp -R "$ARCHIVE_PATH/Products/Applications/OISP.app" "$EXPORT_DIR/"
fi

echo "App exported: $EXPORT_DIR/OISP.app"

# Verify the app bundle
echo "Verifying app bundle..."
if [ -d "$EXPORT_DIR/OISP.app" ]; then
    echo "  App bundle exists"

    # Check for system extension
    SYSEXT_PATH="$EXPORT_DIR/OISP.app/Contents/Library/SystemExtensions"
    if [ -d "$SYSEXT_PATH" ]; then
        echo "  System extension found"
        ls -la "$SYSEXT_PATH"
    else
        echo "  WARNING: System extension not found in app bundle"
    fi

    # Check code signature (if signed)
    if [ "$SKIP_SIGN" = false ]; then
        echo "  Checking code signature..."
        codesign --verify --deep --strict "$EXPORT_DIR/OISP.app" && echo "  Code signature valid" || echo "  WARNING: Code signature invalid"
    fi
fi

# Notarize (if not skipped and signed)
if [ "$SKIP_SIGN" = false ] && [ "$SKIP_NOTARIZE" = false ]; then
    echo ""
    echo "=== Notarization ==="
    echo "Run ./notarize.sh to submit the app for notarization"
fi

echo ""
echo "=== Build Complete ==="
echo "App: $EXPORT_DIR/OISP.app"
echo ""
echo "Next steps:"
echo "1. Run ./notarize.sh to notarize the app"
echo "2. Run ./create-dmg.sh to create the DMG installer"
