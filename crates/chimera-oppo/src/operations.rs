// chimera-oppo/src/operations.rs
// OPPO / Realme / OnePlus unified operations

use chimera_core::error::Result;
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::device::DeviceInfo;
use chimera_adb::client::AdbClient;
use chimera_adb::shell::AdbShell;
use chimera_fastboot::client::FastbootClient;
use crate::coloros;
use log::warn;

pub struct OppoOperations<'a> {
    adb: &'a AdbClient,
    serial: &'a str,
}

impl<'a> OppoOperations<'a> {
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
            let _ = tx.send(Progress::new("Get Info").step("Reading ColorOS properties...").percent(20.0));
        }

        if let Ok(m) = sh.get_prop("ro.product.oplusmodel") {
            if !m.is_empty() { info.model = m; }
        } else if let Ok(m) = sh.get_prop("ro.product.model") {
            info.model = m;
        }

        if let Ok(v) = sh.get_prop("ro.build.version.release") {
            info.android_version = Some(v);
        }

        // Get IMEI
        if let Ok(out) = sh.run("service call iphonesubinfo 1") {
            info.imei = parse_imei(&out);
        }
        if let Ok(out) = sh.run("service call iphonesubinfo 3") {
            info.imei2 = parse_imei(&out);
        }

        // ColorOS/OxygenOS version
        let coloros_ver = coloros::get_coloros_version(&sh);
        let _ = coloros_ver; // stored in extended fields if needed

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Complete").percent(100.0).complete());
        }
        Ok(info)
    }

    pub fn remove_frp(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("Clearing OPPO/Realme FRP...").percent(20.0));
        }

        // OPPO/Realme FRP locations
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/frp bs=4096 count=1");
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/config bs=512 count=4 2>/dev/null");
        let _ = sh.run_root("rm -rf /data/system/users/0/accounts.db");
        let _ = sh.run_root("settings put secure frp_credential_handle null");
        // OPPO-specific: clear oplus frp
        let _ = sh.run_root("setprop persist.vendor.oplus.frp 0");
        let _ = sh.run_root("content delete --uri content://settings/secure --where \"name='frp_credential_handle'\"");

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("FRP removed").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn remove_screenlock(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock").step("Removing...").percent(30.0));
        }

        let _ = sh.run_root("rm -f /data/system/locksettings.db*");
        let _ = sh.run_root("rm -f /data/system/gesture.key /data/system/password.key");
        let _ = sh.run_root("rm -f /data/system/gatekeeper.*");
        let _ = sh.run_root("settings put secure lockscreen.password_type 0");

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock").step("Complete").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn factory_reset(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Factory Reset").step("Wiping...").percent(40.0));
        }
        sh.run("am broadcast -a android.intent.action.FACTORY_RESET --include-stopped-packages")?;
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Factory Reset").step("Reset triggered").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn unlock_bootloader(&self, fastboot: &mut FastbootClient, progress: Option<&ProgressSender>) -> Result<()> {
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("BL Unlock").step("Unlocking via fastboot oem unlock...").percent(40.0));
        }
        warn!("Unlocking OPPO/Realme/OnePlus bootloader — data will be erased!");
        // OnePlus
        let _ = fastboot.command(&chimera_fastboot::protocol::FastbootCommand::OemCommand("unlock".to_string()));
        // Standard AOSP
        let _ = fastboot.command(&chimera_fastboot::protocol::FastbootCommand::OemCommand("flashing unlock".to_string()));
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("BL Unlock").step("Done").percent(100.0).complete());
        }
        Ok(())
    }

    /// Disable OnePlus/Realme anti-rollback (requires root)
    pub fn disable_anti_rollback(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Anti-Rollback").step("Disabling...").percent(50.0));
        }
        coloros::disable_anti_rollback(&sh)?;
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Anti-Rollback").step("Done").percent(100.0).complete());
        }
        Ok(())
    }
}

fn parse_imei(output: &str) -> Option<String> {
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
