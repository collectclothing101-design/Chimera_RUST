// crates/chimera-gui/src/ui/dashboard.rs
// Dashboard page — exact match to HTML pg-dash
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use egui::{Color32, RichText, Stroke};
use crate::theme::ChimeraTheme;
use crate::state::AppState;

pub fn render_dashboard(ui: &mut egui::Ui, state: &mut AppState) {
    // ── Page header ───────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("01 · WORKSPACE").size(8.0).color(ChimeraTheme::T3));
            ui.label(RichText::new("DASHBOARD").size(18.0).strong().color(ChimeraTheme::T0));
            ui.label(RichText::new("Overview  ·  Connection Mode  ·  Active Workspace")
                .size(9.5).color(ChimeraTheme::T2));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            crate::app::btn_s(ui, "Device Info ↗");
        });
    });
    ui.add_space(13.0);
    ui.separator();
    ui.add_space(14.0);

    // ── Mode cards (.g2) ─────────────────────────────────────────
    ui.columns(2, |cols| {
        // AUTO DETECT (active)
        mode_card(&mut cols[0], "AUTO DETECT",
            "Continuously monitors USB, ADB and service channels for newly connected devices. Recommended for daily repair workflow.",
            true);
        // DEVICE WIZARD
        mode_card(&mut cols[1], "DEVICE WIZARD",
            "Guided step-by-step sequence — device info read, pre-check, repair execution and post-action result validation.",
            false);
    });
    ui.add_space(14.0);

    // ── Stats grid (.g4) ─────────────────────────────────────────
    let col_w = (ui.available_width() - 42.0) / 4.0;
    ui.horizontal(|ui| {
        stat_card(ui, col_w, "Connected Devices",  "0",              "USB · ADB · Fastboot", true);
        stat_card(ui, col_w, "Supported Models",   "65",             "Device database entries", false);
        stat_card(ui, col_w, "Active Workspace",   "Device Info",    "Current service tab", false);
        stat_card(ui, col_w, "Last Operation",     "No jobs yet",    "Recent status", false);
    });
    ui.add_space(16.0);

    // ── Quick Access ─────────────────────────────────────────────
    crate::app::section_hd(ui, "Quick Access");
    ui.columns(3, |cols| {
        qa_card(&mut cols[0], "◫", "Scan Devices",      "USB · ADB · Fastboot · Download mode");
        qa_card(&mut cols[1], "⌘", "Apple Operations",  "DFU · IPSW flash · iCloud · Passcode");
        qa_card(&mut cols[2], "◉", "AU Carrier Unlock", "Telstra · Optus · Vodafone AU + more");
    });
    ui.add_space(16.0);

    // ── System Status (.g2) ───────────────────────────────────────
    crate::app::section_hd(ui, "System Status");
    ui.columns(2, |cols| {
        // Runtime environment card
        crate::app::card_frame().show(&mut cols[0], |ui| {
            ui.label(RichText::new("RUNTIME ENVIRONMENT").size(8.5)
                .strong().color(ChimeraTheme::T2));
            ui.add_space(8.0);
            crate::app::kv(ui, "Version", "ChimeraRS v1.3.13", false);
            crate::app::kv(ui, "Build", "2026-03-31 · 14:39 UTC", false);
            crate::app::kv(ui, "Architecture", "macOS · x86_64-apple-darwin", false);
            crate::app::kv(ui, "Worker pool", "Running", false);
            // Re-probe ADB at most once every 5 seconds so the dashboard
            // reflects plug/unplug + manual installs in near real-time.
            state.refresh_adb_throttled();
            let adb_value = if state.adb_ok {
                match (state.adb_path.as_deref(), state.adb_version.as_deref()) {
                    (Some(p), Some(v)) => format!("{} — {}", v, p),
                    (Some(p), None)    => format!("Found at {}", p),
                    _                  => "Found".to_string(),
                }
            } else {
                state.adb_error.clone()
                    .unwrap_or_else(|| "Not found in PATH".to_string())
            };
            crate::app::kv(ui, "ADB daemon", &adb_value, state.adb_ok);
            ui.separator();
            crate::app::kv(ui, "libusb", "v0.9.4 — linked", false);
        });

        // Loaded modules card
        crate::app::card_frame().show(&mut cols[1], |ui| {
            ui.label(RichText::new("LOADED MODULES · RUNTIME TOPOLOGY").size(8.5)
                .strong().color(ChimeraTheme::T2));
            ui.add_space(8.0);

            module_group(ui, "Transport", &["adb","fastboot","edl","libusb"]);
            ui.add_space(6.0);
            module_group(ui, "Platform",  &["apple","samsung","xiaomi","huawei","mtk","motorola","sony","nokia","oppo"]);
            ui.add_space(6.0);
            module_group(ui, "Workspace", &["core","devices","firmware","utils","gui","api"]);
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                crate::app::chip_a(ui, "18 active modules");
                crate::app::chip_b(ui, "148 files · 27,244 LOC");
            });
        });
    });
}

fn mode_card(ui: &mut egui::Ui, title: &str, desc: &str, active: bool) {
    let (bg, border) = if active {
        (Color32::from_rgba_premultiplied(255, 255, 255, 7),
         ChimeraTheme::LNH)
    } else {
        (ChimeraTheme::S02, ChimeraTheme::LN)
    };

    egui::Frame::NONE
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, border))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            if active {
                // "Active" tag
                egui::Frame::NONE
                    .fill(ChimeraTheme::A18)
                    .stroke(Stroke::new(1.0_f32, ChimeraTheme::A30))
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin { left:6, right:6, top:2, bottom:2 })
                    .show(ui, |ui| {
                        ui.label(RichText::new("ACTIVE")
                            .size(8.0).strong().color(ChimeraTheme::A));
                    });
                ui.add_space(4.0);
            }
            ui.label(RichText::new(title)
                .size(12.5).strong().color(ChimeraTheme::T0));
            ui.add_space(4.0);
            ui.label(RichText::new(desc.to_uppercase())
                .size(9.5).color(ChimeraTheme::T2));
        });
}

fn stat_card(ui: &mut egui::Ui, width: f32, label: &str, value: &str, meta: &str, amber: bool) {
    let (_rect_id, _) = ui.allocate_space(egui::vec2(width, 0.0));
    let (_, _child_rect) = ui.allocate_space(egui::vec2(0.0, 0.0));
    let child_rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 80.0));
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(child_rect).layout(egui::Layout::top_down(egui::Align::LEFT)));

    egui::Frame::NONE
        .fill(ChimeraTheme::S02)
        .stroke(Stroke::new(1.0_f32, ChimeraTheme::LN))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(12))
        .show(&mut child_ui, |ui| {
            // left accent bar
            let r = ui.available_rect_before_wrap();
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(r.left() - 12.0, r.top() - 12.0),
                    egui::vec2(2.0, r.height() + 24.0)),
                egui::CornerRadius::same(1),
                if amber { ChimeraTheme::A } else { ChimeraTheme::T3 },
            );

            ui.label(RichText::new(label.to_uppercase())
                .size(8.5).color(ChimeraTheme::T2));
            ui.label(RichText::new(value)
                .size(22.0).strong()
                .color(if amber { ChimeraTheme::A } else { ChimeraTheme::T0 }));
            ui.label(RichText::new(meta.to_uppercase())
                .size(8.0).color(ChimeraTheme::T3));
        });
}

fn qa_card(ui: &mut egui::Ui, icon: &str, title: &str, sub: &str) {
    egui::Frame::NONE
        .fill(ChimeraTheme::S02)
        .stroke(Stroke::new(1.0_f32, ChimeraTheme::LN))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.label(RichText::new(icon)
                .size(14.0)
                .color(Color32::from_rgba_premultiplied(255, 255, 255, 71)));
            ui.add_space(5.0);
            ui.label(RichText::new(title.to_uppercase())
                .size(11.0).strong().color(ChimeraTheme::T0));
            ui.add_space(1.0);
            ui.label(RichText::new(sub.to_uppercase())
                .size(9.0).color(ChimeraTheme::T2));
        });
}

fn module_group(ui: &mut egui::Ui, group: &str, modules: &[&str]) {
    egui::Frame::NONE
        .fill(Color32::from_rgba_premultiplied(255, 255, 255, 6))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255,255,255,18)))
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin { left:11, right:11, top:8, bottom:8 })
        .show(ui, |ui| {
            ui.label(RichText::new(group.to_uppercase())
                .size(7.5).strong()
                .color(Color32::from_rgba_premultiplied(255, 255, 255, 115)));
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(5.0, 4.0);
                for m in modules {
                    crate::app::chip_g(ui, m);
                }
            });
        });
}
