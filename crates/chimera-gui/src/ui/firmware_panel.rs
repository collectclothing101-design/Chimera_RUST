// chimera-gui/src/ui/firmware_panel.rs
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
use eframe::egui::{self, RichText};
use crate::state::AppState;
use crate::worker::OperationRequest;
use crate::ui::common::*;
use chimera_core::device::{ConnectionMode, DeviceBrand};
use chimera_devices::database::DeviceDatabase;
use chimera_devices::detector::DeviceDetector;
use crossbeam_channel::Sender;

fn flash_route_label(brand: &DeviceBrand, mode: &ConnectionMode) -> &'static str {
    match (brand, mode) {
        (DeviceBrand::Samsung, ConnectionMode::DownloadOdin | ConnectionMode::SamsungEub) => "Samsung Download Mode → Odin flasher",
        (_, ConnectionMode::Fastboot) => "Fastboot mode → partition image flasher",
        (_, ConnectionMode::Edl) => "Qualcomm EDL mode → rawprogram / firehose flasher",
        (_, ConnectionMode::MtkBootRom) => "MTK BootROM mode → scatter flasher",
        (DeviceBrand::Samsung, ConnectionMode::Adb) => "Samsung ADB mode → reboot to Download mode before flashing",
        (_, ConnectionMode::Adb) => "ADB mode → reboot to the vendor flashing mode first",
        _ => "No supported flashing route for the current device mode",
    }
}

fn flash_button_label(brand: &DeviceBrand, mode: &ConnectionMode) -> &'static str {
    match (brand, mode) {
        (DeviceBrand::Samsung, ConnectionMode::DownloadOdin | ConnectionMode::SamsungEub) => "⚡ Flash via Odin",
        (_, ConnectionMode::Fastboot) => "⚡ Flash via Fastboot",
        (_, ConnectionMode::Edl) => "⚡ Flash via EDL",
        (_, ConnectionMode::MtkBootRom) => "⚡ Flash via MTK",
        _ => "⚡ Flash Firmware",
    }
}

pub fn render_firmware(ui: &mut egui::Ui, state: &mut AppState, op_tx: &Sender<OperationRequest>, device_db: &DeviceDatabase) {
    let selected_snapshot = state.selected_device_id.clone().and_then(|id| {
        state.devices.get(&id).map(|d| {
            (
                id,
                d.device.brand.clone(),
                d.device.connection_mode.clone(),
                d.device.model.clone(),
                d.device.serial.clone().unwrap_or_default(),
                d.firmware_path.clone(),
                DeviceDetector::get_supported_ops(&d.device),
            )
        })
    });

    ui.columns(2, |cols| {
        // LEFT - Flash firmware
        let ui = &mut cols[0];
        section_header(ui, "📦 Flash Firmware");

        ui.label("Firmware file/folder:");
        ui.horizontal(|ui| {
            let fw_path = if let Some(id) = &state.selected_device_id.clone() {
                state.devices.get(id).map(|d| d.firmware_path.clone()).unwrap_or_default()
            } else { String::new() };
            let mut fw = fw_path.clone();
            if ui.text_edit_singleline(&mut fw).changed() {
                if let Some(id) = &state.selected_device_id.clone() {
                    if let Some(d) = state.devices.get_mut(id) {
                        d.firmware_path = fw.clone();
                    }
                }
            }
            if ui.button("📁 File").clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Firmware", &["zip", "tar", "tar.md5", "pac", "lz4", "bin", "img", "xml", "txt"])
                    .pick_file() {
                    if let Some(id) = &state.selected_device_id.clone() {
                        if let Some(d) = state.devices.get_mut(id) {
                            d.firmware_path = p.to_string_lossy().to_string();
                        }
                    }
                }
            }
            if ui.button("📂 Folder").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                    if let Some(id) = &state.selected_device_id.clone() {
                        if let Some(d) = state.devices.get_mut(id) {
                            d.firmware_path = p.to_string_lossy().to_string();
                        }
                    }
                }
            }
        });

        ui.add_space(8.0);
        warning_box(ui, "Flashing wrong firmware may brick your device!");
        ui.add_space(4.0);

        if let Some((_, brand, mode, model, _, fw_path, supported)) = &selected_snapshot {
            ui.label(RichText::new(format!("Route: {}", flash_route_label(brand, mode))).strong());
            ui.label(format!("Selected device: {} | Brand: {:?} | Mode: {}", model, brand, mode));

            if matches!((brand, mode), (&DeviceBrand::Samsung, &ConnectionMode::Adb)) {
                warning_box(ui, "Samsung devices must be in Download / Odin mode before the integrated Odin flasher can run.");
            }

            let can_flash = supported.update_firmware && !fw_path.trim().is_empty();
            if ui.add_enabled(can_flash, egui::Button::new(RichText::new(flash_button_label(brand, mode)).strong()).min_size(egui::vec2(180.0, 36.0))).clicked() {
                if let Some((device_id, brand, mode, model, serial, _fw_path, _)) = selected_snapshot.clone() {
                    if let Some(d) = state.devices.get(&device_id) {
                        let _ = op_tx.send(OperationRequest::FlashFirmware {
                            device_id,
                            serial,
                            brand,
                            connection_mode: mode,
                            model,
                            firmware_path: d.firmware_path.clone(),
                        });
                    }
                }
            }

            if !supported.update_firmware {
                ui.label(RichText::new("Current device mode does not advertise firmware flashing support.").italics());
            }
        } else {
            ui.label(RichText::new("Connect and select a device to enable routed firmware flashing.").italics());
            ui.add_enabled(false, egui::Button::new(RichText::new("⚡ Flash Firmware").strong()).min_size(egui::vec2(180.0, 36.0)));
        }

        ui.add_space(16.0);
        section_header(ui, "📂 Firmware Extractor");
        ui.label("Supports: ZIP, TAR, TAR.MD5, LZ4, PAC");
        ui.add_space(4.0);

        let mut src = String::new();
        let mut dst = String::new();

        file_picker(ui, "Source:", &mut src, "zip,tar,pac,lz4");
        dir_picker(ui, "Destination:", &mut dst);

        if ui.button("📦 Extract").clicked() && !src.is_empty() && !dst.is_empty() {
            let _ = op_tx.send(OperationRequest::ExtractFirmware { source: src, dest: dst });
        }

        // RIGHT - Search / Download
        let ui = &mut cols[1];
        section_header(ui, "🔍 Search Firmware");

        ui.horizontal(|ui| {
            ui.label("Brand:");
            egui::ComboBox::from_id_salt("fw_brand")
                .selected_text(&state.firmware_search_brand)
                .show_ui(ui, |ui| {
                    for brand in &["Samsung", "Xiaomi", "Huawei", "Motorola", "Nokia", "LG", "Sony"] {
                        ui.selectable_value(&mut state.firmware_search_brand, brand.to_string(), *brand);
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("Model:");
            ui.text_edit_singleline(&mut state.firmware_search_model);
        });

        ui.horizontal(|ui| {
            ui.label("Region/CSC:");
            ui.text_edit_singleline(&mut state.firmware_search_region);
        });

        if ui.button("🔍 Search").clicked() {
            state.firmware_search_results.clear();
            state.firmware_search_results.push(format!(
                "{} {} [{}] - Latest available",
                state.firmware_search_brand, state.firmware_search_model, state.firmware_search_region
            ));
        }

        ui.add_space(4.0);
        let _fw_results = state.firmware_search_results.clone();
    for result in &_fw_results {
            ui.horizontal(|ui| {
                ui.label(result);
                if ui.small_button("⬇ Download").clicked() {
                    state.add_log(crate::state::LogEntry::info(format!("Would download: {}", result)));
                }
            });
        }

        ui.add_space(16.0);
        section_header(ui, "📊 Device Database");
        ui.label(format!("Database contains {} device models across {} brands", device_db.count(), device_db.all_brands().len()));
    });
}
