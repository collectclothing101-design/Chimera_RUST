//! **DataWedge** — Zebra's bundled scanner middleware. Configures the
//! integrated SE4710 / SE4770 / SE55 imager: barcode symbology enable
//! list, keystroke vs. intent vs. content-provider output, decode beep,
//! aim/illumination mode, and per-app scanning profiles.
//!
//! All interaction happens via Android intents — no root, no special
//! permissions beyond DataWedge being installed (which it always is on
//! a Zebra device).

use std::process::Command;
use serde::{Serialize, Deserialize};
use crate::{Result, ZebraError};

const DW_PKG:    &str = "com.symbol.datawedge";
const DW_ACTION: &str = "com.symbol.datawedge.api.ACTION";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataWedgeStatus {
    pub installed:        bool,
    pub version:          Option<String>,
    pub active_profile:   Option<String>,
    pub all_profiles:     Vec<String>,
    pub scanner_enabled:  bool,
}

/// Query DataWedge installation + version.
pub fn status(target: Option<&str>) -> Result<DataWedgeStatus> {
    let out = adb_shell(target, &format!(
        "dumpsys package {} | grep -E 'versionName=|versionCode='", DW_PKG
    ))?;
    let version = out.lines()
        .find_map(|l| l.split("versionName=").nth(1))
        .map(|s| s.split_whitespace().next().unwrap_or("").to_string());
    let installed = version.is_some();

    let profiles = list_profiles(target).unwrap_or_default();
    let active   = active_profile(target).ok();

    Ok(DataWedgeStatus {
        installed,
        version,
        active_profile: active,
        all_profiles: profiles,
        scanner_enabled: true,
    })
}

/// Switch the active DataWedge profile (e.g. to "Profile0" or a custom one).
pub fn switch_profile(target: Option<&str>, profile_name: &str) -> Result<String> {
    adb_shell(target, &format!(
        "am broadcast -a {} -p {} \
         --es com.symbol.datawedge.api.SWITCH_TO_PROFILE \"{}\"",
        DW_ACTION, DW_PKG, profile_name
    ))
}

/// Enable / disable the scanner globally.
pub fn enable_scanner(target: Option<&str>, enabled: bool) -> Result<String> {
    let verb = if enabled { "ENABLE_PLUGIN" } else { "DISABLE_PLUGIN" };
    adb_shell(target, &format!(
        "am broadcast -a {} -p {} \
         --es com.symbol.datawedge.api.{} BARCODE",
        DW_ACTION, DW_PKG, verb
    ))
}

/// Issue a soft scan trigger (presses the scan button by intent).
pub fn soft_trigger(target: Option<&str>, start: bool) -> Result<String> {
    let verb = if start { "START_SCANNING" } else { "STOP_SCANNING" };
    adb_shell(target, &format!(
        "am broadcast -a {} -p {} \
         --es com.symbol.datawedge.api.SOFT_SCAN_TRIGGER {}",
        DW_ACTION, DW_PKG, verb
    ))
}

/// List every profile name in the DataWedge profile store.
pub fn list_profiles(target: Option<&str>) -> Result<Vec<String>> {
    let out = adb_shell(target, &format!(
        "content query --uri content://{}.profile.provider/profiles \
         --projection PROFILE_NAME", DW_PKG
    )).unwrap_or_default();
    let mut profiles = Vec::new();
    for line in out.lines() {
        if let Some(idx) = line.find("PROFILE_NAME=") {
            let v = &line[idx + 13..];
            let end = v.find(',').unwrap_or(v.len());
            let name = v[..end].trim();
            if !name.is_empty() { profiles.push(name.to_string()); }
        }
    }
    Ok(profiles)
}

/// Return the currently active profile name.
pub fn active_profile(target: Option<&str>) -> Result<String> {
    let out = adb_shell(target, &format!(
        "settings get system {}.active_profile 2>/dev/null", DW_PKG
    )).unwrap_or_default();
    let trimmed = out.trim().to_string();
    if trimmed.is_empty() || trimmed == "null" {
        Err(ZebraError::Other("no active profile (DataWedge defaults in use)".into()))
    } else {
        Ok(trimmed)
    }
}

fn adb_shell(target: Option<&str>, cmd: &str) -> Result<String> {
    let probe = chimera_utils::host_probes::detect_adb();
    if !probe.found { return Err(ZebraError::Adb("adb missing".into())); }
    let adb = probe.path.as_ref().unwrap();
    let mut c = Command::new(adb);
    if let Some(s) = target { c.args(["-s", s]); }
    c.arg("shell").arg(cmd);
    let out = c.output().map_err(|e| ZebraError::Adb(format!("spawn: {}", e)))?;
    if !out.status.success() {
        return Err(ZebraError::Adb(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkg_constants() {
        assert!(DW_PKG.contains("datawedge"));
        assert!(DW_ACTION.contains("datawedge.api.ACTION"));
    }

    #[test]
    fn status_default_safe() {
        let s = DataWedgeStatus::default();
        assert!(!s.installed);
        assert!(s.active_profile.is_none());
    }
}
