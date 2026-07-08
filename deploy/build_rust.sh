#!/usr/bin/env bash
#
#  build_rust.sh — compile chimera-ffi for one or both Apple architectures
#                  and lipo the result.
#
#  Used by:  deploy/build_app.sh
#  Outputs:
#      target/<arch>-apple-darwin/<profile>/libchimera_ffi.a   (per-arch)
#      target/universal/<profile>/libchimera_ffi.a             (with --universal)
#      target/<arch>-apple-darwin/<profile>/libchimera_ffi.dylib
#
set -euo pipefail

cd "$(dirname "$0")/.."     # workspace root

PROFILE="${1:-debug}"       # Default to debug to prevent production flags on dev cycles
UNIVERSAL="${2:-0}"         # 0 | 1

# Configure Cargo compiler arrays instead of risky unquoted strings
if [[ "$PROFILE" == "release" ]]; then
    CARGO_FLAGS=("--release")
    OUT="release"
else
    CARGO_FLAGS=()
    OUT="debug"
fi

build_arch() {
    local triple="$1"
    
    # 1. Target Guard: Automatically configure missing rustup architectures on clean environments
    if ! rustup target list --installed | grep -q "$triple"; then
        echo "→ Toolchain missing target. Running: rustup target add $triple"
        rustup target add "$triple"
    fi
    
    # 2. Cross-Compilation Environment Isolation
    # Map architectures to explicit clang compiler targets to prevent host bleed-through
    local target_arch="${triple%%-*}"
    [[ "$target_arch" == "aarch64" ]] && target_arch="arm64"
    
    export CFLAGS="-target ${target_arch}-apple-macos10.14"
    export CXXFLAGS="-target ${target_arch}-apple-macos10.14"
    
    echo "→ chimera-ffi: cargo build ${CARGO_FLAGS[*]} --target $triple"
    cargo build "${CARGO_FLAGS[@]}" --target "$triple" -p chimera-ffi
}

if [[ "$UNIVERSAL" == "1" ]]; then
    echo "→ Initializing parallel multi-architecture Rust compilation pipeline..."
    
    # Fire off compilation forks into background loops to build both architectures simultaneously
    build_arch x86_64-apple-darwin &
    PID_X86=$!
    
    build_arch aarch64-apple-darwin &
    PID_ARM=$!
    
    # Synchronize execution paths back into parent shell process
    wait "$PID_X86"
    wait "$PID_ARM"
    
    UNIVERSAL_DIR="target/universal/$OUT"
    mkdir -p "$UNIVERSAL_DIR"
    
    # Merge Static slices
    echo "→ lipo → $UNIVERSAL_DIR/libchimera_ffi.a"
    lipo -create -output "$UNIVERSAL_DIR/libchimera_ffi.a" \
        "target/x86_64-apple-darwin/$OUT/libchimera_ffi.a" \
        "target/aarch64-apple-darwin/$OUT/libchimera_ffi.a"
        
    # Merge Dynamic slices    
    echo "→ lipo → $UNIVERSAL_DIR/libchimera_ffi.dylib"
    lipo -create -output "$UNIVERSAL_DIR/libchimera_ffi.dylib" \
        "target/x86_64-apple-darwin/$OUT/libchimera_ffi.dylib" \
        "target/aarch64-apple-darwin/$OUT/libchimera_ffi.dylib"

    # Enforce standard dynamic framework execution permissions and inject local @rpath ID 
    # Without this, the final binary will look for target/x86_64... folders on the customer machine and crash.
    echo "→ Updating internal dylib install name ID for relative framework loading"
    chmod 755 "$UNIVERSAL_DIR/libchimera_ffi.dylib"
    install_name_tool -id "@rpath/libchimera_ffi.dylib" "$UNIVERSAL_DIR/libchimera_ffi.dylib"
else
    # Auto-detect host architecture with strict fallback handling
    case "$(uname -m)" in
        arm64)  build_arch aarch64-apple-darwin ;;
        x86_64) build_arch x86_64-apple-darwin ;;
        *)      echo "Error: Unknown architecture runtime profile $(uname -m)" >&2; exit 1 ;;
    esac
fi

echo "✓ chimera-ffi build complete (profile=$PROFILE, universal=$UNIVERSAL)"
