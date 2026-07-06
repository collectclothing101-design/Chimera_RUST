// chimera-ffi/tests/dispatch_test.rs
// Unit tests for FFI dispatch and C ABI functions.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// Import FFI functions
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

#[test]
fn test_chimera_init() {
    // Initialize the engine
    let result = unsafe { chimera_init() };
    assert_eq!(result, 0, "chimera_init should return 0 on success");
}

#[test]
fn test_chimera_version() {
    // Get version string
    let version_ptr = unsafe { chimera_version() };
    let version = unsafe { c_str_to_string(version_ptr) };
    assert!(!version.is_empty(), "Version should not be empty");
    // Version should contain a dot (e.g., "1.0.0")
    assert!(version.contains('.'), "Version should contain a dot: {}", version);
}

#[test]
fn test_chimera_dispatch_ping() {
    // Initialize first
    unsafe { chimera_init(); }

    // Send ping request
    let request = CString::new(r#"{"op":"ping"}"#).unwrap();
    let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
    let response = unsafe { c_str_to_string(response_ptr) };

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(response.contains("ok"), "Response should contain 'ok': {}", response);
}

#[test]
fn test_chimera_dispatch_version() {
    // Initialize first
    unsafe { chimera_init(); }

    // Send version request
    let request = CString::new(r#"{"op":"version"}"#).unwrap();
    let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
    let response = unsafe { c_str_to_string(response_ptr) };

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(response.contains("version"), "Response should contain 'version': {}", response);
}

#[test]
fn test_chimera_dispatch_invalid_json() {
    // Initialize first
    unsafe { chimera_init(); }

    // Send invalid JSON
    let request = CString::new("not json").unwrap();
    let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
    let response = unsafe { c_str_to_string(response_ptr) };

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(response.contains("err"), "Response should contain error: {}", response);
}

#[test]
fn test_chimera_dispatch_unknown_op() {
    // Initialize first
    unsafe { chimera_init(); }

    // Send unknown operation
    let request = CString::new(r#"{"op":"unknown_op"}"#).unwrap();
    let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
    let response = unsafe { c_str_to_string(response_ptr) };

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(response.contains("err"), "Response should contain error: {}", response);
}

#[test]
fn test_chimera_dispatch_validate_imei() {
    // Initialize first
    unsafe { chimera_init(); }

    // Send validate_imei request with valid IMEI
    let request = CString::new(r#"{"op":"validate_imei","imei":"352099001761481"}"#).unwrap();
    let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
    let response = unsafe { c_str_to_string(response_ptr) };

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(response.contains("valid"), "Response should contain 'valid': {}", response);
}

#[test]
fn test_chimera_dispatch_validate_imei_invalid() {
    // Initialize first
    unsafe { chimera_init(); }

    // Send validate_imei request with invalid IMEI
    let request = CString::new(r#"{"op":"validate_imei","imei":"12345"}"#).unwrap();
    let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
    let response = unsafe { c_str_to_string(response_ptr) };

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(response.contains("err"), "Response should contain error: {}", response);
}

#[test]
fn test_chimera_dispatch_validate_mac() {
    // Initialize first
    unsafe { chimera_init(); }

    // Send validate_mac request with valid MAC
    let request = CString::new(r#"{"op":"validate_mac","mac":"AA:BB:CC:DD:EE:FF"}"#).unwrap();
    let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
    let response = unsafe { c_str_to_string(response_ptr) };

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(response.contains("valid"), "Response should contain 'valid': {}", response);
}

#[test]
fn test_chimera_dispatch_validate_mac_invalid() {
    // Initialize first
    unsafe { chimera_init(); }

    // Send validate_mac request with invalid MAC
    let request = CString::new(r#"{"op":"validate_mac","mac":"invalid"}"#).unwrap();
    let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
    let response = unsafe { c_str_to_string(response_ptr) };

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(response.contains("err"), "Response should contain error: {}", response);
}

#[test]
fn test_chimera_dispatch_list_devices() {
    // Initialize first
    unsafe { chimera_init(); }

    // Send list_devices request
    let request = CString::new(r#"{"op":"list_devices"}"#).unwrap();
    let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
    let response = unsafe { c_str_to_string(response_ptr) };

    assert!(!response.is_empty(), "Response should not be empty");
    // Should return a valid JSON response (may be empty array if no devices)
    assert!(response.contains("devices") || response.contains("[]"), "Response should contain devices: {}", response);
}

#[test]
fn test_chimera_dispatch_drain_logs() {
    // Initialize first
    unsafe { chimera_init(); }

    // Send drain_logs request
    let request = CString::new(r#"{"op":"drain_logs"}"#).unwrap();
    let response_ptr = unsafe { chimera_dispatch(request.as_ptr()) };
    let response = unsafe { c_str_to_string(response_ptr) };

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(response.contains("ok"), "Response should contain 'ok': {}", response);
}
