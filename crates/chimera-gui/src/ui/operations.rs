// chimera-gui/src/ui/operations.rs
// Operations tab - all device repair operations
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui::{self, RichText};
use crate::state::{AppState, OperationStatus};
use crate::worker::OperationRequest;
use crate::ui::common::*;
use chimera_core::device::{DeviceBrand, ConnectionMode, SupportedOperations};
use chimera_devices::detector::DeviceDetector;
use crossbeam_channel::Sender;

fn twrp_route_label(brand: &DeviceBrand, mode: &ConnectionMode) -> &'static str {
    match (brand, mode) {
        (DeviceBrand::Samsung, _) => "Samsung devices should use Odin / recovery TAR workflows instead of generic TWRP fastboot flashing",
        (DeviceBrand::Huawei | DeviceBrand::Honor, ConnectionMode::HuaweiFastboot) => "Huawei fastboot route is not wired for generic TWRP flashing in this build",
        (_, ConnectionMode::Fastboot) => "Fastboot mode → boot or flash a TWRP/custom recovery image",
        (_, ConnectionMode::Adb) => "ADB mode → reboot to Bootloader/Fastboot first, then use TWRP actions",
        _ => "No TWRP route exposed for the current device mode",
    }
}

fn twrp_supported_brand(brand: &DeviceBrand) -> bool {
    !matches!(brand, DeviceBrand::Samsung)
}

fn twrp_section_visible(brand: &DeviceBrand, mode: &ConnectionMode, supported: &SupportedOperations) -> bool {
    match mode {
        ConnectionMode::Fastboot => twrp_supported_brand(brand),
        ConnectionMode::Adb => twrp_supported_brand(brand) && (supported.bootloader_unlock || supported.update_firmware || supported.root || supported.magisk_root),
        _ => false,
    }
}

fn twrp_ready_to_run(brand: &DeviceBrand, mode: &ConnectionMode) -> bool {
    twrp_supported_brand(brand) && matches!(mode, ConnectionMode::Fastboot)
}

pub fn render_operations(
    ui: &mut egui::Ui,
    state: &mut AppState,
    device_id: &str,
    op_tx: &Sender<OperationRequest>,
) {
    let device_state = match state.devices.get(device_id) {
        Some(d) => d.clone(),
        None => return,
    };
    
    let device = &device_state.device;
    let serial = device.serial.clone().unwrap_or_default();
    let supported = DeviceDetector::get_supported_ops(device);
    
    // Show operation status
    match &device_state.operation_status {
        OperationStatus::Running { name, percent, step } => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new(format!("Running: {} - {}", name, step)).strong());
            });
            progress_bar(ui, *percent, name);
            ui.add_space(8.0);
        }
        OperationStatus::Success(msg) => {
            success_box(ui, msg);
            ui.add_space(4.0);
        }
        OperationStatus::Failed(msg) => {
            error_box(ui, msg);
            ui.add_space(4.0);
        }
        OperationStatus::Idle => {}
    }
    
    // Operations are organized in groups
    ui.columns(2, |cols| {
        // LEFT COLUMN
        let ui = &mut cols[0];
        
        // ---- BASIC OPERATIONS ----
        section_header(ui, "🔧 Basic Operations");
        
        if supported.get_info {
            if op_button(ui, "Get Device Info", "📋") {
                let _ = op_tx.send(OperationRequest::GetInfo {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        if supported.factory_reset {
            if op_button(ui, "Factory Reset", "🔄") {
                let _ = op_tx.send(OperationRequest::FactoryReset {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        if supported.network_factory_reset {
            if op_button(ui, "Network Factory Reset", "📡") {
                let _ = op_tx.send(OperationRequest::SamsungNetworkFactoryReset {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        ui.add_space(8.0);
        
        // ---- UNLOCK OPERATIONS ----
        section_header(ui, "🔓 Unlock Operations");
        
        if supported.frp_remove {
            if op_button(ui, "Remove FRP Lock", "🔓") {
                let _ = op_tx.send(OperationRequest::RemoveFrp {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        if supported.reset_screenlock {
            if op_button(ui, "Reset Screen Lock", "🔒") {
                let _ = op_tx.send(OperationRequest::ResetScreenLock {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        if supported.reset_reactivation_lock {
            if op_button(ui, "Reset Reactivation Lock", "🔐") {
                let _ = op_tx.send(OperationRequest::SamsungResetReactivationLock {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        if supported.mdm_remove {
            if op_button(ui, "Remove MDM Lock", "🏢") {
                let _ = op_tx.send(OperationRequest::RemoveMdm {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        if supported.knox_guard_remove {
            if op_button(ui, "Remove Knox Guard", "🔐") {
                let _ = op_tx.send(OperationRequest::SamsungKnoxGuardRemove {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        if supported.remove_lost_mode {
            if op_button(ui, "Remove Lost Mode", "📍") {
                let _ = op_tx.send(OperationRequest::SamsungRemoveLostMode {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        if supported.demo_remove {
            if op_button(ui, "Remove Demo Mode", "🏪") {
                let _ = op_tx.send(OperationRequest::RemoveDemoMode {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        if supported.remove_warnings {
            if op_button(ui, "Remove Warning Logos", "⚠️") {
                let _ = op_tx.send(OperationRequest::SamsungRemoveWarnings {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        if supported.remove_huawei_id {
            if op_button(ui, "Disable Huawei ID", "🔴") {
                let _ = op_tx.send(OperationRequest::HuaweiDisableHuaweiId {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                });
            }
        }
        
        // RIGHT COLUMN
        let ui = &mut cols[1];
        
        // ---- IMEI OPERATIONS ----
        section_header(ui, "📋 IMEI Repair");
        
        // Get device IMEI input fields from state
        let current_imei = {
            state.devices.get(device_id)
                .map(|d| d.imei_input.clone())
                .unwrap_or_default()
        };
        
        let mut imei_val = current_imei.clone();
        ui.horizontal(|ui| {
            ui.label("IMEI 1:");
            if ui.text_edit_singleline(&mut imei_val).changed() {
                if let Some(d) = state.devices.get_mut(device_id) {
                    d.imei_input = imei_val.clone();
                }
            }
        });
        
        let current_imei2 = {
            state.devices.get(device_id)
                .map(|d| d.imei2_input.clone())
                .unwrap_or_default()
        };
        let mut imei2_val = current_imei2.clone();
        ui.horizontal(|ui| {
            ui.label("IMEI 2:");
            if ui.text_edit_singleline(&mut imei2_val).changed() {
                if let Some(d) = state.devices.get_mut(device_id) {
                    d.imei2_input = imei2_val.clone();
                }
            }
        });
        
        if supported.repair_imei {
            if op_button(ui, "Repair IMEI", "🔧") {
                let _ = op_tx.send(OperationRequest::RepairImei {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                    imei1: current_imei.clone(),
                    imei2: if current_imei2.is_empty() { None } else { Some(current_imei2.clone()) },
                });
            }
        }
        
        if supported.repair_imei_patch {
            if op_button(ui, "Repair IMEI (Patch)", "🔧") {
                let _ = op_tx.send(OperationRequest::RepairImeiPatch {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                    imei1: current_imei,
                    imei2: if current_imei2.is_empty() { None } else { Some(current_imei2) },
                });
            }
        }
        
        ui.add_space(8.0);
        
        // ---- SAMSUNG CSC ----
        if supported.csc_change {
            section_header(ui, "🗺️ CSC / Region");
            
            let current_csc = {
                state.devices.get(device_id)
                    .map(|d| d.csc_input.clone())
                    .unwrap_or_default()
            };
            let mut csc_val = current_csc.clone();
            
            ui.horizontal(|ui| {
                ui.label("New CSC:");
                if ui.text_edit_singleline(&mut csc_val).changed() {
                    if let Some(d) = state.devices.get_mut(device_id) {
                        d.csc_input = csc_val.clone();
                    }
                }
            });
            
            // Common CSC values
            ui.horizontal_wrapped(|ui| {
                for csc in &["OXM", "DBT", "EUR", "XXV", "BTU", "CHC", "AUS", "SIN"] {
                    if ui.small_button(*csc).clicked() {
                        if let Some(d) = state.devices.get_mut(device_id) {
                            d.csc_input = csc.to_string();
                        }
                    }
                }
            });
            
            if op_button(ui, "Change CSC", "🗺️") {
                let _ = op_tx.send(OperationRequest::SamsungCscChange {
                    device_id: device_id.to_string(),
                    serial: serial.clone(),
                    new_csc: current_csc,
                });
            }
            
            ui.add_space(8.0);
        }
        
        // ---- BACKUP/RESTORE ----
        if supported.restore_store_backup {
            section_header(ui, "💾 Backup / Restore");
            
            let current_backup = {
                state.devices.get(device_id)
                    .map(|d| d.backup_path.clone())
                    .unwrap_or_default()
            };
            let mut bp_val = current_backup.clone();
            
            ui.horizontal(|ui| {
                ui.label("Backup path:");
                ui.text_edit_singleline(&mut bp_val);
                if ui.small_button("📁").clicked() {
                    if let Some(p) = rfd::FileDialog::new().save_file() {
                        bp_val = p.to_string_lossy().to_string();
                    }
                }
            });
            
            if bp_val != current_backup {
                if let Some(d) = state.devices.get_mut(device_id) {
                    d.backup_path = bp_val.clone();
                }
            }
            
            ui.horizontal(|ui| {
                if op_button(ui, "Store Backup", "💾") {
                    let _ = op_tx.send(OperationRequest::SamsungStoreBackup {
                        device_id: device_id.to_string(),
                        serial: serial.clone(),
                        path: current_backup.clone(),
                    });
                }
                if op_button(ui, "Restore Backup", "📤") {
                    let _ = op_tx.send(OperationRequest::SamsungRestoreBackup {
                        device_id: device_id.to_string(),
                        serial: serial.clone(),
                        path: current_backup,
                    });
                }
            });
            
            ui.add_space(8.0);
        }
        
        // ---- BOOTLOADER ----
        if supported.bootloader_unlock || supported.bootloader_relock {
            section_header(ui, "🔒 Bootloader");
            
            warning_box(ui, "Unlocking bootloader will wipe all data!");
            ui.add_space(4.0);
            
            ui.horizontal(|ui| {
                if supported.bootloader_unlock {
                    if op_button(ui, "Unlock Bootloader", "🔓") {
                        let _ = op_tx.send(OperationRequest::UnlockBootloader {
                            device_id: device_id.to_string(),
                            serial: serial.clone(),
                        });
                    }
                }
                if supported.bootloader_relock {
                    if op_button(ui, "Relock Bootloader", "🔒") {
                        let _ = op_tx.send(OperationRequest::LockBootloader {
                            device_id: device_id.to_string(),
                            serial: serial.clone(),
                        });
                    }
                }
            });
            
            ui.add_space(8.0);
        }
        
        if twrp_section_visible(&device.brand, &device.connection_mode, &supported) {
            section_header(ui, "🩹 Custom Recovery / TWRP");
            ui.label(RichText::new(format!("Route: {}", twrp_route_label(&device.brand, &device.connection_mode))).strong());

            let current_twrp_image = {
                state.devices.get(device_id)
                    .map(|d| d.twrp_image_path.clone())
                    .unwrap_or_default()
            };
            let mut twrp_image = current_twrp_image.clone();

            ui.horizontal(|ui| {
                ui.label("Recovery image:");
                if ui.text_edit_singleline(&mut twrp_image).changed() {
                    if let Some(d) = state.devices.get_mut(device_id) {
                        d.twrp_image_path = twrp_image.clone();
                    }
                }
                if ui.small_button("📁").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("Recovery image", &["img"])
                        .pick_file()
                    {
                        if let Some(d) = state.devices.get_mut(device_id) {
                            d.twrp_image_path = p.to_string_lossy().to_string();
                        }
                    }
                }
            });

            warning_box(ui, "TWRP/custom recovery requires an unlocked bootloader and a device/ROM that supports custom recovery images.");
            ui.add_space(4.0);

            let run_ready = twrp_ready_to_run(&device.brand, &device.connection_mode)
                && !current_twrp_image.trim().is_empty();

            if matches!(device.connection_mode, ConnectionMode::Adb) {
                ui.horizontal(|ui| {
                    if op_button(ui, "Reboot to Bootloader", "🔁") {
                        let _ = op_tx.send(OperationRequest::RebootDevice {
                            device_id: device_id.to_string(),
                            serial: serial.clone(),
                            mode: Some("bootloader".to_string()),
                        });
                    }
                    ui.label(RichText::new("Then reconnect in Fastboot mode to boot/flash TWRP.").italics());
                });
            }

            ui.horizontal(|ui| {
                if ui.add_enabled(run_ready, egui::Button::new(RichText::new("🚀 Boot TWRP (temp)").strong())).clicked() {
                    let image_path = state.devices.get(device_id)
                        .map(|d| d.twrp_image_path.clone())
                        .unwrap_or_default();
                    let _ = op_tx.send(OperationRequest::BootRecoveryImage {
                        device_id: device_id.to_string(),
                        serial: serial.clone(),
                        brand: device.brand.clone(),
                        connection_mode: device.connection_mode.clone(),
                        model: device.model.clone(),
                        recovery_image_path: image_path,
                    });
                }
                if ui.add_enabled(run_ready, egui::Button::new(RichText::new("🩹 Flash TWRP Recovery").strong())).clicked() {
                    let image_path = state.devices.get(device_id)
                        .map(|d| d.twrp_image_path.clone())
                        .unwrap_or_default();
                    let _ = op_tx.send(OperationRequest::FlashRecoveryImage {
                        device_id: device_id.to_string(),
                        serial: serial.clone(),
                        brand: device.brand.clone(),
                        connection_mode: device.connection_mode.clone(),
                        model: device.model.clone(),
                        recovery_image_path: image_path,
                    });
                }
            });

            if !run_ready {
                ui.label(RichText::new("TWRP actions are enabled when the device is in Fastboot mode and a .img recovery file has been selected.").italics());
            }

            ui.add_space(8.0);
        }

        // ---- ROOT ----
        if supported.root || supported.magisk_root {
            section_header(ui, "🌲 Root");
            
            if supported.magisk_root {
                ui.horizontal(|ui| {
                    ui.label("Magisk APK:");
                    // Bind magisk_apk_path to the device's UI state
                    if let Some(dev_state) = state.devices.get_mut(device_id) {
                        ui.text_edit_singleline(&mut dev_state.magisk_apk_path);
                        if ui.small_button("📁").clicked() {
                            if let Some(p) = rfd::FileDialog::new()
                                .add_filter("APK", &["apk"])
                                .pick_file()
                            {
                                dev_state.magisk_apk_path = p.to_string_lossy().to_string();
                            }
                        }
                    }
                });
                
                if op_button(ui, "Root with Magisk", "🌲") {
                    let magisk_path = state.devices.get(device_id)
                        .map(|d| d.magisk_apk_path.clone())
                        .unwrap_or_default();
                    let _ = op_tx.send(OperationRequest::MagiskRoot {
                        device_id: device_id.to_string(),
                        serial: serial.clone(),
                        magisk_path,
                    });
                }
            }
            
            ui.add_space(8.0);
        }
        
        // ---- MISC ----
        section_header(ui, "🔁 Reboot");
        ui.horizontal_wrapped(|ui| {
            for mode in &[("System", None), ("Bootloader", Some("bootloader")), ("Recovery", Some("recovery")), ("Download", Some("download"))] {
                if ui.button(mode.0).clicked() {
                    let _ = op_tx.send(OperationRequest::RebootDevice {
                        device_id: device_id.to_string(),
                        serial: serial.clone(),
                        mode: mode.1.map(String::from),
                    });
                }
            }
        });
    });
}
