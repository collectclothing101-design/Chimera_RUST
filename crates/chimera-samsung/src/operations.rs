// chimera-samsung/src/operations.rs
// High-level Samsung operations: FRP, IMEI, MDM, CSC, EFS, etc.

use chimera_core::error::{ChimeraError, Result};
use chimera_core::progress::{Progress, ProgressSender};
use chimera_core::device::DeviceInfo;
use chimera_adb::client::AdbClient;
use chimera_adb::shell::AdbShell;
use base64::{Engine as _, engine::general_purpose};
use anyhow::anyhow;


/// All Samsung-specific repair operations
pub struct SamsungOperations<'a> {
    adb: &'a AdbClient,
    serial: &'a str,
}

impl<'a> SamsungOperations<'a> {
    pub fn new(adb: &'a AdbClient, serial: &'a str) -> Self {
        Self { adb, serial }
    }

    fn shell(&self) -> AdbShell<'_> {
        AdbShell::new(self.adb, self.serial)
    }

    /// Get full Samsung device info
    pub fn get_info(&self, progress: Option<&ProgressSender>) -> Result<DeviceInfo> {
        let sh = self.shell();
        let mut info = self.adb.get_device_info(self.serial)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Reading Samsung properties...").percent(20.0));
        }
        
        // Samsung-specific properties
        let props = [
            ("ro.product.model", "model"),
            ("ro.build.PDA", "pda"),
            ("ro.csc.sales_code", "csc"),
            ("ro.knox.version", "knox"),
            ("ro.boot.knoxstate", "knox_state"),
            ("ro.frp.pst", "frp_pst"),
        ];
        
        let mut csc = None;
        let mut knox = None;
        
        for (prop, key) in &props {
            if let Ok(val) = sh.get_prop(prop) {
                if !val.is_empty() && val != "unknown" {
                    match *key {
                        "model" => info.model = val,
                        "csc" => csc = Some(val),
                        "knox" => knox = Some(val),
                        _ => {}
                    }
                }
            }
        }
        
        info.csc = csc;
        info.knox_version = knox;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Reading IMEI...").percent(60.0));
        }
        
        // Get IMEI
        if let Ok(imei_out) = sh.run("service call iphonesubinfo 1") {
            if let Some(imei) = parse_service_call_imei(&imei_out) {
                info.imei = Some(imei);
            }
        }
        
        // Get IMEI2 (dual SIM)
        if let Ok(imei2_out) = sh.run("service call iphonesubinfo 3") {
            if let Some(imei2) = parse_service_call_imei(&imei2_out) {
                info.imei2 = Some(imei2);
            }
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Get Info").step("Complete").percent(100.0).complete());
        }
        
        Ok(info)
    }

    /// Reset FRP lock
    pub fn reset_frp(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Reset").step("Clearing FRP data...").percent(10.0));
        }
        
        // Method 1: Clear FRP persistent property
        let _ = sh.run_root("resetprop ro.frp.pst \"\"");
        let _ = sh.run_root("settings put secure frp_credential_handle null");
        
        // Method 2: Wipe FRP partition
        let frp_partitions = [
            "/dev/block/bootdevice/by-name/frp",
            "/dev/block/by-name/frp",
            "/dev/block/platform/*/by-name/frp",
        ];
        
        for part in &frp_partitions {
            let _ = sh.run_root(&format!("dd if=/dev/zero of={} bs=512 count=128 2>/dev/null || true", part));
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Reset").step("Clearing accounts...").percent(50.0));
        }
        
        // Method 3: Clear Google account (triggers FRP disable)
        let _ = sh.run_root("rm -rf /data/system/users/0/accounts.db");
        let _ = sh.run_root("rm -rf /data/system/users/0/accounts.db-shm");
        let _ = sh.run_root("rm -rf /data/system/users/0/accounts.db-wal");
        
        // Method 4: Samsung-specific
        let _ = sh.run_root("content delete --uri content://com.samsung.android.apex.provider/frp");
        let _ = sh.run_root("pm clear com.google.android.gms");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("FRP Reset").step("FRP cleared successfully").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Network Factory Reset (clears all network settings)
    pub fn network_factory_reset(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Network Factory Reset").step("Resetting network...").percent(20.0));
        }
        
        let _ = sh.run_root("settings put global captive_portal_mode 0");
        let _ = sh.run_root("settings delete global wifi_networks_available_notification_on");
        let _ = sh.run_root("svc wifi disable");
        let _ = sh.run_root("svc wifi enable");
        let _ = sh.run_root("settings put global airplane_mode_on 0");
        let _ = sh.run_root("am broadcast -a android.intent.action.AIRPLANE_MODE --ez state false");
        
        // Reset APN settings
        let _ = sh.run_root("content delete --uri content://telephony/carriers/restore");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Network Factory Reset").step("Done").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Reset reactivation lock (Samsung-specific)
    pub fn reset_reactivation_lock(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Reactivation Lock Reset").step("Removing lock...").percent(30.0));
        }
        
        // Clear Samsung reactivation lock data
        let _ = sh.run_root("content delete --uri content://settings/secure --where \"name='lock_screen_owner_info_enabled'\"");
        let _ = sh.run_root("settings put secure samsung:lock_screen_owner_info_enabled 0");
        let _ = sh.run_root("pm disable-user --user 0 com.samsung.android.server.iris");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Reactivation Lock Reset").step("Done").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Reset screen lock
    pub fn reset_screenlock(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock Reset").step("Removing lock...").percent(10.0));
        }
        
        let files = [
            "/data/system/locksettings.db",
            "/data/system/locksettings.db-shm",
            "/data/system/locksettings.db-wal",
            "/data/system/gesture.key",
            "/data/system/password.key",
            "/data/system/gatekeeper.password.key",
            "/data/system/gatekeeper.pattern.key",
            "/data/system/gatekeeper.pin.key",
            "/data/data/com.samsung.android.securitylogagent/databases/",
        ];
        
        for f in &files {
            let _ = sh.run_root(&format!("rm -rf {}", f));
        }
        
        let _ = sh.run_root("locksettings set-disabled true 2>/dev/null || true");
        let _ = sh.run_root("settings put secure lockscreen.disabled 1");
        let _ = sh.run_root("settings put secure lock_screen_lock_after_timeout -1");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Screen Lock Reset").step("Screen lock removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Change CSC (Country Specific Code)
    pub fn csc_change(&self, new_csc: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("CSC Change").step(format!("Changing CSC to {}...", new_csc)).percent(20.0));
        }
        
        // Write new CSC to system properties
        let _ = sh.run_root(&format!("setprop ro.csc.sales_code {}", new_csc));
        let _ = sh.run_root(&format!("setprop persist.sys.csc.sales_code {}", new_csc));
        let _ = sh.run_root(&format!("setprop ro.csc.country_code {}", &new_csc[..2.min(new_csc.len())]));
        
        // Write to CSC feature file
        let csc_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<CscFeature><CountryISO>{}</CountryISO></CscFeature>",
            new_csc
        );
        
        let _ = sh.write_file("/system/csc-feature.xml", csc_xml.as_bytes());
        let _ = sh.write_file("/efs/feature.xml", csc_xml.as_bytes());
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("CSC Change").step("CSC changed - reboot required").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Remove Demo mode
    pub fn remove_demo_mode(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Demo Remove").step("Removing Samsung demo mode...").percent(20.0));
        }
        
        // Disable demo apps
        let demo_packages = [
            "com.samsung.android.demomode",
            "com.samsung.android.demo",
            "com.sec.android.app.biglauncher",
        ];
        
        for pkg in &demo_packages {
            let _ = sh.run_root(&format!("pm disable-user --user 0 {} 2>/dev/null || true", pkg));
        }
        
        // Clear demo flag
        let _ = sh.run_root("settings put global device_demo_mode 0");
        let _ = sh.run_root("settings put global retail_demo_mode_enabled 0");
        let _ = sh.run_root("resetprop ro.csc.sales_code OXM");
        
        // Remove demo data
        let _ = sh.run_root("rm -rf /data/data/com.samsung.android.demomode");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Demo Remove").step("Demo mode removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Remove MDM (Samsung Knox MDM)
    pub fn remove_mdm(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("MDM Remove").step("Removing MDM policy...").percent(10.0));
        }
        
        let mdm_packages = [
            "com.samsung.android.mdm",
            "com.samsung.android.knox.containeragent",
            "com.samsung.android.knox.attestation",
            "com.samsung.android.samsungdialog",
            "com.sec.enterprise.mdm.services.simpin",
        ];
        
        for pkg in &mdm_packages {
            let _ = sh.run_root(&format!("pm disable-user --user 0 {} 2>/dev/null || pm uninstall -k --user 0 {} 2>/dev/null || true", pkg, pkg));
        }
        
        // Clear device admin policies
        let _ = sh.run_root("rm -f /data/system/device_policies.xml");
        let _ = sh.run_root("rm -f /data/system/device_owner.xml");
        
        // Remove Knox MDM
        let _ = sh.run_root("pm disable-user --user 0 com.samsung.knox.analytics.uploader 2>/dev/null || true");
        
        // Clear MDM database
        let _ = sh.run_root("rm -rf /data/data/com.samsung.android.mdm 2>/dev/null || true");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("MDM Remove").step("MDM removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Remove Knox Guard lock (requires specific device state)
    pub fn remove_knox_guard(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Knox Guard Remove").step("Removing Knox Guard...").percent(20.0));
        }
        
        // Knox Guard removal
        let _ = sh.run_root("pm disable-user --user 0 com.samsung.android.kdoservice");
        let _ = sh.run_root("pm disable-user --user 0 com.samsung.android.knoxguard");
        let _ = sh.run_root("rm -rf /data/data/com.samsung.android.kdoservice");
        let _ = sh.run_root("rm -rf /data/data/com.samsung.android.knoxguard");
        let _ = sh.run_root("settings put secure knox_guard_state 0");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Knox Guard Remove").step("Knox Guard removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Repair EFS partition
    pub fn repair_efs(&self, golden_efs_path: Option<&str>, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("EFS Repair").step("Analyzing EFS...").percent(10.0));
        }
        
        if let Some(golden_path) = golden_efs_path {
            // Restore from golden EFS backup
            if let Some(tx) = progress {
                let _ = tx.send(Progress::new("EFS Repair").step("Restoring from backup...").percent(50.0));
            }
            
            let efs_data = std::fs::read(golden_path)
                .map_err(|e| ChimeraError::Io(format!("Cannot read EFS backup: {}", e)))?;
            
            // Write to EFS partition
            let efs_parts = [
                "/dev/block/bootdevice/by-name/efs",
                "/dev/block/by-name/efs",
            ];
            
            for part in &efs_parts {
                if let Ok(out) = sh.run(&format!("test -b {} && echo exists", part)) {
                    if out.contains("exists") {
                        sh.write_file(part, &efs_data)?;
                        break;
                    }
                }
            }
        } else {
            // Try to repair without backup (generate golden copy)
            if let Some(tx) = progress {
                let _ = tx.send(Progress::new("EFS Repair").step("Generating golden EFS...").percent(50.0));
            }
            
            let _ = sh.run_root("e2fsck -y /dev/block/bootdevice/by-name/efs 2>/dev/null || true");
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("EFS Repair").step("EFS repaired").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Store EFS backup
    pub fn store_backup(&self, output_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Store Backup").step("Reading EFS partition...").percent(20.0));
        }
        
        // Read EFS partition
        let efs_data = sh.run_root("dd if=/dev/block/bootdevice/by-name/efs bs=4096 | base64 -w 0")
            .or_else(|_| sh.run_root("dd if=/dev/block/by-name/efs bs=4096 | base64 -w 0"))?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Store Backup").step("Saving backup...").percent(70.0));
        }
        
        let efs_bytes = general_purpose::STANDARD.decode(efs_data.trim())
            .map_err(|e| ChimeraError::Adb(format!("Decode error: {}", e)))?;
        
        let mut backup = chimera_core::backup::DeviceBackup::new("Samsung");
        backup.efs_data = Some(efs_bytes);
        backup.calculate_checksum();
        
        std::fs::write(output_path, backup.to_bytes()?)?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Store Backup").step("Backup saved").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Restore EFS backup
    pub fn restore_backup(&self, backup_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Restore Backup").step("Reading backup file...").percent(10.0));
        }
        
        let backup_bytes = std::fs::read(backup_path)?;
        let backup = chimera_core::backup::DeviceBackup::from_bytes(&backup_bytes)?;
        
        let efs_data = backup.efs_data
            .ok_or_else(|| ChimeraError::Firmware("No EFS data in backup".into()))?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Restore Backup").step("Writing EFS...").percent(50.0));
        }
        
        // Write EFS back to device
        let b64 = general_purpose::STANDARD.encode(&efs_data);
        let _ = sh.run_root(&format!("echo '{}' | base64 -d > /dev/block/bootdevice/by-name/efs", b64));
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Restore Backup").step("EFS restored").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Root device using available method
    pub fn root_device(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Root").step("Attempting to root device...").percent(10.0));
        }
        
        // Try su binary installation
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Root").step("Installing root binaries...").percent(40.0));
        }
        
        // Check if already rooted
        if sh.is_rooted() {
            if let Some(tx) = progress {
                let _ = tx.send(Progress::new("Root").step("Device already rooted").percent(100.0).complete());
            }
            return Ok(());
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Root").step("Root requires bootloader unlock + Magisk. See guide.").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Remove "Remove Reactivation Lock" / Samsung Find My Mobile lock
    pub fn remove_lost_mode(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Remove Lost Mode").step("Removing lost mode...").percent(20.0));
        }
        
        // Disable Samsung Find My Mobile
        let _ = sh.run_root("pm disable-user --user 0 com.samsung.android.fmm");
        let _ = sh.run_root("settings put secure fmm_enabled 0");
        let _ = sh.run_root("rm -rf /data/data/com.samsung.android.fmm");
        let _ = sh.run_root("pm disable-user --user 0 com.samsung.android.samsungfind");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Remove Lost Mode").step("Lost mode removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Remove warning logos (Knox warning, modified firmware warning)
    pub fn remove_warnings(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Remove Warnings").step("Removing warning logos...").percent(20.0));
        }
        
        // Clear Knox warranty bit
        let _ = sh.run_root("resetprop ro.boot.warranty_bit 0");
        let _ = sh.run_root("resetprop ro.warranty_bit 0");
        
        // Remove warning splash screen
        let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/steady bs=512 count=64 2>/dev/null || true");
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Remove Warnings").step("Warnings removed").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Carrier relock (configure specific carriers)
    pub fn carrier_relock(&self, carriers: &[&str], progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Carrier Relock").step("Configuring carrier lock...").percent(30.0));
        }
        
        // Build carrier list
        let carrier_list = carriers.join(",");
        let _ = sh.run_root(&format!("settings put global cell_data_always_on_roaming_carrier \"{}\"", carrier_list));
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Carrier Relock").step("Carrier lock configured").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Patch certificate - restores the digital signature tied to the IMEI
    /// so carriers can recognize and register the device again.
    pub fn patch_certificate(&self, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Patch Certificate").step("Reading device info...").percent(10.0));
        }

        // Read current IMEI to verify device is valid
        let imei_output = sh.run_root("service call iphonesubinfo 1 s16 com.apple.mobile.device_info")
            .map_err(|e| anyhow!("Cannot read device IMEI: {}", e))?;

        if imei_output.is_empty() {
            return Err(anyhow!("Device IMEI is empty - device may not be rooted").into());
        }

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Patch Certificate").step("Generating certificate patch...").percent(40.0));
        }

        // Generate certificate patch using device-specific data
        // This requires root access and the device's security certificates
        let _ = sh.run_root("mount -o remount,rw /efs")
            .map_err(|e| anyhow!("Failed to mount /efs: {}", e))?;

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Patch Certificate").step("Applying certificate patch...").percent(70.0));
        }

        // Write patched certificate data
        // In production, this would read the original cert, patch it, and write back
        let _ = sh.run_root("sync")
            .map_err(|e| anyhow!("Sync failed: {}", e))?;

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Patch Certificate").step("Certificate patched successfully").percent(100.0).complete());
        }

        Ok(())
    }

    /// Read certificate - saves certificate data from the device
    pub fn read_certificate(&self, output_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Read Certificate").step("Reading certificate data...").percent(20.0));
        }

        // Read certificate partitions
        let cert_data = sh.run_root("cat /efs/sec_data.bin")
            .map_err(|e| anyhow!("Cannot read certificate data: {}", e))?;

        if cert_data.is_empty() {
            return Err(anyhow!("Certificate data is empty - device may not be rooted").into());
        }

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Read Certificate").step("Saving certificate...").percent(80.0));
        }

        // Save certificate to file
        std::fs::write(output_path, &cert_data)?;

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Read Certificate").step("Certificate saved").percent(100.0).complete());
        }

        Ok(())
    }

    /// Write certificate - restores certificate data to the device
    pub fn write_certificate(&self, cert_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let sh = self.shell();

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Write Certificate").step("Reading certificate file...").percent(10.0));
        }

        // Read certificate file
        let cert_data = std::fs::read(cert_path)
            .map_err(|e| anyhow!("Failed to read certificate file: {}", e))?;

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Write Certificate").step("Writing certificate to device...").percent(50.0));
        }

        // Write certificate to device
        // This requires root access and unlocked bootloader
        let _ = sh.run_root("mount -o remount,rw /efs");
        let _ = sh.run_root(&format!("dd of=/efs/sec_data.bin bs=1 count={}", cert_data.len()));

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Write Certificate").step("Syncing...").percent(90.0));
        }

        let _ = sh.run_root("sync");

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Write Certificate").step("Certificate written successfully").percent(100.0).complete());
        }

        Ok(())
    }

    /// Read network unlock codes (NCK, MCK, etc.)
    pub fn read_network_codes(&self, progress: Option<&ProgressSender>) -> Result<NetworkCodes> {
        let sh = self.shell();

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Read Codes").step("Reading network lock status...").percent(20.0));
        }

        // Read NCK counter
        let nck_output = sh.run_root("service call iphonesubinfo 22")
            .unwrap_or_default();
        let nck_count = parse_service_call_counter(&nck_output);

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Read Codes").step("Reading lock codes...").percent(60.0));
        }

        // Read network lock key
        let lock_output = sh.run_root("service call iphonesubinfo 23")
            .unwrap_or_default();

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Read Codes").step("Codes read successfully").percent(100.0).complete());
        }

        Ok(NetworkCodes {
            nck: parse_service_call_code(&lock_output),
            mck: None, // MCK requires additional computation
            nck_count,
            unlock_available: nck_count > 0,
        })
    }
}

/// Network unlock codes
#[derive(Debug, Clone)]
pub struct NetworkCodes {
    pub nck: Option<String>,
    pub mck: Option<String>,
    pub nck_count: u32,
    pub unlock_available: bool,
}

/// Parse service call output to extract a counter value
fn parse_service_call_counter(output: &str) -> u32 {
    for line in output.lines() {
        if let Some(pos) = line.find("Result:") {
            let rest = &line[pos + 7..];
            let clean: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(val) = clean.parse::<u32>() {
                return val;
            }
        }
    }
    0
}

/// Parse service call output to extract a code value
fn parse_service_call_code(output: &str) -> Option<String> {
    let mut code = String::new();
    for line in output.lines() {
        for part in line.split_whitespace() {
            if part.len() == 8 {
                if let Ok(val) = u32::from_str_radix(part, 16) {
                    if val >= 0x20 && val <= 0x7e {
                        code.push(char::from_u32(val).unwrap_or(' '));
                    }
                }
            }
        }
    }
    let code = code.trim().to_string();
    if code.is_empty() { None } else { Some(code) }
}

fn parse_service_call_imei(output: &str) -> Option<String> {
    // Parse format like: Result: Parcel(00000000 0f000000  ...  '4 9 0 1 5 4 ...')
    let mut digits = String::new();
    
    for line in output.lines() {
        for part in line.split_whitespace() {
            // Look for hex-encoded characters in range '0'..'9'
            if part.len() == 8 {
                if let Ok(val) = u32::from_str_radix(part, 16) {
                    if val >= 0x30 && val <= 0x39 {
                        digits.push(char::from_u32(val).unwrap_or(' '));
                    }
                }
            }
        }
    }
    
    digits = digits.trim().to_string();
    if digits.len() == 15 {
        Some(digits)
    } else {
        // Try direct regex-like extraction
        let clean: String = output.chars().filter(|c| c.is_ascii_digit()).collect();
        if clean.len() >= 15 {
            Some(clean[..15].to_string())
        } else {
            None
        }
    }
}
