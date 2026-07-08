// chimera-core/src/backup.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A device security backup (EFS/certificate data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceBackup {
    pub version: u32,
    pub device_model: String,
    pub imei: Option<String>,
    pub imei2: Option<String>,
    pub mac_address: Option<String>,
    pub wifi_mac: Option<String>,
    pub bt_mac: Option<String>,
    pub certificate_data: Option<Vec<u8>>,
    pub efs_data: Option<Vec<u8>>,
    pub nvram_data: Option<Vec<u8>>,
    pub calibration_data: Option<Vec<u8>>,
    pub extra_fields: HashMap<String, Vec<u8>>,
    pub timestamp: u64,
    pub checksum: String,
}

impl DeviceBackup {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            version: 1,
            device_model: model.into(),
            imei: None,
            imei2: None,
            mac_address: None,
            wifi_mac: None,
            bt_mac: None,
            certificate_data: None,
            efs_data: None,
            nvram_data: None,
            calibration_data: None,
            extra_fields: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            checksum: String::new(),
        }
    }

    /// Serialize to JSON bytes
    pub fn to_bytes(&self) -> crate::error::Result<Vec<u8>> {
        Ok(serde_json::to_vec_pretty(self)?)
    }

    /// Deserialize from JSON bytes
    pub fn from_bytes(data: &[u8]) -> crate::error::Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }

    /// Calculate checksum of the backup data
    pub fn calculate_checksum(&mut self) {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&self.device_model.as_bytes());
        if let Some(imei) = &self.imei {
            hasher.update(imei.as_bytes());
        }
        if let Some(cert) = &self.certificate_data {
            hasher.update(cert);
        }
        if let Some(efs) = &self.efs_data {
            hasher.update(efs);
        }
        self.checksum = hex::encode(hasher.finalize());
    }
}
