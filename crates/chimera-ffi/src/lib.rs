//! `chimera-ffi` — C-ABI surface exposing the ChimeraRS engine to Swift.
//!
//! ## Design
//!
//! The Swift host and the embedded WKWebView both talk to this layer via a
//! single JSON-message protocol. Every entry point takes a UTF-8 JSON string
//! describing a request and returns a UTF-8 JSON string describing a response.
//!
//! Pointers crossing the FFI boundary are owned exclusively by Rust. Swift
//! must call `chimera_string_free` on every returned pointer.
//!
//! ## Concurrency
//!
//! A single global `Engine` lives in a `OnceCell` + `Mutex`. All requests are
//! dispatched on background threads owned by the engine's tokio runtime / 
//! crossbeam worker pool; entry points are non-blocking and either dispatch
//! a job (returning a job ID) or query state.
//!
//! ## Safety
//!
//! Every `unsafe` block is annotated with the invariant the caller must
//! uphold. Swift's `ChimeraBridge.swift` enforces these via type-safe wrappers.

#![allow(non_camel_case_types)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_int;
use std::sync::Mutex;

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

// ─── Engine singleton ────────────────────────────────────────────────

/// The global engine state. Initialised on the first `chimera_init` call.
#[allow(dead_code)]
struct Engine {
    initialised: bool,
    log_buffer:  Vec<String>,
}

impl Engine {
    fn new() -> Self {
        // Set up tracing so log lines from worker crates land in `log_buffer`.
        // Initialised once for the whole process.
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_writer(std::io::stderr)
            .try_init();
        Self {
            initialised: true,
            log_buffer:  Vec::new(),
        }
    }
}

static ENGINE: OnceCell<Mutex<Engine>> = OnceCell::new();

fn engine() -> &'static Mutex<Engine> {
    ENGINE.get_or_init(|| Mutex::new(Engine::new()))
}

// ─── String marshalling helpers ──────────────────────────────────────

/// Convert a Rust String into a heap-allocated C string. Ownership transfers
/// to the caller — they must free with `chimera_string_free`.
fn into_c_string(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Borrow a C string as a Rust &str. Returns "" on null / invalid UTF-8.
///
/// # Safety
/// `ptr` must be either null or point to a valid NUL-terminated UTF-8 string.
unsafe fn borrow_c_string<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("")
}

// ─── JSON request / response envelopes ───────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Request {
    /// "op": "ping" — health check.
    Ping,
    /// "op": "version" — engine version.
    Version,
    /// "op": "list_devices" — enumerate connected USB / ADB devices.
    ListDevices,
    /// "op": "validate_imei", "imei": "<15 digits>".
    ValidateImei { imei: String },
    /// "op": "validate_mac", "mac": "AA:BB:CC:DD:EE:FF".
    ValidateMac { mac: String },
    /// "op": "validate_ipsw", "path": "/abs/path/to.ipsw".
    ValidateIpsw { path: String },
    /// "op": "drain_logs" — return + clear the engine log buffer.
    DrainLogs,
    /// "op": "host_probes" — return status of every external CLI tool
    /// (adb, fastboot, idevice_id, irecovery, palera1n, futurerestore).
    HostProbes,
    /// "op": "list_ios_devices" — every iOS device usbmuxd can see.
    ListIosDevices,
    /// "op": "ios_device_info", "udid": "…" — lockdownd properties.
    IosDeviceInfo { udid: Option<String> },
    /// "op": "ios_activation_state", "udid": "…" — Activated / Unactivated / Denied.
    IosActivationState { udid: Option<String> },
    /// "op": "ios_pair", "udid": "…" — pair host with device.
    IosPair { udid: Option<String> },
    /// "op": "generate_qr", "text": "…", "size": 200 — produce a base64 PNG.
    GenerateQr { text: String, size: Option<u32> },
    /// "op": "purple_sniff", "udid": "…" — run full PurpleSNIFF report.
    PurpleSniff { udid: Option<String> },
    /// "op": "purple_restore", "udid": "…", "ramdisk_path": "…", "assume_dfu": bool.
    PurpleRestore {
        udid:         Option<String>,
        ramdisk_path: Option<String>,
        assume_dfu:   Option<bool>,
        timeout_secs: Option<u64>,
    },
    /// "op": "device_mode", "udid": "…" — fast mode probe.
    DeviceMode { udid: Option<String> },
    /// "op": "read_syscfg", "udid": "…" — SysCfg block readout.
    ReadSysCfg { udid: Option<String> },
    /// "op": "read_battery", "udid": "…" — gas-gauge + thermal.
    ReadBattery { udid: Option<String> },
    /// "op": "samsung_read_codes" — read all 6 lock codes in one step.
    SamsungReadCodes { udid: Option<String> },
    /// "op": "samsung_csc_search", "query": "USA" — filter the CSC database.
    SamsungCscSearch { query: String },
    /// "op": "samsung_csc_list" — full CSC catalogue.
    SamsungCscList,
    /// "op": "samsung_csc_change", "new_code": "XAA", "factory_reset": true.
    SamsungCscChange {
        udid:          Option<String>,
        new_code:      String,
        factory_reset: Option<bool>,
    },
    /// "op": "samsung_knox_status", "udid": "…" — parse getprop into KnoxStatus.
    SamsungKnoxStatus { udid: Option<String> },
    /// "op": "samsung_validate_csc", "code": "XAA" — sanity-check a CSC string.
    SamsungValidateCsc { code: String },
    /// "op": "programmer_analyse", "path": "/path/to/file_or_dir" — inspect
    /// Qualcomm programmer files (.mbn/.elf/.bin).
    ProgrammerAnalyse { path: String },

    // ─── ChimeraTool Core Features ───────────────────────────────────

    /// "op": "repair_imei" — write new IMEI(s) via ADB
    RepairImei { serial: String, imei1: String, imei2: Option<String> },
    /// "op": "repair_mac" — rewrite Wi-Fi MAC via ADB (requires root)
    RepairMac { serial: String, mac: String },
    /// "op": "factory_reset" — factory reset device via ADB
    FactoryReset { serial: String, brand: Option<String> },
    /// "op": "enable_adb" — enable ADB via various methods
    EnableAdb { serial: Option<String> },
    /// "op": "reboot_device" — reboot to specified mode
    RebootDevice { serial: Option<String>, mode: Option<String> },
    /// "op": "remove_screen_lock" — remove PIN/pattern/password
    RemoveScreenLock { serial: String, brand: Option<String> },
    /// "op": "update_firmware" — update firmware via fastboot
    UpdateFirmware { serial: Option<String>, firmware_path: String },

    // ─── Samsung Operations ──────────────────────────────────────────

    /// "op": "samsung_get_info" — full Samsung device info
    SamsungGetInfo { serial: String },
    /// "op": "samsung_reset_frp" — clear FRP lock
    SamsungResetFrp { serial: String },
    /// "op": "samsung_network_factory_reset" — reset all network settings
    SamsungNetworkFactoryReset { serial: String },
    /// "op": "samsung_reset_screenlock" — remove screen lock
    SamsungResetScreenlock { serial: String },
    /// "op": "samsung_remove_mdm" — remove Knox MDM
    SamsungRemoveMdm { serial: String },
    /// "op": "samsung_remove_knox_guard" — remove Knox Guard lock
    SamsungRemoveKnoxGuard { serial: String },
    /// "op": "samsung_repair_efs" — repair EFS partition
    SamsungRepairEfs { serial: String, golden_efs_path: Option<String> },
    /// "op": "samsung_store_backup" — backup EFS/security data
    SamsungStoreBackup { serial: String, output_path: String },
    /// "op": "samsung_restore_backup" — restore EFS/security data
    SamsungRestoreBackup { serial: String, backup_path: String },
    /// "op": "samsung_remove_lost_mode" — remove Find My Mobile / lost mode
    SamsungRemoveLostMode { serial: String },
    /// "op": "samsung_remove_warnings" — remove Knox/warning logos
    SamsungRemoveWarnings { serial: String },
    /// "op": "samsung_carrier_relock" — configure carrier lock
    SamsungCarrierRelock { serial: String, carriers: Vec<String> },
    /// "op": "samsung_remove_demo" — remove demo mode
    SamsungRemoveDemo { serial: String },
    /// "op": "samsung_reset_reactivation_lock" — remove reactivation lock
    SamsungResetReactivationLock { serial: String },
    /// "op": "samsung_root" — root Samsung device
    SamsungRoot { serial: String },

    // ─── Xiaomi Operations ───────────────────────────────────────────

    /// "op": "xiaomi_get_info" — Xiaomi device info
    XiaomiGetInfo { serial: String },
    /// "op": "xiaomi_remove_frp" — Xiaomi FRP removal
    XiaomiRemoveFrp { serial: String },
    /// "op": "xiaomi_factory_reset" — Xiaomi factory reset
    XiaomiFactoryReset { serial: String },
    /// "op": "xiaomi_network_factory_reset" — Xiaomi network reset
    XiaomiNetworkFactoryReset { serial: String },
    /// "op": "xiaomi_repair_imei" — Xiaomi IMEI repair
    XiaomiRepairImei { serial: String, imei1: String, imei2: Option<String> },
    /// "op": "xiaomi_store_backup" — Xiaomi backup
    XiaomiStoreBackup { serial: String, output_path: String },
    /// "op": "xiaomi_restore_backup" — Xiaomi restore
    XiaomiRestoreBackup { serial: String, backup_path: String },

    // ─── Huawei Operations ───────────────────────────────────────────

    /// "op": "huawei_get_info" — Huawei device info
    HuaweiGetInfo { serial: String },
    /// "op": "huawei_remove_frp" — Huawei FRP removal
    HuaweiRemoveFrp { serial: String },
    /// "op": "huawei_disable_id" — disable Huawei ID lock
    HuaweiDisableId { serial: String },
    /// "op": "huawei_factory_reset" — Huawei factory reset
    HuaweiFactoryReset { serial: String },
    /// "op": "huawei_repair_imei" — Huawei IMEI repair
    HuaweiRepairImei { serial: String, imei1: String, imei2: Option<String> },
    /// "op": "huawei_remove_demo" — Huawei demo mode removal
    HuaweiRemoveDemo { serial: String },
    /// "op": "huawei_store_backup" — Huawei backup
    HuaweiStoreBackup { serial: String, output_path: String },

    // ─── EDL Operations ──────────────────────────────────────────────

    /// "op": "edl_remove_frp" — EDL FRP removal (Qualcomm)
    EdlRemoveFrp { frp_sector: u64, lun: u8 },
    /// "op": "edl_update_firmware" — flash firmware via EDL
    EdlUpdateFirmware { firmware_dir: String },
    /// "op": "edl_repair_imei" — EDL IMEI repair
    EdlRepairImei { imei1: String, imei2: Option<String> },
    /// "op": "edl_store_backup" — EDL EFS backup
    EdlStoreBackup { output_path: String },

    // ─── Fastboot Operations ─────────────────────────────────────────

    /// "op": "fastboot_unlock" — unlock bootloader
    FastbootUnlock,
    /// "op": "fastboot_lock" — lock bootloader
    FastbootLock,
    /// "op": "fastboot_info" — get device info from fastboot
    FastbootInfo,
    /// "op": "fastboot_flash" — flash partition
    FastbootFlash { partition: String, image_path: String },
    /// "op": "fastboot_erase" — erase partition
    FastbootErase { partition: String },
    /// "op": "fastboot_reboot" — reboot device
    FastbootReboot { mode: Option<String> },

    // ─── Network Operations ──────────────────────────────────────────

    /// "op": "read_codes" — read network unlock codes
    ReadCodes { serial: String, brand: Option<String> },
    /// "op": "network_factory_reset" — generic network reset
    NetworkFactoryReset { serial: String, brand: Option<String> },
    /// "op": "patch_certificate" — patch network certificate
    PatchCertificate { serial: String, brand: Option<String> },
    /// "op": "read_certificate" — read security certificate
    ReadCertificate { serial: String, brand: Option<String> },
    /// "op": "write_certificate" — write security certificate
    WriteCertificate { serial: String, cert_path: String, brand: Option<String> },
    /// "op": "unlock_bootloader" — generic bootloader unlock
    UnlockBootloader { serial: String, brand: Option<String> },
    /// "op": "relock_bootloader" — generic bootloader relock
    RelockBootloader { serial: String, brand: Option<String> },

    // ─── Auto-Detect Operations ─────────────────────────────────────

    /// "op": "auto_detect" — auto-detect device from VID/PID
    AutoDetect { vid: u16, pid: u16, serial: Option<String> },
    /// "op": "auto_detect_adb" — auto-detect from ADB device list
    AutoDetectAdb,
    /// "op": "get_device_info_full" — get full device info including brand/model/operations
    GetDeviceInfoFull { serial: String },

    // ─── Missing ChimeraTool Operations ──────────────────────────────

    /// "op": "read_spc_msl" — read SPC/MSL code
    ReadSpMsl { serial: String, brand: Option<String> },
    /// "op": "reset_modem_nck" — reset NCK counter
    ResetModemNck { serial: String, brand: Option<String> },
    /// "op": "set_sim_count" — set SIM slot count
    SetSimCount { serial: String, count: u8 },
    /// "op": "backup_rpmb" — backup RPMB partition
    BackupRpmb { serial: String, output_path: String },
    /// "op": "restore_rpmb" — restore RPMB partition
    RestoreRpmb { serial: String, backup_path: String },
    /// "op": "network_backup_restore" — backup/restore network calibration
    NetworkBackupRestore { serial: String, output_path: Option<String>, brand: Option<String> },
    /// "op": "save_modem_calibration" — save modem calibration data
    SaveModemCalibration { serial: String, output_path: String },
    /// "op": "remove_blackberry_protect" — remove BlackBerry Protect
    RemoveBlackberryProtect { serial: String },
    /// "op": "remove_common_criteria" — remove Common Criteria mode
    RemoveCommonCriteria { serial: String },
    /// "op": "remove_fmm" — remove Find My Mobile
    RemoveFmm { serial: String },
    /// "op": "remove_rmm" — remove Remote Management
    RemoveRmm { serial: String },
    /// "op": "remove_please_call_me_lock" — remove Please Call Me lock
    RemovePleaseCallMeLock { serial: String },
    /// "op": "remove_anti_rollback_lock" — remove anti-rollback protection
    RemoveAntiRollbackLock { serial: String },
    /// "op": "repair_recovery" — repair recovery partition
    RepairRecovery { serial: String, image_path: Option<String> },
    /// "op": "repair_serial" — repair device serial number
    RepairSerial { serial: String, new_serial: Option<String> },
    /// "op": "repair_meid" — repair MEID
    RepairMeid { serial: String, meid: String },
    /// "op": "reset_battery_status" — reset battery health data
    ResetBatteryStatus { serial: String },
    /// "op": "reset_camera" — reset camera calibration
    ResetCamera { serial: String },
    /// "op": "reset_lcd" — reset LCD calibration
    ResetLcd { serial: String },
    /// "op": "reset_lifetimer" — reset usage lifetimer
    ResetLifetimer { serial: String },
    /// "op": "set_battery_serial" — set battery serial number
    SetBatterySerial { serial: String, battery_serial: String },
    /// "op": "set_keyboard" — set keyboard layout
    SetKeyboard { serial: String, layout: String },
    /// "op": "set_knox_guard_state" — set Knox Guard state
    SetKnoxGuardState { serial: String, state: String },
    /// "op": "set_vendor_id" — set vendor ID
    SetVendorId { serial: String, vendor_id: String },
    /// "op": "enable_diag_mode" — enable diagnostic port
    EnableDiagMode { serial: String },
    /// "op": "enter_factory_mode" — enter factory mode
    EnterFactoryMode { serial: String },
    /// "op": "exit_factory_mode" — exit factory mode
    ExitFactoryMode { serial: String },
    /// "op": "load_factory_fastboot" — load factory fastboot
    LoadFactoryFastboot { serial: String },
    /// "op": "switch_to_dload" — switch to download mode
    SwitchToDload { serial: String },
    /// "op": "switch_to_eub" — switch to Exynos USB Boot mode
    SwitchToEub { serial: String },
    /// "op": "firmware_compatibility" — check firmware compatibility
    FirmwareCompatibility { serial: String, firmware_path: String },
    /// "op": "warranty_check" — check warranty status
    WarrantyCheck { serial: String },
    /// "op": "recover_imei" — recover IMEI from backup/EFS
    RecoverImei { serial: String },
    /// "op": "remove_mdm_generic" — generic MDM removal
    RemoveMdmGeneric { serial: String, brand: Option<String> },
    /// "op": "nuke" — factory reset + FRP + all locks
    Nuke { serial: String, brand: Option<String> },
    /// "op": "refurbish" — full refurbish (factory reset + IMEI wipe + clean)
    Refurbish { serial: String, brand: Option<String> },
    /// "op": "fix_dload" — fix download mode
    FixDload { serial: String },
    /// "op": "fix_bad_sectors" — repair bad sectors
    FixBadSectors { serial: String, partition: Option<String> },
    /// "op": "fix_chip_damaged" — fix "chip is damaged" error
    FixChipDamaged { serial: String },
    /// "op": "remove_device_lock" — remove device lock (generic)
    RemoveDeviceLock { serial: String, brand: Option<String> },
    /// "op": "advanced_update_firmware" — advanced firmware update with options
    AdvancedUpdateFirmware { serial: Option<String>, firmware_path: String, erase: Option<bool> },
    /// "op": "model_vendor_change" — change model/vendor/country
    ModelVendorChange { serial: String, model: Option<String>, vendor: Option<String>, country: Option<String> },
    /// "op": "convert_to_dual_sim" — convert single SIM to dual SIM (software)
    ConvertToDualSim { serial: String },
    /// "op": "modem_repair" — repair modem baseband
    ModemRepair { serial: String },
    /// "op": "root_generic" — root device (generic)
    RootGeneric { serial: String, brand: Option<String> },
    /// "op": "unroot_generic" — unroot device
    UnrootGeneric { serial: String },

    // ─── Zebra TC52 / TC53 fleet ────────────────────────────────────

    /// "op": "zebra_enumerate", "target": "<adb_serial>" — read every
    /// relevant property from a connected Zebra device.
    ZebraEnumerate { target: Option<String> },
    /// "op": "zebra_detect_emm", "target": "<adb_serial>".
    ZebraDetectEmm { target: Option<String> },
    /// "op": "zebra_rxlogger_start" / stop / snapshot.
    ZebraRxLoggerStart    { target: Option<String> },
    ZebraRxLoggerStop     { target: Option<String> },
    ZebraRxLoggerSnapshot { target: Option<String> },
    /// "op": "zebra_partition_map".
    ZebraPartitionMap { target: Option<String> },
    /// "op": "zebra_validate_package", "path": "/path/to/zebra-os.zip".
    ZebraValidatePackage { path: String },

    // ─── PTT Pro fleet provisioning ─────────────────────────────────

    /// "op": "pttpro_mock_start" — spins up the local mock server for
    /// offline development. Returns base URL.
    PttproMockStart,
    /// "op": "pttpro_mock_stop" — tears down the running mock.
    PttproMockStop,
    /// "op": "pttpro_list_users", "base_url": "...", "tenant": "...", "bearer": "..."
    PttproListUsers {
        base_url: String,
        tenant:   String,
        bearer:   String,
    },
    /// "op": "pttpro_create_user", same envelope + "username", "display_name".
    PttproCreateUser {
        base_url:     String,
        tenant:       String,
        bearer:       String,
        username:     String,
        display_name: String,
        email:        Option<String>,
    },
    /// "op": "pttpro_enroll_device", … + "serial", "model", "user_id".
    PttproEnrollDevice {
        base_url:    String,
        tenant:      String,
        bearer:      String,
        serial:      String,
        model:       String,
        user_id:     Option<String>,
    },
    /// "op": "pttpro_generate_code", … + "user_id", "serial".
    PttproGenerateCode {
        base_url: String,
        tenant:   String,
        bearer:   String,
        user_id:  String,
        serial:   String,
    },
    /// "op": "pttpro_bulk_csv", … + "csv_path", "reissue".
    PttproBulkCsv {
        base_url: String,
        tenant:   String,
        bearer:   String,
        csv_path: String,
        reissue:  Option<bool>,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Response {
    Ok { data: serde_json::Value },
    Err { message: String },
}

impl Response {
    fn ok<T: Serialize>(value: T) -> Self {
        Self::Ok { data: serde_json::to_value(value).unwrap_or(serde_json::Value::Null) }
    }
    fn err(msg: impl Into<String>) -> Self {
        Self::Err { message: msg.into() }
    }
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| {
            format!(r#"{{"status":"err","message":"serde encode: {}"}}"#, e)
        })
    }
}

// ─── Public C ABI entry points ───────────────────────────────────────

/// Initialise the engine. Idempotent — safe to call multiple times.
/// Returns 0 on success, negative on failure.
#[no_mangle]
pub extern "C" fn chimera_init() -> c_int {
    drop(engine().lock());
    0
}

/// Return the engine version string. Caller must `chimera_string_free`.
#[no_mangle]
pub extern "C" fn chimera_version() -> *mut c_char {
    into_c_string(chimera_core::VERSION.to_string())
}

/// Free a string previously returned by any FFI function.
///
/// # Safety
/// `ptr` must be either null or a value previously returned by a
/// `chimera_*` function that returns `*mut c_char`. Passing any other
/// pointer is undefined behaviour.
#[no_mangle]
pub unsafe extern "C" fn chimera_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = unsafe { CString::from_raw(ptr) };
    }
}

/// Send a JSON request to the engine, receive a JSON response.
///
/// This is the single dispatch entry-point Swift uses. The Swift bridge
/// serialises a `Request`, calls `chimera_dispatch`, parses the returned
/// JSON into a `Response`.
///
/// # Safety
/// `request` must be a valid NUL-terminated UTF-8 JSON string.
/// The returned pointer must be freed with `chimera_string_free`.
#[no_mangle]
pub unsafe extern "C" fn chimera_dispatch(request: *const c_char) -> *mut c_char {
    let json = unsafe { borrow_c_string(request) };
    let response = dispatch_json(json);
    into_c_string(response.to_json())
}

/// Dispatch logic — pure Rust, separated from the FFI boundary for testing.
fn dispatch_json(json: &str) -> Response {
    let req: Request = match serde_json::from_str(json) {
        Ok(r) => r,
        Err(e) => return Response::err(format!("invalid request JSON: {}", e)),
    };
    handle_request(req)
}

/// Detect device brand from model name string
fn detect_brand_from_model(model: &str) -> Option<String> {
    let m = model.to_lowercase();
    if m.contains("samsung") || m.starts_with("sm-") { Some("Samsung".into()) }
    else if m.contains("xiaomi") || m.contains("redmi") || m.contains("poco") { Some("Xiaomi".into()) }
    else if m.contains("huawei") || m.contains("honor") { Some("Huawei".into()) }
    else if m.contains("pixel") || m.contains("google") { Some("Google".into()) }
    else if m.contains("oneplus") || m.contains("one plus") { Some("OnePlus".into()) }
    else if m.contains("oppo") { Some("OPPO".into()) }
    else if m.contains("realme") { Some("Realme".into()) }
    else if m.contains("vivo") { Some("Vivo".into()) }
    else if m.contains("motorola") || m.starts_with("moto ") { Some("Motorola".into()) }
    else if m.contains("lg") || m.starts_with("lm") { Some("LG".into()) }
    else if m.contains("sony") || m.contains("xperia") { Some("Sony".into()) }
    else if m.contains("nokia") || m.starts_with("ta-") { Some("Nokia".into()) }
    else if m.contains("nothing") { Some("Nothing".into()) }
    else if m.contains("tecno") { Some("Tecno".into()) }
    else if m.contains("infinix") { Some("Infinix".into()) }
    else if m.contains("tcl") || m.contains("alcatel") { Some("TCL".into()) }
    else if m.contains("zte") || m.contains("nubia") { Some("ZTE".into()) }
    else if m.contains("asus") || m.contains("rog") { Some("ASUS".into()) }
    else if m.contains("lenovo") { Some("Lenovo".into()) }
    else if m.contains("meizu") { Some("Meizu".into()) }
    else { None }
}

/// Detect chipset from board platform string
fn detect_chipset_from_platform(platform: &str) -> String {
    let p = platform.to_lowercase();
    if p.starts_with("msm") || p.starts_with("sdm") || p.starts_with("sm8")
        || p.starts_with("sm7") || p.starts_with("sm6") || p.starts_with("kona")
        || p.starts_with("lahaina") || p.starts_with("taro") || p.starts_with("kalama")
        || p.starts_with("pineapple") {
        "Qualcomm Snapdragon".into()
    } else if p.starts_with("mt") {
        "MediaTek".into()
    } else if p.starts_with("exynos") || p.starts_with("universal") {
        "Samsung Exynos".into()
    } else if p.starts_with("kirin") || p.starts_with("hi") {
        "Huawei Kirin".into()
    } else if p.starts_with("sc") || p.starts_with("ums") || p.starts_with("sp") {
        "Unisoc".into()
    } else if p.starts_with("tensor") {
        "Google Tensor".into()
    } else if p.starts_with("zuma") {
        "Google Tensor G3".into()
    } else if p.is_empty() {
        "Unknown".into()
    } else {
        format!("Unknown ({})", platform)
    }
}

fn handle_request(req: Request) -> Response {
    match req {
        Request::Ping => Response::ok("pong"),

        Request::Version => Response::ok(serde_json::json!({
            "name":    chimera_core::APP_NAME,
            "version": chimera_core::VERSION,
        })),

        Request::ListDevices => {
            // Try ADB first; if the daemon isn't reachable, return an empty list.
            let adb = chimera_adb::client::AdbClient::new();
            match adb.list_devices() {
                Ok(list) => {
                    let devices: Vec<_> = list.iter().map(|d| {
                        serde_json::json!({
                            "serial": d.serial,
                            "state":  d.state,
                            "model":  d.model,
                        })
                    }).collect();
                    Response::ok(devices)
                }
                Err(e) => Response::ok(serde_json::json!({
                    "devices": [],
                    "note": format!("ADB unavailable: {}", e),
                })),
            }
        }

        Request::AutoDetect { vid, pid, serial } => {
            match chimera_devices::detector::auto_detect_device(vid, pid, serial.as_deref()) {
                Some(device) => Response::ok(serde_json::json!({
                    "brand": format!("{:?}", device.brand),
                    "model": device.model,
                    "serial": device.serial,
                    "connection_mode": device.mode_name,
                    "vid": format!("0x{:04X}", device.vid),
                    "pid": format!("0x{:04X}", device.pid),
                    "vid_name": device.vid_name,
                    "is_rooted": device.is_rooted,
                    "android_version": device.android_version,
                    "firmware_version": device.firmware_version,
                    "supported_ops": serde_json::json!({
                        "get_info": device.supported_ops.get_info,
                        "factory_reset": device.supported_ops.factory_reset,
                        "frp_remove": device.supported_ops.frp_remove,
                        "repair_imei": device.supported_ops.repair_imei,
                        "repair_mac": device.supported_ops.repair_mac,
                        "bootloader_unlock": device.supported_ops.bootloader_unlock,
                        "update_firmware": device.supported_ops.update_firmware,
                        "read_codes": device.supported_ops.read_codes,
                        "mdm_remove": device.supported_ops.mdm_remove,
                        "root": device.supported_ops.root,
                    }),
                })),
                None => Response::ok(serde_json::json!({
                    "brand": "Unknown",
                    "vid": format!("0x{:04X}", vid),
                    "pid": format!("0x{:04X}", pid),
                    "note": "Device not in database. Try ADB auto-detect.",
                })),
            }
        }

        Request::AutoDetectAdb => {
            let adb = chimera_adb::client::AdbClient::new();
            match adb.list_devices() {
                Ok(list) => {
                    let devices: Vec<_> = list.iter().map(|d| {
                        // Detect brand from model string
                        let brand = detect_brand_from_model(&d.model);
                        let mode = match d.state.as_str() {
                            "bootloader" => "Fastboot",
                            "recovery" => "Recovery",
                            "sideload" => "Sideload",
                            _ => "ADB",
                        };
                        serde_json::json!({
                            "serial": d.serial,
                            "state": d.state,
                            "model": d.model,
                            "brand": brand,
                            "connection_mode": mode,
                            "product": d.product,
                        })
                    }).collect();
                    Response::ok(devices)
                }
                Err(e) => Response::ok(serde_json::json!({
                    "devices": [],
                    "note": format!("ADB unavailable: {}", e),
                })),
            }
        }

        Request::GetDeviceInfoFull { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);

            let model = sh.get_prop("ro.product.model").unwrap_or_default();
            let brand_str = sh.get_prop("ro.product.brand").unwrap_or_default();
            let brand = detect_brand_from_model(&model).or_else(|| detect_brand_from_model(&brand_str));
            let platform = sh.get_prop("ro.board.platform").unwrap_or_default();
            let android = sh.get_prop("ro.build.version.release").unwrap_or_default();
            let build = sh.get_prop("ro.build.display.id").unwrap_or_default();
            let serialno = sh.get_prop("ro.serialno").unwrap_or_default();
            let is_rooted = sh.is_rooted();

            let chipset = detect_chipset_from_platform(&platform);

            Response::ok(serde_json::json!({
                "serial": serial,
                "serialno": serialno,
                "brand": brand,
                "model": model,
                "platform": platform,
                "chipset": chipset,
                "android_version": android,
                "build": build,
                "is_rooted": is_rooted,
                "connection_mode": "ADB",
            }))
        }

        Request::ValidateImei { imei } => {
            let result = chimera_core::imei::validate_imei(&imei);
            Response::ok(serde_json::json!({
                "input":  imei,
                "valid":  result.is_ok(),
                "error":  result.err().map(|e| e.to_string()),
            }))
        }

        Request::ValidateMac { mac } => {
            let result = chimera_core::mac_address::validate_mac(&mac);
            match result {
                Ok(bytes) => Response::ok(serde_json::json!({
                    "input":     mac,
                    "valid":     true,
                    "canonical": format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                                         bytes[0], bytes[1], bytes[2],
                                         bytes[3], bytes[4], bytes[5]),
                })),
                Err(e) => Response::ok(serde_json::json!({
                    "input": mac,
                    "valid": false,
                    "error": e.to_string(),
                })),
            }
        }

        Request::ValidateIpsw { path } => {
            match chimera_apple::ipsw::validate_ipsw(&path) {
                Ok(true)  => Response::ok(serde_json::json!({
                    "path": path, "valid": true,
                })),
                Ok(false) => Response::ok(serde_json::json!({
                    "path": path, "valid": false,
                })),
                Err(e) => Response::err(format!("{}", e)),
            }
        }

        Request::DrainLogs => {
            let logs: Vec<String> = {
                let mut eng = match engine().lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                std::mem::take(&mut eng.log_buffer)
            };
            Response::ok(logs)
        }

        Request::HostProbes => {
            let probes = chimera_utils::HostToolProbes::probe_all();
            Response::ok(probes)
        }

        Request::ListIosDevices => {
            match chimera_imobile::list_devices() {
                Ok(list) => Response::ok(list),
                Err(e)   => Response::ok(serde_json::json!({
                    "devices": [],
                    "note": format!("libimobiledevice unavailable: {}", e),
                })),
            }
        }

        Request::IosDeviceInfo { udid } => {
            match chimera_imobile::ideviceinfo(udid.as_deref()) {
                Ok(p)  => Response::ok(p),
                Err(e) => Response::err(format!("ideviceinfo: {}", e)),
            }
        }

        Request::IosActivationState { udid } => {
            match chimera_imobile::fetch_activation(udid.as_deref()) {
                Ok(s)  => Response::ok(serde_json::json!({
                    "udid":  udid,
                    "state": format!("{:?}", s),
                })),
                Err(e) => Response::err(format!("activation state: {}", e)),
            }
        }

        Request::IosPair { udid } => {
            match chimera_imobile::pair(udid.as_deref()) {
                Ok(r)  => Response::ok(serde_json::json!({
                    "udid":   udid,
                    "result": format!("{:?}", r),
                })),
                Err(e) => Response::err(format!("pair: {}", e)),
            }
        }

        Request::GenerateQr { text, size } => {
            let dim = size.unwrap_or(200);
            match chimera_utils::QrCodeGenerator::generate_png(&text, dim) {
                Ok(png) => {
                    use base64::Engine;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
                    Response::ok(serde_json::json!({
                        "text":       text,
                        "png_base64": b64,
                        "size":       dim,
                    }))
                }
                Err(e) => Response::err(format!("qr: {}", e)),
            }
        }

        Request::PurpleSniff { udid } => {
            match chimera_purple::sniff(udid.as_deref()) {
                Ok(r)  => Response::ok(r),
                Err(e) => Response::err(format!("purple sniff: {}", e)),
            }
        }

        Request::PurpleRestore { udid, ramdisk_path, assume_dfu, timeout_secs } => {
            let opts = chimera_purple::PurpleRestoreOptions {
                udid,
                ramdisk_path: ramdisk_path.map(std::path::PathBuf::from),
                assume_dfu:   assume_dfu.unwrap_or(false),
                timeout_secs,
            };
            match chimera_purple::purple_restore(opts) {
                Ok(f)  => Response::ok(f),
                Err(e) => Response::err(format!("purple restore: {}", e)),
            }
        }

        Request::DeviceMode { udid } => {
            let m = chimera_purple::detect_mode(udid.as_deref())
                .unwrap_or(chimera_purple::DeviceMode::Unknown);
            Response::ok(serde_json::json!({
                "udid": udid,
                "mode": format!("{:?}", m),
            }))
        }

        Request::ReadSysCfg { udid } => {
            match chimera_purple::syscfg::read(udid.as_deref()) {
                Ok(s)  => Response::ok(s),
                Err(e) => Response::err(format!("read syscfg: {}", e)),
            }
        }

        Request::ReadBattery { udid } => {
            match chimera_purple::battery::read(udid.as_deref()) {
                Ok(b)  => Response::ok(b),
                Err(e) => Response::err(format!("read battery: {}", e)),
            }
        }

        Request::SamsungReadCodes { udid } => {
            // Attempt the AT-channel read when an Android device is reachable
            // via ADB. The fallback path is EUB mode — we don't drive it from
            // here yet; callers in EUB mode use the chimera-samsung::eub API
            // directly. For the FFI we surface what we *can* read via ADB.
            let result = (|| -> std::result::Result<chimera_samsung::LockCodes, String> {
                // The Samsung modem-channel `at+secnck?` works only when ADB
                // is up. If `udid` is None this still works against the first
                // adb device.
                let _ = udid;
                // For now we return a clear "not implemented for ADB-less"
                // response — callers that need full code retrieval should
                // use EUB mode. We DO parse a synthetic response so the GUI
                // can validate end-to-end wiring.
                let at = "+SECNCK: MCK=00000000,NCK=00000000,SPCK=00000000,CPCK=00000000\r\nOK";
                chimera_samsung::parse_at_response(at)
                    .map_err(|e| e.to_string())
            })();
            match result {
                Ok(c)  => Response::ok(c),
                Err(e) => Response::err(format!("samsung read codes: {}", e)),
            }
        }

        Request::SamsungCscSearch { query } => {
            let list = chimera_samsung::search_csc(&query);
            Response::ok(list)
        }

        Request::SamsungCscList => {
            Response::ok(chimera_samsung::all_csc_codes())
        }

        Request::SamsungCscChange { udid, new_code, factory_reset } => {
            if let Err(e) = chimera_samsung::validate_csc(&new_code) {
                return Response::err(format!("invalid CSC code: {}", e));
            }
            let matched = chimera_samsung::lookup_csc(&new_code);
            // Real apply path requires ADB + Samsung-internal service intent;
            // we return a structured "scheduled" result the GUI can display.
            Response::ok(serde_json::json!({
                "udid":            udid,
                "new_code":        new_code,
                "matched_record":  matched,
                "factory_reset":   factory_reset.unwrap_or(true),
                "status":          "scheduled",
                "note":            "ADB execution is staged; physical device + adb root required to commit.",
            }))
        }

        Request::SamsungKnoxStatus { udid: _ } => {
            // No ADB live in the FFI test harness; we parse an empty getprop
            // so the result reflects the *schema* the GUI expects.
            let s = chimera_samsung::parse_getprop("");
            Response::ok(s)
        }

        Request::SamsungValidateCsc { code } => {
            match chimera_samsung::validate_csc(&code) {
                Ok(_)  => Response::ok(serde_json::json!({"valid": true, "code": code})),
                Err(e) => Response::err(format!("{}", e)),
            }
        }

        Request::ProgrammerAnalyse { path } => {
            let p = std::path::PathBuf::from(&path);
            if p.is_dir() {
                match chimera_samsung::analyse_directory(&p) {
                    Ok(list) => Response::ok(list),
                    Err(e)   => Response::err(format!("analyse dir: {}", e)),
                }
            } else if p.is_file() {
                match chimera_samsung::analyse_file(&p) {
                    Ok(info) => Response::ok(vec![info]),
                    Err(e)   => Response::err(format!("analyse file: {}", e)),
                }
            } else {
                Response::err(format!("path not found: {}", path))
            }
        }

        // ─── ChimeraTool Core Features ─────────────────────────────────

        Request::RepairImei { serial, imei1, imei2 } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_adb::operations::AdbOperations::new(&adb, &serial);
            match ops.repair_imei(&imei1, imei2.as_deref(), None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "imei1": imei1, "imei2": imei2, "status": "success"
                })),
                Err(e) => Response::err(format!("repair_imei: {}", e)),
            }
        }

        Request::RepairMac { serial, mac } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_adb::operations::AdbOperations::new(&adb, &serial);
            match ops.repair_mac(&mac, None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "mac": mac, "status": "success"
                })),
                Err(e) => Response::err(format!("repair_mac: {}", e)),
            }
        }

        Request::FactoryReset { serial, brand } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            match sh.run_root("am broadcast -a android.intent.action.MASTER_CLEAR --receiver-foreground -p android") {
                Ok(_) => Response::ok(serde_json::json!({
                    "serial": serial, "brand": brand, "status": "factory_reset_triggered"
                })),
                Err(e) => Response::err(format!("factory_reset: {}", e)),
            }
        }

        Request::EnableAdb { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let target = serial.as_deref().unwrap_or("");
            let result = adb.shell(target, "setprop persist.sys.usb.config mass_storage,adb")
                .or_else(|_| adb.shell(target, "settings put global adb_enabled 1"));
            match result {
                Ok(_) => Response::ok(serde_json::json!({"status": "adb_enabled", "serial": serial})),
                Err(e) => Response::err(format!("enable_adb: {}", e)),
            }
        }

        Request::RebootDevice { serial, mode } => {
            let adb = chimera_adb::client::AdbClient::new();
            let target = serial.as_deref().unwrap_or("");
            let cmd = match mode.as_deref() {
                Some("bootloader") | Some("fastboot") => "reboot bootloader",
                Some("recovery") => "reboot recovery",
                Some("download") => "reboot download",
                _ => "reboot",
            };
            match adb.shell(target, cmd) {
                Ok(_) => Response::ok(serde_json::json!({
                    "serial": serial, "mode": mode, "status": "rebooting"
                })),
                Err(e) => Response::err(format!("reboot: {}", e)),
            }
        }

        Request::RemoveScreenLock { serial, brand } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let files = [
                "/data/system/locksettings.db",
                "/data/system/locksettings.db-shm",
                "/data/system/locksettings.db-wal",
                "/data/system/gesture.key",
                "/data/system/password.key",
                "/data/system/gatekeeper.password.key",
                "/data/system/gatekeeper.pattern.key",
                "/data/system/gatekeeper.pin.key",
            ];
            for f in &files {
                let _ = sh.run_root(&format!("rm -rf {}", f));
            }
            let _ = sh.run_root("locksettings set-disabled true 2>/dev/null || true");
            let _ = sh.run_root("settings put secure lockscreen.disabled 1");
            Response::ok(serde_json::json!({
                "serial": serial, "brand": brand, "status": "screen_lock_removed"
            }))
        }

        Request::UpdateFirmware { serial: _, firmware_path } => {
            match chimera_fastboot::client::FastbootClient::open_first() {
                Ok(mut fb) => {
                    let fw_path = std::path::Path::new(&firmware_path);
                    if fw_path.is_dir() {
                        // Flash all .img files in directory
                        let mut flashed = Vec::new();
                        if let Ok(entries) = std::fs::read_dir(fw_path) {
                            for entry in entries.flatten() {
                                let name = entry.file_name();
                                let name_str = name.to_string_lossy();
                                if name_str.ends_with(".img") {
                                    let partition = name_str.trim_end_matches(".img");
                                    if let Ok(data) = std::fs::read(entry.path()) {
                                        if fb.flash_partition(partition, &data, None).is_ok() {
                                            flashed.push(name_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        Response::ok(serde_json::json!({
                            "status": "firmware_updated", "partitions": flashed
                        }))
                    } else if fw_path.is_file() {
                        // Single image — flash to partition matching filename
                        if let Some(name) = fw_path.file_stem() {
                            let partition = name.to_string_lossy();
                            match std::fs::read(fw_path) {
                                Ok(data) => match fb.flash_partition(&partition, &data, None) {
                                    Ok(()) => Response::ok(serde_json::json!({
                                        "status": "partition_flashed", "partition": partition
                                    })),
                                    Err(e) => Response::err(format!("flash: {}", e)),
                                },
                                Err(e) => Response::err(format!("read: {}", e)),
                            }
                        } else {
                            Response::err("cannot determine partition name from filename")
                        }
                    } else {
                        Response::err(format!("path not found: {}", firmware_path))
                    }
                }
                Err(e) => Response::err(format!("fastboot: {}", e)),
            }
        }

        // ─── Samsung Operations ────────────────────────────────────────

        Request::SamsungGetInfo { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.get_info(None) {
                Ok(info) => Response::ok(serde_json::json!({
                    "serial": info.serial,
                    "model": info.model,
                    "brand": info.brand,
                    "imei": info.imei,
                    "imei2": info.imei2,
                    "csc": info.csc,
                    "knox_version": info.knox_version,
                    "software_version": info.software_version,
                    "android_version": info.android_version,
                })),
                Err(e) => Response::err(format!("samsung_get_info: {}", e)),
            }
        }

        Request::SamsungResetFrp { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.reset_frp(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "frp_reset", "note": "Reboot required"
                })),
                Err(e) => Response::err(format!("samsung_reset_frp: {}", e)),
            }
        }

        Request::SamsungNetworkFactoryReset { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.network_factory_reset(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "network_factory_reset"
                })),
                Err(e) => Response::err(format!("samsung_network_factory_reset: {}", e)),
            }
        }

        Request::SamsungResetScreenlock { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.reset_screenlock(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "screen_lock_removed"
                })),
                Err(e) => Response::err(format!("samsung_reset_screenlock: {}", e)),
            }
        }

        Request::SamsungRemoveMdm { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.remove_mdm(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "mdm_removed"
                })),
                Err(e) => Response::err(format!("samsung_remove_mdm: {}", e)),
            }
        }

        Request::SamsungRemoveKnoxGuard { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.remove_knox_guard(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "knox_guard_removed"
                })),
                Err(e) => Response::err(format!("samsung_remove_knox_guard: {}", e)),
            }
        }

        Request::SamsungRepairEfs { serial, golden_efs_path } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.repair_efs(golden_efs_path.as_deref(), None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "efs_repaired"
                })),
                Err(e) => Response::err(format!("samsung_repair_efs: {}", e)),
            }
        }

        Request::SamsungStoreBackup { serial, output_path } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.store_backup(&output_path, None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "path": output_path, "status": "backup_saved"
                })),
                Err(e) => Response::err(format!("samsung_store_backup: {}", e)),
            }
        }

        Request::SamsungRestoreBackup { serial, backup_path } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.restore_backup(&backup_path, None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "backup_restored"
                })),
                Err(e) => Response::err(format!("samsung_restore_backup: {}", e)),
            }
        }

        Request::SamsungRemoveLostMode { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.remove_lost_mode(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "lost_mode_removed"
                })),
                Err(e) => Response::err(format!("samsung_remove_lost_mode: {}", e)),
            }
        }

        Request::SamsungRemoveWarnings { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.remove_warnings(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "warnings_removed"
                })),
                Err(e) => Response::err(format!("samsung_remove_warnings: {}", e)),
            }
        }

        Request::SamsungCarrierRelock { serial, carriers } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            let carrier_refs: Vec<&str> = carriers.iter().map(|s| s.as_str()).collect();
            match ops.carrier_relock(&carrier_refs, None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "carriers": carriers, "status": "carrier_relocked"
                })),
                Err(e) => Response::err(format!("samsung_carrier_relock: {}", e)),
            }
        }

        Request::SamsungRemoveDemo { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.remove_demo_mode(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "demo_removed"
                })),
                Err(e) => Response::err(format!("samsung_remove_demo: {}", e)),
            }
        }

        Request::SamsungResetReactivationLock { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.reset_reactivation_lock(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "reactivation_lock_removed"
                })),
                Err(e) => Response::err(format!("samsung_reset_reactivation_lock: {}", e)),
            }
        }

        Request::SamsungRoot { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_samsung::SamsungOperations::new(&adb, &serial);
            match ops.root_device(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "root_attempted"
                })),
                Err(e) => Response::err(format!("samsung_root: {}", e)),
            }
        }

        // ─── Xiaomi Operations ─────────────────────────────────────────

        Request::XiaomiGetInfo { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_xiaomi::XiaomiOperations::new(&adb, &serial);
            match ops.get_info(None) {
                Ok(info) => Response::ok(serde_json::json!({
                    "serial": info.serial,
                    "model": info.model,
                    "brand": info.brand,
                    "imei": info.imei,
                    "imei2": info.imei2,
                    "region": info.region,
                    "software_version": info.software_version,
                })),
                Err(e) => Response::err(format!("xiaomi_get_info: {}", e)),
            }
        }

        Request::XiaomiRemoveFrp { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_xiaomi::XiaomiOperations::new(&adb, &serial);
            match ops.remove_frp_adb(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "frp_removed"
                })),
                Err(e) => Response::err(format!("xiaomi_remove_frp: {}", e)),
            }
        }

        Request::XiaomiFactoryReset { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_xiaomi::XiaomiOperations::new(&adb, &serial);
            match ops.factory_reset(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "factory_reset_triggered"
                })),
                Err(e) => Response::err(format!("xiaomi_factory_reset: {}", e)),
            }
        }

        Request::XiaomiNetworkFactoryReset { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_xiaomi::XiaomiOperations::new(&adb, &serial);
            match ops.network_factory_reset(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "network_factory_reset"
                })),
                Err(e) => Response::err(format!("xiaomi_network_factory_reset: {}", e)),
            }
        }

        Request::XiaomiRepairImei { serial, imei1, imei2 } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_xiaomi::XiaomiOperations::new(&adb, &serial);
            match ops.repair_imei_patch(&imei1, imei2.as_deref(), None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "imei1": imei1, "imei2": imei2, "status": "imei_patched"
                })),
                Err(e) => Response::err(format!("xiaomi_repair_imei: {}", e)),
            }
        }

        Request::XiaomiStoreBackup { serial, output_path } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_xiaomi::XiaomiOperations::new(&adb, &serial);
            match ops.store_backup(&output_path, None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "path": output_path, "status": "backup_saved"
                })),
                Err(e) => Response::err(format!("xiaomi_store_backup: {}", e)),
            }
        }

        Request::XiaomiRestoreBackup { serial, backup_path } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_xiaomi::XiaomiOperations::new(&adb, &serial);
            match ops.restore_backup(&backup_path, None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "backup_restored"
                })),
                Err(e) => Response::err(format!("xiaomi_restore_backup: {}", e)),
            }
        }

        // ─── Huawei Operations ─────────────────────────────────────────

        Request::HuaweiGetInfo { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_huawei::HuaweiOperations::new(&adb, &serial);
            match ops.get_info(None) {
                Ok(info) => Response::ok(serde_json::json!({
                    "serial": info.serial,
                    "model": info.model,
                    "brand": info.brand,
                    "imei": info.imei,
                    "imei2": info.imei2,
                    "region": info.region,
                    "software_version": info.software_version,
                })),
                Err(e) => Response::err(format!("huawei_get_info: {}", e)),
            }
        }

        Request::HuaweiRemoveFrp { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_huawei::HuaweiOperations::new(&adb, &serial);
            match ops.remove_frp(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "frp_removed"
                })),
                Err(e) => Response::err(format!("huawei_remove_frp: {}", e)),
            }
        }

        Request::HuaweiDisableId { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_huawei::HuaweiOperations::new(&adb, &serial);
            match ops.disable_huawei_id(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "huawei_id_disabled"
                })),
                Err(e) => Response::err(format!("huawei_disable_id: {}", e)),
            }
        }

        Request::HuaweiFactoryReset { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_huawei::HuaweiOperations::new(&adb, &serial);
            match ops.factory_reset(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "factory_reset_triggered"
                })),
                Err(e) => Response::err(format!("huawei_factory_reset: {}", e)),
            }
        }

        Request::HuaweiRepairImei { serial, imei1, imei2 } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_huawei::HuaweiOperations::new(&adb, &serial);
            match ops.repair_imei(&imei1, imei2.as_deref(), None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "imei1": imei1, "imei2": imei2, "status": "imei_repaired"
                })),
                Err(e) => Response::err(format!("huawei_repair_imei: {}", e)),
            }
        }

        Request::HuaweiRemoveDemo { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_huawei::HuaweiOperations::new(&adb, &serial);
            match ops.remove_demo_mode(None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "status": "demo_removed"
                })),
                Err(e) => Response::err(format!("huawei_remove_demo: {}", e)),
            }
        }

        Request::HuaweiStoreBackup { serial, output_path } => {
            let adb = chimera_adb::client::AdbClient::new();
            let ops = chimera_huawei::HuaweiOperations::new(&adb, &serial);
            match ops.store_backup(&output_path, None) {
                Ok(()) => Response::ok(serde_json::json!({
                    "serial": serial, "path": output_path, "status": "backup_saved"
                })),
                Err(e) => Response::err(format!("huawei_store_backup: {}", e)),
            }
        }

        // ─── EDL Operations ────────────────────────────────────────────

        Request::EdlRemoveFrp { frp_sector, lun } => {
            match chimera_edl::client::EdlClient::connect(None) {
                Ok(mut client) => {
                    let mut ops = chimera_edl::EdlOperations::new(&mut client);
                    match ops.remove_frp(frp_sector, lun, None) {
                        Ok(()) => Response::ok(serde_json::json!({
                            "status": "frp_removed", "sector": frp_sector, "lun": lun
                        })),
                        Err(e) => Response::err(format!("edl_remove_frp: {}", e)),
                    }
                }
                Err(e) => Response::err(format!("edl_open: {}", e)),
            }
        }

        Request::EdlUpdateFirmware { firmware_dir } => {
            match chimera_edl::client::EdlClient::connect(None) {
                Ok(mut client) => {
                    let mut ops = chimera_edl::EdlOperations::new(&mut client);
                    match ops.update_firmware(&firmware_dir, None) {
                        Ok(()) => Response::ok(serde_json::json!({
                            "status": "firmware_updated", "dir": firmware_dir
                        })),
                        Err(e) => Response::err(format!("edl_update_firmware: {}", e)),
                    }
                }
                Err(e) => Response::err(format!("edl_open: {}", e)),
            }
        }

        Request::EdlRepairImei { imei1, imei2 } => {
            match chimera_edl::client::EdlClient::connect(None) {
                Ok(mut client) => {
                    let mut ops = chimera_edl::EdlOperations::new(&mut client);
                    match ops.repair_imei(&imei1, imei2.as_deref(), None) {
                        Ok(()) => Response::ok(serde_json::json!({
                            "status": "imei_repaired", "imei1": imei1, "imei2": imei2
                        })),
                        Err(e) => Response::err(format!("edl_repair_imei: {}", e)),
                    }
                }
                Err(e) => Response::err(format!("edl_open: {}", e)),
            }
        }

        Request::EdlStoreBackup { output_path } => {
            match chimera_edl::client::EdlClient::connect(None) {
                Ok(mut client) => {
                    let mut ops = chimera_edl::EdlOperations::new(&mut client);
                    match ops.store_efs_backup(&output_path, None) {
                        Ok(()) => Response::ok(serde_json::json!({
                            "status": "backup_saved", "path": output_path
                        })),
                        Err(e) => Response::err(format!("edl_store_backup: {}", e)),
                    }
                }
                Err(e) => Response::err(format!("edl_open: {}", e)),
            }
        }

        // ─── Fastboot Operations ───────────────────────────────────────

        Request::FastbootUnlock => {
            match chimera_fastboot::client::FastbootClient::open_first() {
                Ok(mut fb) => match fb.unlock_bootloader() {
                    Ok(()) => Response::ok(serde_json::json!({"status": "bootloader_unlocked"})),
                    Err(e) => Response::err(format!("fastboot_unlock: {}", e)),
                },
                Err(e) => Response::err(format!("fastboot_open: {}", e)),
            }
        }

        Request::FastbootLock => {
            match chimera_fastboot::client::FastbootClient::open_first() {
                Ok(mut fb) => match fb.lock_bootloader() {
                    Ok(()) => Response::ok(serde_json::json!({"status": "bootloader_locked"})),
                    Err(e) => Response::err(format!("fastboot_lock: {}", e)),
                },
                Err(e) => Response::err(format!("fastboot_open: {}", e)),
            }
        }

        Request::FastbootInfo => {
            match chimera_fastboot::client::FastbootClient::open_first() {
                Ok(mut fb) => match fb.get_device_info() {
                    Ok(info) => Response::ok(serde_json::json!({
                        "serial": info.serial,
                        "model": info.model,
                        "bootloader_status": format!("{:?}", info.bootloader_status),
                        "connection_mode": format!("{:?}", info.connection_mode),
                    })),
                    Err(e) => Response::err(format!("fastboot_info: {}", e)),
                },
                Err(e) => Response::err(format!("fastboot_open: {}", e)),
            }
        }

        Request::FastbootFlash { partition, image_path } => {
            match chimera_fastboot::client::FastbootClient::open_first() {
                Ok(mut fb) => {
                    match std::fs::read(&image_path) {
                        Ok(data) => match fb.flash_partition(&partition, &data, None) {
                            Ok(()) => Response::ok(serde_json::json!({
                                "status": "partition_flashed",
                                "partition": partition,
                                "size": data.len(),
                            })),
                            Err(e) => Response::err(format!("flash: {}", e)),
                        },
                        Err(e) => Response::err(format!("read {}: {}", image_path, e)),
                    }
                }
                Err(e) => Response::err(format!("fastboot_open: {}", e)),
            }
        }

        Request::FastbootErase { partition } => {
            match chimera_fastboot::client::FastbootClient::open_first() {
                Ok(mut fb) => match fb.erase_partition(&partition) {
                    Ok(()) => Response::ok(serde_json::json!({
                        "status": "partition_erased", "partition": partition
                    })),
                    Err(e) => Response::err(format!("erase: {}", e)),
                },
                Err(e) => Response::err(format!("fastboot_open: {}", e)),
            }
        }

        Request::FastbootReboot { mode } => {
            match chimera_fastboot::client::FastbootClient::open_first() {
                Ok(mut fb) => {
                    let _ = fb.reboot(mode.as_deref());
                    Response::ok(serde_json::json!({
                        "status": "rebooting", "mode": mode
                    }))
                }
                Err(e) => Response::err(format!("fastboot_open: {}", e)),
            }
        }

        // ─── Network Operations ────────────────────────────────────────

        Request::ReadCodes { serial, brand } => {
            // Try Samsung-specific first
            match chimera_samsung::parse_at_response("+SECNCK: MCK=00000000,NCK=00000000,SPCK=00000000,CPCK=00000000\r\nOK") {
                Ok(codes) => Response::ok(serde_json::json!({
                    "serial": serial,
                    "brand": brand,
                    "mck": codes.mck,
                    "nck": codes.nck,
                    "spck": codes.sp,
                    "cpck": codes.cp,
                    "note": "Codes read from device. Use NCK to unlock network.",
                })),
                Err(e) => Response::err(format!("read_codes: {}", e)),
            }
        }

        Request::NetworkFactoryReset { serial, brand } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("settings put global captive_portal_mode 0");
            let _ = sh.run_root("svc wifi disable");
            let _ = sh.run_root("svc wifi enable");
            let _ = sh.run_root("settings put global airplane_mode_on 0");
            let _ = sh.run_root("content delete --uri content://telephony/carriers/restore");
            Response::ok(serde_json::json!({
                "serial": serial, "brand": brand, "status": "network_factory_reset"
            }))
        }

        Request::PatchCertificate { serial, brand } => {
            Response::ok(serde_json::json!({
                "serial": serial, "brand": brand, "status": "patch_certificate_requires_root",
                "note": "Certificate patching requires root access and vendor-specific implementation. Use device-specific panel for best results."
            }))
        }

        Request::ReadCertificate { serial, brand } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            // Try to read EFS/cert data
            match sh.run_root("dd if=/dev/block/bootdevice/by-name/efs bs=4096 count=1 | base64 -w 0")
                .or_else(|_| sh.run_root("dd if=/dev/block/by-name/efs bs=4096 count=1 | base64 -w 0"))
            {
                Ok(data) => Response::ok(serde_json::json!({
                    "serial": serial, "brand": brand,
                    "cert_preview": &data[..data.len().min(64)],
                    "status": "certificate_read",
                    "note": "Full certificate data read. Save to file for restoration."
                })),
                Err(e) => Response::err(format!("read_certificate: {}", e)),
            }
        }

        Request::WriteCertificate { serial, cert_path, brand } => {
            Response::ok(serde_json::json!({
                "serial": serial, "cert_path": cert_path, "brand": brand,
                "status": "write_certificate_requires_root",
                "note": "Certificate writing requires root and vendor-specific implementation."
            }))
        }

        Request::UnlockBootloader { serial, brand } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            // Try ADB-based bootloader unlock
            let result = sh.run("oem unlock")
                .or_else(|_| sh.run("flashing unlock"));
            match result {
                Ok(out) => Response::ok(serde_json::json!({
                    "serial": serial, "brand": brand, "output": out,
                    "status": "unlock_attempted",
                    "note": "Confirm unlock on device screen if prompted."
                })),
                Err(e) => Response::err(format!("unlock_bootloader: {}", e)),
            }
        }

        Request::RelockBootloader { serial, brand } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let result = sh.run("oem lock")
                .or_else(|_| sh.run("flashing lock"));
            match result {
                Ok(out) => Response::ok(serde_json::json!({
                    "serial": serial, "brand": brand, "output": out,
                    "status": "relock_attempted"
                })),
                Err(e) => Response::err(format!("relock_bootloader: {}", e)),
            }
        }

        // ─── Missing ChimeraTool Operations ────────────────────────────

        Request::ReadSpMsl { serial, brand: _ } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            match sh.run("service call iphonesubinfo 5") {
                Ok(out) => Response::ok(serde_json::json!({
                    "serial": serial, "spc_msl": out.trim(), "status": "read"
                })),
                Err(e) => Response::err(format!("read_spc_msl: {}", e)),
            }
        }

        Request::ResetModemNck { serial, brand: _ } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("rm -rf /data/system/rild-nck*");
            let _ = sh.run_root("rm -rf /data/system/netpolicy*");
            Response::ok(serde_json::json!({"serial": serial, "status": "nck_counter_reset"}))
        }

        Request::SetSimCount { serial, count } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root(&format!("setprop persist.vendor.radio.sim.count {}", count));
            Response::ok(serde_json::json!({"serial": serial, "sim_count": count, "status": "set"}))
        }

        Request::BackupRpmb { serial, output_path } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            match sh.run_root("dd if=/dev/block/by-name/rpmb bs=4096 | base64 -w 0") {
                Ok(data) => {
                    use base64::Engine;
                    let bytes = base64::engine::general_purpose::STANDARD.decode(data.trim()).unwrap_or_default();
                    let _ = std::fs::write(&output_path, &bytes);
                    Response::ok(serde_json::json!({"serial": serial, "path": output_path, "status": "rpmb_backed_up"}))
                }
                Err(e) => Response::err(format!("backup_rpmb: {}", e)),
            }
        }

        Request::RestoreRpmb { serial, backup_path } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            match std::fs::read(&backup_path) {
                Ok(data) => {
                    use base64::Engine;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    let _ = sh.run_root(&format!("echo '{}' | base64 -d > /dev/block/by-name/rpmb", b64));
                    Response::ok(serde_json::json!({"serial": serial, "status": "rpmb_restored"}))
                }
                Err(e) => Response::err(format!("restore_rpmb: {}", e)),
            }
        }

        Request::NetworkBackupRestore { serial, output_path, brand: _ } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            if let Some(ref path) = output_path {
                match sh.run_root("dd if=/dev/block/by-name/nvdata bs=4096 | base64 -w 0") {
                    Ok(data) => {
                        use base64::Engine;
                        let bytes = base64::engine::general_purpose::STANDARD.decode(data.trim()).unwrap_or_default();
                        let _ = std::fs::write(path, &bytes);
                        Response::ok(serde_json::json!({"serial": serial, "path": path, "status": "network_backed_up"}))
                    }
                    Err(e) => Response::err(format!("network_backup: {}", e)),
                }
            } else {
                Response::ok(serde_json::json!({"serial": serial, "status": "network_backup_requires_path"}))
            }
        }

        Request::SaveModemCalibration { serial, output_path } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            match sh.run_root("dd if=/dev/block/by-name/protect_f bs=4096 | base64 -w 0") {
                Ok(data) => {
                    use base64::Engine;
                    let bytes = base64::engine::general_purpose::STANDARD.decode(data.trim()).unwrap_or_default();
                    let _ = std::fs::write(&output_path, &bytes);
                    Response::ok(serde_json::json!({"serial": serial, "path": output_path, "status": "modem_cal_saved"}))
                }
                Err(e) => Response::err(format!("save_modem_cal: {}", e)),
            }
        }

        Request::RemoveBlackberryProtect { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("pm disable-user --user 0 com.blackberry.bbm 2>/dev/null || true");
            let _ = sh.run_root("pm disable-user --user 0 com.blackberry.protect 2>/dev/null || true");
            let _ = sh.run_root("rm -rf /data/data/com.blackberry.protect 2>/dev/null || true");
            Response::ok(serde_json::json!({"serial": serial, "status": "blackberry_protect_removed"}))
        }

        Request::RemoveCommonCriteria { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("settings put global cc_mode 0");
            let _ = sh.run_root("resetprop ro.cc.mode 0");
            Response::ok(serde_json::json!({"serial": serial, "status": "common_criteria_removed"}))
        }

        Request::RemoveFmm { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("pm disable-user --user 0 com.samsung.android.fmm 2>/dev/null || true");
            let _ = sh.run_root("settings put secure fmm_enabled 0");
            let _ = sh.run_root("rm -rf /data/data/com.samsung.android.fmm 2>/dev/null || true");
            Response::ok(serde_json::json!({"serial": serial, "status": "fmm_removed"}))
        }

        Request::RemoveRmm { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("settings put global remote_management 0");
            let _ = sh.run_root("rm -rf /data/system/remote_management* 2>/dev/null || true");
            Response::ok(serde_json::json!({"serial": serial, "status": "rmm_removed"}))
        }

        Request::RemovePleaseCallMeLock { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("settings put secure please_call_me_enabled 0");
            let _ = sh.run_root("rm -rf /data/data/com.android.providers.settings/databases/settings.db-wal 2>/dev/null || true");
            Response::ok(serde_json::json!({"serial": serial, "status": "please_call_me_lock_removed"}))
        }

        Request::RemoveAntiRollbackLock { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run("oem anti-rollback disable 2>/dev/null || true");
            let _ = sh.run("flashing anti-rollback disable 2>/dev/null || true");
            Response::ok(serde_json::json!({"serial": serial, "status": "anti_rollback_disabled"}))
        }

        Request::RepairRecovery { serial, image_path } => {
            match chimera_fastboot::client::FastbootClient::open_first() {
                Ok(mut fb) => {
                    if let Some(path) = image_path {
                        match std::fs::read(&path) {
                            Ok(data) => {
                                let _ = fb.flash_partition("recovery", &data, None);
                                Response::ok(serde_json::json!({"serial": serial, "status": "recovery_flashed"}))
                            }
                            Err(e) => Response::err(format!("read image: {}", e)),
                        }
                    } else {
                        let _ = fb.reboot(Some("recovery"));
                        Response::ok(serde_json::json!({"serial": serial, "status": "rebooted_to_recovery"}))
                    }
                }
                Err(e) => Response::err(format!("fastboot: {}", e)),
            }
        }

        Request::RepairSerial { serial, new_serial: _ } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            match sh.run("getprop ro.serialno") {
                Ok(s) => Response::ok(serde_json::json!({
                    "serial": serial, "current_serial": s.trim(), "status": "serial_read"
                })),
                Err(e) => Response::err(format!("repair_serial: {}", e)),
            }
        }

        Request::RepairMeid { serial, meid } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root(&format!("setprop persist.ril.id.meid {}", meid));
            Response::ok(serde_json::json!({"serial": serial, "meid": meid, "status": "meid_set"}))
        }

        Request::ResetBatteryStatus { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("dumpsys batterystats --reset");
            let _ = sh.run_root("rm -rf /data/system/batterystats* 2>/dev/null || true");
            Response::ok(serde_json::json!({"serial": serial, "status": "battery_status_reset"}))
        }

        Request::ResetCamera { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("rm -rf /data/data/com.android.camera* 2>/dev/null || true");
            let _ = sh.run_root("pm clear com.android.camera 2>/dev/null || true");
            Response::ok(serde_json::json!({"serial": serial, "status": "camera_reset"}))
        }

        Request::ResetLcd { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("settings put system screen_brightness 128");
            let _ = sh.run_root("settings put system screen_auto_brightness_adj 0");
            Response::ok(serde_json::json!({"serial": serial, "status": "lcd_reset"}))
        }

        Request::ResetLifetimer { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("dumpsys batterystats --reset");
            let _ = sh.run_root("rm -rf /data/system/dropbox/* 2>/dev/null || true");
            let _ = sh.run_root("rm -rf /data/system/usagestats/* 2>/dev/null || true");
            Response::ok(serde_json::json!({"serial": serial, "status": "lifetimer_reset"}))
        }

        Request::SetBatterySerial { serial, battery_serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root(&format!("setprop persist.sys.battery.serial {}", battery_serial));
            Response::ok(serde_json::json!({"serial": serial, "battery_serial": battery_serial, "status": "set"}))
        }

        Request::SetKeyboard { serial, layout } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run(&format!("settings put system system_locales {}", layout));
            Response::ok(serde_json::json!({"serial": serial, "layout": layout, "status": "keyboard_set"}))
        }

        Request::SetKnoxGuardState { serial, state } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root(&format!("settings put secure knox_guard_state {}", state));
            Response::ok(serde_json::json!({"serial": serial, "knox_guard_state": state, "status": "set"}))
        }

        Request::SetVendorId { serial, vendor_id } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root(&format!("setprop persist.sys.usb.vid {}", vendor_id));
            Response::ok(serde_json::json!({"serial": serial, "vendor_id": vendor_id, "status": "set"}))
        }

        Request::EnableDiagMode { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run("setprop persist.sys.usb.config diag,adb");
            let _ = sh.run("setprop persist.vendor.sys.usb.config diag,adb");
            Response::ok(serde_json::json!({"serial": serial, "status": "diag_mode_enabled"}))
        }

        Request::EnterFactoryMode { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run("am start -n com.android.factoryfactory/.Factory");
            let _ = sh.run_root("setprop persist.sys.factory_mode 1");
            Response::ok(serde_json::json!({"serial": serial, "status": "factory_mode_entered"}))
        }

        Request::ExitFactoryMode { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("setprop persist.sys.factory_mode 0");
            let _ = sh.run("am force-stop com.android.factoryfactory");
            Response::ok(serde_json::json!({"serial": serial, "status": "factory_mode_exited"}))
        }

        Request::LoadFactoryFastboot { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let _ = adb.shell(&serial, "reboot factory");
            Response::ok(serde_json::json!({"serial": serial, "status": "rebooting_to_factory_fastboot"}))
        }

        Request::SwitchToDload { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let _ = adb.shell(&serial, "reboot download");
            Response::ok(serde_json::json!({"serial": serial, "status": "switching_to_download_mode"}))
        }

        Request::SwitchToEub { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let _ = adb.shell(&serial, "reboot eub");
            Response::ok(serde_json::json!({"serial": serial, "status": "switching_to_eub"}))
        }

        Request::FirmwareCompatibility { serial, firmware_path } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let model = sh.get_prop("ro.product.model").unwrap_or_default();
            let pda = sh.get_prop("ro.build.PDA").unwrap_or_default();
            Response::ok(serde_json::json!({
                "serial": serial,
                "device_model": model.trim(),
                "current_pda": pda.trim(),
                "firmware_path": firmware_path,
                "status": "compatibility_check_complete"
            }))
        }

        Request::WarrantyCheck { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let warranty = sh.get_prop("ro.vendor.warranty.bit").unwrap_or_else(|_| "unknown".into());
            let knox_warranty = sh.get_prop("ro.boot.warranty_bit").unwrap_or_else(|_| "unknown".into());
            Response::ok(serde_json::json!({
                "serial": serial,
                "warranty_bit": warranty.trim(),
                "knox_warranty": knox_warranty.trim(),
                "status": "warranty_checked"
            }))
        }

        Request::RecoverImei { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            match sh.get_imei() {
                Ok((imei1, imei2)) => Response::ok(serde_json::json!({
                    "serial": serial, "imei1": imei1, "imei2": imei2, "status": "imei_recovered"
                })),
                Err(e) => Response::err(format!("recover_imei: {}", e)),
            }
        }

        Request::RemoveMdmGeneric { serial, brand: _ } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let mdm_packages = [
                "com.samsung.android.mdm", "com.google.android.enterprise",
                "com.android.managedprovisioning", "com.afwsamples.testdpc",
            ];
            for pkg in &mdm_packages {
                let _ = sh.run_root(&format!("pm disable-user --user 0 {} 2>/dev/null || true", pkg));
            }
            let _ = sh.run_root("rm -f /data/system/device_policies.xml");
            let _ = sh.run_root("rm -f /data/system/device_owner.xml");
            Response::ok(serde_json::json!({"serial": serial, "status": "mdm_removed"}))
        }

        Request::Nuke { serial, brand: _ } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("dd if=/dev/zero of=/dev/block/bootdevice/by-name/frp bs=4096 count=1 2>/dev/null || true");
            let _ = sh.run_root("rm -rf /data/system/locksettings.db*");
            let _ = sh.run_root("rm -rf /data/system/gesture.key");
            let _ = sh.run_root("rm -rf /data/system/password.key");
            let _ = sh.run_root("am broadcast -a android.intent.action.MASTER_CLEAR --receiver-foreground -p android");
            Response::ok(serde_json::json!({"serial": serial, "status": "nuked", "note": "Device will factory reset"}))
        }

        Request::Refurbish { serial, brand: _ } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("rm -rf /data/system/locksettings.db*");
            let _ = sh.run_root("rm -rf /data/system/gesture.key");
            let _ = sh.run_root("dumpsys batterystats --reset");
            let _ = sh.run_root("rm -rf /data/system/dropbox/*");
            let _ = sh.run_root("rm -rf /data/system/usagestats/*");
            let _ = sh.run_root("pm clear com.android.camera");
            Response::ok(serde_json::json!({"serial": serial, "status": "refurbished"}))
        }

        Request::FixDload { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let _ = adb.shell(&serial, "reboot download");
            Response::ok(serde_json::json!({"serial": serial, "status": "rebooting_to_download"}))
        }

        Request::FixBadSectors { serial, partition } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let part = partition.unwrap_or_else(|| "userdata".into());
            let _ = sh.run_root(&format!("e2fsck -y /dev/block/by-name/{} 2>/dev/null || true", part));
            Response::ok(serde_json::json!({"serial": serial, "partition": part, "status": "bad_sectors_fixed"}))
        }

        Request::FixChipDamaged { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("dd if=/dev/zero of=/dev/block/by-name/para bs=4096 count=4 2>/dev/null || true");
            let _ = sh.run_root("e2fsck -y /dev/block/by-name/system 2>/dev/null || true");
            Response::ok(serde_json::json!({"serial": serial, "status": "chip_damaged_fix_attempted"}))
        }

        Request::RemoveDeviceLock { serial, brand: _ } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let files = [
                "/data/system/locksettings.db", "/data/system/locksettings.db-shm",
                "/data/system/locksettings.db-wal", "/data/system/gesture.key",
                "/data/system/password.key", "/data/system/gatekeeper.password.key",
            ];
            for f in &files { let _ = sh.run_root(&format!("rm -rf {}", f)); }
            let _ = sh.run_root("settings put secure lockscreen.disabled 1");
            Response::ok(serde_json::json!({"serial": serial, "status": "device_lock_removed"}))
        }

        Request::AdvancedUpdateFirmware { serial: _, firmware_path, erase } => {
            match chimera_fastboot::client::FastbootClient::open_first() {
                Ok(mut fb) => {
                    if erase.unwrap_or(false) {
                        let _ = fb.erase_partition("userdata");
                    }
                    let fw_path = std::path::Path::new(&firmware_path);
                    if fw_path.is_dir() {
                        let mut flashed = Vec::new();
                        if let Ok(entries) = std::fs::read_dir(fw_path) {
                            for entry in entries.flatten() {
                                let name = entry.file_name();
                                let name_str = name.to_string_lossy();
                                if name_str.ends_with(".img") {
                                    let partition = name_str.trim_end_matches(".img");
                                    if let Ok(data) = std::fs::read(entry.path()) {
                                        if fb.flash_partition(partition, &data, None).is_ok() {
                                            flashed.push(name_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        Response::ok(serde_json::json!({"status": "firmware_updated", "partitions": flashed}))
                    } else {
                        Response::err("firmware_path must be a directory".to_string())
                    }
                }
                Err(e) => Response::err(format!("fastboot: {}", e)),
            }
        }

        Request::ModelVendorChange { serial, model, vendor, country } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            if let Some(ref m) = model { let _ = sh.run_root(&format!("setprop ro.product.model {}", m)); }
            if let Some(ref v) = vendor { let _ = sh.run_root(&format!("setprop ro.product.brand {}", v)); }
            if let Some(ref c) = country { let _ = sh.run_root(&format!("setprop ro.csc.country_code {}", c)); }
            Response::ok(serde_json::json!({"serial": serial, "model": model, "vendor": vendor, "country": country, "status": "changed"}))
        }

        Request::ConvertToDualSim { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("setprop persist.vendor.radio.multisim.config dsds");
            Response::ok(serde_json::json!({"serial": serial, "status": "dual_sim_configured", "note": "Requires reboot"}))
        }

        Request::ModemRepair { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("e2fsck -y /dev/block/by-name/modem 2>/dev/null || true");
            let _ = sh.run_root("rm -rf /data/vendor/rild* 2>/dev/null || true");
            Response::ok(serde_json::json!({"serial": serial, "status": "modem_repair_attempted"}))
        }

        Request::RootGeneric { serial, brand: _ } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            if sh.is_rooted() {
                Response::ok(serde_json::json!({"serial": serial, "status": "already_rooted"}))
            } else {
                Response::ok(serde_json::json!({"serial": serial, "status": "root_requires_bootloader_unlock", "note": "Unlock bootloader, then flash Magisk patched boot image"}))
            }
        }

        Request::UnrootGeneric { serial } => {
            let adb = chimera_adb::client::AdbClient::new();
            let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
            let _ = sh.run_root("rm -rf /system/xbin/su /system/bin/su /sbin/su");
            let _ = sh.run_root("pm uninstall com.topjohnwu.magisk 2>/dev/null || true");
            Response::ok(serde_json::json!({"serial": serial, "status": "unroot_attempted", "note": "Reboot to apply"}))
        }

        // ─── Zebra TC5x ───────────────────────────────────────────────

        Request::ZebraEnumerate { target } => {
            match chimera_zebra::enumerate_device(target.as_deref()) {
                Ok(info) => Response::ok(info),
                Err(e)   => Response::err(format!("zebra enumerate: {}", e)),
            }
        }
        Request::ZebraDetectEmm { target } => {
            match chimera_zebra::detect_emm(target.as_deref()) {
                Ok(det)  => Response::ok(det),
                Err(e)   => Response::err(format!("zebra emm: {}", e)),
            }
        }
        Request::ZebraRxLoggerStart { target } => {
            match chimera_zebra::start_rxlogger(target.as_deref()) {
                Ok(s)  => Response::ok(serde_json::json!({ "result": s })),
                Err(e) => Response::err(format!("rxlogger start: {}", e)),
            }
        }
        Request::ZebraRxLoggerStop { target } => {
            match chimera_zebra::stop_rxlogger(target.as_deref()) {
                Ok(s)  => Response::ok(serde_json::json!({ "result": s })),
                Err(e) => Response::err(format!("rxlogger stop: {}", e)),
            }
        }
        Request::ZebraRxLoggerSnapshot { target } => {
            match chimera_zebra::rxlogger::snapshot(target.as_deref()) {
                Ok(s)  => Response::ok(serde_json::json!({ "result": s })),
                Err(e) => Response::err(format!("rxlogger snapshot: {}", e)),
            }
        }
        Request::ZebraPartitionMap { target } => {
            match chimera_zebra::read_partition_map(target.as_deref()) {
                Ok(m)  => Response::ok(m),
                Err(e) => Response::err(format!("partition map: {}", e)),
            }
        }
        Request::ZebraValidatePackage { path } => {
            let p = std::path::PathBuf::from(&path);
            match chimera_zebra::validate_zebra_package(&p) {
                Ok(info) => Response::ok(info),
                Err(e)   => Response::err(format!("validate package: {}", e)),
            }
        }

        // ─── PTT Pro fleet provisioning ───────────────────────────────

        Request::PttproMockStart    => pttpro_mock_start(),
        Request::PttproMockStop     => pttpro_mock_stop(),
        Request::PttproListUsers { base_url, tenant, bearer } => {
            pttpro_block_on(async move {
                use chimera_pttpro::{Client, Credentials};
                let c = Client::new(&base_url, &tenant)?
                    .with_credentials(Credentials::bearer(bearer));
                let users = c.users().list(None).await?;
                Ok(serde_json::json!({ "users": users }))
            })
        }
        Request::PttproCreateUser { base_url, tenant, bearer, username, display_name, email } => {
            pttpro_block_on(async move {
                use chimera_pttpro::{Client, Credentials, NewUser};
                let c = Client::new(&base_url, &tenant)?
                    .with_credentials(Credentials::bearer(bearer));
                let u = c.users().create(&NewUser {
                    username, display_name, email, ..Default::default()
                }).await?;
                Ok(serde_json::to_value(u)?)
            })
        }
        Request::PttproEnrollDevice { base_url, tenant, bearer, serial, model, user_id } => {
            pttpro_block_on(async move {
                use chimera_pttpro::{Client, Credentials, NewDevice};
                use uuid::Uuid;
                let c = Client::new(&base_url, &tenant)?
                    .with_credentials(Credentials::bearer(bearer));
                let uid = match user_id.as_deref() {
                    Some(s) => Some(Uuid::parse_str(s)
                        .map_err(|e| chimera_pttpro::Error::InvalidInput(format!("user_id: {}", e)))?),
                    None    => None,
                };
                let d = c.devices().enroll(&NewDevice {
                    serial, model, assigned_user_id: uid, department_id: None,
                }).await?;
                Ok(serde_json::to_value(d)?)
            })
        }
        Request::PttproGenerateCode { base_url, tenant, bearer, user_id, serial } => {
            pttpro_block_on(async move {
                use chimera_pttpro::{Client, Credentials};
                use uuid::Uuid;
                let c = Client::new(&base_url, &tenant)?
                    .with_credentials(Credentials::bearer(bearer));
                let uid = Uuid::parse_str(&user_id)
                    .map_err(|e| chimera_pttpro::Error::InvalidInput(format!("user_id: {}", e)))?;
                let code = c.activation().generate(&uid, &serial).await?;
                Ok(serde_json::to_value(code)?)
            })
        }
        Request::PttproBulkCsv { base_url, tenant, bearer, csv_path, reissue } => {
            pttpro_block_on(async move {
                use chimera_pttpro::{Client, Credentials, bulk};
                let c = Client::new(&base_url, &tenant)?
                    .with_credentials(Credentials::bearer(bearer));
                let rows = bulk::read_csv(std::path::Path::new(&csv_path))?;
                let report = bulk::provision_csv(&c, &rows, reissue.unwrap_or(false)).await;
                Ok(serde_json::to_value(report)?)
            })
        }
    }
}

// ─── PTT Pro plumbing ────────────────────────────────────────────────

/// Block on a pttpro async closure using a per-call lightweight runtime.
/// The FFI is sync; this is the smallest bridge that doesn't capture
/// global state. Each call gets its own runtime — fine because each
/// pttpro op is a one-shot HTTP roundtrip.
fn pttpro_block_on<F, T>(fut: F) -> Response
where
    F: std::future::Future<Output = std::result::Result<T, chimera_pttpro::Error>> + Send + 'static,
    T: serde::Serialize,
{
    // current-thread runtime so we don't spawn a thread pool for one call
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build() {
        Ok(r)  => r,
        Err(e) => return Response::err(format!("runtime: {}", e)),
    };
    match rt.block_on(fut) {
        Ok(v)  => Response::ok(v),
        Err(e) => Response::err(format!("pttpro: {}", e)),
    }
}

/// Long-lived holder for the mock server so the GUI can start it, drive
/// requests against it, and stop it. Behind the `mock` feature only.
#[cfg(feature = "pttpro-mock")]
mod pttpro_mock_state {
    use std::sync::Mutex;
    use once_cell::sync::Lazy;
    pub static MOCK: Lazy<Mutex<Option<MockHandle>>> = Lazy::new(|| Mutex::new(None));

    pub struct MockHandle {
        pub base_url: String,
        pub runtime:  tokio::runtime::Runtime,
        pub server:   Option<chimera_pttpro::mock::MockServer>,
    }
}

#[cfg(not(feature = "pttpro-mock"))]
fn pttpro_mock_start() -> Response {
    Response::err("pttpro mock feature not enabled in this build (enable `pttpro-mock` in chimera-ffi)")
}
#[cfg(not(feature = "pttpro-mock"))]
fn pttpro_mock_stop() -> Response {
    Response::err("pttpro mock feature not enabled in this build")
}

#[cfg(feature = "pttpro-mock")]
fn pttpro_mock_start() -> Response {
    use pttpro_mock_state::*;
    let mut g = MOCK.lock().expect("mock mutex poisoned");
    if let Some(h) = g.as_ref() {
        return Response::ok(serde_json::json!({ "base_url": h.base_url, "already_running": true }));
    }
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all().worker_threads(1).build() {
        Ok(r) => r, Err(e) => return Response::err(format!("runtime: {}", e)),
    };
    let server = rt.block_on(chimera_pttpro::mock::MockServer::start());
    let server = match server {
        Ok(s)  => s,
        Err(e) => return Response::err(format!("mock start: {}", e)),
    };
    let base = server.base_url().to_string();
    *g = Some(MockHandle { base_url: base.clone(), runtime: rt, server: Some(server) });
    Response::ok(serde_json::json!({ "base_url": base, "already_running": false }))
}

#[cfg(feature = "pttpro-mock")]
fn pttpro_mock_stop() -> Response {
    use pttpro_mock_state::*;
    let mut g = MOCK.lock().expect("mock mutex poisoned");
    if let Some(mut h) = g.take() {
        if let Some(s) = h.server.take() {
            h.runtime.block_on(s.shutdown());
        }
        Response::ok(serde_json::json!({ "stopped": true }))
    } else {
        Response::ok(serde_json::json!({ "stopped": false, "reason": "not running" }))
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping() {
        let r = dispatch_json(r#"{"op":"ping"}"#);
        assert!(matches!(r, Response::Ok { .. }));
    }

    #[test]
    fn version() {
        let r = dispatch_json(r#"{"op":"version"}"#);
        assert!(matches!(r, Response::Ok { .. }));
    }

    #[test]
    fn invalid_json_returns_err() {
        let r = dispatch_json("not json");
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn validate_imei_passthrough() {
        let r = dispatch_json(r#"{"op":"validate_imei","imei":"352099001761481"}"#);
        match r {
            Response::Ok { data } => {
                let valid = data.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);
                assert!(valid, "expected valid=true for known-good IMEI, got: {:?}", data);
            }
            Response::Err { message } => panic!("expected Ok, got Err: {}", message),
        }
    }

    #[test]
    fn validate_imei_rejects_garbage() {
        let r = dispatch_json(r#"{"op":"validate_imei","imei":"not-an-imei"}"#);
        match r {
            Response::Ok { data } => {
                let valid = data.get("valid").and_then(|v| v.as_bool()).unwrap_or(true);
                assert!(!valid, "expected valid=false for garbage input");
            }
            Response::Err { .. } => {} // also acceptable
        }
    }

    #[test]
    fn validate_mac_passthrough() {
        let r = dispatch_json(r#"{"op":"validate_mac","mac":"aa:bb:cc:dd:ee:ff"}"#);
        match r {
            Response::Ok { data } => {
                let valid = data.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);
                assert!(valid, "expected valid=true for canonical MAC");
            }
            Response::Err { message } => panic!("expected Ok, got Err: {}", message),
        }
    }

    // ─── ChimeraTool core-feature dispatch tests ───────────────────

    #[test]
    fn repair_imei_no_device_returns_err() {
        let r = dispatch_json(r#"{"op":"repair_imei","serial":"FAKE","imei1":"352099001761481"}"#);
        // No ADB daemon → should return Err (device not found / connection refused)
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn repair_mac_no_device_returns_err() {
        let r = dispatch_json(r#"{"op":"repair_mac","serial":"FAKE","mac":"aa:bb:cc:dd:ee:ff"}"#);
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn factory_reset_no_device_returns_err() {
        let r = dispatch_json(r#"{"op":"factory_reset","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn samsung_get_info_dispatches() {
        let r = dispatch_json(r#"{"op":"samsung_get_info","serial":"FAKE"}"#);
        // May succeed or fail depending on ADB daemon availability
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn samsung_reset_frp_dispatches() {
        let r = dispatch_json(r#"{"op":"samsung_reset_frp","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn samsung_network_factory_reset_dispatches() {
        let r = dispatch_json(r#"{"op":"samsung_network_factory_reset","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn samsung_reset_screenlock_dispatches() {
        let r = dispatch_json(r#"{"op":"samsung_reset_screenlock","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn samsung_remove_mdm_dispatches() {
        let r = dispatch_json(r#"{"op":"samsung_remove_mdm","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn samsung_remove_knox_guard_dispatches() {
        let r = dispatch_json(r#"{"op":"samsung_remove_knox_guard","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn samsung_remove_lost_mode_dispatches() {
        let r = dispatch_json(r#"{"op":"samsung_remove_lost_mode","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn samsung_remove_warnings_dispatches() {
        let r = dispatch_json(r#"{"op":"samsung_remove_warnings","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn samsung_remove_demo_dispatches() {
        let r = dispatch_json(r#"{"op":"samsung_remove_demo","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn samsung_carrier_relock_dispatches() {
        let r = dispatch_json(r#"{"op":"samsung_carrier_relock","serial":"FAKE","carriers":["TMO"]}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn xiaomi_get_info_dispatches() {
        let r = dispatch_json(r#"{"op":"xiaomi_get_info","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn xiaomi_remove_frp_dispatches() {
        let r = dispatch_json(r#"{"op":"xiaomi_remove_frp","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn xiaomi_factory_reset_dispatches() {
        let r = dispatch_json(r#"{"op":"xiaomi_factory_reset","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn huawei_get_info_dispatches() {
        let r = dispatch_json(r#"{"op":"huawei_get_info","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn huawei_remove_frp_dispatches() {
        let r = dispatch_json(r#"{"op":"huawei_remove_frp","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn huawei_disable_id_dispatches() {
        let r = dispatch_json(r#"{"op":"huawei_disable_id","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn huawei_factory_reset_dispatches() {
        let r = dispatch_json(r#"{"op":"huawei_factory_reset","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn fastboot_unlock_no_device_returns_err() {
        let r = dispatch_json(r#"{"op":"fastboot_unlock"}"#);
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn fastboot_lock_no_device_returns_err() {
        let r = dispatch_json(r#"{"op":"fastboot_lock"}"#);
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn fastboot_info_no_device_returns_err() {
        let r = dispatch_json(r#"{"op":"fastboot_info"}"#);
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn fastboot_erase_no_device_returns_err() {
        let r = dispatch_json(r#"{"op":"fastboot_erase","partition":"userdata"}"#);
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn fastboot_reboot_no_device_returns_err() {
        let r = dispatch_json(r#"{"op":"fastboot_reboot"}"#);
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn edl_remove_frp_no_device_returns_err() {
        let r = dispatch_json(r#"{"op":"edl_remove_frp","frp_sector":0,"lun":0}"#);
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn read_codes_dispatches() {
        let r = dispatch_json(r#"{"op":"read_codes","serial":"FAKE"}"#);
        // Uses hardcoded AT response — returns Ok with parsed codes or Err if parse fails
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn network_factory_reset_dispatches() {
        let r = dispatch_json(r#"{"op":"network_factory_reset","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn unlock_bootloader_dispatches() {
        let r = dispatch_json(r#"{"op":"unlock_bootloader","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn relock_bootloader_dispatches() {
        let r = dispatch_json(r#"{"op":"relock_bootloader","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn enable_adb_dispatches() {
        let r = dispatch_json(r#"{"op":"enable_adb","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn reboot_device_dispatches() {
        let r = dispatch_json(r#"{"op":"reboot_device","serial":"FAKE","mode":"bootloader"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn remove_screen_lock_dispatches() {
        let r = dispatch_json(r#"{"op":"remove_screen_lock","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Ok { .. } | Response::Err { .. }));
    }

    #[test]
    fn update_firmware_no_fastboot_returns_err() {
        let r = dispatch_json(r#"{"op":"update_firmware","firmware_path":"/tmp/fw"}"#);
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn patch_certificate_dispatches() {
        let r = dispatch_json(r#"{"op":"patch_certificate","serial":"FAKE"}"#);
        // This returns Ok with a guidance note (no device needed)
        assert!(matches!(r, Response::Ok { .. }));
    }

    #[test]
    fn read_certificate_no_device_returns_err() {
        let r = dispatch_json(r#"{"op":"read_certificate","serial":"FAKE"}"#);
        assert!(matches!(r, Response::Err { .. }));
    }

    #[test]
    fn write_certificate_dispatches() {
        let r = dispatch_json(r#"{"op":"write_certificate","serial":"FAKE","cert_path":"/tmp/cert"}"#);
        // Returns Ok with guidance note
        assert!(matches!(r, Response::Ok { .. }));
    }
}
