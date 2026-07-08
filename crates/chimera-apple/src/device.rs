// chimera-apple/src/device.rs
// Apple device detection, model identification, and hardware info.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Apple silicon / chipset families
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppleChipset {
    A4,   // iPhone 4
    A5,   // iPhone 4S
    A6,   // iPhone 5/5C
    A7,   // iPhone 5S  (checkm8-vulnerable)
    A8,   // iPhone 6/6+  (checkm8-vulnerable)
    A9,   // iPhone 6S/SE1  (checkm8-vulnerable)
    A10,  // iPhone 7  (checkm8-vulnerable)
    A11,  // iPhone 8/X  (checkm8-vulnerable – last vulnerable chip)
    A12,  // iPhone XS/XR
    A13,  // iPhone 11
    A14,  // iPhone 12
    A15,  // iPhone 13/14
    A16,  // iPhone 14 Pro / 15
    A17Pro, // iPhone 15 Pro
    A18,  // iPhone 16
    A18Pro, // iPhone 16 Pro
    A19,    // iPhone 17
    A19Pro, // iPhone 17 Pro
    M1,   // iPad Pro
    M2,
    M3,
    M4,
    Unknown,
}

impl AppleChipset {
    /// Returns true if this chipset is vulnerable to the checkm8 bootrom exploit.
    /// Affected: A5–A11 (iPhone 4S through iPhone X/8 Plus).
    pub fn is_checkm8_vulnerable(&self) -> bool {
        matches!(
            self,
            AppleChipset::A5
                | AppleChipset::A6
                | AppleChipset::A7
                | AppleChipset::A8
                | AppleChipset::A9
                | AppleChipset::A10
                | AppleChipset::A11
        )
    }

    /// Returns true for chips that support palera1n (rootful jailbreak on A9–A11, iOS 15–16).
    pub fn supports_palera1n(&self) -> bool {
        matches!(
            self,
            AppleChipset::A9 | AppleChipset::A10 | AppleChipset::A11
        )
    }
}

impl fmt::Display for AppleChipset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Current connection mode of an Apple device
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IosConnectionMode {
    /// Normal iOS operation – accessible via lockdownd
    Normal,
    /// Recovery mode (iBEC/iBoot) – limited restore commands
    Recovery,
    /// Device Firmware Update – bootrom-level, lowest-level access
    Dfu,
    /// Activation required – new/erased device waiting for iCloud activation
    ActivationRequired,
    /// Device locked with passcode, trust dialog needed
    Locked,
    /// iTunes/Finder pairing required
    PairingRequired,
    /// Unknown / not yet determined
    Unknown,
}

impl fmt::Display for IosConnectionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IosConnectionMode::Normal => write!(f, "Normal"),
            IosConnectionMode::Recovery => write!(f, "Recovery Mode"),
            IosConnectionMode::Dfu => write!(f, "DFU Mode"),
            IosConnectionMode::ActivationRequired => write!(f, "Activation Required"),
            IosConnectionMode::Locked => write!(f, "Locked"),
            IosConnectionMode::PairingRequired => write!(f, "Pairing Required"),
            IosConnectionMode::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Comprehensive Apple device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleDeviceInfo {
    pub udid: String,
    pub serial_number: String,
    pub model_identifier: String, // e.g. "iPhone14,3"
    pub model_name: String,       // e.g. "iPhone 13 Pro Max"
    pub chipset: AppleChipset,
    pub ios_version: Option<String>,
    pub build_version: Option<String>,
    pub connection_mode: IosConnectionMode,
    pub imei: Option<String>,
    pub imei2: Option<String>,
    pub meid: Option<String>,
    pub iccid: Option<String>,
    pub phone_number: Option<String>,
    pub wifi_address: Option<String>,
    pub bluetooth_address: Option<String>,
    pub hardware_model: Option<String>,
    pub cpu_architecture: Option<String>,
    pub is_activation_locked: bool,
    pub is_passcode_set: bool,
    pub is_jailbroken: Option<bool>,
    pub storage_gb: Option<u32>,
    pub battery_level: Option<u8>,
    pub carrier: Option<String>,
    pub region: Option<String>,
    /// MCC+MNC of the current carrier network
    pub mccmnc: Option<String>,
}

impl AppleDeviceInfo {
    pub fn new(udid: String) -> Self {
        Self {
            udid,
            serial_number: String::new(),
            model_identifier: String::new(),
            model_name: String::new(),
            chipset: AppleChipset::Unknown,
            ios_version: None,
            build_version: None,
            connection_mode: IosConnectionMode::Unknown,
            imei: None,
            imei2: None,
            meid: None,
            iccid: None,
            phone_number: None,
            wifi_address: None,
            bluetooth_address: None,
            hardware_model: None,
            cpu_architecture: None,
            is_activation_locked: false,
            is_passcode_set: false,
            is_jailbroken: None,
            storage_gb: None,
            battery_level: None,
            carrier: None,
            region: None,
            mccmnc: None,
        }
    }
}

/// Minimal handle returned by USB scan
#[derive(Debug, Clone)]
pub struct AppleDevice {
    pub usb_location: u32,
    pub vid: u16,
    pub pid: u16,
    pub info: AppleDeviceInfo,
}

/// Map a known Apple model identifier to a human-readable name and chipset.
pub fn resolve_model(identifier: &str) -> (String, AppleChipset) {
    match identifier {
        // ── iPhone ──────────────────────────────────────────────
        "iPhone12,8" => ("iPhone SE (2nd gen)".into(), AppleChipset::A13),
        "iPhone13,1" => ("iPhone 12 mini".into(), AppleChipset::A14),
        "iPhone13,2" => ("iPhone 12".into(), AppleChipset::A14),
        "iPhone13,3" => ("iPhone 12 Pro".into(), AppleChipset::A14),
        "iPhone13,4" => ("iPhone 12 Pro Max".into(), AppleChipset::A14),
        "iPhone14,4" => ("iPhone 13 mini".into(), AppleChipset::A15),
        "iPhone14,5" => ("iPhone 13".into(), AppleChipset::A15),
        "iPhone14,2" => ("iPhone 13 Pro".into(), AppleChipset::A15),
        "iPhone14,3" => ("iPhone 13 Pro Max".into(), AppleChipset::A15),
        "iPhone14,6" => ("iPhone SE (3rd gen)".into(), AppleChipset::A15),
        "iPhone14,7" => ("iPhone 14".into(), AppleChipset::A15),
        "iPhone14,8" => ("iPhone 14 Plus".into(), AppleChipset::A15),
        "iPhone15,2" => ("iPhone 14 Pro".into(), AppleChipset::A16),
        "iPhone15,3" => ("iPhone 14 Pro Max".into(), AppleChipset::A16),
        "iPhone15,4" => ("iPhone 15".into(), AppleChipset::A16),
        "iPhone15,5" => ("iPhone 15 Plus".into(), AppleChipset::A16),
        "iPhone16,1" => ("iPhone 15 Pro".into(), AppleChipset::A17Pro),
        "iPhone16,2" => ("iPhone 15 Pro Max".into(), AppleChipset::A17Pro),
        "iPhone17,3" => ("iPhone 16".into(),           AppleChipset::A18),
        "iPhone17,4" => ("iPhone 16 Plus".into(),        AppleChipset::A18),
        "iPhone17,1" => ("iPhone 16 Pro".into(),         AppleChipset::A18Pro),
        "iPhone17,2" => ("iPhone 16 Pro Max".into(),     AppleChipset::A18Pro),
        // iPhone 16e (2025 — budget A16)
        "iPhone17,5" => ("iPhone 16e".into(),            AppleChipset::A16),
        // Legacy checkm8-vulnerable
        "iPhone8,1"  => ("iPhone 6S".into(), AppleChipset::A9),
        "iPhone8,2"  => ("iPhone 6S Plus".into(), AppleChipset::A9),
        "iPhone8,4"  => ("iPhone SE (1st gen)".into(), AppleChipset::A9),
        "iPhone9,1"  => ("iPhone 7".into(), AppleChipset::A10),
        "iPhone9,3"  => ("iPhone 7".into(), AppleChipset::A10),
        "iPhone9,2"  => ("iPhone 7 Plus".into(), AppleChipset::A10),
        "iPhone9,4"  => ("iPhone 7 Plus".into(), AppleChipset::A10),
        "iPhone10,1" => ("iPhone 8".into(), AppleChipset::A11),
        "iPhone10,4" => ("iPhone 8".into(), AppleChipset::A11),
        "iPhone10,2" => ("iPhone 8 Plus".into(), AppleChipset::A11),
        "iPhone10,5" => ("iPhone 8 Plus".into(), AppleChipset::A11),
        "iPhone10,3" => ("iPhone X".into(), AppleChipset::A11),
        "iPhone10,6" => ("iPhone X".into(), AppleChipset::A11),
        "iPhone11,2" => ("iPhone XS".into(), AppleChipset::A12),
        "iPhone11,4" => ("iPhone XS Max".into(), AppleChipset::A12),
        "iPhone11,6" => ("iPhone XS Max".into(), AppleChipset::A12),
        "iPhone11,8" => ("iPhone XR".into(), AppleChipset::A12),
        "iPhone12,1" => ("iPhone 11".into(), AppleChipset::A13),
        "iPhone12,3" => ("iPhone 11 Pro".into(), AppleChipset::A13),
        "iPhone12,5" => ("iPhone 11 Pro Max".into(), AppleChipset::A13),
        // ── iPhone 17 series (2025) ──────────────────────────────────────
        // iPhone 17 Air – ultra-thin form factor (replaces Plus line)
        "iPhone18,5" => ("iPhone 17 Air".into(), AppleChipset::A19),
        // iPhone 17
        "iPhone18,3" => ("iPhone 17".into(), AppleChipset::A19),
        "iPhone18,4" => ("iPhone 17".into(), AppleChipset::A19),
        // iPhone 17 Pro
        "iPhone18,1" => ("iPhone 17 Pro".into(), AppleChipset::A19Pro),
        "iPhone18,2" => ("iPhone 17 Pro Max".into(), AppleChipset::A19Pro),

        // ── iPad Air 16 (M3, 2025) ───────────────────────────────────────
        "iPad14,8"   => ("iPad Air 11-inch (M3)".into(), AppleChipset::M3),
        "iPad14,9"   => ("iPad Air 13-inch (M3)".into(), AppleChipset::M3),
        // ── iPad Air 17 (M4, 2026) ───────────────────────────────────────
        "iPad16,3"   => ("iPad Air 11-inch (M4)".into(), AppleChipset::M4),
        "iPad16,4"   => ("iPad Air 13-inch (M4)".into(), AppleChipset::M4),
        // ── iPad Pro M4 (2024) ───────────────────────────────────────────
        "iPad16,5"   => ("iPad Pro 11-inch (M4)".into(), AppleChipset::M4),
        "iPad16,6"   => ("iPad Pro 13-inch (M4)".into(), AppleChipset::M4),
        // ── iPad mini 7 (A17 Pro, 2024) ──────────────────────────────────
        "iPad16,1"   => ("iPad mini 7".into(), AppleChipset::A17Pro),
        "iPad16,2"   => ("iPad mini 7".into(), AppleChipset::A17Pro),
        // ── iPad (11th gen, 2025) ────────────────────────────────────────
        "iPad14,10"  => ("iPad 11th generation".into(), AppleChipset::A16),
        "iPad14,11"  => ("iPad 11th generation".into(), AppleChipset::A16),
        _ => (format!("Apple Device ({})", identifier), AppleChipset::Unknown),
    }
}
