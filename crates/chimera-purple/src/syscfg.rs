//! `SysCfg` block — sensor calibration, NAND identity, factory color, model
//! number, region, etc. The block on the device is read-only outside of
//! AppleInternal; we expose it read-only.

use serde::{Serialize, Deserialize};
use crate::Result;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SysCfg {
    pub serial_number:           Option<String>,
    pub model_number:            Option<String>,
    pub region_info:             Option<String>,
    pub hardware_model:          Option<String>,
    pub device_color:            Option<String>,
    pub enclosure_color:         Option<String>,
    pub housing_color:           Option<String>,
    pub mlb_serial:              Option<String>,        // main-logic-board serial
    pub config_number:           Option<String>,
    pub original_config_number:  Option<String>,
    pub nand_size_gb:            Option<u32>,
    pub nand_capacity_bytes:     Option<u64>,
    pub ecid:                    Option<u64>,
    pub board_id:                Option<u32>,
    pub chip_id:                 Option<u32>,
    /// Wi-Fi calibration GUID (per-device).
    pub wifi_calibration_guid:   Option<String>,
    /// Gyroscope / accelerometer per-axis trim values.
    pub motion_calibration:      Option<String>,
    /// Camera calibration BLOB length (we don't ship the BLOB itself).
    pub camera_calibration_size: Option<u32>,
}

/// Read every SysCfg-class lockdownd key for one device.
///
/// On retail builds many SysCfg keys are gated behind AppleInternal so
/// the caller may see `None` for them — that's expected. Service+repair
/// scenarios where Apple-Authorized accounts unlock these are out of scope.
pub fn read(udid: Option<&str>) -> Result<SysCfg> {
    let g = |k: &str| chimera_imobile::info::key(udid, k).ok().filter(|s| !s.is_empty());
    let u32f = |k: &str| g(k).and_then(|v| v.parse::<u32>().ok());
    let u64f = |k: &str| g(k).and_then(|v| v.parse::<u64>().ok());

    Ok(SysCfg {
        serial_number:           g("SerialNumber"),
        model_number:            g("ModelNumber"),
        region_info:             g("RegionInfo"),
        hardware_model:          g("HardwareModel"),
        device_color:            g("DeviceColor"),
        enclosure_color:         g("DeviceEnclosureColor"),
        housing_color:           g("DeviceHousingColor"),
        mlb_serial:              g("MLBSerialNumber"),
        config_number:           g("ConfigNumber"),
        original_config_number:  g("OriginalConfigNumber"),
        nand_size_gb:            u32f("NandSizeGB").or_else(|| u32f("DiskUsage")),
        nand_capacity_bytes:     u64f("NandCapacityBytes"),
        ecid:                    u64f("UniqueChipID"),
        board_id:                u32f("BoardId"),
        chip_id:                 u32f("ChipID"),
        wifi_calibration_guid:   g("WifiCalibrationGuid"),
        motion_calibration:      g("MotionCalibration"),
        camera_calibration_size: u32f("CameraCalibrationSize"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_empty() {
        let s = SysCfg::default();
        assert!(s.serial_number.is_none());
        assert!(s.nand_size_gb.is_none());
    }

    #[test]
    fn serialises_to_json() {
        let mut s = SysCfg::default();
        s.serial_number = Some("F2LXXXX".into());
        s.nand_size_gb  = Some(256);
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("F2LXXXX"));
        assert!(j.contains("256"));
    }
}
