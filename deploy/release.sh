#!/bin/bash
# ═════════════════════════════════════════════════════════════════════════════
#  ChimeraRS Release Script
#  Builds, packages, and publishes a release to GitHub.
#
#  Usage:
#    ./deploy/release.sh                          # Interactive release
#    ./deploy/release.sh --version 1.4.0          # Specify version
#    ./deploy/release.sh --skip-build             # Use existing build
#    ./deploy/release.sh --dry-run                # Preview without publishing
#    ./deploy/release.sh --upload-only            # Upload existing DMG only
#
#  Requirements:
#    - gh CLI (brew install gh)
#    - gh auth login (one-time setup)
# ═════════════════════════════════════════════════════════════════════════════

set -euo pipefail

# ═════════════════════════════════════════════════════════════════════════════
#  CONFIGURATION
# ═════════════════════════════════════════════════════════════════════════════

REPO="collectclothing101-design/Chimera_RUST"
APP_NAME="Chimera"
MIN_MACOS="10.14"

# ═════════════════════════════════════════════════════════════════════════════
#  PARSE ARGUMENTS
# ═════════════════════════════════════════════════════════════════════════════

VERSION=""
SKIP_BUILD=false
DRY_RUN=false
UPLOAD_ONLY=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --version)      VERSION="$2"; shift 2 ;;
        --skip-build)   SKIP_BUILD=true; shift ;;
        --dry-run)      DRY_RUN=true; shift ;;
        --upload-only)  UPLOAD_ONLY=true; shift ;;
        --help|-h)
            sed -n '2,15p' "$0"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# ═════════════════════════════════════════════════════════════════════════════
#  FUNCTIONS
# ═════════════════════════════════════════════════════════════════════════════

log()     { echo -e "\033[1;36m→ $1\033[0m"; }
success() { echo -e "\033[1;32m✓ $1\033[0m"; }
error()   { echo -e "\033[1;31m✗ $1\033[0m"; exit 1; }
warn()    { echo -e "\033[1;33m⚠ $1\033[0m"; }

get_version() {
    # Try to get version from Cargo.toml
    local cargo_version
    cargo_version=$(awk -F'"' '/^version = /{print $2; exit}' crates/chimera-core/Cargo.toml 2>/dev/null || echo "")
    
    if [[ -n "$cargo_version" ]]; then
        echo "$cargo_version"
    else
        echo "1.0.0"
    fi
}

generate_changelog() {
    local version="$1"
    local tag="$2"
    local previous_tag=""
    
    # Find previous tag
    previous_tag=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")
    
    echo "## What's New in v${version}"
    echo ""
    echo "### Features"
    
    # Get commits since last tag
    if [[ -n "$previous_tag" ]]; then
        git log --pretty=format:"- %s" "${previous_tag}..HEAD" --no-merges | grep -iE "^(feat|add|implement|support)" | head -20 || echo "- Bug fixes and improvements"
    else
        git log --pretty=format:"- %s" --no-merges -20 | grep -iE "^(feat|add|implement|support)" | head -10 || echo "- Initial release"
    fi
    
    echo ""
    echo "### Installation"
    echo "1. Download the DMG file below"
    echo "2. Mount the DMG"
    echo "3. Drag \`Chimera.app\` to Applications"
    echo "4. Launch from Applications"
    echo ""
    echo "### Supported Platforms"
    echo "- macOS ${MIN_MACOS} Mojave or later"
    echo "- Intel (x86_64) and Apple Silicon (arm64)"
    echo ""
    echo "### SHA-256"
    if [[ -f "ChimeraRS_${version}.dmg" ]]; then
        shasum -a 256 "ChimeraRS_${version}.dmg" | awk '{print "`"$1"`"}'
    fi
}

# ═════════════════════════════════════════════════════════════════════════════
#  PRE-FLIGHT CHECKS
# ═════════════════════════════════════════════════════════════════════════════

log "Running pre-flight checks..."

# Check required tools
for cmd in cargo git hdiutil; do
    command -v "$cmd" >/dev/null 2>&1 || error "$cmd not found"
done

# Check gh CLI for upload
if [[ "$UPLOAD_ONLY" == true ]] || [[ "$DRY_RUN" == false ]]; then
    if ! command -v gh &>/dev/null; then
        warn "gh CLI not found. Install with: brew install gh"
        warn "Then authenticate: gh auth login"
        read -p "Continue without GitHub upload? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
        SKIP_UPLOAD=true
    else
        # Check gh auth status
        if ! gh auth status &>/dev/null; then
            warn "gh not authenticated. Run: gh auth login"
            read -p "Continue without GitHub upload? (y/N) " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                exit 1
            fi
            SKIP_UPLOAD=true
        else
            SKIP_UPLOAD=false
        fi
    fi
else
    SKIP_UPLOAD=true
fi

# Get or prompt for version
if [[ -z "$VERSION" ]]; then
    DEFAULT_VERSION=$(get_version)
    read -p "Release version [${DEFAULT_VERSION}]: " VERSION
    VERSION="${VERSION:-$DEFAULT_VERSION}"
fi

TAG="v${VERSION}"
DMG_NAME="ChimeraRS_${VERSION}.dmg"

log "Version: ${VERSION}"
log "Tag: ${TAG}"
log "DMG: ${DMG_NAME}"

# ═════════════════════════════════════════════════════════════════════════════
#  BUILD (unless --skip-build or --upload-only)
# ═════════════════════════════════════════════════════════════════════════════

if [[ "$SKIP_BUILD" == false ]] && [[ "$UPLOAD_ONLY" == false ]]; then
    
    # Check git status
    if [ -n "$(git status --porcelain)" ]; then
        warn "Working directory is not clean"
        read -p "Continue anyway? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
    
    log "Building universal app..."
    ./deploy/build_app.sh --release --universal --no-sign
    
    log "Packaging DMG..."
    rm -f "$DMG_NAME"
    
    DMG_TEMP=$(mktemp -d)
    trap "rm -rf $DMG_TEMP" EXIT
    
    cp -R "target/release/Chimera.app" "$DMG_TEMP/"
    ln -s /Applications "$DMG_TEMP/Applications"
    
    hdiutil create \
        -volname "ChimeraRS" \
        -srcfolder "$DMG_TEMP" \
        -ov \
        -format UDZO \
        "$DMG_NAME"
    
    rm -rf "$DMG_TEMP"
    
    success "DMG created: ${DMG_NAME} ($(du -h "$DMG_NAME" | cut -f1))"
    
else
    # Check if DMG exists
    if [[ ! -f "$DMG_NAME" ]]; then
        # Try to find any DMG
        DMG_NAME=$(ls ChimeraRS_*.dmg 2>/dev/null | head -1)
        if [[ -z "$DMG_NAME" ]]; then
            error "No DMG found. Run without --skip-build first."
        fi
        warn "Using existing DMG: ${DMG_NAME}"
    fi
fi

# ═════════════════════════════════════════════════════════════════════════════
#  DRY RUN CHECK
# ═════════════════════════════════════════════════════════════════════════════

if [[ "$DRY_RUN" == true ]]; then
    echo ""
    echo "═══════════════════════════════════════════════════════════════════"
    echo "  DRY RUN - Would perform:"
    echo "═══════════════════════════════════════════════════════════════════"
    echo ""
    echo "  1. Create git tag: ${TAG}"
    echo "  2. Create GitHub release: ${TAG}"
    echo "  3. Upload: ${DMG_NAME}"
    echo ""
    echo "  Release notes preview:"
    echo "───────────────────────────────────────────────────────────────────"
    generate_changelog "$VERSION" "$TAG" | head -20
    echo "───────────────────────────────────────────────────────────────────"
    echo ""
    exit 0
fi

# ═════════════════════════════════════════════════════════════════════════════
#  GIT TAG
# ═════════════════════════════════════════════════════════════════════════════

if [[ "$UPLOAD_ONLY" == false ]]; then
    log "Creating git tag ${TAG}..."
    
    # Check if tag exists
    if git rev-parse "$TAG" >/dev/null 2>&1; then
        warn "Tag ${TAG} already exists"
        read -p "Delete and recreate? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            git tag -d "$TAG"
            git push origin :refs/tags/"$TAG" 2>/dev/null || true
        else
            error "Tag already exists. Use a different version."
        fi
    fi
    
    git tag -a "$TAG" -m "Release ${TAG}"
    log "Pushing tag to origin..."
    git push origin "$TAG"
    success "Tag ${TAG} created"
fi

# ═════════════════════════════════════════════════════════════════════════════
#  GITHUB RELEASE
# ═════════════════════════════════════════════════════════════════════════════

if [[ "${SKIP_UPLOAD:-false}" == false ]]; then
    log "Creating GitHub release..."
    
    # Generate changelog
    CHANGELOG=$(generate_changelog "$VERSION" "$TAG")
    
    # Check if release exists
    if gh release view "$TAG" --repo "$REPO" &>/dev/null; then
        warn "Release ${TAG} already exists"
        read -p "Delete and recreate? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            gh release delete "$TAG" --repo "$REPO" --yes
        else
            error "Release already exists."
        fi
    fi
    
    # Create release with DMG
    gh release create "$TAG" \
        --repo "$REPO" \
        --title "ChimeraRS ${TAG}" \
        --notes "$CHANGELOG" \
        "$DMG_NAME"
    
    success "Release published: https://github.com/${REPO}/releases/tag/${TAG}"
else
    warn "Skipping GitHub upload"
    echo "To upload manually:"
    echo "  gh release create ${TAG} --repo ${REPO} --title 'ChimeraRS ${TAG}' ${DMG_NAME}"
fi

# ═════════════════════════════════════════════════════════════════════════════
#  SUMMARY
# ═════════════════════════════════════════════════════════════════════════════

echo ""
echo "═══════════════════════════════════════════════════════════════════════"
echo "  Release Complete!"
echo "═══════════════════════════════════════════════════════════════════════"
echo ""
echo "  Version:  ${VERSION}"
echo "  Tag:      ${TAG}"
echo "  DMG:      ${DMG_NAME} ($(du -h "$DMG_NAME" | cut -f1))"
echo ""
echo "  Artifacts:"
echo "    - target/release/Chimera.app"
echo "    - ${DMG_NAME}"
echo ""
if [[ "${SKIP_UPLOAD:-false}" == false ]]; then
    echo "  Release URL:"
    echo "    https://github.com/${REPO}/releases/tag/${TAG}"
fi
echo ""
