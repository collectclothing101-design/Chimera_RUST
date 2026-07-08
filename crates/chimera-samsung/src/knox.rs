//! Samsung **Knox** state + **Knoxguard** + **Common Criteria** mode
//! management. These are Samsung's enterprise-grade security states:
//!
//! - **Knox warranty bit**: a one-way OTP fuse — once tripped, certain
//!   Knox features (Secure Folder, Samsung Pay, Samsung Health Monitor,
//!   Knox Mobile Enrolment) stop working. We expose a *read* function to
//!   surface the current state; the fuse itself cannot be reset.
//!
//! - **Knoxguard**: Samsung's anti-theft "device pawned" lock. Used by
//!   carriers to brick devices reported lost/stolen. ChimeraTool's
//!   "Knoxguard Remove" works through EUB mode up to the Feb 2024 patch.
//!
//! - **Common Criteria (CC) mode**: a hardened-runtime mode required by
//!   some government / enterprise deployments. Adds password-quality
//!   requirements and additional crypto-policy enforcement.
//!
//! - **Lost Mode**: remote lock activated via Samsung Find My Mobile.
//!
//! - **Bootloader-unlock warnings**: the orange-banner warning that
//!   appears at boot when the bootloader has been unlocked.

use serde::{Serialize, Deserialize};
use chimera_core::error::{ChimeraError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnoxWarrantyState {
    /// Knox warranty bit untripped — full Knox functionality available.
    Untripped,
    /// Knox warranty bit tripped — Secure Folder + Samsung Pay disabled.
    /// One-way and IRREVERSIBLE; this read-out is informational only.
    Tripped,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnoxguardState {
    /// Device is not enrolled in Knoxguard.
    Disabled,
    /// Device is enrolled but not currently locked.
    EnrolledUnlocked,
    /// Device is enrolled AND remote-locked (the "this device has been
    /// reported lost" red banner).
    EnrolledLocked,
    Unknown,
}

/// Full Knox status snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnoxStatus {
    pub warranty:           KnoxWarrantyState,
    pub knoxguard:          KnoxguardState,
    pub common_criteria_on: bool,
    pub lost_mode_on:       bool,
    pub bootloader_unlocked:bool,
    pub frp_enabled:        bool,
    pub kg_version:         Option<String>,
    pub kpe_supported:      bool,    // Knox Platform for Enterprise
    pub knox_version:       Option<String>,
}

impl Default for KnoxStatus {
    fn default() -> Self {
        Self {
            warranty:            KnoxWarrantyState::Unknown,
            knoxguard:           KnoxguardState::Unknown,
            common_criteria_on:  false,
            lost_mode_on:        false,
            bootloader_unlocked: false,
            frp_enabled:         false,
            kg_version:          None,
            kpe_supported:       false,
            knox_version:        None,
        }
    }
}

/// Parse `getprop` output into a typed KnoxStatus. The parser is tolerant
/// of property-key changes across firmware generations.
pub fn parse_getprop(getprop_output: &str) -> KnoxStatus {
    let mut s = KnoxStatus::default();
    for line in getprop_output.lines() {
        let line = line.trim();
        // Expected format:  [property.name]: [value]
        if let Some(eq) = line.find("]: [") {
            let key = &line[1..eq];
            let val = line[eq + 4..].trim_end_matches(']');
            match key {
                "ro.boot.warranty_bit" | "ro.warranty_bit" => {
                    s.warranty = if val == "0" { KnoxWarrantyState::Untripped }
                                 else if val == "1" { KnoxWarrantyState::Tripped }
                                 else { KnoxWarrantyState::Unknown };
                }
                "ro.boot.flash.locked" | "ro.boot.veritymode" => {
                    s.bootloader_unlocked = val == "0" || val == "unlocked"
                                              || val.eq_ignore_ascii_case("disabled");
                }
                "ro.boot.knox_factory_state" => {
                    // factory state value mirrors Knoxguard activation
                    s.knoxguard = match val {
                        "0"            => KnoxguardState::Disabled,
                        "1"            => KnoxguardState::EnrolledUnlocked,
                        "2"            => KnoxguardState::EnrolledLocked,
                        _              => KnoxguardState::Unknown,
                    };
                }
                "ro.security.cc.mode" | "persist.security.cc.enabled" => {
                    s.common_criteria_on = val == "1" || val == "true";
                }
                "ro.security.fmm.lock_state" | "persist.sys.find_my_mobile_lock" => {
                    s.lost_mode_on = val == "1" || val == "true";
                }
                "ro.security.knox.cert_revoked" |
                "persist.sys.frp.lock_status" => {
                    s.frp_enabled = val == "1" || val == "true";
                }
                "ro.boot.knoxguard_version" |
                "ro.security.knoxguard.ver" => {
                    s.kg_version = Some(val.to_string());
                }
                "ro.config.knox" => {
                    s.knox_version = Some(val.to_string());
                    s.kpe_supported = val.contains("v3") || val.contains("v4");
                }
                _ => {}
            }
        }
    }
    s
}

/// Refuse to assist if the Knox warranty bit is already tripped — surfaced
/// to the user so they know operations like "Reset Knox State" are
/// scientifically impossible (one-way fuse).
pub fn require_warranty_untripped(s: &KnoxStatus) -> Result<()> {
    if s.warranty == KnoxWarrantyState::Tripped {
        return Err(ChimeraError::Unknown(
            "Knox warranty bit is already tripped (one-way fuse). \
             Operations that depend on a clean Knox state cannot proceed.".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const SAMPLE: &str = r#"
[ro.boot.warranty_bit]: [0]
[ro.boot.flash.locked]: [1]
[ro.boot.knox_factory_state]: [0]
[ro.security.cc.mode]: [0]
[ro.security.fmm.lock_state]: [0]
[persist.sys.frp.lock_status]: [0]
[ro.config.knox]: [v33]
"#;

    #[test]
    fn parses_untripped_locked() {
        let s = parse_getprop(SAMPLE);
        assert_eq!(s.warranty, KnoxWarrantyState::Untripped);
        assert!(!s.bootloader_unlocked);
        assert_eq!(s.knoxguard, KnoxguardState::Disabled);
        assert!(!s.common_criteria_on);
    }

    #[test]
    fn detects_tripped_warranty() {
        let s = parse_getprop("[ro.boot.warranty_bit]: [1]\n");
        assert_eq!(s.warranty, KnoxWarrantyState::Tripped);
        assert!(require_warranty_untripped(&s).is_err());
    }

    #[test]
    fn detects_unlocked_bootloader() {
        let s = parse_getprop("[ro.boot.flash.locked]: [0]\n");
        assert!(s.bootloader_unlocked);
    }

    #[test]
    fn detects_knoxguard_locked() {
        let s = parse_getprop("[ro.boot.knox_factory_state]: [2]\n");
        assert_eq!(s.knoxguard, KnoxguardState::EnrolledLocked);
    }
}
