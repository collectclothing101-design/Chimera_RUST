// chimera-gui/src/ui/history_panel.rs
// Operation history / audit log panel
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui::{self, RichText, Color32};
use crate::state::{AppState};
use crate::ui::common::*;
use chimera_core::session::RecordStatus;
use crossbeam_channel::Sender;
use crate::worker::OperationRequest;

pub fn render_history(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
    ui.horizontal(|ui| {
        section_header(ui, "📋 Operation History");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🗑 Clear").clicked() {
                state.session.clear_history();
                state.add_log(crate::state::LogEntry::info("Operation history cleared."));
            }
            if ui.button("💾 Export CSV").clicked() {
                let csv = state.session.export_csv();
                let _path = if let Some(p) = rfd::FileDialog::new()
                    .add_filter("CSV", &["csv"])
                    .set_file_name("chimera_history.csv")
                    .save_file() {
                        std::fs::write(&p, &csv).ok();
                        state.add_log(crate::state::LogEntry::success(
                            format!("History exported to: {}", p.display())
                        ));
                    };
            }
        });
    });

    ui.add_space(4.0);

    // Filter bar
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(&mut state.history_filter);
        if ui.button("✖").clicked() {
            state.history_filter.clear();
        }
    });

    ui.add_space(4.0);
    ui.separator();

    let filter = state.history_filter.to_lowercase();

    // Table header
    egui::Grid::new("history_header")
        .num_columns(6)
        .min_col_width(60.0)
        .show(ui, |ui| {
            ui.label(RichText::new("#").strong());
            ui.label(RichText::new("Time").strong());
            ui.label(RichText::new("Device").strong());
            ui.label(RichText::new("Serial").strong());
            ui.label(RichText::new("Operation").strong());
            ui.label(RichText::new("Status").strong());
            ui.end_row();
        });
    ui.separator();

    egui::ScrollArea::vertical()
        .max_height(400.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let history: Vec<_> = state.session.history()
                .iter()
                .rev()
                .filter(|r| {
                    if filter.is_empty() { return true; }
                    r.device_model.to_lowercase().contains(&filter)
                        || r.operation.to_lowercase().contains(&filter)
                        || r.device_serial.as_deref().unwrap_or("").to_lowercase().contains(&filter)
                })
                .cloned()
                .collect();

            if history.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("No operation records found.").italics().weak());
                });
                return;
            }

            egui::Grid::new("history_table")
                .num_columns(6)
                .striped(true)
                .min_col_width(60.0)
                .show(ui, |ui| {
                    for record in &history {
                        ui.label(format!("{}", record.id));
                        ui.label(record.timestamp.format("%m/%d %H:%M:%S").to_string());
                        ui.label(&record.device_model);
                        ui.label(record.device_serial.as_deref().unwrap_or("-"));
                        ui.label(&record.operation);

                        let (status_text, color) = match &record.status {
                            RecordStatus::Success    => ("✅ OK",    Color32::from_rgb(76, 175, 80)),
                            RecordStatus::Failed     => ("❌ Fail",  Color32::from_rgb(244, 67, 54)),
                            RecordStatus::Cancelled  => ("⚠️ Cancel", Color32::from_rgb(255, 193, 7)),
                            RecordStatus::InProgress => ("⏳ ...",   Color32::from_rgb(33, 150, 243)),
                        };
                        ui.colored_label(color, status_text);
                        ui.end_row();

                        // Show details if available
                        if let Some(details) = &record.details {
                            if !details.is_empty() {
                                ui.label("");
                                ui.label(RichText::new(format!("  ↳ {}", details)).weak().small());
                                ui.end_row();
                            }
                        }
                    }
                });
        });

    ui.add_space(8.0);

    // Device history section
    ui.separator();
    section_header(ui, "📱 Device History");
    ui.add_space(4.0);

    egui::Grid::new("device_history")
        .num_columns(5)
        .striped(true)
        .min_col_width(60.0)
        .show(ui, |ui| {
            ui.label(RichText::new("Brand").strong());
            ui.label(RichText::new("Model").strong());
            ui.label(RichText::new("Serial").strong());
            ui.label(RichText::new("Last Seen").strong());
            ui.label(RichText::new("Operations").strong());
            ui.end_row();

            for entry in state.session.device_history() {
                ui.label(&entry.brand);
                ui.label(&entry.model);
                ui.label(&entry.serial);
                ui.label(entry.last_seen.format("%Y-%m-%d %H:%M").to_string());
                ui.label(format!("{}", entry.operation_count));
                ui.end_row();
            }
        });
}
