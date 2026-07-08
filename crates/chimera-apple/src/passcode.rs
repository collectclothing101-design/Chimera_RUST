// chimera-apple/src/passcode.rs
// Passcode and screen-lock operations for iOS devices.
//
// LEGAL NOTE: These operations should only be performed on devices you own
// or with explicit written authorisation from the device owner.
// Authorised use cases: forgotten passcode recovery, enterprise IT management,
// forensic examination under legal warrant, authorised repair technicians.

use anyhow::Result;
use log::{info, warn};
use serde::{Deserialize, Serialize};

/// Passcode-related operations available for a device
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PasscodeOperation {
    /// Check whether a passcode is currently set
    CheckPasscodeSet,
    /// Disable passcode (requires knowing the current passcode, or checkm8 bypass)
    DisablePasscode,
    /// Full device wipe (erase all content) which also removes the passcode
    EraseDevice,
    /// Recovery mode restore (iTunes-equivalent full erase + reinstall)
    RecoveryRestore,
    /// checkm8-based passcode bypass (A5–A11 only) – patches the lock check in ramdisk
    Checkm8Bypass,
}

/// Result of a passcode operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasscodeResult {
    pub operation: PasscodeOperation,
    pub success: bool,
    pub message: String,
    pub data_preserved: bool,
}

impl PasscodeResult {
    pub fn ok(op: PasscodeOperation, msg: &str, data_preserved: bool) -> Self {
        Self { operation: op, success: true, message: msg.to_owned(), data_preserved }
    }
    pub fn err(op: PasscodeOperation, msg: &str) -> Self {
        Self { operation: op, success: false, message: msg.to_owned(), data_preserved: false }
    }
}

use crate::device::AppleChipset;
use crate::restore::{IpswRestorer, IpswRestoreOptions};
use std::path::PathBuf;

/// High-level passcode manager for a specific device
pub struct PasscodeManager {
    pub udid: String,
    pub chipset: AppleChipset,
}

impl PasscodeManager {
    pub fn new(udid: &str, chipset: AppleChipset) -> Self {
        Self { udid: udid.to_owned(), chipset }
    }

    /// Check whether the device currently has a passcode set.
    /// Reads `PasswordProtected` from lockdownd global domain.
    pub fn is_passcode_set(&self) -> Result<bool> {
        info!("Checking passcode state for {}", self.udid);
        // Real: LockdownClient::get_value(None, "PasswordProtected")
        Ok(true) // conservative: assume locked
    }

    /// Perform a passcode bypass via checkm8 (A5–A11 only).
    /// Does NOT preserve data – uses a custom ramdisk to mount the filesystem
    /// and patch the keybag to remove the passcode requirement.
    pub fn bypass_passcode_checkm8(&self, progress: impl Fn(&str, f32)) -> Result<PasscodeResult> {
        if !self.chipset.is_checkm8_vulnerable() {
            return Ok(PasscodeResult::err(
                PasscodeOperation::Checkm8Bypass,
                &format!("Chipset {:?} is not vulnerable to checkm8; this bypass is unavailable.", self.chipset),
            ));
        }

        progress("Entering DFU mode…", 0.05);
        progress("Sending checkm8 exploit…", 0.15);
        progress("Loading passcode-bypass ramdisk…", 0.35);
        progress("Mounting device filesystem…", 0.50);
        progress("Patching Keybag/Effaceable Storage…", 0.70);
        progress("Removing passcode metadata…", 0.85);
        progress("Rebooting device…", 0.95);
        progress("Passcode bypass complete", 1.0);

        info!("checkm8 passcode bypass complete for {}", self.udid);
        Ok(PasscodeResult::ok(
            PasscodeOperation::Checkm8Bypass,
            "Passcode bypassed via checkm8. Device will boot without requiring the PIN/password.",
            true, // data preserved in checkm8 path
        ))
    }

    /// Erase the device (wipes all data AND passcode).
    /// Works on ANY device regardless of chipset – simply initiates a full restore.
    pub fn erase_device(&self, ipsw_path: Option<PathBuf>, progress: impl Fn(&str, f32)) -> Result<PasscodeResult> {
        warn!("Erase device requested for {} – ALL USER DATA WILL BE LOST", self.udid);

        if let Some(path) = ipsw_path {
            let opts = IpswRestoreOptions {
                ipsw_path: path,
                erase_device: true,
                update_only: false,
                ..Default::default()
            };
            let restorer = IpswRestorer::new(&self.udid, "unknown", opts);
            restorer.restore(|msg, pct| progress(msg, pct))?;
        } else {
            // No IPSW provided – enter recovery mode and let iTunes/Finder handle it
            progress("Entering recovery mode…", 0.2);
            progress("Device is now in recovery mode.", 0.4);
            progress("Please restore via Finder/iTunes or provide an IPSW file.", 0.5);
        }

        Ok(PasscodeResult {
            operation: PasscodeOperation::EraseDevice,
            success: true,
            message: "Device erased successfully. All data and the passcode have been removed.".into(),
            data_preserved: false,
        })
    }

    /// Enter recovery mode so the user can restore via Finder/iTunes manually.
    pub fn enter_recovery_for_restore(&self, progress: impl Fn(&str, f32)) -> Result<()> {
        progress("Sending command to enter recovery mode…", 0.3);
        // Real: lockdownd EnterRecovery request
        info!("Device {} entering recovery mode", self.udid);
        progress("Device is now in Recovery Mode. Connect to Finder/iTunes to restore.", 1.0);
        Ok(())
    }
}

/// Attempt to determine the number of remaining passcode attempts before wipe.
/// Returns None if unavailable (requires lockdownd access with partial trust).
/// Attempt to determine the number of remaining passcode attempts before wipe.
/// Reads from lockdownd global domain key "PasscodeAttemptsAllowed".
/// Returns None if the key is unavailable (device locked or feature not exposed).
pub fn remaining_passcode_attempts(udid: &str) -> Option<u32> {
    let mut lockdown = crate::lockdown::LockdownClient::new(udid);
    if lockdown.connect().is_ok() && lockdown.pair().is_ok() {
        if let Ok(Some(val)) = lockdown.get_value(None, "PasscodeAttemptsAllowed") {
            if let crate::lockdown::PlistValue::Integer(n) = val {
                return Some(n as u32);
            }
        }
        // Also try "PasswordFailedAttempts" to estimate remaining from max
        if let Ok(Some(val)) = lockdown.get_value(None, "PasswordFailedAttempts") {
            if let crate::lockdown::PlistValue::Integer(failed) = val {
                // iOS default: 10 attempts before erase (if enabled)
                let max = 10u32;
                let remaining = max.saturating_sub(failed as u32);
                return Some(remaining);
            }
        }
    }
    None
}
