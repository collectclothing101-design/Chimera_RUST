// chimera-core/src/mac_address.rs
// MAC address utilities: validation, generation, NV backup/restore

use crate::error::{ChimeraError, Result};

/// Validate a MAC address string (XX:XX:XX:XX:XX:XX)
pub fn validate_mac(mac: &str) -> Result<[u8; 6]> {
    let clean = mac.replace(['-', '.'], ":");
    let parts: Vec<&str> = clean.split(':').collect();
    if parts.len() != 6 {
        return Err(ChimeraError::Parse(format!("Invalid MAC format: {}", mac)));
    }
    let mut bytes = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        bytes[i] = u8::from_str_radix(part, 16)
            .map_err(|_| ChimeraError::Parse(format!("Invalid MAC octet: {}", part)))?;
    }
    Ok(bytes)
}

/// Format MAC bytes as XX:XX:XX:XX:XX:XX
pub fn format_mac(bytes: &[u8; 6]) -> String {
    format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5])
}

/// Check if the MAC is a locally administered address (private)
pub fn is_locally_administered(bytes: &[u8; 6]) -> bool {
    bytes[0] & 0x02 != 0
}

/// Check if the MAC is a multicast address
pub fn is_multicast(bytes: &[u8; 6]) -> bool {
    bytes[0] & 0x01 != 0
}

/// Derive a valid, locally-administered MAC from a seed (e.g., IMEI)
pub fn derive_mac_from_seed(seed: &str, index: u8) -> [u8; 6] {
    use sha2::{Sha256, Digest};
    let mut h = Sha256::new();
    h.update(seed.as_bytes());
    h.update(&[index]);
    let hash = h.finalize();
    let mut mac = [hash[0], hash[1], hash[2], hash[3], hash[4], hash[5]];
    // Make locally administered, unicast
    mac[0] = (mac[0] | 0x02) & 0xFE;
    mac
}

/// Samsung NV item index for Wi-Fi MAC
pub const SAMSUNG_NV_WIFI_MAC: u32 = 4678;
/// Samsung NV item index for BT MAC
pub const SAMSUNG_NV_BT_MAC: u32 = 447;

/// Pack MAC bytes into Samsung NV format
pub fn pack_samsung_nv_mac(bytes: &[u8; 6]) -> Vec<u8> {
    bytes.to_vec()
}

/// Unpack Samsung NV format to MAC bytes
pub fn unpack_samsung_nv_mac(data: &[u8]) -> Option<[u8; 6]> {
    if data.len() >= 6 {
        let mut b = [0u8; 6];
        b.copy_from_slice(&data[..6]);
        Some(b)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_validate_mac() {
        assert!(validate_mac("AA:BB:CC:DD:EE:FF").is_ok());
        assert!(validate_mac("AA-BB-CC-DD-EE-FF").is_ok());
        assert!(validate_mac("ZZ:00:00:00:00:00").is_err());
        assert!(validate_mac("AA:BB:CC").is_err());
    }
    #[test]
    fn test_format_mac() {
        let b = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        assert_eq!(format_mac(&b), "AA:BB:CC:DD:EE:FF");
    }
    #[test]
    fn test_derive_mac() {
        let mac = derive_mac_from_seed("490154203237518", 0);
        assert!(!is_multicast(&mac));
        assert!(is_locally_administered(&mac));
    }
}
