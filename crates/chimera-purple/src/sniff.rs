//! PurpleSNIFF report builder.
//!
//! Pulls every lockdownd / mobilegestalt / diagnostics_relay value into a
//! single typed report. The shape matches the original PurpleSNIFF UI:
//!
//!   SNIFF · Battery · SysCfg · Wireless · Diagnostic · Debug · Developer ·
//!   DeviceMode · General

use serde::{Serialize, Deserialize};
use chimera_imobile::info::DeviceProperties;
use crate::{Result, syscfg::SysCfg, battery::BatteryStats, station::StationTimeline,
            mode::{DeviceMode, detect_mode}};

/// Top-level PurpleSNIFF report. JSON-serialisable for FFI transit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurpleSniffReport {
    pub sniff:        SniffSection,
    pub general:      DeviceProperties,
    pub battery:      Option<BatteryStats>,
    pub syscfg:       Option<SysCfg>,
    pub wireless:     WirelessSection,
    pub diagnostic:   StationTimeline,
    pub debug:        DebugSection,
    pub developer:    DeveloperSection,
    pub device_mode:  DeviceMode,
}

/// "SNIFF" metadata header — report identity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SniffSection {
    /// ISO-8601 UTC timestamp the report was generated.
    pub timestamp:     String,
    /// Host machine that issued the report.
    pub host_name:     String,
    /// Reader-tool identity ("ChimeraRS PurpleSNIFF clean-room v1.0").
    pub reader:        String,
    /// Schema version — bumped when fields are added.
    pub schema_version: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WirelessSection {
    pub wifi_address:       Option<String>,
    pub bluetooth_address:  Option<String>,
    pub ethernet_address:   Option<String>,
    pub bonjour_name:       Option<String>,
    pub wifi_sync_supported:Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugSection {
    /// CLTM (Closed-Loop Thermal Management) extended log buffer.
    pub thermal_log:      Vec<String>,
    /// `lockdownd_extended_log` entries, if exposed.
    pub lockdown_log:     Vec<String>,
    /// Crash-report names known to crash-report-mover.
    pub crash_reports:    Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeveloperSection {
    /// True when the Xcode "developer disk image" is mounted.
    pub developer_disk_mounted: bool,
    /// True when the device is in `AppleInternal` mode (engineering build).
    pub apple_internal:         bool,
}

/// Build a full SNIFF report for one device. `udid = None` ⇒ first device.
///
/// Performs the same I/O pattern as Apple's PurpleSNIFF binary: walks the
/// public lockdownd service ports and collects every keyspace the daemons
/// expose. The crate never touches Apple-internal binaries.
pub fn sniff(udid: Option<&str>) -> Result<PurpleSniffReport> {
    let general = chimera_imobile::ideviceinfo(udid)?;

    let device_mode = detect_mode(udid).unwrap_or(DeviceMode::Unknown);

    let wireless = WirelessSection {
        wifi_address:        general.wifi_address.clone(),
        bluetooth_address:   general.bluetooth_address.clone(),
        ethernet_address:    chimera_imobile::info::key(udid, "EthernetAddress").ok(),
        bonjour_name:        chimera_imobile::info::key(udid, "DeviceName").ok(),
        wifi_sync_supported: chimera_imobile::info::key(udid, "WiFiSyncEnabled")
                                .ok()
                                .map(|s| s == "true" || s == "1"),
    };

    let syscfg   = crate::syscfg::read(udid).ok();
    let battery  = crate::battery::read(udid).ok();

    let diagnostic = crate::station::read_timeline(udid).unwrap_or_default();

    let debug = DebugSection {
        thermal_log:   chimera_imobile::info::key(udid, "CLTMThermalLog")
                          .ok().map(|s| s.lines().map(String::from).collect())
                          .unwrap_or_default(),
        lockdown_log:  vec![],
        crash_reports: vec![],
    };

    let developer = DeveloperSection {
        developer_disk_mounted: chimera_imobile::info::key(udid,
                                  "DeveloperDiskImageMounted").ok().as_deref() == Some("true"),
        apple_internal:         chimera_imobile::info::key(udid,
                                  "AppleInternal").ok().as_deref() == Some("true"),
    };

    Ok(PurpleSniffReport {
        sniff: SniffSection {
            timestamp:      chrono::Utc::now().to_rfc3339(),
            host_name:      hostname_or_unknown(),
            reader:         "ChimeraRS PurpleSNIFF (clean-room) v1.0".into(),
            schema_version: 1,
        },
        general,
        battery,
        syscfg,
        wireless,
        diagnostic,
        debug,
        developer,
        device_mode,
    })
}

fn hostname_or_unknown() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .or_else(|_| {
            use std::process::Command;
            Command::new("hostname")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .ok_or(())
                .map_err(|_| std::env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| "unknown".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniff_section_serialisable() {
        let s = SniffSection {
            timestamp:      "2026-01-01T00:00:00Z".into(),
            host_name:      "test".into(),
            reader:         "x".into(),
            schema_version: 1,
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("schema_version"));
        let back: SniffSection = serde_json::from_str(&j).unwrap();
        assert_eq!(back.schema_version, 1);
    }

    #[test]
    fn wireless_default_is_all_none() {
        let w = WirelessSection::default();
        assert!(w.wifi_address.is_none());
        assert!(w.bluetooth_address.is_none());
    }

    #[test]
    fn hostname_returns_something() {
        let h = hostname_or_unknown();
        assert!(!h.is_empty());
    }
}
