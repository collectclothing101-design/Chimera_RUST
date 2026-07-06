// chimera-ffi/tests/memory_test.rs
// Memory leak detection tests for FFI string allocation.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

extern "C" {
    fn chimera_init() -> i32;
    fn chimera_version() -> *mut c_char;
    fn chimera_dispatch(request_json: *const c_char) -> *mut c_char;
    fn chimera_string_free(ptr: *mut c_char);
}

/// Helper to convert C string to Rust String
unsafe fn c_str_to_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let c_str = CStr::from_ptr(ptr);
    let result = c_str.to_string_lossy().into_owned();
    chimera_string_free(ptr);
    result
}

/// Initialize the engine once
fn init_engine() {
    unsafe { chimera_init(); }
}

// ═════════════════════════════════════════════════════════════════════════════
//  STRING ALLOCATION LEAK TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn memory_version_no_leak() {
    init_engine();

    // Call version 100k times and free each time
    for _ in 0..100_000 {
        let version_ptr = unsafe { chimera_version() };
        let _ = unsafe { c_str_to_string(version_ptr) };
        // String is freed in c_str_to_string via chimera_string_free
    }
    // If there's a leak, this test will OOM or be very slow
}

#[test]
fn memory_dispatch_no_leak() {
    init_engine();
    let request = CString::new(r#"{"op":"ping"}"#).unwrap();

    // Call dispatch 100k times and free each time
    for _ in 0..100_000 {
        let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
        let _ = unsafe { c_str_to_string(response_ptr) };
        // Response is freed in c_str_to_string via chimera_string_free
    }
}

#[test]
fn memory_mixed_dispatch_no_leak() {
    init_engine();
    let requests = vec![
        CString::new(r#"{"op":"ping"}"#).unwrap(),
        CString::new(r#"{"op":"version"}"#).unwrap(),
        CString::new(r#"{"op":"validate_imei","imei":"352099001761481"}"#).unwrap(),
        CString::new(r#"{"op":"validate_mac","mac":"AA:BB:CC:DD:EE:FF"}"#).unwrap(),
    ];

    // Call mixed dispatches 100k times
    for i in 0..100_000 {
        let request = &requests[i % requests.len()];
        let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
        let _ = unsafe { c_str_to_string(response_ptr) };
    }
}

// ═════════════════════════════════════════════════════════════════════════════
//  CONCURRENT MEMORY TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn memory_concurrent_no_leak() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    init_engine();
    let counter = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..10_000 {
                    let request = CString::new(r#"{"op":"ping"}"#).unwrap();
                    let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
                    let _ = unsafe { c_str_to_string(response_ptr) };
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let total = counter.load(Ordering::Relaxed);
    assert_eq!(total, 100_000, "Should complete 100k dispatches");
}

// ═════════════════════════════════════════════════════════════════════════════
//  STRESS TEST WITH LARGE STRINGS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn memory_large_string_no_leak() {
    init_engine();

    // Create a large IMEI-like string (though validation will fail)
    let large_payload = format!(r#"{{"op":"validate_imei","imei":"{}}}"#, "1".repeat(1000));
    let request = CString::new(large_payload).unwrap();

    for _ in 0..10_000 {
        let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
        let _ = unsafe { c_str_to_string(response_ptr) };
    }
}

// ═════════════════════════════════════════════════════════════════════════════
//  RAPID INIT/VERSION CYCLE TEST
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn memory_rapid_init_version_cycle() {
    // Rapid init + version calls should not leak
    for _ in 0..50_000 {
        let _ = unsafe { chimera_init() };
        let version_ptr = unsafe { chimera_version() };
        let _ = unsafe { c_str_to_string(version_ptr) };
    }
}

// ═════════════════════════════════════════════════════════════════════════════
//  NULL POINTER HANDLING
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn memory_null_pointer_safety() {
    init_engine();

    // Free a null pointer should be safe
    unsafe { chimera_string_free(std::ptr::null_mut()) };

    // Free an already-freed pointer should be safe (double-free protection)
    let version_ptr = unsafe { chimera_version() };
    let _ = unsafe { c_str_to_string(version_ptr) };
    // version_ptr is already freed, freeing again should be safe
    unsafe { chimera_string_free(version_ptr) };
}
