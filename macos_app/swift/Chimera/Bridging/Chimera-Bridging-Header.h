//
//  Chimera-Bridging-Header.h
//  Auto-included by the Xcode project's SWIFT_OBJC_BRIDGING_HEADER setting.
//  Exposes the C-ABI of the Rust engine to Swift.
//
//  Pre-build phase:
//      Xcode runs `scripts/build_rust.sh` which calls
//      `cargo build -p chimera-ffi --release --target $(ARCH)-apple-darwin`
//      and copies libchimera_ffi.a + chimera_ffi.h into the Frameworks dir.
//

#ifndef Chimera_Bridging_Header_h
#define Chimera_Bridging_Header_h

#import "chimera_ffi.h"

#endif /* Chimera_Bridging_Header_h */
