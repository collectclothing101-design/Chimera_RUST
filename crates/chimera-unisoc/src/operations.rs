// chimera-unisoc/src/operations.rs
use chimera_core::error::Result;
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::device::DeviceInfo;
use chimera_adb::client::AdbClient;
use chimera_adb::shell::AdbShell;

pub struct UnisocOperations<'a> {
    adb: &'a AdbClient,
    serial: &'a str,
}

impl<'a> UnisocOperations<'a> {
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
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/frp bs=4096 count=1");
        let _ = sh.run_root("rm -rf /data/system/users/0/accounts.db");
        let _ = sh.run_root("settings put secure frp_credential_handle null");
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("FRP removed").percent(100.0).complete());
        }
        Ok(())
    }

    pub fn update_firmware_pac(&self, pac_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        use crate::pac::PacFile;
        use crate::brom::UnisocBrom;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Update Unisoc").step("Parsing PAC file...").percent(5.0));
        }
        
        let pac_data = std::fs::read(pac_path)?;
        let pac = PacFile::parse(&pac_data)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Update Unisoc").step("Connecting BROM...").percent(10.0));
        }
        
        let mut brom = UnisocBrom::open()?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Update Unisoc").step("Flashing...").percent(50.0));
        }
        
        // Flash each PAC entry
        for entry in &pac.entries {
            // Flash entry data
            brom.write(&entry.data)?;
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Firmware Update Unisoc").step("Done").percent(100.0).complete());
        }
        
        Ok(())
    }
}
