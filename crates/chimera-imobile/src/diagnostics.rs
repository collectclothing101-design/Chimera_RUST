//! Wrap `idevicediagnostics` — battery info, sleep / restart / shutdown.

use std::time::Duration;
use crate::tool::{run, ImobileTool, ImobileError};

/// Trigger the diagnostics-relay action for the named verb.
fn diag(verb: &str, udid: Option<&str>) -> Result<(), ImobileError> {
    let mut args = vec![verb];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    run(ImobileTool::Idevicediagnostics, &args, Duration::from_secs(10)).map(|_| ())
}

pub fn restart(udid: Option<&str>) -> Result<(), ImobileError>  { diag("restart", udid) }
pub fn shutdown(udid: Option<&str>) -> Result<(), ImobileError> { diag("shutdown", udid) }
pub fn sleep(udid: Option<&str>) -> Result<(), ImobileError>    { diag("sleep", udid) }

/// Read IORegistry battery information.
pub fn battery_info(udid: Option<&str>) -> Result<String, ImobileError> {
    let mut args = vec!["ioreg", "-p", "IOPower"];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    let output = run(ImobileTool::Idevicediagnostics, &args, Duration::from_secs(10))?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
