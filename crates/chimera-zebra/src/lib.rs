//! `chimera-zebra` — Zebra Technologies enterprise-handheld support.
//!
//! Covers the **TC52**, **TC52x**, **TC52ax**, **TC53**, **TC53e** and
//! **TC53-RFID** families of rugged Android handhelds. These devices share
//! a common Qualcomm-Snapdragon platform (SD660 → SDM6375) with Zebra's
//! Mobility Extensions (MX) stack layered on top of stock AOSP.
//!
//! ## What this crate exposes
//!
//! - [`models`]    — device model identification + USB VID/PID table
//! - [`enumerate`] — read every Zebra/OEM/AOSP property via ADB
//! - [`emm`]       — detect the installed EMM agent and dump device-owner
//!                   state
//! - [`stagenow`]  — generate StageNow profile XML for first-boot
//!                   provisioning
//! - [`rxlogger`]  — start / stop / dump RxLogger diagnostic captures
//! - [`datawedge`] — DataWedge profile query + scanner configuration
//! - [`partitions`]— UFS partition map enumeration (A/B + dynamic super)
//! - [`firmware`]  — sideload, OTA package validation, A/B slot management
//! - [`edl`]       — EDL / Sahara / Firehose entry helpers
//! - [`debug`]     — DBG console + diagnostic surfaces
//!
//! ## Ethical & legal posture
//!
//! Every operation is intended for **authorized service work** on devices
//! you own or are explicitly authorized to service. The crate does NOT
//! include methods to defeat Android Enterprise device-owner enrollment,
//! FRP without the original account, signed Zebra Firehose programmers,
//! or bootloader-unlock keys.

pub mod models;
pub mod enumerate;
pub mod emm;
pub mod stagenow;
pub mod rxlogger;
pub mod datawedge;
pub mod partitions;
pub mod firmware;
pub mod edl;
pub mod debug;

pub use models::{ZebraModel, ZebraVariant, identify_model, ZEBRA_USB_DB};
pub use enumerate::{ZebraDeviceInfo, enumerate_device};
pub use emm::{EmmAgent, EmmDetection, detect_emm};
pub use rxlogger::{RxLoggerStatus, start_rxlogger, stop_rxlogger, pull_rxlogger_dump};
pub use partitions::{Partition, PartitionMap, read_partition_map};
pub use firmware::{FirmwareSideload, sideload, validate_zebra_package};
pub use edl::{EdlEntryMethod, enter_edl};
pub use debug::{DebugDump, collect_debug_dump};

pub type Result<T> = std::result::Result<T, ZebraError>;

#[derive(Debug, thiserror::Error)]
pub enum ZebraError {
    #[error("device not recognised as a Zebra handheld (got vid={vid:#06x} pid={pid:#06x})")]
    NotZebraDevice { vid: u16, pid: u16 },
    #[error("ADB error: {0}")]
    Adb(String),
    #[error("fastboot error: {0}")]
    Fastboot(String),
    #[error("EDL error: {0}")]
    Edl(String),
    #[error("firmware package invalid: {0}")]
    InvalidPackage(String),
    #[error("operation not permitted on stock-locked Zebra firmware: {0}")]
    OemLocked(String),
    #[error("EMM enrollment present — refusing to proceed: {0}")]
    EmmEnrolled(String),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}
