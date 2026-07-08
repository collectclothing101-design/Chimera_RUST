// chimera-firmware/src/checker.rs
// Firmware integrity checker (checksum verification)
use chimera_core::error::Result;
use sha2::{Sha256, Digest};
use md5::Md5;

pub struct FirmwareChecker;

impl FirmwareChecker {
    pub fn verify_md5(path: &str, expected_md5: &str) -> Result<bool> {
        let data = std::fs::read(path)?;
        let mut hasher = Md5::new();
        hasher.update(&data);
        let actual = hex::encode(hasher.finalize());
        Ok(actual.eq_ignore_ascii_case(expected_md5))
    }

    pub fn verify_sha256(path: &str, expected: &str) -> Result<bool> {
        let data = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let actual = hex::encode(hasher.finalize());
        Ok(actual.eq_ignore_ascii_case(expected))
    }

    pub fn calculate_md5(path: &str) -> Result<String> {
        let data = std::fs::read(path)?;
        let mut hasher = Md5::new();
        hasher.update(&data);
        Ok(hex::encode(hasher.finalize()))
    }
}
