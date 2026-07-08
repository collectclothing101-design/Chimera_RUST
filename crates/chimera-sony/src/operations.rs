// chimera-sony/src/operations.rs
// Sony Xperia high-level operations

use chimera_core::error::Result;
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::device::DeviceInfo;
use chimera_adb::client::AdbClient;
use chimera_adb::shell::AdbShell;
use crate::ta_partition::{self, TaPartition, ta_units};
use log::info;

pub struct SonyOperations<'a> {
    adb: &'a AdbClient,
    serial: &'a str,
}

impl<'a> SonyOperations<'a> {
    pub fn new(adb: &'a AdbClient, serial: &'a str) -> Self {
        Self { adb, serial }
    }

    fn shell(&self) -> AdbShell<'_> {
        AdbShell::new(self.adb, self.serial)
    }

    /// Get comprehensive device info
    pub fn get_info(&self, progress: Option<&ProgressSender>) -> Result<DeviceInfo> {
        let sh = self.shell();
        let mut info = self.adb.get_device_info(self.serial)?;

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Reading Sony properties...").percent(20.0));
        }

        let props = [
            ("ro.product.name",    "product_name"),
            ("ro.product.device",  "device"),
            ("ro.build.version.release", "android"),
            ("ro.build.fingerprint", "fingerprint"),
            ("persist.sys.timezone", "timezone"),
        ];

        for (prop, _key) in &props {
            let _ = sh.get_prop(prop);
        }

        // Read IMEI
        if let Ok(imei) = sh.run("service call iphonesubinfo 1") {
            if let Some(i) = parse_service_call_imei(&imei) {
                info.imei = Some(i);
            }
        }

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Complete").percent(100.0).complete());
        }

        Ok(info)
    }

    /// Remove FRP lock on Sony device
    pub fn remove_frp(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("Clearing FRP...").percent(25.0));
        }

        // Sony stores FRP in /dev/block/by-name/frp or /persistent
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/frp bs=4096 count=1 2>/dev/null");
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/config bs=4096 count=1 2>/dev/null");
        let _ = sh.run_root("rm -rf /data/system/users/0/accounts.db");
        let _ = sh.run_root("settings put secure frp_credential_handle null");
        let _ = sh.run_root("content delete --uri content://settings/secure --where \"name='frp_credential_handle'\"");

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("FRP removed").percent(100.0).complete());
        }

        Ok(())
    }

    /// Remove screen lock
    pub fn remove_screenlock(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();

        let lock_files = [
            "/data/system/locksettings.db",
            "/data/system/locksettings.db-shm",
            "/data/system/locksettings.db-wal",
            "/data/system/gesture.key",
            "/data/system/password.key",
            "/data/system/gatekeeper.password.key",
            "/data/system/gatekeeper.pattern.key",
        ];

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock Remove").step("Removing lock files...").percent(30.0));
        }

        for f in &lock_files {
            let _ = sh.run_root(&format!("rm -f {}", f));
        }

        let _ = sh.run_root("settings put secure lockscreen.password_type 0");
        let _ = sh.run_root("settings put secure lock_screen_lock_after_timeout 0");

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock Remove").step("Complete. Reboot required.").percent(100.0).complete());
        }

        Ok(())
    }

    /// Backup TA partition to file
    pub fn backup_ta(&self, dest_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("TA Backup").step("Reading TA partition...").percent(30.0));
        }

        let ta = ta_partition::read_ta_via_adb(&sh)?;

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("TA Backup").step("Saving to file...").percent(70.0));
        }

        std::fs::write(dest_path, &ta.raw)?;
        info!("TA backup saved to: {}", dest_path);

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("TA Backup").step("Backup complete").percent(100.0).complete());
        }

        Ok(())
    }

    /// Restore TA partition from file (DANGEROUS — verify file first!)
    pub fn restore_ta(&self, src_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("TA Restore").step("Reading backup...").percent(20.0));
        }

        let raw = std::fs::read(src_path)?;
        let ta = TaPartition::parse(&raw)?;

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("TA Restore").step("Writing TA partition (DO NOT DISCONNECT!)...").percent(50.0));
        }

        ta_partition::write_ta_via_adb(&sh, &ta)?;

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("TA Restore").step("TA restored successfully").percent(100.0).complete());
        }

        Ok(())
    }

    /// Get TA partition info
    pub fn get_ta_info(&self, progress: Option<&ProgressSender>) -> Result<Vec<(String, String)>> {
        let sh = self.shell();

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("TA Info").step("Reading TA partition...").percent(50.0));
        }

        let ta = ta_partition::read_ta_via_adb(&sh)?;
        let mut info = Vec::new();

        info.push(("Bootloader Locked".to_string(),
            (!ta.is_bootloader_unlocked()).to_string()));
        info.push(("Unlock Counter".to_string(),
            ta.get_unlock_counter().to_string()));

        if let Some(e) = ta.find(ta_units::IMEI_PRIMARY) {
            info.push(("IMEI (TA)".to_string(), hex::encode(&e.data)));
        }
        if let Some(e) = ta.find(ta_units::WIFI_MAC) {
            info.push(("Wi-Fi MAC (TA)".to_string(), e.as_hex()));
        }
        if let Some(e) = ta.find(ta_units::PRODUCT_ID) {
            info.push(("Product ID".to_string(), e.as_string().unwrap_or_else(|| e.as_hex())));
        }

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("TA Info").step("Complete").percent(100.0).complete());
        }

        Ok(info)
    }

    /// Factory reset via ADB
    pub fn factory_reset(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Factory Reset").step("Wiping data...").percent(30.0));
        }

        sh.run_root("recovery --wipe_data")?;

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Factory Reset").step("Complete").percent(100.0).complete());
        }

        Ok(())
    }
}

fn parse_service_call_imei(output: &str) -> Option<String> {
    // Parse iphonesubinfo output: "Result: Parcel(...\n  '123456789012345'\n"
    let mut result = String::new();
    for line in output.lines() {
        if line.contains('\'') {
            for part in line.split('\'') {
                let part = part.trim();
                if part.len() == 15 && part.chars().all(|c| c.is_ascii_digit()) {
                    return Some(part.to_string());
                }
                // Filter partial hex strings into digits
                let digits: String = part.chars()
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                result.push_str(&digits);
            }
        }
    }
    if result.len() == 15 {
        Some(result)
    } else {
        None
    }
}
