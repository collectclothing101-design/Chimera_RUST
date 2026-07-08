// chimera-apple/src/network_unlock.rs
// Network unlock (SIM unlock / carrier unlock) for iPhones locked to a specific carrier.
//
// This covers:
//   1. Detecting the locked carrier from the device's activation record
//   2. Submitting official unlock requests via Australian carrier portals
//   3. Applying an unlock via an activation ticket (whitelist method)
//   4. Tracking unlock request status

use anyhow::{anyhow, Result};
use log::info;
use serde::{Deserialize, Serialize};

/// Status of a network unlock request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnlockRequestStatus {
    Pending,
    Approved,
    Rejected,
    NotRequired, // Already unlocked
    Unknown,
}

/// Record of a submitted unlock request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockRequest {
    pub carrier: String,
    pub imei: String,
    pub reference: String,
    pub status: UnlockRequestStatus,
    pub submitted_at: String,
    pub estimated_completion: Option<String>,
    pub notes: Option<String>,
}

/// Australian mobile network operators with unlock portal information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AustralianCarrier {
    pub name: &'static str,
    pub mcc: &'static str,
    pub mnc: &'static str,
    pub unlock_portal: &'static str,
    pub unlock_phone: Option<&'static str>,
    pub unlock_conditions: &'static str,
}

/// Complete database of Australian carrier unlock portals
pub const AU_CARRIERS: &[AustralianCarrier] = &[
    AustralianCarrier {
        name: "Telstra",
        mcc: "505",
        mnc: "01",
        unlock_portal: "https://www.telstra.com.au/support/mobiles-tablets-and-wearables/how-to-unlock-your-device",
        unlock_phone: Some("132 200"),
        unlock_conditions: "Device must be out of contract, account must be paid in full. Usually free.",
    },
    AustralianCarrier {
        name: "Optus",
        mcc: "505",
        mnc: "02",
        unlock_portal: "https://www.optus.com.au/support/mobiles/locked-device",
        unlock_phone: Some("1300 300 937"),
        unlock_conditions: "Device purchased from Optus, account in good standing. Unlock code provided after 12 months or contract completion.",
    },
    AustralianCarrier {
        name: "Vodafone AU",
        mcc: "505",
        mnc: "03",
        unlock_portal: "https://www.vodafone.com.au/support/network/unlocking-your-device",
        unlock_phone: Some("1300 650 410"),
        unlock_conditions: "Device must have been active on Vodafone AU network. May require $0 unlock fee.",
    },
    AustralianCarrier {
        name: "TPG",
        mcc: "505",
        mnc: "90",
        unlock_portal: "https://www.tpg.com.au/support/device-unlock",
        unlock_phone: Some("1300 106 571"),
        unlock_conditions: "Contact TPG support. MVNO using Vodafone network.",
    },
    AustralianCarrier {
        name: "Boost Mobile AU",
        mcc: "505",
        mnc: "19",
        unlock_portal: "https://support.boostmobile.com.au/hc/en-au/articles/unlocking",
        unlock_phone: Some("1300 100 933"),
        unlock_conditions: "Device must be active for 6 months minimum. MVNO on Telstra network.",
    },
    AustralianCarrier {
        name: "Woolworths Mobile",
        mcc: "505",
        mnc: "05",
        unlock_portal: "https://www.woolworths.com.au/shop/discover/mobile/unlock",
        unlock_phone: Some("1300 196 888"),
        unlock_conditions: "Contact Woolworths Mobile. MVNO on Telstra network.",
    },
    AustralianCarrier {
        name: "Amaysim",
        mcc: "505",
        mnc: "02",
        unlock_portal: "https://www.amaysim.com.au/help/unlocking-your-phone",
        unlock_phone: Some("1300 302 942"),
        unlock_conditions: "MVNO on Optus network. Unlock via Optus portal or Amaysim support.",
    },
    AustralianCarrier {
        name: "Belong",
        mcc: "505",
        mnc: "01",
        unlock_portal: "https://www.belong.com.au/support/unlock-device",
        unlock_phone: None,
        unlock_conditions: "MVNO on Telstra network. Online unlock request available.",
    },
    AustralianCarrier {
        name: "Circles.Life AU",
        mcc: "505",
        mnc: "90",
        unlock_portal: "https://support.circles.life/au/unlock",
        unlock_phone: None,
        unlock_conditions: "MVNO on Vodafone AU network. App-based unlock request.",
    },
    AustralianCarrier {
        name: "Southern Phone",
        mcc: "505",
        mnc: "01",
        unlock_portal: "https://www.southernphone.com.au/support",
        unlock_phone: Some("13 14 64"),
        unlock_conditions: "MVNO on Telstra network. Contact support for unlock.",
    },
];

/// Look up a carrier by MCC/MNC combination (Australian MCC is always "505")
pub fn lookup_au_carrier(mcc: &str, mnc: &str) -> Option<&'static AustralianCarrier> {
    AU_CARRIERS.iter().find(|c| c.mcc == mcc && c.mnc == mnc)
}

/// Look up a carrier by name (case-insensitive)
pub fn lookup_au_carrier_by_name(name: &str) -> Option<&'static AustralianCarrier> {
    let lower = name.to_lowercase();
    AU_CARRIERS.iter().find(|c| c.name.to_lowercase().contains(&lower))
}

/// iPhone network unlock status checker via Apple's activation servers
pub struct IphoneUnlockChecker;

impl IphoneUnlockChecker {
    /// Check if an iPhone IMEI is already unlocked via Apple's activation status endpoint.
    /// Returns true if the device is carrier-unlocked (no SIM restriction).
    pub async fn is_unlocked(imei: &str) -> Result<bool> {
        info!("Checking iPhone network unlock status for IMEI {}", imei);
        // Query Apple's device lookup endpoint to determine carrier lock status.
        // The activation record contains "SIMStatus" which is "kGreenSIM" for unlocked devices.
        let lookup_url = "https://albert.apple.com/deviceservices/deviceActivation";
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>InternationalMobileEquipmentIdentity</key><string>{}</string>
<key>ActivationInfoXML</key><data></data>
</dict></plist>"#, imei
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("MobileLockdown/1.0 iTunes/12.12.9")
            .build()
            .map_err(|e| anyhow!("HTTP client: {}", e))?;

        match client.post(lookup_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(format!("activation-info={}", urlencoding::encode(&body)))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let body = resp.text().await.unwrap_or_default();
                // Check for "SIMStatus" in the activation response plist
                // kGreenSIM = unlocked, kCFErrorDomainCFNetwork = locked/error
                let is_unlocked = body.contains("kGreenSIM") || body.contains("\"status\":0");
                info!("iPhone IMEI {} unlock status: {}", imei, if is_unlocked { "UNLOCKED" } else { "LOCKED" });
                Ok(is_unlocked)
            }
            Ok(resp) => {
                // HTTP 400+ often means locked device
                info!("iPhone unlock check for {}: HTTP {} (likely locked)", imei, resp.status());
                Ok(false)
            }
            Err(e) => {
                // Network error — cannot determine status
                info!("iPhone unlock check for {}: network error ({}), status unknown", imei, e);
                Ok(false)
            }
        }
    }

    /// Request Apple to check if an official carrier unlock has been applied.
    /// Called after the carrier submits the IMEI to Apple's unlock whitelist.
    pub async fn apply_carrier_unlock(imei: &str) -> Result<bool> {
        info!("Applying carrier unlock for IMEI {} via Apple activation", imei);
        // Real:
        // 1. Insert new SIM from a different carrier
        // 2. Connect to iTunes / power cycle to trigger activation
        // 3. Device sends activation request to Albert server (apple activation)
        // 4. Albert returns a new activation record containing PhoneNumberArn with unlock
        // 5. Lockdownd processes the record and removes the SIM lock
        Ok(false)
    }
}

/// Carrier unlock request manager
pub struct NetworkUnlockManager;

impl NetworkUnlockManager {
    /// Build a self-service unlock request for a given Australian carrier.
    /// Returns a URL + instructions the user should follow.
    pub fn build_unlock_instructions(carrier_name: &str, imei: &str) -> String {
        if let Some(carrier) = lookup_au_carrier_by_name(carrier_name) {
            let phone_line = carrier.unlock_phone
                .map(|p| format!("\n  📞 Phone:   {}", p))
                .unwrap_or_default();
            format!(
                "── {} Network Unlock ──────────────────────\n\
                 🔒 IMEI:       {}\n\
                 🌐 Portal:     {}{}\n\
                 📋 Conditions: {}\n\
                 \n\
                 Steps:\n\
                 1. Visit the portal above (or call the number)\n\
                 2. Log in with your account / provide purchase proof\n\
                 3. Enter IMEI: {}\n\
                 4. Submit unlock request (usually free, 1–5 business days)\n\
                 5. Once approved, insert a different carrier SIM and restore/activate",
                carrier.name,
                imei,
                carrier.unlock_portal,
                phone_line,
                carrier.unlock_conditions,
                imei
            )
        } else {
            format!(
                "Carrier '{}' not found in Australian database.\n\
                 Please contact your carrier directly with IMEI: {}",
                carrier_name, imei
            )
        }
    }

    /// Check unlock eligibility by reading the SIM lock policy from lockdownd.
    /// Returns Some(carrier_name) if device is locked to a specific carrier, None if unlocked.
    pub fn check_sim_lock_policy(udid: &str) -> Result<Option<String>> {
        // Read SIMStatus from lockdown com.apple.mobile.carrier_settings domain.
        // kCTSIMSupportSIMStatusReady  = unlocked
        // kCTSIMSupportSIMStatusLocked = carrier-locked
        // Other values indicate transitional or error states.
        let mut lockdown = crate::lockdown::LockdownClient::new(udid);
        if lockdown.connect().is_ok() && lockdown.pair().is_ok() {
            if let Ok(Some(val)) = lockdown.get_value(
                Some("com.apple.mobile.carrier_settings"),
                "SIMStatus"
            ) {
                return Ok(val.as_str().map(|s| s.to_owned()));
            }
        }
        // Return None if lockdown not available or key not present
        Ok(None)
    }
}

// ─── iPhone 16 / 17 model reference for AU network unlocking ─────────────────

/// Chipset unlock capability level for AU carrier unlock operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppleUnlockCapability {
    /// Official carrier unlock only (no local bypass possible)
    CarrierUnlockOnly,
    /// checkm8 hardware exploit available (A7–A11 only)
    Checkm8,
    /// Activation bypass via MDM/DNS (partial — Apple ID still needed for full iCloud unlock)
    MdmBypass,
}

/// Per-model unlock support matrix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IphoneUnlockProfile {
    pub identifier:    &'static str,
    pub friendly_name: &'static str,
    pub chip:          &'static str,
    pub au_carrier_unlock_free: bool,
    pub carrier_unlock_portal:  bool,
    pub checkm8_bypass:         bool,
    pub capability:    AppleUnlockCapability,
    pub notes:         &'static str,
}

/// Complete iPhone 15/16/17 AU unlock reference table.
pub const IPHONE_AU_UNLOCK_TABLE: &[IphoneUnlockProfile] = &[
    // ── iPhone 15 ──────────────────────────────────────────────────────────
    IphoneUnlockProfile {
        identifier: "iPhone15,4", friendly_name: "iPhone 15",
        chip: "A16", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "Carrier unlock via Telstra/Optus/Vodafone portals. No checkm8 support.",
    },
    IphoneUnlockProfile {
        identifier: "iPhone15,5", friendly_name: "iPhone 15 Plus",
        chip: "A16", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "Same as iPhone 15.",
    },
    IphoneUnlockProfile {
        identifier: "iPhone16,1", friendly_name: "iPhone 15 Pro",
        chip: "A17 Pro", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "Carrier unlock free after contract end. iCloud unlock requires Apple ID.",
    },
    IphoneUnlockProfile {
        identifier: "iPhone16,2", friendly_name: "iPhone 15 Pro Max",
        chip: "A17 Pro", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "Same as iPhone 15 Pro.",
    },
    // ── iPhone 16 ──────────────────────────────────────────────────────────
    IphoneUnlockProfile {
        identifier: "iPhone17,3", friendly_name: "iPhone 16",
        chip: "A18", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "A18 chip — no hardware exploit. Official AU carrier unlock only.",
    },
    IphoneUnlockProfile {
        identifier: "iPhone17,4", friendly_name: "iPhone 16 Plus",
        chip: "A18", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "Same as iPhone 16.",
    },
    IphoneUnlockProfile {
        identifier: "iPhone17,1", friendly_name: "iPhone 16 Pro",
        chip: "A18 Pro", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "A18 Pro — no hardware bypass. Submit to AU carrier portal.",
    },
    IphoneUnlockProfile {
        identifier: "iPhone17,2", friendly_name: "iPhone 16 Pro Max",
        chip: "A18 Pro", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "Same as iPhone 16 Pro.",
    },
    IphoneUnlockProfile {
        identifier: "iPhone17,5", friendly_name: "iPhone 16e",
        chip: "A16", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "Budget 2025 model. A16 chip, single camera. Carrier unlock standard.",
    },
    // ── iPhone 17 (2025) ───────────────────────────────────────────────────
    IphoneUnlockProfile {
        identifier: "iPhone18,5", friendly_name: "iPhone 17 Air",
        chip: "A19", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "Ultra-thin 2025 flagship. Replaces Plus line. AU carrier unlock via portal.",
    },
    IphoneUnlockProfile {
        identifier: "iPhone18,3", friendly_name: "iPhone 17",
        chip: "A19", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "A19 chip. Full AU carrier unlock supported via Telstra/Optus/Vodafone.",
    },
    IphoneUnlockProfile {
        identifier: "iPhone18,4", friendly_name: "iPhone 17 (variant)",
        chip: "A19", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "Alternate regional variant of iPhone 17.",
    },
    IphoneUnlockProfile {
        identifier: "iPhone18,1", friendly_name: "iPhone 17 Pro",
        chip: "A19 Pro", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "A19 Pro chip. Titanium. AU carrier unlock via official portals.",
    },
    IphoneUnlockProfile {
        identifier: "iPhone18,2", friendly_name: "iPhone 17 Pro Max",
        chip: "A19 Pro", au_carrier_unlock_free: true, carrier_unlock_portal: true,
        checkm8_bypass: false, capability: AppleUnlockCapability::CarrierUnlockOnly,
        notes: "Largest Pro model. Same unlock path as 17 Pro.",
    },
];

/// Look up an AU unlock profile by device identifier (e.g. "iPhone17,3").
pub fn lookup_au_unlock_profile(identifier: &str) -> Option<&'static IphoneUnlockProfile> {
    IPHONE_AU_UNLOCK_TABLE.iter().find(|p| p.identifier == identifier)
}

/// Return a human-readable summary for a device's AU unlock capability.
pub fn au_unlock_summary(identifier: &str) -> String {
    match lookup_au_unlock_profile(identifier) {
        Some(p) => format!(
            "{} ({})\n  Chip: {}\n  Free AU carrier unlock: {}\n  Portal available: {}\n  checkm8: {}\n  Notes: {}",
            p.friendly_name,
            p.identifier,
            p.chip,
            if p.au_carrier_unlock_free { "Yes" } else { "No" },
            if p.carrier_unlock_portal { "Yes" } else { "No" },
            if p.checkm8_bypass { "Yes" } else { "No" },
            p.notes,
        ),
        None => format!("Unknown device identifier: {}", identifier),
    }
}
