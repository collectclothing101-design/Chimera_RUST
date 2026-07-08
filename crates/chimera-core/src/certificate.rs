// chimera-core/src/certificate.rs
use serde::{Deserialize, Serialize};

/// Samsung/device certificate structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    pub cert_type: CertificateType,
    pub imei: Option<String>,
    pub data: Vec<u8>,
    pub version: u8,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CertificateType {
    Samsung,
    Huawei,
    Qualcomm,
    Generic,
}

impl Certificate {
    pub fn new(cert_type: CertificateType, data: Vec<u8>) -> Self {
        Self {
            cert_type,
            imei: None,
            data,
            version: 1,
            model: None,
        }
    }

    pub fn validate(&self) -> bool {
        !self.data.is_empty()
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

// chimera-core/src/firmware_meta.rs
