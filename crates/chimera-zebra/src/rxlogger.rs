//! **RxLogger** — Zebra's built-in diagnostic capture tool, preinstalled on
//! every TC5x device. Captures pcap, logcat, dumpsys, MX events, scanner
//! traces, and modem AT-channel logs into `/sdcard/RxLogger/`.
//!
//! ## Public broadcast API (no root required)
//!
//! ```text
//! am broadcast -a com.symbol.rxlogger.intent.action.RXLOGGER_START
//! am broadcast -a com.symbol.rxlogger.intent.action.RXLOGGER_STOP
//! am broadcast -a com.symbol.rxlogger.intent.action.RXLOGGER_GENERATE_SNAPSHOT
//! ```
//!
//! ## Output paths
//!
//!   /sdcard/RxLogger/<YYYYMMDD-HHMMSS>/   one dir per capture session
//!     ├── logcat.txt
//!     ├── tcpdump.pcap
//!     ├── dmesg.txt
//!     ├── bugreport.zip
//!     ├── dumpsys.txt
//!     ├── mx_events.csv
//!     └── scanner_diag.json (if scanner trace enabled)

use std::path::{Path, PathBuf};
use std::process::Command;
use serde::{Serialize, Deserialize};
use crate::{Result, ZebraError};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RxLoggerStatus {
    pub running:        bool,
    pub current_session:Option<String>,
    pub captured_bytes: u64,
    pub session_dirs:   Vec<String>,
}

const RXL_PKG: &str = "com.symbol.rxlogger";
const ACT_START: &str = "com.symbol.rxlogger.intent.action.RXLOGGER_START";
const ACT_STOP:  &str = "com.symbol.rxlogger.intent.action.RXLOGGER_STOP";
const ACT_SNAP:  &str = "com.symbol.rxlogger.intent.action.RXLOGGER_GENERATE_SNAPSHOT";

/// Start RxLogger capture. Returns the broadcast result text.
pub fn start_rxlogger(target: Option<&str>) -> Result<String> {
    adb_shell(target, &format!("am broadcast -a {} -p {}", ACT_START, RXL_PKG))
}

/// Stop RxLogger capture.
pub fn stop_rxlogger(target: Option<&str>) -> Result<String> {
    adb_shell(target, &format!("am broadcast -a {} -p {}", ACT_STOP, RXL_PKG))
}

/// Take a one-shot snapshot (start + capture-for-N + stop in one go).
pub fn snapshot(target: Option<&str>) -> Result<String> {
    adb_shell(target, &format!("am broadcast -a {} -p {}", ACT_SNAP, RXL_PKG))
}

/// List session directories under `/sdcard/RxLogger/`.
pub fn list_sessions(target: Option<&str>) -> Result<Vec<String>> {
    let out = adb_shell(target, "ls -1 /sdcard/RxLogger/ 2>/dev/null")?;
    Ok(out.lines().filter(|l| !l.trim().is_empty()).map(|s| s.trim().to_string()).collect())
}

/// Pull the entire `/sdcard/RxLogger/` tree (or a specific session) to a
/// local directory.
pub fn pull_rxlogger_dump(target: Option<&str>, session: Option<&str>,
                          local_dest: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(local_dest)?;
    let remote = match session {
        Some(s) => format!("/sdcard/RxLogger/{}", s),
        None    => "/sdcard/RxLogger/".to_string(),
    };
    let probe = chimera_utils::host_probes::detect_adb();
    let adb   = probe.path.as_ref()
        .ok_or_else(|| ZebraError::Adb("adb missing".into()))?;
    let mut c = Command::new(adb);
    if let Some(s) = target { c.args(["-s", s]); }
    c.args(["pull", &remote, &local_dest.to_string_lossy()]);
    let out = c.output().map_err(|e| ZebraError::Adb(format!("adb pull: {}", e)))?;
    if !out.status.success() {
        return Err(ZebraError::Adb(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(local_dest.to_path_buf())
}

/// Query current RxLogger status without altering it.
pub fn status(target: Option<&str>) -> Result<RxLoggerStatus> {
    let dirs = list_sessions(target).unwrap_or_default();
    // The package's running state can be probed via dumpsys
    let dump = adb_shell(target, "dumpsys activity services com.symbol.rxlogger | head -30")
        .unwrap_or_default();
    let running = dump.contains("RxLoggerService") || dump.contains("Started: true");
    let current_session = dirs.last().cloned();
    // Total bytes captured (best-effort)
    let size = adb_shell(target, "du -sb /sdcard/RxLogger/ 2>/dev/null | awk '{print $1}'")
        .unwrap_or_default();
    let bytes = size.trim().parse::<u64>().unwrap_or(0);
    Ok(RxLoggerStatus {
        running,
        current_session,
        captured_bytes: bytes,
        session_dirs: dirs,
    })
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
    fn status_default_zero() {
        let s = RxLoggerStatus::default();
        assert!(!s.running);
        assert_eq!(s.captured_bytes, 0);
        assert!(s.session_dirs.is_empty());
    }

    #[test]
    fn pkg_constants_present() {
        assert!(RXL_PKG.contains("rxlogger"));
        assert!(ACT_START.contains("RXLOGGER_START"));
        assert!(ACT_STOP.contains("RXLOGGER_STOP"));
        assert!(ACT_SNAP.contains("SNAPSHOT"));
    }
}
