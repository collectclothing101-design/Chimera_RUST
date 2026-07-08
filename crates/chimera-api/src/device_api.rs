// chimera-api/src/device_api.rs
// Reverse-engineered device operation API (api.chimeratool.com /v2/operation/*)
// Documents request/response shapes for every operation type.
// ChimeraRS executes all of these locally — no API call needed.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Operations observed in ChimeraTool's operation dispatch API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    // ── Universal ───────────────────────────────────────────────────────
    GetInfo,
    FactoryReset,
    RemoveFrp,
    ResetScreenLock,
    RemoveMdm,
    RemoveDemoMode,
    RepairImei,
    RepairMac,
    UnlockBootloader,
    LockBootloader,
    RebootDevice,
    FlashFirmware,
    // ── Samsung ─────────────────────────────────────────────────────────
    SamsungCscChange,
    SamsungEfsBackup,
    SamsungEfsRestore,
    SamsungKnoxGuardRemove,
    SamsungReadCertificate,
    SamsungWriteCertificate,
    SamsungNetworkFactoryReset,
    SamsungReactivationLockReset,
    SamsungReadCodes,
    SamsungCarrierRelock,
    // ── Xiaomi ──────────────────────────────────────────────────────────
    XiaomiRemoveFrp,
    XiaomiNetworkFactoryReset,
    XiaomiFlashEdl,
    // ── Apple ───────────────────────────────────────────────────────────
    AppleGetInfo,
    AppleFlashIpsw,
    AppleCheckActivationLock,
    AppleBypassActivationLock,
    AppleRemovePasscode,
    AppleCarrierUnlock,
    // ── Network Unlock ───────────────────────────────────────────────────
    CalculateNck,
    NetworkUnlock,
    // ── Firmware ─────────────────────────────────────────────────────────
    FirmwareSearch,
    FirmwareDownload,
}

/// Operation dispatch request (POST api.chimeratool.com/v2/operation/dispatch)
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationDispatchRequest {
    pub operation: OperationType,
    pub device_serial: String,
    pub device_imei: Option<String>,
    pub device_model: Option<String>,
    pub params: HashMap<String, serde_json::Value>,
    pub connection_mode: String, // "adb", "fastboot", "edl", "odin", "lockdown", "recovery"
}

/// Operation dispatch response
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationDispatchResponse {
    pub operation_id: String,
    pub status: OperationStatus,
    pub credits_deducted: u32,
    pub message: Option<String>,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Queued,
    Running,
    Success,
    Failed,
    Cancelled,
    InsufficientCredits,
    UnsupportedDevice,
}

/// Device registration request (POST api.chimeratool.com/v2/device/register)
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceRegisterRequest {
    pub serial: String,
    pub imei: Option<String>,
    pub model: Option<String>,
    pub brand: Option<String>,
    pub android_version: Option<String>,
    pub build_number: Option<String>,
    pub connection_mode: String,
}

/// Result from secure.chimeratool.com /v1/imei/check
#[derive(Debug, Serialize, Deserialize)]
pub struct SecureImeiCheckResult {
    pub imei: String,
    pub valid: bool,
    pub blacklisted: bool,
    pub brand: Option<String>,
    pub model: Option<String>,
    pub country: Option<String>,
    pub carrier: Option<String>,
    pub warranty: Option<bool>,
}

/// FRP bypass ticket from secure.chimeratool.com /v1/frp/ticket
/// ChimeraTool generates server-side keys for certain OEM unlock procedures.
/// ChimeraRS replaces this with local ADB/EDL operations.
#[derive(Debug, Serialize, Deserialize)]
pub struct FrpTicket {
    pub ticket_id: String,
    pub device_serial: String,
    pub oem: String,
    pub operation: String,
    pub signed_payload: Vec<u8>, // AES-256-ECB encrypted
    pub expiry: u64,
}

/// Network unlock code response from secure.chimeratool.com /v1/nck/calculate
/// ChimeraRS replaces this with local NCK algorithms.
#[derive(Debug, Serialize, Deserialize)]
pub struct NckResponse {
    pub imei: String,
    pub carrier_mccmnc: String,
    pub nck1: String,            // Primary unlock code
    pub nck2: Option<String>,    // Secondary (SP lock) code
    pub nsck: Option<String>,    // Network subset code
    pub spck: Option<String>,    // Service provider code
    pub cpck: Option<String>,    // Corporate personalisation code
    pub algorithm: String,       // "samsung_sha256", "lg_sha256", etc.
}
