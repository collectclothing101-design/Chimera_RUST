// chimera-gui/src/ui/about.rs
// About dialog — ChimeraTool-matched dark amber style
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui::{self, RichText, Color32};
use crate::state::AppState;
use crate::theme::ChimeraTheme;
use chimera_core::{VERSION, APP_NAME};

pub fn render_about_dialog(ctx: &egui::Context, state: &mut AppState) {
    let mut open = state.show_about;

    egui::Window::new(RichText::new(format!("About  {}", APP_NAME)).color(ChimeraTheme::ACCENT).strong())
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .min_width(420.0)
        .frame(
            egui::Frame::window(&ctx.global_style())
                .fill(ChimeraTheme::BG_CARD)
                .stroke(egui::Stroke::new(1.5_f32, ChimeraTheme::BORDER_ACCENT))
                .corner_radius(egui::CornerRadius::same(10)),
        )
        .show(ctx, |ui| {
            ui.set_width(400.0);

            // Logo area
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.colored_label(ChimeraTheme::ACCENT, RichText::new("⚡").size(48.0));
                ui.add_space(4.0);
                ui.colored_label(
                    ChimeraTheme::ACCENT,
                    RichText::new(APP_NAME).strong().size(22.0),
                );
                ui.colored_label(
                    ChimeraTheme::TEXT_SECONDARY,
                    RichText::new(format!("Version {}", VERSION)).size(13.0),
                );
            });
            ui.add_space(12.0);

            // Divider
            ui.painter().hline(
                ui.available_rect_before_wrap().x_range(),
                ui.cursor().top(),
                egui::Stroke::new(1.0_f32, ChimeraTheme::BORDER),
            );
            ui.add_space(10.0);

            // Info rows
            let frame = egui::Frame::NONE
                .fill(ChimeraTheme::BG_ELEVATED)
                .inner_margin(egui::Margin::same(10))
                .corner_radius(egui::CornerRadius::same(6))
                .stroke(egui::Stroke::new(1.0_f32, ChimeraTheme::BORDER));

            frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                let rows = [
                    ("Purpose",   "iOS restore / downgrade research tool"),
                    ("Platform",  "Windows 10/11  (Rust + egui)"),
                    ("Backend",   "chimera-apple, chimera-adb, chimera-core"),
                    ("iCloud",    "198 endpoints catalogued (27 live-probed)"),
                    ("iOS range", "iOS 15 – 18.x  (A11+ devices)"),
                    ("Unlock",    "AU carrier unlock via Telstra/Optus/Vodafone/TPG"),
                    ("Legal",     "Research/educational use only"),
                ];
                for (k, v) in &rows {
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            ChimeraTheme::TEXT_SECONDARY,
                            RichText::new(format!("{:<12}", k)).monospace().size(12.0),
                        );
                        ui.colored_label(
                            ChimeraTheme::TEXT_PRIMARY,
                            RichText::new(*v).size(12.5),
                        );
                    });
                }
            });

            ui.add_space(10.0);

            // Legal disclaimer
            let legal = egui::Frame::NONE
                .fill(Color32::from_rgb(0x18, 0x14, 0x08))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .corner_radius(egui::CornerRadius::same(6))
                .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(0x60, 0x50, 0x10)));
            legal.show(ui, |ui| {
                ui.colored_label(
                    Color32::from_rgb(0xcc, 0xaa, 0x44),
                    RichText::new(
                        "⚖  This is a research tool. Bypassing activation or carrier \
                         locks may violate AU Criminal Code §478 or US CFAA 18 U.S.C. §1030. \
                         Use only on devices you own or have explicit written permission to service.",
                    )
                    .size(11.0),
                );
            });

            ui.add_space(10.0);

            // Close button
            ui.vertical_centered(|ui| {
                if ChimeraTheme::accent_button(ui, "  Close  ").clicked() {
                    state.show_about = false;
                }
            });
            ui.add_space(6.0);
        });

    state.show_about = open;
}
