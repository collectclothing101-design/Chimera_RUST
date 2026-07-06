// chimera-core/src/error.rs
use thiserror::Error;
use serde::{Serialize, Deserialize};

pub type Result<T> = std::result::Result<T, ChimeraError>;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum ChimeraError {
    // USB / Communication
    #[error("USB error: {0}")]
    Usb(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Device disconnected")]
    DeviceDisconnected,

    #[error("Connection timeout after {timeout_ms}ms")]
    ConnectionTimeout { timeout_ms: u64 },

    #[error("Communication error: {0}")]
    Communication(String),

    // ADB
    #[error("ADB error: {0}")]
    Adb(String),

    #[error("ADB command failed: {cmd} => {output}")]
    AdbCommandFailed { cmd: String, output: String },

    #[error("ADB authentication failed")]
    AdbAuthFailed,

    // Fastboot
    #[error("Fastboot error: {0}")]
    Fastboot(String),

    #[error("Fastboot command failed: {cmd} => {response}")]
    FastbootFailed { cmd: String, response: String },

    // EDL (Emergency Download)
    #[error("EDL error: {0}")]
    Edl(String),

    #[error("Sahara protocol error: {0}")]
    Sahara(String),

    #[error("FIREHOSE error: {0}")]
    Firehose(String),

    // ODIN / Samsung Download Mode
    #[error("ODIN protocol error: {0}")]
    Odin(String),

    #[error("Samsung operation failed: {0}")]
    Samsung(String),

    // Xiaomi
    #[error("Xiaomi operation failed: {0}")]
    Xiaomi(String),

    // Huawei
    #[error("Huawei operation failed: {0}")]
    Huawei(String),

    // MTK
    #[error("MediaTek DA error: {0}")]
    Mtk(String),

    // Unisoc
    #[error("Unisoc/SPD error: {0}")]
    Unisoc(String),

    // Firmware
    #[error("Firmware error: {0}")]
    Firmware(String),

    #[error("Firmware checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Unsupported firmware format: {0}")]
    UnsupportedFormat(String),

    // IMEI
    #[error("Invalid IMEI: {0}")]
    InvalidImei(String),

    #[error("IMEI repair failed: {0}")]
    ImeiRepairFailed(String),

    // Certificate
    #[error("Certificate error: {0}")]
    Certificate(String),

    // FRP
    #[error("FRP removal failed: {0}")]
    FrpFailed(String),

    // Operations
    #[error("Operation not supported for this device")]
    OperationNotSupported,

    #[error("Operation cancelled by user")]
    OperationCancelled,

    #[error("Operation failed: {0}")]
    OperationFailed(String),

    // IO
    #[error("IO error: {0}")]
    Io(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    // Parsing
    #[error("Parse error: {0}")]
    Parse(String),

    // Generic
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<std::io::Error> for ChimeraError {
    fn from(e: std::io::Error) -> Self {
        ChimeraError::Io(e.to_string())
    }
}

impl From<rusb::Error> for ChimeraError {
    fn from(e: rusb::Error) -> Self {
        ChimeraError::Usb(e.to_string())
    }
}

impl From<serde_json::Error> for ChimeraError {
    fn from(e: serde_json::Error) -> Self {
        ChimeraError::Parse(e.to_string())
    }
}

impl From<anyhow::Error> for ChimeraError {
    fn from(e: anyhow::Error) -> Self {
        ChimeraError::Unknown(e.to_string())
    }
}
