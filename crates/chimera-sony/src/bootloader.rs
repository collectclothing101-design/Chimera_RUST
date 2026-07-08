// chimera-sony/src/bootloader.rs
// Sony bootloader unlock via official Sony Unlock Service (no account needed via fastboot OEM)

use chimera_core::error::Result;
use chimera_core::progress::{Progress, ProgressSender};
use chimera_fastboot::client::FastbootClient;
use log::{info, warn};

pub struct SonyBootloaderOps {
    fastboot: FastbootClient,
}

impl SonyBootloaderOps {
    pub fn new(fastboot: FastbootClient) -> Self {
        Self { fastboot }
    }

    /// Get unlock key (requires fastboot mode)
    pub fn get_unlock_key(&mut self) -> Result<String> {
        // Sony devices expose a unique identifier via getvar
        let key = self.fastboot.get_var("oem_unlockid")?;
        if key.is_empty() {
            // Fallback: try unlock_code
            let code = self.fastboot.get_var("unlock_code")
                .unwrap_or_else(|_| "not_available".to_string());
            Ok(code)
        } else {
            Ok(key)
        }
    }

    /// Check if bootloader is locked
    pub fn is_locked(&mut self) -> Result<bool> {
        let state = self.fastboot.get_var("unlocked")?;
        Ok(state.to_lowercase() != "yes" && state.to_lowercase() != "true" && state != "1")
    }

    /// Get device info (all fastboot vars)
    pub fn get_device_info(&mut self) -> Result<Vec<(String, String)>> {
        let vars = vec![
            "product", "version", "version-baseband", "version-bootloader",
            "serialno", "imei", "unlocked", "oem_unlockid",
            "secure", "current-slot", "slot-count",
            "has-slot:boot", "has-slot:system",
        ];
        
        let mut result = Vec::new();
        for var in &vars {
            if let Ok(val) = self.fastboot.get_var(var) {
                result.push((var.to_string(), val));
            }
        }
        Ok(result)
    }

    /// Unlock bootloader using unlock code from Sony DevWorld
    /// The user must visit https://developer.sony.com/open-source/aosp-on-xperia-open-devices/bootloader-unlock
    pub fn unlock_with_code(&mut self, unlock_code: &str, progress: Option<&ProgressSender>) -> Result<()> {
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Bootloader Unlock").step("Sending unlock command...").percent(40.0));
        }
        
        warn!("Unlocking bootloader — all user data will be erased!");
        self.fastboot.command(&chimera_fastboot::protocol::FastbootCommand::OemCommand(format!("unlock 0x{}", unlock_code)))?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Bootloader Unlock").step("Unlocked. Rebooting...").percent(90.0).complete());
        }
        
        let _ = self.fastboot.reboot(None);
        info!("Sony bootloader unlocked");
        Ok(())
    }

    /// Relock bootloader
    pub fn relock_bootloader(&mut self, progress: Option<&ProgressSender>) -> Result<()> {
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Bootloader Relock").step("Relocking...").percent(40.0));
        }
        
        self.fastboot.command(&chimera_fastboot::protocol::FastbootCommand::OemCommand("lock".to_string()))?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Bootloader Relock").step("Relocked. Rebooting...").percent(90.0).complete());
        }
        
        let _ = self.fastboot.reboot(None);
        info!("Sony bootloader relocked");
        Ok(())
    }
}
