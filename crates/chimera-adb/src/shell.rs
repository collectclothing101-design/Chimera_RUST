// chimera-adb/src/shell.rs
// ADB Shell command helpers for device operations

use chimera_core::error::{ChimeraError, Result};
use crate::client::AdbClient;
use base64::{Engine as _, engine::general_purpose};

/// High-level shell operations on an ADB device
pub struct AdbShell<'a> {
    client: &'a AdbClient,
    serial: &'a str,
}

impl<'a> AdbShell<'a> {
    pub fn new(client: &'a AdbClient, serial: &'a str) -> Self {
        Self { client, serial }
    }

    pub fn run(&self, cmd: &str) -> Result<String> {
        self.client.shell(self.serial, cmd)
    }

    pub fn run_root(&self, cmd: &str) -> Result<String> {
        self.run(&format!("su -c '{}'", cmd))
    }

    pub fn get_prop(&self, prop: &str) -> Result<String> {
        self.client.get_prop(self.serial, prop)
    }

    pub fn set_prop(&self, prop: &str, value: &str) -> Result<()> {
        self.client.set_prop(self.serial, prop, value)
    }

    /// Check if device is rooted
    pub fn is_rooted(&self) -> bool {
        self.run("su -c id 2>&1")
            .map(|output| output.contains("uid=0"))
            .unwrap_or(false)
    }

    /// Read file from device filesystem
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let b64 = self.run(&format!("base64 -w 0 {} 2>/dev/null || cat {} | base64", path, path))?;
        general_purpose::STANDARD.decode(b64.trim()).map_err(|e| ChimeraError::Adb(format!("Base64 decode: {}", e)))
    }

    /// Write file to device filesystem (requires root)
    pub fn write_file(&self, path: &str, data: &[u8]) -> Result<()> {
        let b64 = general_purpose::STANDARD.encode(data);
        self.run_root(&format!("echo '{}' | base64 -d > {}", b64, path))?;
        Ok(())
    }

    /// List directory
    pub fn ls(&self, path: &str) -> Result<Vec<String>> {
        let output = self.run(&format!("ls -la {}", path))?;
        Ok(output.lines().map(|l| l.to_string()).collect())
    }

    /// Mount a partition as read-write
    pub fn remount_rw(&self, mount_point: &str) -> Result<()> {
        self.run_root(&format!("mount -o remount,rw {}", mount_point))?;
        Ok(())
    }

    /// Factory reset via shell
    pub fn factory_reset(&self) -> Result<()> {
        self.run("am broadcast -a android.intent.action.FACTORY_RESET --receiver-foreground -p android")?;
        Ok(())
    }

    /// Wipe userdata
    pub fn wipe_data(&self) -> Result<()> {
        self.run_root("wipe data")?;
        Ok(())
    }

    /// Remove FRP via shell (works on older Android)
    pub fn remove_frp_shell(&self) -> Result<()> {
        // Try multiple methods
        let methods = [
            "content delete --uri content://settings/secure --where \"name='android_id'\"",
            "rm -rf /data/system/users/0/accounts.db",
            "settings put secure frp_credential_handle null",
        ];
        
        for method in &methods {
            let _ = self.run_root(method);
        }
        Ok(())
    }

    /// Enable ADB in developer options
    pub fn enable_adb(&self) -> Result<()> {
        self.run_root("settings put global adb_enabled 1")?;
        self.run_root("settings put global development_settings_enabled 1")?;
        Ok(())
    }

    /// Get all device properties as map
    pub fn get_all_props(&self) -> Result<std::collections::HashMap<String, String>> {
        let output = self.run("getprop")?;
        let mut map = std::collections::HashMap::new();
        for line in output.lines() {
            if let Some((key, val)) = parse_prop_line(line) {
                map.insert(key, val);
            }
        }
        Ok(map)
    }

    /// Get IMEI numbers
    pub fn get_imei(&self) -> Result<(Option<String>, Option<String>)> {
        // Method 1: service call
        if let Ok(out) = self.run("service call iphonesubinfo 1 | grep -oP '(?<=\")\\d+'") {
            let imei = out.trim().to_string();
            if imei.len() == 15 {
                let imei2 = self.run("service call iphonesubinfo 3 | grep -oP '(?<=\")\\d+'").ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| s.len() == 15);
                return Ok((Some(imei), imei2));
            }
        }
        
        // Method 2: dumpsys
        if let Ok(out) = self.run("dumpsys iphonesubinfo | grep IMEI") {
            for line in out.lines() {
                if let Some(imei_str) = line.split("IMEI =").nth(1) {
                    let imei = imei_str.trim().to_string();
                    if imei.len() == 15 {
                        return Ok((Some(imei), None));
                    }
                }
            }
        }
        
        Ok((None, None))
    }

    /// Get screen resolution
    pub fn get_screen_res(&self) -> Result<(u32, u32)> {
        let out = self.run("wm size")?;
        if let Some(size_str) = out.split("Physical size:").nth(1) {
            let size_str = size_str.trim();
            let parts: Vec<&str> = size_str.split('x').collect();
            if parts.len() == 2 {
                let w = parts[0].trim().parse().unwrap_or(0);
                let h = parts[1].trim().parse().unwrap_or(0);
                return Ok((w, h));
            }
        }
        Ok((0, 0))
    }
}

fn parse_prop_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if line.starts_with('[') {
        let key_end = line.find(']')?;
        let key = line[1..key_end].to_string();
        let rest = &line[key_end + 1..];
        let _val_start = rest.find('[')? + rest.find('[')? + 2;
        if let Some(val_section) = rest.get(1..) {
            if let Some(inner) = val_section.strip_prefix(": [") {
                let val = inner.trim_end_matches(']').to_string();
                return Some((key, val));
            }
        }
        // Simpler parse
        if let Some(colon_pos) = rest.find(": [") {
            let val = rest[colon_pos + 3..].trim_end_matches(']').to_string();
            return Some((key, val));
        }
    }
    None
}
