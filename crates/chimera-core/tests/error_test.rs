// chimera-core/tests/error_test.rs
// Unit tests for ChimeraError serialization, display, and conversion.

use chimera_core::error::ChimeraError;

// ═════════════════════════════════════════════════════════════════════════════
//  ERROR DISPLAY TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn error_display_usb() {
    let err = ChimeraError::Usb("device busy".into());
    assert_eq!(format!("{}", err), "USB error: device busy");
}

#[test]
fn error_display_device_not_found() {
    let err = ChimeraError::DeviceNotFound("ABC123".into());
    assert_eq!(format!("{}", err), "Device not found: ABC123");
}

#[test]
fn error_display_device_disconnected() {
    let err = ChimeraError::DeviceDisconnected;
    assert_eq!(format!("{}", err), "Device disconnected");
}

#[test]
fn error_display_connection_timeout() {
    let err = ChimeraError::ConnectionTimeout { timeout_ms: 5000 };
    assert_eq!(format!("{}", err), "Connection timeout after 5000ms");
}

#[test]
fn error_display_communication() {
    let err = ChimeraError::Communication("protocol mismatch".into());
    assert_eq!(format!("{}", err), "Communication error: protocol mismatch");
}

#[test]
fn error_display_adb() {
    let err = ChimeraError::Adb("daemon not running".into());
    assert_eq!(format!("{}", err), "ADB error: daemon not running");
}

#[test]
fn error_display_adb_command_failed() {
    let err = ChimeraError::AdbCommandFailed {
        cmd: "shell getprop".into(),
        output: "error: device offline".into(),
    };
    assert_eq!(format!("{}", err), "ADB command failed: shell getprop => error: device offline");
}

#[test]
fn error_display_adb_auth_failed() {
    let err = ChimeraError::AdbAuthFailed;
    assert_eq!(format!("{}", err), "ADB authentication failed");
}

#[test]
fn error_display_fastboot() {
    let err = ChimeraError::Fastboot("flash failed".into());
    assert_eq!(format!("{}", err), "Fastboot error: flash failed");
}

#[test]
fn error_display_fastboot_failed() {
    let err = ChimeraError::FastbootFailed {
        cmd: "flash boot".into(),
        response: "FAILED (remote: device error)".into(),
    };
    assert_eq!(format!("{}", err), "Fastboot command failed: flash boot => FAILED (remote: device error)");
}

#[test]
fn error_display_edl() {
    let err = ChimeraError::Edl("connection refused".into());
    assert_eq!(format!("{}", err), "EDL error: connection refused");
}

#[test]
fn error_display_sahara() {
    let err = ChimeraError::Sahara("invalid command".into());
    assert_eq!(format!("{}", err), "Sahara protocol error: invalid command");
}

#[test]
fn error_display_firehose() {
    let err = ChimeraError::Firehose("NACK received".into());
    assert_eq!(format!("{}", err), "FIREHOSE error: NACK received");
}

#[test]
fn error_display_odin() {
    let err = ChimeraError::Odin("handshake failed".into());
    assert_eq!(format!("{}", err), "ODIN protocol error: handshake failed");
}

#[test]
fn error_display_samsung() {
    let err = ChimeraError::Samsung("FRP reset failed".into());
    assert_eq!(format!("{}", err), "Samsung operation failed: FRP reset failed");
}

#[test]
fn error_display_xiaomi() {
    let err = ChimeraError::Xiaomi("Mi account bypass failed".into());
    assert_eq!(format!("{}", err), "Xiaomi operation failed: Mi account bypass failed");
}

#[test]
fn error_display_huawei() {
    let err = ChimeraError::Huawei("ID lock removal failed".into());
    assert_eq!(format!("{}", err), "Huawei operation failed: ID lock removal failed");
}

#[test]
fn error_display_mtk() {
    let err = ChimeraError::Mtk("DA upload failed".into());
    assert_eq!(format!("{}", err), "MediaTek DA error: DA upload failed");
}

#[test]
fn error_display_unisoc() {
    let err = ChimeraError::Unisoc("PAC flash failed".into());
    assert_eq!(format!("{}", err), "Unisoc/SPD error: PAC flash failed");
}

#[test]
fn error_display_firmware() {
    let err = ChimeraError::Firmware("invalid format".into());
    assert_eq!(format!("{}", err), "Firmware error: invalid format");
}

#[test]
fn error_display_checksum_mismatch() {
    let err = ChimeraError::ChecksumMismatch {
        expected: "abc123".into(),
        actual: "def456".into(),
    };
    assert_eq!(format!("{}", err), "Firmware checksum mismatch: expected abc123, got def456");
}

#[test]
fn error_display_unsupported_format() {
    let err = ChimeraError::UnsupportedFormat(".rar".into());
    assert_eq!(format!("{}", err), "Unsupported firmware format: .rar");
}

#[test]
fn error_display_invalid_imei() {
    let err = ChimeraError::InvalidImei("too short".into());
    assert_eq!(format!("{}", err), "Invalid IMEI: too short");
}

#[test]
fn error_display_imei_repair_failed() {
    let err = ChimeraError::ImeiRepairFailed("root required".into());
    assert_eq!(format!("{}", err), "IMEI repair failed: root required");
}

#[test]
fn error_display_certificate() {
    let err = ChimeraError::Certificate("expired".into());
    assert_eq!(format!("{}", err), "Certificate error: expired");
}

#[test]
fn error_display_frp_failed() {
    let err = ChimeraError::FrpFailed("unsupported model".into());
    assert_eq!(format!("{}", err), "FRP removal failed: unsupported model");
}

#[test]
fn error_display_operation_not_supported() {
    let err = ChimeraError::OperationNotSupported;
    assert_eq!(format!("{}", err), "Operation not supported for this device");
}

#[test]
fn error_display_operation_cancelled() {
    let err = ChimeraError::OperationCancelled;
    assert_eq!(format!("{}", err), "Operation cancelled by user");
}

#[test]
fn error_display_operation_failed() {
    let err = ChimeraError::OperationFailed("timeout".into());
    assert_eq!(format!("{}", err), "Operation failed: timeout");
}

#[test]
fn error_display_io() {
    let err = ChimeraError::Io("permission denied".into());
    assert_eq!(format!("{}", err), "IO error: permission denied");
}

#[test]
fn error_display_file_not_found() {
    let err = ChimeraError::FileNotFound("/tmp/test.bin".into());
    assert_eq!(format!("{}", err), "File not found: /tmp/test.bin");
}

#[test]
fn error_display_parse() {
    let err = ChimeraError::Parse("invalid JSON".into());
    assert_eq!(format!("{}", err), "Parse error: invalid JSON");
}

#[test]
fn error_display_unknown() {
    let err = ChimeraError::Unknown("something went wrong".into());
    assert_eq!(format!("{}", err), "Unknown error: something went wrong");
}

// ═════════════════════════════════════════════════════════════════════════════
//  ERROR CLONE TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn error_clone_usb() {
    let err = ChimeraError::Usb("test".into());
    let cloned = err.clone();
    assert_eq!(format!("{}", err), format!("{}", cloned));
}

#[test]
fn error_clone_device_not_found() {
    let err = ChimeraError::DeviceNotFound("test".into());
    let cloned = err.clone();
    assert_eq!(format!("{}", err), format!("{}", cloned));
}

#[test]
fn error_clone_device_disconnected() {
    let err = ChimeraError::DeviceDisconnected;
    let cloned = err.clone();
    assert_eq!(format!("{}", err), format!("{}", cloned));
}

#[test]
fn error_clone_connection_timeout() {
    let err = ChimeraError::ConnectionTimeout { timeout_ms: 1000 };
    let cloned = err.clone();
    assert_eq!(format!("{}", err), format!("{}", cloned));
}

#[test]
fn error_clone_adb_command_failed() {
    let err = ChimeraError::AdbCommandFailed {
        cmd: "test".into(),
        output: "error".into(),
    };
    let cloned = err.clone();
    assert_eq!(format!("{}", err), format!("{}", cloned));
}

// ═════════════════════════════════════════════════════════════════════════════
//  ERROR CONVERSION TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let chimera_err: ChimeraError = io_err.into();
    assert!(matches!(chimera_err, ChimeraError::Io(_)));
}

#[test]
fn error_from_io_permission() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let chimera_err: ChimeraError = io_err.into();
    assert!(matches!(chimera_err, ChimeraError::Io(_)));
}

#[test]
fn error_from_io_connection_refused() {
    let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
    let chimera_err: ChimeraError = io_err.into();
    assert!(matches!(chimera_err, ChimeraError::Io(_)));
}

#[test]
fn error_from_io_timeout() {
    let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out");
    let chimera_err: ChimeraError = io_err.into();
    assert!(matches!(chimera_err, ChimeraError::Io(_)));
}

#[test]
fn error_from_json() {
    let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let chimera_err: ChimeraError = json_err.into();
    assert!(matches!(chimera_err, ChimeraError::Parse(_)));
}

#[test]
fn error_from_anyhow() {
    let anyhow_err = anyhow::anyhow!("test error");
    let chimera_err: ChimeraError = anyhow_err.into();
    assert!(matches!(chimera_err, ChimeraError::Unknown(_)));
}

// ═════════════════════════════════════════════════════════════════════════════
//  ERROR DEBUG TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn error_debug_usb() {
    let err = ChimeraError::Usb("test".into());
    let debug = format!("{:?}", err);
    assert!(debug.contains("Usb"));
    assert!(debug.contains("test"));
}

#[test]
fn error_debug_device_not_found() {
    let err = ChimeraError::DeviceNotFound("ABC123".into());
    let debug = format!("{:?}", err);
    assert!(debug.contains("DeviceNotFound"));
    assert!(debug.contains("ABC123"));
}

#[test]
fn error_debug_device_disconnected() {
    let err = ChimeraError::DeviceDisconnected;
    let debug = format!("{:?}", err);
    assert!(debug.contains("DeviceDisconnected"));
}

#[test]
fn error_debug_connection_timeout() {
    let err = ChimeraError::ConnectionTimeout { timeout_ms: 5000 };
    let debug = format!("{:?}", err);
    assert!(debug.contains("ConnectionTimeout"));
    assert!(debug.contains("5000"));
}

// ═════════════════════════════════════════════════════════════════════════════
//  ERROR JSON SERIALIZATION TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn error_json_usb() {
    let err = ChimeraError::Usb("test error".into());
    let json = serde_json::to_string(&err).unwrap();
    // JSON serialization may vary, just verify it contains the message
    assert!(json.contains("test error"));
}

#[test]
fn error_json_device_not_found() {
    let err = ChimeraError::DeviceNotFound("XYZ789".into());
    let json = serde_json::to_string(&err).unwrap();
    assert!(json.contains("XYZ789"));
}

#[test]
fn error_json_device_disconnected() {
    let err = ChimeraError::DeviceDisconnected;
    let json = serde_json::to_string(&err).unwrap();
    // Just verify it serializes without error
    assert!(!json.is_empty());
}

#[test]
fn error_json_connection_timeout() {
    let err = ChimeraError::ConnectionTimeout { timeout_ms: 3000 };
    let json = serde_json::to_string(&err).unwrap();
    assert!(json.contains("3000"));
}

#[test]
fn error_json_adb_command_failed() {
    let err = ChimeraError::AdbCommandFailed {
        cmd: "shell".into(),
        output: "error".into(),
    };
    let json = serde_json::to_string(&err).unwrap();
    assert!(json.contains("shell"));
    assert!(json.contains("error"));
}

// ═════════════════════════════════════════════════════════════════════════════
//  ERROR ROUND-TRIP TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn error_roundtrip_usb() {
    let err = ChimeraError::Usb("test".into());
    let json = serde_json::to_string(&err).unwrap();
    let back: ChimeraError = serde_json::from_str(&json).unwrap();
    assert_eq!(format!("{}", err), format!("{}", back));
}

#[test]
fn error_roundtrip_device_not_found() {
    let err = ChimeraError::DeviceNotFound("test".into());
    let json = serde_json::to_string(&err).unwrap();
    let back: ChimeraError = serde_json::from_str(&json).unwrap();
    assert_eq!(format!("{}", err), format!("{}", back));
}

#[test]
fn error_roundtrip_device_disconnected() {
    let err = ChimeraError::DeviceDisconnected;
    let json = serde_json::to_string(&err).unwrap();
    let back: ChimeraError = serde_json::from_str(&json).unwrap();
    assert_eq!(format!("{}", err), format!("{}", back));
}

#[test]
fn error_roundtrip_connection_timeout() {
    let err = ChimeraError::ConnectionTimeout { timeout_ms: 5000 };
    let json = serde_json::to_string(&err).unwrap();
    let back: ChimeraError = serde_json::from_str(&json).unwrap();
    assert_eq!(format!("{}", err), format!("{}", back));
}

#[test]
fn error_roundtrip_operation_failed() {
    let err = ChimeraError::OperationFailed("test".into());
    let json = serde_json::to_string(&err).unwrap();
    let back: ChimeraError = serde_json::from_str(&json).unwrap();
    assert_eq!(format!("{}", err), format!("{}", back));
}
