//! Wrap `ideviceenterrecovery` (normal → recovery) and `irecovery`
//! (interact with the device while in recovery / DFU).

use std::time::Duration;
use serde::{Serialize, Deserialize};
use crate::tool::{run, ImobileTool, ImobileError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// `irecovery -r` — send Restart command, exits recovery.
    Reboot,
    /// `irecovery -n` — set auto-boot, continue normal boot.
    SetAutoBootOn,
    /// `irecovery -p disable` — disable auto-boot (tethered).
    SetAutoBootOff,
    /// `irecovery -s` — drop into iBoot shell (interactive).
    Shell,
}

/// Send a device from normal/lockdownd mode into iBoot Recovery mode.
pub fn enter_recovery(udid: &str) -> Result<(), ImobileError> {
    run(
        ImobileTool::Ideviceenterrecovery,
        &[udid],
        Duration::from_secs(15),
    ).map(|_| ())
}

/// Exit recovery → normal boot. Equivalent to `irecovery -n` then `-r`.
pub fn exit_recovery() -> Result<(), ImobileError> {
    // Set auto-boot=true (env-var on iBoot), then send Reboot.
    let _ = run(ImobileTool::Irecovery, &["-n"], Duration::from_secs(10))?;
    run(ImobileTool::Irecovery, &["-r"], Duration::from_secs(10)).map(|_| ())
}

/// Run an `irecovery` action against the device currently in recovery / DFU.
pub fn irecovery_action(action: RecoveryAction) -> Result<(), ImobileError> {
    let args: &[&str] = match action {
        RecoveryAction::Reboot         => &["-r"],
        RecoveryAction::SetAutoBootOn  => &["-c", "setenv auto-boot true",  "-c", "saveenv"],
        RecoveryAction::SetAutoBootOff => &["-c", "setenv auto-boot false", "-c", "saveenv"],
        RecoveryAction::Shell          => &["-s"],
    };
    run(ImobileTool::Irecovery, args, Duration::from_secs(30)).map(|_| ())
}

/// Send a raw iBoot command via `irecovery -c "<cmd>"`.
pub fn send_iboot_command(cmd: &str) -> Result<String, ImobileError> {
    let output = run(ImobileTool::Irecovery, &["-c", cmd], Duration::from_secs(15))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
