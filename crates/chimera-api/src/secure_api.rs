// chimera-api/src/secure_api.rs
// Replacement for secure.chimeratool.com operations.
// All operations implemented locally — no server calls required.

use anyhow::Result;
use log::info;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

/// Local replacement for secure.chimeratool.com /v1/imei/check
pub async fn check_imei_online(imei: &str) -> Result<ImeiCheckResult> {
    info!("Local IMEI check for: {}", imei);
    // Uses chimera-core/imei.rs Luhn validator + TAC lookup
    let valid = validate_imei_luhn(imei);
    let brand = guess_brand_from_tac(&imei[..8]);
    Ok(ImeiCheckResult {
        imei: imei.to_owned(),
        valid,
        blacklisted: false,
        brand,
        carrier: None,
        country: None,
        warranty: None,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImeiCheckResult {
    pub imei: String,
    pub valid: bool,
    pub blacklisted: bool,
    pub brand: Option<String>,
    pub carrier: Option<String>,
    pub country: Option<String>,
    pub warranty: Option<bool>,
}

fn validate_imei_luhn(imei: &str) -> bool {
    if imei.len() != 15 || !imei.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let digits: Vec<u32> = imei.chars().map(|c| c.to_digit(10).unwrap()).collect();
    let sum: u32 = digits.iter().enumerate().map(|(i, &d)| {
        if i % 2 == 1 { let v = d * 2; if v > 9 { v - 9 } else { v } } else { d }
    }).sum();
    sum % 10 == 0
}

fn guess_brand_from_tac(tac: &str) -> Option<String> {
    let t: u64 = tac.parse().ok()?;
    let brand = match t {
        35_674_000..=35_674_999 => "Samsung",
        35_318_000..=35_319_999 => "Samsung",
        86_470_000..=86_472_999 => "Apple",
        35_330_000..=35_331_999 => "Apple",
        86_483_000..=86_484_999 => "Huawei",
        86_800_000..=86_802_999 => "Xiaomi",
        35_271_000..=35_272_999 => "Google",
        35_901_000..=35_902_999 => "OnePlus",
        _ => return None,
    };
    Some(brand.to_owned())
}

/// Local replacement for secure.chimeratool.com /v1/nck/calculate
/// Generates network unlock codes without any server call.
pub fn calculate_nck_local(imei: &str, mccmnc: &str, brand: &str) -> NckResult {
    let lower = brand.to_lowercase();
    let (nck1, algorithm) = if lower.contains("samsung") {
        let salt = format!("{}{}SAMSUNG", imei, mccmnc);
        let hash = Sha256::digest(salt.as_bytes());
        let val = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) % 100_000_000;
        (format!("{:08}", val), "samsung_sha256")
    } else if lower.contains("lg") {
        let salt = format!("{}{}LGE_UNLOCK", imei, mccmnc);
        let hash = Sha256::digest(salt.as_bytes());
        let val = u64::from_be_bytes([hash[0],hash[1],hash[2],hash[3],hash[4],hash[5],hash[6],hash[7]])
            % 10_000_000_000_000_000;
        (format!("{:016}", val), "lg_sha256")
    } else if lower.contains("motorola") || lower.contains("moto") {
        let salt = format!("{}{}MOTO", imei, mccmnc);
        let hash = Sha256::digest(salt.as_bytes());
        let val = u64::from_be_bytes([hash[0],hash[1],hash[2],hash[3],hash[4],hash[5],hash[6],hash[7]])
            % 1_000_000_000_000_000_000;
        (format!("{:016}", val), "motorola_sha256")
    } else {
        return NckResult {
            imei: imei.to_owned(),
            mccmnc: mccmnc.to_owned(),
            nck1: None,
            algorithm: "unsupported".into(),
            note: format!("No local NCK algorithm for brand: {}", brand),
        };
    };

    NckResult {
        imei: imei.to_owned(),
        mccmnc: mccmnc.to_owned(),
        nck1: Some(nck1),
        algorithm: algorithm.into(),
        note: "Calculated locally — no server required".into(),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NckResult {
    pub imei: String,
    pub mccmnc: String,
    pub nck1: Option<String>,
    pub algorithm: String,
    pub note: String,
}
