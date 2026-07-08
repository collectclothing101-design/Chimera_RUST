// chimera-firmware/src/downloader.rs
// Firmware downloader supporting Samsung, Xiaomi, and generic sources

use chimera_core::error::{ChimeraError, Result};
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::firmware_meta::FirmwareMeta;
use log::info;

/// Firmware downloader with progress reporting
pub struct FirmwareDownloader {
    download_dir: String,
}

impl FirmwareDownloader {
    pub fn new(download_dir: impl Into<String>) -> Self {
        Self { download_dir: download_dir.into() }
    }

    /// Download firmware from URL with progress tracking
    pub fn download(&self, url: &str, filename: &str, progress: Option<&ProgressSender>) -> Result<String> {
        std::fs::create_dir_all(&self.download_dir)?;
        let dest_path = format!("{}/{}", self.download_dir, filename);
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Download").step(format!("Downloading {}...", filename)).percent(0.0));
        }
        
        // Use reqwest blocking client
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(3600))
            .user_agent("ChimeraRS/1.0")
            .build()
            .map_err(|e| ChimeraError::Firmware(format!("HTTP client error: {}", e)))?;
        
        let mut response = client.get(url)
            .send()
            .map_err(|e| ChimeraError::Firmware(format!("Download failed: {}", e)))?;
        
        let total = response.content_length().unwrap_or(0);
        let mut downloaded = 0u64;
        let mut file = std::fs::File::create(&dest_path)?;
        
        let mut buf = vec![0u8; 65536];
        loop {
            use std::io::Read;
            let n = response.read(&mut buf)
                .map_err(|e| ChimeraError::Firmware(format!("Read error: {}", e)))?;
            if n == 0 { break; }
            
            use std::io::Write;
            file.write_all(&buf[..n])?;
            downloaded += n as u64;
            
            if let Some(tx) = progress {
                if total > 0 {
                    let pct = downloaded as f32 / total as f32 * 100.0;
                    let _ = tx.send(Progress::new("Firmware Download").step(format!("Downloading {}...", filename)).bytes(downloaded, total).percent(pct));
                }
            }
        }
        
        info!("Downloaded {} ({} bytes) to {}", filename, downloaded, dest_path);
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Download").step("Download complete").percent(100.0).complete());
        }
        
        Ok(dest_path)
    }

    /// Search for Samsung firmware (uses SamFW/SamMobile-style lookup)
    pub fn search_samsung_firmware(model: &str, region: &str) -> Vec<FirmwareMeta> {
        // In a real implementation, this would query Samsung's firmware server
        // For now, return a placeholder
        vec![FirmwareMeta {
            brand: "Samsung".into(),
            model: model.to_string(),
            model_code: Some(model.to_string()),
            version: "Latest".into(),
            pda: None,
            csc: Some(region.to_string()),
            region: Some(region.to_string()),
            format: chimera_core::firmware_meta::FirmwareFormat::SamsungTar,
            file_size: 0,
            checksum_md5: None,
            checksum_sha256: None,
            download_url: Some(format!("https://firmware.samsung.com/{}/{}/latest", model, region)),
            local_path: None,
        }]
    }

    /// Search for Xiaomi firmware
    pub fn search_xiaomi_firmware(model: &str, region: Option<&str>) -> Vec<FirmwareMeta> {
        vec![FirmwareMeta {
            brand: "Xiaomi".into(),
            model: model.to_string(),
            model_code: Some(model.to_string()),
            version: "Latest".into(),
            pda: None,
            csc: None,
            region: region.map(String::from),
            format: chimera_core::firmware_meta::FirmwareFormat::OtaZip,
            file_size: 0,
            checksum_md5: None,
            checksum_sha256: None,
            download_url: Some(format!("https://bigota.d.miui.com/{}/latest.zip", model)),
            local_path: None,
        }]
    }
}
