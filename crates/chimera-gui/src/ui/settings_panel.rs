// chimera-gui/src/ui/settings_panel.rs
// Settings panel — ChimeraTool-matched layout
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui;
use crate::state::AppState;
use crate::theme::ChimeraTheme;
use crate::ui::common::*;

pub fn render_settings(ui: &mut egui::Ui, state: &mut AppState) {
    section_header(ui, "⚙️  Application Settings");
    ui.add_space(4.0);

    ui.columns(2, |cols| {
        // ── Left column ───────────────────────────────────────────────────
        let ui = &mut cols[0];

        sub_header(ui, "ADB Settings");
        ChimeraTheme::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::Grid::new("adb_settings")
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "Server Host:");
                    ui.text_edit_singleline(&mut state.settings.adb_server_host);
                    ui.end_row();

                    ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "Server Port:");
                    let mut port_str = state.settings.adb_server_port.to_string();
                    if ui.text_edit_singleline(&mut port_str).changed() {
                        if let Ok(p) = port_str.parse() {
                            state.settings.adb_server_port = p;
                        }
                    }
                    ui.end_row();

                    ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "Scan Interval:");
                    let mut si = state.settings.scan_interval_ms.to_string();
                    if ui.text_edit_singleline(&mut si).changed() {
                        if let Ok(v) = si.parse() {
                            state.settings.scan_interval_ms = v;
                        }
                    }
                    ui.end_row();
                });
        });

        ui.add_space(8.0);
        sub_header(ui, "File Paths");
        ChimeraTheme::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            dir_picker(ui, "Download dir:", &mut state.settings.download_dir);
            ui.add_space(4.0);
            dir_picker(ui, "Backup dir:",   &mut state.settings.backup_dir);
        });

        // ── Right column ──────────────────────────────────────────────────
        let ui = &mut cols[1];

        sub_header(ui, "Appearance");
        ChimeraTheme::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.checkbox(&mut state.settings.dark_mode, "Dark Mode (restart to apply)");
            ui.horizontal(|ui| {
                ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "Font Size:");
                ui.add(egui::Slider::new(&mut state.settings.font_size, 10.0..=20.0).suffix("pt"));
            });
            ui.horizontal(|ui| {
                ui.colored_label(ChimeraTheme::TEXT_SECONDARY, "Max log lines:");
                let mut s = state.settings.max_log_lines.to_string();
                if ui.add(egui::TextEdit::singleline(&mut s).desired_width(60.0)).changed() {
                    if let Ok(v) = s.parse() {
                        state.settings.max_log_lines = v;
                    }
                }
            });
        });

        ui.add_space(8.0);
        sub_header(ui, "Behaviour");
        ChimeraTheme::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.checkbox(&mut state.settings.auto_scan,              "Auto-scan for devices");
            ui.checkbox(&mut state.settings.log_to_file,            "Save log to file");
            ui.checkbox(&mut state.settings.show_developer_options, "Show developer options");
            ui.checkbox(&mut state.settings.confirm_dangerous_ops,  "Confirm dangerous operations");
            ui.checkbox(&mut state.settings.auto_backup_before_ops, "Auto backup before operations");
        });

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            if ChimeraTheme::accent_button(ui, "💾 Save").clicked() {
                state.add_log(crate::state::LogEntry::success("Settings saved."));
            }
            ui.add_space(6.0);
            if ChimeraTheme::outline_button(ui, "🔄 Reset Defaults").clicked() {
                state.settings = crate::state::AppSettings::default();
                state.add_log(crate::state::LogEntry::info("Settings reset to defaults."));
            }
        });
    });
}
