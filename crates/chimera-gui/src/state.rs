// crates/chimera-gui/src/state.rs
// Application state — extended with all Phase 0-11 additions
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_mut)]
use chimera_core::DeviceInfo;
use chimera_core::event::{ChimeraEvent, LogLevel};
use chimera_core::session::SessionManager;
use chimera_core::diagnostics::FullDiagnostics;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crossbeam_channel::{Sender, Receiver, unbounded};
use crate::worker::OperationRequest;
use crate::local_event::{LocalEvent, IpswEntry, DownloadTask, DownloadStatus};
use crate::history::HistoryEntry;
use chrono::Local;
use std::path::PathBuf;

// ── Log entry ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level:     LogLevel,
    pub message:   String,
}

impl LogEntry {
    pub fn info   (msg: impl Into<String>) -> Self { Self::new(LogLevel::Info,    msg) }
    pub fn success(msg: impl Into<String>) -> Self { Self::new(LogLevel::Success, msg) }
    pub fn error  (msg: impl Into<String>) -> Self { Self::new(LogLevel::Error,   msg) }
    pub fn warn   (msg: impl Into<String>) -> Self { Self::new(LogLevel::Warning, msg) }
    fn new(level: LogLevel, msg: impl Into<String>) -> Self {
        Self {
            timestamp: Local::now().format("%H:%M:%S%.3f").to_string(),
            level,
            message: msg.into(),
        }
    }
}

// ── Operation status ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum OperationStatus {
    Idle,
    Running { name: String, percent: f32, step: String },
    Success(String),
    Failed(String),
}

// ── Active tab (legacy, kept for compatibility) ────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ActiveTab {
    DeviceInfo, Operations, Firmware, Utilities, Diagnostics,
    History, Settings, Apple, AuNetworkUnlock, ApiTools, Log, ShshManager, MediaTek,
}

// ── Per-device UI state ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DeviceUiState {
    pub device:           DeviceInfo,
    pub operation_status: OperationStatus,
    pub imei_input:       String,
    pub imei2_input:      String,
    pub mac_input:        String,
    pub csc_input:        String,
    pub firmware_path:    String,
    pub backup_path:      String,
    pub diagnostics:      Option<FullDiagnostics>,
    pub ta_info:          Vec<(String, String)>,
    pub extra_notes:      String,
    pub magisk_apk_path:  String,
    pub twrp_image_path:  String,
}

impl DeviceUiState {
    pub fn new(device: DeviceInfo) -> Self {
        let imei  = device.imei.clone().unwrap_or_default();
        let imei2 = device.imei2.clone().unwrap_or_default();
        let mac   = device.mac_address.clone().unwrap_or_default();
        let csc   = device.csc.clone().unwrap_or_default();
        Self {
            device, operation_status: OperationStatus::Idle,
            imei_input: imei, imei2_input: imei2, mac_input: mac, csc_input: csc,
            firmware_path: String::new(), backup_path: String::new(),
            diagnostics: None, ta_info: Vec::new(), extra_notes: String::new(),
            magisk_apk_path: String::new(), twrp_image_path: String::new(),
        }
    }
}

// ── Apple UI types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AppleActivationStatus { Activated, ActivationRequired, Unactivated, Unknown }

#[derive(Debug, Clone)]
pub struct AppleActivationUiInfo {
    pub status:       AppleActivationStatus,
    pub account_hint: Option<String>,
    pub is_supervised:bool,
    pub mdm_org:      Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppleBypassMethodUI {
    Checkm8, Palera1n, EraseRestore, DnsServer, MdmDep,
}

impl AppleBypassMethodUI {
    pub fn label(&self) -> &str {
        match self {
            AppleBypassMethodUI::Checkm8      => "checkm8 (A5-A11)",
            AppleBypassMethodUI::Palera1n     => "palera1n (A9-A11)",
            AppleBypassMethodUI::EraseRestore => "Erase & Restore",
            AppleBypassMethodUI::DnsServer    => "DNS Activation Server",
            AppleBypassMethodUI::MdmDep       => "MDM/DEP Enrollment",
        }
    }
    pub fn description(&self) -> &str {
        match self {
            AppleBypassMethodUI::Checkm8      => "Bootrom exploit for A5-A11 chips. Unpatchable hardware vulnerability.",
            AppleBypassMethodUI::Palera1n     => "Semi-tethered jailbreak for A9-A11 running iOS 15-17.",
            AppleBypassMethodUI::EraseRestore => "Full erase via DFU. ALL data lost. Works on any device.",
            AppleBypassMethodUI::DnsServer    => "Legacy DNS trick. Low success rate on iOS 13+.",
            AppleBypassMethodUI::MdmDep       => "Enterprise MDM re-enrollment. Requires DEP authority.",
        }
    }
}

impl Default for AppleBypassMethodUI {
    fn default() -> Self { AppleBypassMethodUI::Checkm8 }
}

// ── SHSH tab ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ShshTab {
    #[default] SaveBlobs, LocalBlobs, DowngradeReport, FutureRestore, ErrorCatalogue,
}

// ── Confirmation dialog ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ConfirmDialog {
    pub title:      String,
    pub message:    String,
    pub on_confirm: OperationRequest,
}

// ── Application settings (persisted) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub adb_server_host:          String,
    pub adb_server_port:          u16,
    pub download_dir:             String,
    pub backup_dir:               String,
    pub dark_mode:                bool,
    pub font_size:                f32,
    pub auto_scan:                bool,
    pub log_to_file:              bool,
    pub show_developer_options:   bool,
    pub confirm_dangerous_ops:    bool,
    pub adb_path:                 String,
    pub fastboot_path:            String,
    pub futurerestore_path:       String,
    pub irecovery_path:           String,
    pub auto_backup_before_ops:   bool,
    pub max_log_lines:            usize,
    pub scan_interval_ms:         u64,
    pub use_system_proxy:         bool,
    pub proxy_url:                String,
    pub verify_tls:               bool,
    pub api_mock_mode:            bool,
    pub prevent_sleep:            bool,
    pub audible_alert:            bool,
    pub compact_sidebar:          bool,
    pub show_console:             bool,
    pub log_verbose:              bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        let home     = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let download = dirs::download_dir()
            .unwrap_or_else(|| home.join("Downloads"))
            .to_string_lossy().to_string();
        let data_backup = dirs::data_dir()
            .map(|d| d.join("ChimeraRS").join("backups"))
            .unwrap_or_else(|| home.join("Library").join("Application Support")
                .join("ChimeraRS").join("backups"))
            .to_string_lossy().to_string();
        Self {
            adb_server_host:        "127.0.0.1".into(),
            adb_server_port:        5037,
            download_dir:           download,
            backup_dir:             data_backup,
            dark_mode:              true,
            font_size:              13.0,
            auto_scan:              true,
            log_to_file:            false,
            show_developer_options: false,
            confirm_dangerous_ops:  true,
            adb_path:               String::new(),
            fastboot_path:          String::new(),
            futurerestore_path:     String::new(),
            irecovery_path:         String::new(),
            auto_backup_before_ops: false,
            max_log_lines:          1000,
            scan_interval_ms:       1000,
            use_system_proxy:       false,
            proxy_url:              String::new(),
            verify_tls:             true,
            api_mock_mode:          false,
            prevent_sleep:          true,
            audible_alert:          false,
            compact_sidebar:        false,
            show_console:           true,
            log_verbose:            false,
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════
// FULL APPLICATION STATE
// ══════════════════════════════════════════════════════════════════════════

pub struct AppState {
    // ── Core I/O ────────────────────────────────────────────────
    pub event_tx:           Sender<ChimeraEvent>,
    pub event_rx:           Receiver<ChimeraEvent>,
    /// Local GUI events from the worker (SSH lines, QR, IPSW results …)
    pub local_event_tx:     Sender<LocalEvent>,
    pub local_event_rx:     Receiver<LocalEvent>,
    /// Worker operation channel
    pub op_tx:              Option<Sender<OperationRequest>>,

    // ── Devices ─────────────────────────────────────────────────
    pub devices:              HashMap<String, DeviceUiState>,
    pub selected_device_id:   Option<String>,
    pub selected_device_ids:  HashSet<String>,   // multi-select
    pub active_tab:           ActiveTab,
    pub session:              SessionManager,

    // ── Logs ────────────────────────────────────────────────────
    pub log_entries:  Vec<LogEntry>,
    pub log_filter:   String,

    // ── History ─────────────────────────────────────────────────
    pub history:        Vec<HistoryEntry>,
    pub history_filter: String,

    // ── Settings ────────────────────────────────────────────────
    pub settings:       AppSettings,
    pub settings_dirty: bool,
    pub last_font_size: f32,

    // ── Navigation ──────────────────────────────────────────────
    pub current_page:      crate::ui::nav::Page,
    pub show_about_modal:  bool,
    pub show_about:        bool,
    pub pending_confirm:   Option<ConfirmDialog>,
    pub is_scanning:       bool,
    pub app_uptime_secs:   u64,
    pub dashboard_mode:    u8,

    // ── Apple device state ───────────────────────────────────────
    pub apple_device_info:      HashMap<String, chimera_apple::device::AppleDeviceInfo>,
    pub apple_activation_info:  HashMap<String, AppleActivationUiInfo>,
    pub apple_network_locked:   HashMap<String, bool>,
    pub apple_unlock_instructions: HashMap<String, String>,
    pub apple_tab:              crate::ui::apple_panel::AppleTab,
    pub apple_ipsw_path:        String,
    pub apple_erase_mode:       bool,
    pub apple_verify_tss:       bool,
    pub apple_skip_baseband:    bool,
    pub apple_download_dir:     String,
    pub apple_bypass_method:    AppleBypassMethodUI,
    pub apple_dns_server:       String,
    pub apple_au_carrier:       String,
    pub apple_carrier_account:  String,
    pub apple_erase_restore:    bool,
    pub apple_verify_sha1:      bool,
    pub apple_passcode_method:  String,
    pub apple_bypass_technique: String,
    pub apple_shsh_path:        String,

    // ── Downloads ────────────────────────────────────────────────
    pub downloads_tab:          crate::ui::downloads::DownloadsTab,
    pub ipsw_model_selected:    String,
    pub ipsw_search_query:      String,
    pub ipsw_search_results:    Vec<IpswEntry>,
    pub ipsw_searching:         bool,
    pub active_downloads:       Vec<DownloadTask>,
    pub samsung_fw_model:       String,
    pub samsung_fw_csc:         String,
    pub firmware_search_brand:  String,
    pub firmware_search_model:  String,
    pub firmware_search_region: String,
    pub firmware_search_results:Vec<String>,

    // ── SSH / VPN ────────────────────────────────────────────────
    pub ssh_tab:             crate::ui::ssh_panel::SshTab,
    pub ssh_tab2:            usize,
    pub ssh_host:            String,
    pub ssh_port:            String,
    pub ssh_username:        String,
    pub ssh_auth_method:     String,
    pub ssh_password:        String,
    pub ssh_key_path:        String,
    pub ssh_passphrase:      String,
    pub ssh_connected:       bool,
    pub ssh_terminal_output: String,
    pub ssh_command_input:   String,
    pub ssh_command_history: Vec<String>,
    pub ssh_history_idx:     usize,
    pub ssh_tunnels:         Vec<crate::ui::ssh_panel::SshTunnel>,
    pub ssh_new_local_port:  String,
    pub ssh_new_remote_host: String,
    pub ssh_new_remote_port: String,
    /// Live SSH I/O channels (set when connected)
    pub ssh_output_rx:       Option<crossbeam_channel::Receiver<String>>,
    pub ssh_input_tx:        Option<crossbeam_channel::Sender<String>>,

    // ── AU Unlock ────────────────────────────────────────────────
    pub au_unlock_imei:         String,
    pub au_unlock_brand:        String,
    pub au_unlock_carrier:      String,
    pub au_unlock_mccmnc:       String,
    pub au_unlock_nck_result:   Option<String>,
    pub au_unlock_instructions: Option<String>,
    pub au_show_carrier_table:  bool,
    pub au_wizard_imei:         String,
    pub au_wizard_carrier:      String,
    pub au_wizard_result:       Option<String>,
    pub au_validate_imei:       String,
    pub au_validate_result:     String,
    pub au_tab:                 usize,

    // ── MediaTek state ────────────────────────────────────────────
    pub mtk_tab:                crate::ui::mediatek_panel::MtkTab,
    pub mtk_connected:          bool,
    pub mtk_chipset:            Option<String>,
    pub mtk_da_path:            String,
    pub mtk_scatter_path:       String,
    pub mtk_partition_name:     String,
    pub mtk_output_path:        String,
    pub mtk_file_path:          String,
    pub mtk_imei_input:         String,
    pub mtk_chipset_search:     String,

    // ── Hash tools ───────────────────────────────────────────────
    pub hash_input:    String,
    pub hash_result:   String,
    pub hash_algo:     String,
    pub hash_hmac_key: String,

    // ── MAC tools ────────────────────────────────────────────────
    pub mac_input:           String,
    pub mac_validate_result: String,
    pub mac_derive_input:    String,
    pub mac_derive_result:   String,

    // ── Encode/Decode tools ──────────────────────────────────────
    pub encode_tab:     u8,  // 0=Base64 1=Hex 2=URL 3=JWT 4=NumConv
    pub encode_input:   String,
    pub encode_output:  String,
    pub jwt_input:      String,
    pub jwt_header:     String,
    pub jwt_payload:    String,
    pub numconv_input:  String,

    // ── ADB shell (Tools page) ───────────────────────────────────
    pub adb_shell_cmd:     String,
    pub adb_shell_output:  String,
    pub adb_shell_history: Vec<String>,

    // ── Network tools ────────────────────────────────────────────
    pub tcp_test_host:    String,
    pub tcp_test_port:    String,
    pub tcp_test_result:  String,
    pub dns_lookup_host:  String,
    pub dns_lookup_result:String,
    pub tools_probe_host: String,

    // ── API Tools ────────────────────────────────────────────────
    pub api_tab:              usize,
    pub api_base_url:         String,
    pub api_token:            String,
    pub api_response:         String,
    pub api_selected_endpoint:usize,
    pub api_request_body:     String,
    pub api_last_status:      String,
    pub api_mock_mode:        bool,
    pub api_imei_query:       String,
    pub api_firmware_model:   String,
    pub api_firmware_region:  String,
    pub api_latency_ms:       u64,
    pub api_requesting:       bool,
    pub api_probe_host:       String,
    pub api_probe_port:       String,

    // ── SHSH Blobs ───────────────────────────────────────────────
    pub shsh_tab:                usize,
    pub shsh_active_tab:         ShshTab,
    pub shsh_ecid_input:         String,
    pub shsh_model_input:        String,
    pub shsh_build_input:        String,
    pub shsh_blob_path:          String,
    pub shsh_nonce_gen:          String,
    pub shsh_use_latest_sep:     bool,
    pub shsh_use_latest_baseband:bool,
    pub shsh_report:             String,
    pub shsh_futurerestore_cmd:  String,
    pub shsh_saved_blobs:        Vec<String>,
    pub shsh_ecid:               String,
    pub shsh_board:              String,
    pub shsh_build:              String,
    pub shsh_ecid2:              String,
    pub shsh_model:              String,
    pub downgrade_ios:           String,
    pub futurerestore_ipsw:      String,
    pub futurerestore_shsh:      String,
    pub futurerestore_latest_sep:bool,
    pub futurerestore_latest_bb: bool,
    pub futurerestore_erase:     bool,
    pub futurerestore_running:   bool,
    pub futurerestore_log:       String,
    pub apnonce_generator:       String,

    // ── Jailbreak ────────────────────────────────────────────────
    pub jailbreak_tab:          usize,
    pub jb_strategy:            String,
    pub jb_flash_method:        String,
    pub jb_recovery_img_path:   String,
    pub jb_opt_verify_checksum: bool,
    pub jb_opt_stage_twrp:      bool,
    pub jb_opt_patch_boot:      bool,
    pub jb_twrp_apk_path:       String,
    pub jb_magisk_path:         String,
    pub jb_frp_manufacturer:    String,

    // ── Activation ───────────────────────────────────────────────
    pub activation_tab:           usize,
    pub activation_imei:          String,
    pub activation_bypass_method: String,
    pub activation_ecid:          String,

    // ── Misc tab indices ─────────────────────────────────────────
    pub devices_tab:     usize,
    pub downloads_tab2:  usize,
    pub utilities_tab:   usize,
    pub settings_tab:    usize,
    pub apple_ios_tab:   usize,
    pub network_tab:     usize,
    pub tools_tab:       usize,

    // ── Settings panel fields ────────────────────────────────────
    pub settings_use_system_proxy:    bool,
    pub settings_proxy_url:           String,
    pub settings_verify_tls:          bool,
    pub settings_api_mock:            bool,
    pub settings_adb_path:            String,
    pub settings_fastboot_path:       String,
    pub settings_futurerestore_path:  String,
    pub settings_irecovery_path:      String,
    pub settings_dark_mode:           bool,
    pub settings_amber_accent:        bool,
    pub settings_compact_sidebar:     bool,
    pub settings_show_console:        bool,
    pub settings_dot_grid:            bool,
    pub settings_grain:               bool,
    pub settings_shimmer:             bool,
    pub settings_auto_detect:         bool,
    pub settings_confirm_destructive: bool,
    pub settings_audible_alert:       bool,
    pub settings_prevent_sleep:       bool,

    // ── Misc UI fields ───────────────────────────────────────────
    pub imei_check_input:  String,
    pub imei_check_result: Option<String>,
    pub network_code_input:String,
    pub network_code_result:Option<String>,
    pub utility_imei_input:String,
    pub utility_nck_result:String,
    pub utility_imei_result:String,
    pub adb_tcp_host:      String,
    pub adb_tcp_port:      String,
    pub nck_imei_input:    String,
    pub nck_algo:          String,
    pub imei_decode_input: String,
    pub qr_input:          String,
    pub qr_svg:            Option<String>,   // rendered QR SVG
    pub network_sim_imei:  String,
    pub network_mcc_imei:  String,
    pub evlog_filter:      String,
    pub hostname: String,
    pub worker_ok: bool,
    pub adb_ok: bool,
    /// Where the detected adb binary lives, e.g. "/usr/local/bin/adb".
    pub adb_path:     Option<String>,
    /// The version line captured from `adb version`, for the dashboard.
    pub adb_version:  Option<String>,
    /// Last error from a failed ADB probe (binary missing / `adb version` failed).
    pub adb_error:    Option<String>,
    /// Full snapshot of every host-tool probe — used by the Diagnostics panel.
    pub host_tools:   chimera_utils::HostToolProbes,
    /// Wall-clock instant of the last ADB re-probe, used to throttle to ~5s.
    pub adb_last_probe: Option<std::time::Instant>,
    pub usb_ok: bool,
    pub build_ts: String,
    pub build_hash: String,
    pub started_at: std::time::Instant,
}

// ── Constructor ────────────────────────────────────────────────────────────

impl AppState {
    pub fn new() -> Self {
        let (event_tx, event_rx)       = unbounded::<ChimeraEvent>();
        let (local_event_tx, local_event_rx) = unbounded::<LocalEvent>();

        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let session_dir = dirs::data_dir()
            .map(|d| d.join("ChimeraRS"))
            .unwrap_or_else(|| home.join("Library").join("Application Support").join("ChimeraRS"));
        let session = SessionManager::new(&session_dir);

        // Load persisted settings and history
        let settings = crate::persistence::load_settings();
        let history  = crate::persistence::load_history();
        let shsh_saved_blobs = crate::persistence::list_shsh_blobs();
        let last_font_size = settings.font_size;

        let mut state = Self {
            event_tx, event_rx,
            local_event_tx, local_event_rx,
            op_tx: None,
            devices: HashMap::new(),
            selected_device_id: None,
            selected_device_ids: HashSet::new(),
            active_tab: ActiveTab::DeviceInfo,
            session,
            log_entries: Vec::new(),
            log_filter: String::new(),
            history,
            history_filter: String::new(),
            last_font_size,
            settings_dirty: false,
            settings,
            current_page: Default::default(),
            show_about_modal: false,
            show_about: false,
            pending_confirm: None,
            is_scanning: false,
            app_uptime_secs: 0,
            dashboard_mode: 0,
            apple_device_info: HashMap::new(),
            apple_activation_info: HashMap::new(),
            apple_network_locked: HashMap::new(),
            apple_unlock_instructions: HashMap::new(),
            apple_tab: crate::ui::apple_panel::AppleTab::default(),
            apple_ipsw_path: String::new(),
            apple_erase_mode: false,
            apple_verify_tss: true,
            apple_skip_baseband: false,
            apple_download_dir: dirs::download_dir().unwrap_or_default().to_string_lossy().to_string(),
            apple_bypass_method: AppleBypassMethodUI::default(),
            apple_dns_server: "78.100.17.60".into(),
            apple_au_carrier: "Telstra".into(),
            apple_carrier_account: String::new(),
            apple_erase_restore: false,
            apple_verify_sha1: true,
            apple_passcode_method: "checkm8 bypass exploit (A7-A11)".into(),
            apple_bypass_technique: "checkm8 (A7-A11)".into(),
            apple_shsh_path: String::new(),
            downloads_tab: Default::default(),
            ipsw_model_selected: "Select a device model".into(),
            ipsw_search_query: String::new(),
            ipsw_search_results: Vec::new(),
            ipsw_searching: false,
            active_downloads: Vec::new(),
            samsung_fw_model: String::new(),
            samsung_fw_csc: String::new(),
            firmware_search_brand: "Samsung".into(),
            firmware_search_model: String::new(),
            firmware_search_region: "OXM".into(),
            firmware_search_results: Vec::new(),
            ssh_tab: Default::default(),
            ssh_tab2: 0,
            ssh_host: String::new(),
            ssh_port: "22".into(),
            ssh_username: "root".into(),
            ssh_auth_method: "Password".into(),
            ssh_password: String::new(),
            ssh_key_path: String::new(),
            ssh_passphrase: String::new(),
            ssh_connected: false,
            ssh_terminal_output: String::new(),
            ssh_command_input: String::new(),
            ssh_command_history: Vec::new(),
            ssh_history_idx: 0,
            ssh_tunnels: Vec::new(),
            ssh_new_local_port: String::new(),
            ssh_new_remote_host: String::new(),
            ssh_new_remote_port: String::new(),
            ssh_output_rx: None,
            ssh_input_tx: None,
            au_unlock_imei: String::new(),
            au_unlock_brand: String::new(),
            au_unlock_carrier: String::new(),
            au_unlock_mccmnc: String::new(),
            au_unlock_nck_result: None,
            au_unlock_instructions: None,
            au_show_carrier_table: false,
            au_wizard_imei: String::new(),
            au_wizard_carrier: "Telstra (MCC 505/01)".into(),
            au_wizard_result: None,
            au_validate_imei: String::new(),
            au_validate_result: String::new(),
            au_tab: 0,
            mtk_tab: crate::ui::mediatek_panel::MtkTab::default(),
            mtk_connected: false,
            mtk_chipset: None,
            mtk_da_path: String::new(),
            mtk_scatter_path: String::new(),
            mtk_partition_name: String::new(),
            mtk_output_path: String::new(),
            mtk_file_path: String::new(),
            mtk_imei_input: String::new(),
            mtk_chipset_search: String::new(),
            hash_input: String::new(),
            hash_result: String::new(),
            hash_algo: "SHA-256".into(),
            hash_hmac_key: String::new(),
            mac_input: String::new(),
            mac_validate_result: String::new(),
            mac_derive_input: String::new(),
            mac_derive_result: String::new(),
            encode_tab: 0,
            encode_input: String::new(),
            encode_output: String::new(),
            jwt_input: String::new(),
            jwt_header: String::new(),
            jwt_payload: String::new(),
            numconv_input: String::new(),
            adb_shell_cmd: String::new(),
            adb_shell_output: String::new(),
            adb_shell_history: Vec::new(),
            tcp_test_host: "10.0.0.1".into(),
            tcp_test_port: "9001".into(),
            tcp_test_result: String::new(),
            dns_lookup_host: String::new(),
            dns_lookup_result: String::new(),
            tools_probe_host: String::new(),
            api_tab: 0,
            api_base_url: "https://api.chimeratool.com".into(),
            api_token: String::new(),
            api_response: String::new(),
            api_selected_endpoint: 0,
            api_request_body: String::new(),
            api_last_status: String::new(),
            api_mock_mode: false,
            api_imei_query: String::new(),
            api_firmware_model: String::new(),
            api_firmware_region: "OXM".into(),
            api_latency_ms: 0,
            api_requesting: false,
            api_probe_host: String::new(),
            api_probe_port: String::new(),
            shsh_tab: 0,
            shsh_active_tab: ShshTab::default(),
            shsh_ecid_input: String::new(),
            shsh_model_input: String::new(),
            shsh_build_input: String::new(),
            shsh_blob_path: String::new(),
            shsh_nonce_gen: String::new(),
            shsh_use_latest_sep: true,
            shsh_use_latest_baseband: false,
            shsh_report: String::new(),
            shsh_futurerestore_cmd: String::new(),
            shsh_saved_blobs,
            shsh_ecid: String::new(),
            shsh_board: String::new(),
            shsh_build: String::new(),
            shsh_ecid2: String::new(),
            shsh_model: String::new(),
            downgrade_ios: String::new(),
            futurerestore_ipsw: String::new(),
            futurerestore_shsh: String::new(),
            futurerestore_latest_sep: true,
            futurerestore_latest_bb: true,
            futurerestore_erase: false,
            futurerestore_running: false,
            futurerestore_log: String::new(),
            apnonce_generator: "None".into(),
            jailbreak_tab: 0,
            jb_strategy: "Patch boot.img + stage TWRP App".into(),
            jb_flash_method: "Fastboot boot recovery.img".into(),
            jb_recovery_img_path: String::new(),
            jb_opt_verify_checksum: true,
            jb_opt_stage_twrp: true,
            jb_opt_patch_boot: true,
            jb_twrp_apk_path: "./TWRP/me.twrp.twrpapp-26.apk".into(),
            jb_magisk_path: String::new(),
            jb_frp_manufacturer: "Auto-detect from device".into(),
            activation_tab: 0,
            activation_imei: String::new(),
            activation_bypass_method: "checkm8 exploit (A7-A11)".into(),
            activation_ecid: String::new(),
            devices_tab: 0,
            downloads_tab2: 0,
            utilities_tab: 0,
            settings_tab: 0,
            apple_ios_tab: 0,
            network_tab: 0,
            tools_tab: 0,
            settings_use_system_proxy: false,
            settings_proxy_url: String::new(),
            settings_verify_tls: true,
            settings_api_mock: false,
            settings_adb_path: "/usr/local/bin/adb".into(),
            settings_fastboot_path: "/usr/local/bin/fastboot".into(),
            settings_futurerestore_path: String::new(),
            settings_irecovery_path: String::new(),
            settings_dark_mode: true,
            settings_amber_accent: true,
            settings_compact_sidebar: false,
            settings_show_console: true,
            settings_dot_grid: true,
            settings_grain: true,
            settings_shimmer: true,
            settings_auto_detect: true,
            settings_confirm_destructive: true,
            settings_audible_alert: false,
            settings_prevent_sleep: true,
            imei_check_input: String::new(),
            imei_check_result: None,
            network_code_input: String::new(),
            network_code_result: None,
            utility_imei_input: String::new(),
            utility_nck_result: String::new(),
            utility_imei_result: String::new(),
            adb_tcp_host: String::new(),
            adb_tcp_port: "5555".into(),
            nck_imei_input: String::new(),
            nck_algo: "Samsung (DCK)".into(),
            imei_decode_input: String::new(),
            qr_input: String::new(),
            qr_svg: None,
            network_sim_imei: String::new(),
            network_mcc_imei: String::new(),
            evlog_filter: "All levels".into(),

            // Sidebar / About / status fields used by app.rs::render_sidebar
            hostname:    hostname::get()
                            .ok()
                            .and_then(|h| h.into_string().ok())
                            .unwrap_or_else(|| "localhost".into()),
            worker_ok:   true,
            adb_ok:      false,  // overwritten by the probe call below.
            adb_path:    None,
            adb_version: None,
            adb_error:   None,
            host_tools:  chimera_utils::HostToolProbes::default(),
            adb_last_probe: None,
            usb_ok:      true,
            build_ts:    env!("CARGO_PKG_VERSION").to_string(),
            build_hash:  "local".to_string(),
            started_at:  std::time::Instant::now(),
        };

        // Run the host-tool probes synchronously on startup.
        state.probe_host_tools();
        state
    }

    /// Run the full set of host-tool probes (adb, fastboot, irecovery, …).
    /// Called once at startup from `new()` and again on demand via menu /
    /// dashboard refresh button.
    pub fn probe_host_tools(&mut self) {
        let probes = chimera_utils::HostToolProbes::probe_all();
        self.adb_ok      = probes.adb.found;
        self.adb_path    = probes.adb.path.as_ref().map(|p| p.display().to_string());
        self.adb_version = probes.adb.version.clone();
        self.adb_error   = probes.adb.error.clone();
        self.host_tools  = probes;
        self.adb_last_probe = Some(std::time::Instant::now());
    }

    /// Cheaper refresh — re-probes only `adb` (the most frequently changing
    /// state). Throttled to once every 5 seconds; safe to call from the egui
    /// update loop on every frame.
    pub fn refresh_adb_throttled(&mut self) {
        let due = self.adb_last_probe
            .map(|t| t.elapsed() >= std::time::Duration::from_secs(5))
            .unwrap_or(true);
        if !due { return; }
        let p = chimera_utils::detect_adb();
        self.adb_ok      = p.found;
        self.adb_path    = p.path.as_ref().map(|p| p.display().to_string());
        self.adb_version = p.version.clone();
        self.adb_error   = p.error.clone();
        self.host_tools.adb = p;
        self.adb_last_probe = Some(std::time::Instant::now());
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    pub fn add_log(&mut self, entry: LogEntry) {
        if self.settings.log_to_file {
            let line = format!("[{}] [{:?}] {}", entry.timestamp, entry.level, entry.message);
            let _ = crate::persistence::append_log_line(&line);
        }
        self.log_entries.push(entry);
        // Trim
        if self.log_entries.len() > self.settings.max_log_lines {
            let drain = self.log_entries.len() - self.settings.max_log_lines;
            self.log_entries.drain(..drain);
        }
    }

    /// Log a single error message — preserved from original state.rs API.
    pub fn log_error(&mut self, msg: impl Into<String>) {
        self.add_log(LogEntry::error(msg));
    }


    pub fn send_operation(&self, req: OperationRequest) {
        if let Some(tx) = &self.op_tx { let _ = tx.send(req); }
    }

    /// Queue a dangerous operation through the confirmation dialog
    pub fn confirm_op(&mut self, title: &str, msg: &str, op: OperationRequest) {
        if self.settings.confirm_dangerous_ops {
            self.pending_confirm = Some(ConfirmDialog {
                title: title.into(), message: msg.into(), on_confirm: op,
            });
        } else {
            self.send_operation(op);
        }
    }

    pub fn mark_settings_dirty(&mut self) { self.settings_dirty = true; }

    /// Record a completed operation to history and persist it
    pub fn record_history(&mut self, entry: crate::history::HistoryEntry) {
        let _ = crate::persistence::append_history_entry(&entry);
        self.history.push(entry);
    }

    /// Refresh SHSH blob list from disk
    pub fn refresh_shsh_list(&mut self) {
        self.shsh_saved_blobs = crate::persistence::list_shsh_blobs();
    }

    /// Drain pending SSH output into terminal buffer
    fn drain_ssh_output(&mut self) {
        if let Some(rx) = &self.ssh_output_rx {
            while let Ok(line) = rx.try_recv() {
                self.ssh_terminal_output.push_str(&line);
                self.ssh_terminal_output.push('\n');
            }
        }
    }

    // ── Event processing ───────────────────────────────────────────────────

    pub fn process_events(&mut self) {
        // 1. Core ChimeraEvents
        while let Ok(ev) = self.event_rx.try_recv() {
            match ev {
                ChimeraEvent::DeviceConnected(info) => {
                    let id = info.id.clone();
                    self.session.record_device_seen(&info);
                    self.add_log(LogEntry::success(format!(
                        "Device connected: {:?} {} [{}]",
                        info.brand, info.model,
                        info.serial.as_deref().unwrap_or("?")
                    )));
                    self.devices.entry(id.clone()).or_insert_with(|| DeviceUiState::new(info));
                    if self.selected_device_id.is_none() {
                        self.selected_device_id = Some(id);
                    }
                    self.is_scanning = false;
                }
                ChimeraEvent::DeviceDisconnected(id) => {
                    self.add_log(LogEntry::warn(format!("Device disconnected: {}", id)));
                    self.devices.remove(&id);
                    self.selected_device_ids.remove(&id);
                    if self.selected_device_id.as_deref() == Some(&id) {
                        self.selected_device_id = self.devices.keys().next().cloned();
                    }
                }
                ChimeraEvent::DeviceInfoUpdated(info) => {
                    let id = info.id.clone();
                    if let Some(ds) = self.devices.get_mut(&id) {
                        ds.imei_input  = info.imei.clone().unwrap_or_default();
                        ds.imei2_input = info.imei2.clone().unwrap_or_default();
                        ds.mac_input   = info.mac_address.clone().unwrap_or_default();
                        ds.csc_input   = info.csc.clone().unwrap_or_default();
                        ds.device = info;
                    }
                }
                ChimeraEvent::OperationProgress(dev_id, prog) => {
                    if let Some(ds) = self.devices.get_mut(&dev_id) {
                        if prog.is_complete {
                            ds.operation_status = OperationStatus::Idle;
                        } else {
                            ds.operation_status = OperationStatus::Running {
                                name: prog.operation.clone(),
                                percent: prog.percent,
                                step: prog.step.clone(),
                            };
                        }
                    }
                }
                ChimeraEvent::OperationSuccess(dev_id, msg) => {
                    if let Some(ds) = self.devices.get_mut(&dev_id) {
                        ds.operation_status = OperationStatus::Success(msg.clone());
                        // Record to history
                        let entry = crate::history::HistoryEntry::new(
                            "Operation", &ds.device.model,
                            &format!("{:?}", ds.device.brand),
                            ds.device.serial.as_deref().unwrap_or(""),
                            crate::history::HistoryResult::Success,
                            &msg,
                        );
                        self.record_history(entry);
                    }
                    self.add_log(LogEntry::success(msg));
                    // Refresh SHSH list in case a blob was just saved
                    self.refresh_shsh_list();
                }
                ChimeraEvent::OperationFailed(dev_id, err) => {
                    if let Some(ds) = self.devices.get_mut(&dev_id) {
                        ds.operation_status = OperationStatus::Failed(err.clone());
                        let entry = crate::history::HistoryEntry::new(
                            "Operation", &ds.device.model,
                            &format!("{:?}", ds.device.brand),
                            ds.device.serial.as_deref().unwrap_or(""),
                            crate::history::HistoryResult::Failed,
                            &err,
                        );
                        self.record_history(entry);
                    }
                    self.add_log(LogEntry::error(err));
                }
                ChimeraEvent::Log(level, msg) => {
                    self.add_log(match level {
                        LogLevel::Error   => LogEntry::error(msg),
                        LogLevel::Warning => LogEntry::warn(msg),
                        LogLevel::Success => LogEntry::success(msg),
                        _                 => LogEntry::info(msg),
                    });
                }
                ChimeraEvent::DiagnosticsReady(dev_id, diag) => {
                    if let Some(ds) = self.devices.get_mut(&dev_id) {
                        ds.diagnostics = Some(*diag);
                    }
                    self.add_log(LogEntry::success("Diagnostics collected."));
                }
                ChimeraEvent::DeviceInfoPayload { device_id, payload_json } => {
                    match serde_json::from_str::<chimera_apple::device::AppleDeviceInfo>(&payload_json) {
                        Ok(info) => {
                            self.apple_device_info.insert(device_id, info);
                            self.add_log(LogEntry::success("Apple device info updated."));
                        }
                        Err(e) => self.add_log(LogEntry::error(format!("Apple info parse: {}", e))),
                    }
                }
                ChimeraEvent::ActivationStatus { device_id, status, account_hint, is_supervised, mdm_org } => {
                    let mapped = match status.as_str() {
                        "Activated"           => AppleActivationStatus::Activated,
                        "ActivationRequired"  => AppleActivationStatus::ActivationRequired,
                        "Unactivated"         => AppleActivationStatus::Unactivated,
                        _                     => AppleActivationStatus::Unknown,
                    };
                    self.apple_activation_info.insert(device_id, AppleActivationUiInfo {
                        status: mapped, account_hint,
                        is_supervised: is_supervised.unwrap_or(false), mdm_org,
                    });
                }
                ChimeraEvent::NetworkLockStatus { device_id, locked } => {
                    self.apple_network_locked.insert(device_id, locked);
                }
                ChimeraEvent::UnlockGuideResult { device_id, instructions } => {
                    self.apple_unlock_instructions.insert(device_id, instructions);
                }
                ChimeraEvent::NckResult { imei, nck } => {
                    if self.au_unlock_imei.is_empty() || self.au_unlock_imei == imei {
                        self.au_unlock_imei = imei;
                        self.au_unlock_nck_result = Some(nck.clone());
                        self.add_log(LogEntry::success(format!("NCK: {}", nck)));
                    }
                }
                ChimeraEvent::AuGuideResult { imei, instructions } => {
                    if self.au_unlock_imei.is_empty() || self.au_unlock_imei == imei {
                        self.au_unlock_imei = imei;
                        self.au_unlock_instructions = Some(instructions);
                    }
                }
                _ => {} // future core events
            }
        }

        // 2. GUI-local events
        while let Ok(ev) = self.local_event_rx.try_recv() {
            match ev {
                LocalEvent::SshConnected { output_rx, input_tx } => {
                    self.ssh_connected = true;
                    self.ssh_output_rx = Some(output_rx);
                    self.ssh_input_tx  = Some(input_tx);
                    self.ssh_terminal_output.clear();
                    self.add_log(LogEntry::success(format!(
                        "SSH connected to {}:{}", self.ssh_host, self.ssh_port)));
                }
                LocalEvent::SshDisconnected => {
                    self.ssh_connected = false;
                    self.ssh_output_rx = None;
                    self.ssh_input_tx  = None;
                    self.add_log(LogEntry::warn("SSH disconnected."));
                }
                LocalEvent::SshLine(line) => {
                    self.ssh_terminal_output.push_str(&line);
                    self.ssh_terminal_output.push('\n');
                }
                LocalEvent::QrReady { serial, svg } => {
                    self.qr_svg = Some(svg);
                    self.add_log(LogEntry::success(
                        format!("QR code ready for pairing — serial: {}", serial)));
                }
                LocalEvent::TaInfoResult { device_id, entries } => {
                    if let Some(ds) = self.devices.get_mut(&device_id) {
                        ds.ta_info = entries;
                    }
                    self.add_log(LogEntry::success("Sony TA info retrieved."));
                }
                LocalEvent::AuDeviceRead { device_id, imei, carrier } => {
                    self.au_unlock_imei    = imei.clone();
                    self.au_unlock_carrier = carrier.clone();
                    self.add_log(LogEntry::success(
                        format!("AU device read: IMEI={} carrier={}", imei, carrier)));
                }
                LocalEvent::IpswSearchResults { entries } => {
                    self.ipsw_search_results = entries;
                    self.ipsw_searching = false;
                    self.add_log(LogEntry::success(format!(
                        "IPSW search: {} results", self.ipsw_search_results.len())));
                }
                LocalEvent::DownloadProgress { id, bytes_done, bytes_total } => {
                    if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == id) {
                        task.bytes_done  = bytes_done;
                        task.bytes_total = bytes_total;
                        task.status      = DownloadStatus::Running;
                    }
                }
                LocalEvent::DownloadComplete { id, dest_path } => {
                    if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == id) {
                        task.status = DownloadStatus::Done;
                    }
                    self.add_log(LogEntry::success(format!("Download complete: {}", dest_path)));
                    self.refresh_shsh_list();
                }
                LocalEvent::DownloadFailed { id, error } => {
                    if let Some(task) = self.active_downloads.iter_mut().find(|t| t.id == id) {
                        task.status = DownloadStatus::Failed(error.clone());
                    }
                    self.add_log(LogEntry::error(format!("Download failed: {}", error)));
                }
                LocalEvent::ApiResponse { status, body, latency_ms } => {
                    self.api_last_status = status;
                    self.api_response    = body;
                    self.api_latency_ms  = latency_ms;
                    self.api_requesting  = false;
                }
                LocalEvent::TcpTestResult { host, port, open, latency_ms } => {
                    self.tcp_test_result = if open {
                        format!("{}:{} OPEN — {}ms", host, port, latency_ms)
                    } else {
                        format!("{}:{} CLOSED/TIMEOUT", host, port)
                    };
                    self.add_log(if open {
                        LogEntry::success(self.tcp_test_result.clone())
                    } else {
                        LogEntry::error(self.tcp_test_result.clone())
                    });
                }
                LocalEvent::DnsResult { host, ips } => {
                    self.dns_lookup_result = ips.join(", ");
                    self.add_log(LogEntry::success(
                        format!("DNS {} → {}", host, self.dns_lookup_result)));
                }
                LocalEvent::FuturerestoreLine(line) => {
                    self.futurerestore_log.push_str(&line);
                    self.futurerestore_log.push('\n');
                    self.add_log(LogEntry::info(line));
                }
                LocalEvent::FuturerestoreDone { success, message } => {
                    self.futurerestore_running = false;
                    if success {
                        self.add_log(LogEntry::success(message));
                    } else {
                        self.add_log(LogEntry::error(message));
                    }
                }
            }
        }

        // 3. Drain live SSH output every frame
        self.drain_ssh_output();
    }
}

impl Default for AppState {
    fn default() -> Self { Self::new() }
}
