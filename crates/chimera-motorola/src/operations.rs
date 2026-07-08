use chimera_core::error::Result;
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::device::DeviceInfo;
use chimera_adb::client::AdbClient;
use chimera_adb::shell::AdbShell;
use chimera_fastboot::client::FastbootClient;

pub struct MotorolaOperations<'a> { adb: &'a AdbClient, serial: &'a str }
impl<'a> MotorolaOperations<'a> {
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
            let _ = tx.send(Progress::new("FRP Remove").step("FRP removed").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn factory_reset(&self, _progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        sh.run_root("am broadcast -a android.intent.action.MASTER_CLEAR --receiver-foreground -p android")?;
        Ok(())
    }

    pub fn repair_imei(&self, imei1: &str, _imei2: Option<&str>, progress: Option<&ProgressSender>) -> Result<()> {
        chimera_core::imei::validate_imei(imei1)?;
        let sh = self.shell();
        let imei_bytes = chimera_core::imei::imei_to_bytes(imei1);
        let hex_str = hex::encode(&imei_bytes);
        let _ = sh.run_root(&format!("echo '{}' | xxd -r -p > /dev/block/bootdevice/by-name/modemst1", hex_str));
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("IMEI Repair").step("IMEI written").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn unlock_bootloader_fastboot() -> Result<()> {
        let mut fb = FastbootClient::open_first()?;
        fb.unlock_bootloader()?;
        Ok(())
    }
}
