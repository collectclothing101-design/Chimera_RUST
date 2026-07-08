// chimera-fastboot/src/flash.rs
// Firmware flashing helpers for various image formats

use chimera_core::error::{ChimeraError, Result};
use chimera_core::progress::{Progress, ProgressSender};
use crate::client::FastbootClient;
use std::path::Path;

/// Flash result for one partition
#[derive(Debug)]
pub struct FlashResult {
    pub partition: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Flash a complete firmware package
pub struct FirmwareFlasher<'a> {
    client: &'a mut FastbootClient,
}

impl<'a> FirmwareFlasher<'a> {
    pub fn new(client: &'a mut FastbootClient) -> Self {
        Self { client }
    }

    /// Flash from a directory containing partition images
    pub fn flash_directory(&mut self, dir: &str, progress: Option<&ProgressSender>) -> Result<Vec<FlashResult>> {
        let path = Path::new(dir);
        let mut results = Vec::new();
        
        // Common partition files to look for
        let partition_map = [
            ("boot.img", "boot"),
            ("recovery.img", "recovery"),
            ("system.img", "system"),
            ("vendor.img", "vendor"),
            ("userdata.img", "userdata"),
            ("cache.img", "cache"),
            ("super.img", "super"),
            ("vbmeta.img", "vbmeta"),
        ];
        
        let total = partition_map.len() as f32;
        for (i, (filename, partition)) in partition_map.iter().enumerate() {
            let file_path = path.join(filename);
            if !file_path.exists() {
                continue;
            }
            
            if let Some(tx) = progress {
                let pct = (i as f32 / total) * 100.0;
                let _ = tx.send(Progress::new("Firmware Flash").step(format!("Flashing {}...", partition)).percent(pct));
            }
            
            let data = match std::fs::read(&file_path) {
                Ok(d) => d,
                Err(e) => {
                    results.push(FlashResult {
                        partition: partition.to_string(),
                        success: false,
                        error: Some(e.to_string()),
                    });
                    continue;
                }
            };
            
            match self.client.flash_partition(partition, &data, progress) {
                Ok(_) => {
                    results.push(FlashResult {
                        partition: partition.to_string(),
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    results.push(FlashResult {
                        partition: partition.to_string(),
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }
        
        Ok(results)
    }

    /// Flash a single image file to a partition
    pub fn flash_image(&mut self, image_path: &str, partition: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let data = std::fs::read(image_path)
            .map_err(|e| ChimeraError::Io(format!("Cannot read {}: {}", image_path, e)))?;
        
        self.client.flash_partition(partition, &data, progress)?;
        Ok(())
    }
}
