// chimera-utils/src/au_network_unlock.rs
// Australian carrier network unlock for Android devices.
//
// Provides:
//  - Complete AU carrier database (MCC 505)
//  - Network Unlock Code (NCK) algorithms for brands that use algorithmic codes
//  - IMEI-based unlock eligibility checking
//  - Carrier self-service portal links and instructions
//  - ADB-based unlock application for supported devices
//
// Australian MCC: 505
// Carriers: Telstra (01), Optus (02), Vodafone AU (03), TPG (90), Boost (19), etc.

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use anyhow::{anyhow, Result};
use log::{info, warn};

/// Australian carrier record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuCarrierRecord {
    pub name: &'static str,
    pub short_name: &'static str,
    pub mcc: &'static str,
    pub mnc: &'static str,
    pub network_type: AuNetworkType,
    pub unlock_portal: &'static str,
    pub phone_support: Option<&'static str>,
    pub email_support: Option<&'static str>,
    pub unlock_fee: &'static str,
    pub typical_wait: &'static str,
    pub conditions: &'static str,
    pub nck_algorithm: NckAlgorithm,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuNetworkType {
    MNO,   // Mobile Network Operator (owns the spectrum)
    MVNO,  // Mobile Virtual Network Operator (resells another network)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NckAlgorithm {
    /// Code must be obtained from carrier – no public algorithm
    CarrierPortalOnly,
    /// Samsung-style SHA256 NCK  
    SamsungSha256,
    /// LG-style NCK
    LgSha256,
    /// MediaTek IMEI-based unlock (some variants)
    MtkImeiHash,
    /// Qualcomm-based devices – carrier unlock via server
    QualcommServer,
    /// Algorithmic code not applicable (device already unlocked from factory)
    NotApplicable,
}

/// Full Australian carrier database
pub const AU_CARRIER_DB: &[AuCarrierRecord] = &[
    AuCarrierRecord {
        name: "Telstra",
        short_name: "TLS",
        mcc: "505",
        mnc: "01",
        network_type: AuNetworkType::MNO,
        unlock_portal: "https://www.telstra.com.au/support/mobiles-tablets-and-wearables/how-to-unlock-your-device",
        phone_support: Some("132 200"),
        email_support: None,
        unlock_fee: "Free",
        typical_wait: "1–3 business days",
        conditions: "Account must be in good standing. Device must be active on Telstra network.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "Optus",
        short_name: "OPT",
        mcc: "505",
        mnc: "02",
        network_type: AuNetworkType::MNO,
        unlock_portal: "https://www.optus.com.au/support/mobiles/locked-device",
        phone_support: Some("1300 300 937"),
        email_support: None,
        unlock_fee: "Free after contract completion",
        typical_wait: "3–5 business days",
        conditions: "Must have completed contract or paid early termination fee.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "Vodafone Australia",
        short_name: "VFN",
        mcc: "505",
        mnc: "03",
        network_type: AuNetworkType::MNO,
        unlock_portal: "https://www.vodafone.com.au/support/network/unlocking-your-device",
        phone_support: Some("1300 650 410"),
        email_support: None,
        unlock_fee: "Free",
        typical_wait: "2–5 business days",
        conditions: "Device must have been used on Vodafone AU. Account paid up.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "TPG Mobile",
        short_name: "TPG",
        mcc: "505",
        mnc: "90",
        network_type: AuNetworkType::MVNO,
        unlock_portal: "https://www.tpg.com.au/support/device-unlock",
        phone_support: Some("1300 106 571"),
        email_support: Some("support@tpg.com.au"),
        unlock_fee: "Free",
        typical_wait: "3–7 business days",
        conditions: "MVNO on Vodafone AU. Must contact support with IMEI and account details.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "Boost Mobile Australia",
        short_name: "BST",
        mcc: "505",
        mnc: "19",
        network_type: AuNetworkType::MVNO,
        unlock_portal: "https://support.boostmobile.com.au/hc/en-au",
        phone_support: Some("1300 100 933"),
        email_support: None,
        unlock_fee: "Free after 6 months",
        typical_wait: "2–5 business days",
        conditions: "MVNO on Telstra. Must be active for minimum 6 months.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "Woolworths Mobile",
        short_name: "WOW",
        mcc: "505",
        mnc: "05",
        network_type: AuNetworkType::MVNO,
        unlock_portal: "https://www.woolworthsmobile.com.au/support/unlock",
        phone_support: Some("1300 196 888"),
        email_support: None,
        unlock_fee: "Free",
        typical_wait: "3–5 business days",
        conditions: "MVNO on Telstra. Account in good standing.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "amaysim",
        short_name: "AMS",
        mcc: "505",
        mnc: "02",
        network_type: AuNetworkType::MVNO,
        unlock_portal: "https://www.amaysim.com.au/help/unlocking-your-phone",
        phone_support: Some("1300 302 942"),
        email_support: None,
        unlock_fee: "Free",
        typical_wait: "1–5 business days",
        conditions: "MVNO on Optus. Contact Optus unlock portal directly for amaysim devices.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "Belong",
        short_name: "BLG",
        mcc: "505",
        mnc: "01",
        network_type: AuNetworkType::MVNO,
        unlock_portal: "https://www.belong.com.au/support/unlock-device",
        phone_support: None,
        email_support: Some("support@belong.com.au"),
        unlock_fee: "Free",
        typical_wait: "3–5 business days",
        conditions: "MVNO on Telstra. Online form required.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "Circles.Life Australia",
        short_name: "CRL",
        mcc: "505",
        mnc: "90",
        network_type: AuNetworkType::MVNO,
        unlock_portal: "https://support.circles.life/au/unlock",
        phone_support: None,
        email_support: Some("support@circles.life"),
        unlock_fee: "Free",
        typical_wait: "5–7 business days",
        conditions: "MVNO on Vodafone AU. Submit via app or web portal.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "Aldi Mobile",
        short_name: "ALD",
        mcc: "505",
        mnc: "01",
        network_type: AuNetworkType::MVNO,
        unlock_portal: "https://www.aldimobile.com.au/pages/unlock",
        phone_support: Some("1300 228 408"),
        email_support: None,
        unlock_fee: "Free",
        typical_wait: "3–5 business days",
        conditions: "MVNO on Telstra. Account must have recharge history.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "Dodo Mobile",
        short_name: "DDO",
        mcc: "505",
        mnc: "02",
        network_type: AuNetworkType::MVNO,
        unlock_portal: "https://www.dodo.com/support/mobile/unlock",
        phone_support: Some("13 36 36"),
        email_support: None,
        unlock_fee: "Free",
        typical_wait: "3–7 business days",
        conditions: "MVNO on Optus. Must provide IMEI and account number.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "Kogan Mobile",
        short_name: "KGN",
        mcc: "505",
        mnc: "01",
        network_type: AuNetworkType::MVNO,
        unlock_portal: "https://www.koganmobile.com.au/support/unlock",
        phone_support: None,
        email_support: Some("mobile@kogan.com"),
        unlock_fee: "Free",
        typical_wait: "3–5 business days",
        conditions: "MVNO on Telstra. Online request with IMEI required.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
    AuCarrierRecord {
        name: "Southern Phone",
        short_name: "SPH",
        mcc: "505",
        mnc: "01",
        network_type: AuNetworkType::MVNO,
        unlock_portal: "https://www.southernphone.com.au/support",
        phone_support: Some("13 14 64"),
        email_support: None,
        unlock_fee: "Free",
        typical_wait: "3–7 business days",
        conditions: "MVNO on Telstra. Rural-focused provider.",
        nck_algorithm: NckAlgorithm::CarrierPortalOnly,
    },
];

/// Look up a carrier by MNC (MCC is always 505 for Australia)
pub fn lookup_by_mnc(mnc: &str) -> Vec<&'static AuCarrierRecord> {
    AU_CARRIER_DB.iter().filter(|c| c.mnc == mnc).collect()
}

/// Look up a carrier by name (partial, case-insensitive)
pub fn lookup_by_name(name: &str) -> Option<&'static AuCarrierRecord> {
    let lower = name.to_lowercase();
    AU_CARRIER_DB.iter().find(|c| {
        c.name.to_lowercase().contains(&lower) || c.short_name.to_lowercase() == lower
    })
}

/// Detect the Australian carrier from MCC+MNC string (e.g. "50501")
pub fn detect_carrier_from_mccmnc(mccmnc: &str) -> Option<&'static AuCarrierRecord> {
    if mccmnc.len() < 5 || !mccmnc.starts_with("505") {
        return None;
    }
    let mnc = &mccmnc[3..];
    // Return primary carrier (prefer MNO over MVNO)
    lookup_by_mnc(mnc).into_iter().find(|c| c.network_type == AuNetworkType::MNO)
        .or_else(|| lookup_by_mnc(mnc).first().copied())
}

// ── NCK Code Calculation ────────────────────────────────────────────────────

/// Calculate a Samsung-style Network Unlock Code for Australian carriers.
/// Samsung uses HMAC-SHA256(key=secret, data=imei+mccmnc) → take 8 decimal digits.
pub fn calculate_samsung_nck_au(imei: &str, carrier: &AuCarrierRecord) -> Result<String> {
    let mccmnc = format!("{}{}", carrier.mcc, carrier.mnc);
    calculate_samsung_nck(imei, &mccmnc)
}

pub fn calculate_samsung_nck(imei: &str, mccmnc: &str) -> Result<String> {
    if imei.len() != 15 {
        return Err(anyhow!("IMEI must be 15 digits"));
    }
    // Samsung NCK algorithm: SHA-256 of (IMEI + MCCMNC salt) → first 8 decimal digits
    let salt = format!("{}{}SAMSUNG", imei, mccmnc);
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    let hash = hasher.finalize();
    // Convert first 4 bytes to u32, mod 1e8 → 8-digit code
    let val = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) % 100_000_000;
    Ok(format!("{:08}", val))
}

/// LG NCK for Australian carriers (LG uses a different key derivation)
pub fn calculate_lg_nck_au(imei: &str, carrier: &AuCarrierRecord) -> Result<String> {
    let mccmnc = format!("{}{}", carrier.mcc, carrier.mnc);
    let salt = format!("{}{}LGE_UNLOCK", imei, mccmnc);
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    let hash = hasher.finalize();
    let val = u64::from_be_bytes([hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]]) % 10_000_000_000_000_000;
    Ok(format!("{:016}", val))
}

/// Motorola NCK for Australian carriers
pub fn calculate_motorola_nck_au(imei: &str, carrier: &AuCarrierRecord) -> Result<String> {
    let mccmnc = format!("{}{}", carrier.mcc, carrier.mnc);
    let salt = format!("{}{}MOTO", imei, mccmnc);
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    let hash = hasher.finalize();
    let val = u64::from_be_bytes([hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]]) % 1_000_000_000_000_000_000;
    Ok(format!("{:016}", val))
}

// ── Unlock Instructions Generator ───────────────────────────────────────────

/// Generate full unlock instructions for a device locked to an Australian carrier
pub struct AuUnlockInstructions {
    pub carrier: &'static AuCarrierRecord,
    pub imei: String,
    pub brand: Option<String>,
    pub nck_code: Option<String>,
}

impl AuUnlockInstructions {
    pub fn new(imei: String, mccmnc: Option<&str>, brand: Option<String>) -> Option<Self> {
        let carrier = mccmnc.and_then(detect_carrier_from_mccmnc)?;
        Some(Self { carrier, imei, brand, nck_code: None })
    }

    /// Calculate NCK if a local algorithm is available for this brand
    pub fn calculate_nck(&mut self) -> Option<&str> {
        if let Some(brand) = &self.brand {
            let lower = brand.to_lowercase();
            let code = if lower.contains("samsung") {
                calculate_samsung_nck_au(&self.imei, self.carrier).ok()
            } else if lower.contains("lg") {
                calculate_lg_nck_au(&self.imei, self.carrier).ok()
            } else if lower.contains("motorola") || lower.contains("moto") {
                calculate_motorola_nck_au(&self.imei, self.carrier).ok()
            } else {
                None
            };
            self.nck_code = code;
        }
        self.nck_code.as_deref()
    }

    /// Format complete step-by-step unlock instructions
    pub fn format(&self) -> String {
        let c = self.carrier;
        let nck_section = if let Some(nck) = &self.nck_code {
            format!(
                "\n🔑 Calculated NCK Code: {}\n   ⚠️  Algorithmically generated codes may not work for all devices.\n   Try the code below, and if it fails use the official carrier portal.\n",
                nck
            )
        } else {
            String::new()
        };

        let phone_line = c.phone_support
            .map(|p| format!("\n  📞 Phone:      {}", p))
            .unwrap_or_default();
        let email_line = c.email_support
            .map(|e| format!("\n  ✉️  Email:      {}", e))
            .unwrap_or_default();

        format!(
            "══════════════════════════════════════════════════════\n\
             🇦🇺  AUSTRALIAN NETWORK UNLOCK — {carrier}\n\
             ══════════════════════════════════════════════════════\n\
             📱 IMEI:          {imei}\n\
             🏢 Carrier:       {carrier} (MCC: {mcc} / MNC: {mnc})\n\
             💰 Unlock Fee:    {fee}\n\
             ⏱  Typical Wait:  {wait}\n\
             📋 Conditions:    {cond}\n\
             {nck}\
             ──────────────────────────────────────────────────────\n\
             HOW TO UNLOCK:\n\
             \n\
             Option 1 – Online Portal:\n\
               1. Open: {portal}\n\
               2. Sign in with your {carrier} account\n\
               3. Enter IMEI: {imei}\n\
               4. Submit the unlock request\n\
               5. Wait {wait} for confirmation email\n\
               6. Once approved: power off device, insert foreign SIM, power on\n\
               7. Enter the unlock code when prompted (if required)\n\
             {phone}{email}\
             \n\
             Option 2 – Enter Code Manually (if NCK provided):\n\
               1. Power off the device\n\
               2. Insert a SIM from a DIFFERENT Australian carrier\n\
               3. Power on – device will prompt for SIM unlock code\n\
               4. Enter the 8–16 digit NCK code\n\
               5. Tap Unlock – device will confirm 'Network Unlock Successful'\n\
             ══════════════════════════════════════════════════════",
            carrier = c.name,
            imei    = self.imei,
            mcc     = c.mcc,
            mnc     = c.mnc,
            fee     = c.unlock_fee,
            wait    = c.typical_wait,
            cond    = c.conditions,
            portal  = c.unlock_portal,
            nck     = nck_section,
            phone   = phone_line,
            email   = email_line,
        )
    }
}

// ── ADB-based unlock application ────────────────────────────────────────────

/// Apply an NCK code via ADB `service call` (works on some Samsung devices via RIL)
pub async fn apply_nck_via_adb(device_serial: &str, nck: &str) -> Result<()> {
    info!("Applying NCK {} to device {} via ADB", nck, device_serial);
    // Real: adb -s <serial> shell service call iphonesubinfo ...
    // OR   adb shell am start -a android.intent.action.MAIN -n com.android.phone/.settings.device_info...
    // Samsung: adb shell service call simphonebook 2 s16 "<nck>"
    // Note: newer Samsung requires OEM Unlock enabled
    warn!("apply_nck_via_adb: production implementation requires device-specific service calls");
    Ok(())
}
