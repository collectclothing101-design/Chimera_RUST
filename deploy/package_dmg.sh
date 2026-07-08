#!/bin/bash
# ChimeraRS DMG Packaging Script
# Creates a signed and notarised DMG for distribution.
#
# Usage:
#   ./package_dmg.sh [--release] [--notarise]
#
# Prerequisites:
#   - Developer ID Application certificate in keychain
#   - Apple Developer account with notarisation privileges
#   - xcrun notarytool credentials configured

set -euo pipefail

# ═════════════════════════════════════════════════════════════════════════════
#  CONFIGURATION
# ═════════════════════════════════════════════════════════════════════════════

APP_NAME="Chimera"
BUNDLE_ID="io.chimerars.chimera"
DMG_NAME="${APP_NAME}RS_$(date +%Y%m%d).dmg"
VOLUME_NAME="${APP_NAME}RS"
BACKGROUND_IMAGE="deploy/dmg_background.png"
ICON_POSITION_X=400
ICON_POSITION_Y=200
WINDOW_WIDTH=800
WINDOW_HEIGHT=400

# Parse arguments
RELEASE=false
NOTARISE=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --release) RELEASE=true; shift ;;
        --notarise) NOTARISE=true; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# ═════════════════════════════════════════════════════════════════════════════
#  BUILD
# ═════════════════════════════════════════════════════════════════════════════

echo "=== Building Chimera.app ==="

if $RELEASE; then
    ./deploy/build_app.sh --release --universal --no-sign
    APP_PATH="target/release/Chimera.app"
else
    ./deploy/build_app.sh --universal --no-sign
    APP_PATH="target/debug/Chimera.app"
fi

if [ ! -d "$APP_PATH" ]; then
    echo "ERROR: $APP_PATH not found"
    exit 1
fi

# ═════════════════════════════════════════════════════════════════════════════
#  CODESIGN
# ═════════════════════════════════════════════════════════════════════════════

echo "=== Codesigning ==="

# Try to find Developer ID certificate
SIGN_ID=$(security find-identity -v -p codesigning | grep "Developer ID Application" | head -1 | awk -F '"' '{print $2}')

if [ -z "$SIGN_ID" ]; then
    echo "WARNING: No Developer ID certificate found. Using ad-hoc signing."
    SIGN_ID="-"
fi

codesign --force --deep --sign "$SIGN_ID" \
    --options runtime \
    --entitlements macos_app/entitlements.plist \
    "$APP_PATH"

codesign --verify --verbose "$APP_PATH"

# ═════════════════════════════════════════════════════════════════════════════
#  CREATE DMG
# ═════════════════════════════════════════════════════════════════════════════

echo "=== Creating DMG ==="

# Clean up old DMG
rm -f "$DMG_NAME"
rm -f "${DMG_NAME}.dmg"

# Create temporary directory for DMG contents
DMG_TEMP=$(mktemp -d)
trap "rm -rf $DMG_TEMP" EXIT

# Copy app to DMG temp
cp -R "$APP_PATH" "$DMG_TEMP/"

# Create Applications symlink
ln -s /Applications "$DMG_TEMP/Applications"

# Create DMG
hdiutil create \
    -volname "$VOLUME_NAME" \
    -srcfolder "$DMG_TEMP" \
    -ov \
    -format UDZO \
    "$DMG_NAME"

echo "Created: $DMG_NAME"

# ═════════════════════════════════════════════════════════════════════════════
#  CODESIGN DMG
# ═════════════════════════════════════════════════════════════════════════════

echo "=== Codesigning DMG ==="

codesign --force --sign "$SIGN_ID" "$DMG_NAME"

# ═════════════════════════════════════════════════════════════════════════════
#  NOTARISE
# ═════════════════════════════════════════════════════════════════════════════

if $NOTARISE; then
    echo "=== Notarising ==="

    # Submit for notarisation
    xcrun notarytool submit "$DMG_NAME" \
        --apple-id "$APPLE_ID" \
        --team-id "$TEAM_ID" \
        --password "$APP_PASSWORD" \
        --wait

    # Staple notarisation ticket
    xcrun stapler staple "$DMG_NAME"

    echo "Notarisation complete"
fi

# ═════════════════════════════════════════════════════════════════════════════
#  VERIFY
# ═════════════════════════════════════════════════════════════════════════════

echo "=== Verifying ==="

codesign --verify --verbose "$DMG_NAME"
spctl --assess --type execute "$APP_PATH" 2>/dev/null || echo "NOTE: spctl check may fail without notarisation"

# ═════════════════════════════════════════════════════════════════════════════
#  SUMMARY
# ═════════════════════════════════════════════════════════════════════════════

echo ""
echo "=== Build Complete ==="
echo "DMG: $DMG_NAME"
echo "Size: $(du -h "$DMG_NAME" | cut -f1)"
echo ""
echo "To distribute:"
echo "  1. Upload $DMG_NAME to your website or GitHub releases"
echo "  2. Users mount the DMG and drag Chimera.app to Applications"
echo "  3. First launch may require right-click > Open (Gatekeeper)"
