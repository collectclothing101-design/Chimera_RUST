// crates/chimera-gui/src/ui/menu.rs
// Top menu bar — all actions wired (Phase 4)
#![allow(dead_code)]

use eframe::egui::{self, RichText, Color32};
use crate::state::{AppState, ActiveTab};
use crate::theme::ChimeraTheme;
use crate::worker::OperationRequest;
use crate::ui::nav::Page;
use crossbeam_channel::Sender;

pub fn render_menu_bar(
    ui: &mut egui::Ui,
    state: &mut AppState,
    op_tx: &Sender<OperationRequest>,
) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.visuals_mut().panel_fill = Color32::TRANSPARENT;

        // ── File ────────────────────────────────────────────────────────
        ui.menu_button(
            RichText::new("File").color(ChimeraTheme::TEXT_PRIMARY).size(13.0),
            |ui| {
                if ui.button("📁  Open Firmware…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Firmware", &["zip","tar","pac","lz4","bin","img","ipsw","tar.md5"])
                        .pick_file()
                    {
                        let path_str = path.to_string_lossy().to_string();
                        // Set on the selected device, or store globally
                        if let Some(id) = state.selected_device_id.clone() {
                            if let Some(dev) = state.devices.get_mut(&id) {
                                dev.firmware_path = path_str.clone();
                            }
                        }
                        state.add_log(crate::state::LogEntry::info(
                            format!("Firmware selected: {}", path_str)));
                    }
                    ui.close();
                }

                if ui.button("💾  Export Log…").clicked() {
                    let filename = format!(
                        "chimera_log_{}.txt",
                        chrono::Local::now().format("%Y%m%d_%H%M%S")
                    );
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(&filename)
                        .add_filter("Text", &["txt"])
                        .save_file()
                    {
                        let content = state.log_entries.iter()
                            .map(|e| format!("[{}] [{:?}] {}",
                                e.timestamp, e.level, e.message))
                            .collect::<Vec<_>>()
                            .join("\n");
                        match std::fs::write(&path, &content) {
                            Ok(_) => state.add_log(crate::state::LogEntry::success(
                                format!("Log exported: {}", path.display()))),
                            Err(e) => state.add_log(crate::state::LogEntry::error(
                                format!("Export failed: {}", e))),
                        }
                    }
                    ui.close();
                }

                ui.separator();

                if ui.button("⚙️  Settings").clicked() {
                    state.current_page = Page::Settings;
                    ui.close();
                }

                ui.separator();

                if ui.button("🚪  Exit").clicked() {
                    // Graceful: persist settings then close window
                    state.settings_dirty = true;
                    let _ = crate::persistence::save_settings(&state.settings);
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close();
                }
            },
        );

        // ── Tools ───────────────────────────────────────────────────────
        ui.menu_button(
            RichText::new("Tools").color(ChimeraTheme::TEXT_PRIMARY).size(13.0),
            |ui| {
                if ui.button("🔍  Scan Devices").clicked() {
                    state.is_scanning = true;
                    state.add_log(crate::state::LogEntry::info("Quick scan initiated."));
                    let _ = op_tx.send(OperationRequest::QuickScan);
                    ui.close();
                }
                if ui.button("🛠️  Utilities").clicked() {
                    state.current_page = Page::Utilities;
                    ui.close();
                }
                if ui.button("📦  Firmware / Downloads").clicked() {
                    state.current_page = Page::Downloads;
                    ui.close();
                }
                ui.separator();
                if ui.button("📋  IMEI Checker").clicked() {
                    state.current_page = Page::Utilities;
                    ui.close();
                }
                if ui.button("📡  NCK Calculator").clicked() {
                    state.current_page = Page::Utilities;
                    ui.close();
                }
                ui.separator();
                if ui.button("🍎  Apple / iOS").clicked() {
                    state.current_page = Page::AppleIos;
                    ui.close();
                }
                if ui.button("🔐  SHSH Blobs").clicked() {
                    state.current_page = Page::ShshBlobs;
                    ui.close();
                }
                if ui.button("🇦🇺  AU Network Unlock").clicked() {
                    state.current_page = Page::AuUnlock;
                    ui.close();
                }
                if ui.button("🌐  API Tools").clicked() {
                    state.current_page = Page::ApiTools;
                    ui.close();
                }
                ui.separator();
                if ui.button("📺  Event Log").clicked() {
                    state.current_page = Page::EventLog;
                    ui.close();
                }
            },
        );

        // ── Help ────────────────────────────────────────────────────────
        ui.menu_button(
            RichText::new("Help").color(ChimeraTheme::TEXT_PRIMARY).size(13.0),
            |ui| {
                if ui.button("ℹ️  About ChimeraRS").clicked() {
                    state.show_about_modal = true;
                    ui.close();
                }
                if ui.button("📖  Connection Guide").clicked() {
                    // Navigate to Utilities which has the connection guide
                    state.current_page = Page::Utilities;
                    state.utilities_tab = 1; // guide tab
                    ui.close();
                }
                ui.separator();
                if ui.button("🔗  chimeratool.com").clicked() {
                    let _ = webbrowser::open("https://chimeratool.com");
                    ui.close();
                }
                if ui.button("📄  Open Settings Folder").clicked() {
                    let dir = crate::persistence::data_dir();
                    let _ = std::process::Command::new("open")
                        .arg(dir.to_string_lossy().as_ref())
                        .spawn();
                    ui.close();
                }
            },
        );
    });
}
