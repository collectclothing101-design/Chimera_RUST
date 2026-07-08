// chimera-devices/src/detector.rs
// Auto-detect device brand, model, and capabilities

use chimera_core::device::{DeviceInfo, DeviceBrand, DeviceChipset, ConnectionMode, SupportedOperations};
use chimera_core::usb::{lookup_device, vid_description, is_edl_device, is_mtk_brom, mode_description};

/// Result of auto-detecting a connected device
#[derive(Debug, Clone)]
pub struct DetectedDevice {
    pub brand: DeviceBrand,
    pub model: String,
    pub serial: String,
    pub connection_mode: ConnectionMode,
    pub chipset: DeviceChipset,
    pub vid: u16,
    pub pid: u16,
    pub vid_name: String,
    pub mode_name: String,
    pub supported_ops: SupportedOperations,
    pub is_rooted: bool,
    pub android_version: Option<String>,
    pub firmware_version: Option<String>,
}

/// Auto-detect connected device and return full info
pub fn auto_detect_device(vid: u16, pid: u16, adb_serial: Option<&str>) -> Option<DetectedDevice> {
    let usb_info = lookup_device(vid, pid)?;

    let mut info = DeviceInfo::new_unknown(adb_serial.unwrap_or("unknown").to_string());
    info.brand = usb_info.brand.clone();
    info.connection_mode = usb_info.mode.clone();

    let supported_ops = DeviceDetector::get_supported_ops(&info);

    Some(DetectedDevice {
        brand: usb_info.brand.clone(),
        model: usb_info.description.to_string(),
        serial: adb_serial.unwrap_or("unknown").to_string(),
        connection_mode: usb_info.mode.clone(),
        chipset: DeviceChipset::Unknown,
        vid,
        pid,
        vid_name: vid_description(vid).to_string(),
        mode_name: mode_description(&usb_info.mode).to_string(),
        supported_ops,
        is_rooted: false,
        android_version: None,
        firmware_version: None,
    })
}

/// Auto-detect from ADB device list output
pub fn auto_detect_from_adb_output(adb_output: &str) -> Vec<DetectedDevice> {
    let mut devices = Vec::new();

    for line in adb_output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("List of devices") || line.starts_with("*") {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        let serial = parts[0];
        let state = parts[1];

        // Default to generic ADB device (VID 0x18D1 Google, PID 0x4EE1 ADB)
        let vid = 0x18D1;
        let pid = 0x4EE1;

        if let Some(mut device) = auto_detect_device(vid, pid, Some(serial)) {
            device.model = format!("ADB Device ({})", serial);
            devices.push(device);
        }
    }

    devices
}

/// Detect device from USB vendor/product ID and enrich with ADB properties
pub fn detect_and_enrich(vid: u16, pid: u16, serial: &str, props: &std::collections::HashMap<String, String>) -> Option<DetectedDevice> {
    let mut device = auto_detect_device(vid, pid, Some(serial))?;

    // Enrich with ADB properties
    device.model = props.get("ro.product.model")
        .or_else(|| props.get("ro.product.name"))
        .cloned()
        .unwrap_or(device.model);

    device.is_rooted = props.get("ro.build.type")
        .map(|t| t == "userdebug" || t == "eng")
        .unwrap_or(false);

    device.android_version = props.get("ro.build.version.release").cloned();
    device.firmware_version = props.get("ro.build.display.id")
        .or_else(|| props.get("ro.build.PDA"))
        .cloned();

    device.chipset = DeviceDetector::detect_chipset(props);

    Some(device)
}

/// Detect device type and capabilities
pub struct DeviceDetector;

impl DeviceDetector {
    /// Given a connected DeviceInfo, determine its capabilities
    pub fn get_supported_ops(info: &DeviceInfo) -> SupportedOperations {
        let mut ops = SupportedOperations::default();
        
        match (&info.brand, &info.connection_mode) {
            (DeviceBrand::Samsung, ConnectionMode::Adb) => {
                ops.get_info = true;
                ops.factory_reset = true;
                ops.frp_remove = true;
                ops.reset_screenlock = true;
                ops.reset_reactivation_lock = true;
                ops.demo_remove = true;
                ops.csc_change = true;
                ops.mdm_remove = true;
                ops.knox_guard_remove = true;
                ops.remove_lost_mode = true;
                ops.remove_warnings = true;
                ops.repair_efs = true;
                ops.network_factory_reset = true;
                ops.root = true;
                ops.magisk_root = true;
                ops.restore_store_backup = true;
                ops.carrier_relock = true;
                ops.repair_mac = true;
            }
            (DeviceBrand::Samsung, ConnectionMode::DownloadOdin) => {
                ops.get_info = true;
                ops.update_firmware = true;
                ops.advanced_update_firmware = true;
                ops.read_write_certificate = true;
                ops.patch_certificate = true;
                ops.repair_imei = true;
                ops.repair_drk = true;
                ops.network_repair = true;
                ops.repair_efs = true;
                ops.restore_store_backup = true;
                ops.bootloader_unlock = true;
                ops.bootloader_relock = true;
                ops.read_codes = true;
                ops.reset_screenlock = true;
                ops.frp_remove = true;
            }
            (DeviceBrand::Samsung, ConnectionMode::SamsungEub) => {
                ops.get_info = true;
                ops.update_firmware = true;
                ops.repair_imei = true;
                ops.patch_certificate = true;
                ops.read_write_certificate = true;
                ops.read_codes = true;
                ops.repair_boot = true;
                ops.remove_warnings = true;
                ops.remove_lost_mode = true;
                ops.frp_remove = true;
                ops.demo_remove = true;
            }
            (DeviceBrand::Xiaomi, ConnectionMode::Adb) => {
                ops.get_info = true;
                ops.factory_reset = true;
                ops.frp_remove = true;
                ops.network_factory_reset = true;
                ops.update_firmware = true;
                ops.restore_store_backup = true;
            }
            (DeviceBrand::Xiaomi, ConnectionMode::Edl) => {
                ops.get_info = true;
                ops.factory_reset = true;
                ops.frp_remove = true;
                ops.update_firmware = true;
                ops.advanced_update_firmware = true;
                ops.repair_imei = true;
                ops.restore_store_backup = true;
            }
            (DeviceBrand::Xiaomi, ConnectionMode::Fastboot) => {
                ops.get_info = true;
                ops.factory_reset = true;
                ops.update_firmware = true;
                ops.bootloader_unlock = true;
                ops.bootloader_relock = true;
                ops.repair_imei_patch = true;
            }
            (DeviceBrand::Huawei | DeviceBrand::Honor, ConnectionMode::Adb) => {
                ops.get_info = true;
                ops.factory_reset = true;
                ops.frp_remove = true;
                ops.remove_huawei_id = true;
                ops.demo_remove = true;
                ops.restore_store_backup = true;
            }
            (DeviceBrand::Huawei | DeviceBrand::Honor, ConnectionMode::HuaweiFastboot) => {
                ops.get_info = true;
                ops.update_firmware = true;
                ops.frp_remove = true;
                ops.repair_imei = true;
                ops.read_write_certificate = true;
                ops.restore_store_backup = true;
                ops.remove_huawei_id = true;
                ops.demo_remove = true;
            }
            (_, ConnectionMode::Adb) => {
                // Generic ADB
                ops.get_info = true;
                ops.factory_reset = true;
                ops.frp_remove = true;
                ops.reset_screenlock = true;
                ops.demo_remove = true;
                ops.root = true;
                ops.magisk_root = true;
                ops.enable_adb = true;
            }
            (_, ConnectionMode::Fastboot) => {
                ops.get_info = true;
                ops.factory_reset = true;
                ops.update_firmware = true;
                ops.bootloader_unlock = true;
                ops.bootloader_relock = true;
            }
            (_, ConnectionMode::MtkBootRom) => {
                ops.get_info = true;
                ops.factory_reset = true;
                ops.frp_remove = true;
                ops.update_firmware = true;
                ops.repair_imei = true;
                ops.repair_imei_patch = true;
                ops.restore_store_backup = true;
                ops.bootloader_unlock = true;
                ops.root = true;
                ops.reset_screenlock = true;
                ops.network_factory_reset = true;
            }
            (_, ConnectionMode::Edl) => {
                ops.get_info = true;
                ops.factory_reset = true;
                ops.frp_remove = true;
                ops.update_firmware = true;
                ops.repair_imei = true;
                ops.restore_store_backup = true;
            }
            (_, ConnectionMode::UnisocBrom) => {
                ops.update_firmware = true;
                ops.frp_remove = true;
            }
            _ => {}
        }
        
        ops
    }

    /// Determine chipset from device properties
    pub fn detect_chipset(props: &std::collections::HashMap<String, String>) -> DeviceChipset {
        let hardware = props.get("ro.hardware").map(String::as_str).unwrap_or("");
        let soc = props.get("ro.hardware.chipname").map(String::as_str).unwrap_or("");
        let platform = props.get("ro.board.platform").map(String::as_str).unwrap_or("");
        
        let all = format!("{} {} {}", hardware, soc, platform).to_lowercase();
        
        if all.contains("qcom") || all.contains("msm") || all.contains("sm8") || all.contains("sm7") {
            DeviceChipset::Qualcomm
        } else if all.contains("exynos") {
            DeviceChipset::Exynos
        } else if all.contains("kirin") || all.contains("hi36") || all.contains("hi38") {
            DeviceChipset::Kirin
        } else if all.contains("helio") || all.contains("mt6") || all.contains("mtk") || all.contains("mediatek") {
            DeviceChipset::MediaTek
        } else if all.contains("unisoc") || all.contains("sc9863") || all.contains("sc9832") {
            DeviceChipset::Unisoc
        } else if all.contains("dimensity") || all.contains("mt68") {
            DeviceChipset::Dimensity
        } else {
            DeviceChipset::Unknown
        }
    }
}
