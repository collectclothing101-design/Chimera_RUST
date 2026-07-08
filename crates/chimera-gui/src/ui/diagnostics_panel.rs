// chimera-gui/src/ui/diagnostics_panel.rs
// Diagnostics tab — battery, RAM, storage, thermals, CPU, network overview
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use chimera_core::diagnostics::{BatteryHealth, ChargingStatus};
use eframe::egui::{self, RichText, Color32, ProgressBar};
use crate::state::{AppState, OperationStatus};
use crate::worker::OperationRequest;
use crate::ui::common::*;
use crossbeam_channel::Sender;
pub fn render_diagnostics(
    ui: &mut egui::Ui,
    state: &mut AppState,
    device_id: &str,
    op_tx: &Sender<OperationRequest>,
) {
    let device_state = match state.devices.get(device_id) {
        Some(d) => d.clone(),
        None => return,
    };

    let serial = device_state.device.serial.clone().unwrap_or_default();

    // Collect button
    ui.horizontal(|ui| {
        if ui.add_sized([160.0, 30.0], egui::Button::new(
            RichText::new("🔍 Collect Diagnostics").strong()
        )).clicked() {
            let _ = op_tx.send(OperationRequest::CollectDiagnostics {
                device_id: device_id.to_string(),
                serial: serial.clone(),
            });
        }
        if let OperationStatus::Running { name, percent: _, step } = &device_state.operation_status {
            if name.contains("Diagnostics") {
                ui.spinner();
                ui.label(step);
            }
        }
    });

    ui.add_space(8.0);

    let diag = match &device_state.diagnostics {
        Some(d) => d.clone(),
        None => {
            ui.label(RichText::new("No diagnostics collected yet. Click 'Collect Diagnostics'.").italics());
            return;
        }
    };

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.columns(2, |cols| {
            // LEFT COLUMN
            let ui = &mut cols[0];

            // --- BATTERY ---
            section_header(ui, "🔋 Battery");
            if let Some(bat) = &diag.battery {
                if let Some(lvl) = bat.level_percent {
                    let color = if lvl > 50 { Color32::from_rgb(76, 175, 80) }
                                else if lvl > 20 { Color32::from_rgb(255, 193, 7) }
                                else { Color32::from_rgb(244, 67, 54) };
                    ui.horizontal(|ui| {
                        ui.label("Level:");
                        ui.colored_label(color, format!("{}%", lvl));
                        ui.add(ProgressBar::new(lvl as f32 / 100.0).desired_width(80.0));
                    });
                }
                if let Some(health) = &bat.health {
                    let color = if *health == BatteryHealth::Good {
                        Color32::from_rgb(76, 175, 80)
                    } else {
                        Color32::from_rgb(244, 67, 54)
                    };
                    ui.horizontal(|ui| {
                        ui.label("Health:");
                        ui.colored_label(color, format!("{}", health));
                    });
                }
                if let Some(temp) = bat.temperature_c {
                    let color = if temp < 35.0 { Color32::from_rgb(76, 175, 80) }
                                else if temp < 45.0 { Color32::from_rgb(255, 193, 7) }
                                else { Color32::from_rgb(244, 67, 54) };
                    ui.horizontal(|ui| {
                        ui.label("Temperature:");
                        ui.colored_label(color, format!("{:.1}°C", temp));
                    });
                }
                if let Some(mv) = bat.voltage_mv {
                    ui.label(format!("Voltage: {} mV", mv));
                }
                if let Some(tech) = &bat.technology {
                    ui.label(format!("Technology: {}", tech));
                }
                if let Some(status) = &bat.status {
                    let status_str = match status {
                        ChargingStatus::Charging    => "⚡ Charging",
                        ChargingStatus::Discharging => "🔋 Discharging",
                        ChargingStatus::Full        => "✅ Full",
                        ChargingStatus::NotCharging => "⏸ Not Charging",
                        ChargingStatus::Unknown     => "❓ Unknown",
                    };
                    ui.label(format!("Status: {}", status_str));
                }
            } else {
                ui.label(RichText::new("Battery info unavailable").weak());
            }

            ui.add_space(12.0);

            // --- RAM ---
            section_header(ui, "💾 Memory (RAM)");
            if let Some(ram) = &diag.ram {
                if let (Some(total), Some(used)) = (ram.total_mb, ram.used_mb) {
                    let pct = used as f32 / total as f32;
                    let _color = if pct < 0.7 { Color32::from_rgb(76, 175, 80) }
                                else if pct < 0.9 { Color32::from_rgb(255, 193, 7) }
                                else { Color32::from_rgb(244, 67, 54) };
                    ui.horizontal(|ui| {
                        ui.label(format!("Used: {} / {} MB", used, total));
                        ui.add(ProgressBar::new(pct).desired_width(80.0));
                    });
                }
                if let Some(avail) = ram.available_mb {
                    ui.label(format!("Available: {} MB", avail));
                }
            } else {
                ui.label(RichText::new("RAM info unavailable").weak());
            }

            ui.add_space(12.0);

            // --- CPU ---
            section_header(ui, "⚙️ CPU");
            if let Some(cores) = diag.cpu_cores {
                ui.label(format!("Cores: {}", cores));
            }
            if let Some(freq) = diag.cpu_freq_mhz {
                ui.label(format!("Frequency: {} MHz", freq));
            }
            if let Some(up) = diag.uptime_seconds {
                let h = up / 3600;
                let m = (up % 3600) / 60;
                let s = up % 60;
                ui.label(format!("Uptime: {}h {}m {}s", h, m, s));
            }
            if let Some(kver) = &diag.kernel_version {
                ui.label(format!("Kernel: {}", kver));
            }

            // RIGHT COLUMN
            let ui = &mut cols[1];

            // --- STORAGE ---
            section_header(ui, "📦 Storage");
            if let Some(storage) = &diag.storage {
                if let (Some(total), Some(used)) = (storage.total_internal_mb, storage.used_internal_mb) {
                    let pct = used as f32 / total as f32;
                    let _color = if pct < 0.7 { Color32::from_rgb(76, 175, 80) }
                                else if pct < 0.9 { Color32::from_rgb(255, 193, 7) }
                                else { Color32::from_rgb(244, 67, 54) };
                    ui.horizontal(|ui| {
                        ui.label(format!("Internal: {} / {} MB", used, total));
                        ui.add(ProgressBar::new(pct).desired_width(80.0));
                    });
                }
                if let (Some(total), Some(used)) = (storage.total_sdcard_mb, storage.used_sdcard_mb) {
                    ui.label(format!("SD Card: {} / {} MB", used, total));
                }
                if !storage.partitions.is_empty() {
                    ui.add_space(4.0);
                    ui.label("Partitions:");
                    egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                        for p in &storage.partitions {
                            ui.label(format!("   → {} {} ({} MB free)", p.name, p.mount_point, p.free_mb));
                        }
                    });
                }
            } else {
                ui.label(RichText::new("Storage info unavailable").weak());
            }

            ui.add_space(12.0);

            // --- THERMALS ---
            section_header(ui, "🌡️ Thermals");
            if let Some(thermal) = &diag.thermal {
                if thermal.zones.is_empty() {
                    ui.label(RichText::new("No thermal zones found").weak());
                } else {
                    egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                        for zone in &thermal.zones {
                            let color = if zone.temperature_c < 40.0 { Color32::from_rgb(76, 175, 80) }
                                        else if zone.temperature_c < 60.0 { Color32::from_rgb(255, 193, 7) }
                                        else { Color32::from_rgb(244, 67, 54) };
                            ui.horizontal(|ui| {
                                ui.label(format!(": {}", zone.name));
                                ui.colored_label(color, format!("{:.1}°C", zone.temperature_c));
                            });
                        }
                    });
                }
            } else {
                ui.label(RichText::new("Thermal info unavailable").weak());
            }

            ui.add_space(12.0);

            // --- NETWORK ---
            section_header(ui, "📡 Network");
            if let Some(net) = &diag.network {
                let wifi_str = if net.wifi_enabled { "✅ Enabled" } else { "❌ Disabled" };
                ui.label(format!("Wi-Fi: {}", wifi_str));
                if let Some(ssid) = &net.wifi_ssid {
                    ui.label(format!("SSID: {}", ssid));
                }
                let airplane = if net.airplane_mode { "✈️ Airplane Mode ON" } else { "📶 Normal" };
                ui.label(airplane);
                if let Some(op) = &net.operator {
                    ui.label(format!("Operator: {}", op));
                }
                if let Some(mcc) = &net.mcc_mnc {
                    ui.label(format!("MCC/MNC: {}", mcc));
                }
            } else {
                ui.label(RichText::new("Network info unavailable").weak());
            }
        });
    });
}
