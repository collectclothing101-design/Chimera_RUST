#!/bin/bash
# ChimeraRS Release Build Script
# Builds, tests, and packages a release version.
#
# Usage:
#   ./release.sh [--skip-tests] [--skip-dmg]

set -euo pipefail

SKIP_TESTS=false
SKIP_DMG=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-tests) SKIP_TESTS=true; shift ;;
        --skip-dmg) SKIP_DMG=true; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# ═════════════════════════════════════════════════════════════════════════════
#  PRE-FLIGHT CHECKS
# ═════════════════════════════════════════════════════════════════════════════

echo "=== Pre-flight checks ==="

# Check Rust toolchain
if ! command -v cargo &> /dev/null; then
    echo "ERROR: cargo not found"
    exit 1
fi

# Check Xcode tools
if ! command -v xcodebuild &> /dev/null; then
    echo "WARNING: xcodebuild not found"
fi

# Check git status
if [ -n "$(git status --porcelain)" ]; then
    echo "WARNING: Working directory is not clean"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# ═════════════════════════════════════════════════════════════════════════════
#  BUILD
# ═════════════════════════════════════════════════════════════════════════════

echo ""
echo "=== Building release ==="

# Clean previous builds
make clean

# Check workspace
echo "Running cargo check..."
cargo +nightly check --workspace

# Build release
echo "Building release..."
cargo +nightly build --release --workspace

# ═════════════════════════════════════════════════════════════════════════════
#  TESTS
# ═════════════════════════════════════════════════════════════════════════════

if ! $SKIP_TESTS; then
    echo ""
    echo "=== Running tests ==="

    # Unit tests
    echo "Running unit tests..."
    cargo +nightly test --workspace

    # Clippy
    echo "Running clippy..."
    cargo +nightly clippy --workspace -- -D warnings

    # Format check
    echo "Checking formatting..."
    cargo +nightly fmt --check
fi

# ═════════════════════════════════════════════════════════════════════════════
#  BUILD APP
# ═════════════════════════════════════════════════════════════════════════════

echo ""
echo "=== Building Chimera.app ==="

./deploy/build_app.sh --release --universal

# ═════════════════════════════════════════════════════════════════════════════
#  PACKAGE DMG
# ═════════════════════════════════════════════════════════════════════════════

if ! $SKIP_DMG; then
    echo ""
    echo "=== Packaging DMG ==="

    ./deploy/package_dmg.sh --release
fi

# ═════════════════════════════════════════════════════════════════════════════
#  SUMMARY
# ═════════════════════════════════════════════════════════════════════════════

echo ""
echo "=== Release Build Complete ==="
echo ""
echo "Artifacts:"
echo "  - target/release/Chimera.app"
if ! $SKIP_DMG; then
    echo "  - Chimera_$(date +%Y%m%d).dmg"
fi
echo ""
echo "Next steps:"
echo "  1. Test the app manually"
echo "  2. Run: ./deploy/package_dmg.sh --release --notarise"
echo "  3. Upload to GitHub releases"
