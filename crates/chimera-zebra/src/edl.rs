//! EDL (Emergency Download) entry helpers for Zebra TC5x.
//!
//! Three documented entry paths:
//!
//!   1. `adb reboot edl`           — engineering / debug builds only
//!   2. `fastboot oem reboot-edl`  — only on signed-firmware revs that
//!                                   ship the OEM extension
//!   3. **Test-points**            — two pads on the PCB shorted while
//!                                   USB is connected. Pad locations
//!                                   differ per model — refer to Zebra's
//!                                   Board Service Manual (NDA).
//!
//! Once in EDL the device exposes Qualcomm Sahara on bulk USB endpoint
//! 0x05C6:0x9008. The Firehose programmer needed to talk Firehose is
//! **signed with Zebra's OEM key** and ships inside the official
//! firmware bundle from `support.zebra.com`. We do not ship one.
//!
//! Once a programmer is loaded the actual partition I/O is handled by
//! `chimera-edl`.

use std::process::Command;
use serde::{Serialize, Deserialize};
use crate::{Result, ZebraError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdlEntryMethod {
    /// `adb reboot edl` — quickest, requires device in OSMode + dev build
    /// or post-test-point unlocked state.
    AdbReboot,
    /// `fastboot oem reboot-edl` — works on retail builds that ship the
    /// Zebra OEM extension.
    FastbootOemReboot,
    /// Hardware test-points shorted during USB enumeration.
    HardwareTestpoint,
}

/// Trigger EDL via the chosen method. The hardware-testpoint variant is
/// documentational — physical action required from the operator.
pub fn enter_edl(target: Option<&str>, method: EdlEntryMethod) -> Result<String> {
    match method {
        EdlEntryMethod::AdbReboot => {
            let probe = chimera_utils::host_probes::detect_adb();
            if !probe.found { return Err(ZebraError::Adb("adb missing".into())); }
            let adb = probe.path.as_ref().unwrap();
            let mut c = Command::new(adb);
            if let Some(s) = target { c.args(["-s", s]); }
            c.args(["reboot", "edl"]);
            let out = c.output().map_err(|e| ZebraError::Adb(e.to_string()))?;
            if !out.status.success() {
                return Err(ZebraError::Adb(
                    String::from_utf8_lossy(&out.stderr).trim().to_string()));
            }
            Ok("Issued `adb reboot edl`. Device should now enumerate as 05C6:9008.".into())
        }
        EdlEntryMethod::FastbootOemReboot => {
            let probe = chimera_utils::host_probes::detect_fastboot();
            if !probe.found { return Err(ZebraError::Fastboot("fastboot missing".into())); }
            let fb = probe.path.as_ref().unwrap();
            let mut c = Command::new(fb);
            if let Some(s) = target { c.args(["-s", s]); }
            c.args(["oem", "reboot-edl"]);
            let out = c.output().map_err(|e| ZebraError::Fastboot(e.to_string()))?;
            if !out.status.success() {
                return Err(ZebraError::Fastboot(
                    String::from_utf8_lossy(&out.stderr).trim().to_string()));
            }
            Ok("Issued `fastboot oem reboot-edl`. Watch for 05C6:9008 on next enumeration.".into())
        }
        EdlEntryMethod::HardwareTestpoint => {
            Ok("Hardware test-point entry is a physical operation. Power off the device, \
                short the two EDL pads near the SoC on the back PCB while connecting USB. \
                Pad locations differ per model — refer to Zebra Board Service Manual.".into())
        }
    }
}

/// Reverse — leave EDL via Sahara reset command (when programmer is loaded).
/// Without a programmer loaded, the only way out is power-off + battery pull.
pub fn leave_edl_via_sahara() -> Result<String> {
    // The actual Sahara RESET packet is 0x07 0x00 0x00 0x00 0x08 0x00 0x00 0x00
    // sent on the bulk endpoint. Implementation lives in chimera-edl.
    Err(ZebraError::Edl(
        "leave_edl_via_sahara() lives in chimera-edl::sahara::reset(); call that path".into()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn method_strings_render() {
        let _ = format!("{:?}", EdlEntryMethod::AdbReboot);
        let _ = format!("{:?}", EdlEntryMethod::FastbootOemReboot);
        let _ = format!("{:?}", EdlEntryMethod::HardwareTestpoint);
    }
    #[test]
    fn hw_testpoint_returns_instructions_only() {
        let r = enter_edl(None, EdlEntryMethod::HardwareTestpoint).unwrap();
        assert!(r.contains("test-point"));
        assert!(r.contains("Board Service Manual"));
    }
}
