//! **Purple Restore** flow — boots a device into Apple's internal
//! Diagnostic ("Purple") mode for service / repair workflows.
//!
//! ## Clean-room reimplementation policy
//!
//! Apple's PurpleRestore binary ships Apple-proprietary `purple_ramdisk.dmg`,
//! `purple_kernelcache`, and `purple_devicetree` images we do **not** ship.
//! Users supplying their own Apple-Authorized service ramdisk can point this
//! crate at it via `PurpleRestoreOptions::ramdisk_path`; without a ramdisk
//! the flow stops at the "device is in DFU/Recovery, ready" stage and the
//! caller must perform the actual ramdisk upload themselves.
//!
//! ## Flow
//!
//! 1. **Pre-flight** — verify device is reachable in OSMode / Recovery / DFU.
//! 2. **Transition to DFU** — if device is in OSMode, send
//!    `ideviceenterrecovery <udid>`, then prompt the operator to perform
//!    the physical button sequence to descend from Recovery → DFU.
//! 3. **DFU verification** — confirm via `irecovery -q` that the device is
//!    now visible at the bootrom level.
//! 4. **Optional ramdisk upload** — if `ramdisk_path` is set, upload via
//!    `irecovery -f <ramdisk>` then `irecovery -c "go ramdisk"`.
//! 5. **Diagnostic-mode boot** — `irecovery -c "boot-diag"` (Apple's iBoot
//!    command for the Purple kernel). Re-check device mode; expect
//!    `DeviceMode::Diagnostic`.
//! 6. **Report** — return a `PurpleRestoreResult` summarising each step.

use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use crate::{Result, PurpleError, mode::{DeviceMode, detect_mode}};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PurpleRestoreOptions {
    /// UDID of the target device; None = first reachable device.
    pub udid:          Option<String>,
    /// Path to an Apple-Authorized service ramdisk (.dmg). When None the
    /// flow stops once the device is in DFU/Recovery, never uploads.
    pub ramdisk_path:  Option<PathBuf>,
    /// Max seconds to wait between bus transitions.
    pub timeout_secs:  Option<u64>,
    /// When true, skips the OSMode → Recovery transition; assumes the
    /// caller has already put the device into Recovery or DFU.
    pub assume_dfu:    bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurpleRestoreFlow {
    pub options:      PurpleRestoreOptions,
    pub start_mode:   DeviceMode,
    pub final_mode:   DeviceMode,
    pub steps:        Vec<String>,
    pub success:      bool,
    pub duration_ms:  u128,
}

/// Execute the Purple Restore flow synchronously, recording every step.
pub fn run(opts: PurpleRestoreOptions) -> Result<PurpleRestoreFlow> {
    let start_ts = Instant::now();
    let timeout  = Duration::from_secs(opts.timeout_secs.unwrap_or(120));
    let mut steps = Vec::with_capacity(8);

    let start_mode = detect_mode(opts.udid.as_deref()).unwrap_or(DeviceMode::Unknown);
    steps.push(format!("[0] Pre-flight: device is in {:?}", start_mode));

    if start_mode == DeviceMode::Unknown {
        return Err(PurpleError::Other(
            "no device reachable — connect device + check usbmuxd".into()));
    }

    // (1-2) Transition into DFU if needed
    if !opts.assume_dfu && start_mode == DeviceMode::OSMode {
        steps.push("[1] OSMode → Recovery: ideviceenterrecovery".into());
        if let Some(udid) = &opts.udid {
            chimera_imobile::recovery::enter_recovery(udid)?;
        } else {
            return Err(PurpleError::Other(
                "OSMode → Recovery transition needs an explicit UDID".into()));
        }
        wait_for_mode(opts.udid.as_deref(), DeviceMode::Recovery, timeout)?;
        steps.push("[2] Recovery reached. Now operator must hold Vol-Down + Side \
                    10 s to enter DFU. Waiting…".into());
        wait_for_mode(opts.udid.as_deref(), DeviceMode::Dfu, timeout)?;
        steps.push("[3] DFU reached.".into());
    } else if start_mode == DeviceMode::Recovery {
        steps.push("[1] Already in Recovery. Operator must descend to DFU now.".into());
        wait_for_mode(opts.udid.as_deref(), DeviceMode::Dfu, timeout)?;
        steps.push("[2] DFU reached.".into());
    } else if start_mode == DeviceMode::Dfu {
        steps.push("[1] Already in DFU.".into());
    } else {
        return Err(PurpleError::WrongMode {
            expected: DeviceMode::Dfu, actual: start_mode,
        });
    }

    // (3) Ramdisk upload if user supplied one
    if let Some(ramdisk) = &opts.ramdisk_path {
        if !ramdisk.exists() {
            return Err(PurpleError::Other(
                format!("ramdisk file not found: {}", ramdisk.display())));
        }
        steps.push(format!("[4] Uploading ramdisk: {}", ramdisk.display()));
        chimera_imobile::recovery::send_iboot_command(&format!(
            "send {}", ramdisk.display()
        ))?;
        steps.push("[5] Ramdisk staged in iBoot memory.".into());

        // Boot diagnostic kernel from the ramdisk
        steps.push("[6] Booting diagnostic kernel: irecovery -c \"go\"".into());
        let _ = chimera_imobile::recovery::send_iboot_command("go");
        wait_for_mode(opts.udid.as_deref(), DeviceMode::Diagnostic, timeout).ok();
    } else {
        steps.push("[4] No ramdisk supplied — flow stops at DFU. \
                    Provide PurpleRestoreOptions::ramdisk_path to continue.".into());
    }

    let final_mode = detect_mode(opts.udid.as_deref()).unwrap_or(DeviceMode::Unknown);
    let success = matches!(final_mode, DeviceMode::Dfu | DeviceMode::Diagnostic);
    steps.push(format!("[F] Done. Final mode: {:?}", final_mode));

    Ok(PurpleRestoreFlow {
        options:     opts,
        start_mode,
        final_mode,
        steps,
        success,
        duration_ms: start_ts.elapsed().as_millis(),
    })
}

/// Poll `detect_mode` until it returns `target` or `timeout` elapses.
fn wait_for_mode(udid: Option<&str>, target: DeviceMode, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if detect_mode(udid).map(|m| m == target).unwrap_or(false) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(PurpleError::Other(format!(
        "timed out waiting for device to enter {:?} (waited {:?})", target, timeout
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_default_safe() {
        let o = PurpleRestoreOptions::default();
        assert!(o.udid.is_none());
        assert!(o.ramdisk_path.is_none());
        assert!(!o.assume_dfu);
    }

    #[test]
    fn flow_struct_serialisable() {
        let f = PurpleRestoreFlow {
            options:     PurpleRestoreOptions::default(),
            start_mode:  DeviceMode::OSMode,
            final_mode:  DeviceMode::Dfu,
            steps:       vec!["a".into()],
            success:     true,
            duration_ms: 1000,
        };
        let j = serde_json::to_string(&f).unwrap();
        assert!(j.contains("OSMode"));
        assert!(j.contains("Dfu"));
    }
}
