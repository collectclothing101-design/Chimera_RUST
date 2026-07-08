#!/usr/bin/env bash
#
#  build_app.sh — assemble Chimera.app (Swift host + Rust engine + HTML UI).
#
#  Pipeline:
#      1. cargo build chimera-ffi (static + dynamic libs, per-arch)
#      2. swiftc compile all .swift sources, importing the bridging header
#      3. Bundle:  Chimera.app/Contents/{MacOS,Resources,Frameworks}
#      4. Codesign (Developer ID if present, ad-hoc otherwise)
#
#  Args:
#      --release      : optimised build (default: debug)
#      --universal    : build x86_64 + arm64 fat binary
#      --no-sign      : skip codesign step
#
set -euo pipefail

cd "$(dirname "$0")/.."

PROFILE="debug"
UNIVERSAL=0
SIGN=1
for arg in "$@"; do
    case "$arg" in
        --release)   PROFILE="release" ;;
        --universal) UNIVERSAL=1 ;;
        --no-sign)   SIGN=0 ;;
        -h|--help)   sed -n '2,18p' "$0"; exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 1 ;;
    esac
done

VERSION=$(awk -F'"' '/^version = /{print $2; exit}' crates/chimera-gui/Cargo.toml)
BUILD=$(date +%Y%m%d%H%M)

# ─── 1. Rust engine ─────────────────────────────────────────────────
./deploy/build_rust.sh "$PROFILE" "$UNIVERSAL"

# ─── 2. Setup Bundle Directories ─────────────────────────────────────
APP="target/$PROFILE/Chimera.app"
echo "→ Assembling structure for $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/idevice" "$APP/Contents/Frameworks"

# ─── 3. Swift host ──────────────────────────────────────────────────
echo "→ Building Swift host"
SWIFT_OUT="target/$PROFILE/chimera-swift"
mkdir -p "$SWIFT_OUT"

SWIFT_SOURCES=$(find macos_app/swift/Chimera/Sources -name '*.swift')
BRIDGING="macos_app/swift/Chimera/Bridging/Chimera-Bridging-Header.h"
INCLUDE_DIR="crates/chimera-ffi/include"

# Configure build flags based on target runtime optimization profile
SWIFT_FLAGS=(-O -import-objc-header "$BRIDGING" -I "$INCLUDE_DIR" -framework Cocoa -framework WebKit)
if [[ "$PROFILE" == "debug" ]]; then
    SWIFT_FLAGS=(-Onone -g -import-objc-header "$BRIDGING" -I "$INCLUDE_DIR" -framework Cocoa -framework WebKit)
fi

if [[ "$UNIVERSAL" == "1" ]]; then
    # Fix: Link individual Swift architecture slices against matching standalone Rust target slices
    for A in x86_64 arm64; do
        echo "  swiftc ($A)"
        RUST_TRIPLE="x86_64-apple-darwin"
        [[ "$A" == "arm64" ]] && RUST_TRIPLE="aarch64-apple-darwin"
        
        swiftc \
            -target ${A}-apple-macos10.14 \
            "${SWIFT_FLAGS[@]}" \
            -L "target/$RUST_TRIPLE/$PROFILE" \
            -lchimera_ffi \
            -o "$SWIFT_OUT/Chimera-$A" \
            $SWIFT_SOURCES
    done
    lipo -create -output "$APP/Contents/MacOS/Chimera" \
        "$SWIFT_OUT/Chimera-x86_64" "$SWIFT_OUT/Chimera-arm64"
else
    case "$(uname -m)" in
        arm64)  TRIPLE="aarch64-apple-darwin"; SWIFT_ARCH="arm64" ;;
        x86_64) TRIPLE="x86_64-apple-darwin";  SWIFT_ARCH="x86_64" ;;
    esac
    swiftc -target ${SWIFT_ARCH}-apple-macos10.14 \
        "${SWIFT_FLAGS[@]}" \
        -L "target/$TRIPLE/$PROFILE" \
        -lchimera_ffi \
        -o "$APP/Contents/MacOS/Chimera" \
        $SWIFT_SOURCES
fi
chmod +x "$APP/Contents/MacOS/Chimera"

# ─── 4. Bundle Resources ────────────────────────────────────────────
sed -e "s|\$(BUNDLE_VERSION)|$VERSION|g" \
    -e "s|\$(BUNDLE_BUILD)|$BUILD|g" \
    macos_app/Info.plist > "$APP/Contents/Info.plist"
printf "APPL????" > "$APP/Contents/PkgInfo"

cp docs/chimera-gui.html              "$APP/Contents/Resources/chimera-gui.html"
cp macos_app/swift/Chimera/Resources/bridge.js "$APP/Contents/Resources/bridge.js"
if [[ -f crates/chimera-gui/assets/AppIcon.icns ]]; then
    cp crates/chimera-gui/assets/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
fi

# ─── 5. libimobiledevice toolchain & rpath injection ────────────────
IDEVICE_DIR="$APP/Contents/Resources/idevice"
FRAMEWORKS_DIR="$APP/Contents/Frameworks"

VENDORED="vendor/idevice/$(uname -m)"
if [[ -d "$VENDORED" ]]; then
    echo "→ Bundling vendored libimobiledevice from $VENDORED"
    # Ensure dylibs reside properly in Frameworks, not Resources
    find "$VENDORED" -name "*.dylib" -exec cp {} "$FRAMEWORKS_DIR/" \;
    find "$VENDORED" -type f ! -name "*.dylib" -exec cp {} "$IDEVICE_DIR/" \;
else
    for brew_dir in /opt/homebrew/bin /usr/local/bin; do
        if [[ -x "$brew_dir/idevice_id" ]]; then
            echo "→ Bundling Homebrew libimobiledevice from $brew_dir"
            for tool in idevice_id ideviceinfo ideviceactivation idevicebackup2 \
                        idevicerestore idevicepair ideviceenterrecovery \
                        idevicediagnostics idevicedebug idevicename idevicedate \
                        idevicescreenshot idevicenotificationproxy idevicesyslog \
                        ideviceimagemounter ideviceprovision ideviceinstaller \
                        idevicecrashreport idevicesetlocation inetcat iproxy \
                        irecovery plistutil; do
                if [[ -x "$brew_dir/$tool" ]]; then
                    cp "$brew_dir/$tool" "$IDEVICE_DIR/$tool"
                fi
            done
            # Route dynamic framework files strictly to Contents/Frameworks/
            for lib in libimobiledevice-1.0.6.dylib libplist-2.0.4.dylib \
                       libusbmuxd-2.0.7.dylib libimobiledevice-glue-1.0.0.dylib \
                       libirecovery-1.0.0.dylib; do
                for src_lib in "$(dirname $brew_dir)/lib/$lib" "$brew_dir/../lib/$lib"; do
                    if [[ -f "$src_lib" ]]; then
                        cp "$src_lib" "$FRAMEWORKS_DIR/$lib"
                        break
                    fi
                done
            done
            break
        fi
    done
fi

# Sanitize write flags, map shared system libraries, and rewrite loader paths
if [[ -d "$IDEVICE_DIR" ]] && [[ -n "$(ls -A "$IDEVICE_DIR" 2>/dev/null)" ]]; then
    echo "→ Injecting relative framework paths to bundled CLI tools..."
    for tool in "$IDEVICE_DIR"/*; do
        if [[ -f "$tool" && -x "$tool" && "$tool" != *"MANIFEST.txt" ]]; then
            chmod 755 "$tool"
            install_name_tool -add_rpath "@executable_path/../../Frameworks" "$tool" 2>/dev/null || true
            
            for lib in libimobiledevice-1.0.6.dylib libplist-2.0.4.dylib \
                       libusbmuxd-2.0.7.dylib libimobiledevice-glue-1.0.0.dylib \
                       libirecovery-1.0.0.dylib; do
                install_name_tool -change "/opt/homebrew/opt/libimobiledevice/lib/$lib" "@rpath/$lib" "$tool" 2>/dev/null || true
                install_name_tool -change "/usr/local/opt/libimobiledevice/lib/$lib" "@rpath/$lib" "$tool" 2>/dev/null || true
                install_name_tool -change "/opt/homebrew/lib/$lib" "@rpath/$lib" "$tool" 2>/dev/null || true
                install_name_tool -change "/usr/local/lib/$lib" "@rpath/$lib" "$tool" 2>/dev/null || true
            done
        fi
    done
    ls "$IDEVICE_DIR" | sed 's/^/    /' | head -20
    ls "$IDEVICE_DIR" > "$IDEVICE_DIR/MANIFEST.txt"
else
    echo "→ Note: no libimobiledevice tools bundled. iOS workflows fall back to PATH."
    rmdir "$IDEVICE_DIR" 2>/dev/null || true
fi

# Clean framework load linkage layers
for lib in "$FRAMEWORKS_DIR"/*.dylib; do
    if [[ -f "$lib" ]]; then
        base_lib=$(basename "$lib")
        chmod 755 "$lib"
        install_name_tool -id "@rpath/$base_lib" "$lib"
        for dep in libimobiledevice-1.0.6.dylib libplist-2.0.4.dylib \
                   libusbmuxd-2.0.7.dylib libimobiledevice-glue-1.0.0.dylib \
                   libirecovery-1.0.0.dylib; do
            install_name_tool -change "/opt/homebrew/lib/$dep" "@rpath/$dep" "$lib" 2>/dev/null || true
            install_name_tool -change "/usr/local/lib/$dep" "@rpath/$dep" "$lib" 2>/dev/null || true
        done
    fi
done

# ─── 6. Codesign ────────────────────────────────────────────────────
if [[ "$SIGN" == "1" ]]; then
    ENTITLEMENTS="macos_app/entitlements.plist"
    if [[ ! -f "$ENTITLEMENTS" ]]; then
        echo "Warning: $ENTITLEMENTS not found. Removing explicit entitlement flags."
        ENTITLEMENTS=""
    fi

    # Phase 1: Sign the deepest sub-layer components first (Dynamic Frameworks)
    echo "→ Codesigning internal dynamic libraries..."
    for lib in "$FRAMEWORKS_DIR"/*.dylib; do
        if [[ -f "$lib" ]]; then
            codesign --force --timestamp --sign - "$lib"
        fi
    done
    
    # Phase 2: Sign structural executable dependencies (CLI utilities)
    echo "→ Codesigning embedded CLI tools..."
    if [[ -d "$IDEVICE_DIR" ]]; then
        for tool in "$IDEVICE_DIR"/*; do
            if [[ -f "$tool" && -x "$tool" && "$tool" != *"MANIFEST.txt" ]]; then
                codesign --force --timestamp --sign - "$tool"
            fi
        done
    fi

    # Phase 3: Apply structural signatures to parent bundle
    if security find-identity -v -p codesigning 2>/dev/null | grep -q "Developer ID Application"; then
        IDENTITY=$(security find-identity -v -p codesigning | grep "Developer ID Application" | head -1 | awk '{print $2}')
        echo "→ Codesigning App Bundle with Developer ID $IDENTITY (hardened runtime)"
        
        SIGN_FLAGS=(--force --options=runtime --deep --sign "$IDENTITY")
        [[ -n "$ENTITLEMENTS" ]] && SIGN_FLAGS+=(--entitlements "$ENTITLEMENTS")
        
        codesign "${SIGN_FLAGS[@]}" "$APP"
    else
        echo "→ Codesigning App Bundle via ad-hoc signatures"
        codesign --force --deep --sign - "$APP"
    fi
    
    echo "→ Verifying structural layout signature security..."
    codesign --verify --verbose=2 "$APP"
fi

echo
echo "✓ Built: $APP"
echo "  Open:  open $APP"
echo "  Size:  $(du -sh "$APP" | cut -f1)"
