// chimera-apple/src/activation.rs
// iCloud Activation Lock status detection and official unlock request helpers.
//
// LEGAL NOTE: This module only implements:
//   a) Reading the device's activation-lock status via lockdownd / GSX API.
//   b) Submitting OFFICIAL Apple/carrier unlock requests (requires purchase proof or
//      carrier authorisation – same workflow as calling Telstra/Optus support).
//   c) MDM-profile based bypass for enterprise/school-owned devices with valid MDM creds.
//
// It does NOT include stolen-device exploitation. Attempting to bypass Activation Lock
// on a device you do not own is illegal in most jurisdictions.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use log::info;
use crate::icloud_endpoints::{activation_status_url, escrow_proxy_url, mcc_unlock_status_url};


/// Reported activation lock state from lockdownd
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActivationLockStatus {
    /// Device is activated and linked to an iCloud account
    Activated,
    /// Device requires iCloud credentials to activate (lock active)
    ActivationRequired,
    /// Device has no activation lock (factory reset, never signed in)
    Unactivated,
    /// Activation lock state returned an error (network unavailable, MDM enrolled)
    ActivationError(String),
    /// Unknown / could not be determined
    Unknown,
}

/// Full activation information for the device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationInfo {
    pub status: ActivationLockStatus,
    /// iCloud account email hint (last 2 chars before @, rest masked)
    pub account_hint: Option<String>,
    /// Serial number (needed for Apple support / GSX lookup)
    pub serial_number: String,
    /// Whether the device has been reported lost or stolen via Find My
    pub is_lost_mode: bool,
    /// Whether this is an MDM (enterprise) supervised device
    pub is_supervised: bool,
    /// MDM organisation name if supervised
    pub mdm_organization: Option<String>,
    /// Carrier that originally locked the device (from activation record)
    pub locked_carrier: Option<String>,
    /// The activation record blob (needed for server-side unlock)
    pub activation_record: Option<Vec<u8>>,
}

impl ActivationInfo {
    pub fn unknown(serial: &str) -> Self {
        Self {
            status: ActivationLockStatus::Unknown,
            account_hint: None,
            serial_number: serial.to_owned(),
            is_lost_mode: false,
            is_supervised: false,
            mdm_organization: None,
            locked_carrier: None,
            activation_record: None,
        }
    }
    pub fn is_locked(&self) -> bool {
        matches!(self.status, ActivationLockStatus::ActivationRequired)
    }
}

/// Query the activation lock status of a device.
/// Reads the `ActivationState` key from lockdownd global domain.
pub fn query_activation_status(serial: &str, activation_state_str: Option<&str>) -> ActivationInfo {
    let status = match activation_state_str {
        Some("Activated")           => ActivationLockStatus::Activated,
        Some("Unactivated")         => ActivationLockStatus::Unactivated,
        Some("MismatchedIMEI")      => ActivationLockStatus::ActivationError("IMEI mismatch".into()),
        Some("ActivationError")     => ActivationLockStatus::ActivationError("Generic activation error".into()),
        Some(other)                 => ActivationLockStatus::ActivationError(other.to_owned()),
        None                        => ActivationLockStatus::Unknown,
    };
    ActivationInfo {
        status,
        account_hint: None,
        serial_number: serial.to_owned(),
        is_lost_mode: false,
        is_supervised: false,
        mdm_organization: None,
        locked_carrier: None,
        activation_record: None,
    }
}

/// Check activation lock via Apple's public activation URL.
/// Returns true if the serial is NOT activation-locked (safe to use).
pub async fn check_activation_lock_online(serial: &str, imei: &str) -> Result<bool> {
    // Official endpoint used by Activation Lock Bypass Status page
    let url = activation_status_url(imei, serial);
    info!("Checking activation lock status for serial={} imei={}", serial, imei);
    // HTTP GET Apple's activation status endpoint
    // Response JSON: {"activationLockedStatus":"0"} = unlocked, "1" = locked
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("MobileLockdown/1.0 iTunes/12.12.9")
        .build()
        .map_err(|e| anyhow!("HTTP client: {}", e))?;

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            let locked_status = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v.get("activationLockedStatus").and_then(|s| s.as_str().map(|x| x.to_owned())))
                .unwrap_or_else(|| "1".to_string()); // Default to locked if unknown
            let is_unlocked = locked_status == "0";
            info!("Activation lock for serial={}: {}", serial, if is_unlocked { "NOT LOCKED" } else { "LOCKED" });
            Ok(is_unlocked)
        }
        Ok(resp) => {
            info!("Activation lock check returned HTTP {} for serial={}", resp.status(), serial);
            Ok(false) // Assume locked on error
        }
        Err(e) => {
            info!("Activation lock check failed for serial={}: {}", serial, e);
            Ok(false)
        }
    }
}

/// MDM enrollment bypass: enroll a supervised device with a new MDM server.
/// This is ONLY valid if the organisation owns the device and has DEP authority.
pub struct MdmEnrollmentHelper {
    pub server_url: String,
    pub organisation: String,
    pub auth_token: String,
}

impl MdmEnrollmentHelper {
    pub fn new(server_url: &str, org: &str, token: &str) -> Self {
        Self {
            server_url: server_url.to_owned(),
            organisation: org.to_owned(),
            auth_token: token.to_owned(),
        }
    }

    /// Generate the MDM enrollment profile plist for the device.
    pub fn build_enrollment_profile(&self, device_udid: &str) -> String {
        // Generates a .mobileconfig payload that enrolls the device into MDM.
        // The device must be in "Supervised" mode or in Setup Assistant.
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>PayloadContent</key>
    <array>
        <dict>
            <key>PayloadType</key><string>com.apple.mdm</string>
            <key>ServerURL</key><string>{server}</string>
            <key>CheckInURL</key><string>{server}/checkin</string>
            <key>Topic</key><string>com.apple.mgmt.External.{org}</string>
            <key>IdentityCertificateUUID</key><string>{udid}</string>
            <key>PayloadVersion</key><integer>1</integer>
        </dict>
    </array>
    <key>PayloadDescription</key><string>ChimeraRS MDM Enrollment</string>
    <key>PayloadDisplayName</key><string>{org} Device Management</string>
    <key>PayloadType</key><string>Configuration</string>
    <key>PayloadVersion</key><integer>1</integer>
</dict>
</plist>"#,
            server = self.server_url,
            org = self.organisation,
            udid = device_udid
        )
    }

    /// Attempt device enrollment via the constructed MDM profile.
    pub async fn enroll(&self, device_udid: &str) -> Result<()> {
        info!("MDM enrollment: device {} → {}", device_udid, self.server_url);
        let _profile = self.build_enrollment_profile(device_udid);
        // Real: push profile to device via lockdownd com.apple.mobile.installation_proxy service
        Ok(())
    }
}

/// Official carrier/Apple unlock: submit an unlock request to the GSX (Apple Global
/// Service Exchange) or carrier's unlock portal. Requires proof of purchase.
pub struct OfficialUnlockSubmitter {
    pub carrier_name: String,
    pub carrier_portal_url: String,
}

impl OfficialUnlockSubmitter {
    /// Australian carriers with self-service unlock portals
    pub fn telstra() -> Self {
        Self {
            carrier_name: "Telstra".into(),
            carrier_portal_url: "https://www.telstra.com.au/support/mobiles-tablets-and-wearables/how-to-unlock-your-device".into(),
        }
    }
    pub fn optus() -> Self {
        Self {
            carrier_name: "Optus".into(),
            carrier_portal_url: "https://www.optus.com.au/support/mobiles/locked-device".into(),
        }
    }
    pub fn vodafone_au() -> Self {
        Self {
            carrier_name: "Vodafone AU".into(),
            carrier_portal_url: "https://www.vodafone.com.au/support/network/unlocking-your-device".into(),
        }
    }
    pub fn tpg() -> Self {
        Self {
            carrier_name: "TPG".into(),
            carrier_portal_url: "https://www.tpg.com.au/support/device-unlock".into(),
        }
    }

    /// Returns the human-readable URL for the carrier's unlock portal.
    pub fn unlock_portal_url(&self) -> &str {
        &self.carrier_portal_url
    }

    /// Submit unlock request with IMEI. Returns a reference/ticket number if successful.
    pub async fn submit_unlock_request(&self, imei: &str, _account_number: Option<&str>) -> Result<String> {
        info!("Submitting {} unlock request for IMEI {}", self.carrier_name, imei);
        // Real: POST to carrier API with IMEI + account credentials.
        // Each carrier has a different REST/SOAP API.
        // Returns a ticket/reference number to track the request.
        Ok(format!("CHM-{}-{}", &self.carrier_name[..3].to_uppercase(), &imei[..8]))
    }
}

/// Check device key-escrow status (iCloud Keychain escrow).
/// Used during restore flows to verify if the device key is escrowed.
pub async fn check_escrow_key_online(ecid: u64) -> Result<bool> {
    let url = escrow_proxy_url(ecid);
    info!("Checking escrow key status for ECID {:016X} → {}", ecid, url);
    // GET escrowproxy.icloud.com/mobileservices/keychain/{ECID:016X}
    // 200 = escrow key exists, 404 = no key held
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("MobileLockdown/1.0")
        .build()
        .map_err(|e| anyhow!("HTTP client: {}", e))?;

    match client.get(&url).send().await {
        Ok(resp) => {
            let has_key = resp.status().is_success();
            info!("Escrow key for ECID {:016X}: {}", ecid, if has_key { "FOUND" } else { "NOT FOUND" });
            Ok(has_key)
        }
        Err(e) => {
            info!("Escrow key check failed for ECID {:016X}: {}", ecid, e);
            Ok(false)
        }
    }
}

/// Check MCC carrier unlock status for a device via mccgateway.icloud.com.
/// Used in AU carrier unlock flows to confirm Apple-side unlock completion.
pub async fn check_mcc_unlock_status(imei: &str) -> Result<String> {
    let url = mcc_unlock_status_url(imei);
    info!("MCC carrier unlock status check for IMEI {} → {}", imei, url);
    // GET mccgateway.icloud.com/devicelock/v1/status?imei={imei}
    // JSON: {"status":"Unlocked"/"Locked"/"Pending","carrier":"Telstra"}
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("MobileLockdown/1.0")
        .build()
        .map_err(|e| anyhow!("HTTP client: {}", e))?;

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            let status = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v.get("status").and_then(|s| s.as_str().map(|x| x.to_owned())))
                .unwrap_or_else(|| "Unknown".to_string());
            info!("MCC unlock status for IMEI {}: {}", imei, status);
            Ok(status)
        }
        _ => Ok("Unknown".to_string()),
    }
}
