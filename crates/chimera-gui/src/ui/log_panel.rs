// chimera-gui/src/ui/log_panel.rs
// Bottom console log — ChimeraTool dark style with amber highlights
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui::{self, RichText, Color32};
use crate::state::{AppState, LogEntry};
use crate::theme::ChimeraTheme;
use chimera_core::event::LogLevel;

pub fn render_log(ui: &mut egui::Ui, state: &mut AppState) {
    // Console header strip
    let header_frame = egui::Frame::NONE
        .fill(Color32::from_rgb(0x0e, 0x10, 0x18))
        .inner_margin(egui::Margin::symmetric(10, 4));

    header_frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            // Console icon + title
            ui.colored_label(ChimeraTheme::ACCENT, "▶");
            ui.add_space(4.0);
            ui.colored_label(
                ChimeraTheme::TEXT_SECONDARY,
                RichText::new("CONSOLE").strong().size(11.0),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Clear").size(11.0))
                            .fill(Color32::TRANSPARENT)
                            .stroke(egui::Stroke::new(1.0_f32, ChimeraTheme::BORDER)),
                    )
                    .clicked()
                {
                    state.log_entries.clear();
                }

                ui.add_space(4.0);

                // Log entry count
                ui.colored_label(
                    ChimeraTheme::TEXT_DISABLED,
                    RichText::new(format!("{} entries", state.log_entries.len())).size(10.5),
                );
            });
        });
    });

    // Thin amber separator line
    ui.painter().hline(
        ui.clip_rect().x_range(),
        ui.cursor().top(),
        egui::Stroke::new(1.0_f32, ChimeraTheme::BORDER_SUBTLE),
    );

    // Log scroll area
    let log_frame = egui::Frame::NONE
        .fill(Color32::from_rgb(0x10, 0x11, 0x1a))
        .inner_margin(egui::Margin::symmetric(8, 4));

    log_frame.show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                // Show last 500 entries
                let entries: Vec<LogEntry> = state.log_entries.iter().rev().take(500).cloned().collect::<Vec<_>>()
                    .into_iter().rev().collect();

                for entry in &entries {
                    render_log_entry(ui, entry);
                }
            });
    });
}

fn render_log_entry(ui: &mut egui::Ui, entry: &LogEntry) {
    let (level_str, level_color) = match entry.level {
        LogLevel::Success => ("OK ", ChimeraTheme::SUCCESS),
        LogLevel::Error   => ("ERR", ChimeraTheme::ERROR),
        LogLevel::Warning => ("WRN", ChimeraTheme::WARNING),
        LogLevel::Info    => ("INF", ChimeraTheme::INFO),
        LogLevel::Debug   => ("DBG", ChimeraTheme::TEXT_DISABLED),
    };

    ui.horizontal(|ui| {
        // Timestamp
        ui.colored_label(
            ChimeraTheme::TEXT_DISABLED,
            RichText::new(format!("{} ", &entry.timestamp)).monospace().size(11.0),
        );

        // Level badge
        ui.colored_label(
            level_color,
            RichText::new(format!("[{}]", level_str)).monospace().size(11.0).strong(),
        );

        ui.add_space(4.0);

        // Message
        let msg_color = match entry.level {
            LogLevel::Error   => ChimeraTheme::ERROR,
            LogLevel::Warning => ChimeraTheme::WARNING,
            LogLevel::Success => ChimeraTheme::SUCCESS,
            LogLevel::Debug   => ChimeraTheme::TEXT_DISABLED,
            _                 => ChimeraTheme::TEXT_PRIMARY,
        };
        ui.colored_label(msg_color, RichText::new(&entry.message).monospace().size(11.5));
    });
}
