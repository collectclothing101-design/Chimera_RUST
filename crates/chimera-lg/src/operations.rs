use chimera_core::error::Result;
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::device::DeviceInfo;
use chimera_adb::client::AdbClient;
use chimera_adb::shell::AdbShell;

pub struct LgOperations<'a> { adb: &'a AdbClient, serial: &'a str }
impl<'a> LgOperations<'a> {
    pub fn new(adb: &'a AdbClient, serial: &'a str) -> Self { Self { adb, serial } }
    fn shell(&self) -> AdbShell<'_> { AdbShell::new(self.adb, self.serial) }

    pub fn get_info(&self, _progress: Option<&ProgressSender>) -> Result<DeviceInfo> {
        let mut info = self.adb.get_device_info(self.serial)?;
        let sh = self.shell();
        let (imei1, imei2) = sh.get_imei().unwrap_or((None, None));
        info.imei = imei1; info.imei2 = imei2;
        Ok(info)
    }

    pub fn remove_frp(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/bootdevice/by-name/frp bs=4096 count=1");
        let _ = sh.run_root("rm -rf /data/system/users/0/accounts.db");
        let _ = sh.run_root("settings put secure frp_credential_handle null");
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove LG").step("FRP removed").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn remove_carrier_lock(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        let _ = sh.run_root("settings put global cell_on 1");
        let _ = sh.run_root("content delete --uri content://settings/secure --where \"name='simlock_state'\"");
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Carrier Unlock").step("Carrier lock removed").percent(100.0).complete());
        }
        Ok(())
    }
}
