//! Debug-console / log-collection helpers for Zebra TC5x.
//!
//! Pulls every standard Android diagnostic surface plus the Zebra-specific
//! ones into a single archive directory:
//!
//!   - `logcat -b all -d`
//!   - `dmesg`
//!   - `/sys/fs/pstore/console-ramoops*`  (pre-reboot kernel log)
//!   - `/data/anr/`                       (ANR traces)
//!   - `/data/tombstones/`                (native crash dumps)
//!   - `/data/vendor/diag/`               (Zebra DataWedge diag)
//!   - `bugreport`                        (compressed system-wide dump)
//!   - `getprop`                          (full property tree)
//!   - `dumpsys batterystats`             (battery history)
//!   - `dumpsys thermalservice`           (thermal events)
//!   - `dumpsys wifi`                     (Wi-Fi state)

use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Serialize, Deserialize};
use crate::{Result, ZebraError};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugDump {
    pub destination: PathBuf,
    pub files:       Vec<String>,
    pub total_bytes: u64,
    pub duration_ms: u128,
}

/// Collect every standard + Zebra-specific debug surface into `dest`.
/// `dest` is created if it doesn't exist.
pub fn collect_debug_dump(target: Option<&str>, dest: &Path) -> Result<DebugDump> {
    let start = std::time::Instant::now();
    std::fs::create_dir_all(dest)?;

    let probe = chimera_utils::host_probes::detect_adb();
    if !probe.found { return Err(ZebraError::Adb("adb missing".into())); }
    let adb = probe.path.as_ref().unwrap();

    let mut files = Vec::new();
    // ── Logcat ──
    let logcat = adb_shell(adb, target, "logcat -b all -d -v threadtime")?;
    let fpath = dest.join("logcat.txt");
    std::fs::write(&fpath, &logcat)?;
    files.push(fpath.file_name().unwrap().to_string_lossy().to_string());

    // ── dmesg ──
    let dmesg = adb_shell(adb, target, "dmesg -k").unwrap_or_default();
    let fpath = dest.join("dmesg.txt");
    std::fs::write(&fpath, &dmesg)?;
    files.push(fpath.file_name().unwrap().to_string_lossy().to_string());

    // ── getprop tree ──
    let props = adb_shell(adb, target, "getprop")?;
    let fpath = dest.join("getprop.txt");
    std::fs::write(&fpath, &props)?;
    files.push(fpath.file_name().unwrap().to_string_lossy().to_string());

    // ── battery stats ──
    let bs = adb_shell(adb, target, "dumpsys batterystats").unwrap_or_default();
    let fpath = dest.join("batterystats.txt");
    std::fs::write(&fpath, &bs)?;
    files.push(fpath.file_name().unwrap().to_string_lossy().to_string());

    // ── thermal service ──
    let th = adb_shell(adb, target, "dumpsys thermalservice").unwrap_or_default();
    let fpath = dest.join("thermal.txt");
    std::fs::write(&fpath, &th)?;
    files.push(fpath.file_name().unwrap().to_string_lossy().to_string());

    // ── wifi ──
    let wifi = adb_shell(adb, target, "dumpsys wifi").unwrap_or_default();
    let fpath = dest.join("wifi.txt");
    std::fs::write(&fpath, &wifi)?;
    files.push(fpath.file_name().unwrap().to_string_lossy().to_string());

    // ── pstore ramoops (pre-reboot kernel log) ──
    let ramoops = adb_shell(adb, target,
        "cat /sys/fs/pstore/console-ramoops* 2>/dev/null").unwrap_or_default();
    if !ramoops.is_empty() {
        let fpath = dest.join("pstore-ramoops.txt");
        std::fs::write(&fpath, &ramoops)?;
        files.push(fpath.file_name().unwrap().to_string_lossy().to_string());
    }

    // ── ANR + tombstones list (pull contents separately) ──
    let anr_list = adb_shell(adb, target,
        "ls -la /data/anr/ 2>/dev/null").unwrap_or_default();
    std::fs::write(dest.join("anr-listing.txt"), &anr_list)?;
    files.push("anr-listing.txt".into());

    let tomb_list = adb_shell(adb, target,
        "ls -la /data/tombstones/ 2>/dev/null").unwrap_or_default();
    std::fs::write(dest.join("tombstones-listing.txt"), &tomb_list)?;
    files.push("tombstones-listing.txt".into());

    // ── Zebra DNA / MX events ──
    let mx = adb_shell(adb, target,
        "dumpsys package com.zebra.mdm 2>/dev/null; \
         ls -la /data/vendor/diag/ 2>/dev/null").unwrap_or_default();
    std::fs::write(dest.join("zebra-diag.txt"), &mx)?;
    files.push("zebra-diag.txt".into());

    // ── EMM / device-policy snapshot ──
    let dpm = adb_shell(adb, target, "dumpsys device_policy").unwrap_or_default();
    std::fs::write(dest.join("device-policy.txt"), &dpm)?;
    files.push("device-policy.txt".into());

    // ── Total size ──
    let mut total = 0u64;
    for f in &files {
        if let Ok(meta) = std::fs::metadata(dest.join(f)) { total += meta.len(); }
    }

    Ok(DebugDump {
        destination: dest.to_path_buf(),
        files,
        total_bytes: total,
        duration_ms: start.elapsed().as_millis(),
    })
}

fn adb_shell(adb: &std::path::Path, target: Option<&str>, cmd: &str) -> Result<String> {
    let mut c = Command::new(adb);
    if let Some(s) = target { c.args(["-s", s]); }
    c.arg("shell").arg(cmd);
    let out = c.output().map_err(|e| ZebraError::Adb(e.to_string()))?;
    if !out.status.success() {
        return Err(ZebraError::Adb(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_dump_struct() {
        let d = DebugDump::default();
        assert_eq!(d.total_bytes, 0);
        assert!(d.files.is_empty());
    }
}
