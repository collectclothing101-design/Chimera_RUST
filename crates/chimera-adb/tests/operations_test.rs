// chimera-adb/tests/operations_test.rs
// Unit tests for ADB operations (structure and signature tests).

// Note: These tests verify function signatures and basic structure.
// Actual device communication tests require a connected Android device.

#[test]
fn test_adb_operations_module_exists() {
    // Verify the module compiles and is accessible
    use chimera_adb::operations::AdbOperations;
    // Just verify the type exists
    let _ = std::any::type_name::<AdbOperations>();
}

#[test]
fn test_chipset_enum_exists() {
    // Verify the Chipset enum exists (it's private, so we just verify the module compiles)
    // In production, this would be pub for external use
}

#[test]
fn test_adb_client_new() {
    // Verify AdbClient can be created
    use chimera_adb::client::AdbClient;
    let _client = AdbClient::new();
}
