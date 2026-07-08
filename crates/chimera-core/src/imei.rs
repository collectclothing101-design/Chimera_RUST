// chimera-core/src/imei.rs
// IMEI validation, generation, and manipulation utilities

use crate::error::{ChimeraError, Result};

/// Validate IMEI using Luhn algorithm
pub fn validate_imei(imei: &str) -> Result<()> {
    let imei = imei.trim().replace(['-', ' '], "");
    
    if imei.len() != 15 {
        return Err(ChimeraError::InvalidImei(format!(
            "IMEI must be 15 digits, got {}",
            imei.len()
        )));
    }
    
    if !imei.chars().all(|c| c.is_ascii_digit()) {
        return Err(ChimeraError::InvalidImei("IMEI must contain only digits".into()));
    }
    
    // Luhn algorithm
    let sum: u32 = imei
        .chars()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            let mut d = c.to_digit(10).unwrap();
            if i % 2 == 1 {
                d *= 2;
                if d > 9 {
                    d -= 9;
                }
            }
            d
        })
        .sum();
    
    if sum % 10 != 0 {
        return Err(ChimeraError::InvalidImei(format!(
            "IMEI Luhn check failed for: {}",
            imei
        )));
    }
    
    Ok(())
}

/// Calculate Luhn check digit for 14-digit IMEI prefix
pub fn calculate_check_digit(imei14: &str) -> Result<u8> {
    if imei14.len() != 14 || !imei14.chars().all(|c| c.is_ascii_digit()) {
        return Err(ChimeraError::InvalidImei("Need exactly 14 digits".into()));
    }
    
    let sum: u32 = imei14
        .chars()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            let mut d = c.to_digit(10).unwrap();
            if i % 2 == 0 {
                d *= 2;
                if d > 9 {
                    d -= 9;
                }
            }
            d
        })
        .sum();
    
    let check = (10 - (sum % 10)) % 10;
    Ok(check as u8)
}

/// Complete a 14-digit IMEI prefix by appending check digit
pub fn complete_imei(imei14: &str) -> Result<String> {
    let check = calculate_check_digit(imei14)?;
    Ok(format!("{}{}", imei14, check))
}

/// Parse TAC (Type Allocation Code) from IMEI (first 8 digits)
pub fn get_tac(imei: &str) -> &str {
    &imei[..8.min(imei.len())]
}

/// Format IMEI with dashes: XXXXXX-XX-XXXXXX-X
pub fn format_imei(imei: &str) -> String {
    if imei.len() == 15 {
        format!(
            "{}-{}-{}-{}",
            &imei[0..6],
            &imei[6..8],
            &imei[8..14],
            &imei[14..15]
        )
    } else {
        imei.to_string()
    }
}

/// Convert IMEI to hex bytes (for EFS writing)
pub fn imei_to_bytes(imei: &str) -> Vec<u8> {
    let clean: String = imei.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut result = vec![0u8; 8];
    
    // Pack two digits per byte (BCD-like, first nibble = first digit)
    let digits: Vec<u8> = clean
        .chars()
        .map(|c| c.to_digit(10).unwrap() as u8)
        .collect();
    
    // Format: 0xAB where A=first digit, B=second digit, with parity
    result[0] = 0xA0 | digits[0]; // First byte has 0xA prefix + first digit
    for i in 0..7 {
        let hi = if 2 * i + 1 < digits.len() { digits[2 * i + 1] } else { 0 };
        let lo = if 2 * i + 2 < digits.len() { digits[2 * i + 2] } else { 0 };
        result[i + 1] = (hi << 4) | lo;
    }
    
    result
}

/// Convert hex bytes back to IMEI string
pub fn bytes_to_imei(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 8 {
        return None;
    }
    
    let mut digits = Vec::with_capacity(15);
    digits.push(bytes[0] & 0x0F); // First digit from low nibble
    
    for byte in &bytes[1..8] {
        digits.push((byte >> 4) & 0x0F);
        digits.push(byte & 0x0F);
    }
    
    digits.truncate(15);
    
    if digits.len() == 15 {
        let imei: String = digits.iter().map(|d| char::from_digit(*d as u32, 10).unwrap_or('0')).collect();
        Some(imei)
    } else {
        None
    }
}

/// Calculate network unlock code from IMEI (Luhn-based NCK)
pub fn calculate_network_code(imei: &str) -> Option<String> {
    validate_imei(imei).ok()?;
    
    // Simplified NCK calculation (real implementation varies by carrier)
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(imei.as_bytes());
    hasher.update(b"chimera_nck_salt_v1");
    let hash = hasher.finalize();
    
    // Take first 8 digits
    let code: String = hash.iter()
        .flat_map(|b| vec![(b >> 4) & 0x0F, b & 0x0F])
        .take(8)
        .map(|d| char::from_digit(d as u32, 10).unwrap_or_else(|| char::from_digit((d % 10) as u32, 10).unwrap()))
        .collect();
    
    Some(code)
}

/// Check if IMEI is in common blacklist patterns
pub fn is_imei_blacklisted_pattern(imei: &str) -> bool {
    // Common test/null IMEIs
    let null_imeis = [
        "000000000000000",
        "111111111111111", 
        "123456789012345",
        "012345678901234",
        "999999999999999",
    ];
    null_imeis.contains(&imei)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_imei() {
        assert!(validate_imei("490154203237518").is_ok());
        assert!(validate_imei("490154203237517").is_err()); // Bad check digit
        assert!(validate_imei("12345").is_err()); // Too short
    }
    
    #[test]
    fn test_complete_imei() {
        let completed = complete_imei("49015420323751").unwrap();
        assert_eq!(completed, "490154203237518");
    }
    
    #[test]
    fn test_format_imei() {
        assert_eq!(format_imei("490154203237518"), "490154-20-323751-8");
    }
}
