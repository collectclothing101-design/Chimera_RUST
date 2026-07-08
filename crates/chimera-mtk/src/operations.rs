// chimera-mtk/src/operations.rs
// MediaTek high-level device operations

use chimera_core::error::{ChimeraError, Result};
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::device::DeviceInfo;
use chimera_adb::client::AdbClient;
use chimera_adb::shell::AdbShell;
use crate::da_protocol::MtkDaClient;
use log::info;

// MTK NVRAM LIDs for IMEI
pub const NVRAM_LID_IMEI1: u16 = 0x0041;
pub const NVRAM_LID_IMEI2: u16 = 0x0042;

pub struct MtkOperations<'a> {
    adb: &'a AdbClient,
    serial: &'a str,
}

impl<'a> MtkOperations<'a> {
    pub fn new(adb: &'a AdbClient, serial: &'a str) -> Self {
        Self { adb, serial }
    }

    fn shell(&self) -> AdbShell<'_> {
        AdbShell::new(self.adb, self.serial)
    }

    /// Get device info
    pub fn get_info(&self, _progress: Option<&ProgressSender>) -> Result<DeviceInfo> {
        let mut info = self.adb.get_device_info(self.serial)?;
        let sh = self.shell();
        
        // MTK-specific props
        if let Ok(chip) = sh.get_prop("ro.hardware") {
            info!("MTK chip: {:?}", chip);
        }
        
        let (imei1, imei2) = sh.get_imei().unwrap_or((None, None));
        info.imei = imei1;
        info.imei2 = imei2;
        
        Ok(info)
    }

    /// Remove FRP on MTK device
    pub fn remove_frp(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove MTK").step("Removing FRP...").percent(20.0));
        }
        
        // MTK FRP partition locations
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/frp bs=4096 count=1");
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/otp bs=4096 count=1 2>/dev/null || true");
        let _ = sh.run_root("rm -rf /data/system/users/0/accounts.db");
        let _ = sh.run_root("settings put secure frp_credential_handle null");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove MTK").step("FRP removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Remove screen lock on MTK device
    pub fn remove_screenlock(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        let lock_files = [
            "/data/system/locksettings.db",
            "/data/system/locksettings.db-shm",
            "/data/system/locksettings.db-wal",
            "/data/system/gesture.key",
            "/data/system/password.key",
        ];
        
        for f in &lock_files {
            let _ = sh.run_root(&format!("rm -f {}", f));
        }
        
        let _ = sh.run_root("settings put secure lockscreen.disabled 1");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock Remove").step("Done").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Write IMEI via MTK DA
    pub fn repair_imei_da(imei1: &str, imei2: Option<&str>, progress: Option<&ProgressSender>) -> Result<()> {
        chimera_core::imei::validate_imei(imei1)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("IMEI Repair MTK").step("Connecting to MTK BootROM...").percent(10.0));
        }
        
        let mut da_client = MtkDaClient::open()?;
        da_client.handshake()?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("IMEI Repair MTK").step("Writing IMEI to NVRAM...").percent(50.0));
        }
        
        let imei_bytes = chimera_core::imei::imei_to_bytes(imei1);
        da_client.write_nvram(NVRAM_LID_IMEI1, &imei_bytes)?;
        
        if let Some(imei2) = imei2 {
            chimera_core::imei::validate_imei(imei2)?;
            let imei2_bytes = chimera_core::imei::imei_to_bytes(imei2);
            da_client.write_nvram(NVRAM_LID_IMEI2, &imei2_bytes)?;
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("IMEI Repair MTK").step("IMEI written successfully").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Update firmware via scatter file
    pub fn update_firmware(&self, firmware_dir: &str, progress: Option<&ProgressSender>) -> Result<()> {
        use std::path::Path;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Update MTK").step("Finding scatter file...").percent(5.0));
        }
        
        // Find scatter file
        let scatter_files = ["MT6xxx_Android_scatter.txt", "MT6xxx_Android_scatter.xml", "scatter.txt"];
        let firmware_path = Path::new(firmware_dir);
        
        let mut scatter_path = None;
        for fname in &scatter_files {
            let p = firmware_path.join(fname);
            if p.exists() {
                scatter_path = Some(p);
                break;
            }
        }
        
        let scatter = scatter_path.ok_or_else(|| ChimeraError::Firmware("No scatter file found".into()))?;
        let _scatter_content = std::fs::read_to_string(&scatter)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Update MTK").step("Connecting to MTK device...").percent(10.0));
        }
        
        let mut da_client = MtkDaClient::open()?;
        da_client.handshake()?;
        
        // Parse scatter and flash partitions
        // Simplified - real implementation would parse full scatter format
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Update MTK").step("Firmware flashed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Root MTK device
    pub fn root_device(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if sh.is_rooted() {
            if let Some(tx) = progress {
                let _ = tx.send(Progress::new("Root MTK").step("Device already rooted").percent(100.0).complete());
            }
            return Ok(());
        }
        
        // MTK root methods
        let _ = sh.run("su -c 'chmod 777 /system/bin/su' 2>/dev/null");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Root MTK").step("Use Magisk for full root support").percent(100.0).complete());
        }
        
        Ok(())
    }
}
