// crates/chimera-gui/src/theme.rs
// All CSS variables from chimera-gui.html mapped 1-to-1 as Rust Color32 constants.
#![allow(dead_code, non_upper_case_globals)]

use egui::{Color32, FontFamily, FontId, Stroke, Visuals};

/// C = CSS token namespace. Every --var in the HTML has a matching C::VAR here.
pub struct C;
impl C {
    // Surface scale
    pub const S00: Color32 = Color32::from_rgb(0x04,0x05,0x06);
    pub const S01: Color32 = Color32::from_rgb(0x08,0x0A,0x0C);
    pub const S02: Color32 = Color32::from_rgb(0x0C,0x0F,0x12);
    pub const S03: Color32 = Color32::from_rgb(0x11,0x14,0x18);
    pub const S04: Color32 = Color32::from_rgb(0x16,0x1A,0x1F);
    pub const S05: Color32 = Color32::from_rgb(0x1B,0x20,0x28);
    pub const S06: Color32 = Color32::from_rgb(0x21,0x28,0x30);
    pub const S07: Color32 = Color32::from_rgb(0x29,0x31,0x3B);

    // Amber accent
    pub const A:   Color32 = Color32::from_rgb(0xE8,0x95,0x1E);
    pub const AH:  Color32 = Color32::from_rgb(0xF5,0xA6,0x23);
    pub const AL:  Color32 = Color32::from_rgb(0xB8,0x73,0x18);
    pub const A03: Color32 = Color32::from_rgba_premultiplied(0xE8,0x95,0x1E,0x08);
    pub const A06: Color32 = Color32::from_rgba_premultiplied(0xE8,0x95,0x1E,0x0F);
    pub const A10: Color32 = Color32::from_rgba_premultiplied(0xE8,0x95,0x1E,0x1A);
    pub const A18: Color32 = Color32::from_rgba_premultiplied(0xE8,0x95,0x1E,0x2E);
    pub const A30: Color32 = Color32::from_rgba_premultiplied(0xE8,0x95,0x1E,0x4D);
    pub const A48: Color32 = Color32::from_rgba_premultiplied(0xE8,0x95,0x1E,0x7A);

    // Text hierarchy
    pub const T0: Color32 = Color32::from_rgb(0xE2,0xE6,0xEC);
    pub const T1: Color32 = Color32::from_rgb(0xA4,0xAE,0xBA);
    pub const T2: Color32 = Color32::from_rgb(0x59,0x64,0x72);
    pub const T3: Color32 = Color32::from_rgb(0x32,0x3A,0x46);

    // Status
    pub const G:   Color32 = Color32::from_rgb(0x1A,0xB8,0x6A);
    pub const GBG: Color32 = Color32::from_rgba_premultiplied(0x1A,0xB8,0x6A,0x0F);
    pub const GBR: Color32 = Color32::from_rgba_premultiplied(0x1A,0xB8,0x6A,0x29);
    pub const R:   Color32 = Color32::from_rgb(0xC2,0x44,0x44);
    pub const RBG: Color32 = Color32::from_rgba_premultiplied(0xC2,0x44,0x44,0x0F);
    pub const RBR: Color32 = Color32::from_rgba_premultiplied(0xC2,0x44,0x44,0x29);
    pub const B:   Color32 = Color32::from_rgb(0x2F,0x7D,0xC0);
    pub const BBG: Color32 = Color32::from_rgba_premultiplied(0x2F,0x7D,0xC0,0x0F);
    pub const BBR: Color32 = Color32::from_rgba_premultiplied(0x2F,0x7D,0xC0,0x29);

    // Lines
    pub const LN:  Color32 = Color32::from_rgba_premultiplied(255,255,255,12);
    pub const LNH: Color32 = Color32::from_rgba_premultiplied(255,255,255,21);
    pub const LNA: Color32 = Color32::from_rgba_premultiplied(0xE8,0x95,0x1E,0x1A);

    // Legacy aliases (keeps existing code compiling)
    pub const ACCENT:         Color32 = Self::A;
    pub const ACCENT_HOVER:   Color32 = Self::AH;
    pub const ACCENT_DARK:    Color32 = Self::AL;
    pub const ACCENT_FILL:    Color32 = Self::A18;
    pub const BG_DARK:        Color32 = Self::S00;
    pub const BG_SIDEBAR:     Color32 = Self::S01;
    pub const BG_CARD:        Color32 = Self::S02;
    pub const BG_ELEVATED:    Color32 = Self::S04;
    pub const BG_ACTIVE:      Color32 = Self::S05;
    pub const BG_HEADER:      Color32 = Self::S01;
    pub const TAB_ACTIVE_BG:  Color32 = Color32::from_rgb(0x2E,0x2A,0x14);
    pub const TEXT_PRIMARY:   Color32 = Self::T0;
    pub const TEXT_SECONDARY: Color32 = Self::T1;
    pub const TEXT_DISABLED:  Color32 = Self::T2;
    pub const TEXT_HEADING:   Color32 = Self::T0;
    pub const BORDER:         Color32 = Self::LN;
    pub const BORDER_ACCENT:  Color32 = Self::A18;
    pub const BORDER_SUBTLE:  Color32 = Self::LN;
    pub const SUCCESS:        Color32 = Self::G;
    pub const SUCCESS_BG:     Color32 = Self::GBG;
    pub const WARNING:        Color32 = Self::A;
    pub const WARNING_BG:     Color32 = Self::A06;
    pub const ERROR:          Color32 = Self::R;
    pub const ERROR_BG:       Color32 = Self::RBG;
    pub const INFO:           Color32 = Self::B;
    pub const INFO_BG:        Color32 = Self::BBG;
    pub const SAMSUNG_BLUE:   Color32 = Color32::from_rgb(0x14,0x5A,0xAA);
    pub const XIAOMI_ORANGE:  Color32 = Color32::from_rgb(0xFF,0x6C,0x00);
    pub const HUAWEI_RED:     Color32 = Color32::from_rgb(0xCF,0x00,0x0F);
    pub const MTK_GREEN:      Color32 = Color32::from_rgb(0x00,0x96,0x3C);
    pub const APPLE_SILVER:   Color32 = Color32::from_rgb(0xA0,0xA8,0xB8);

    pub fn brand_color(brand: &str) -> Color32 {
        match brand.to_lowercase().as_str() {
            "samsung"  => Self::SAMSUNG_BLUE,
            "xiaomi"   => Self::XIAOMI_ORANGE,
            "huawei"   => Self::HUAWEI_RED,
            "mtk"|"mediatek" => Self::MTK_GREEN,
            "apple"    => Self::APPLE_SILVER,
            _          => Self::A,
        }
    }
    pub fn status_color(s: &str) -> Color32 {
        match s.to_lowercase().as_str() {
            "ok"|"success"|"connected"|"valid" => Self::G,
            "warn"|"warning"|"partial"         => Self::A,
            "err"|"error"|"failed"|"invalid"   => Self::R,
            _                                  => Self::T2,
        }
    }

    // ── Constants used by older panels ─────────────────────────────
    pub const CARD_BG:   Color32 = Self::S02;
    pub const R_SM: f32 = 5.0;
    pub const R_MD: f32 = 8.0;
    pub const R_LG: f32 = 10.0;

    pub fn accent_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(egui::Button::new(
            egui::RichText::new(label).size(10.5).strong()
                .color(egui::Color32::from_rgb(3, 1, 0)))
            .fill(Self::A)
            .stroke(egui::Stroke::new(1.0_f32, Self::AH))
            .corner_radius(8)
            .min_size(egui::vec2(0.0, 30.0)))
    }
    pub fn outline_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(egui::Button::new(
            egui::RichText::new(label).size(10.5).color(Self::T1))
            .fill(egui::Color32::from_rgb(26, 30, 36))
            .stroke(egui::Stroke::new(1.0_f32,
                egui::Color32::from_rgba_premultiplied(255, 255, 255, 18)))
            .corner_radius(8)
            .min_size(egui::vec2(0.0, 30.0)))
    }
    pub fn status_dot(ui: &mut egui::Ui, color: Color32) {
        let (r, _) = ui.allocate_exact_size(
            egui::vec2(6.0, 6.0), egui::Sense::hover());
        if ui.is_rect_visible(r) {
            ui.painter().circle_filled(r.center(), 3.0, color);
        }
    }
    pub fn golden_badge(ui: &mut egui::Ui, label: &str) {
        egui::Frame::NONE
            .fill(Self::A18)
            .stroke(egui::Stroke::new(1.0_f32, Self::A30))
            .corner_radius(4)
            .inner_margin(egui::Margin {
                left: 6, right: 6, top: 2, bottom: 2,
            })
            .show(ui, |ui| {
                ui.label(egui::RichText::new(label.to_uppercase())
                    .size(8.0).strong().color(Self::A));
            });
    }
    pub fn nav_section_label(ui: &mut egui::Ui, label: &str) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(13.0);
            ui.label(egui::RichText::new(label.to_uppercase())
                .size(7.0).strong()
                .color(egui::Color32::from_rgba_premultiplied(255, 255, 255, 92)));
        });
        ui.add_space(3.0);
    }
    pub fn nav_item(
        ui: &mut egui::Ui,
        icon: &str,
        label: &str,
        active: bool,
    ) -> egui::Response {
        let (outer, resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 34.0),
            egui::Sense::click(),
        );
        if ui.is_rect_visible(outer) {
            let inner = outer.shrink2(egui::vec2(8.0, 2.0));
            let bg = if active {
                egui::Color32::from_rgba_premultiplied(255, 255, 255, 18)
            } else if resp.hovered() {
                egui::Color32::from_rgba_premultiplied(255, 255, 255, 10)
            } else {
                egui::Color32::TRANSPARENT
            };
            let stroke = if active {
                egui::Stroke::new(1.0_f32,
                    egui::Color32::from_rgba_premultiplied(255, 255, 255, 45))
            } else {
                egui::Stroke::NONE
            };
            let p = ui.painter();
            p.rect(inner, egui::CornerRadius::same(8), bg, stroke, egui::StrokeKind::Middle);
            let ic = if active { Self::T0 }
                else if resp.hovered() { Self::T1 }
                else { Self::T2 };
            let lc = if active {
                egui::Color32::from_rgb(0xF4, 0xF7, 0xFA)
            } else if resp.hovered() {
                egui::Color32::from_rgb(0xEE, 0xF2, 0xF5)
            } else {
                egui::Color32::from_rgb(0xB1, 0xBA, 0xC5)
            };
            p.text(
                egui::pos2(inner.left() + 10.0, inner.center().y),
                egui::Align2::LEFT_CENTER, icon,
                egui::FontId::new(9.0, egui::FontFamily::Proportional),
                ic,
            );
            p.text(
                egui::pos2(inner.left() + 28.0, inner.center().y),
                egui::Align2::LEFT_CENTER, &label.to_uppercase(),
                egui::FontId::new(11.0, egui::FontFamily::Proportional),
                lc,
            );
        }
        resp
    }
    pub fn card_frame() -> egui::Frame {
        egui::Frame::NONE
            .fill(egui::Color32::from_rgb(16, 19, 23))
            .stroke(egui::Stroke::new(1.0_f32,
                egui::Color32::from_rgba_premultiplied(255, 255, 255, 15)))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(15))
            .shadow(egui::epaint::Shadow { offset: [0i8, 2i8], blur: 8, spread: 0, color: egui::Color32::from_black_alpha(46) })
    }
}

// Keep ChimeraTheme as alias
pub type ChimeraTheme = C;

pub fn apply(ctx: &egui::Context) {
    ctx.set_visuals(build_visuals());
    apply_text_styles(ctx);
}

fn build_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.window_fill      = C::S02;
    v.panel_fill       = C::S01;
    v.faint_bg_color   = C::S03;
    v.extreme_bg_color = C::S00;
    v.code_bg_color    = C::S00;
    v.window_corner_radius = egui::CornerRadius::same(8);
    v.window_stroke    = Stroke::new(1.0_f32, C::LN);
    v.window_shadow    = egui::epaint::Shadow { offset: [0i8, 4i8], blur: 12, spread: 0, color: egui::Color32::from_black_alpha(120) };
    v.popup_shadow     = egui::epaint::Shadow { offset: [0i8, 2i8], blur: 6, spread: 0, color: egui::Color32::from_black_alpha(80) };
    v.selection.bg_fill= C::A18;
    v.selection.stroke = Stroke::new(1.0_f32, C::A);
    v.hyperlink_color  = C::AH;

    let wi = |w: &mut egui::style::WidgetVisuals, fill: Color32, stroke: Color32, fg: Color32| {
        w.bg_fill      = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke    = Stroke::new(1.0_f32, stroke);
        w.fg_stroke    = Stroke::new(1.0_f32, fg);
        w.corner_radius = egui::CornerRadius::same(8);
    };
    wi(&mut v.widgets.noninteractive, C::S02, C::LN,  C::T2);
    wi(&mut v.widgets.inactive,       Color32::from_rgb(0x1E,0x22,0x28), C::LN,  C::T1);
    wi(&mut v.widgets.hovered,        Color32::from_rgb(0x24,0x28,0x30), C::LNH, C::T0);
    wi(&mut v.widgets.active,         Color32::from_rgb(0x28,0x2C,0x36), C::A30, C::A);
    wi(&mut v.widgets.open,           Color32::from_rgb(0x1E,0x22,0x28), C::A18, C::A);
    v
}

fn apply_text_styles(ctx: &egui::Context) {
    use egui::TextStyle::*;
    let mut style = (*ctx.global_style()).clone();
    style.text_styles = [
        (Heading,   FontId::new(15.0, FontFamily::Proportional)),
        (Body,      FontId::new(11.5, FontFamily::Proportional)),
        (Monospace, FontId::new(10.5, FontFamily::Monospace)),
        (Button,    FontId::new(10.5, FontFamily::Proportional)),
        (Small,     FontId::new(9.0,  FontFamily::Proportional)),
    ].into();
    style.spacing.item_spacing     = egui::vec2(8.0, 6.0);
    style.spacing.button_padding   = egui::vec2(10.0, 5.0);
    style.spacing.scroll.bar_width = 3.0;
    ctx.set_global_style(style);
}
