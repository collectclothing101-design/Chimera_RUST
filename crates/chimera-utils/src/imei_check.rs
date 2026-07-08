// chimera-utils/src/imei_check.rs
// IMEI blacklist and validity checker

use chimera_core::error::{ChimeraError, Result};
use chimera_core::imei;

#[derive(Debug, Clone)]
pub struct ImeiCheckResult {
    pub imei: String,
    pub is_valid: bool,
    pub luhn_valid: bool,
    pub tac: String,
    pub brand_hint: Option<String>,
    pub is_blacklisted: bool,
    pub blacklist_source: Option<String>,
}

pub struct ImeiChecker;

impl ImeiChecker {
    /// Full IMEI check (local validation + optional online check)
    pub fn check(imei_str: &str) -> ImeiCheckResult {
        let imei = imei_str.trim().replace(['-', ' '], "");
        
        let luhn_valid = imei::validate_imei(&imei).is_ok();
        let tac = if imei.len() >= 8 { imei[..8].to_string() } else { imei.clone() };
        
        let brand_hint = Self::brand_from_tac(&tac);
        let is_blacklisted = imei::is_imei_blacklisted_pattern(&imei);
        
        ImeiCheckResult {
            imei: imei.clone(),
            is_valid: luhn_valid && imei.len() == 15,
            luhn_valid,
            tac,
            brand_hint,
            is_blacklisted,
            blacklist_source: None,
        }
    }

    /// Check IMEI against GSMA database (simplified local lookup)
    fn brand_from_tac(tac: &str) -> Option<String> {
        // TAC ranges for common manufacturers (partial list)
        let tac_num: u64 = tac.parse().ok()?;
        
        let brand = match tac_num {
            // Samsung
            35_180_000..=35_199_999 => "Samsung",
            35_290_000..=35_309_999 => "Samsung",
            35_400_000..=35_419_999 => "Samsung",
            // Apple
            35_289_000..=35_289_999 => "Apple",
            35_383_000..=35_384_999 => "Apple",
            // Huawei
            86_800_000..=86_819_999 => "Huawei",
            86_710_000..=86_719_999 => "Huawei",
            // Xiaomi
            86_890_000..=86_909_999 => "Xiaomi",
            // Google
            35_330_000..=35_349_999 => "Google",
            // OnePlus
            86_780_000..=86_789_999 => "OnePlus",
            _ => return None,
        };
        
        Some(brand.to_string())
    }

    /// Online IMEI check using public API
    pub fn check_online(imei: &str) -> Result<bool> {
        // Use a free public API for IMEI blacklist check
        // Returns true if clean, false if blacklisted
        let url = format!("https://www.imei.info/api/v2/imei/{}/?format=json", imei);
        
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("ChimeraRS/1.0")
            .build()
            .map_err(|e| ChimeraError::Unknown(e.to_string()))?;
        
        match client.get(&url).send() {
            Ok(resp) => {
                if resp.status().is_success() {
                    let body: serde_json::Value = resp.json()
                        .map_err(|e| ChimeraError::Parse(e.to_string()))?;
                    
                    // Parse blacklist status
                    let blacklisted = body.get("blacklisted")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    
                    Ok(!blacklisted)
                } else {
                    Err(ChimeraError::Unknown(format!("API returned status {}", resp.status())))
                }
            }
            Err(e) => Err(ChimeraError::Unknown(format!("API request failed: {}", e))),
        }
    }
}
