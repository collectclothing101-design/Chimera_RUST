//! Wrap `idevicebackup2` — backup / restore / encrypt iOS data.

use std::path::Path;
use std::time::Duration;
use crate::tool::{run, ImobileTool, ImobileError};

/// Backup a device to `dest_dir`. `udid` = None → first device.
/// Internally invokes `idevicebackup2 backup --full <dest>`.
pub fn backup(udid: Option<&str>, dest_dir: &Path) -> Result<(), ImobileError> {
    let dest = dest_dir.to_string_lossy().into_owned();
    let mut args: Vec<&str> = vec!["backup", "--full"];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    args.push(dest.as_str());
    run(ImobileTool::Idevicebackup2, &args, Duration::from_secs(60 * 60)).map(|_| ())
}

/// Restore a device from a previous backup directory.
pub fn restore(udid: Option<&str>, source_dir: &Path) -> Result<(), ImobileError> {
    let src = source_dir.to_string_lossy().into_owned();
    let mut args: Vec<&str> = vec!["restore", "--system", "--settings", "--reboot"];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    args.push(src.as_str());
    run(ImobileTool::Idevicebackup2, &args, Duration::from_secs(60 * 60)).map(|_| ())
}

/// Enable backup encryption with the given password. Returns Ok on success.
pub fn enable_encryption(udid: Option<&str>, password: &str) -> Result<(), ImobileError> {
    let mut args: Vec<&str> = vec!["encryption", "on", password];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    run(ImobileTool::Idevicebackup2, &args, Duration::from_secs(30)).map(|_| ())
}

/// Disable backup encryption. The device may prompt for the existing password.
pub fn disable_encryption(udid: Option<&str>, password: &str) -> Result<(), ImobileError> {
    let mut args: Vec<&str> = vec!["encryption", "off", password];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    run(ImobileTool::Idevicebackup2, &args, Duration::from_secs(30)).map(|_| ())
}
