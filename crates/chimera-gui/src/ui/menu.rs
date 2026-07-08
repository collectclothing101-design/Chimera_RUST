#![allow(dead_code)]
// chimera-gui/src/ui/menu.rs
// Top header bar — ChimeraTool style
// ┌──────────────────────────────────────────────────────────────────┐
// │ ⚡ CHIMERA RS  │  File  Tools  Help  │  ... status ...  │  icons │
// └──────────────────────────────────────────────────────────────────┘

use eframe::egui::{self, RichText, Color32};
use crate::state::{AppState, ActiveTab};
use crate::theme::ChimeraTheme;
use crate::worker::OperationRequest;
use crossbeam_channel::Sender;
use chimera_core::VERSION;

pub fn render_header(ui: &mut egui::Ui, state: &mut AppState) {
    ui.set_min_height(42.0);

    ui.horizontal_centered(|ui| {
        // ── Brand logo area ───────────────────────────────────────────────
        ui.add_space(4.0);
        ui.colored_label(ChimeraTheme::ACCENT, RichText::new("⚡").size(18.0).strong());
        ui.add_space(4.0);
        ui.colored_label(
            ChimeraTheme::ACCENT,
            RichText::new("CHIMERA").strong().size(15.0),
        );
        ui.colored_label(
            ChimeraTheme::TEXT_DISABLED,
            RichText::new(format!(" RS  v{}", VERSION)).size(11.0),
        );

        // Separator
        ui.separator();

        // ── Menu items ────────────────────────────────────────────────────
        egui::MenuBar::new().ui(ui, |ui| {
            // Override menu bar bg (it's inside our header frame already)
            ui.visuals_mut().panel_fill = Color32::TRANSPARENT;

            ui.menu_button(
                RichText::new("File").color(ChimeraTheme::TEXT_PRIMARY).size(13.0),
                |ui| {
                    if ui.button("📁  Open Firmware…").clicked() { ui.close(); }
                    if ui.button("💾  Export Log…").clicked()    { ui.close(); }
                    ui.separator();
                    if ui.button("⚙️  Settings").clicked() {
                        state.active_tab = ActiveTab::Settings;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("🚪  Exit").clicked() { std::process::exit(0); }
                },
            );

            ui.menu_button(
                RichText::new("Tools").color(ChimeraTheme::TEXT_PRIMARY).size(13.0),
                |ui| {
                    if ui.button("🔍  Scan Devices").clicked() {
                        state.add_log(crate::state::LogEntry::info("Scanning for devices…"));
                        ui.close();
                    }
                    if ui.button("🛠️  Utilities").clicked() {
                        state.active_tab = ActiveTab::Utilities;
                        ui.close();
                    }
                    if ui.button("📦  Firmware Manager").clicked() {
                        state.active_tab = ActiveTab::Firmware;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("📋  IMEI Checker").clicked() {
                        state.active_tab = ActiveTab::Utilities;
                        ui.close();
                    }
                    if ui.button("📡  Network Code Calculator").clicked() {
                        state.active_tab = ActiveTab::Utilities;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("🍎  Apple / iOS Panel").clicked() {
                        state.active_tab = ActiveTab::Apple;
                        ui.close();
                    }
                    if ui.button("🔑  SHSH Blob Manager").clicked() {
                        state.active_tab = ActiveTab::ShshManager;
                        ui.close();
                    }
                    if ui.button("🇦🇺  AU Network Unlock").clicked() {
                        state.active_tab = ActiveTab::AuNetworkUnlock;
                        ui.close();
                    }
                    if ui.button("🌐  iCloud / API Tools").clicked() {
                        state.active_tab = ActiveTab::ApiTools;
                        ui.close();
                    }
                },
            );

            ui.menu_button(
                RichText::new("Help").color(ChimeraTheme::TEXT_PRIMARY).size(13.0),
                |ui| {
                    if ui.button("ℹ️  About ChimeraRS").clicked() {
                        state.show_about = true;
                        ui.close();
                    }
                    if ui.button("📖  Connection Guide").clicked() { ui.close(); }
                    if ui.button("🔗  chimeratool.com").clicked() {
                        // NOTE: Open URL in browser — use 'open' crate if available
                        // For now just close menu (user can visit manually)
                        ui.close();
                    }
                },
            );
        });

        // ── Right-side status cluster ──────────────────────────────────────
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);

            // ── Spinner / operation indicator ────────────────────────────
            let is_running = state.devices.values().any(|d| {
                matches!(&d.operation_status, crate::state::OperationStatus::Running { .. })
            });
            if is_running {
                ui.colored_label(ChimeraTheme::WARNING, "⏳");
                ui.colored_label(
                    ChimeraTheme::WARNING,
                    RichText::new("Working…").size(11.5),
                );
                ui.separator();
            }

            // ── Device count ─────────────────────────────────────────────
            let dev_count = state.devices.len();
            if dev_count > 0 {
                ChimeraTheme::status_dot(ui, ChimeraTheme::SUCCESS);
                ui.colored_label(
                    ChimeraTheme::SUCCESS,
                    RichText::new(format!("{} device(s)", dev_count)).size(11.5),
                );
            } else {
                ChimeraTheme::status_dot(ui, ChimeraTheme::TEXT_DISABLED);
                ui.colored_label(
                    ChimeraTheme::TEXT_DISABLED,
                    RichText::new("No devices").size(11.5),
                );
            }
            ui.separator();

            // ── Legal / Research badge ────────────────────────────────────
            ChimeraTheme::golden_badge(ui, "RESEARCH BUILD");
        });
    });
}

// Keep old name as alias so any stray references still compile
pub fn menu_bar(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
    render_header(ui, state);
}
