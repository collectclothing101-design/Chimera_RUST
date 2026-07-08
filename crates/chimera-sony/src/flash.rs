// chimera-sony/src/flash.rs
// Sony Flashtool-compatible firmware flashing (.ftf format)

use chimera_core::error::{ChimeraError, Result};
use chimera_core::progress::{Progress, ProgressSender};
use chimera_fastboot::client::FastbootClient;
use std::path::Path;
use std::fs;
use log::info;

/// Parse an .ftf firmware archive (zip-based format)
pub struct FtfFirmware {
    pub partitions: Vec<FtfPartition>,
}

#[derive(Debug)]
pub struct FtfPartition {
    pub name: String,
    pub data: Vec<u8>,
    pub size: u64,
}

impl FtfFirmware {
    /// Load an .ftf file (Sony's zip-based format)
    pub fn load(path: &Path) -> Result<Self> {
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| ChimeraError::Firmware(e.to_string()))?;
        
        let mut partitions = Vec::new();
        
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| ChimeraError::Firmware(e.to_string()))?;
            
            let name = entry.name().to_string();
            
            // Skip metadata files
            if name.starts_with("META-INF") || name == "content.xml" {
                continue;
            }
            
            let size = entry.size();
            let mut data = Vec::with_capacity(size as usize);
            std::io::Read::read_to_end(&mut entry, &mut data)?;
            
            partitions.push(FtfPartition { name, data, size });
        }
        
        Ok(FtfFirmware { partitions })
    }

    /// Flash all partitions to device via fastboot
    pub fn flash_all(&self, fastboot: &mut FastbootClient, progress: Option<&ProgressSender>) -> Result<()> {
        let total = self.partitions.len();
        
        for (i, part) in self.partitions.iter().enumerate() {
            let pct = (i as f32 / total as f32) * 90.0;
            if let Some(tx) = progress {
                let _ = tx.send(Progress::new("Flash Firmware")
                    .step(&format!("Flashing {}", part.name))
                    .percent(pct));
            }
            
            info!("Flashing partition: {} ({} bytes)", part.name, part.data.len());
            fastboot.flash_partition(&part.name, &part.data, None)?;
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Flash Firmware").step("Rebooting...").percent(95.0));
        }
        
        let _ = fastboot.reboot(None);
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Flash Firmware").step("Complete").percent(100.0).complete());
        }
        
        Ok(())
    }
}
