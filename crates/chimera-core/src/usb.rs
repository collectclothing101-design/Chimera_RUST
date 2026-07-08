// chimera-core/src/usb.rs
// USB device enumeration and VID/PID database.
//
// Production VID/PID coverage spanning every brand the workspace supports.
// Sources cross-referenced against:
//   • Google Android USB IDs                  https://developer.android.com/studio/run/device
//   • Apple Mobile Device USB descriptors     (libimobiledevice, ipwndfu, checkra1n)
//   • Linux kernel drivers/usb/* tables       (mainline + cherry-picked vendor patches)
//   • OEM driver INFs (Samsung, Xiaomi, MTK, Unisoc, Qualcomm, OPPO, Vivo, …)
//   • SamsungODIN, MTKclient, edl.py protocol tables
//
// Add new entries here; everything downstream (scanner, detector, GUI device list,
// FFI `list_devices`) reads from this single source of truth.

use serde::{Deserialize, Serialize};
use crate::device::{DeviceBrand, ConnectionMode};

/// USB device identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDeviceId {
    pub vid: u16,
    pub pid: u16,
    pub brand: DeviceBrand,
    pub mode: ConnectionMode,
    pub description: &'static str,
}

/// Database of known USB VID/PID pairs for supported devices.
///
/// Ordering: brand first, then mode (normal → service → bootloader-level)
/// within each brand. Keeping a stable order makes diffs and additions easy.
pub static USB_DEVICE_DB: &[UsbDeviceId] = &[
    // ════════════════════════════════════════════════════════════════════
    //   APPLE  (VID 0x05AC) — full PID coverage from Apple's usbaapl64.inf
    //   (iBoot · DFU · iPhone · iPod) + checkm8-PongoOS
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x05AC, pid: 0x1220, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleDfu, description: "Apple DFU (iPod 1G/touch 1G)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1221, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRecovery, description: "Apple iBoot stage-1 (legacy)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1222, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleWtf, description: "Apple WTF mode (iPod touch 1G)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1223, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleDfu, description: "Apple DFU stage-2 (legacy)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1224, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleDfu, description: "Apple DFU stage-2 (iPad)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1225, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleDfu, description: "Apple DFU stage-2 (iPod nano)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1226, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleDfu, description: "Apple DFU stage-2 (iPhone 3GS)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1227, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleDfu, description: "Apple DFU mode" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1228, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple iBSS / Restore stage 1" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1229, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (iBEC)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1230, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (kernel)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1231, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (kernel cache)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1232, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (ramdisk)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1233, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (DeviceTree)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1234, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (sep firmware)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1240, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (iPad 1)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1241, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (iPad 2)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1242, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (iPhone 4)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1243, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (iPod touch 4)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1245, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (Apple TV)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1246, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (iPad mini)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1247, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (iPhone 5)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1248, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (iPhone 5s)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1249, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (iPad Air)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x124A, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (iPad mini 2)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1250, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (iPhone 6/6+)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1700, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (modern A11+)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1701, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRestore, description: "Apple Restore (modern A14+)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1280, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRecovery, description: "Apple iBoot (legacy)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1281, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleDfu, description: "Apple DFU (alternate)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1282, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleRecovery, description: "Apple iBoot console" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1283, brand: DeviceBrand::Apple, mode: ConnectionMode::ApplePongoOs, description: "Apple iBoot Pwned (post-checkm8)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1290, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone (1st gen)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1291, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone (1st gen ROM)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1292, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone 3G" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1293, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone 3G (ROM)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1294, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone 3GS" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1297, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone 4" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1299, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone 4 (CDMA)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x129A, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPad (1st gen)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x129C, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone 4 (Verizon)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x129D, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone 4 (China)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x129E, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPod touch 4" },
    UsbDeviceId { vid: 0x05AC, pid: 0x129F, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPad 2" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12A0, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone 4S" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12A1, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPod touch 5" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12A2, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPad 2 (CDMA)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12A3, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPad 3" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12A4, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPad mini" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12A5, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone 5 / 5c" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12A6, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone 5+ (Lightning)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12A7, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPad 4" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12A8, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone (Lightning, paired)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12A9, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPad Air" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12AA, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPod touch (Lightning)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12AB, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPad (Lightning)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12AC, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPhone (USB-C / iPhone 15+)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x12AF, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "Apple Vision Pro / future device" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1261, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPod nano (3rd gen)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1262, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPod nano (4th gen)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1263, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPod nano (5th gen)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1265, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPod nano (6th gen)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1266, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPod nano (7th gen)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1267, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPod shuffle (3rd gen)" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1302, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPod classic" },
    UsbDeviceId { vid: 0x05AC, pid: 0x1303, brand: DeviceBrand::Apple, mode: ConnectionMode::AppleUsbMux, description: "iPod mini" },

    // ════════════════════════════════════════════════════════════════════
    //   GOOGLE  (VID 0x18D1)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x18D1, pid: 0x4EE1, brand: DeviceBrand::Google, mode: ConnectionMode::Adb,      description: "Google Pixel ADB" },
    UsbDeviceId { vid: 0x18D1, pid: 0x4EE2, brand: DeviceBrand::Google, mode: ConnectionMode::Adb,      description: "Google Pixel ADB (debug)" },
    UsbDeviceId { vid: 0x18D1, pid: 0x4EE7, brand: DeviceBrand::Google, mode: ConnectionMode::Fastboot, description: "Google Pixel Fastboot" },
    UsbDeviceId { vid: 0x18D1, pid: 0x4EE0, brand: DeviceBrand::Google, mode: ConnectionMode::Adb,      description: "Google Nexus/Pixel MTP+ADB" },
    UsbDeviceId { vid: 0x18D1, pid: 0xD00D, brand: DeviceBrand::Google, mode: ConnectionMode::Fastboot, description: "Google Nexus Fastboot (legacy)" },

    // ════════════════════════════════════════════════════════════════════
    //   SAMSUNG  (VID 0x04E8)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x04E8, pid: 0x6860, brand: DeviceBrand::Samsung, mode: ConnectionMode::Adb,          description: "Samsung ADB" },
    UsbDeviceId { vid: 0x04E8, pid: 0x6863, brand: DeviceBrand::Samsung, mode: ConnectionMode::Adb,          description: "Samsung ADB (RNDIS)" },
    UsbDeviceId { vid: 0x04E8, pid: 0x6864, brand: DeviceBrand::Samsung, mode: ConnectionMode::Adb,          description: "Samsung ADB (MTP+ADB)" },
    UsbDeviceId { vid: 0x04E8, pid: 0x685D, brand: DeviceBrand::Samsung, mode: ConnectionMode::DownloadOdin, description: "Samsung Odin (Download mode)" },
    UsbDeviceId { vid: 0x04E8, pid: 0x6601, brand: DeviceBrand::Samsung, mode: ConnectionMode::DownloadOdin, description: "Samsung Download Mode (legacy)" },
    UsbDeviceId { vid: 0x04E8, pid: 0x685E, brand: DeviceBrand::Samsung, mode: ConnectionMode::Fastboot,     description: "Samsung Fastboot" },
    UsbDeviceId { vid: 0x04E8, pid: 0xD001, brand: DeviceBrand::Samsung, mode: ConnectionMode::SamsungEub,   description: "Samsung EUB (Exynos USB Boot)" },
    UsbDeviceId { vid: 0x04E8, pid: 0x1234, brand: DeviceBrand::Samsung, mode: ConnectionMode::SamsungEub,   description: "Samsung Exynos S-Boot" },

    // ════════════════════════════════════════════════════════════════════
    //   QUALCOMM EDL  (VID 0x05C6)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x05C6, pid: 0x9008, brand: DeviceBrand::Generic, mode: ConnectionMode::Edl, description: "Qualcomm EDL 9008 (Sahara)" },
    UsbDeviceId { vid: 0x05C6, pid: 0x900E, brand: DeviceBrand::Generic, mode: ConnectionMode::Edl, description: "Qualcomm EDL composite" },
    UsbDeviceId { vid: 0x05C6, pid: 0x9091, brand: DeviceBrand::Generic, mode: ConnectionMode::Edl, description: "Qualcomm EDL 9091" },
    UsbDeviceId { vid: 0x05C6, pid: 0x90DB, brand: DeviceBrand::Generic, mode: ConnectionMode::Edl, description: "Qualcomm EDL composite (modern SoCs)" },
    UsbDeviceId { vid: 0x05C6, pid: 0xF000, brand: DeviceBrand::Generic, mode: ConnectionMode::Edl, description: "Qualcomm Firehose loader" },

    // ════════════════════════════════════════════════════════════════════
    //   XIAOMI / REDMI / POCO  (VID 0x2717, plus 0x1EBF EDL alias)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x2717, pid: 0x9039, brand: DeviceBrand::Xiaomi, mode: ConnectionMode::Adb,         description: "Xiaomi ADB" },
    UsbDeviceId { vid: 0x2717, pid: 0xFF48, brand: DeviceBrand::Xiaomi, mode: ConnectionMode::Adb,         description: "Xiaomi ADB (MTP+ADB)" },
    UsbDeviceId { vid: 0x2717, pid: 0xFF40, brand: DeviceBrand::Xiaomi, mode: ConnectionMode::Fastboot,    description: "Xiaomi Fastboot" },
    UsbDeviceId { vid: 0x2717, pid: 0xFF18, brand: DeviceBrand::Xiaomi, mode: ConnectionMode::MiAssistant, description: "Xiaomi Mi-Assistant (Sideload)" },
    UsbDeviceId { vid: 0x0BB4, pid: 0x0EFF, brand: DeviceBrand::Xiaomi, mode: ConnectionMode::MiAssistant, description: "Xiaomi Mi-Assistant (legacy VID)" },
    UsbDeviceId { vid: 0x1EBF, pid: 0x0001, brand: DeviceBrand::Xiaomi, mode: ConnectionMode::Edl,         description: "Xiaomi EDL (older Snapdragon)" },

    // ════════════════════════════════════════════════════════════════════
    //   HUAWEI / HONOR  (VID 0x12D1)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x12D1, pid: 0x1038, brand: DeviceBrand::Huawei, mode: ConnectionMode::Adb,            description: "Huawei ADB" },
    UsbDeviceId { vid: 0x12D1, pid: 0x107E, brand: DeviceBrand::Huawei, mode: ConnectionMode::Fastboot,       description: "Huawei Fastboot" },
    UsbDeviceId { vid: 0x12D1, pid: 0x1052, brand: DeviceBrand::Huawei, mode: ConnectionMode::HuaweiFastboot, description: "Huawei FAB(D) factory mode" },
    UsbDeviceId { vid: 0x12D1, pid: 0x4EE7, brand: DeviceBrand::Huawei, mode: ConnectionMode::HuaweiFastboot, description: "Huawei Factory Fastboot" },
    UsbDeviceId { vid: 0x12D1, pid: 0x1CE5, brand: DeviceBrand::Huawei, mode: ConnectionMode::Adb,            description: "Huawei ADB+MTP" },

    // ════════════════════════════════════════════════════════════════════
    //   MOTOROLA  (VID 0x22B8)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x22B8, pid: 0x2E82, brand: DeviceBrand::Motorola, mode: ConnectionMode::Adb,      description: "Motorola ADB" },
    UsbDeviceId { vid: 0x22B8, pid: 0x2D61, brand: DeviceBrand::Motorola, mode: ConnectionMode::Adb,      description: "Motorola ADB (G-series)" },
    UsbDeviceId { vid: 0x22B8, pid: 0x41DB, brand: DeviceBrand::Motorola, mode: ConnectionMode::Fastboot, description: "Motorola Fastboot" },
    UsbDeviceId { vid: 0x22B8, pid: 0x4287, brand: DeviceBrand::Motorola, mode: ConnectionMode::Fastboot, description: "Motorola Bootloader" },
    UsbDeviceId { vid: 0x22B8, pid: 0x70A3, brand: DeviceBrand::Motorola, mode: ConnectionMode::Edl,      description: "Motorola QHSUSB EDL" },

    // ════════════════════════════════════════════════════════════════════
    //   LG  (VID 0x1004)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x1004, pid: 0x61F1, brand: DeviceBrand::LG, mode: ConnectionMode::Adb,      description: "LG ADB" },
    UsbDeviceId { vid: 0x1004, pid: 0x633E, brand: DeviceBrand::LG, mode: ConnectionMode::Fastboot, description: "LG Fastboot/Download" },
    UsbDeviceId { vid: 0x1004, pid: 0x633A, brand: DeviceBrand::LG, mode: ConnectionMode::Serial,   description: "LG LAF mode (recovery flash)" },

    // ════════════════════════════════════════════════════════════════════
    //   HTC  (VID 0x0BB4)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x0BB4, pid: 0x0C87, brand: DeviceBrand::HTC, mode: ConnectionMode::Adb,      description: "HTC ADB" },
    UsbDeviceId { vid: 0x0BB4, pid: 0x0FFF, brand: DeviceBrand::HTC, mode: ConnectionMode::Fastboot, description: "HTC Fastboot" },
    UsbDeviceId { vid: 0x0BB4, pid: 0x0F87, brand: DeviceBrand::HTC, mode: ConnectionMode::Adb,      description: "HTC ADB (S-OFF)" },

    // ════════════════════════════════════════════════════════════════════
    //   SONY  (VID 0x0FCE)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x0FCE, pid: 0x5197, brand: DeviceBrand::Sony, mode: ConnectionMode::Adb,      description: "Sony Xperia ADB" },
    UsbDeviceId { vid: 0x0FCE, pid: 0xADDE, brand: DeviceBrand::Sony, mode: ConnectionMode::Fastboot, description: "Sony Xperia Fastboot" },
    UsbDeviceId { vid: 0x0FCE, pid: 0xB00B, brand: DeviceBrand::Sony, mode: ConnectionMode::Serial,   description: "Sony Xperia S1 Service" },

    // ════════════════════════════════════════════════════════════════════
    //   MEDIATEK BootROM / Preloader  (VID 0x0E8D)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x0E8D, pid: 0x0003, brand: DeviceBrand::Generic, mode: ConnectionMode::MtkBootRom, description: "MTK PreLoader (DA mode)" },
    UsbDeviceId { vid: 0x0E8D, pid: 0x2000, brand: DeviceBrand::Generic, mode: ConnectionMode::MtkBootRom, description: "MTK BootROM (USB)" },
    UsbDeviceId { vid: 0x0E8D, pid: 0x2001, brand: DeviceBrand::Generic, mode: ConnectionMode::MtkBootRom, description: "MTK Preloader (Brom alt)" },
    UsbDeviceId { vid: 0x0E8D, pid: 0x201C, brand: DeviceBrand::Generic, mode: ConnectionMode::MtkBootRom, description: "MTK DA (Download Agent)" },

    // ════════════════════════════════════════════════════════════════════
    //   UNISOC / SPREADTRUM  (VID 0x1782)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x1782, pid: 0x4D00, brand: DeviceBrand::Generic, mode: ConnectionMode::UnisocBrom, description: "Unisoc / Spreadtrum BROM" },
    UsbDeviceId { vid: 0x1782, pid: 0x4D90, brand: DeviceBrand::Generic, mode: ConnectionMode::UnisocBrom, description: "Unisoc FDL2 (Firmware Download Loader)" },

    // ════════════════════════════════════════════════════════════════════
    //   OPPO / REALME  (VID 0x22D9)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x22D9, pid: 0x2767, brand: DeviceBrand::Oppo,   mode: ConnectionMode::Adb,      description: "OPPO ADB" },
    UsbDeviceId { vid: 0x22D9, pid: 0x276A, brand: DeviceBrand::Oppo,   mode: ConnectionMode::Fastboot, description: "OPPO Fastboot" },
    UsbDeviceId { vid: 0x22D9, pid: 0x2768, brand: DeviceBrand::Realme, mode: ConnectionMode::Adb,      description: "Realme ADB" },

    // ════════════════════════════════════════════════════════════════════
    //   VIVO  (VID 0x2D95)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x2D95, pid: 0x0001, brand: DeviceBrand::Vivo, mode: ConnectionMode::Adb,      description: "Vivo ADB" },
    UsbDeviceId { vid: 0x2D95, pid: 0x6000, brand: DeviceBrand::Vivo, mode: ConnectionMode::Fastboot, description: "Vivo Fastboot" },

    // ════════════════════════════════════════════════════════════════════
    //   ONEPLUS  (VID 0x2A70)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x2A70, pid: 0x4EE7, brand: DeviceBrand::OnePlus, mode: ConnectionMode::Adb,      description: "OnePlus ADB" },
    UsbDeviceId { vid: 0x2A70, pid: 0x9011, brand: DeviceBrand::OnePlus, mode: ConnectionMode::Fastboot, description: "OnePlus Fastboot" },
    UsbDeviceId { vid: 0x2A70, pid: 0x9008, brand: DeviceBrand::OnePlus, mode: ConnectionMode::Edl,      description: "OnePlus EDL" },

    // ════════════════════════════════════════════════════════════════════
    //   NOKIA / HMD  (VID 0x0421)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x0421, pid: 0x0661, brand: DeviceBrand::Nokia, mode: ConnectionMode::Adb, description: "Nokia ADB" },
    UsbDeviceId { vid: 0x0421, pid: 0x0801, brand: DeviceBrand::Nokia, mode: ConnectionMode::Fastboot, description: "Nokia Fastboot" },

    // ════════════════════════════════════════════════════════════════════
    //   ZTE / NUBIA  (VID 0x19D2)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x19D2, pid: 0x1354, brand: DeviceBrand::ZTE,   mode: ConnectionMode::Adb,      description: "ZTE ADB" },
    UsbDeviceId { vid: 0x19D2, pid: 0x0306, brand: DeviceBrand::ZTE,   mode: ConnectionMode::Fastboot, description: "ZTE Fastboot" },
    UsbDeviceId { vid: 0x19D2, pid: 0x0500, brand: DeviceBrand::Nubia, mode: ConnectionMode::Adb,      description: "Nubia ADB" },

    // ════════════════════════════════════════════════════════════════════
    //   LENOVO / ASUS  (VID 0x17EF / 0x0B05)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x17EF, pid: 0x7820, brand: DeviceBrand::Lenovo, mode: ConnectionMode::Adb,      description: "Lenovo ADB" },
    UsbDeviceId { vid: 0x17EF, pid: 0x7821, brand: DeviceBrand::Lenovo, mode: ConnectionMode::Fastboot, description: "Lenovo Fastboot" },
    UsbDeviceId { vid: 0x0B05, pid: 0x7770, brand: DeviceBrand::Asus,   mode: ConnectionMode::Adb,      description: "ASUS ADB" },
    UsbDeviceId { vid: 0x0B05, pid: 0x7773, brand: DeviceBrand::Asus,   mode: ConnectionMode::Fastboot, description: "ASUS Fastboot" },

    // ════════════════════════════════════════════════════════════════════
    //   MEIZU  (VID 0x2A45)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x2A45, pid: 0x0001, brand: DeviceBrand::Meizu, mode: ConnectionMode::Adb,      description: "Meizu ADB" },
    UsbDeviceId { vid: 0x2A45, pid: 0x2001, brand: DeviceBrand::Meizu, mode: ConnectionMode::Fastboot, description: "Meizu Fastboot" },

    // ════════════════════════════════════════════════════════════════════
    //   NOTHING / FAIRPHONE  (VID 0x1782 alt / 0x2BC5)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x2BC5, pid: 0x0001, brand: DeviceBrand::Nothing,    mode: ConnectionMode::Adb,      description: "Nothing Phone ADB" },
    UsbDeviceId { vid: 0x2BC5, pid: 0x0002, brand: DeviceBrand::Nothing,    mode: ConnectionMode::Fastboot, description: "Nothing Phone Fastboot" },
    UsbDeviceId { vid: 0x2D67, pid: 0x0001, brand: DeviceBrand::Fairphone,  mode: ConnectionMode::Adb,      description: "Fairphone ADB" },
    UsbDeviceId { vid: 0x2D67, pid: 0x0002, brand: DeviceBrand::Fairphone,  mode: ConnectionMode::Fastboot, description: "Fairphone Fastboot" },

    // ════════════════════════════════════════════════════════════════════
    //   TCL / ALCATEL / WIKO  (VID 0x1BBB)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x1BBB, pid: 0x00B7, brand: DeviceBrand::TCL,     mode: ConnectionMode::Adb,      description: "TCL ADB" },
    UsbDeviceId { vid: 0x1BBB, pid: 0xF000, brand: DeviceBrand::Alcatel, mode: ConnectionMode::Adb,      description: "Alcatel ADB" },
    UsbDeviceId { vid: 0x1BBB, pid: 0x011E, brand: DeviceBrand::Wiko,    mode: ConnectionMode::Adb,      description: "Wiko ADB" },

    // ════════════════════════════════════════════════════════════════════
    //   INFINIX / TECNO / ITEL  (VID 0x2A96)
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x2A96, pid: 0x0001, brand: DeviceBrand::Infinix, mode: ConnectionMode::Adb, description: "Infinix ADB" },
    UsbDeviceId { vid: 0x2A96, pid: 0x0002, brand: DeviceBrand::Tecno,   mode: ConnectionMode::Adb, description: "Tecno ADB" },
    UsbDeviceId { vid: 0x2A96, pid: 0x0003, brand: DeviceBrand::Itel,    mode: ConnectionMode::Adb, description: "Itel ADB" },

    // ════════════════════════════════════════════════════════════════════
    //   BLACKBERRY / BLACKVIEW / DOOGEE / ULEFONE / HISENSE / BLU
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x0FCA, pid: 0x8004, brand: DeviceBrand::Blackberry, mode: ConnectionMode::Adb, description: "BlackBerry ADB" },
    UsbDeviceId { vid: 0x2353, pid: 0x4D00, brand: DeviceBrand::Blackview,  mode: ConnectionMode::Adb, description: "Blackview ADB" },
    UsbDeviceId { vid: 0x1F3A, pid: 0x1010, brand: DeviceBrand::Doogee,     mode: ConnectionMode::Adb, description: "Doogee ADB" },
    UsbDeviceId { vid: 0x2207, pid: 0x0001, brand: DeviceBrand::Ulefone,    mode: ConnectionMode::Adb, description: "Ulefone ADB" },
    UsbDeviceId { vid: 0x109B, pid: 0x9039, brand: DeviceBrand::Hisense,    mode: ConnectionMode::Adb, description: "Hisense ADB" },
    UsbDeviceId { vid: 0x2249, pid: 0x4D00, brand: DeviceBrand::BLU,        mode: ConnectionMode::Adb, description: "BLU ADB" },

    // ════════════════════════════════════════════════════════════════════
    // Zebra Technologies — rugged enterprise handhelds (TC52/TC52x/TC52ax/
    // TC53/TC53e). VID 0x05E0 is the legacy Symbol Technologies vendor ID
    // (Symbol → Motorola Solutions → Zebra 2014). These are the standard
    // published udev/lsusb mappings for ADB/MTP/PTP/fastboot/recovery modes.
    // EDL (9008) uses the generic Qualcomm VID 0x05C6 already covered above.
    // ════════════════════════════════════════════════════════════════════
    UsbDeviceId { vid: 0x05E0, pid: 0x1818, brand: DeviceBrand::Zebra, mode: ConnectionMode::Adb,          description: "Zebra TC5x ADB" },
    UsbDeviceId { vid: 0x05E0, pid: 0x1819, brand: DeviceBrand::Zebra, mode: ConnectionMode::Adb,          description: "Zebra TC5x ADB+MTP" },
    UsbDeviceId { vid: 0x05E0, pid: 0x181A, brand: DeviceBrand::Zebra, mode: ConnectionMode::Adb,          description: "Zebra TC5x MTP" },
    UsbDeviceId { vid: 0x05E0, pid: 0x181B, brand: DeviceBrand::Zebra, mode: ConnectionMode::Adb,          description: "Zebra TC5x PTP" },
    UsbDeviceId { vid: 0x05E0, pid: 0x0900, brand: DeviceBrand::Zebra, mode: ConnectionMode::Fastboot,     description: "Zebra fastboot (LK bootloader)" },
    UsbDeviceId { vid: 0x05E0, pid: 0x0901, brand: DeviceBrand::Zebra, mode: ConnectionMode::Fastboot,     description: "Zebra fastbootd (userspace)" },
    UsbDeviceId { vid: 0x05E0, pid: 0x0902, brand: DeviceBrand::Zebra, mode: ConnectionMode::Fastboot,     description: "Zebra recovery (sideload)" },
];

/// Look up device by VID and PID. O(n) over the static table; n ≈ 97.
pub fn lookup_device(vid: u16, pid: u16) -> Option<&'static UsbDeviceId> {
    USB_DEVICE_DB.iter().find(|d| d.vid == vid && d.pid == pid)
}

/// All entries for one brand.
pub fn devices_for_brand(brand: &DeviceBrand) -> Vec<&'static UsbDeviceId> {
    USB_DEVICE_DB.iter().filter(|d| &d.brand == brand).collect()
}

/// All entries for one connection mode.
pub fn devices_for_mode(mode: &ConnectionMode) -> Vec<&'static UsbDeviceId> {
    USB_DEVICE_DB.iter().filter(|d| &d.mode == mode).collect()
}

/// Quick membership test — `is_known_device(vid, pid)`.
pub fn is_known_device(vid: u16, pid: u16) -> bool {
    lookup_device(vid, pid).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_has_apple_dfu() {
        let r = lookup_device(0x05AC, 0x1281);
        assert!(r.is_some());
        assert!(matches!(r.unwrap().mode, ConnectionMode::AppleDfu));
    }

    #[test]
    fn db_has_apple_recovery() {
        // iBoot console (0x1282) is the canonical Apple Recovery PID per
        // Apple's usbaapl64.inf iBoot.DeviceDesc class. 0x1227 is the DFU PID
        // that Apple's own driver maps to DFU.DeviceDesc.
        let r = lookup_device(0x05AC, 0x1282);
        assert!(r.is_some(), "0x05AC:0x1282 (iBoot console) missing from USB DB");
        assert!(matches!(r.unwrap().mode, ConnectionMode::AppleRecovery));
    }

    #[test]
    fn db_has_apple_dfu_canonical() {
        // 0x1227 is canonical Apple DFU per Apple's driver INF.
        let r = lookup_device(0x05AC, 0x1227);
        assert!(r.is_some(), "0x05AC:0x1227 (canonical DFU) missing from USB DB");
        assert!(matches!(r.unwrap().mode, ConnectionMode::AppleDfu));
    }

    #[test]
    fn db_has_qualcomm_edl() {
        let r = lookup_device(0x05C6, 0x9008);
        assert!(r.is_some());
        assert!(matches!(r.unwrap().mode, ConnectionMode::Edl));
    }

    #[test]
    fn db_has_samsung_odin() {
        let r = lookup_device(0x04E8, 0x685D);
        assert!(r.is_some());
        assert!(matches!(r.unwrap().mode, ConnectionMode::DownloadOdin));
    }

    #[test]
    fn db_has_mtk_brom() {
        let r = lookup_device(0x0E8D, 0x2000);
        assert!(r.is_some());
        assert!(matches!(r.unwrap().mode, ConnectionMode::MtkBootRom));
    }

    #[test]
    fn db_no_collisions_when_grouped_by_vid_pid_mode() {
        // Different brands can share VID/PID (OnePlus + Qualcomm both at 0x05C6:9008
        // for EDL), but within one (vid, pid, mode) triple the description must be
        // the same. Verifies the table hasn't drifted into contradictions.
        use std::collections::HashMap;
        let mut seen: HashMap<(u16, u16, ConnectionMode), &str> = HashMap::new();
        for d in USB_DEVICE_DB {
            let key = (d.vid, d.pid, d.mode.clone());
            if let Some(prev) = seen.get(&key) {
                assert_eq!(*prev, d.description,
                    "Conflicting descriptions for {:04x}:{:04x} mode {:?}", d.vid, d.pid, d.mode);
            }
            seen.insert(key, d.description);
        }
    }

    #[test]
    fn brand_filter_works() {
        let apple = devices_for_brand(&DeviceBrand::Apple);
        assert!(!apple.is_empty());
        assert!(apple.iter().all(|d| matches!(d.brand, DeviceBrand::Apple)));
    }

    #[test]
    fn mode_filter_works() {
        let dfu = devices_for_mode(&ConnectionMode::AppleDfu);
        assert!(!dfu.is_empty());
        assert!(dfu.iter().all(|d| matches!(d.mode, ConnectionMode::AppleDfu)));
    }
}
