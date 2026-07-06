//! Zebra TC52 / TC53 family identification.
//!
//! Maps USB VID/PID, build-fingerprint, and `ro.product.model` strings
//! onto a typed [`ZebraModel`] + [`ZebraVariant`]. Used by the dashboard
//! to render the correct "Detected device" pill and by the enumerate
//! module to drive per-model branches.

use serde::{Serialize, Deserialize};

/// Top-level Zebra model family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZebraModel {
    /// Original TC52 — Snapdragon 660 (MSM8956 Plus). Launched Android 8.1.
    Tc52,
    /// TC52x — refresh with USB-C, longer-range scanner, Android 10 launch.
    Tc52x,
    /// TC52ax — TC52x with SD662 + extended ruggedness rating.
    Tc52ax,
    /// TC53 — Snapdragon 6375. Launched Android 11. Wi-Fi only.
    Tc53,
    /// TC53e — TC53 with WWAN cellular modem (Verizon/AT&T/T-Mobile bands).
    Tc53e,
    /// TC53 Premium with embedded RFID UHF reader.
    Tc53Rfid,
    /// Unknown SKU — surfaced for diagnostics but no per-model branches.
    Unknown,
}

impl ZebraModel {
    /// Display name suitable for the GUI.
    pub fn display(&self) -> &'static str {
        match self {
            ZebraModel::Tc52     => "Zebra TC52",
            ZebraModel::Tc52x    => "Zebra TC52x",
            ZebraModel::Tc52ax   => "Zebra TC52ax",
            ZebraModel::Tc53     => "Zebra TC53",
            ZebraModel::Tc53e    => "Zebra TC53e (WWAN)",
            ZebraModel::Tc53Rfid => "Zebra TC53-RFID",
            ZebraModel::Unknown  => "Zebra (unidentified)",
        }
    }

    /// Qualcomm SoC family for this model.
    pub fn soc(&self) -> &'static str {
        match self {
            ZebraModel::Tc52     => "SD660 (MSM8956 Plus)",
            ZebraModel::Tc52x    => "SD660",
            ZebraModel::Tc52ax   => "SD662",
            ZebraModel::Tc53 | ZebraModel::Tc53e | ZebraModel::Tc53Rfid => "SDM6375",
            ZebraModel::Unknown  => "unknown",
        }
    }

    /// Maximum supported Android API level as of 2026.
    pub fn max_android_api(&self) -> u32 {
        match self {
            ZebraModel::Tc52                   => 30,  // Android 11
            ZebraModel::Tc52x | ZebraModel::Tc52ax => 33,  // Android 13
            ZebraModel::Tc53 | ZebraModel::Tc53e | ZebraModel::Tc53Rfid => 35,  // Android 15
            ZebraModel::Unknown                => 0,
        }
    }
}

/// Per-SKU connectivity / radio variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZebraVariant {
    WlanOnly,
    Wwan,
    WwanWithRfid,
    Unknown,
}

/// One row of the Zebra USB device table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZebraUsbId {
    pub vid:           u16,
    pub pid:           u16,
    pub mode_label:    &'static str,
    pub description:   &'static str,
}

/// Zebra-specific VID/PIDs across normal / Fastboot / EDL modes.
///
/// VID 0x05E0 is Symbol Technologies / Zebra (Symbol was acquired by
/// Motorola Solutions, then divested to Zebra in 2014; the legacy
/// "Symbol" VID is still used).
/// VID 0x05C6 is generic Qualcomm — used by every Snapdragon-based
/// device when in EDL/9008 mode.
pub const ZEBRA_USB_DB: &[ZebraUsbId] = &[
    // Normal-mode ADB
    ZebraUsbId { vid: 0x05E0, pid: 0x1818, mode_label: "adb",      description: "Zebra TC5x ADB" },
    ZebraUsbId { vid: 0x05E0, pid: 0x1819, mode_label: "adb+mtp",  description: "Zebra TC5x ADB+MTP" },
    ZebraUsbId { vid: 0x05E0, pid: 0x181A, mode_label: "mtp",      description: "Zebra TC5x MTP" },
    ZebraUsbId { vid: 0x05E0, pid: 0x181B, mode_label: "ptp",      description: "Zebra TC5x PTP" },

    // Fastboot / bootloader
    ZebraUsbId { vid: 0x05E0, pid: 0x0900, mode_label: "fastboot", description: "Zebra fastboot (LK bootloader)" },
    ZebraUsbId { vid: 0x05E0, pid: 0x0901, mode_label: "fastbootd",description: "Zebra fastbootd (userspace)" },

    // Recovery
    ZebraUsbId { vid: 0x05E0, pid: 0x0902, mode_label: "recovery", description: "Zebra recovery (sideload)" },

    // Qualcomm EDL — same VID/PID across all Qualcomm OEMs in 9008 mode
    ZebraUsbId { vid: 0x05C6, pid: 0x9008, mode_label: "edl",      description: "Qualcomm EDL (Sahara/Firehose)" },
    ZebraUsbId { vid: 0x05C6, pid: 0x900E, mode_label: "edl-emerg",description: "Qualcomm EDL emergency" },
];

/// Identify a Zebra model from `ro.product.model` plus
/// `ro.zebra.build.id` / `ro.product.device` heuristics.
pub fn identify_model(props: &std::collections::HashMap<String, String>) -> ZebraModel {
    let model_str = props.get("ro.product.model")
        .map(|s| s.to_uppercase())
        .unwrap_or_default();
    let device_str = props.get("ro.product.device")
        .map(|s| s.to_uppercase())
        .unwrap_or_default();
    let zebra_id = props.get("ro.zebra.build.id")
        .map(|s| s.to_uppercase())
        .unwrap_or_default();

    let joined = format!("{} {} {}", model_str, device_str, zebra_id);
    let j = joined.as_str();

    // Order matters — more specific variants first.
    if j.contains("TC53") && j.contains("RFID") {
        ZebraModel::Tc53Rfid
    } else if j.contains("TC53E") || (j.contains("TC53") && j.contains("WWAN")) {
        ZebraModel::Tc53e
    } else if j.contains("TC53") {
        ZebraModel::Tc53
    } else if j.contains("TC52AX") {
        ZebraModel::Tc52ax
    } else if j.contains("TC52X") {
        ZebraModel::Tc52x
    } else if j.contains("TC52") {
        ZebraModel::Tc52
    } else {
        ZebraModel::Unknown
    }
}

/// Map a Zebra USB ID to a friendly label.
pub fn lookup_zebra_usb(vid: u16, pid: u16) -> Option<&'static ZebraUsbId> {
    ZEBRA_USB_DB.iter().find(|e| e.vid == vid && e.pid == pid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn props(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn identifies_tc52() {
        let p = props(&[("ro.product.model", "TC52")]);
        assert_eq!(identify_model(&p), ZebraModel::Tc52);
    }
    #[test]
    fn identifies_tc52x() {
        let p = props(&[("ro.product.model", "TC52X")]);
        assert_eq!(identify_model(&p), ZebraModel::Tc52x);
    }
    #[test]
    fn identifies_tc52ax() {
        let p = props(&[("ro.product.model", "TC52AX")]);
        assert_eq!(identify_model(&p), ZebraModel::Tc52ax);
    }
    #[test]
    fn identifies_tc53() {
        let p = props(&[("ro.product.model", "TC53")]);
        assert_eq!(identify_model(&p), ZebraModel::Tc53);
    }
    #[test]
    fn identifies_tc53e_wwan() {
        let p = props(&[("ro.product.model", "TC53E"), ("ro.zebra.build.id", "WWAN")]);
        assert_eq!(identify_model(&p), ZebraModel::Tc53e);
    }
    #[test]
    fn identifies_tc53_rfid() {
        let p = props(&[("ro.product.model", "TC53-RFID")]);
        assert_eq!(identify_model(&p), ZebraModel::Tc53Rfid);
    }
    #[test]
    fn unknown_when_no_signals() {
        let p = props(&[("ro.product.model", "Pixel 7")]);
        assert_eq!(identify_model(&p), ZebraModel::Unknown);
    }

    #[test]
    fn usb_db_covers_all_modes() {
        // ADB + fastboot + recovery + EDL must all be present
        for mode in &["adb", "fastboot", "recovery", "edl"] {
            assert!(ZEBRA_USB_DB.iter().any(|e| e.mode_label == *mode),
                "missing mode entry: {}", mode);
        }
    }

    #[test]
    fn lookup_zebra_adb_pid() {
        let r = lookup_zebra_usb(0x05E0, 0x1818).unwrap();
        assert_eq!(r.mode_label, "adb");
    }

    #[test]
    fn lookup_edl_pid() {
        let r = lookup_zebra_usb(0x05C6, 0x9008).unwrap();
        assert_eq!(r.mode_label, "edl");
    }

    #[test]
    fn model_displays() {
        assert!(ZebraModel::Tc52ax.display().contains("TC52ax"));
        assert!(ZebraModel::Tc53Rfid.display().contains("RFID"));
    }

    #[test]
    fn soc_strings_populated() {
        assert!(ZebraModel::Tc52.soc().contains("SD660"));
        assert!(ZebraModel::Tc53.soc().contains("SDM6375"));
    }

    #[test]
    fn android_api_progression() {
        // Newer models support newer Android
        assert!(ZebraModel::Tc53.max_android_api() > ZebraModel::Tc52.max_android_api());
    }
}
