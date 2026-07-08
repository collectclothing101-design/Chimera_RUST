// chimera-core/src/device.rs
use serde::{Deserialize, Serialize};
use std::fmt;

/// All supported device brands
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceBrand {
    Apple,
    Google,
    Samsung,
    Xiaomi,
    Redmi,
    Poco,
    Huawei,
    Honor,
    Oppo,
    Realme,
    Vivo,
    OnePlus,
    Motorola,
    LG,
    HTC,
    Sony,
    Nokia,
    Alcatel,
    TCL,
    Wiko,
    Lenovo,
    Asus,
    ZTE,
    Nubia,
    Infinix,
    Tecno,
    Itel,
    Blackberry,
    Blackview,
    Lumia,  // Windows Phone / Nokia Lumia
    Meizu,
    BLU,
    Hisense,
    Ulefone,
    Doogee,
    Blaupunkt,
    Cricket,
    DeutscheTelekom,
    Nothing,
    Fairphone,
    Generic,  // Generic Android
    Unknown,
}

impl fmt::Display for DeviceBrand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Chipset family
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceChipset {
    Qualcomm,
    MediaTek,
    Exynos,
    Kirin,   // HiSilicon (Huawei)
    Unisoc,  // Spreadtrum SPD
    Helio,   // MTK sub-brand
    Dimensity, // MTK 5G
    Snapdragon, // Qualcomm sub-brand
    Unknown,
}

/// How the device is connected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConnectionMode {
    /// Normal ADB mode (USB debugging enabled)
    Adb,
    /// Android Fastboot mode
    Fastboot,
    /// Samsung Download/ODIN mode
    DownloadOdin,
    /// Qualcomm Emergency Download (EDL/Sahara)
    Edl,
    /// Huawei Factory Fastboot
    HuaweiFastboot,
    /// Xiaomi Mi-Assistant/Sideload mode
    MiAssistant,
    /// MediaTek BootRom mode
    MtkBootRom,
    /// Unisoc/SPD mode  
    UnisocBrom,
    /// Samsung Exynos USB Booting (EUB)
    SamsungEub,
    /// Serial/COM port
    Serial,
    /// ADB over TCP/IP (WiFi)
    AdbTcp,
    // ─── Apple iOS device modes ─────────────────────────────────
    /// Apple normal mode (iPhone/iPad over usbmuxd via lockdownd)
    AppleUsbMux,
    /// Apple DFU (Device Firmware Update) mode — checkm8 / restore entry
    AppleDfu,
    /// Apple Recovery mode (iBoot console)
    AppleRecovery,
    /// Apple PongoOS shell (post-checkm8 second-stage)
    ApplePongoOs,
    /// Apple Restore mode (during firmware install)
    AppleRestore,
    /// Apple WTF (DFU-like, only on iPhone OG / iPod 1G)
    AppleWtf,
    Unknown,
}

impl fmt::Display for ConnectionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionMode::Adb => write!(f, "ADB"),
            ConnectionMode::Fastboot => write!(f, "Fastboot"),
            ConnectionMode::DownloadOdin => write!(f, "Download (ODIN)"),
            ConnectionMode::Edl => write!(f, "EDL"),
            ConnectionMode::HuaweiFastboot => write!(f, "Huawei Fastboot"),
            ConnectionMode::MiAssistant => write!(f, "Mi-Assistant"),
            ConnectionMode::MtkBootRom => write!(f, "MTK BootRom"),
            ConnectionMode::UnisocBrom => write!(f, "Unisoc BROM"),
            ConnectionMode::SamsungEub => write!(f, "Samsung EUB"),
            ConnectionMode::Serial => write!(f, "Serial"),
            ConnectionMode::AdbTcp => write!(f, "ADB TCP"),
            ConnectionMode::AppleUsbMux => write!(f, "iOS (usbmuxd)"),
            ConnectionMode::AppleDfu => write!(f, "Apple DFU"),
            ConnectionMode::AppleRecovery => write!(f, "Apple Recovery"),
            ConnectionMode::ApplePongoOs => write!(f, "PongoOS"),
            ConnectionMode::AppleRestore => write!(f, "Apple Restore"),
            ConnectionMode::AppleWtf => write!(f, "Apple WTF"),
            ConnectionMode::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Device connection/operational state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceState {
    Disconnected,
    Connecting,
    Connected,
    Authorized,
    Unauthorized,
    Offline,
    Recovery,
    Sideload,
    Bootloader,
}

/// Full device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub serial: Option<String>,
    pub brand: DeviceBrand,
    pub model: String,
    pub model_code: Option<String>,
    pub chipset: DeviceChipset,
    pub connection_mode: ConnectionMode,
    pub state: DeviceState,
    pub imei: Option<String>,
    pub imei2: Option<String>,
    pub android_version: Option<String>,
    pub build_number: Option<String>,
    pub software_version: Option<String>,
    pub baseband_version: Option<String>,
    pub security_patch: Option<String>,
    pub bootloader_status: Option<BootloaderStatus>,
    pub frp_enabled: Option<bool>,
    pub root_status: Option<bool>,
    pub usb_vid: Option<u16>,
    pub usb_pid: Option<u16>,
    pub csc: Option<String>,
    pub region: Option<String>,
    pub carrier: Option<String>,
    pub knox_version: Option<String>,
    pub drk_status: Option<bool>,
    pub efs_status: Option<EfsStatus>,
    pub mac_address: Option<String>,
    pub wifi_mac: Option<String>,
    pub bt_mac: Option<String>,
}

impl DeviceInfo {
    pub fn new_unknown(serial: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            serial: Some(serial.into()),
            brand: DeviceBrand::Unknown,
            model: String::from("Unknown"),
            model_code: None,
            chipset: DeviceChipset::Unknown,
            connection_mode: ConnectionMode::Unknown,
            state: DeviceState::Connected,
            imei: None,
            imei2: None,
            android_version: None,
            build_number: None,
            software_version: None,
            baseband_version: None,
            security_patch: None,
            bootloader_status: None,
            frp_enabled: None,
            root_status: None,
            usb_vid: None,
            usb_pid: None,
            csc: None,
            region: None,
            carrier: None,
            knox_version: None,
            drk_status: None,
            efs_status: None,
            mac_address: None,
            wifi_mac: None,
            bt_mac: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootloaderStatus {
    Locked,
    Unlocked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EfsStatus {
    Ok,
    Corrupted,
    Empty,
    Unknown,
}

/// Supported operations for a device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedOperations {
    pub get_info: bool,
    pub factory_reset: bool,
    pub frp_remove: bool,
    pub repair_imei: bool,
    pub repair_imei_patch: bool,
    pub restore_store_backup: bool,
    pub update_firmware: bool,
    pub advanced_update_firmware: bool,
    pub read_write_certificate: bool,
    pub patch_certificate: bool,
    pub repair_drk: bool,
    pub repair_efs: bool,
    pub network_repair: bool,
    pub network_factory_reset: bool,
    pub reset_screenlock: bool,
    pub reset_reactivation_lock: bool,
    pub reset_ee_lock: bool,
    pub demo_remove: bool,
    pub csc_change: bool,
    pub carrier_relock: bool,
    pub mdm_remove: bool,
    pub knox_guard_remove: bool,
    pub remove_lost_mode: bool,
    pub remove_warnings: bool,
    pub root: bool,
    pub unroot: bool,
    pub magisk_root: bool,
    pub bootloader_unlock: bool,
    pub bootloader_relock: bool,
    pub repair_mac: bool,
    pub read_codes: bool,
    pub enable_adb: bool,
    pub remove_huawei_id: bool,
    pub repair_boot: bool,
}

impl Default for SupportedOperations {
    fn default() -> Self {
        Self {
            get_info: false,
            factory_reset: false,
            frp_remove: false,
            repair_imei: false,
            repair_imei_patch: false,
            restore_store_backup: false,
            update_firmware: false,
            advanced_update_firmware: false,
            read_write_certificate: false,
            patch_certificate: false,
            repair_drk: false,
            repair_efs: false,
            network_repair: false,
            network_factory_reset: false,
            reset_screenlock: false,
            reset_reactivation_lock: false,
            reset_ee_lock: false,
            demo_remove: false,
            csc_change: false,
            carrier_relock: false,
            mdm_remove: false,
            knox_guard_remove: false,
            remove_lost_mode: false,
            remove_warnings: false,
            root: false,
            unroot: false,
            magisk_root: false,
            bootloader_unlock: false,
            bootloader_relock: false,
            repair_mac: false,
            read_codes: false,
            enable_adb: false,
            remove_huawei_id: false,
            repair_boot: false,
        }
    }
}
