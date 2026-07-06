//! Device-mode detection — figures out whether an attached iDevice is in
//! OSMode (normal lockdownd), Recovery (iBoot), DFU (bootrom), Diagnostic
//! (Purple), Restore (iBSS/iBEC kernel), or PongoOS (post-checkm8).
//!
//! Detection is layered, fastest first:
//!   1. `idevice_id -l`            → if UDID listed → at minimum OSMode-paired
//!   2. `irecovery -q`             → emits ProductType + BootStage when in
//!                                   recovery / DFU / restore stages
//!   3. USB PID lookup via rusb    → fallback when neither daemon sees the
//!                                   device (works in DFU / WTF / PongoOS)

use serde::{Serialize, Deserialize};
use chimera_core::usb::lookup_device;
use chimera_core::device::ConnectionMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceMode {
    /// Normal iOS, lockdownd reachable, device paired or pair-pending.
    OSMode,
    /// iBoot Recovery (PID range 0x1280-0x1282).
    Recovery,
    /// DFU mode (canonical PID 0x1227).
    Dfu,
    /// Apple-internal Diagnostic ("Purple") boot.
    Diagnostic,
    /// In-restore-stage (iBSS / iBEC / kernel).
    Restore,
    /// Post-checkm8 PongoOS shell mode.
    PongoOs,
    /// WTF (iPod touch 1G specific bring-up mode).
    Wtf,
    Unknown,
}

impl From<ConnectionMode> for DeviceMode {
    fn from(c: ConnectionMode) -> Self {
        match c {
            ConnectionMode::AppleUsbMux   => DeviceMode::OSMode,
            ConnectionMode::AppleRecovery => DeviceMode::Recovery,
            ConnectionMode::AppleDfu      => DeviceMode::Dfu,
            ConnectionMode::AppleRestore  => DeviceMode::Restore,
            ConnectionMode::ApplePongoOs  => DeviceMode::PongoOs,
            ConnectionMode::AppleWtf      => DeviceMode::Wtf,
            _                             => DeviceMode::Unknown,
        }
    }
}

/// Detect the current mode for a UDID. `None` ⇒ first reachable device.
///
/// Layered detection — falls through cleanly when a faster probe fails.
pub fn detect_mode(udid: Option<&str>) -> Option<DeviceMode> {
    // (1) Is the device paired with usbmuxd? — that proves OSMode.
    if let Ok(list) = chimera_imobile::list_devices() {
        if let Some(u) = udid {
            if list.iter().any(|e| e.udid == u) { return Some(DeviceMode::OSMode); }
        } else if !list.is_empty() { return Some(DeviceMode::OSMode); }
    }

    // (2) Try irecovery to detect recovery/DFU/restore stages.
    if let Ok(out) = chimera_imobile::recovery::send_iboot_command("getenv build-style") {
        let lower = out.to_lowercase();
        if lower.contains("diagnostics") || lower.contains("purple") {
            return Some(DeviceMode::Diagnostic);
        }
        if lower.contains("pongo") { return Some(DeviceMode::PongoOs); }
        // BootStage hints
        if lower.contains("ibss") || lower.contains("ibec") || lower.contains("kernelcache") {
            return Some(DeviceMode::Restore);
        }
        // Default to Recovery when irecovery succeeds but doesn't say
        // anything specific — being in iBoot at all = Recovery mode.
        return Some(DeviceMode::Recovery);
    }

    // (3) USB descriptor fallback: walk rusb for an Apple device,
    // map the PID through the chimera-core USB DB.
    if let Ok(devs) = rusb_safe_enumerate() {
        for (vid, pid) in devs {
            if vid == 0x05AC {
                if let Some(entry) = lookup_device(vid, pid) {
                    return Some(DeviceMode::from(entry.mode.clone()));
                }
            }
        }
    }

    Some(DeviceMode::Unknown)
}

/// rusb enumeration wrapped so the rest of this module never panics if
/// libusb is missing on the host (some Linux CI images lack it).
fn rusb_safe_enumerate() -> std::result::Result<Vec<(u16, u16)>, ()> {
    use rusb::UsbContext;
    let ctx = rusb::Context::new().map_err(|_| ())?;
    let list = ctx.devices().map_err(|_| ())?;
    let mut out = Vec::new();
    for d in list.iter() {
        if let Ok(desc) = d.device_descriptor() {
            out.push((desc.vendor_id(), desc.product_id()));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_mode_serialisable() {
        let m = DeviceMode::Diagnostic;
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("Diagnostic"));
        let back: DeviceMode = serde_json::from_str(&s).unwrap();
        assert_eq!(back, DeviceMode::Diagnostic);
    }

    #[test]
    fn connection_mode_maps_correctly() {
        assert_eq!(DeviceMode::from(ConnectionMode::AppleDfu),      DeviceMode::Dfu);
        assert_eq!(DeviceMode::from(ConnectionMode::AppleRecovery), DeviceMode::Recovery);
        assert_eq!(DeviceMode::from(ConnectionMode::ApplePongoOs),  DeviceMode::PongoOs);
        assert_eq!(DeviceMode::from(ConnectionMode::AppleWtf),      DeviceMode::Wtf);
        assert_eq!(DeviceMode::from(ConnectionMode::AppleUsbMux),   DeviceMode::OSMode);
    }
}
