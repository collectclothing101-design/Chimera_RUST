//! Wrap `ideviceinfo` — reads lockdownd values from an iOS device.
//!
//! Usage:
//!     ideviceinfo                # all values for default device
//!     ideviceinfo -u UDID -x     # XML plist for one device
//!     ideviceinfo -k ProductType # one key only

use std::time::Duration;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::tool::{run, ImobileTool, ImobileError};

/// Subset of lockdownd values the workspace consumes. Extra keys land in
/// `extra` so callers can extend without re-cutting this struct.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceProperties {
    pub udid:                String,
    pub product_type:        Option<String>,  // e.g. "iPhone14,5"
    pub product_name:        Option<String>,  // e.g. "iPhone 13"
    pub product_version:     Option<String>,  // iOS version
    pub build_version:       Option<String>,  // e.g. "20H30"
    pub serial_number:       Option<String>,
    pub device_class:        Option<String>,  // iPhone / iPad / iPod
    pub device_color:        Option<String>,
    pub model_number:        Option<String>,
    pub region_info:         Option<String>,
    pub hardware_model:      Option<String>,  // e.g. "D17AP"
    pub chip_id:             Option<u32>,
    pub board_id:            Option<u32>,
    pub unique_chip_id:      Option<u64>,     // ECID
    pub wifi_address:        Option<String>,
    pub bluetooth_address:   Option<String>,
    pub phone_number:        Option<String>,
    pub imei:                Option<String>,
    pub imei2:               Option<String>,
    pub meid:                Option<String>,
    pub iccid:               Option<String>,
    pub firmware_version:    Option<String>,
    pub baseband_version:    Option<String>,
    pub activation_state:    Option<String>,
    pub passcode_protected:  Option<bool>,
    pub trust_status:        Option<String>,
    pub battery_capacity:    Option<u32>,
    pub extra:               HashMap<String, plist::Value>,
}

/// Read all lockdownd values for one device. UDID omitted = first device.
pub fn ideviceinfo(udid: Option<&str>) -> Result<DeviceProperties, ImobileError> {
    let mut args = vec!["-x"];  // XML plist for parseable output
    if let Some(u) = udid {
        args.push("-u"); args.push(u);
    }
    let output = run(ImobileTool::Ideviceinfo, &args, Duration::from_secs(10))?;
    parse_plist(udid.unwrap_or("").to_string(), &output.stdout)
}

/// Read a single key — faster than parsing the full plist when you only
/// need one value.
pub fn key(udid: Option<&str>, key: &str) -> Result<String, ImobileError> {
    let mut args = vec!["-k", key];
    if let Some(u) = udid {
        args.push("-u"); args.push(u);
    }
    let output = run(ImobileTool::Ideviceinfo, &args, Duration::from_secs(10))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_plist(udid_in: String, bytes: &[u8]) -> Result<DeviceProperties, ImobileError> {
    let value: plist::Value = plist::from_bytes(bytes)
        .map_err(|e| ImobileError::Parse {
            tool: "ideviceinfo".into(),
            detail: e.to_string(),
        })?;
    let dict = value.into_dictionary()
        .ok_or_else(|| ImobileError::Parse {
            tool: "ideviceinfo".into(),
            detail: "root not a dictionary".into(),
        })?;

    let s = |k: &str| dict.get(k).and_then(|v| v.as_string()).map(String::from);
    let u32f = |k: &str| dict.get(k).and_then(|v| v.as_unsigned_integer()).map(|x| x as u32);
    let u64f = |k: &str| dict.get(k).and_then(|v| v.as_unsigned_integer());
    let b = |k: &str| dict.get(k).and_then(|v| v.as_boolean());

    let udid = s("UniqueDeviceID").unwrap_or(udid_in);

    let mut extra = HashMap::new();
    let known_keys: &[&str] = &[
        "UniqueDeviceID", "ProductType", "ProductName", "ProductVersion",
        "BuildVersion", "SerialNumber", "DeviceClass", "DeviceColor",
        "ModelNumber", "RegionInfo", "HardwareModel", "ChipID", "BoardId",
        "UniqueChipID", "WiFiAddress", "BluetoothAddress", "PhoneNumber",
        "InternationalMobileEquipmentIdentity", "InternationalMobileEquipmentIdentity2",
        "MobileEquipmentIdentifier", "IntegratedCircuitCardIdentity",
        "FirmwareVersion", "BasebandVersion", "ActivationState",
        "PasswordProtected", "TrustedHostAttached", "BatteryCurrentCapacity",
    ];
    for (k, v) in &dict {
        if !known_keys.contains(&k.as_str()) {
            extra.insert(k.clone(), v.clone());
        }
    }

    Ok(DeviceProperties {
        udid,
        product_type:        s("ProductType"),
        product_name:        s("ProductName"),
        product_version:     s("ProductVersion"),
        build_version:       s("BuildVersion"),
        serial_number:       s("SerialNumber"),
        device_class:        s("DeviceClass"),
        device_color:        s("DeviceColor"),
        model_number:        s("ModelNumber"),
        region_info:         s("RegionInfo"),
        hardware_model:      s("HardwareModel"),
        chip_id:             u32f("ChipID"),
        board_id:            u32f("BoardId"),
        unique_chip_id:      u64f("UniqueChipID"),
        wifi_address:        s("WiFiAddress"),
        bluetooth_address:   s("BluetoothAddress"),
        phone_number:        s("PhoneNumber"),
        imei:                s("InternationalMobileEquipmentIdentity"),
        imei2:               s("InternationalMobileEquipmentIdentity2"),
        meid:                s("MobileEquipmentIdentifier"),
        iccid:               s("IntegratedCircuitCardIdentity"),
        firmware_version:    s("FirmwareVersion"),
        baseband_version:    s("BasebandVersion"),
        activation_state:    s("ActivationState"),
        passcode_protected:  b("PasswordProtected"),
        trust_status:        s("TrustedHostAttached"),
        battery_capacity:    u32f("BatteryCurrentCapacity"),
        extra,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_plist() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>UniqueDeviceID</key><string>00008030-001A2B3C4D5E6F70</string>
    <key>ProductType</key><string>iPhone14,5</string>
    <key>ProductVersion</key><string>16.5.1</string>
    <key>ProductName</key><string>iPhone OS</string>
    <key>BuildVersion</key><string>20F75</string>
    <key>ChipID</key><integer>32800</integer>
    <key>UniqueChipID</key><integer>1234567890123</integer>
    <key>PasswordProtected</key><true/>
</dict>
</plist>"#;
        let p = parse_plist("".into(), xml).unwrap();
        assert_eq!(p.udid, "00008030-001A2B3C4D5E6F70");
        assert_eq!(p.product_type.as_deref(), Some("iPhone14,5"));
        assert_eq!(p.product_version.as_deref(), Some("16.5.1"));
        assert_eq!(p.chip_id, Some(32800));
        assert_eq!(p.unique_chip_id, Some(1234567890123));
        assert_eq!(p.passcode_protected, Some(true));
    }

    #[test]
    fn unknown_keys_land_in_extra() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>UniqueDeviceID</key><string>x</string>
    <key>SomeNewKeyAppleAdded</key><string>val</string>
</dict>
</plist>"#;
        let p = parse_plist("".into(), xml).unwrap();
        assert!(p.extra.contains_key("SomeNewKeyAppleAdded"));
    }
}
