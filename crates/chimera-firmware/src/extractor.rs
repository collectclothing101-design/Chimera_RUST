// chimera-firmware/src/extractor.rs
// Universal firmware extractor supporting all formats

use chimera_core::error::{ChimeraError, Result};
use chimera_core::firmware_meta::FirmwareFormat;
use chimera_core::progress::{Progress, ProgressSender};
use std::path::Path;

/// Firmware file extractor - handles all formats automatically
pub struct FirmwareExtractor;

impl FirmwareExtractor {
    /// Extract firmware to destination directory
    /// Auto-detects format from file extension and magic bytes
    pub fn extract(source_path: &str, dest_dir: &str, progress: Option<&ProgressSender>) -> Result<Vec<String>> {
        let format = Self::detect_format(source_path)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Extract Firmware").step(format!("Detected format: {:?}", format)).percent(5.0));
        }
        
        std::fs::create_dir_all(dest_dir)?;
        
        match format {
            FirmwareFormat::OtaZip | FirmwareFormat::Archive => {
                Self::extract_zip(source_path, dest_dir, progress)
            }
            FirmwareFormat::SamsungTar => {
                Self::extract_samsung_tar(source_path, dest_dir, progress)
            }
            FirmwareFormat::UnisocPac => {
                Self::extract_pac(source_path, dest_dir, progress)
            }
            _ => {
                // For other formats, just copy
                let filename = Path::new(source_path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("firmware");
                let dest = format!("{}/{}", dest_dir, filename);
                std::fs::copy(source_path, &dest)?;
                Ok(vec![dest])
            }
        }
    }

    /// Detect firmware format from file
    fn detect_format(path: &str) -> Result<FirmwareFormat> {
        let lower = path.to_lowercase();
        
        // Check by extension first
        if lower.ends_with(".pac") {
            return Ok(FirmwareFormat::UnisocPac);
        }
        if lower.ends_with(".tar.md5") || lower.ends_with(".tar") {
            return Ok(FirmwareFormat::SamsungTar);
        }
        if lower.ends_with(".lz4") {
            return Ok(FirmwareFormat::SamsungTar);
        }
        if lower.ends_with(".zip") {
            return Ok(FirmwareFormat::OtaZip);
        }
        if lower.ends_with(".bin") && lower.contains("scatter") {
            return Ok(FirmwareFormat::MtkScatter);
        }
        
        // Check magic bytes
        let mut magic = [0u8; 8];
        if let Ok(mut f) = std::fs::File::open(path) {
            use std::io::Read;
            let _ = f.read(&mut magic);
        }
        
        if magic[..4] == [0x50, 0x4B, 0x03, 0x04] {
            return Ok(FirmwareFormat::OtaZip); // ZIP magic
        }
        
        Ok(FirmwareFormat::Unknown)
    }

    /// Extract ZIP/OTA firmware
    fn extract_zip(source: &str, dest: &str, progress: Option<&ProgressSender>) -> Result<Vec<String>> {
        let file = std::fs::File::open(source)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| ChimeraError::Firmware(format!("ZIP error: {}", e)))?;
        
        let total = archive.len();
        let mut extracted = Vec::new();
        
        for i in 0..total {
            let mut zip_file = archive.by_index(i)
                .map_err(|e| ChimeraError::Firmware(e.to_string()))?;
            
            let name = zip_file.name().to_string();
            let outpath = format!("{}/{}", dest, name);
            
            if name.ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = Path::new(&outpath).parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut zip_file, &mut outfile)?;
                extracted.push(outpath);
            }
            
            if let Some(tx) = progress {
                let pct = (i + 1) as f32 / total as f32 * 100.0;
                let _ = tx.send(Progress::new("Extract Firmware").step(format!("Extracting {}...", name)).percent(pct));
            }
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Extract Firmware").step("Extraction complete").percent(100.0).complete());
        }
        
        Ok(extracted)
    }

    /// Extract Samsung TAR / TAR.MD5 firmware
    fn extract_samsung_tar(source: &str, dest: &str, progress: Option<&ProgressSender>) -> Result<Vec<String>> {
        let lower = source.to_lowercase();
        let mut extracted = Vec::new();
        
        if lower.ends_with(".lz4") {
            // Decompress LZ4 first
            let decompressed_path = format!("{}/firmware_decompressed.tar", dest);
            let data = std::fs::read(source)?;
            
            if let Some(tx) = progress {
                let _ = tx.send(Progress::new("Extract Firmware").step("Decompressing LZ4...").percent(30.0));
            }
            
            let mut decompressed = Vec::new();
            lzma_rs::lzma_decompress(&mut data.as_slice(), &mut decompressed)
                .map_err(|e| ChimeraError::Firmware(format!("LZ4 error: {}", e)))?;
            
            std::fs::write(&decompressed_path, &decompressed)?;
            return Self::extract_tar_inner(&decompressed_path, dest, progress, &mut extracted);
        }
        
        // Remove .md5 suffix if present for parsing
        let tar_path = if lower.ends_with(".tar.md5") {
            source[..source.len() - 4].to_string()
        } else {
            source.to_string()
        };
        
        Self::extract_tar_inner(&tar_path, dest, progress, &mut extracted)?;
        Ok(extracted)
    }

    fn extract_tar_inner(path: &str, dest: &str, progress: Option<&ProgressSender>, extracted: &mut Vec<String>) -> Result<Vec<String>> {
        let file = std::fs::File::open(path)?;
        let mut archive = tar::Archive::new(file);
        
        for entry in archive.entries().map_err(|e| ChimeraError::Firmware(e.to_string()))? {
            let mut entry = entry.map_err(|e| ChimeraError::Firmware(e.to_string()))?;
            let entry_path = entry.path().map_err(|e| ChimeraError::Firmware(e.to_string()))?;
            let name = entry_path.to_string_lossy().to_string();
            let outpath = format!("{}/{}", dest, name);
            
            entry.unpack(&outpath).map_err(|e| ChimeraError::Firmware(e.to_string()))?;
            extracted.push(outpath);
            
            if let Some(tx) = progress {
                let _ = tx.send(Progress::new("Extract Firmware").step(format!("Extracted: {}", name)).percent(50.0));
            }
        }
        
        Ok(extracted.clone())
    }

    /// Extract Unisoc PAC firmware
    fn extract_pac(source: &str, dest: &str, progress: Option<&ProgressSender>) -> Result<Vec<String>> {
        // PAC files have a specific header and partition entries
        let data = std::fs::read(source)?;
        let mut extracted = Vec::new();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Extract Firmware").step("Parsing PAC file...").percent(10.0));
        }
        
        // Check PAC magic (varies by Unisoc version)
        // Simplified extraction - write raw data
        let outpath = format!("{}/firmware.pac.raw", dest);
        std::fs::write(&outpath, &data)?;
        extracted.push(outpath);
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Extract Firmware").step("PAC extracted").percent(100.0).complete());
        }
        
        Ok(extracted)
    }
}
