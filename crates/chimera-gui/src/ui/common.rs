#![allow(dead_code)]
// chimera-gui/src/ui/common.rs
// Shared styled UI components — ChimeraTool aesthetic
// All widgets use ChimeraTheme colours for visual consistency.

use eframe::egui::{self, Color32, RichText, Stroke};
use crate::theme::ChimeraTheme;

// ─── Section headers ─────────────────────────────────────────────────────────

/// Bold amber section header with golden left-border accent
pub fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.add_space(8.0);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 26.0),
        egui::Sense::hover(),
    );
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        // left accent bar
        p.rect_filled(
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height())),
            egui::CornerRadius::same(1),
            ChimeraTheme::ACCENT,
        );
        // title text
        p.text(
            rect.left_center() + egui::vec2(10.0, 0.0),
            egui::Align2::LEFT_CENTER,
            title,
            egui::FontId::proportional(14.0),
            ChimeraTheme::TEXT_HEADING,
        );
    }
    ui.add_space(4.0);
}

/// Small sub-section label (muted)
pub fn sub_header(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.colored_label(
        ChimeraTheme::TEXT_SECONDARY,
        RichText::new(title).strong().size(12.0),
    );
    ui.add_space(2.0);
}

// ─── Buttons ─────────────────────────────────────────────────────────────────

/// Primary golden action button (fixed width)
pub fn op_button(ui: &mut egui::Ui, label: &str, icon: &str) -> bool {
    let text = format!("{} {}", icon, label);
    ChimeraTheme::accent_button(
        ui,
        &text,
    )
    .clicked()
}

/// Outlined secondary button (fixed width)
pub fn secondary_button(ui: &mut egui::Ui, label: &str, icon: &str) -> bool {
    let text = format!("{} {}", icon, label);
    ChimeraTheme::outline_button(ui, &text).clicked()
}

/// Danger / destructive button (red outline)
pub fn danger_button(ui: &mut egui::Ui, label: &str) -> bool {
    let btn = egui::Button::new(
        RichText::new(label).color(ChimeraTheme::ERROR).size(13.0),
    )
    .fill(Color32::TRANSPARENT)
    .stroke(Stroke::new(1.0_f32, ChimeraTheme::ERROR))
    .corner_radius(egui::CornerRadius::same(5));
    ui.add(btn).clicked()
}

// ─── Progress ────────────────────────────────────────────────────────────────

/// Amber-tinted progress bar with label
pub fn progress_bar(ui: &mut egui::Ui, percent: f32, label: &str) {
    ui.add_space(4.0);
    let bar = egui::ProgressBar::new(percent / 100.0)
        .text(format!("{} ({:.1}%)", label, percent))
        .animate(percent > 0.0 && percent < 100.0)
        .fill(ChimeraTheme::ACCENT);
    ui.add(bar);
}

// ─── Badges ──────────────────────────────────────────────────────────────────

/// Pill-shaped status badge with customisable colour
pub fn status_badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    let frame = egui::Frame::NONE
        .fill(color.linear_multiply(0.18))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .corner_radius(egui::CornerRadius::same(12))
        .stroke(Stroke::new(1.0_f32, color));
    frame.show(ui, |ui| {
        ui.colored_label(color, RichText::new(text).small().strong());
    });
}

/// Convenience wrappers
pub fn badge_success(ui: &mut egui::Ui, text: &str) { status_badge(ui, text, ChimeraTheme::SUCCESS); }
pub fn badge_warning(ui: &mut egui::Ui, text: &str) { status_badge(ui, text, ChimeraTheme::WARNING); }
pub fn badge_error  (ui: &mut egui::Ui, text: &str) { status_badge(ui, text, ChimeraTheme::ERROR  ); }
pub fn badge_info   (ui: &mut egui::Ui, text: &str) { status_badge(ui, text, ChimeraTheme::INFO   ); }
pub fn badge_accent (ui: &mut egui::Ui, text: &str) { status_badge(ui, text, ChimeraTheme::ACCENT ); }

// ─── Info rows ───────────────────────────────────────────────────────────────

/// Label : Value row (muted label, monospace value)
pub fn info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.colored_label(ChimeraTheme::TEXT_SECONDARY, format!("{}: ", label));
        ui.colored_label(ChimeraTheme::TEXT_PRIMARY, RichText::new(value).monospace());
    });
}

/// Label : Value row with 📋 copy button
pub fn info_row_copy(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.colored_label(ChimeraTheme::TEXT_SECONDARY, format!("{}: ", label));
        ui.colored_label(ChimeraTheme::TEXT_PRIMARY, RichText::new(value).monospace());
        if !value.is_empty() {
            if ui.small_button("📋").on_hover_text("Copy to clipboard").clicked() {
                ui.output_mut(|o| o.commands.push(egui::OutputCommand::CopyText(value.to_string())));
            }
        }
    });
}

/// Two-column key/value grid  (label left-aligned, value right-aligned)
pub fn kv_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.columns(2, |cols| {
        cols[0].colored_label(ChimeraTheme::TEXT_SECONDARY, label);
        cols[1].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(ChimeraTheme::TEXT_PRIMARY, RichText::new(value).monospace().size(12.5));
        });
    });
}

// ─── Notification boxes ──────────────────────────────────────────────────────

pub fn warning_box(ui: &mut egui::Ui, message: &str) {
    let frame = egui::Frame::NONE
        .fill(ChimeraTheme::WARNING_BG)
        .inner_margin(egui::Margin::symmetric(10, 7))
        .corner_radius(egui::CornerRadius::same(6))
        .stroke(Stroke::new(1.0_f32, ChimeraTheme::WARNING));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.colored_label(ChimeraTheme::WARNING, "⚠  ");
            ui.colored_label(ChimeraTheme::WARNING, message);
        });
    });
}

pub fn success_box(ui: &mut egui::Ui, message: &str) {
    let frame = egui::Frame::NONE
        .fill(ChimeraTheme::SUCCESS_BG)
        .inner_margin(egui::Margin::symmetric(10, 7))
        .corner_radius(egui::CornerRadius::same(6))
        .stroke(Stroke::new(1.0_f32, ChimeraTheme::SUCCESS));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.colored_label(ChimeraTheme::SUCCESS, "✔  ");
            ui.colored_label(ChimeraTheme::SUCCESS, message);
        });
    });
}

pub fn error_box(ui: &mut egui::Ui, message: &str) {
    let frame = egui::Frame::NONE
        .fill(ChimeraTheme::ERROR_BG)
        .inner_margin(egui::Margin::symmetric(10, 7))
        .corner_radius(egui::CornerRadius::same(6))
        .stroke(Stroke::new(1.0_f32, ChimeraTheme::ERROR));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.colored_label(ChimeraTheme::ERROR, "✘  ");
            ui.colored_label(ChimeraTheme::ERROR, message);
        });
    });
}

pub fn info_box(ui: &mut egui::Ui, message: &str) {
    let frame = egui::Frame::NONE
        .fill(ChimeraTheme::INFO_BG)
        .inner_margin(egui::Margin::symmetric(10, 7))
        .corner_radius(egui::CornerRadius::same(6))
        .stroke(Stroke::new(1.0_f32, ChimeraTheme::INFO));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.colored_label(ChimeraTheme::INFO, "ℹ  ");
            ui.colored_label(ChimeraTheme::INFO, message);
        });
    });
}

/// Legal disclaimer / research-only reminder box
pub fn legal_box(ui: &mut egui::Ui) {
    let frame = egui::Frame::NONE
        .fill(Color32::from_rgb(0x18, 0x14, 0x08))
        .inner_margin(egui::Margin::symmetric(10, 7))
        .corner_radius(egui::CornerRadius::same(6))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x60, 0x50, 0x10)));
    frame.show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(ChimeraTheme::WARNING, "⚖  ");
            ui.colored_label(
                Color32::from_rgb(0xcc, 0xaa, 0x44),
                "Research & educational use only. \
                 Bypassing activation/carrier locks may violate AU Criminal Code §478 \
                 or US CFAA 18 U.S.C. §1030. Proceed only with device owner permission.",
            );
        });
    });
}

// ─── Collapsible sections ─────────────────────────────────────────────────────

pub fn collapsible_section<R>(
    ui:           &mut egui::Ui,
    id:           &str,
    title:        &str,
    default_open: bool,
    content:      impl FnOnce(&mut egui::Ui) -> R,
) {
    let persistent_id = ui.make_persistent_id(id);
    egui::collapsing_header::CollapsingHeader::new(
        RichText::new(title).color(ChimeraTheme::TEXT_SECONDARY).size(13.0),
    )
    .id_salt(persistent_id)
    .default_open(default_open)
    .show(ui, content);
}

// ─── File pickers ─────────────────────────────────────────────────────────────

/// Inline file-picker row  label | [path text-edit] | [Browse]
pub fn file_picker(ui: &mut egui::Ui, label: &str, path: &mut String, ext: &str) {
    ui.horizontal(|ui| {
        ui.colored_label(ChimeraTheme::TEXT_SECONDARY, label);
        ui.add(
            egui::TextEdit::singleline(path)
                .hint_text("(none)")
                .desired_width(260.0),
        );
        if ChimeraTheme::outline_button(ui, "📁 Browse").clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("Firmware", &[ext])
                .pick_file()
            {
                *path = p.to_string_lossy().to_string();
            }
        }
    });
}

/// Inline directory-picker row
pub fn dir_picker(ui: &mut egui::Ui, label: &str, path: &mut String) {
    ui.horizontal(|ui| {
        ui.colored_label(ChimeraTheme::TEXT_SECONDARY, label);
        ui.add(
            egui::TextEdit::singleline(path)
                .hint_text("(none)")
                .desired_width(260.0),
        );
        if ChimeraTheme::outline_button(ui, "📁 Browse").clicked() {
            if let Some(p) = rfd::FileDialog::new().pick_folder() {
                *path = p.to_string_lossy().to_string();
            }
        }
    });
}

// ─── Horizontal divider ───────────────────────────────────────────────────────

pub fn divider(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.painter().hline(
        ui.available_rect_before_wrap().x_range(),
        ui.cursor().top() + 3.0,
        Stroke::new(1.0_f32, ChimeraTheme::BORDER_SUBTLE),
    );
    ui.add_space(8.0);
}

// ─── Code / monospace block ───────────────────────────────────────────────────

pub fn code_block(ui: &mut egui::Ui, text: &str) {
    let frame = egui::Frame::NONE
        .fill(ChimeraTheme::BG_SIDEBAR)
        .inner_margin(egui::Margin::same(8))
        .corner_radius(egui::CornerRadius::same(5))
        .stroke(Stroke::new(1.0_f32, ChimeraTheme::BORDER_SUBTLE));
    frame.show(ui, |ui| {
        ui.colored_label(
            ChimeraTheme::TEXT_PRIMARY,
            RichText::new(text).monospace().size(12.0),
        );
    });
}
