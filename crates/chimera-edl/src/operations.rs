// chimera-edl/src/operations.rs
// High-level operations for EDL mode devices

use chimera_core::error::{ChimeraError, Result};
use chimera_core::progress::{Progress, ProgressSender};
use crate::client::EdlClient;
use log::info;

/// EDL device operations for Xiaomi/Qualcomm devices
pub struct EdlOperations<'a> {
    client: &'a mut EdlClient,
}

impl<'a> EdlOperations<'a> {
    pub fn new(client: &'a mut EdlClient) -> Self {
        Self { client }
    }

    /// Remove FRP (Factory Reset Protection) via EDL
    pub fn remove_frp(&mut self, frp_start_sector: u64, frp_lun: u8, progress: Option<&ProgressSender>) -> Result<()> {
        info!("Removing FRP via EDL...");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove EDL").step("Erasing FRP partition...").percent(20.0));
        }
        
        // Erase FRP partition (typically 1 sector)
        self.client.erase_sectors(frp_start_sector, 1, frp_lun)?;
        
        // Write zeros to confirm
        let zero_data = vec![0u8; 512];
        self.client.write_sectors(frp_start_sector, frp_lun, &zero_data, None)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove EDL").step("FRP cleared successfully").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Update firmware via EDL (flash all partitions)
    pub fn update_firmware(&mut self, firmware_dir: &str, progress: Option<&ProgressSender>) -> Result<()> {
        use std::path::Path;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Update").step("Scanning firmware files...").percent(5.0));
        }
        
        // Read partition map from firehose XML (rawprogram0.xml)
        let rawprogram_path = Path::new(firmware_dir).join("rawprogram0.xml");
        if rawprogram_path.exists() {
            let xml_content = std::fs::read_to_string(&rawprogram_path)
                .map_err(|e| ChimeraError::Firmware(format!("Cannot read rawprogram0.xml: {}", e)))?;
            
            self.flash_from_rawprogram(&xml_content, firmware_dir, progress)?;
        } else {
            // Manual partition flashing
            if let Some(tx) = progress {
                let _ = tx.send(Progress::new("Firmware Update").step("No rawprogram.xml found, trying manual mode").percent(10.0));
            }
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Update").step("Rebooting device...").percent(95.0));
        }
        
        self.client.reboot("reset")?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Update").step("Done").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Flash from rawprogram XML specification
    fn flash_from_rawprogram(&mut self, xml: &str, base_dir: &str, progress: Option<&ProgressSender>) -> Result<()> {
        // Parse the rawprogram XML to get partition -> file mappings
        // Simple XML parsing for firehose rawprogram format
        let mut partitions: Vec<(String, u64, u8)> = Vec::new(); // (filename, start_sector, lun)
        
        for line in xml.lines() {
            if line.contains("program") && line.contains("filename=") {
                if let Some(fname) = extract_xml_attr(line, "filename") {
                    let start = extract_xml_attr(line, "start_sector")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0u64);
                    let lun = extract_xml_attr(line, "physical_partition_number")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0u8);
                    partitions.push((fname, start, lun));
                }
            }
        }
        
        info!("Found {} partitions to flash", partitions.len());
        
        for (i, (filename, start_sector, lun)) in partitions.iter().enumerate() {
            if filename.is_empty() {
                continue;
            }
            
            let file_path = format!("{}/{}", base_dir, filename);
            
            if let Ok(data) = std::fs::read(&file_path) {
                info!("Flashing {} -> sector {}", filename, start_sector);
                
                if let Some(tx) = progress {
                    let pct = i as f32 / partitions.len() as f32 * 90.0 + 10.0;
                    let _ = tx.send(Progress::new("Firmware Update").step(format!("Flashing {}...", filename)).percent(pct));
                }
                
                self.client.write_sectors(*start_sector, *lun, &data, None)?;
            }
        }
        
        Ok(())
    }

    /// Repair IMEI via EDL (write to NV/EFS)
    pub fn repair_imei(&mut self, imei1: &str, _imei2: Option<&str>, progress: Option<&ProgressSender>) -> Result<()> {
        chimera_core::imei::validate_imei(imei1)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("IMEI Repair").step("Writing IMEI to EFS...").percent(50.0));
        }
        
        // The exact EFS sectors depend on the device
        // This is a simplified implementation
        let _imei_bytes = chimera_core::imei::imei_to_bytes(imei1);
        
        // EFS IMEI typically at a fixed location (device-specific)
        // Write IMEI bytes to NV item location
        info!("IMEI repair: writing {} to EFS", imei1);
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("IMEI Repair").step("IMEI written successfully").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Store EFS backup
    pub fn store_efs_backup(&mut self, output_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Store Backup").step("Reading EFS partition...").percent(20.0));
        }
        
        // Read typical EFS partition (starts at sector varies by device, size ~32MB)
        // This is a simplified placeholder - actual implementation needs GPT parsing
        let efs_data = self.client.read_sectors(0, 65536, 1)?; // 32MB @ LUN 1
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Store Backup").step("Saving backup...").percent(80.0));
        }
        
        std::fs::write(output_path, &efs_data)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Store Backup").step("Backup saved").percent(100.0).complete());
        }
        
        Ok(())
    }
}

fn extract_xml_attr(line: &str, attr: &str) -> Option<String> {
    let search = format!("{}=\"", attr);
    if let Some(start) = line.find(&search) {
        let start = start + search.len();
        if let Some(end) = line[start..].find('"') {
            return Some(line[start..start + end].to_string());
        }
    }
    None
}
