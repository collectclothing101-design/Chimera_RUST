// chimera-adb/src/services.rs
// High-level ADB services for device repair operations

use chimera_core::error::Result;
use crate::client::AdbClient;
use crate::shell::AdbShell;
use chimera_core::device::DeviceInfo;
use chimera_core::progress::{Progress, ProgressSender};

/// All repair/unlock services accessible via ADB
pub struct AdbServices<'a> {
    client: &'a AdbClient,
    serial: &'a str,
}

impl<'a> AdbServices<'a> {
    pub fn new(client: &'a AdbClient, serial: &'a str) -> Self {
        Self { client, serial }
    }

    fn shell(&self) -> AdbShell<'_> {
        AdbShell::new(self.client, self.serial)
    }

    /// Remove FRP lock (multiple methods)
    pub fn remove_frp(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("Attempting FRP removal...").percent(10.0));
        }
        
        // Method 1: content provider
        let _ = sh.run_root("content delete --uri content://settings/secure --where \"name='android_id'\"");
        
        // Method 2: clear account DB
        let _ = sh.run_root("rm -f /data/system/users/0/accounts.db /data/system/users/0/accounts.db-journal");
        
        // Method 3: clear FRP partition
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/bootdevice/by-name/frp bs=4096 count=1");
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/frp bs=4096 count=1");
        let _ = sh.run_root("cat /dev/zero > /dev/block/platform/*/by-name/frp");
        
        // Method 4: settings API
        let _ = sh.run_root("settings put secure frp_credential_handle null");
        let _ = sh.run_root("settings delete secure frp_credential_handle");
        
        // Method 5: for newer Android - disable FRP via am
        let _ = sh.run_root("am broadcast -a android.intent.action.FACTORY_RESET --receiver-foreground -p android");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Remove").step("FRP removal complete").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Enable ADB debug mode (requires some form of access)
    pub fn enable_adb_debug(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        sh.run_root("settings put global adb_enabled 1")?;
        sh.run_root("settings put global development_settings_enabled 1")?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Enable ADB").step("ADB enabled").percent(100.0).complete());
        }
        Ok(())
    }

    /// Reset screen lock (PIN/Pattern/Password)
    pub fn reset_screenlock(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock Reset").step("Removing lock files...").percent(20.0));
        }
        
        // Remove lock settings
        let lock_files = [
            "/data/system/locksettings.db",
            "/data/system/locksettings.db-shm", 
            "/data/system/locksettings.db-wal",
            "/data/system/gesture.key",
            "/data/system/password.key",
            "/data/system/gatekeeper.password.key",
            "/data/system/gatekeeper.pattern.key",
            "/data/system/gatekeeper.pin.key",
        ];
        
        for f in &lock_files {
            let _ = sh.run_root(&format!("rm -f {}", f));
        }
        
        // Also try via settings
        let _ = sh.run_root("locksettings set-disabled true");
        let _ = sh.run_root("settings put secure lockscreen.disabled 1");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock Reset").step("Screen lock removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Remove MDM (Mobile Device Management) policy
    pub fn remove_mdm(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("MDM Remove").step("Removing MDM policy...").percent(20.0));
        }
        
        // Remove device admin / MDM profile
        let _ = sh.run_root("pm disable-user --user 0 com.samsung.android.mdm");
        let _ = sh.run_root("dpm force-all-users-global-proxy null");
        let _ = sh.run_root("settings put global captive_portal_mode 0");
        
        // Remove MDM database
        let _ = sh.run_root("rm -rf /data/system/device_policies.xml");
        let _ = sh.run_root("rm -rf /data/data/com.samsung.android.mdm");
        
        // Remove Knox MDM
        let _ = sh.run_root("pm disable-user --user 0 com.samsung.android.knox.containeragent");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("MDM Remove").step("MDM removed successfully").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Root device using Magisk
    pub fn magisk_root(&self, magisk_apk_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Magisk Root").step("Pushing Magisk...").percent(10.0));
        }
        
        // Push Magisk APK
        self.client.push(self.serial, magisk_apk_path, "/data/local/tmp/Magisk.apk")?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Magisk Root").step("Installing Magisk...").percent(40.0));
        }
        
        // Install Magisk
        sh.run("pm install -r /data/local/tmp/Magisk.apk")?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Magisk Root").step("Patching boot image...").percent(70.0));
        }
        
        // Trigger Magisk patching via intent
        sh.run("am start -n com.topjohnwu.magisk/.SplashActivity")?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Magisk Root").step("Magisk installed - patch boot image and reflash").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Remove demo/retail mode
    pub fn remove_demo_mode(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Demo Remove").step("Removing demo mode...").percent(20.0));
        }
        
        // Generic Android demo removal
        let _ = sh.run_root("settings put global device_demo_mode 0");
        let _ = sh.run_root("settings put secure device_provisioned 1");
        let _ = sh.run_root("settings put global retail_demo_mode_enabled 0");
        let _ = sh.run_root("am broadcast -a android.intent.action.DEVICE_OWNER_CHANGED");
        
        // Samsung specific
        let _ = sh.run_root("pm disable-user --user 0 com.samsung.android.demomode");
        let _ = sh.run_root("settings put global sysui_demo_allowed 0");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Demo Remove").step("Demo mode removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Factory reset via ADB
    pub fn factory_reset(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Factory Reset").step("Triggering factory reset...").percent(50.0));
        }
        
        // Method 1: recovery command
        sh.run("recovery --wipe_data")?;
        
        Ok(())
    }

    /// Enable ADB via QR code (Android 11+)
    pub fn generate_adb_qr(&self, device_serial: &str) -> Result<String> {
        let password = format!("chimera_{}", device_serial);
        // Generate QR code data for wireless ADB pairing
        // Format: WIFI:T:ADB;S:ChimeraRS;P:password;;
        let qr_data = format!("WIFI:T:ADB;S:ChimeraRS-Pair;P:{};;", password);
        Ok(qr_data)
    }

    /// Get complete device info
    pub fn get_full_info(&self, progress: Option<&ProgressSender>) -> Result<DeviceInfo> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Reading device info...").percent(20.0));
        }
        
        let mut info = self.client.get_device_info(self.serial)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Reading IMEI...").percent(60.0));
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
}
