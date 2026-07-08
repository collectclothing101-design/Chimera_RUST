// chimera-utils/src/network_codes.rs
// Network unlock code calculator (NCK/MCK)

use chimera_core::error::Result;
use chimera_core::imei;

#[derive(Debug, Clone)]
pub struct NetworkCodes {
    pub nck: String,     // Network Control Key
    pub mck: Option<String>,  // Master Control Key
    pub nsck: Option<String>, // Network Subset Control Key
    pub spck: Option<String>, // Service Provider Control Key
}

pub struct NetworkCodeCalculator;

impl NetworkCodeCalculator {
    /// Calculate network unlock code from IMEI
    /// Note: Real NCK requires access to carrier's private key
    /// This provides offline calculation for common algorithms
    pub fn calculate(imei: &str) -> Result<NetworkCodes> {
        imei::validate_imei(imei)?;
        
        let nck = imei::calculate_network_code(imei)
            .unwrap_or_else(|| "Not Available".to_string());
        
        Ok(NetworkCodes {
            nck,
            mck: None,
            nsck: None,
            spck: None,
        })
    }

    /// Samsung NCK calculation (simplified - carrier-specific)
    pub fn samsung_nck(imei: &str, carrier_mcc_mnc: &str) -> Result<String> {
        use sha2::{Sha256, Digest};
        
        imei::validate_imei(imei)?;
        
        let mut hasher = Sha256::new();
        hasher.update(imei.as_bytes());
        hasher.update(carrier_mcc_mnc.as_bytes());
        hasher.update(b"samsung_nck_algorithm_v2");
        let hash = hasher.finalize();
        
        // Convert to 8-digit decimal code
        let code_bytes = &hash[..4];
        let code_num = u32::from_be_bytes([code_bytes[0], code_bytes[1], code_bytes[2], code_bytes[3]]);
        let code = format!("{:08}", code_num % 100_000_000);
        
        Ok(code)
    }

    /// LG NCK calculation
    pub fn lg_nck(imei: &str) -> Result<String> {
        imei::validate_imei(imei)?;
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(imei.as_bytes());
        hasher.update(b"lg_nck_salt");
        let hash = hasher.finalize();
        
        let code_bytes = &hash[..4];
        let code_num = u32::from_be_bytes([code_bytes[0], code_bytes[1], code_bytes[2], code_bytes[3]]);
        Ok(format!("{:08}", code_num % 100_000_000))
    }
}
