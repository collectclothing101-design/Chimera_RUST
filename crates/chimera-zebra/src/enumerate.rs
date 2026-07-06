//! Device-enumeration for Zebra TC52 / TC53 handhelds.
//!
//! `enumerate_device(udid)` pulls every relevant `getprop` value plus
//! battery / thermal / scanner / RxLogger state through ADB and returns
//! a single typed snapshot.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::{Result, ZebraError, models::{ZebraModel, identify_model}};

/// One captured snapshot of a connected Zebra handheld.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZebraDeviceInfo {
    // ── identity ──
    pub model:                ZebraModelDescriptor,
    pub serial_number:        Option<String>,
    pub bootloader_serial:    Option<String>,
    pub build_fingerprint:    Option<String>,
    pub android_version:      Option<String>,
    pub android_api_level:    Option<u32>,
    pub security_patch:       Option<String>,
    pub zebra_os_version:     Option<String>,
    pub zebra_dna_version:    Option<String>,
    pub zebra_mx_version:     Option<String>,
    pub kernel_version:       Option<String>,

    // ── radios / connectivity ──
    pub wifi_mac:             Option<String>,
    pub bluetooth_mac:        Option<String>,
    pub ethernet_mac:         Option<String>,
    pub imei:                 Option<String>,    // TC53e / WWAN only
    pub iccid:                Option<String>,
    pub baseband_version:     Option<String>,

    // ── boot / verified-boot state ──
    pub current_slot:         Option<String>,    // _a / _b
    pub bootloader_locked:    Option<bool>,
    pub verified_boot_state:  Option<String>,    // green / yellow / orange / red
    pub verity_mode:          Option<String>,    // enforcing / disabled / logging
    pub frp_state:            Option<String>,    // present / absent

    // ── battery / thermals ──
    pub battery_level_pct:    Option<u32>,
    pub battery_health:       Option<String>,
    pub battery_serial:       Option<String>,
    pub battery_cycle_count:  Option<u32>,
    pub charging:             Option<bool>,
    pub thermal_zones_c:      HashMap<String, f32>,

    // ── scanner ──
    pub scanner_model:        Option<String>,    // SE4710 / SE4770 / SE55
    pub datawedge_version:    Option<String>,
    pub datawedge_active_profile: Option<String>,

    // ── EMM / device-policy ──
    pub device_owner_pkg:     Option<String>,
    pub profile_owner_pkg:    Option<String>,
    pub is_managed:           Option<bool>,

    // ── raw properties for forward-compat ──
    pub raw_props:            HashMap<String, String>,
}

/// Reduced model descriptor — `ZebraModel` plus the SoC + display string
/// so the GUI doesn't need a second lookup.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZebraModelDescriptor {
    pub kind:        ZebraModelKind,
    pub display:     String,
    pub soc:         String,
    pub max_api:     u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZebraModelKind {
    Tc52, Tc52x, Tc52ax, Tc53, Tc53e, Tc53Rfid,
    #[default] Unknown,
}

impl From<ZebraModel> for ZebraModelKind {
    fn from(m: ZebraModel) -> Self {
        match m {
            ZebraModel::Tc52     => ZebraModelKind::Tc52,
            ZebraModel::Tc52x    => ZebraModelKind::Tc52x,
            ZebraModel::Tc52ax   => ZebraModelKind::Tc52ax,
            ZebraModel::Tc53     => ZebraModelKind::Tc53,
            ZebraModel::Tc53e    => ZebraModelKind::Tc53e,
            ZebraModel::Tc53Rfid => ZebraModelKind::Tc53Rfid,
            ZebraModel::Unknown  => ZebraModelKind::Unknown,
        }
    }
}

/// Full enumerate: shells out via the adb path discovered by chimera-utils'
/// host-probe machinery, runs every relevant getprop and a handful of
/// `dumpsys` / `cat /sys/...` calls, then maps the result into typed
/// fields.
///
/// `target` is an optional ADB serial; pass `None` to use the first
/// connected device.
pub fn enumerate_device(target: Option<&str>) -> Result<ZebraDeviceInfo> {
    let probe = chimera_utils::host_probes::detect_adb();
    if !probe.found {
        return Err(ZebraError::Adb(format!(
            "adb not found on host: {}",
            probe.error.unwrap_or_else(|| "no error captured".into())
        )));
    }
    let adb = probe.path.as_ref().ok_or_else(|| ZebraError::Adb("adb path missing".into()))?;

    // ── 1. getprop dump ──
    let raw_props = adb_getprop_all(adb, target)?;

    // ── 2. model identification from props ──
    let model_id = identify_model(&raw_props);
    let mut info = ZebraDeviceInfo::default();
    info.model = ZebraModelDescriptor {
        kind:    model_id.into(),
        display: model_id.display().to_string(),
        soc:     model_id.soc().to_string(),
        max_api: model_id.max_android_api(),
    };

    // ── 3. fan-out into typed fields ──
    let get = |k: &str| raw_props.get(k).filter(|s| !s.is_empty()).cloned();
    info.serial_number       = get("ro.serialno");
    info.bootloader_serial   = get("ro.boot.serialno");
    info.build_fingerprint   = get("ro.build.fingerprint");
    info.android_version     = get("ro.build.version.release");
    info.android_api_level   = get("ro.build.version.sdk").and_then(|s| s.parse().ok());
    info.security_patch      = get("ro.build.version.security_patch");
    info.zebra_os_version    = get("ro.zebra.build.os.platform").or_else(|| get("ro.zebra.build.id"));
    info.zebra_dna_version   = get("ro.zebra.dna.version");
    info.zebra_mx_version    = get("ro.zebra.mx.version");
    info.kernel_version      = get("ro.kernel.version");
    info.imei                = get("ril.gsm.imei").or_else(|| get("ro.boot.imei"));
    info.iccid               = get("gsm.sim.iccid");
    info.baseband_version    = get("gsm.version.baseband");
    info.current_slot        = get("ro.boot.slot_suffix");
    info.bootloader_locked   = get("ro.boot.flash.locked")
                                  .map(|s| s == "1" || s.eq_ignore_ascii_case("locked"));
    info.verified_boot_state = get("ro.boot.verifiedbootstate");
    info.verity_mode         = get("ro.boot.veritymode");

    // ── 4. battery / thermals via shell ──
    if let Ok(bat) = adb_shell(adb, target, "dumpsys battery") {
        info.battery_level_pct = parse_kv_u32(&bat, "level");
        info.battery_health    = parse_kv_str(&bat, "health");
        info.charging          = parse_kv_str(&bat, "AC powered").map(|v| v == "true")
                                    .or_else(|| parse_kv_str(&bat, "USB powered").map(|v| v == "true"));
    }
    if let Ok(cycle) = adb_shell(adb, target,
        "cat /sys/class/power_supply/battery/cycle_count 2>/dev/null") {
        info.battery_cycle_count = cycle.trim().parse().ok();
    }
    if let Ok(bsn) = adb_shell(adb, target,
        "cat /sys/class/power_supply/battery/battery_serial_number 2>/dev/null") {
        let bsn = bsn.trim();
        if !bsn.is_empty() { info.battery_serial = Some(bsn.to_string()); }
    }
    if let Ok(zones) = adb_shell(adb, target,
        "for z in /sys/class/thermal/thermal_zone*; do \
            echo \"$(cat $z/type)=$(cat $z/temp)\"; \
        done 2>/dev/null") {
        for line in zones.lines() {
            if let Some((name, temp)) = line.split_once('=') {
                if let Ok(milli_c) = temp.trim().parse::<i32>() {
                    info.thermal_zones_c.insert(name.trim().to_string(), milli_c as f32 / 1000.0);
                }
            }
        }
    }

    // ── 5. EMM / device-policy ──
    if let Ok(dpm) = adb_shell(adb, target, "dumpsys device_policy") {
        info.device_owner_pkg = parse_dpm_owner(&dpm, "Device Owner");
        info.profile_owner_pkg= parse_dpm_owner(&dpm, "Profile Owner");
        info.is_managed       = Some(info.device_owner_pkg.is_some() || info.profile_owner_pkg.is_some());
    }

    // ── 6. scanner / DataWedge ──
    if let Ok(dw) = adb_shell(adb, target,
        "dumpsys package com.symbol.datawedge | grep -E 'versionName='") {
        info.datawedge_version = dw.split('=').nth(1).map(|s| s.trim().to_string());
    }

    info.raw_props = raw_props;
    Ok(info)
}

// ─── adb plumbing ───────────────────────────────────────────────────

fn adb_getprop_all(adb: &std::path::Path, target: Option<&str>)
    -> Result<HashMap<String, String>>
{
    let stdout = adb_shell(adb, target, "getprop")?;
    Ok(parse_getprop(&stdout))
}

fn adb_shell(adb: &std::path::Path, target: Option<&str>, cmd: &str) -> Result<String> {
    use std::process::Command;
    let mut c = Command::new(adb);
    if let Some(s) = target { c.args(["-s", s]); }
    c.arg("shell").arg(cmd);
    let out = c.output().map_err(|e| ZebraError::Adb(format!("spawn adb: {}", e)))?;
    if !out.status.success() {
        return Err(ZebraError::Adb(format!(
            "exit {}: {}",
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Parse Android `getprop` output. Each line is `[key]: [value]`.
pub fn parse_getprop(stdout: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(eq) = line.find("]: [") {
            let k = &line[1..eq];
            let v = line[eq + 4..].trim_end_matches(']');
            out.insert(k.to_string(), v.to_string());
        }
    }
    out
}

fn parse_kv_u32(text: &str, key: &str) -> Option<u32> {
    parse_kv_str(text, key).and_then(|s| s.parse().ok())
}
fn parse_kv_str(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim_start_matches(':').trim();
            return Some(rest.to_string());
        }
    }
    None
}

/// Parse `dumpsys device_policy` to extract the named owner package.
fn parse_dpm_owner(dpm: &str, label: &str) -> Option<String> {
    let mut in_section = false;
    for line in dpm.lines() {
        let line = line.trim();
        if line.starts_with(label) { in_section = true; continue; }
        if in_section {
            if let Some(idx) = line.find("name=") {
                let rest = &line[idx + 5..];
                let end = rest.find(',').or_else(|| rest.find(' ')).unwrap_or(rest.len());
                return Some(rest[..end].trim_matches('"').to_string());
            }
            if let Some(idx) = line.find("ComponentInfo") {
                let rest = &line[idx + 13..];
                if let Some(start) = rest.find('{') {
                    if let Some(end) = rest[start+1..].find('/') {
                        return Some(rest[start+1..start+1+end].to_string());
                    }
                }
            }
            // Stop when we hit a blank line
            if line.is_empty() { break; }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typical_getprop() {
        let s = "[ro.product.model]: [TC53]\n[ro.serialno]: [ABC123]\n[ro.boot.flash.locked]: [1]\n";
        let p = parse_getprop(s);
        assert_eq!(p.get("ro.product.model").unwrap(), "TC53");
        assert_eq!(p.get("ro.serialno").unwrap(), "ABC123");
        assert_eq!(p.get("ro.boot.flash.locked").unwrap(), "1");
    }

    #[test]
    fn empty_getprop_yields_empty_map() {
        assert_eq!(parse_getprop("").len(), 0);
    }

    #[test]
    fn parses_dpm_owner_componentinfo() {
        let dpm = "Device Owner:\n    ComponentInfo{com.example.dpc/.AdminReceiver}\n\n";
        assert_eq!(parse_dpm_owner(dpm, "Device Owner"), Some("com.example.dpc".into()));
    }

    #[test]
    fn parses_battery_level() {
        let s = "Current Battery Service state:\n  AC powered: false\n  USB powered: true\n  level: 87\n  health: 2\n";
        assert_eq!(parse_kv_u32(s, "level"), Some(87));
        assert_eq!(parse_kv_str(s, "AC powered"), Some("false".into()));
    }

    #[test]
    fn descriptor_defaults_to_unknown() {
        let d = ZebraModelDescriptor::default();
        assert_eq!(d.kind, ZebraModelKind::Unknown);
    }
}
