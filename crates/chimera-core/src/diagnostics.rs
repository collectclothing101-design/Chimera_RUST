// chimera-core/src/diagnostics.rs
// Device diagnostics: battery health, storage, RAM, thermals, hardware tests

use serde::{Deserialize, Serialize};
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub level_percent: Option<u8>,
    pub health: Option<BatteryHealth>,
    pub voltage_mv: Option<u32>,
    pub temperature_c: Option<f32>,
    pub technology: Option<String>,
    pub cycle_count: Option<u32>,
    pub capacity_mah: Option<u32>,
    pub status: Option<ChargingStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BatteryHealth {
    Good,
    Overheat,
    Dead,
    OverVoltage,
    Failure,
    Cold,
    Unknown,
}

impl std::fmt::Display for BatteryHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Good       => write!(f, "Good"),
            Self::Overheat   => write!(f, "Overheat"),
            Self::Dead       => write!(f, "Dead"),
            Self::OverVoltage=> write!(f, "Over Voltage"),
            Self::Failure    => write!(f, "Failure"),
            Self::Cold       => write!(f, "Cold"),
            Self::Unknown    => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChargingStatus {
    Charging,
    Discharging,
    NotCharging,
    Full,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub total_internal_mb: Option<u64>,
    pub used_internal_mb: Option<u64>,
    pub free_internal_mb: Option<u64>,
    pub total_sdcard_mb: Option<u64>,
    pub used_sdcard_mb: Option<u64>,
    pub partitions: Vec<PartitionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionInfo {
    pub name: String,
    pub mount_point: String,
    pub filesystem: String,
    pub total_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RamInfo {
    pub total_mb: Option<u64>,
    pub available_mb: Option<u64>,
    pub used_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalInfo {
    pub zones: Vec<ThermalZone>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalZone {
    pub name: String,
    pub temperature_c: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub wifi_enabled: bool,
    pub wifi_ssid: Option<String>,
    pub mobile_data_enabled: bool,
    pub airplane_mode: bool,
    pub operator: Option<String>,
    pub mcc_mnc: Option<String>,
    pub signal_strength_dbm: Option<i32>,
    pub network_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullDiagnostics {
    pub battery: Option<BatteryInfo>,
    pub storage: Option<StorageInfo>,
    pub ram: Option<RamInfo>,
    pub thermal: Option<ThermalInfo>,
    pub network: Option<NetworkInfo>,
    pub cpu_cores: Option<u32>,
    pub cpu_freq_mhz: Option<u32>,
    pub uptime_seconds: Option<u64>,
    pub kernel_version: Option<String>,
}

/// Parse battery dump output from ADB
pub fn parse_battery_dump(dump: &str) -> BatteryInfo {
    let mut info = BatteryInfo {
        level_percent: None, health: None, voltage_mv: None,
        temperature_c: None, technology: None, cycle_count: None,
        capacity_mah: None, status: None,
    };
    for line in dump.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("level: ") {
            info.level_percent = val.trim().parse().ok();
        } else if let Some(val) = line.strip_prefix("voltage: ") {
            info.voltage_mv = val.trim().parse().ok();
        } else if let Some(val) = line.strip_prefix("temperature: ") {
            // Android reports in 1/10 degrees
            if let Ok(t) = val.trim().parse::<i32>() {
                info.temperature_c = Some(t as f32 / 10.0);
            }
        } else if let Some(val) = line.strip_prefix("technology: ") {
            info.technology = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("health: ") {
            info.health = Some(match val.trim() {
                "2" => BatteryHealth::Good,
                "3" => BatteryHealth::Overheat,
                "4" => BatteryHealth::Dead,
                "5" => BatteryHealth::OverVoltage,
                "6" => BatteryHealth::Failure,
                "7" => BatteryHealth::Cold,
                _ => BatteryHealth::Unknown,
            });
        } else if let Some(val) = line.strip_prefix("status: ") {
            info.status = Some(match val.trim() {
                "2" => ChargingStatus::Charging,
                "3" => ChargingStatus::Discharging,
                "4" => ChargingStatus::NotCharging,
                "5" => ChargingStatus::Full,
                _ => ChargingStatus::Unknown,
            });
        }
    }
    info
}

/// Parse /proc/meminfo output
pub fn parse_meminfo(meminfo: &str) -> RamInfo {
    let mut total = None;
    let mut available = None;
    for line in meminfo.lines() {
        if let Some(val) = line.strip_prefix("MemTotal:") {
            if let Ok(kb) = val.trim().trim_end_matches(" kB").parse::<u64>() {
                total = Some(kb / 1024);
            }
        } else if let Some(val) = line.strip_prefix("MemAvailable:") {
            if let Ok(kb) = val.trim().trim_end_matches(" kB").parse::<u64>() {
                available = Some(kb / 1024);
            }
        }
    }
    RamInfo {
        total_mb: total,
        available_mb: available,
        used_mb: total.zip(available).map(|(t, a)| t.saturating_sub(a)),
    }
}

/// Parse df output for storage
pub fn parse_df(df_output: &str) -> Vec<PartitionInfo> {
    let mut result = Vec::new();
    for line in df_output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 6 {
            let name = parts[0].to_string();
            let total_kb: u64 = parts[1].parse().unwrap_or(0);
            let used_kb: u64 = parts[2].parse().unwrap_or(0);
            let free_kb: u64 = parts[3].parse().unwrap_or(0);
            let mount = parts.last().unwrap_or(&"").to_string();
            result.push(PartitionInfo {
                name,
                mount_point: mount,
                filesystem: String::from("unknown"),
                total_mb: total_kb / 1024,
                used_mb: used_kb / 1024,
                free_mb: free_kb / 1024,
            });
        }
    }
    result
}
