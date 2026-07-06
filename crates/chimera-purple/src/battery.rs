//! Battery & thermals — gas-gauge + thermal sensor readout. Driven by the
//! same `idevicediagnostics ioreg -p IOPower` data PurpleSNIFF uses.

use serde::{Serialize, Deserialize};
use crate::Result;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BatteryStats {
    /// % of full charge (0-100).
    pub current_capacity:  Option<u32>,
    /// Designed full charge in mAh (factory spec).
    pub design_capacity:   Option<u32>,
    /// Actual full-charge capacity now in mAh (degrades over cycles).
    pub max_capacity:      Option<u32>,
    /// Discharge cycle count.
    pub cycle_count:       Option<u32>,
    /// Health % = max_capacity / design_capacity × 100.
    pub health_pct:        Option<u32>,
    /// Voltage in mV.
    pub voltage_mv:        Option<u32>,
    /// Current in mA (negative = discharging).
    pub current_ma:        Option<i32>,
    /// Temperature in centi-degrees C (e.g. 2542 = 25.42°C).
    pub temperature_c100:  Option<i32>,
    /// Serial number of the battery pack itself.
    pub battery_serial:    Option<String>,
    /// Charging power state.
    pub is_charging:       Option<bool>,
    /// External adapter connected.
    pub external_connected:Option<bool>,
}

pub fn read(udid: Option<&str>) -> Result<BatteryStats> {
    let raw = chimera_imobile::diagnostics::battery_info(udid)?;
    Ok(parse_ioreg(&raw))
}

fn parse_ioreg(text: &str) -> BatteryStats {
    let mut b = BatteryStats::default();
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("CurrentCapacity = ") {
            b.current_capacity = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("DesignCapacity = ") {
            b.design_capacity = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("MaxCapacity = ") {
            b.max_capacity = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("CycleCount = ") {
            b.cycle_count = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("Voltage = ") {
            b.voltage_mv = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("Amperage = ") {
            b.current_ma = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("Temperature = ") {
            b.temperature_c100 = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("BatterySerialNumber = ") {
            b.battery_serial = Some(v.trim_matches('"').to_string());
        } else if let Some(v) = line.strip_prefix("IsCharging = ") {
            b.is_charging = match v { "true" | "Yes" => Some(true),
                                       "false" | "No" => Some(false), _ => None };
        } else if let Some(v) = line.strip_prefix("ExternalConnected = ") {
            b.external_connected = match v { "true" | "Yes" => Some(true),
                                              "false" | "No" => Some(false), _ => None };
        }
    }
    // Derive health if both numbers present.
    if let (Some(max), Some(design)) = (b.max_capacity, b.design_capacity) {
        if design > 0 {
            b.health_pct = Some((max * 100) / design);
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typical_ioreg_block() {
        let txt = r#"
CurrentCapacity = 87
DesignCapacity = 3274
MaxCapacity = 3120
CycleCount = 412
Voltage = 4180
Amperage = -325
Temperature = 2542
BatterySerialNumber = "DGQ1234567890ABC"
IsCharging = false
ExternalConnected = false
"#;
        let b = parse_ioreg(txt);
        assert_eq!(b.current_capacity, Some(87));
        assert_eq!(b.design_capacity,  Some(3274));
        assert_eq!(b.max_capacity,     Some(3120));
        assert_eq!(b.cycle_count,      Some(412));
        assert_eq!(b.voltage_mv,       Some(4180));
        assert_eq!(b.current_ma,       Some(-325));
        assert_eq!(b.temperature_c100, Some(2542));
        assert_eq!(b.battery_serial.as_deref(), Some("DGQ1234567890ABC"));
        assert_eq!(b.is_charging,      Some(false));
        // Health: 3120 / 3274 * 100 = 95
        assert_eq!(b.health_pct,       Some(95));
    }

    #[test]
    fn empty_text_yields_default() {
        let b = parse_ioreg("");
        assert!(b.current_capacity.is_none());
        assert!(b.health_pct.is_none());
    }
}
