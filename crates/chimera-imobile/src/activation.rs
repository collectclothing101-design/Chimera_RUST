//! Wrap `ideviceactivation` — fetch + activate iOS device records.
//!
//! Usage:
//!     ideviceactivation activate              # full online flow via Apple
//!     ideviceactivation deactivate            # remove activation record
//!     ideviceactivation state                 # query current state

use std::time::Duration;
use serde::{Serialize, Deserialize};
use crate::tool::{run, ImobileTool, ImobileError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivationState {
    Activated,
    Unactivated,
    /// Device contacts Apple's activation server but is denied (e.g. iCloud-locked).
    DeniedByApple,
    Unknown,
}

/// Query the device's current activation state via `ideviceactivation state`.
pub fn fetch_activation(udid: Option<&str>) -> Result<ActivationState, ImobileError> {
    let mut args = vec!["state"];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    let output = run(ImobileTool::Ideviceactivation, &args, Duration::from_secs(15))?;
    let s = String::from_utf8_lossy(&output.stdout).to_lowercase();
    Ok(if s.contains("activated")          { ActivationState::Activated }
       else if s.contains("unactivated")   { ActivationState::Unactivated }
       else if s.contains("denied")        { ActivationState::DeniedByApple }
       else                                { ActivationState::Unknown })
}

/// Run the full online activation flow.
pub fn activate(udid: Option<&str>) -> Result<(), ImobileError> {
    let mut args = vec!["activate"];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    run(ImobileTool::Ideviceactivation, &args, Duration::from_secs(60)).map(|_| ())
}

/// Remove the device's activation record.
pub fn deactivate(udid: Option<&str>) -> Result<(), ImobileError> {
    let mut args = vec!["deactivate"];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    run(ImobileTool::Ideviceactivation, &args, Duration::from_secs(15)).map(|_| ())
}
