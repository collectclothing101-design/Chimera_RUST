// chimera-ffi/tests/stress_test.rs
// FFI stress tests: 10k sequential + 1k concurrent dispatches.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::thread;

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
//  SEQUENTIAL STRESS TESTS (10k dispatches)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_10k_sequential_ping() {
    init_engine();
    let request = CString::new(r#"{"op":"ping"}"#).unwrap();

    for _ in 0..10_000 {
        let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
        let response = unsafe { c_str_to_string(response_ptr) };
        assert!(response.contains("ok"), "Ping should return ok");
    }
}

#[test]
fn stress_10k_sequential_version() {
    init_engine();
    let request = CString::new(r#"{"op":"version"}"#).unwrap();

    for _ in 0..10_000 {
        let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
        let response = unsafe { c_str_to_string(response_ptr) };
        assert!(response.contains("version"), "Version should return version info");
    }
}

#[test]
fn stress_10k_sequential_validate_imei() {
    init_engine();
    let request = CString::new(r#"{"op":"validate_imei","imei":"352099001761481"}"#).unwrap();

    for _ in 0..10_000 {
        let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
        let response = unsafe { c_str_to_string(response_ptr) };
        assert!(response.contains("valid"), "IMEI validation should return valid");
    }
}

#[test]
fn stress_10k_sequential_validate_mac() {
    init_engine();
    let request = CString::new(r#"{"op":"validate_mac","mac":"AA:BB:CC:DD:EE:FF"}"#).unwrap();

    for _ in 0..10_000 {
        let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
        let response = unsafe { c_str_to_string(response_ptr) };
        assert!(response.contains("valid"), "MAC validation should return valid");
    }
}

#[test]
fn stress_10k_sequential_mixed() {
    init_engine();
    let requests = vec![
        CString::new(r#"{"op":"ping"}"#).unwrap(),
        CString::new(r#"{"op":"version"}"#).unwrap(),
        CString::new(r#"{"op":"validate_imei","imei":"352099001761481"}"#).unwrap(),
        CString::new(r#"{"op":"validate_mac","mac":"AA:BB:CC:DD:EE:FF"}"#).unwrap(),
    ];

    for i in 0..10_000 {
        let request = &requests[i % requests.len()];
        let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
        let response = unsafe { c_str_to_string(response_ptr) };
        assert!(response.contains("ok") || response.contains("valid"),
                "Mixed dispatch {} should succeed", i);
    }
}

// ═════════════════════════════════════════════════════════════════════════════
//  CONCURRENT STRESS TESTS (1k concurrent dispatches)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_1k_concurrent_ping() {
    init_engine();
    let success_count = Arc::new(AtomicUsize::new(0));
    let fail_count = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..1_000)
        .map(|_| {
            let success = Arc::clone(&success_count);
            let fail = Arc::clone(&fail_count);
            thread::spawn(move || {
                let request = CString::new(r#"{"op":"ping"}"#).unwrap();
                let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
                let response = unsafe { c_str_to_string(response_ptr) };
                if response.contains("ok") {
                    success.fetch_add(1, Ordering::Relaxed);
                } else {
                    fail.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let successes = success_count.load(Ordering::Relaxed);
    let failures = fail_count.load(Ordering::Relaxed);

    assert_eq!(failures, 0, "Concurrent ping should have 0 failures");
    assert_eq!(successes, 1_000, "All 1000 pings should succeed");
}

#[test]
fn stress_1k_concurrent_mixed() {
    init_engine();
    let success_count = Arc::new(AtomicUsize::new(0));
    let fail_count = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..1_000)
        .map(|i| {
            let success = Arc::clone(&success_count);
            let fail = Arc::clone(&fail_count);
            thread::spawn(move || {
                let request = match i % 4 {
                    0 => CString::new(r#"{"op":"ping"}"#).unwrap(),
                    1 => CString::new(r#"{"op":"version"}"#).unwrap(),
                    2 => CString::new(r#"{"op":"validate_imei","imei":"352099001761481"}"#).unwrap(),
                    _ => CString::new(r#"{"op":"validate_mac","mac":"AA:BB:CC:DD:EE:FF"}"#).unwrap(),
                };
                let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
                let response = unsafe { c_str_to_string(response_ptr) };
                if response.contains("ok") || response.contains("valid") {
                    success.fetch_add(1, Ordering::Relaxed);
                } else {
                    fail.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let successes = success_count.load(Ordering::Relaxed);
    let failures = fail_count.load(Ordering::Relaxed);

    assert_eq!(failures, 0, "Concurrent mixed should have 0 failures");
    assert_eq!(successes, 1_000, "All 1000 mixed dispatches should succeed");
}

// ═════════════════════════════════════════════════════════════════════════════
//  RAPID INIT/VERSION TESTS (stress string allocator)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn stress_10k_version_calls() {
    init_engine();

    for _ in 0..10_000 {
        let version_ptr = unsafe { chimera_version() };
        let version = unsafe { c_str_to_string(version_ptr) };
        assert!(!version.is_empty(), "Version should not be empty");
    }
}

#[test]
fn stress_10k_init_calls() {
    // Repeated init should be safe (idempotent)
    for _ in 0..10_000 {
        let result = unsafe { chimera_init() };
        assert_eq!(result, 0, "Init should always return 0");
    }
}

// ═════════════════════════════════════════════════════════════════════════════
//  THROUGHPUT TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn throughput_ping_1s() {
    init_engine();
    let request = CString::new(r#"{"op":"ping"}"#).unwrap();
    let start = std::time::Instant::now();
    let mut count = 0;

    while start.elapsed() < std::time::Duration::from_secs(1) {
        let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
        let _ = unsafe { c_str_to_string(response_ptr) };
        count += 1;
    }

    println!("Throughput: {} pings/second", count);
    assert!(count > 100, "Should achieve >100 pings/second");
}
