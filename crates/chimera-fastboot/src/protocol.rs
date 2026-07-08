// chimera-fastboot/src/protocol.rs
// Fastboot USB protocol constants and message types

use chimera_core::error::{ChimeraError, Result};

pub const FASTBOOT_MAX_DOWNLOAD_SIZE: u64 = 512 * 1024 * 1024; // 512MB
pub const USB_TIMEOUT_MS: u64 = 30000;
pub const FLASH_TIMEOUT_MS: u64 = 120000;

/// Fastboot response types
#[derive(Debug, Clone, PartialEq)]
pub enum FastbootResponse {
    Okay(String),
    Fail(String),
    Info(String),
    Data(u32),   // data length to transfer
}

impl FastbootResponse {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 4 {
            return Err(ChimeraError::Fastboot("Response too short".into()));
        }
        let prefix = &data[..4];
        let message = String::from_utf8_lossy(&data[4..]).to_string();
        
        match prefix {
            b"OKAY" => Ok(FastbootResponse::Okay(message)),
            b"FAIL" => Ok(FastbootResponse::Fail(message)),
            b"INFO" => Ok(FastbootResponse::Info(message)),
            b"DATA" => {
                let len = u32::from_str_radix(message.trim(), 16)
                    .map_err(|e| ChimeraError::Fastboot(format!("DATA parse: {}", e)))?;
                Ok(FastbootResponse::Data(len))
            }
            _ => Err(ChimeraError::Fastboot(format!("Unknown response: {:?}", prefix))),
        }
    }

    pub fn is_okay(&self) -> bool {
        matches!(self, FastbootResponse::Okay(_))
    }

    pub fn message(&self) -> &str {
        match self {
            FastbootResponse::Okay(m) | FastbootResponse::Fail(m) | FastbootResponse::Info(m) => m,
            FastbootResponse::Data(_) => "DATA",
        }
    }
}

/// High-level fastboot commands
#[derive(Debug, Clone)]
pub enum FastbootCommand {
    GetVar(String),
    Download(Vec<u8>),
    Flash(String),           // partition name
    Erase(String),
    Boot,
    Continue,
    Reboot,
    RebootBootloader,
    RebootFastbootd,
    RebootRecovery,
    PowerDown,
    SetActive(String),       // a/b slot
    FlashLock(bool),         // true=lock, false=unlock
    OemCommand(String),
    Custom(String),
}

impl FastbootCommand {
    pub fn to_wire(&self) -> String {
        match self {
            FastbootCommand::GetVar(var) => format!("getvar:{}", var),
            FastbootCommand::Flash(partition) => format!("flash:{}", partition),
            FastbootCommand::Erase(partition) => format!("erase:{}", partition),
            FastbootCommand::Download(data) => format!("download:{:08x}", data.len()),
            FastbootCommand::Boot => "boot".into(),
            FastbootCommand::Continue => "continue".into(),
            FastbootCommand::Reboot => "reboot".into(),
            FastbootCommand::RebootBootloader => "reboot-bootloader".into(),
            FastbootCommand::RebootFastbootd => "reboot-fastboot".into(),
            FastbootCommand::RebootRecovery => "reboot-recovery".into(),
            FastbootCommand::PowerDown => "powerdown".into(),
            FastbootCommand::SetActive(slot) => format!("set_active:{}", slot),
            FastbootCommand::FlashLock(lock) => {
                if *lock { "flashing lock".into() } else { "flashing unlock".into() }
            }
            FastbootCommand::OemCommand(cmd) => format!("oem {}", cmd),
            FastbootCommand::Custom(cmd) => cmd.clone(),
        }
    }
}

/// All known partition names by device type
pub struct PartitionTable;

impl PartitionTable {
    pub fn samsung_partitions() -> &'static [&'static str] {
        &["boot", "recovery", "system", "userdata", "cache", "efs", "param", "fota", "bootloader"]
    }
    
    pub fn xiaomi_qualcomm_partitions() -> &'static [&'static str] {
        &["boot", "recovery", "system", "vendor", "userdata", "cache", "efs", "abl", "xbl", "tz", "devcfg"]
    }
    
    pub fn common_partitions() -> &'static [&'static str] {
        &["boot", "recovery", "system", "vendor", "userdata", "cache", "super", "vbmeta"]
    }
}
