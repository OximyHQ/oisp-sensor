#!/bin/bash
# notarize.sh
# Notarize OISP app with Apple
#
# Usage: ./notarize.sh [app_or_dmg_path]
#
# Requirements:
# - Signed app bundle (Developer ID Application certificate)
# - App-specific password stored in keychain
# - APPLE_ID and APPLE_TEAM_ID environment variables
#
# Setup:
# 1. Generate app-specific password at appleid.apple.com
# 2. Store it: xcrun notarytool store-credentials "OISP-Notarize" \
#              --apple-id "your@email.com" \
#              --team-id "TEAM_ID" \
#              --password "xxxx-xxxx-xxxx-xxxx"

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_DIR/build"
EXPORT_DIR="$BUILD_DIR/Export"

# Configuration
KEYCHAIN_PROFILE="${KEYCHAIN_PROFILE:-OISP-Notarize}"
APPLE_ID="${APPLE_ID:-}"
APPLE_TEAM_ID="${APPLE_TEAM_ID:-}"

# Arguments
INPUT_PATH="${1:-$EXPORT_DIR/OISP.app}"

echo "=== OISP Notarization ==="
echo "Input: $INPUT_PATH"
echo "Keychain Profile: $KEYCHAIN_PROFILE"
echo ""

# Determine if we're notarizing an app or DMG
if [[ "$INPUT_PATH" == *.dmg ]]; then
    NOTARIZE_FILE="$INPUT_PATH"
    IS_DMG=true
elif [[ -d "$INPUT_PATH" ]]; then
    # It's an app bundle - need to create a zip for notarization
    APP_PATH="$INPUT_PATH"
    ZIP_PATH="$BUILD_DIR/OISP-notarize.zip"

    echo "Creating zip for notarization..."
    ditto -c -k --keepParent "$APP_PATH" "$ZIP_PATH"
    NOTARIZE_FILE="$ZIP_PATH"
    IS_DMG=false
else
    echo "ERROR: Input must be an app bundle or DMG"
    exit 1
fi

# Verify code signature before notarization
echo "Verifying code signature..."
if [ "$IS_DMG" = false ]; then
    if ! codesign --verify --deep --strict "$APP_PATH" 2>&1; then
        echo "ERROR: Code signature verification failed"
        echo "Make sure the app is signed with a Developer ID certificate"
        exit 1
    fi
    echo "Code signature verified."
fi

# Submit for notarization
echo ""
echo "Submitting for notarization..."

if xcrun notarytool submit "$NOTARIZE_FILE" \
    --keychain-profile "$KEYCHAIN_PROFILE" \
    --wait \
    --timeout 30m; then
    echo ""
    echo "Notarization successful!"
else
    # Try with explicit credentials if keychain profile fails
    if [ -n "$APPLE_ID" ] && [ -n "$APPLE_TEAM_ID" ]; then
        echo "Keychain profile failed, trying with explicit credentials..."
        echo "Enter your app-specific password:"
        read -s APPLE_PASSWORD

        xcrun notarytool submit "$NOTARIZE_FILE" \
            --apple-id "$APPLE_ID" \
            --team-id "$APPLE_TEAM_ID" \
            --password "$APPLE_PASSWORD" \
            --wait \
            --timeout 30m
    else
        echo ""
        echo "ERROR: Notarization failed"
        echo ""
        echo "Setup instructions:"
        echo "1. Generate an app-specific password at appleid.apple.com"
        echo "2. Store credentials in keychain:"
        echo "   xcrun notarytool store-credentials \"$KEYCHAIN_PROFILE\" \\"
        echo "       --apple-id \"your@email.com\" \\"
        echo "       --team-id \"YOUR_TEAM_ID\" \\"
        echo "       --password \"xxxx-xxxx-xxxx-xxxx\""
        exit 1
    fi
fi

# Staple the notarization ticket
echo ""
echo "Stapling notarization ticket..."

if [ "$IS_DMG" = true ]; then
    xcrun stapler staple "$INPUT_PATH"
else
    xcrun stapler staple "$APP_PATH"
fi

echo ""
echo "=== Notarization Complete ==="
echo ""

# Verify stapling
echo "Verifying stapled notarization..."
if [ "$IS_DMG" = true ]; then
    spctl --assess --type open --context context:primary-signature "$INPUT_PATH" && echo "DMG is properly notarized" || echo "WARNING: DMG verification failed"
else
    spctl --assess --type execute "$APP_PATH" && echo "App is properly notarized" || echo "WARNING: App verification failed"
fi

# Cleanup zip if we created it
if [ "$IS_DMG" = false ] && [ -f "$ZIP_PATH" ]; then
    rm "$ZIP_PATH"
fi

echo ""
echo "Next steps:"
echo "1. Run ./create-dmg.sh to create the final DMG (if not already done)"
echo "2. Distribute the notarized DMG"
