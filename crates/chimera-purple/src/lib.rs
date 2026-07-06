//! `chimera-purple` — clean-room reimplementation of Apple's internal
//! **PurpleSNIFF** diagnostic reader and **Purple Restore** boot-into-
//! diagnostics flows.
//!
//! ## Background
//!
//! PurpleSNIFF is the factory-floor diagnostic tool used by Foxconn /
//! Pegatron technicians and Apple engineers. It reads identification,
//! sensor calibration, NAND, baseband, modem, audio, and test-station
//! data through the standard usbmuxd → lockdownd → mobilegestalt /
//! diagnostics_relay pipeline that is already present on every iOS device.
//!
//! Purple Restore is a sibling tool that boots a device into Apple's
//! internal "Purple" diagnostic OS image — typically used during the
//! manufacturing test process and field-engineer servicing.
//!
//! ## What this crate does
//!
//! We do **not** ship or distribute Apple's internal binaries. We talk to
//! the device using the *same public lockdownd / diagnostics_relay /
//! mobilegestalt service ports* that any libimobiledevice tool would use
//! (`ideviceinfo -q DIAGNOSTICS_RELAY`, `ideviceinfo -k SerialNumber`,
//! and so on). The result is the same data set PurpleSNIFF displays:
//!
//!   - **SNIFF**       — report metadata, host, time
//!   - **Battery**     — charge level, cycle count, temperature
//!   - **SysCfg**      — sensor calibration, NAND size, factory color
//!   - **Wireless**    — Wi-Fi MAC, Bluetooth MAC, Bonjour name
//!   - **Diagnostic**  — every per-station test pass/fail timestamp
//!   - **Debug**       — lockdown + CLTM thermal logs
//!   - **Developer**   — whether the developer disk image is mounted
//!   - **Device Mode** — OSMode / Recovery / DFU / Diagnostic
//!
//! ## Modules
//!
//! - [`sniff`]   — full report builder (`PurpleSnifReport`)
//! - [`restore`] — Purple Restore (boot-into-Diagnostics) flow
//! - [`syscfg`]  — `SysCfg` block parsing (NAND, calibration, factory data)
//! - [`battery`] — `BatteryStats` reader (gas-gauge + thermal extended)
//! - [`station`] — Factory test-station timeline (300+ stations possible)
//! - [`mode`]    — `DeviceMode` detection (OSMode / Recovery / DFU / Diag)

pub mod sniff;
pub mod restore;
pub mod syscfg;
pub mod battery;
pub mod station;
pub mod mode;

pub use sniff::{PurpleSniffReport, SniffSection, sniff};
pub use restore::{PurpleRestoreFlow, PurpleRestoreOptions, run as purple_restore};
pub use mode::{DeviceMode, detect_mode};

/// Crate-wide result alias.
pub type Result<T> = std::result::Result<T, PurpleError>;

#[derive(Debug, thiserror::Error)]
pub enum PurpleError {
    #[error("libimobiledevice error: {0}")]
    Imobile(#[from] chimera_imobile::ImobileError),
    #[error("plist parse: {0}")]
    Plist(#[from] plist::Error),
    #[error("device not in expected mode (need {expected:?}, found {actual:?})")]
    WrongMode { expected: mode::DeviceMode, actual: mode::DeviceMode },
    #[error("operation refused: {0}")]
    Refused(String),
    #[error("{0}")]
    Other(String),
}
