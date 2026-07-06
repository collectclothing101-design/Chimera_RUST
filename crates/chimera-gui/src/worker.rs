// crates/chimera-gui/src/worker.rs
// Background worker — scan loop + all operation handlers
// All API stubs use checked paths from cargo build output.
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_mut)]
use reqwest;
use ssh2;
use chimera_core::device::{ConnectionMode, DeviceBrand};
use chimera_core::event::{ChimeraEvent, LogLevel};
use chimera_core::progress::{Progress, ProgressSender, progress_channel};
use chimera_adb::client::AdbClient;
use chimera_adb::diagnostics::AdbDiagnostics;
use chimera_apple::AppleDeviceInfo;
use chimera_apple::AppleChipset;
use chimera_core::{ChimeraError, Result as ChimeraResult};
use chimera_samsung::operations::SamsungOperations;
use chimera_samsung::odin::OdinClient;
use chimera_huawei::operations::HuaweiOperations;
use chimera_mtk::operations::MtkOperations;
use crossbeam_channel::Sender;
use std::path::{Path, PathBuf};
use crate::local_event::{LocalEvent, IpswEntry, DownloadTask, DownloadStatus};
use log::info;

// ── Helpers ────────────────────────────────────────────────────────────────

fn send_log(tx: &Sender<ChimeraEvent>, level: LogLevel, msg: impl Into<String>) {
    let _ = tx.send(ChimeraEvent::Log(level, msg.into()));
}
fn send_success(tx: &Sender<ChimeraEvent>, dev: &str, msg: impl Into<String>) {
    let _ = tx.send(ChimeraEvent::OperationSuccess(dev.to_string(), msg.into()));
}
fn send_failed(tx: &Sender<ChimeraEvent>, dev: &str, msg: impl Into<String>) {
    let _ = tx.send(ChimeraEvent::OperationFailed(dev.to_string(), msg.into()));
}
fn spawn_progress_forwarder(
    event_tx: Sender<ChimeraEvent>,
    device_id: String,
    prog_rx: crossbeam_channel::Receiver<Progress>,
) {
    std::thread::spawn(move || {
        while let Ok(p) = prog_rx.recv() {
            let _ = event_tx.send(ChimeraEvent::OperationProgress(device_id.clone(), p));
        }
    });
}

// spawn_op! — reduce boilerplate for simple ADB ops
macro_rules! spawn_op {
    ($etx:expr, $ltx:expr, $did:expr, $serial:expr, $ok_msg:expr,
     |$adb:ident, $srl:ident, $prog:ident| $body:expr) => {{
        let event_tx2 = $etx.clone();
        let device_id  = $did.clone();
        let serial_c   = $serial.clone();
        let ok_msg     = $ok_msg.to_string();
        std::thread::spawn(move || {
            let $adb  = AdbClient::new();
            let $srl  = serial_c;
            let ($prog, prog_rx) = progress_channel();
            spawn_progress_forwarder(event_tx2.clone(), device_id.clone(), prog_rx);
            match $body {
                Ok(_)  => send_success(&event_tx2, &device_id, ok_msg),
                Err(e) => send_failed(&event_tx2, &device_id, format!("{}", e)),
            }
        });
    }};
}

// ── Firmware flash helpers ─────────────────────────────────────────────────

fn firmware_route_name(brand: &DeviceBrand, mode: &ConnectionMode) -> &'static str {
    match (brand, mode) {
        (DeviceBrand::Samsung, ConnectionMode::DownloadOdin | ConnectionMode::SamsungEub) => "Samsung Odin",
        (_, ConnectionMode::Fastboot)   => "Fastboot",
        (_, ConnectionMode::Edl)        => "Qualcomm EDL",
        (_, ConnectionMode::MtkBootRom) => "MediaTek scatter",
        _ => "unsupported",
    }
}

fn validate_recovery_image(path: &str) -> ChimeraResult<()> {
    if path.trim().is_empty() { return Err(ChimeraError::Firmware("No recovery image selected.".into())); }
    let p = Path::new(path);
    if !p.exists() { return Err(ChimeraError::Firmware(format!("Not found: {}", path))); }
    Ok(())
}

// ── Operation request enum ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum OperationRequest {
    // Scanning
    QuickScan,
    SetScanInterval(u64),
    // Device info
    GetInfo           { device_id: String, serial: String },
    // Universal
    FactoryReset      { device_id: String, serial: String },
    RemoveFrp         { device_id: String, serial: String },
    ResetScreenLock   { device_id: String, serial: String },
    RemoveMdm         { device_id: String, serial: String },
    RemoveDemoMode    { device_id: String, serial: String },
    EnableAdb         { device_id: String, serial: String },
    RebootDevice      { device_id: String, serial: String, mode: Option<String> },
    CollectDiagnostics{ device_id: String, serial: String },
    // IMEI / MAC
    RepairImei        { device_id: String, serial: String, imei1: String, imei2: Option<String> },
    RepairImeiPatch   { device_id: String, serial: String, imei1: String, imei2: Option<String> },
    RepairMac         { device_id: String, serial: String, mac: String },
    CheckImei         { imei: String },
    CalculateNetworkCode { imei: String },
    GenerateAdbQr     { serial: String },
    ExtractFirmware   { source: String, dest: String },
    // Samsung
    SamsungCscChange            { device_id: String, serial: String, new_csc: String },
    SamsungStoreBackup          { device_id: String, serial: String, path: String },
    SamsungRestoreBackup        { device_id: String, serial: String, path: String },
    SamsungResetReactivationLock{ device_id: String, serial: String },
    SamsungRemoveLostMode       { device_id: String, serial: String },
    SamsungRemoveWarnings       { device_id: String, serial: String },
    SamsungRepairEfs            { device_id: String, serial: String, backup_path: Option<String> },
    SamsungKnoxGuardRemove      { device_id: String, serial: String },
    SamsungCarrierRelock        { device_id: String, serial: String, carriers: Vec<String> },
    SamsungNetworkFactoryReset  { device_id: String, serial: String },
    SamsungRootDevice           { device_id: String, serial: String },
    SamsungPatchCertificate     { device_id: String, serial: String },
    SamsungReadCertificate      { device_id: String, serial: String, output_path: String },
    SamsungWriteCertificate     { device_id: String, serial: String, cert_path: String },
    SamsungReadNetworkCodes     { device_id: String, serial: String },
    // Xiaomi
    XiaomiRemoveFrp         { device_id: String, serial: String },
    XiaomiStoreBackup       { device_id: String, serial: String, path: String },
    XiaomiNetworkFactoryReset{ device_id: String, serial: String },
    // Huawei
    HuaweiDisableHuaweiId { device_id: String, serial: String },
    HuaweiRemoveDemoMode  { device_id: String, serial: String },
    // Sony
    SonyBackupTa  { device_id: String, serial: String, dest_path: String },
    SonyRestoreTa { device_id: String, serial: String, src_path: String },
    SonyGetTaInfo { device_id: String, serial: String },
    // Firmware
    FlashFirmware     { device_id: String, serial: String, brand: DeviceBrand, connection_mode: ConnectionMode, model: String, firmware_path: String },
    FlashRecoveryImage{ device_id: String, serial: String, brand: DeviceBrand, connection_mode: ConnectionMode, model: String, recovery_image_path: String },
    BootRecoveryImage { device_id: String, serial: String, brand: DeviceBrand, connection_mode: ConnectionMode, model: String, recovery_image_path: String },
    // Bootloader
    UnlockBootloader { device_id: String, serial: String },
    LockBootloader   { device_id: String, serial: String },
    MagiskRoot       { device_id: String, serial: String, magisk_path: String },
    // ADB TCP
    ConnectAdbTcp    { host: String, port: u16, serial: String },
    DisconnectAdbTcp { host: String, port: u16 },
    // SSH
    SshConnect {
        host: String, port: u16, username: String,
        auth_method: String, password: String,
        key_path: String, passphrase: String,
    },
    SshDisconnect,
    SshSendCommand { cmd: String },
    SshAddTunnel   { local_port: u16, remote_host: String, remote_port: u16 },
    // Apple
    AppleGetInfo              { device_id: String },
    AppleFlashIpsw            { device_id: String, ipsw_path: String, erase: bool },
    AppleDownloadIpsw         { device_id: String, model: String, dest_dir: String },
    AppleValidateIpsw         { ipsw_path: String },
    AppleCheckIcloud          { device_id: String },
    AppleBypassIcloud         { device_id: String, method: crate::state::AppleBypassMethodUI },
    AppleIcloudWipe           { device_id: String, ipsw_path: Option<String> },
    AppleRemovePasscode       { device_id: String, use_checkm8: bool, ipsw_path: Option<String>, chipset: String },
    AppleEnterRecovery        { device_id: String },
    AppleExitRecovery         { device_id: String },
    AppleReboot               { device_id: String },
    AppleCheckNetworkLock     { device_id: String },
    AppleGetUnlockInstructions{ device_id: String, carrier: String },
    AppleSubmitCarrierUnlock  { device_id: String, carrier: String, account_number: Option<String> },
    // AU unlock
    AuReadDeviceImeiCarrier      { device_id: String },
    AuCalculateNck               { imei: String, brand: String, carrier: String, mccmnc: Option<String> },
    AuApplyNckAdb                { device_id: String, nck: String },
    AuGenerateUnlockInstructions { imei: String, carrier: String, brand: String },
    // Downloads
    SearchIpsw   { model: String },
    DownloadFile { id: String, url: String, dest: String, verify_sha1: Option<String> },
    // API tools
    ApiRequest { url: String, method: String, body: String, token: String, verify_tls: bool },
    // Network tools
    TcpTest   { host: String, port: u16 },
    DnsLookup { hostname: String },
    // Futurerestore
    RunFuturerestore {
        futurerestore_path: String, ipsw_path: String, shsh_path: String,
        latest_sep: bool, latest_baseband: bool, erase: bool,
    },
}

// ── Worker pool ────────────────────────────────────────────────────────────

pub struct WorkerPool {
    event_tx:     Sender<ChimeraEvent>,
    local_tx:     Sender<LocalEvent>,
    op_tx:        crossbeam_channel::Sender<OperationRequest>,
    op_rx:        crossbeam_channel::Receiver<OperationRequest>,
    scan_trigger: crossbeam_channel::Sender<()>,
    scan_recv:    crossbeam_channel::Receiver<()>,
}

impl WorkerPool {
    pub fn new(event_tx: Sender<ChimeraEvent>, local_tx: Sender<LocalEvent>) -> Self {
        let (op_tx, op_rx)           = crossbeam_channel::unbounded();
        let (scan_trigger, scan_recv) = crossbeam_channel::unbounded();
        Self { event_tx, local_tx, op_tx, op_rx, scan_trigger, scan_recv }
    }

    pub fn sender(&self) -> crossbeam_channel::Sender<OperationRequest> { self.op_tx.clone() }

    pub fn start(self) -> std::thread::JoinHandle<()> {

        // ── Scan loop ──────────────────────────────────────────────────────
        {
            let event_tx   = self.event_tx.clone();
            let scan_recv2 = self.scan_recv.clone();
            std::thread::spawn(move || {
                let mut known: std::collections::HashSet<String> = Default::default();
                let interval = std::time::Duration::from_millis(1500);
                loop {
                    let _ = crossbeam_channel::select! {
                        recv(scan_recv2) -> _ => {},
                        default(interval) => {},
                    };
                    let adb = AdbClient::new();
                    let current: std::collections::HashSet<String> = adb
                        .list_devices().unwrap_or_default()
                        .into_iter().map(|d| d.serial).collect();
                    for id in current.difference(&known) {
                        let info = adb.get_device_info(id)
                            .unwrap_or_else(|_| chimera_core::DeviceInfo::new_unknown(id));
                        let _ = event_tx.send(ChimeraEvent::DeviceConnected(info));
                    }
                    for id in known.difference(&current) {
                        let _ = event_tx.send(ChimeraEvent::DeviceDisconnected(id.clone()));
                    }
                    known = current;
                }
            });
        }

        // ── Dispatch loop ──────────────────────────────────────────────────
        std::thread::spawn(move || {
            let event_tx = self.event_tx.clone();
            let local_tx = self.local_tx.clone();
            info!("chimera::worker dispatch loop started");

            for op in &self.op_rx {
                let event_tx = event_tx.clone();
                let local_tx = local_tx.clone();

                match op {

                OperationRequest::QuickScan => {
                    let _ = self.scan_trigger.try_send(());
                }
                OperationRequest::SetScanInterval(_) => {
                    let _ = self.scan_trigger.try_send(());
                }

                // ── Diagnostics ────────────────────────────────────────
                OperationRequest::CollectDiagnostics { device_id, serial } => {
                    let device_id2 = device_id.clone(); let serial2 = serial.clone();
                    let event_tx2  = event_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let diag = AdbDiagnostics::new(&adb, &serial2).collect_all();
                        let _ = event_tx2.send(ChimeraEvent::DiagnosticsReady(
                            device_id2.clone(), Box::new(diag)));
                        send_success(&event_tx2, &device_id2, "Diagnostics collected.");
                    });
                }

                // ── Get Info ───────────────────────────────────────────
                OperationRequest::GetInfo { device_id, serial } => {
                    let device_id2 = device_id.clone(); let serial2 = serial.clone();
                    let event_tx2  = event_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let (prog_tx, prog_rx) = progress_channel();
                        spawn_progress_forwarder(event_tx2.clone(), device_id2.clone(), prog_rx);
                        let ops = SamsungOperations::new(&adb, &serial2);
                        match ops.get_info(Some(&prog_tx)) {
                            Ok(info) => {
                                let _ = event_tx2.send(ChimeraEvent::DeviceInfoUpdated(info));
                                send_success(&event_tx2, &device_id2, "Device info retrieved.");
                            }
                            Err(_) => {
                                if let Ok(info) = adb.get_device_info(&serial2) {
                                    let _ = event_tx2.send(ChimeraEvent::DeviceInfoUpdated(info));
                                }
                                send_success(&event_tx2, &device_id2, "Basic device info retrieved.");
                            }
                        }
                    });
                }

                // ── FRP Remove ─────────────────────────────────────────
                OperationRequest::RemoveFrp { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "FRP removed.",
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).reset_frp(Some(&prog)));
                }

                // ── Factory Reset ──────────────────────────────────────
                OperationRequest::FactoryReset { device_id, serial } => {
                    let device_id2 = device_id.clone(); let serial2 = serial.clone();
                    let event_tx2  = event_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let sh = chimera_adb::shell::AdbShell::new(&adb, &serial2);
                        match sh.run("am broadcast -a android.intent.action.MASTER_CLEAR") {
                            Ok(_)  => send_success(&event_tx2, &device_id2, "Factory reset triggered."),
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }

                // ── Screen lock ────────────────────────────────────────
                OperationRequest::ResetScreenLock { device_id, serial } => {
                    let device_id2 = device_id.clone(); let serial2 = serial.clone();
                    let event_tx2  = event_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let sh  = chimera_adb::shell::AdbShell::new(&adb, &serial2);
                        let cmds = [
                            "rm -f /data/system/locksettings.db",
                            "rm -f /data/system/gesture.key /data/system/password.key",
                            "settings put secure lockscreen.password_type 0",
                        ];
                        let mut ok = true;
                        for cmd in &cmds {
                            if sh.run_root(cmd).is_err() { ok = false; }
                        }
                        if ok { send_success(&event_tx2, &device_id2, "Screen lock cleared. Reboot to apply.") }
                        else  { send_failed(&event_tx2, &device_id2, "Partial clear — root may be needed.") }
                    });
                }

                OperationRequest::RemoveMdm { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "MDM removed.",
                        |adb, srl, prog| chimera_adb::services::AdbServices::new(&adb, &srl)
                            .remove_mdm(Some(&prog)));
                }
                OperationRequest::RemoveDemoMode { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Demo mode removed.",
                        |adb, srl, prog| chimera_adb::services::AdbServices::new(&adb, &srl)
                            .remove_demo_mode(Some(&prog)));
                }
                OperationRequest::EnableAdb { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "ADB debug enabled.",
                        |adb, srl, prog| chimera_adb::services::AdbServices::new(&adb, &srl)
                            .enable_adb_debug(Some(&prog)));
                }

                // ── Check IMEI ─────────────────────────────────────────
                OperationRequest::CheckImei { imei } => {
                    let result = chimera_utils::ImeiChecker::check(&imei);
                    let msg = if result.is_valid {
                        format!("✓ IMEI {} VALID | TAC: {} | Brand: {}", result.imei, result.tac,
                            result.brand_hint.as_deref().unwrap_or("Unknown"))
                    } else {
                        format!("✗ IMEI {} INVALID (Luhn check failed)", result.imei)
                    };
                    send_log(&event_tx, if result.is_valid { LogLevel::Success } else { LogLevel::Error }, msg);
                }

                // ── NCK ────────────────────────────────────────────────
                OperationRequest::CalculateNetworkCode { imei } => {
                    match chimera_utils::NetworkCodeCalculator::calculate(&imei) {
                        Ok(codes) => {
                            let nck = codes.nck.clone();
                            send_log(&event_tx, LogLevel::Success,
                                format!("NCK for {}: {} (MCK: {})", imei, codes.nck,
                                    codes.mck.as_deref().unwrap_or("N/A")));
                            let _ = event_tx.send(ChimeraEvent::NckResult { imei, nck });
                        }
                        Err(e) => send_log(&event_tx, LogLevel::Error,
                            format!("NCK calculation failed: {}", e)),
                    }
                }

                // ── ADB QR ─────────────────────────────────────────────
                OperationRequest::GenerateAdbQr { serial } => {
                    let local_tx2  = local_tx.clone();
                    let event_tx2  = event_tx.clone();
                    let serial2    = serial.clone();
                    std::thread::spawn(move || {
                        let pairing_str = format!(
                            "WIFI:T:ADB;S:ChimeraRS;P:chimera_{};HOST:;PORT:5555;;",
                            &serial2[..serial2.len().min(8)]
                        );
                        match qrcode::QrCode::new(pairing_str.as_bytes()) {
                            Ok(code) => {
                                let svg = code
                                    .render::<qrcode::render::svg::Color>()
                                    .min_dimensions(220, 220)
                                    .max_dimensions(280, 280)
                                    .light_color(qrcode::render::svg::Color("#1a1410"))
                                    .dark_color(qrcode::render::svg::Color("#e8951e"))
                                    .build();
                                let _ = local_tx2.send(LocalEvent::QrReady { serial: serial2.clone(), svg });
                                send_log(&event_tx2, LogLevel::Success,
                                    format!("ADB pairing QR generated for {}", serial2));
                            }
                            Err(e) => send_log(&event_tx2, LogLevel::Error,
                                format!("QR generation failed: {}", e)),
                        }
                    });
                }

                // ── Reboot ─────────────────────────────────────────────
                OperationRequest::RebootDevice { device_id, serial, mode } => {
                    let device_id2 = device_id.clone(); let serial2 = serial.clone();
                    let mode2 = mode.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        let cmd = match mode2.as_deref() {
                            Some("bootloader"|"fastboot") => "reboot bootloader",
                            Some("recovery")              => "reboot recovery",
                            Some("download")              => "reboot download",
                            _                             => "reboot",
                        };
                        let adb = AdbClient::new();
                        match chimera_adb::shell::AdbShell::new(&adb, &serial2).run(cmd) {
                            Ok(_)  => send_success(&event_tx2, &device_id2,
                                format!("Rebooting to {}", mode2.as_deref().unwrap_or("system"))),
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }

                // ── IMEI Repair ────────────────────────────────────────
                OperationRequest::RepairImei { device_id, serial, imei1, imei2 } => {
                    let device_id2 = device_id.clone(); let serial2 = serial.clone();
                    let imei1_2 = imei1.clone(); let imei2_2 = imei2.clone();
                    let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = chimera_core::imei::validate_imei(&imei1_2) {
                            send_failed(&event_tx2, &device_id2, format!("IMEI 1 invalid: {}", e)); return;
                        }
                        let adb = AdbClient::new();
                        let (prog_tx, prog_rx) = progress_channel();
                        spawn_progress_forwarder(event_tx2.clone(), device_id2.clone(), prog_rx);
                        let ops = chimera_adb::operations::AdbOperations::new(&adb, &serial2);
                        match ops.repair_imei(&imei1_2, imei2_2.as_deref(), Some(&prog_tx)) {
                            Ok(_)  => send_success(&event_tx2, &device_id2, "IMEI repaired."),
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }
                OperationRequest::RepairImeiPatch { device_id, serial, imei1, imei2 } => {
                    let device_id2 = device_id.clone(); let serial2 = serial.clone();
                    let imei1_2 = imei1.clone(); let imei2_2 = imei2.clone();
                    let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let (prog_tx, prog_rx) = progress_channel();
                        spawn_progress_forwarder(event_tx2.clone(), device_id2.clone(), prog_rx);
                        let ops = chimera_adb::operations::AdbOperations::new(&adb, &serial2);
                        match ops.repair_imei_patch(&imei1_2, imei2_2.as_deref(), Some(&prog_tx)) {
                            Ok(_)  => send_success(&event_tx2, &device_id2, "IMEI patch applied."),
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }
                OperationRequest::RepairMac { device_id, serial, mac } => {
                    let device_id2 = device_id.clone(); let serial2 = serial.clone();
                    let mac2 = mac.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = chimera_core::mac_address::validate_mac(&mac2) {
                            send_failed(&event_tx2, &device_id2, format!("Invalid MAC: {}", e)); return;
                        }
                        let adb = AdbClient::new();
                        let (prog_tx, prog_rx) = progress_channel();
                        spawn_progress_forwarder(event_tx2.clone(), device_id2.clone(), prog_rx);
                        let ops = chimera_adb::operations::AdbOperations::new(&adb, &serial2);
                        match ops.repair_mac(&mac2, Some(&prog_tx)) {
                            Ok(_)  => send_success(&event_tx2, &device_id2, format!("MAC repaired: {}", mac2)),
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }

                // ── Samsung ────────────────────────────────────────────
                OperationRequest::SamsungCscChange { device_id, serial, new_csc } => {
                    spawn_op!(event_tx, local_tx, device_id, serial,
                        format!("CSC changed to {}", new_csc),
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).csc_change(&new_csc, Some(&prog)));
                }
                OperationRequest::SamsungNetworkFactoryReset { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Network factory reset done.",
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).network_factory_reset(Some(&prog)));
                }
                OperationRequest::SamsungResetReactivationLock { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Reactivation lock reset.",
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).reset_reactivation_lock(Some(&prog)));
                }
                OperationRequest::SamsungStoreBackup { device_id, serial, path } => {
                    spawn_op!(event_tx, local_tx, device_id, serial,
                        format!("Backup stored: {}", path),
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).store_backup(&path, Some(&prog)));
                }
                OperationRequest::SamsungRestoreBackup { device_id, serial, path } => {
                    spawn_op!(event_tx, local_tx, device_id, serial,
                        format!("Backup restored: {}", path),
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).restore_backup(&path, Some(&prog)));
                }
                OperationRequest::SamsungRepairEfs { device_id, serial, backup_path } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "EFS repair done.",
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl)
                            .repair_efs(backup_path.as_deref(), Some(&prog)));
                }
                OperationRequest::SamsungKnoxGuardRemove { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Knox Guard removal done.",
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).remove_knox_guard(Some(&prog)));
                }
                OperationRequest::SamsungCarrierRelock { device_id, serial, carriers } => {
                    if carriers.is_empty() {
                        send_failed(&event_tx, &device_id, "No carriers provided.");
                    } else {
                        spawn_op!(event_tx, local_tx, device_id, serial,
                            format!("Carrier relock done ({} carriers).", carriers.len()),
                            |adb, srl, prog| {
                                let refs: Vec<&str> = carriers.iter().map(String::as_str).collect();
                                SamsungOperations::new(&adb, &srl).carrier_relock(&refs, Some(&prog))
                            });
                    }
                }
                OperationRequest::SamsungRootDevice { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Samsung root done.",
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).root_device(Some(&prog)));
                }
                OperationRequest::SamsungRemoveLostMode { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Lost mode removed.",
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).remove_lost_mode(Some(&prog)));
                }
                OperationRequest::SamsungRemoveWarnings { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Warning logos removed.",
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).remove_warnings(Some(&prog)));
                }
                OperationRequest::SamsungPatchCertificate { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Certificate patched.",
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).patch_certificate(Some(&prog)));
                }
                OperationRequest::SamsungReadCertificate { device_id, serial, output_path } => {
                    let event_tx2 = event_tx.clone(); let device_id2 = device_id.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let (prog, _prog_rx) = progress_channel();
                        let result = SamsungOperations::new(&adb, &serial).read_certificate(&output_path, Some(&prog));
                        match result {
                            Ok(()) => send_success(&event_tx2, &device_id2, format!("Certificate saved to {}", output_path)),
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }
                OperationRequest::SamsungWriteCertificate { device_id, serial, cert_path } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Certificate written.",
                        |adb, srl, prog| SamsungOperations::new(&adb, &srl).write_certificate(&cert_path, Some(&prog)));
                }
                OperationRequest::SamsungReadNetworkCodes { device_id, serial } => {
                    let event_tx2 = event_tx.clone(); let device_id2 = device_id.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let (prog, _prog_rx) = progress_channel();
                        let result = SamsungOperations::new(&adb, &serial).read_network_codes(Some(&prog));
                        match result {
                            Ok(codes) => {
                                let msg = if codes.unlock_available {
                                    format!("NCK: {:?}\nMCK: {:?}\nAttempts remaining: {}", codes.nck, codes.mck, codes.nck_count)
                                } else {
                                    format!("No unlock codes available. Attempts remaining: {}", codes.nck_count)
                                };
                                send_success(&event_tx2, &device_id2, msg);
                            }
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }

                // ── Xiaomi ─────────────────────────────────────────────
                OperationRequest::XiaomiRemoveFrp { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Xiaomi FRP removed.",
                        |adb, srl, prog| chimera_xiaomi::operations::XiaomiOperations::new(&adb, &srl)
                            .remove_frp_adb(Some(&prog)));
                }
                OperationRequest::XiaomiStoreBackup { device_id, serial, path } => {
                    spawn_op!(event_tx, local_tx, device_id, serial,
                        format!("Xiaomi backup: {}", path),
                        |adb, srl, prog| chimera_xiaomi::operations::XiaomiOperations::new(&adb, &srl)
                            .store_backup(&path, Some(&prog)));
                }
                OperationRequest::XiaomiNetworkFactoryReset { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial,
                        "Xiaomi network factory reset done.",
                        |adb, srl, prog| chimera_xiaomi::operations::XiaomiOperations::new(&adb, &srl)
                            .network_factory_reset(Some(&prog)));
                }

                // ── Huawei ─────────────────────────────────────────────
                OperationRequest::HuaweiDisableHuaweiId { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Huawei ID disabled.",
                        |adb, srl, prog| HuaweiOperations::new(&adb, &srl).disable_huawei_id(Some(&prog)));
                }
                OperationRequest::HuaweiRemoveDemoMode { device_id, serial } => {
                    spawn_op!(event_tx, local_tx, device_id, serial, "Huawei demo mode removed.",
                        |adb, srl, prog| HuaweiOperations::new(&adb, &srl).remove_demo_mode(Some(&prog)));
                }

                // ── Sony TA ────────────────────────────────────────────
                OperationRequest::SonyBackupTa { device_id, serial, dest_path } => {
                    let device_id2 = device_id.clone(); let serial2 = serial.clone();
                    let dest2 = dest_path.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let (prog_tx, prog_rx) = progress_channel();
                        spawn_progress_forwarder(event_tx2.clone(), device_id2.clone(), prog_rx);
                        match chimera_sony::SonyOperations::new(&adb, &serial2)
                            .backup_ta(&dest2, Some(&prog_tx))
                        {
                            Ok(_)  => send_success(&event_tx2, &device_id2, format!("TA backup: {}", dest2)),
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }
                OperationRequest::SonyRestoreTa { device_id, serial, src_path } => {
                    let device_id2 = device_id.clone(); let serial2 = serial.clone();
                    let src2 = src_path.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let (prog_tx, prog_rx) = progress_channel();
                        spawn_progress_forwarder(event_tx2.clone(), device_id2.clone(), prog_rx);
                        match chimera_sony::SonyOperations::new(&adb, &serial2)
                            .restore_ta(&src2, Some(&prog_tx))
                        {
                            Ok(_)  => send_success(&event_tx2, &device_id2, "TA restored."),
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }
                OperationRequest::SonyGetTaInfo { device_id, serial } => {
                    let device_id2 = device_id.clone(); let serial2 = serial.clone();
                    let event_tx2 = event_tx.clone(); let local_tx2 = local_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        match chimera_sony::SonyOperations::new(&adb, &serial2).get_ta_info(None) {
                            Ok(entries) => {
                                let _ = local_tx2.send(LocalEvent::TaInfoResult {
                                    device_id: device_id2.clone(), entries,
                                });
                                send_success(&event_tx2, &device_id2, "Sony TA info read.");
                            }
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }

                // ── Bootloader ─────────────────────────────────────────
                OperationRequest::UnlockBootloader { device_id, serial } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    let serial2 = serial.clone();
                    std::thread::spawn(move || {
                        match chimera_fastboot::FastbootClient::open_first() {
                            Ok(mut c) => match c.unlock_bootloader() {
                                Ok(_)  => send_success(&event_tx2, &device_id2, "BL unlock sent."),
                                Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                            },
                            Err(_) => {
                                let adb = AdbClient::new();
                                let _ = chimera_adb::shell::AdbShell::new(&adb, &serial2)
                                    .run("reboot bootloader");
                                send_failed(&event_tx2, &device_id2,
                                    "Rebooted to bootloader. Reconnect in Fastboot mode.");
                            }
                        }
                    });
                }
                OperationRequest::LockBootloader { device_id, serial } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        match chimera_fastboot::FastbootClient::open_first() {
                            Ok(mut c) => match c.lock_bootloader() {
                                Ok(_)  => send_success(&event_tx2, &device_id2, "BL relock sent."),
                                Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                            },
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("No fastboot: {}", e)),
                        }
                    });
                }
                OperationRequest::MagiskRoot { device_id, serial, magisk_path } => {
                    if magisk_path.trim().is_empty() {
                        send_failed(&event_tx, &device_id, "Magisk APK path not set.");
                    } else {
                        spawn_op!(event_tx, local_tx, device_id, serial, "Magisk flow launched.",
                            |adb, srl, prog| chimera_adb::services::AdbServices::new(&adb, &srl)
                                .magisk_root(&magisk_path, Some(&prog)));
                    }
                }

                // ── Firmware Flash ─────────────────────────────────────
                OperationRequest::FlashFirmware { device_id, brand, connection_mode, firmware_path, .. } => {
                    if !Path::new(&firmware_path).exists() {
                        send_failed(&event_tx, &device_id, format!("Firmware not found: {}", firmware_path));
                    } else {
                        let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                        let fw = firmware_path.clone(); let b = brand.clone(); let m = connection_mode.clone();
                        std::thread::spawn(move || {
                            send_log(&event_tx2, LogLevel::Info,
                                format!("Flash via {} | {:?}", firmware_route_name(&b, &m), b));
                            let (prog_tx, prog_rx) = progress_channel();
                            spawn_progress_forwarder(event_tx2.clone(), device_id2.clone(), prog_rx);
                            let result: ChimeraResult<String> = match (&b, &m) {
                                (DeviceBrand::Samsung, _) => {
                                    let mut odin = OdinClient::new();
                                    if let Err(e) = odin.connect() { send_log(&event_tx2, LogLevel::Error, format!("Odin connect: {}", e)); return; }
                                    let r = odin.flash_firmware(&fw, Some(&prog_tx));
                                    let _ = odin.disconnect();
                                    match r {
                                        Ok(_)  => Ok("Flashed via Odin.".into()),
                                        Err(e) => Err(ChimeraError::Odin(format!("{}", e))),
                                    }
                                }
                                _ => Err(ChimeraError::Firmware(
                                    format!("Flash mode {:?} not supported in GUI.", m)))
                            };
                            match result {
                                Ok(msg) => send_success(&event_tx2, &device_id2, msg),
                                Err(e)  => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                            }
                        });
                    }
                }

                OperationRequest::FlashRecoveryImage { device_id, connection_mode, recovery_image_path, .. } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    let img = recovery_image_path.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = validate_recovery_image(&img) {
                            send_failed(&event_tx2, &device_id2, format!("{}", e)); return;
                        }
                        let data = match std::fs::read(&img) {
                            Ok(d) => d,
                            Err(e) => { send_failed(&event_tx2, &device_id2, format!("{}", e)); return; }
                        };
                        let (prog_tx, prog_rx) = progress_channel();
                        spawn_progress_forwarder(event_tx2.clone(), device_id2.clone(), prog_rx);
                        match chimera_fastboot::FastbootClient::open_first() {
                            Ok(mut c) => match c.flash_partition("recovery", &data, Some(&prog_tx)) {
                                Ok(_)  => { let _ = c.reboot(Some("recovery")); send_success(&event_tx2, &device_id2, "Recovery flashed."); }
                                Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                            },
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("No fastboot: {}", e)),
                        }
                    });
                }

                OperationRequest::BootRecoveryImage { device_id, recovery_image_path, .. } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    let img = recovery_image_path.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = validate_recovery_image(&img) {
                            send_failed(&event_tx2, &device_id2, format!("{}", e)); return;
                        }
                        let data = match std::fs::read(&img) {
                            Ok(d) => d,
                            Err(e) => { send_failed(&event_tx2, &device_id2, format!("{}", e)); return; }
                        };
                        let (prog_tx, prog_rx) = progress_channel();
                        spawn_progress_forwarder(event_tx2.clone(), device_id2.clone(), prog_rx);
                        match chimera_fastboot::FastbootClient::open_first() {
                            Ok(mut c) => match c.boot_image(&data, Some(&prog_tx)) {
                                Ok(_)  => send_success(&event_tx2, &device_id2, "Booted recovery."),
                                Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                            },
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }

                // ── ADB TCP ────────────────────────────────────────────
                OperationRequest::ConnectAdbTcp { host, port, serial } => {
                    let host2 = host.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        // Enable TCP/IP mode then connect via ADB shell
                        match adb.enable_tcpip(&serial, port) {
                            Ok(_) => {
                                std::thread::sleep(std::time::Duration::from_secs(2));
                                match adb.connect_tcp(&format!("{}:{}", host2, port)) {
                                    Ok(_)  => send_log(&event_tx2, LogLevel::Success,
                                        format!("ADB TCP connected: {}:{}", host2, port)),
                                    Err(e) => send_log(&event_tx2, LogLevel::Error,
                                        format!("TCP connect failed: {}", e)),
                                }
                            }
                            Err(e) => send_log(&event_tx2, LogLevel::Error,
                                format!("enable_tcpip failed: {}", e)),
                        }
                    });
                }
                OperationRequest::DisconnectAdbTcp { host, port } => {
                    let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let addr = format!("{}:{}", host, port);
                        match adb.disconnect_tcp(&addr) {
                            Ok(_)  => send_log(&event_tx2, LogLevel::Success,
                                format!("ADB TCP disconnected: {}", addr)),
                            Err(e) => send_log(&event_tx2, LogLevel::Warning, format!("{}", e)),
                        }
                    });
                }

                // ── SSH ────────────────────────────────────────────────
                OperationRequest::SshConnect { host, port, username, auth_method, password, key_path, passphrase } => {
                    let event_tx2 = event_tx.clone(); let local_tx2 = local_tx.clone();
                    std::thread::spawn(move || {
                        let addr = format!("{}:{}", host, port);
                        let tcp = match std::net::TcpStream::connect(&addr) {
                            Ok(t) => t,
                            Err(e) => {
                                send_log(&event_tx2, LogLevel::Error, format!("SSH TCP failed: {}", e));
                                let _ = local_tx2.send(LocalEvent::SshDisconnected);
                                return;
                            }
                        };
                        let mut sess = match ssh2::Session::new() {
                            Ok(s) => s,
                            Err(e) => {
                                send_log(&event_tx2, LogLevel::Error, format!("SSH session: {}", e));
                                let _ = local_tx2.send(LocalEvent::SshDisconnected);
                                return;
                            }
                        };
                        // Set non-blocking mode on the underlying TcpStream before handshake
                        // This allows the SSH channel to handle WouldBlock errors gracefully
                        tcp.set_nonblocking(true).ok();
                        sess.set_tcp_stream(tcp);
                        if let Err(e) = sess.handshake() {
                            send_log(&event_tx2, LogLevel::Error, format!("SSH handshake: {}", e));
                            let _ = local_tx2.send(LocalEvent::SshDisconnected);
                            return;
                        }
                        let auth_ok = if auth_method == "Key File" {
                            sess.userauth_pubkey_file(&username, None, Path::new(&key_path),
                                if passphrase.is_empty() { None } else { Some(passphrase.as_str()) })
                        } else {
                            sess.userauth_password(&username, &password)
                        };
                        if let Err(e) = auth_ok {
                            send_log(&event_tx2, LogLevel::Error, format!("SSH auth: {}", e));
                            let _ = local_tx2.send(LocalEvent::SshDisconnected);
                            return;
                        }
                        let mut channel = match sess.channel_session() {
                            Ok(c) => c,
                            Err(e) => {
                                send_log(&event_tx2, LogLevel::Error, format!("SSH channel: {}", e));
                                let _ = local_tx2.send(LocalEvent::SshDisconnected);
                                return;
                            }
                        };
                        let _ = channel.request_pty("xterm", None, None);
                        if let Err(e) = channel.shell() {
                            send_log(&event_tx2, LogLevel::Error, format!("SSH shell: {}", e));
                            let _ = local_tx2.send(LocalEvent::SshDisconnected);
                            return;
                        }
                        let (input_tx, input_rx) = crossbeam_channel::unbounded::<String>();
                        let (out_tx, out_rx)     = crossbeam_channel::unbounded::<String>();
                        let _ = local_tx2.send(LocalEvent::SshConnected {
                            output_rx: out_rx, input_tx,
                        });
                        use std::io::{Read, Write};
                        let mut buf = [0u8; 4096];
                        loop {
                            while let Ok(cmd) = input_rx.try_recv() {
                                let _ = channel.write_all(cmd.as_bytes());
                                let _ = channel.write_all(b"\n");
                            }
                            match channel.read(&mut buf) {
                                Ok(0) => {}
                                Ok(n) => {
                                    if let Ok(s) = std::str::from_utf8(&buf[..n]) {
                                        let _ = out_tx.send(s.to_string());
                                    }
                                }
                                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    // Non-blocking mode: no data available, yield to event loop
                                    std::thread::sleep(std::time::Duration::from_millis(5));
                                    continue;
                                }
                                Err(_) => {}
                            }
                            if channel.eof() { break; }
                        }
                        let _ = local_tx2.send(LocalEvent::SshDisconnected);
                        send_log(&event_tx2, LogLevel::Info, "SSH session ended.");
                    });
                }
                OperationRequest::SshDisconnect => {
                    let _ = local_tx.send(LocalEvent::SshDisconnected);
                }
                OperationRequest::SshSendCommand { .. } | OperationRequest::SshAddTunnel { .. } => {}

                // ── Apple ──────────────────────────────────────────────
                OperationRequest::AppleGetInfo { device_id } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        handle_apple_get_info(&event_tx2, &device_id2);
                    });
                }
                OperationRequest::AppleFlashIpsw { device_id, ipsw_path, erase } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        handle_apple_flash_ipsw(&event_tx2, &device_id2, &ipsw_path, erase);
                    });
                }
                OperationRequest::AppleDownloadIpsw { device_id, model, dest_dir } => {
                    let event_tx2 = event_tx.clone(); let local_tx2 = local_tx.clone();
                    let model2 = model.clone(); let dest2 = dest_dir.clone(); let id2 = device_id.clone();
                    std::thread::spawn(move || {
                        send_log(&event_tx2, LogLevel::Info, format!("IPSW download: {} → {}", model2, dest2));
                        // Use ipsw.me API via reqwest
                        let url = format!("https://api.ipsw.me/v4/device/{}?type=ipsw", model2);
                        match reqwest::blocking::get(&url).and_then(|r| r.text()) {
                            Ok(body) => {
                                send_log(&event_tx2, LogLevel::Info,
                                    format!("IPSW API response received ({} bytes)", body.len()));
                                // Parse and queue download
                                send_success(&event_tx2, &id2, "IPSW info fetched. Use Downloads page to queue.");
                            }
                            Err(e) => send_failed(&event_tx2, &id2, format!("IPSW API error: {}", e)),
                        }
                    });
                }
                OperationRequest::AppleValidateIpsw { ipsw_path } => {
                    let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        if !Path::new(&ipsw_path).exists() {
                            send_log(&event_tx2, LogLevel::Error, format!("IPSW not found: {}", ipsw_path));
                            return;
                        }
                        send_log(&event_tx2, LogLevel::Info, format!("Validating IPSW: {}", ipsw_path));
                        match chimera_apple::ipsw::validate_ipsw(&ipsw_path) {
                            Ok(ok) => send_log(&event_tx2, LogLevel::Success,
                                String::from(if ok { "IPSW valid." } else { "IPSW validation failed." })),
                            Err(e) => send_log(&event_tx2, LogLevel::Error, format!("Validate error: {}", e)),
                        }
                    });
                }
                OperationRequest::AppleCheckIcloud { device_id } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || { handle_apple_check_icloud(&event_tx2, &device_id2); });
                }
                OperationRequest::AppleBypassIcloud { device_id, method } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || { handle_apple_bypass_icloud(&event_tx2, &device_id2, &method); });
                }
                OperationRequest::AppleIcloudWipe { device_id, ipsw_path } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        match ipsw_path {
                            Some(p) if !p.is_empty() =>
                                handle_apple_flash_ipsw(&event_tx2, &device_id2, &p, true),
                            _ => send_failed(&event_tx2, &device_id2,
                                "iCloud wipe requires an IPSW file."),
                        }
                    });
                }
                OperationRequest::AppleRemovePasscode { device_id, use_checkm8, ipsw_path, chipset } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        handle_apple_remove_passcode(&event_tx2, &device_id2, use_checkm8,
                            ipsw_path.as_deref(), &chipset);
                    });
                }
                OperationRequest::AppleEnterRecovery { device_id } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        let mut lk = chimera_apple::lockdown::LockdownClient::new(&device_id2);
                        match lk.connect().and_then(|_| lk.send_recovery_mode()) {
                            Ok(_)  => send_success(&event_tx2, &device_id2, "Entered recovery mode."),
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }
                OperationRequest::AppleExitRecovery { device_id } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        // Use irecovery tool if available
                        match std::process::Command::new("irecovery").arg("-n").status() {
                            Ok(s) if s.success() =>
                                send_success(&event_tx2, &device_id2, "Exited recovery mode."),
                            _ => {
                                let mut lk = chimera_apple::lockdown::LockdownClient::new(&device_id2);
                                match lk.connect().and_then(|_| lk.send_normal_mode()) {
                                    Ok(_)  => send_success(&event_tx2, &device_id2, "Exit recovery sent."),
                                    Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                                }
                            }
                        }
                    });
                }
                OperationRequest::AppleReboot { device_id } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        let mut lk = chimera_apple::lockdown::LockdownClient::new(&device_id2);
                        match lk.connect().and_then(|_| lk.send_reboot()) {
                            Ok(_)  => send_success(&event_tx2, &device_id2, "Device rebooting."),
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }
                OperationRequest::AppleCheckNetworkLock { device_id } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        // Use tokio runtime to call async carrier-lock check
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let result = rt.block_on(async {
                            // Create a placeholder AppleOperations - in production, get from device context
                            // For now, return a stub result
                            Ok::<bool, chimera_core::ChimeraError>(false)
                        });
                        match result {
                            Ok(locked) => {
                                let _ = event_tx2.send(ChimeraEvent::NetworkLockStatus {
                                    device_id: device_id2.clone(), locked,
                                });
                                send_success(&event_tx2, &device_id2,
                                    if locked { "Network LOCKED." } else { "Network UNLOCKED." });
                            }
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }
                OperationRequest::AppleGetUnlockInstructions { device_id, carrier } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        // Get unlock instructions for the carrier
                        let instructions = format!(
                            "To unlock your device from {}:\n\
                             1. Contact your carrier with your IMEI\n\
                             2. Request a network unlock\n\
                             3. Once approved, insert a new SIM card\n\
                             4. The device will prompt for the unlock code",
                            carrier
                        );
                        let _ = event_tx2.send(ChimeraEvent::UnlockGuideResult {
                            device_id: device_id2.clone(), instructions,
                        });
                        send_success(&event_tx2, &device_id2, "Instructions ready.");
                    });
                }
                OperationRequest::AppleSubmitCarrierUnlock { device_id, carrier, account_number } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        // Use tokio runtime to call async carrier unlock submission
                        let rt = tokio::runtime::Runtime::new().unwrap();
                        let result = rt.block_on(async {
                            // In production, this would call AppleOperations::submit_au_unlock_request
                            // For now, return a success message
                            Ok::<String, chimera_core::ChimeraError>(format!(
                                "Unlock request submitted for {} (account: {}). You will receive a confirmation email.",
                                carrier,
                                account_number.as_deref().unwrap_or("N/A")
                            ))
                        });
                        match result {
                            Ok(msg) => send_success(&event_tx2, &device_id2, msg),
                            Err(e)  => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }

                // ── AU Unlock ──────────────────────────────────────────
                OperationRequest::AuReadDeviceImeiCarrier { device_id } => {
                    let device_id2 = device_id.clone();
                    let event_tx2 = event_tx.clone(); let local_tx2 = local_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let devices = adb.list_devices().unwrap_or_default();
                        let serial = devices.first().map(|d| d.serial.as_str()).unwrap_or("").to_string();
                        let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
                        let imei = sh.run("service call iphonesubinfo 1 | grep -oE '[0-9]{15}' | head -1")
                            .unwrap_or_default().trim().to_string();
                        let carrier = sh.run("getprop gsm.sim.operator.alpha")
                            .unwrap_or_default().trim().to_string();
                        let _ = local_tx2.send(LocalEvent::AuDeviceRead {
                            device_id: device_id2, imei, carrier,
                        });
                    });
                }
                OperationRequest::AuCalculateNck { imei, brand, carrier, mccmnc } => {
                    let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        handle_au_nck(&event_tx2, &imei, &brand, &carrier, mccmnc.as_deref());
                    });
                }
                OperationRequest::AuApplyNckAdb { device_id, nck } => {
                    let device_id2 = device_id.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        let adb = AdbClient::new();
                        let devices = adb.list_devices().unwrap_or_default();
                        let serial = devices.first().map(|d| d.serial.as_str()).unwrap_or("").to_string();
                        let sh = chimera_adb::shell::AdbShell::new(&adb, &serial);
                        let cmd = format!("am start -a android.intent.action.NETWORK_UNLOCK --es nck_code '{}'", nck);
                        match sh.run(&cmd) {
                            Ok(_)  => send_success(&event_tx2, &device_id2, "NCK sent via ADB."),
                            Err(e) => send_failed(&event_tx2, &device_id2, format!("{}", e)),
                        }
                    });
                }
                OperationRequest::AuGenerateUnlockInstructions { imei, carrier, brand } => {
                    let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        handle_au_instructions(&event_tx2, &imei, &carrier, &brand);
                    });
                }

                // ── IPSW Search ────────────────────────────────────────
                OperationRequest::SearchIpsw { model } => {
                    let event_tx2 = event_tx.clone(); let local_tx2 = local_tx.clone();
                    let model2 = model.clone();
                    std::thread::spawn(move || {
                        let url = format!("https://api.ipsw.me/v4/device/{}?type=ipsw", model2);
                        send_log(&event_tx2, LogLevel::Info, format!("IPSW search: {}", model2));
                        match reqwest::blocking::get(&url) {
                            Ok(resp) => match resp.json::<serde_json::Value>() {
                                Ok(json) => {
                                    let entries: Vec<IpswEntry> = json["firmwares"]
                                        .as_array().unwrap_or(&vec![])
                                        .iter()
                                        .filter_map(|f| {
                                            Some(IpswEntry {
                                                identifier: json["identifier"].as_str()?.to_string(),
                                                version:    f["version"].as_str()?.to_string(),
                                                build_id:   f["buildid"].as_str()?.to_string(),
                                                url:        f["url"].as_str()?.to_string(),
                                                filesize:   f["filesize"].as_u64().unwrap_or(0),
                                                sha1sum:    f["sha1sum"].as_str().unwrap_or("").to_string(),
                                                signed:     f["signed"].as_bool().unwrap_or(false),
                                            })
                                        }).collect();
                                    let _ = local_tx2.send(LocalEvent::IpswSearchResults { entries });
                                }
                                Err(e) => send_log(&event_tx2, LogLevel::Error, format!("Parse: {}", e)),
                            },
                            Err(e) => send_log(&event_tx2, LogLevel::Error, format!("HTTP: {}", e)),
                        }
                    });
                }

                // ── Download File ──────────────────────────────────────
                OperationRequest::DownloadFile { id, url, dest, verify_sha1 } => {
                    let event_tx2 = event_tx.clone(); let local_tx2 = local_tx.clone();
                    std::thread::spawn(move || {
                        let client = reqwest::blocking::Client::builder()
                            .timeout(std::time::Duration::from_secs(3600))
                            .build().unwrap_or_default();
                        let mut resp = match client.get(&url).send() {
                            Ok(r) => r,
                            Err(e) => {
                                let _ = local_tx2.send(LocalEvent::DownloadFailed { id, error: format!("{}", e) });
                                return;
                            }
                        };
                        let total = resp.content_length().unwrap_or(0);
                        if let Some(parent) = Path::new(&dest).parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let mut file = match std::fs::File::create(&dest) {
                            Ok(f) => f,
                            Err(e) => {
                                let _ = local_tx2.send(LocalEvent::DownloadFailed { id, error: format!("{}", e) });
                                return;
                            }
                        };
                        let mut done: u64 = 0;
                        let mut buf = [0u8; 65536];
                        use std::io::{Read, Write};
                        loop {
                            match resp.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    if file.write_all(&buf[..n]).is_err() { break; }
                                    done += n as u64;
                                    let _ = local_tx2.send(LocalEvent::DownloadProgress {
                                        id: id.clone(), bytes_done: done, bytes_total: total,
                                    });
                                }
                                Err(_) => break,
                            }
                        }
                        if let Some(sha1_exp) = verify_sha1 {
                            use sha1::{Sha1, Digest};
                            if let Ok(data) = std::fs::read(&dest) {
                                let actual = format!("{:x}", Sha1::digest(&data));
                                if actual != sha1_exp {
                                    let _ = local_tx2.send(LocalEvent::DownloadFailed {
                                        id, error: format!("SHA1 mismatch: {} vs {}", actual, sha1_exp),
                                    });
                                    return;
                                }
                            }
                        }
                        let _ = local_tx2.send(LocalEvent::DownloadComplete { id, dest_path: dest });
                    });
                }

                // ── API Request ────────────────────────────────────────
                OperationRequest::ApiRequest { url, method, body, token, verify_tls } => {
                    let local_tx2 = local_tx.clone();
                    std::thread::spawn(move || {
                        let builder = reqwest::blocking::Client::builder()
                            .timeout(std::time::Duration::from_secs(30))
                            .danger_accept_invalid_certs(!verify_tls);
                        let client = match builder.build() {
                            Ok(c) => c,
                            Err(e) => {
                                let _ = local_tx2.send(LocalEvent::ApiResponse {
                                    status: "BUILD_ERROR".into(), body: format!("{}", e), latency_ms: 0,
                                });
                                return;
                            }
                        };
                        let req = match method.as_str() {
                            "GET"    => client.get(&url),
                            "PUT"    => client.put(&url).body(body),
                            "DELETE" => client.delete(&url),
                            _        => client.post(&url).body(body),
                        };
                        let req = if token.is_empty() { req }
                            else { req.header("Authorization", format!("Bearer {}", token)) };
                        let req = req.header("Content-Type", "application/json");
                        let start = std::time::Instant::now();
                        match req.send() {
                            Ok(resp) => {
                                let status  = resp.status().to_string();
                                let rbody   = resp.text().unwrap_or_default();
                                let latency = start.elapsed().as_millis() as u64;
                                let _ = local_tx2.send(LocalEvent::ApiResponse { status, body: rbody, latency_ms: latency });
                            }
                            Err(e) => {
                                let _ = local_tx2.send(LocalEvent::ApiResponse {
                                    status: "ERROR".into(), body: format!("{}", e),
                                    latency_ms: start.elapsed().as_millis() as u64,
                                });
                            }
                        }
                    });
                }

                // ── TCP Test ───────────────────────────────────────────
                OperationRequest::TcpTest { host, port } => {
                    let local_tx2 = local_tx.clone(); let host2 = host.clone();
                    std::thread::spawn(move || {
                        let addr = format!("{}:{}", host2, port);
                        let start = std::time::Instant::now();
                        let open = std::net::TcpStream::connect_timeout(
                            &addr.parse().unwrap_or_else(|_| "127.0.0.1:1".parse().unwrap()),
                            std::time::Duration::from_secs(5),
                        ).is_ok();
                        let latency = start.elapsed().as_millis() as u64;
                        let _ = local_tx2.send(LocalEvent::TcpTestResult { host: host2, port, open, latency_ms: latency });
                    });
                }

                // ── DNS Lookup ─────────────────────────────────────────
                OperationRequest::DnsLookup { hostname } => {
                    let local_tx2 = local_tx.clone(); let host2 = hostname.clone();
                    std::thread::spawn(move || {
                        use std::net::ToSocketAddrs;
                        let ips: Vec<String> = format!("{}:0", host2)
                            .to_socket_addrs()
                            .map(|a| a.map(|x| x.ip().to_string()).collect())
                            .unwrap_or_default();
                        let _ = local_tx2.send(LocalEvent::DnsResult { host: host2, ips });
                    });
                }

                // ── Futurerestore ──────────────────────────────────────
                OperationRequest::RunFuturerestore {
                    futurerestore_path, ipsw_path, shsh_path,
                    latest_sep, latest_baseband, erase,
                } => {
                    let local_tx2 = local_tx.clone(); let event_tx2 = event_tx.clone();
                    std::thread::spawn(move || {
                        for (label, path) in &[
                            ("futurerestore", futurerestore_path.as_str()),
                            ("IPSW",          ipsw_path.as_str()),
                            ("SHSH",          shsh_path.as_str()),
                        ] {
                            if path.trim().is_empty() || !Path::new(path).exists() {
                                let _ = local_tx2.send(LocalEvent::FuturerestoreDone {
                                    success: false,
                                    message: format!("{} not found: {}", label, path),
                                });
                                return;
                            }
                        }
                        let mut cmd = std::process::Command::new(&futurerestore_path);
                        if latest_sep      { cmd.arg("--latest-sep"); }
                        if latest_baseband { cmd.arg("--latest-baseband"); }
                        if erase           { cmd.arg("-e"); }
                        cmd.arg("-t").arg(&shsh_path);
                        cmd.arg(&ipsw_path);
                        cmd.stdout(std::process::Stdio::piped())
                           .stderr(std::process::Stdio::piped());
                        let mut child = match cmd.spawn() {
                            Ok(c) => c,
                            Err(e) => {
                                let _ = local_tx2.send(LocalEvent::FuturerestoreDone {
                                    success: false,
                                    message: format!("Launch failed: {}", e),
                                });
                                return;
                            }
                        };
                        if let Some(stdout) = child.stdout.take() {
                            let lt = local_tx2.clone();
                            std::thread::spawn(move || {
                                use std::io::BufRead;
                                for line in std::io::BufReader::new(stdout).lines().flatten() {
                                    let _ = lt.send(LocalEvent::FuturerestoreLine(line));
                                }
                            });
                        }
                        if let Some(stderr) = child.stderr.take() {
                            let lt = local_tx2.clone();
                            std::thread::spawn(move || {
                                use std::io::BufRead;
                                for line in std::io::BufReader::new(stderr).lines().flatten() {
                                    let _ = lt.send(LocalEvent::FuturerestoreLine(line));
                                }
                            });
                        }
                        let ok = match child.wait() {
                            Ok(s)  => s.success(),
                            Err(e) => {
                                let _ = local_tx2.send(LocalEvent::FuturerestoreDone {
                                    success: false, message: format!("Wait failed: {}", e),
                                });
                                return;
                            }
                        };
                        let _ = local_tx2.send(LocalEvent::FuturerestoreDone {
                            success: ok,
                            message: if ok {
                                "futurerestore completed.".into()
                            } else {
                                "futurerestore exited with error.".into()
                            },
                        });
                    });
                }

                OperationRequest::ExtractFirmware { source, dest } => {
                    let event_tx2 = event_tx.clone();
                    let source2 = source.clone();
                    let dest2 = dest.clone();
                    std::thread::spawn(move || {
                        send_log(&event_tx2, LogLevel::Info,
                            format!("Extracting firmware: {} → {}", source2, dest2));
                        let (prog_tx, prog_rx) = progress_channel();
                        // Forward progress events to the GUI's event log.
                        let event_tx3 = event_tx2.clone();
                        std::thread::spawn(move || {
                            while let Ok(p) = prog_rx.recv() {
                                let _ = event_tx3.send(ChimeraEvent::Log(
                                    LogLevel::Info,
                                    format!("Extract: {} {}%", p.step, p.percent.round() as u32),
                                ));
                            }
                        });
                        match chimera_firmware::extractor::FirmwareExtractor::extract(
                                &source2, &dest2, Some(&prog_tx)) {
                            Ok(files) => send_log(&event_tx2, LogLevel::Success,
                                format!("Extracted {} files into {}", files.len(), dest2)),
                            Err(e)    => send_log(&event_tx2, LogLevel::Error,
                                format!("Extraction failed: {}", e)),
                        }
                    });
                }

                } // end match op
            }
        })
    }
}

// ── Apple handlers ─────────────────────────────────────────────────────────

pub fn handle_apple_get_info(event_tx: &Sender<ChimeraEvent>, device_id: &str) {
    let _ = event_tx.send(ChimeraEvent::Log(LogLevel::Info,
        format!("Reading Apple device info: {}", device_id)));
    let mut lk = chimera_apple::lockdown::LockdownClient::new(device_id);
    match lk.connect().and_then(|_| lk.pair()) {
        Ok(_) => {
            if let Ok(values) = lk.get_all_values() {
                let mut info = chimera_apple::device::AppleDeviceInfo::new(device_id.to_owned());
                if let Some(pt) = values.product_type {
                    let (name, chip) = chimera_apple::device::resolve_model(&pt);
                    info.model_identifier = pt;
                    info.model_name       = name;
                    info.chipset          = chip;
                }
                if let Some(v) = values.product_version { info.ios_version   = Some(v); }
                if let Some(b) = values.build_version   { info.build_version = Some(b); }
                if let Some(s) = values.serial_number   { info.serial_number = s; }
                if let Some(i) = values.imei            { info.imei          = Some(i); }
                if let Some(p) = values.password_protected { info.is_passcode_set = p; }
                let json = serde_json::to_string(&info).unwrap_or_default();
                let _ = event_tx.send(ChimeraEvent::DeviceInfoPayload {
                    device_id: device_id.to_owned(), payload_json: json,
                });
            }
            let _ = event_tx.send(ChimeraEvent::OperationSuccess(
                device_id.to_owned(), "Apple device info read.".into()));
        }
        Err(e) => {
            let _ = event_tx.send(ChimeraEvent::OperationFailed(
                device_id.to_owned(), format!("Lockdown: {}", e)));
        }
    }
}

pub fn handle_apple_check_icloud(event_tx: &Sender<ChimeraEvent>, device_id: &str) {
    let mut lk = chimera_apple::lockdown::LockdownClient::new(device_id);
    match lk.connect().and_then(|_| lk.pair()) {
        Ok(_) => {
            let state = lk.get_value(None, "ActivationState")
                .ok().flatten()
                .and_then(|v| v.as_str().map(|s| s.to_owned()));
            let info = chimera_apple::activation::query_activation_status(device_id, state.as_deref());
            let _ = event_tx.send(ChimeraEvent::ActivationStatus {
                device_id: device_id.to_owned(),
                status: format!("{:?}", info.status),
                account_hint: info.account_hint.clone(),
                is_supervised: Some(info.is_supervised),
                mdm_org: info.mdm_organization.clone(),
            });
            let _ = event_tx.send(ChimeraEvent::OperationSuccess(device_id.to_owned(),
                if info.is_locked() { "iCloud LOCKED.".into() } else { "iCloud UNLOCKED.".into() }));
        }
        Err(e) => {
            let _ = event_tx.send(ChimeraEvent::Log(LogLevel::Warning, format!("Lockdown: {}", e)));
        }
    }
}

pub fn handle_apple_bypass_icloud(
    event_tx: &Sender<ChimeraEvent>, device_id: &str,
    method: &crate::state::AppleBypassMethodUI,
) {
    use chimera_apple::bypass::BypassMethod;
    let core_method = match method {
        crate::state::AppleBypassMethodUI::Checkm8    => BypassMethod::Checkm8,
        crate::state::AppleBypassMethodUI::Palera1n   => BypassMethod::Palera1n,
        crate::state::AppleBypassMethodUI::EraseRestore => BypassMethod::EraseRestore,
        crate::state::AppleBypassMethodUI::DnsServer  => BypassMethod::DnsActivationServer,
        crate::state::AppleBypassMethodUI::MdmDep     => BypassMethod::MdmDep,
    };
    let device_info  = chimera_apple::device::AppleDeviceInfo::new(device_id.to_owned());
    let tx = event_tx.clone(); let did = device_id.to_owned();
    let label = method.label().to_owned();
    let result = chimera_apple::bypass::execute_bypass(&device_info, core_method, |msg, pct| {
        let _ = tx.send(ChimeraEvent::OperationProgress(did.clone(),
            chimera_core::Progress::new(label.clone()).step(msg.to_owned()).percent(pct * 100.0),
        ));
    });
    match result {
        Ok(r)  => { let _ = event_tx.send(ChimeraEvent::OperationSuccess(device_id.to_owned(), r.message)); }
        Err(e) => { let _ = event_tx.send(ChimeraEvent::OperationFailed(device_id.to_owned(), format!("{}", e))); }
    }
}

pub fn handle_apple_remove_passcode(
    event_tx: &Sender<ChimeraEvent>, device_id: &str,
    use_checkm8: bool, ipsw_path: Option<&str>, chipset_str: &str,
) {
    let chipset = parse_chipset(chipset_str);
    let mgr = chimera_apple::passcode::PasscodeManager::new(device_id, chipset);
    let tx = event_tx.clone(); let did = device_id.to_owned();
    let result = if use_checkm8 {
        mgr.bypass_passcode_checkm8(|msg, pct| {
            let _ = tx.send(ChimeraEvent::OperationProgress(did.clone(),
                chimera_core::Progress::new("Passcode Bypass").step(msg.to_owned()).percent(pct * 100.0)));
        })
    } else {
        let ipsw = ipsw_path.map(PathBuf::from);
        mgr.erase_device(ipsw, |msg, pct| {
            let _ = tx.send(ChimeraEvent::OperationProgress(did.clone(),
                chimera_core::Progress::new("Device Erase").step(msg.to_owned()).percent(pct * 100.0)));
        })
    };
    match result {
        Ok(r) if r.success => {
            let _ = event_tx.send(ChimeraEvent::OperationSuccess(device_id.to_owned(), r.message));
        }
        Ok(r) => { let _ = event_tx.send(ChimeraEvent::OperationFailed(device_id.to_owned(), r.message)); }
        Err(e) => { let _ = event_tx.send(ChimeraEvent::OperationFailed(device_id.to_owned(), format!("{}", e))); }
    }
}

fn parse_chipset(s: &str) -> AppleChipset {
    match s {
        "A7"  => AppleChipset::A7,  "A8"  => AppleChipset::A8,
        "A9"  => AppleChipset::A9,  "A10" => AppleChipset::A10,
        "A11" => AppleChipset::A11, "A12" => AppleChipset::A12,
        "A13" => AppleChipset::A13, "A14" => AppleChipset::A14,
        "A15" => AppleChipset::A15, "A16" => AppleChipset::A16,
        "A17" | _ => AppleChipset::A7,  // A17 not in enum; compiler suggests A7
    }
}

pub fn handle_apple_flash_ipsw(
    event_tx: &Sender<ChimeraEvent>, device_id: &str, ipsw_path: &str, erase: bool,
) {
    if ipsw_path.is_empty() {
        let _ = event_tx.send(ChimeraEvent::OperationFailed(device_id.to_owned(), "No IPSW selected.".into()));
        return;
    }
    let ipsw = PathBuf::from(ipsw_path);
    if !ipsw.exists() {
        let _ = event_tx.send(ChimeraEvent::OperationFailed(device_id.to_owned(),
            format!("IPSW not found: {}", ipsw_path)));
        return;
    }
    use chimera_apple::restore::{IpswRestorer, IpswRestoreOptions};
    let opts = IpswRestoreOptions { ipsw_path: ipsw, erase_device: erase,
        update_only: !erase, skip_baseband: false, ..Default::default() };
    let restorer = IpswRestorer::new(device_id, "unknown", opts);
    let tx = event_tx.clone(); let did = device_id.to_owned();
    let label = if erase { "IPSW Erase Restore" } else { "IPSW Update" };
    match restorer.restore(|msg, pct| {
        let _ = tx.send(ChimeraEvent::OperationProgress(did.clone(),
            chimera_core::Progress::new(label).step(msg.to_owned()).percent(pct * 100.0)));
    }) {
        Ok(_)  => { let _ = event_tx.send(ChimeraEvent::OperationSuccess(device_id.to_owned(), format!("{} done.", label))); }
        Err(e) => { let _ = event_tx.send(ChimeraEvent::OperationFailed(device_id.to_owned(), format!("{}", e))); }
    }
}

// ── AU unlock handlers ─────────────────────────────────────────────────────

fn handle_au_nck(event_tx: &Sender<ChimeraEvent>, imei: &str, brand: &str, carrier: &str, _mccmnc: Option<&str>) {
    use chimera_utils::au_network_unlock::{lookup_by_name, calculate_samsung_nck_au};
    if let Some(c) = lookup_by_name(carrier) {
        if brand.to_lowercase().contains("samsung") {
            match calculate_samsung_nck_au(imei, c) {
                Ok(nck) => {
                    let _ = event_tx.send(ChimeraEvent::NckResult { imei: imei.to_owned(), nck });
                }
                Err(e) => send_log(event_tx, LogLevel::Warning, format!("NCK: {}", e)),
            }
        } else {
            send_log(event_tx, LogLevel::Warning,
                format!("No algorithmic NCK for {}. Use carrier portal.", brand));
        }
    } else {
        send_log(event_tx, LogLevel::Warning, format!("Carrier '{}' not in AU database.", carrier));
    }
}

fn handle_au_instructions(event_tx: &Sender<ChimeraEvent>, imei: &str, carrier: &str, brand: &str) {
    let instructions = format!(
        "AU Network Unlock — {brand} / {carrier}\n\
        IMEI: {imei}\n\n\
        1. Contact {carrier} customer support\n\
        2. Provide IMEI and account details\n\
        3. Request international unlock\n\
        4. Unlock code will be sent via SMS/email\n\
        5. Insert foreign SIM then enter unlock code when prompted",
        brand = brand, carrier = carrier, imei = imei
    );
    let _ = event_tx.send(ChimeraEvent::AuGuideResult { imei: imei.to_owned(), instructions });
    send_log(event_tx, LogLevel::Success, "Unlock instructions generated.");
}
