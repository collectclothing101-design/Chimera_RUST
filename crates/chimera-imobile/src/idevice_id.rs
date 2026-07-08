//! Wrap `idevice_id` — lists UDIDs of currently-connected iOS devices.
//!
//! Usage:
//!     idevice_id -l        # list UDIDs over USB + network, one per line
//!     idevice_id -n        # network only
//!     idevice_id -u UDID   # show specific device

use std::time::Duration;
use serde::{Serialize, Deserialize};
use crate::tool::{run, ImobileTool, ImobileError};

/// One entry as returned by `idevice_id -l`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdidEntry {
    pub udid:      String,
    /// "USB" or "Network" — captured from `-l` output prefix if present.
    pub transport: Transport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transport {
    Usb,
    Network,
    Unknown,
}

/// List every iOS device currently paired with usbmuxd.
///
/// Returns an empty vector when no devices are connected. Returns an error
/// only when the binary itself is missing or fails to launch.
pub fn list() -> Result<Vec<UdidEntry>, ImobileError> {
    let output = run(ImobileTool::IdeviceId, &["-l"], Duration::from_secs(5))?;
    parse_list(&String::from_utf8_lossy(&output.stdout))
}

/// Network-only listing (for paired Wi-Fi devices).
pub fn list_network() -> Result<Vec<UdidEntry>, ImobileError> {
    let output = run(ImobileTool::IdeviceId, &["-n"], Duration::from_secs(5))?;
    let mut entries = parse_list(&String::from_utf8_lossy(&output.stdout))?;
    for e in &mut entries { e.transport = Transport::Network; }
    Ok(entries)
}

fn parse_list(stdout: &str) -> Result<Vec<UdidEntry>, ImobileError> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        // idevice_id -l output is usually just one UDID per line. Some
        // builds emit "<udid> (Network)" or "<udid> (USB)".
        let (udid, transport) = if let Some(idx) = line.find('(') {
            let (head, tag) = line.split_at(idx);
            let head = head.trim();
            let tag  = tag.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase();
            let t = match tag.as_str() {
                "usb"     => Transport::Usb,
                "network" => Transport::Network,
                _         => Transport::Unknown,
            };
            (head.to_string(), t)
        } else {
            (line.to_string(), Transport::Usb)
        };
        if udid.len() >= 20 {  // sane UDID length sanity check (16/25/40)
            out.push(UdidEntry { udid, transport });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_udid() {
        let r = parse_list("00008030-001A2B3C4D5E6F70\n").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].udid, "00008030-001A2B3C4D5E6F70");
        assert_eq!(r[0].transport, Transport::Usb);
    }

    #[test]
    fn parses_network_tag() {
        let r = parse_list("00008030-001A2B3C4D5E6F70 (Network)\n").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].transport, Transport::Network);
    }

    #[test]
    fn parses_multiple_devices() {
        let r = parse_list("00008030-001A2B3C4D5E6F70\n00008101-000123456789ABCD\n").unwrap();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn ignores_short_lines() {
        let r = parse_list("\n\nshort\nvalid-udid-1234567890123456\n").unwrap();
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn empty_input_returns_empty() {
        let r = parse_list("").unwrap();
        assert!(r.is_empty());
    }
}
