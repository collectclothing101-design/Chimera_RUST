// chimera-nokia/src/operations.rs
// Nokia HMD Global operations — Android One devices (clean AOSP, easy FRP/bootloader)

use chimera_core::error::Result;
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::device::DeviceInfo;
use chimera_adb::client::AdbClient;
use chimera_adb::shell::AdbShell;
use chimera_fastboot::client::FastbootClient;
use log::warn;

pub struct NokiaOperations<'a> {
    adb: &'a AdbClient,
    serial: &'a str,
}

impl<'a> NokiaOperations<'a> {
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
            let _ = tx.send(Progress::new("Get Info").step("Reading Nokia properties...").percent(30.0));
        }

        // Nokia Android One — pure AOSP props
        if let Ok(model) = sh.get_prop("ro.product.model") { info.model = model; }
        if let Ok(build) = sh.get_prop("ro.build.version.release") {
            info.android_version = Some(build);
        }
        if let Ok(build_id) = sh.get_prop("ro.build.id") {
            info.build_number = Some(build_id);
        }
        if let Ok(bl) = sh.get_prop("ro.boot.verifiedbootstate") {
            use chimera_core::device::BootloaderStatus;
            info.bootloader_status = Some(match bl.as_str() {
                "green" | "self_signed" => BootloaderStatus::Locked,
                "orange" => BootloaderStatus::Unlocked,
                _ => BootloaderStatus::Unknown,
            });
        }

        // IMEI via standard service call
        if let Ok(imei_out) = sh.run("service call iphonesubinfo 1") {
            info.imei = extract_imei_from_service_call(&imei_out);
        }
        if let Ok(imei2_out) = sh.run("service call iphonesubinfo 3") {
            info.imei2 = extract_imei_from_service_call(&imei2_out);
        }

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Complete").percent(100.0).complete());
        }
        Ok(info)
    }

    pub fn remove_frp(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("Clearing FRP...").percent(30.0));
        }

        // Nokia Android One uses standard Android FRP location
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/frp bs=512 count=2");
        let _ = sh.run_root("rm -f /data/system/users/0/accounts.db");
        let _ = sh.run_root("settings put secure frp_credential_handle null");
        let _ = sh.run_root("wipe frp");  // Android 13+ OEM command

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("FRP cleared").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn remove_screenlock(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock Remove").step("Removing...").percent(30.0));
        }

        let _ = sh.run_root("rm -f /data/system/locksettings.db /data/system/locksettings.db-*");
        let _ = sh.run_root("rm -f /data/system/gesture.key /data/system/password.key");
        let _ = sh.run_root("rm -f /data/system/gatekeeper.password.key /data/system/gatekeeper.pattern.key");
        let _ = sh.run_root("settings put secure lockscreen.password_type 0");

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock Remove").step("Done. Reboot to apply.").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn factory_reset(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Factory Reset").step("Triggering wipe...").percent(50.0));
        }
        // Nokia supports standard Android recovery wipe
        sh.run("am broadcast -a android.intent.action.MASTER_CLEAR")?;
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Factory Reset").step("Reset triggered").percent(100.0).complete());
        }
        Ok(())
    }

    /// Unlock bootloader (Nokia Android One — standard fastboot oem unlock)
    pub fn unlock_bootloader_fastboot(&self, fastboot: &mut FastbootClient, progress: Option<&ProgressSender>) -> Result<()> {
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("BL Unlock").step("Sending oem unlock...").percent(50.0));
        }
        warn!("Unlocking bootloader — all data will be erased!");
        fastboot.command(&chimera_fastboot::protocol::FastbootCommand::OemCommand("unlock".to_string()))?;
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("BL Unlock").step("Unlocked").percent(100.0).complete());
        }
        Ok(())
    }
}

fn extract_imei_from_service_call(output: &str) -> Option<String> {
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
