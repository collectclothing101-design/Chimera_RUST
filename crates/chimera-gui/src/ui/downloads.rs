// crates/chimera-gui/src/ui/downloads.rs
// Downloads page — matches HTML pg-dld
// Tabs: Queue · IPSW Finder · Samsung Firmware · Completed
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use egui::RichText;
use crate::theme::ChimeraTheme;
use crate::state::AppState;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum DownloadsTab { #[default] Queue, IpswFinder, SamsungFirmware, Completed }

pub fn render_downloads(ui: &mut egui::Ui, state: &mut AppState) {
    crate::app::page_header(ui, "03 · WORKSPACE", "Downloads",
        "Firmware, IPSW and resource queue management");

    // Action button
    ui.horizontal(|ui| {
        crate::app::btn_p(ui, "+ New Download");
    });
    ui.add_space(10.0);

    // Tabs
    ui.horizontal(|ui| {
        for (tab, label) in [
            (DownloadsTab::Queue,           "Queue"),
            (DownloadsTab::IpswFinder,      "IPSW Finder"),
            (DownloadsTab::SamsungFirmware, "Samsung Firmware"),
            (DownloadsTab::Completed,       "Completed"),
        ] {
            let active = state.downloads_tab == tab;
            if ui.selectable_label(active, label.to_uppercase()).clicked() {
                state.downloads_tab = tab;
            }
        }
    });
    ui.separator();
    ui.add_space(12.0);

    match state.downloads_tab {
        DownloadsTab::Queue => {
            crate::app::empty_state(ui, "↓", "Download Queue Empty",
                "Use IPSW Finder or Samsung Firmware to queue firmware downloads.");
        }
        DownloadsTab::IpswFinder => {
            ui.set_max_width(560.0);
            crate::app::card_frame().show(ui, |ui| {
                crate::app::section_hd(ui, "IPSW Firmware Search");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("ipsw_model")
                        .selected_text(&state.ipsw_model_selected)
                        .width(ui.available_width() - 100.0)
                        .show_ui(ui, |ui| {
                            for model in IPSW_MODELS {
                                ui.selectable_value(&mut state.ipsw_model_selected,
                                    model.to_string(), *model);
                            }
                        });
                    crate::app::btn_p(ui, "Search");
                });
                ui.add_space(4.0);
                crate::app::note_i(ui, "Results from ipsw.me API. Only signed builds can be restored via Apple TSS.");
            });
        }
        DownloadsTab::SamsungFirmware => {
            ui.set_max_width(560.0);
            crate::app::card_frame().show(ui, |ui| {
                crate::app::section_hd(ui, "Samsung Firmware Lookup");
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.samsung_fw_model)
                        .desired_width(ui.available_width() - 140.0)
                        .hint_text("Model number — e.g. SM-S928B"));
                    ui.add(egui::TextEdit::singleline(&mut state.samsung_fw_csc)
                        .desired_width(80.0).hint_text("CSC"));
                    crate::app::btn_p(ui, "Fetch");
                });
                ui.add_space(4.0);
                ui.label(RichText::new("Queries samfw.com — falls back to portal link when direct unavailable.")
                    .size(9.0).color(ChimeraTheme::T2));
            });
        }
        DownloadsTab::Completed => {
            crate::app::empty_state(ui, "✓", "No Completed Downloads",
                "Completed firmware downloads appear here with SHA-1 and MD5 verification.");
        }
    }
}

const IPSW_MODELS: &[&str] = &[
    "Select a device model…",
    "iPhone 17 Pro Max (iPhone18,5)",
    "iPhone 17 Pro (iPhone18,4)",
    "iPhone 17 Air (iPhone18,2)",
    "iPhone 16 Pro Max (iPhone17,2)",
    "iPhone 16 Pro (iPhone17,1)",
    "iPhone 16 (iPhone17,3)",
    "iPhone 16e (iPhone17,5)",
    "iPhone 15 Pro Max (iPhone16,2)",
    "iPhone 15 (iPhone15,4)",
];

// State additions:
// pub downloads_tab:          DownloadsTab,
// pub ipsw_model_selected:    String,
// pub samsung_fw_model:       String,
// pub samsung_fw_csc:         String,
