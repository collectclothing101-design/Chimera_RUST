// chimera-adb/src/diagnostics.rs
// ADB-level diagnostics collector — gathers battery, storage, RAM, thermal, network info

use crate::client::AdbClient;
use crate::shell::AdbShell;
use chimera_core::diagnostics::*;
use chimera_core::error::Result;

pub struct AdbDiagnostics<'a> {
    adb: &'a AdbClient,
    serial: &'a str,
}

impl<'a> AdbDiagnostics<'a> {
    pub fn new(adb: &'a AdbClient, serial: &'a str) -> Self {
        Self { adb, serial }
    }

    fn shell(&self) -> AdbShell<'_> {
        AdbShell::new(self.adb, self.serial)
    }

    /// Collect battery information
    pub fn get_battery(&self) -> Result<BatteryInfo> {
        let sh = self.shell();
        let dump = sh.run("dumpsys battery")?;
        Ok(parse_battery_dump(&dump))
    }

    /// Collect RAM information
    pub fn get_ram(&self) -> Result<RamInfo> {
        let sh = self.shell();
        let meminfo = sh.run("cat /proc/meminfo")?;
        Ok(parse_meminfo(&meminfo))
    }

    /// Collect storage information
    pub fn get_storage(&self) -> Result<StorageInfo> {
        let sh = self.shell();
        let df_out = sh.run("df -k /data /sdcard /system 2>/dev/null")?;
        let partitions = parse_df(&df_out);

        // Get specific values for internal storage
        let data_part = partitions.iter().find(|p| p.mount_point == "/data");
        let sdcard_part = partitions.iter().find(|p| p.mount_point.contains("sdcard"));

        Ok(StorageInfo {
            total_internal_mb: data_part.map(|p| p.total_mb),
            used_internal_mb: data_part.map(|p| p.used_mb),
            free_internal_mb: data_part.map(|p| p.free_mb),
            total_sdcard_mb: sdcard_part.map(|p| p.total_mb),
            used_sdcard_mb: sdcard_part.map(|p| p.used_mb),
            partitions,
        })
    }

    /// Collect thermal information from sysfs
    pub fn get_thermal(&self) -> Result<ThermalInfo> {
        let sh = self.shell();
        let mut zones = Vec::new();

        // Try standard thermal zone paths
        for i in 0..10 {
            let temp_path = format!("/sys/class/thermal/thermal_zone{}/temp", i);
            let type_path = format!("/sys/class/thermal/thermal_zone{}/type", i);
            
            if let (Ok(temp_str), Ok(type_str)) = (sh.run(&format!("cat {}", temp_path)), sh.run(&format!("cat {}", type_path))) {
                let temp_str = temp_str.trim().to_string();
                let type_str = type_str.trim().to_string();
                if let Ok(temp_raw) = temp_str.parse::<i32>() {
                    zones.push(ThermalZone {
                        name: type_str,
                        temperature_c: temp_raw as f32 / 1000.0,
                    });
                }
            }
        }
        Ok(ThermalInfo { zones })
    }

    /// Collect network information
    pub fn get_network(&self) -> Result<NetworkInfo> {
        let sh = self.shell();

        let wifi_enabled = sh.get_prop("init.svc.wpa_supplicant")
            .map(|v| v == "running").unwrap_or(false);

        let wifi_ssid = sh.run("dumpsys wifi 2>/dev/null | grep 'SSID:' | head -1").ok()
            .and_then(|s| {
                s.split("SSID:").nth(1).map(|s| s.trim().trim_matches('"').to_string())
            });

        let airplane_mode = sh.run("settings get global airplane_mode_on").ok()
            .map(|v| v.trim() == "1").unwrap_or(false);

        let operator = sh.get_prop("gsm.operator.alpha").ok()
            .filter(|s| !s.is_empty() && s != "unknown");

        let mcc_mnc = sh.get_prop("gsm.operator.numeric").ok()
            .filter(|s| !s.is_empty() && s != "unknown");

        let network_type = sh.get_prop("telephony.lteOnCdmaDevice").ok()
            .map(|_| "LTE".to_string());

        Ok(NetworkInfo {
            wifi_enabled,
            wifi_ssid,
            mobile_data_enabled: !airplane_mode,
            airplane_mode,
            operator,
            mcc_mnc,
            signal_strength_dbm: None,  // Requires privileged dumpsys
            network_type,
        })
    }

    /// Collect CPU info
    pub fn get_cpu_info(&self) -> Result<(u32, u32)> {
        let sh = self.shell();
        let cores = sh.run("nproc").ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let freq_str = sh.run("cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq 2>/dev/null").ok()
            .unwrap_or_default();
        let freq_mhz = freq_str.trim().parse::<u32>().map(|f| f / 1000).unwrap_or(0);
        Ok((cores, freq_mhz))
    }

    /// Collect uptime
    pub fn get_uptime(&self) -> Result<u64> {
        let sh = self.shell();
        let uptime_str = sh.run("cat /proc/uptime").ok().unwrap_or_default();
        let secs = uptime_str.split_whitespace().next()
            .and_then(|s| s.parse::<f64>().ok())
            .map(|f| f as u64)
            .unwrap_or(0);
        Ok(secs)
    }

    /// Collect all diagnostics at once
    pub fn collect_all(&self) -> FullDiagnostics {
        let battery = self.get_battery().ok();
        let storage = self.get_storage().ok();
        let ram = self.get_ram().ok();
        let thermal = self.get_thermal().ok();
        let network = self.get_network().ok();
        let (cpu_cores, cpu_freq_mhz) = self.get_cpu_info().unwrap_or((0, 0));
        let uptime_seconds = self.get_uptime().ok();
        let sh = self.shell();
        let kernel_version = sh.run("uname -r").ok().map(|s| s.trim().to_string());

        FullDiagnostics {
            battery,
            storage,
            ram,
            thermal,
            network,
            cpu_cores: if cpu_cores > 0 { Some(cpu_cores) } else { None },
            cpu_freq_mhz: if cpu_freq_mhz > 0 { Some(cpu_freq_mhz) } else { None },
            uptime_seconds,
            kernel_version,
        }
    }
}
