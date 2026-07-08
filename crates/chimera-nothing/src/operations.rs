// chimera-nothing/src/operations.rs
// Nothing Phone 1/2/2a operations — NothingOS (near-stock Android)

use chimera_core::error::Result;
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::device::DeviceInfo;
use chimera_adb::client::AdbClient;
use chimera_adb::shell::AdbShell;
use chimera_fastboot::client::FastbootClient;
use log::warn;

pub struct NothingOperations<'a> {
    adb: &'a AdbClient,
    serial: &'a str,
}

impl<'a> NothingOperations<'a> {
    pub fn new(adb: &'a AdbClient, serial: &'a str) -> Self {
        Self { adb, serial }
    }

    fn shell(&self) -> AdbShell<'_> {
        AdbShell::new(self.adb, self.serial)
    }

    pub fn get_info(&self, progress: Option<&ProgressSender>) -> Result<DeviceInfo> {
        let sh = self.shell();
        let mut info = self.adb.get_device_info(self.serial)?;

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Reading Nothing OS properties...").percent(30.0));
        }

        if let Ok(m) = sh.get_prop("ro.product.model") { info.model = m; }
        if let Ok(v) = sh.get_prop("ro.build.version.release") { info.android_version = Some(v); }
        if let Ok(b) = sh.get_prop("ro.build.version.incremental") { info.build_number = Some(b); }
        
        // Nothing OS version
        let _nothing_ver = sh.get_prop("ro.nothing.device.software.version").ok();
        
        if let Ok(bl) = sh.get_prop("ro.boot.verifiedbootstate") {
            use chimera_core::device::BootloaderStatus;
            info.bootloader_status = Some(if bl == "orange" {
                BootloaderStatus::Unlocked
            } else {
                BootloaderStatus::Locked
            });
        }

        if let Ok(out) = sh.run("service call iphonesubinfo 1") {
            info.imei = extract_imei(&out);
        }
        if let Ok(out) = sh.run("service call iphonesubinfo 3") {
            info.imei2 = extract_imei(&out);
        }

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Complete").percent(100.0).complete());
        }
        Ok(info)
    }

    pub fn remove_frp(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("Clearing Nothing Phone FRP...").percent(30.0));
        }

        // Nothing Phone uses standard Android FRP
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/frp bs=512 count=2");
        let _ = sh.run_root("rm -f /data/system/users/0/accounts.db");
        let _ = sh.run_root("settings put secure frp_credential_handle null");
        let _ = sh.run_root("content delete --uri content://settings/secure --where \"name='frp_credential_handle'\"");

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("FRP cleared").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn unlock_bootloader(&self, fastboot: &mut FastbootClient, progress: Option<&ProgressSender>) -> Result<()> {
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("BL Unlock").step("Unlocking Nothing Phone bootloader...").percent(50.0));
        }
        warn!("Unlocking Nothing Phone bootloader — data will be erased. Glyph LEDs will show orange.");
        fastboot.command(&chimera_fastboot::protocol::FastbootCommand::OemCommand("flashing unlock".to_string()))?;
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("BL Unlock").step("Bootloader unlocked").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn remove_screenlock(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock").step("Removing...").percent(30.0));
        }
        let _ = sh.run_root("rm -f /data/system/locksettings.db*");
        let _ = sh.run_root("rm -f /data/system/gatekeeper.*");
        let _ = sh.run_root("settings put secure lockscreen.password_type 0");
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock").step("Done").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn factory_reset(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Factory Reset").step("Wiping Nothing Phone...").percent(40.0));
        }
        sh.run("am broadcast -a android.intent.action.MASTER_CLEAR")?;
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Factory Reset").step("Reset triggered").percent(100.0).complete());
        }
        Ok(())
    }
}

fn extract_imei(output: &str) -> Option<String> {
    for line in output.lines() {
        for part in line.split('\'') {
            let part = part.trim();
            if part.len() == 15 && part.chars().all(|c| c.is_ascii_digit()) {
                return Some(part.to_string());
            }
        }
    }
    None
}
