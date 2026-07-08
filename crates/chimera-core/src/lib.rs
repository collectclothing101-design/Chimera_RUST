// Core types, traits, error handling, and shared utilities for ChimeraRS
// Full open-source reimplementation — no login, no credits, no server required.
#![allow(unused_imports, dead_code)]

pub mod error;
pub mod device;
pub mod protocol;
pub mod imei;
pub mod progress;
pub mod event;
pub mod usb;
pub mod backup;
pub mod certificate;
pub mod firmware_meta;
pub mod session;
pub mod mac_address;
pub mod crypto;
pub mod diagnostics;

pub use error::{ChimeraError, Result};
pub use device::{ConnectionMode, DeviceBrand, DeviceChipset, DeviceInfo, DeviceState};
pub use event::{ChimeraEvent, EventBus};
pub use progress::{Progress, ProgressReporter, ProgressSender};

/// Version of the ChimeraRS tool
pub const VERSION: &str = "1.3.13";
pub const APP_NAME: &str = "ChimeraRS";
pub const APP_AUTHOR: &str = "Open Source Community";
pub const APP_DESC: &str = "Professional mobile device management tool. No login. No credits. No restrictions.";
