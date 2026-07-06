// crates/chimera-gui/src/ui/nav.rs
// Navigation sidebar — exact match to chimera-gui.html sidebar structure.
// Groups: WORKSPACE · DEVICE · PLATFORM
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use egui::{Color32, RichText, Stroke};
use crate::theme::ChimeraTheme;

// ══════════════════════════════════════════════════════════════════
//  PAGE ENUM  — one variant per HTML data-pg value
// ══════════════════════════════════════════════════════════════════
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Page {
    // WORKSPACE group
    #[default]
    Dashboard,     // pg-dash  ▦
    Devices,       // pg-devs  ◫
    Downloads,     // pg-dld   ↓
    History,       // pg-hist  ◷
    Utilities,     // pg-util  ⚒
    Settings,      // pg-cfg   ⚙

    // DEVICE group
    DeviceInfo,    // pg-dinfo ℹ
    Jailbreak,     // pg-jb    ◎
    SshVpn,        // pg-ssh   ⇄
    Activation,    // pg-act   ☁
    Network,       // pg-nwk   ◈
    Tools,         // pg-tls   ⚒

    // PLATFORM group
    AppleIos,      // pg-ios   ⌘
    MediaTek,      // pg-mtk   ⚡
    AuUnlock,      // pg-au    ◉
    ShshBlobs,     // pg-shsh  ◰
    ApiTools,      // pg-api   ⬡
    EventLog,      // pg-evlog ≡
}

struct NavItem {
    page:  Page,
    icon:  &'static str,
    label: &'static str,
}

static WORKSPACE: &[NavItem] = &[
    NavItem { page: Page::Dashboard, icon: "▦", label: "Dashboard"    },
    NavItem { page: Page::Devices,   icon: "◫", label: "Devices"      },
    NavItem { page: Page::Downloads, icon: "↓", label: "Downloads"    },
    NavItem { page: Page::History,   icon: "◷", label: "Work History" },
    NavItem { page: Page::Utilities, icon: "⚒", label: "Utilities"    },
    NavItem { page: Page::Settings,  icon: "⚙", label: "Settings"     },
];

static DEVICE: &[NavItem] = &[
    NavItem { page: Page::DeviceInfo, icon: "ℹ", label: "Device Info" },
    NavItem { page: Page::Jailbreak,  icon: "◎", label: "Jailbreak"   },
    NavItem { page: Page::SshVpn,     icon: "⇄", label: "SSH · VPN"  },
    NavItem { page: Page::Activation, icon: "☁", label: "Activation"  },
    NavItem { page: Page::Network,    icon: "◈", label: "Network"     },
    NavItem { page: Page::Tools,      icon: "⚒", label: "Tools"       },
];

static PLATFORM: &[NavItem] = &[
    NavItem { page: Page::AppleIos,  icon: "⌘", label: "Apple iOS"  },
    NavItem { page: Page::MediaTek,  icon: "⚡", label: "MediaTek"  },
    NavItem { page: Page::AuUnlock,  icon: "◉", label: "AU Unlock"  },
    NavItem { page: Page::ShshBlobs, icon: "◰", label: "SHSH Blobs" },
    NavItem { page: Page::ApiTools,  icon: "⬡", label: "API Tools"  },
    NavItem { page: Page::EventLog,  icon: "≡", label: "Event Log"  },
];

// ══════════════════════════════════════════════════════════════════
//  SIDEBAR RENDER
// ══════════════════════════════════════════════════════════════════
pub fn render_sidebar(
    ui: &mut egui::Ui,
    current: &mut Page,
    hostname: &str,
    worker_ok: bool,
    adb_ok:    bool,
    usb_ok:    bool,
    uptime:    &str,
    build_ts:  &str,
    build_hash: &str,
) {
    // ── Session identity card ─────────────────────────────────────
    egui::Frame::NONE
        .fill(Color32::from_rgba_premultiplied(255, 255, 255, 5))
        .inner_margin(egui::Margin { left:14, right:14, top:14, bottom:12 })
        .show(ui, |ui| {
            // "SESSION" role label
            ui.horizontal(|ui| {
                ui.label(RichText::new("SESSION")
                    .size(7.5).strong()
                    .color(Color32::from_rgba_premultiplied(255,255,255,133)));
            });
            ui.add_space(3.0);

            // Username / hostname
            ui.label(RichText::new("ADMIN_USER")
                .size(12.0).strong().color(ChimeraTheme::T0));
            ui.label(RichText::new(format!("{} · MACOS", hostname.to_uppercase()))
                .size(8.5).color(ChimeraTheme::T1));

            // Status pills
            ui.add_space(7.0);
            ui.horizontal(|ui| {
                status_pill(ui, "Worker", worker_ok);
                status_pill(ui, "ADB",    adb_ok);
                status_pill(ui, "libusb", usb_ok);
            });
        });

    // ── Navigation groups ─────────────────────────────────────────
    egui::ScrollArea::vertical()
        .id_salt("nav_scroll")
        .show(ui, |ui| {
            nav_group(ui, "Workspace", WORKSPACE, current);
            nav_separator(ui);
            nav_group(ui, "Device",    DEVICE,    current);
            nav_separator(ui);
            nav_group(ui, "Platform",  PLATFORM,  current);
        });

    // ── Sidebar footer ─────────────────────────────────────────────
    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        egui::Frame::NONE
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0_f32, ChimeraTheme::LN))
            .inner_margin(egui::Margin { left:10, right:10, top:8, bottom:8 })
            .show(ui, |ui| {
                sf_row(ui, "Hash",   build_hash, true);
                sf_row(ui, "Uptime", uptime, false);
                sf_row(ui, "Build",  build_ts, false);
            });
    });
}

// ── Helpers ───────────────────────────────────────────────────────
fn nav_group(ui: &mut egui::Ui, label: &str, items: &[NavItem], current: &mut Page) {
    ui.add_space(6.0);
    // Group header
    ui.horizontal(|ui| {
        ui.add_space(13.0);
        ui.label(RichText::new(label.to_uppercase())
            .size(7.0).strong()
            .color(Color32::from_rgba_premultiplied(255,255,255,92)));
    });
    ui.add_space(3.0);

    for item in items {
        let active = *current == item.page;
        let resp = nav_item(ui, item.icon, item.label, active);
        if resp.clicked() {
            *current = item.page.clone();
        }
    }
    ui.add_space(2.0);
}

fn nav_item(ui: &mut egui::Ui, icon: &str, label: &str, active: bool) -> egui::Response {
    let _desired = egui::vec2(ui.available_width() - 16.0, 30.0);

    // Outer frame with margin
    let (outer_rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 34.0),
        egui::Sense::click(),
    );

    if ui.is_rect_visible(outer_rect) {
        let _visuals = ui.visuals();
        let inner_rect = outer_rect.shrink2(egui::vec2(8.0, 2.0));

        // Background
        let bg = if active {
            Color32::from_rgba_premultiplied(255, 255, 255, 12)
        } else if resp.hovered() {
            Color32::from_rgba_premultiplied(255, 255, 255, 8)
        } else {
            Color32::TRANSPARENT
        };

        let border = if active {
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 33))
        } else if resp.hovered() {
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 23))
        } else {
            Stroke::NONE
        };

        let painter = ui.painter();
        painter.rect(inner_rect, egui::CornerRadius::same(8), bg, border, egui::StrokeKind::Inside);

        // Icon
        let icon_pos = egui::pos2(inner_rect.left() + 10.0, inner_rect.center().y);
        let icon_color = if active { ChimeraTheme::T0 } else if resp.hovered() { ChimeraTheme::T1 } else { ChimeraTheme::T2 };
        painter.text(
            icon_pos,
            egui::Align2::LEFT_CENTER,
            icon,
            egui::FontId::new(10.0, egui::FontFamily::Proportional),
            icon_color,
        );

        // Label
        let label_color = if active { Color32::from_rgb(0xF3, 0xF6, 0xF8) }
            else if resp.hovered() { Color32::from_rgb(0xEE, 0xF2, 0xF5) }
            else { Color32::from_rgb(0xB1, 0xBA, 0xC5) };

        painter.text(
            egui::pos2(inner_rect.left() + 28.0, inner_rect.center().y),
            egui::Align2::LEFT_CENTER,
            label.to_uppercase(),
            egui::FontId::new(11.0, egui::FontFamily::Proportional),
            label_color,
        );
    }

    resp
}

fn status_pill(ui: &mut egui::Ui, label: &str, ok: bool) {
    let (fg, bg, border) = if ok {
        (ChimeraTheme::G,
         Color32::from_rgba_premultiplied(255,255,255,6),
         Color32::from_rgba_premultiplied(26,184,106,46))
    } else {
        (ChimeraTheme::A,
         Color32::from_rgba_premultiplied(255,255,255,6),
         Color32::from_rgba_premultiplied(232,149,30,46))
    };

    egui::Frame::NONE
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, border))
        .corner_radius(egui::CornerRadius::same(255))
        .inner_margin(egui::Margin { left:6, right:6, top:4, bottom:4 })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // dot
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(4.0, 4.0), egui::Sense::hover());
                if ui.is_rect_visible(rect) {
                    ui.painter().circle_filled(rect.center(), 2.0, fg);
                }
                ui.label(RichText::new(label.to_uppercase())
                    .size(7.8).strong().color(fg));
            });
        });
}

fn nav_separator(ui: &mut egui::Ui) {
    ui.add_space(2.0);
    let rect = ui.available_rect_before_wrap();
    let y = rect.top() + 1.0;
    ui.painter().line_segment(
        [egui::pos2(rect.left() + 10.0, y),
         egui::pos2(rect.left() + rect.width() * 0.7, y)],
        Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255,255,255,20)),
    );
    ui.add_space(5.0);
}

fn sf_row(ui: &mut egui::Ui, key: &str, value: &str, mono: bool) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(key.to_uppercase())
            .size(7.0).color(ChimeraTheme::T3));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if mono {
                ui.label(RichText::new(value)
                    .size(7.0).color(ChimeraTheme::T3)
                    .family(egui::FontFamily::Monospace));
            } else {
                ui.label(RichText::new(value)
                    .size(8.0).color(ChimeraTheme::T1));
            }
        });
    });
}
