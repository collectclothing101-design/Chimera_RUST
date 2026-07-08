// chimera-gui/src/ui/device_info.rs
// Device information tab — ChimeraTool-matched layout
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui::{self, RichText};
use crate::state::{AppState, DeviceUiState, OperationStatus};
use crate::worker::OperationRequest;
use crate::theme::ChimeraTheme;
use crate::ui::common::*;
use chimera_core::device::{BootloaderStatus, DeviceState};
use crossbeam_channel::Sender;

pub fn render_device_info(
    ui:        &mut egui::Ui,
    state:     &mut AppState,
    device_id: &str,
    op_tx:     &Sender<OperationRequest>,
) {
    let device_ui = match state.devices.get(device_id) {
        Some(d) => d.clone(),
        None => {
            ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "Device not found in state.");
            return;
        }
    };
    render_device_info_inner(ui, &device_ui, op_tx);
}

fn render_device_info_inner(
    ui:          &mut egui::Ui,
    device_state: &DeviceUiState,
    op_tx:        &Sender<OperationRequest>,
) {
    let device      = &device_state.device;
    let brand_color = ChimeraTheme::brand_color(&format!("{:?}", device.brand).to_lowercase());

    // ── Device hero card ──────────────────────────────────────────────────────
    ChimeraTheme::card_frame()
        .stroke(egui::Stroke::new(1.5_f32, brand_color))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // Phone icon in brand colour
                ui.colored_label(brand_color, RichText::new("📱").size(42.0));
                ui.add_space(8.0);

                ui.vertical(|ui| {
                    ui.colored_label(
                        ChimeraTheme::TEXT_HEADING,
                        RichText::new(&device.model).strong().size(18.0),
                    );

                    ui.horizontal(|ui| {
                        status_badge(ui, &format!("{:?}", device.brand), brand_color);

                        let mode_str = format!("{}", device.connection_mode);
                        status_badge(ui, &mode_str, ChimeraTheme::ACCENT);

                        let (state_color, state_str) = match device.state {
                            DeviceState::Authorized   => (ChimeraTheme::SUCCESS, "ADB ✔"),
                            DeviceState::Unauthorized => (ChimeraTheme::WARNING, "ADB ✘"),
                            DeviceState::Bootloader   => (ChimeraTheme::WARNING, "Bootloader"),
                            DeviceState::Recovery     => (ChimeraTheme::INFO,    "Recovery"),
                            _                         => (ChimeraTheme::TEXT_SECONDARY, "Unknown"),
                        };
                        status_badge(ui, state_str, state_color);
                    });

                    if let Some(serial) = &device.serial {
                        ui.horizontal(|ui| {
                            ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "Serial:");
                            ui.colored_label(
                                ChimeraTheme::TEXT_PRIMARY,
                                RichText::new(serial).monospace().size(12.0),
                            );
                            if ui.small_button("📋").on_hover_text("Copy serial").clicked() {
                                ui.output_mut(|o| o.commands.push(egui::OutputCommand::CopyText(serial.clone())));
                            }
                        });
                    }
                });

                // Right: Get Info button
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ChimeraTheme::outline_button(ui, "🔄 Get Info").clicked() {
                        if let Some(serial) = &device.serial {
                            let _ = op_tx.send(OperationRequest::GetInfo {
                                device_id: device.id.clone(),
                                serial:    serial.clone(),
                            });
                        }
                    }
                });
            });
        });

    ui.add_space(10.0);

    // ── Two-column info grid ───────────────────────────────────────────────────
    ui.columns(2, |cols| {
        // Left: Device information
        let ui = &mut cols[0];
        section_header(ui, "📋  Device Information");

        ChimeraTheme::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::Grid::new("device_info_grid")
                .num_columns(2)
                .spacing([8.0, 5.0])
                .show(ui, |ui| {
                    grid_row(ui, "Model",    &device.model);
                    grid_row(ui, "Chipset",  &format!("{:?}", device.chipset));
                    grid_row(ui, "Android",  device.android_version.as_deref().unwrap_or("—"));
                    grid_row(ui, "Build",    device.build_number.as_deref().unwrap_or("—"));
                    grid_row(ui, "Security", device.security_patch.as_deref().unwrap_or("—"));
                    if let Some(sw) = &device.software_version {
                        grid_row(ui, "SW Ver", sw);
                    }
                    if let Some(bb) = &device.baseband_version {
                        grid_row_mono(ui, "Baseband", bb);
                    }
                    if let Some(csc) = &device.csc {
                        grid_row(ui, "CSC", csc);
                    }
                    if let Some(region) = &device.region {
                        grid_row(ui, "Region", region);
                    }
                    if let Some(knox) = &device.knox_version {
                        grid_row(ui, "Knox", knox);
                    }
                    if let Some(mc) = &device.model_code {
                        grid_row_mono(ui, "Model Code", mc);
                    }
                });
        });

        // Right: Identifiers + status
        let ui = &mut cols[1];
        section_header(ui, "🆔  Identifiers & Status");

        ChimeraTheme::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::Grid::new("identifiers_grid")
                .num_columns(2)
                .spacing([8.0, 5.0])
                .show(ui, |ui| {
                    // IMEI 1
                    ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "IMEI 1:");
                    if let Some(imei) = &device.imei {
                        ui.horizontal(|ui| {
                            ui.colored_label(ChimeraTheme::TEXT_PRIMARY, RichText::new(imei).monospace());
                            if ui.small_button("📋").clicked() {
                                ui.output_mut(|o| o.commands.push(egui::OutputCommand::CopyText(imei.clone())));
                            }
                        });
                    } else {
                        ui.colored_label(ChimeraTheme::TEXT_DISABLED, "—");
                    }
                    ui.end_row();

                    // IMEI 2
                    ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "IMEI 2:");
                    if let Some(imei2) = &device.imei2 {
                        ui.horizontal(|ui| {
                            ui.colored_label(ChimeraTheme::TEXT_PRIMARY, RichText::new(imei2).monospace());
                            if ui.small_button("📋").clicked() {
                                ui.output_mut(|o| o.commands.push(egui::OutputCommand::CopyText(imei2.clone())));
                            }
                        });
                    } else {
                        ui.colored_label(ChimeraTheme::TEXT_DISABLED, "—");
                    }
                    ui.end_row();

                    if let Some(mac) = &device.wifi_mac {
                        grid_row_mono(ui, "Wi-Fi MAC", mac);
                    }
                    if let Some(mac) = &device.bt_mac {
                        grid_row_mono(ui, "BT MAC", mac);
                    }

                    // Bootloader
                    ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "Bootloader:");
                    let (bl_color, bl_text) = match device.bootloader_status {
                        Some(BootloaderStatus::Unlocked) => (ChimeraTheme::WARNING, "Unlocked"),
                        Some(BootloaderStatus::Locked)   => (ChimeraTheme::SUCCESS, "Locked"),
                        _                                => (ChimeraTheme::TEXT_DISABLED, "—"),
                    };
                    ui.colored_label(bl_color, bl_text);
                    ui.end_row();

                    // FRP
                    ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "FRP:");
                    match device.frp_enabled {
                        Some(true)  => { ui.colored_label(ChimeraTheme::WARNING, "Enabled"); }
                        Some(false) => { ui.colored_label(ChimeraTheme::SUCCESS, "Disabled"); }
                        None        => { ui.colored_label(ChimeraTheme::TEXT_DISABLED, "—"); }
                    }
                    ui.end_row();

                    // Root
                    ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "Root:");
                    match device.root_status {
                        Some(true)  => { ui.colored_label(ChimeraTheme::WARNING, "Rooted"); }
                        Some(false) => { ui.colored_label(ChimeraTheme::SUCCESS, "Not rooted"); }
                        None        => { ui.colored_label(ChimeraTheme::TEXT_DISABLED, "—"); }
                    }
                    ui.end_row();

                    if let (Some(vid), Some(pid)) = (device.usb_vid, device.usb_pid) {
                        grid_row_mono(ui, "USB ID", &format!("{:04X}:{:04X}", vid, pid));
                    }
                });
        });
    });

    // ── Operation status ──────────────────────────────────────────────────────
    ui.add_space(10.0);
    match &device_state.operation_status {
        OperationStatus::Running { name, percent, step } => {
            ChimeraTheme::card_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.add_space(4.0);
                    ui.colored_label(
                        ChimeraTheme::ACCENT,
                        RichText::new(format!("{}: {}", name, step)).strong(),
                    );
                });
                progress_bar(ui, *percent, name);
            });
        }
        OperationStatus::Success(msg) => { success_box(ui, msg); }
        OperationStatus::Failed(msg)  => { error_box(ui, msg); }
        OperationStatus::Idle         => {}
    }
}

// ─── Grid helpers ─────────────────────────────────────────────────────────────
fn grid_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.colored_label(ChimeraTheme::TEXT_SECONDARY, label);
    ui.colored_label(ChimeraTheme::TEXT_PRIMARY, value);
    ui.end_row();
}

fn grid_row_mono(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.colored_label(ChimeraTheme::TEXT_SECONDARY, label);
    ui.colored_label(ChimeraTheme::TEXT_PRIMARY, RichText::new(value).monospace().size(12.0));
    ui.end_row();
}
