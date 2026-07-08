// chimera-core/src/firmware_meta.rs
use serde::{Deserialize, Serialize};

/// Firmware file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareMeta {
    pub brand: String,
    pub model: String,
    pub model_code: Option<String>,
    pub version: String,
    pub pda: Option<String>,
    pub csc: Option<String>,
    pub region: Option<String>,
    pub format: FirmwareFormat,
    pub file_size: u64,
    pub checksum_md5: Option<String>,
    pub checksum_sha256: Option<String>,
    pub download_url: Option<String>,
    pub local_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirmwareFormat {
    /// Samsung .tar, .tar.md5, .lz4
    SamsungTar,
    /// Qualcomm MBN/QFIL
    QualcommMbn,
    /// MTK scatter file + images
    MtkScatter,
    /// Unisoc PAC file
    UnisocPac,
    /// Huawei UPDATE.APP
    HuaweiUpdate,
    /// Xiaomi fastboot images (zip)
    XiaomiFastboot,
    /// AOSP/standard OTA zip
    OtaZip,
    /// Firmware archive containing multiple files
    Archive,
    Unknown,
}

impl FirmwareMeta {
    pub fn detect_format(path: &str) -> FirmwareFormat {
        let lower = path.to_lowercase();
        if lower.ends_with(".pac") {
            FirmwareFormat::UnisocPac
        } else if lower.ends_with(".tar") || lower.ends_with(".tar.md5") || lower.ends_with(".lz4") {
            FirmwareFormat::SamsungTar
        } else if lower.contains("scatter") || lower.ends_with(".bin") {
            FirmwareFormat::MtkScatter
        } else if lower.ends_with(".app") || lower.contains("update.app") {
            FirmwareFormat::HuaweiUpdate
        } else if lower.ends_with(".zip") {
            FirmwareFormat::OtaZip
        } else {
            FirmwareFormat::Unknown
        }
    }
}
