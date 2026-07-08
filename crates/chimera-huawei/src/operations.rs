// chimera-huawei/src/operations.rs
// Huawei device operations

use chimera_core::error::{ChimeraError, Result};
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::device::DeviceInfo;
use chimera_adb::client::AdbClient;
use chimera_adb::shell::AdbShell;
use base64::{Engine as _, engine::general_purpose};


pub struct HuaweiOperations<'a> {
    adb: &'a AdbClient,
    serial: &'a str,
}

impl<'a> HuaweiOperations<'a> {
    pub fn new(adb: &'a AdbClient, serial: &'a str) -> Self {
        Self { adb, serial }
    }

    fn shell(&self) -> AdbShell<'_> {
        AdbShell::new(self.adb, self.serial)
    }

    /// Get device info
    pub fn get_info(&self, _progress: Option<&ProgressSender>) -> Result<DeviceInfo> {
        let sh = self.shell();
        let mut info = self.adb.get_device_info(self.serial)?;
        
        // Huawei-specific properties
        if let Ok(emui) = sh.get_prop("ro.build.version.emui") {
            info.software_version = Some(emui);
        }
        if let Ok(region) = sh.get_prop("ro.config.regional_code") {
            info.region = Some(region);
        }
        
        let (imei1, imei2) = sh.get_imei().unwrap_or((None, None));
        info.imei = imei1;
        info.imei2 = imei2;
        
        Ok(info)
    }

    /// Remove FRP
    pub fn remove_frp(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("Removing Huawei FRP...").percent(20.0));
        }
        
        // Huawei FRP methods
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/frp bs=4096 count=1");
        let _ = sh.run_root("rm -rf /data/system/users/0/accounts.db");
        let _ = sh.run_root("settings put secure frp_credential_handle null");
        
        // Huawei-specific: disable HiCloud account
        let _ = sh.run_root("pm disable-user --user 0 com.huawei.hwid");
        let _ = sh.run_root("pm clear com.huawei.hwid");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("FRP removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Disable Huawei ID lock
    pub fn disable_huawei_id(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Disable Huawei ID").step("Removing Huawei ID lock...").percent(20.0));
        }
        
        // Disable HiCloud services
        let services = [
            "com.huawei.hwid",
            "com.huawei.wallet",
            "com.huawei.cloud",
            "com.huawei.hicloud",
        ];
        
        for svc in &services {
            let _ = sh.run_root(&format!("pm disable-user --user 0 {}", svc));
            let _ = sh.run_root(&format!("pm clear {}", svc));
        }
        
        // Clear Huawei ID data
        let _ = sh.run_root("rm -rf /data/data/com.huawei.hwid");
        let _ = sh.run_root("settings put global hicloud_enabled 0");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Disable Huawei ID").step("Huawei ID disabled").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Factory reset
    pub fn factory_reset(&self, _progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        sh.run_root("am broadcast -a android.intent.action.MASTER_CLEAR --receiver-foreground -p android")?;
        Ok(())
    }

    /// Repair IMEI
    pub fn repair_imei(&self, imei1: &str, imei2: Option<&str>, progress: Option<&ProgressSender>) -> Result<()> {
        chimera_core::imei::validate_imei(imei1)?;
        if let Some(imei2) = imei2 {
            chimera_core::imei::validate_imei(imei2)?;
        }
        
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("IMEI Repair").step("Writing IMEI...").percent(30.0));
        }
        
        // Huawei IMEI locations
        let imei_bytes = chimera_core::imei::imei_to_bytes(imei1);
        let imei_hex = hex::encode(&imei_bytes);
        
        // Write to Huawei NV store
        let _ = sh.run_root(&format!(
            "echo '{}' | xxd -r -p > /data/nvram/md/NVRAM/NVD_IMEI/01",
            imei_hex
        ));
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("IMEI Repair").step("IMEI written - reboot required").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Remove demo mode
    pub fn remove_demo_mode(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        let _ = sh.run_root("settings put global device_demo_mode 0");
        let _ = sh.run_root("pm disable-user --user 0 com.huawei.HwDemoMode 2>/dev/null || true");
        let _ = sh.run_root("rm -rf /data/data/com.huawei.HwDemoMode 2>/dev/null || true");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Demo Remove").step("Demo mode removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Store backup
    pub fn store_backup(&self, output_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Store Backup").step("Reading NVRAM...").percent(20.0));
        }
        
        // Read NVRAM (Huawei EFS equivalent)
        let nvram_b64 = sh.run_root("dd if=/dev/block/by-name/nvram bs=4096 | base64 -w 0")
            .or_else(|_| sh.run_root("tar -czf - /data/nvram | base64 -w 0"))?;
        
        let data = general_purpose::STANDARD.decode(nvram_b64.trim())
            .map_err(|e| ChimeraError::Adb(format!("Decode: {}", e)))?;
        
        let mut backup = chimera_core::backup::DeviceBackup::new("Huawei");
        backup.nvram_data = Some(data);
        backup.calculate_checksum();
        
        std::fs::write(output_path, backup.to_bytes()?)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Store Backup").step("Saved").percent(100.0).complete());
        }
        
        Ok(())
    }
}
