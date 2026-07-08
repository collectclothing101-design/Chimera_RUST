// chimera-xiaomi/src/operations.rs
// High-level Xiaomi device operations

use chimera_core::error::{ChimeraError, Result};
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::device::DeviceInfo;
use chimera_adb::client::AdbClient;
use chimera_adb::shell::AdbShell;
use base64::{Engine as _, engine::general_purpose};


/// Xiaomi device operations
pub struct XiaomiOperations<'a> {
    adb: &'a AdbClient,
    serial: &'a str,
}

impl<'a> XiaomiOperations<'a> {
    pub fn new(adb: &'a AdbClient, serial: &'a str) -> Self {
        Self { adb, serial }
    }

    fn shell(&self) -> AdbShell<'_> {
        AdbShell::new(self.adb, self.serial)
    }

    /// Get device info (Normal mode only)
    pub fn get_info(&self, progress: Option<&ProgressSender>) -> Result<DeviceInfo> {
        let mut info = self.adb.get_device_info(self.serial)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Reading Xiaomi properties...").percent(20.0));
        }
        
        let sh = self.shell();
        
        // Xiaomi-specific properties
        if let Ok(miui_ver) = sh.get_prop("ro.miui.ui.version.name") {
            if !miui_ver.is_empty() {
                info.software_version = Some(miui_ver);
            }
        }
        
        if let Ok(region) = sh.get_prop("ro.miui.region") {
            info.region = Some(region);
        }
        
        // Get IMEI
        let (imei1, imei2) = sh.get_imei().unwrap_or((None, None));
        info.imei = imei1;
        info.imei2 = imei2;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Complete").percent(100.0).complete());
        }
        
        Ok(info)
    }

    /// Remove FRP (Normal mode via ADB)
    pub fn remove_frp_adb(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("Removing FRP (Xiaomi)...").percent(20.0));
        }
        
        // Xiaomi FRP removal
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/bootdevice/by-name/frp bs=4096 count=1");
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/frp bs=4096 count=1");
        let _ = sh.run_root("rm -rf /data/system/users/0/accounts.db");
        let _ = sh.run_root("settings put secure frp_credential_handle null");
        
        // MIUI-specific
        let _ = sh.run_root("pm clear com.google.android.gms");
        let _ = sh.run_root("content delete --uri content://settings/secure --where \"name='frp_credential_handle'\"");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("FRP removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Factory reset
    pub fn factory_reset(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Factory Reset").step("Triggering factory reset...").percent(50.0));
        }
        
        sh.run_root("am broadcast -a android.intent.action.MASTER_CLEAR --receiver-foreground -p android")?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Factory Reset").step("Factory reset initiated").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Network factory reset
    pub fn network_factory_reset(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Network Factory Reset").step("Resetting network...").percent(30.0));
        }
        
        let _ = sh.run_root("settings put global mobile_data 0");
        let _ = sh.run_root("settings put global mobile_data 1");
        let _ = sh.run_root("svc wifi disable");
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = sh.run_root("svc wifi enable");
        let _ = sh.run_root("settings delete global wifi_networks");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Network Factory Reset").step("Done").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Repair IMEI via ADB (MTK Xiaomi - patch method)
    pub fn repair_imei_patch(&self, imei1: &str, imei2: Option<&str>, progress: Option<&ProgressSender>) -> Result<()> {
        chimera_core::imei::validate_imei(imei1)?;
        if let Some(imei2) = imei2 {
            chimera_core::imei::validate_imei(imei2)?;
        }
        
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("IMEI Repair").step("Patching IMEI (MTK)...").percent(20.0));
        }
        
        // MTK Xiaomi IMEI patch
        let nvram_path = "/mnt/vendor/nvdata/md/NVRAM/NVD_IMEI/";
        
        // Write IMEI to NVRAM
        let imei_bytes = chimera_core::imei::imei_to_bytes(imei1);
        let imei_hex = hex::encode(&imei_bytes);
        
        let _ = sh.run_root(&format!("echo '{}' | xxd -r -p > {}01", imei_hex, nvram_path));
        
        if let Some(imei2) = imei2 {
            let imei2_bytes = chimera_core::imei::imei_to_bytes(imei2);
            let imei2_hex = hex::encode(&imei2_bytes);
            let _ = sh.run_root(&format!("echo '{}' | xxd -r -p > {}02", imei2_hex, nvram_path));
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("IMEI Repair").step("IMEI patched - reboot required").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Store EFS/modem backup
    pub fn store_backup(&self, output_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Store Backup").step("Reading modem data...").percent(20.0));
        }
        
        // Read NVDATA partition (Xiaomi EFS equivalent)
        let nvdata_b64 = sh.run_root("dd if=/dev/block/by-name/nvdata bs=4096 | base64 -w 0")
            .or_else(|_| sh.run_root("tar -czf - /mnt/vendor/nvdata | base64 -w 0"))?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Store Backup").step("Saving...").percent(70.0));
        }
        
        let data = general_purpose::STANDARD.decode(nvdata_b64.trim())
            .map_err(|e| ChimeraError::Adb(format!("Decode error: {}", e)))?;
        
        let mut backup = chimera_core::backup::DeviceBackup::new("Xiaomi");
        backup.nvram_data = Some(data);
        backup.calculate_checksum();
        
        std::fs::write(output_path, backup.to_bytes()?)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Store Backup").step("Backup saved").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Restore backup
    pub fn restore_backup(&self, backup_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Restore Backup").step("Reading backup...").percent(10.0));
        }
        
        let backup_bytes = std::fs::read(backup_path)?;
        let backup = chimera_core::backup::DeviceBackup::from_bytes(&backup_bytes)?;
        
        let nvdata = backup.nvram_data
            .ok_or_else(|| ChimeraError::Firmware("No NVDATA in backup".into()))?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Restore Backup").step("Writing NVDATA...").percent(60.0));
        }
        
        let b64 = general_purpose::STANDARD.encode(&nvdata);
        let _ = sh.run_root(&format!("echo '{}' | base64 -d > /dev/block/by-name/nvdata", b64));
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Restore Backup").step("Backup restored").percent(100.0).complete());
        }
        
        Ok(())
    }
}
