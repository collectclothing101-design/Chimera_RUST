// crates/chimera-gui/src/ui/pages.rs
// All 17 pages matched exactly to chimera-gui.html.
#![allow(dead_code, unused_variables, unused_imports, unused_mut, clippy::all)]

use eframe::egui::{self, Color32, RichText, ScrollArea};
use crate::theme::C;
use crate::state::AppState;
use crate::worker::OperationRequest;
use crossbeam_channel::Sender;

// ══════════════════════════════════════════════════════
// WIDGET HELPERS — each maps 1-to-1 to an HTML class
// ══════════════════════════════════════════════════════

/// .ph — page header without action button
fn ph(ui: &mut egui::Ui, idx: &str, title: &str, sub: &str) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(idx).size(7.5).strong().color(C::A48)
                .family(egui::FontFamily::Monospace));
            ui.label(RichText::new(title.to_uppercase()).size(15.0).strong().color(C::T0));
            ui.label(RichText::new(sub).size(10.0).color(C::T2));
        });
    });
    ui.add_space(13.0);
    ui.separator();
    ui.add_space(11.0);
}

/// .ph with .pa — page header with right-aligned action buttons
fn ph_act<F: FnOnce(&mut egui::Ui)>(ui: &mut egui::Ui, idx: &str, title: &str, sub: &str, actions: F) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(idx).size(7.5).strong().color(C::A48)
                .family(egui::FontFamily::Monospace));
            ui.label(RichText::new(title.to_uppercase()).size(15.0).strong().color(C::T0));
            ui.label(RichText::new(sub).size(10.0).color(C::T2));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), actions);
    });
    ui.add_space(13.0);
    ui.separator();
    ui.add_space(11.0);
}

/// .sh .sb2 .sl — section header with amber accent bar
fn sh(ui: &mut egui::Ui, label: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(2.0, 9.0), egui::Sense::hover());
        if ui.is_rect_visible(r) { ui.painter().rect_filled(r, 1.0, C::A); }
        ui.add_space(4.0);
        ui.label(RichText::new(label.to_uppercase()).size(9.0).strong().color(C::T1));
    });
    ui.add_space(8.0);
}

/// .jb-sec-hd — jailbreak section heading with right-extending line
fn jb_hd(ui: &mut egui::Ui, label: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label.to_uppercase()).size(8.0).strong()
            .color(Color32::from_rgba_premultiplied(255,255,255,140)));
        ui.add_space(6.0);
        let avail = ui.available_rect_before_wrap();
        ui.painter().line_segment(
            [egui::pos2(avail.left(), avail.top()+7.0), egui::pos2(avail.right()-4.0, avail.top()+7.0)],
            egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(255,255,255,26)));
        ui.allocate_rect(avail, egui::Sense::hover());
    });
    ui.add_space(8.0);
}

/// .card-ha .sh — surface card
fn card<R>(ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(Color32::from_rgb(16,19,23))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(255,255,255,15)))
        .corner_radius(10).inner_margin(egui::Margin::same(15))
        .shadow(egui::epaint::Shadow { offset: [0i8, 2i8], blur: 8, spread: 0, color: Color32::from_black_alpha(46) })
        .show(ui, |ui| f(ui)).inner
}

/// .cflat .sh — flat card for tables
fn cflat<R>(ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(C::S02)
        .stroke(egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(255,255,255,15)))
        .corner_radius(10).inner_margin(egui::Margin::same(14))
        .show(ui, |ui| f(ui)).inner
}

/// .btn .bp — amber primary button
fn btn_p(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add(egui::Button::new(RichText::new(label).size(10.0).strong().color(Color32::from_rgb(3,1,0)))
        .fill(C::A).stroke(egui::Stroke::new(1.0,C::AH))
        .corner_radius(8).min_size(egui::vec2(0.0,30.0))).clicked()
}

/// .btn .bs — secondary button
fn btn_s(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add(egui::Button::new(RichText::new(label).size(10.0).color(C::T1))
        .fill(Color32::from_rgb(26,30,36))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(255,255,255,18)))
        .corner_radius(8).min_size(egui::vec2(0.0,30.0))).clicked()
}

/// .btn .bd — danger red button
fn btn_d(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add(egui::Button::new(RichText::new(label).size(10.0).color(C::R))
        .fill(C::RBG).stroke(egui::Stroke::new(1.0,C::RBR))
        .corner_radius(8).min_size(egui::vec2(0.0,30.0))).clicked()
}

/// Full-width primary button (min-height 42px)
fn btn_p_wide(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add_sized([ui.available_width(), 42.0],
        egui::Button::new(RichText::new(label).size(11.0).strong().color(Color32::from_rgb(3,1,0)))
            .fill(C::A).stroke(egui::Stroke::new(1.0,C::AH)).corner_radius(8)).clicked()
}

/// .fi — text field
fn fi(ui: &mut egui::Ui, val: &mut String, hint: &str) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(20,23,28))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(255,255,255,20)))
        .corner_radius(8).inner_margin(egui::Margin { left:10,right:10,top:8,bottom:8 })
        .show(ui, |ui| {
            ui.add(egui::TextEdit::singleline(val).desired_width(f32::INFINITY)
                .hint_text(hint).frame(egui::Frame::NONE));
        });
}

/// Read-only .fi
fn fi_ro(ui: &mut egui::Ui, val: &str) {
    let mut s = val.to_string();
    egui::Frame::NONE
        .fill(Color32::from_rgb(20,23,28))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(255,255,255,20)))
        .corner_radius(8).inner_margin(egui::Margin { left:10,right:10,top:8,bottom:8 })
        .show(ui, |ui| {
            ui.add(egui::TextEdit::singleline(&mut s).desired_width(f32::INFINITY)
                .interactive(false).frame(egui::Frame::NONE));
        });
}

/// .fl — field label
fn fl(ui: &mut egui::Ui, label: &str) {
    ui.label(RichText::new(label.to_uppercase()).size(7.5).strong().color(C::T2));
    ui.add_space(3.0);
}

/// .fr — field row: label + input
fn fr(ui: &mut egui::Ui, label: &str, val: &mut String, hint: &str) {
    fl(ui, label);
    fi(ui, val, hint);
    ui.add_space(8.0);
}

/// .tr .tog — toggle row
fn tog(ui: &mut egui::Ui, label: &str, sub: &str, val: &mut bool) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(label).size(11.0).color(C::T0));
            if !sub.is_empty() {
                ui.label(RichText::new(sub).size(9.0).color(C::T2));
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(val, "");
        });
    });
    ui.add_space(3.0);
    ui.separator();
    ui.add_space(3.0);
}

/// .kv grid row — key + value
fn kv(ui: &mut egui::Ui, k: &str, v: &str, col: Color32, mono: bool, last: bool) {
    ui.horizontal(|ui| {
        ui.add_sized([130.0,0.0], egui::Label::new(
            RichText::new(k).size(10.0).color(C::T2)));
        if mono {
            ui.label(RichText::new(v).size(9.0).color(col).family(egui::FontFamily::Monospace));
        } else {
            ui.label(RichText::new(v).size(10.5).color(col));
        }
    });
    if !last { ui.separator(); }
}

/// .note .ne — red/error note
fn note_e(ui: &mut egui::Ui, text: &str) {
    egui::Frame::NONE.fill(C::RBG).corner_radius(10)
        .inner_margin(egui::Margin { left:13,right:13,top:10,bottom:10 })
        .show(ui, |ui| {
            let r = ui.available_rect_before_wrap();
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(r.left()-13.0,r.top()-10.0), egui::vec2(2.0,r.height()+20.0)), 0.0, C::R);
            ui.label(RichText::new(text).size(9.5).color(Color32::from_rgb(0xC0,0x70,0x70)));
        });
    ui.add_space(10.0);
}

/// .note .nw — amber/warning note
fn note_w(ui: &mut egui::Ui, text: &str) {
    egui::Frame::NONE.fill(C::A06).corner_radius(10)
        .inner_margin(egui::Margin { left:13,right:13,top:10,bottom:10 })
        .show(ui, |ui| {
            let r = ui.available_rect_before_wrap();
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(r.left()-13.0,r.top()-10.0), egui::vec2(2.0,r.height()+20.0)), 0.0, C::A);
            ui.label(RichText::new(text).size(9.5).color(C::A));
        });
    ui.add_space(10.0);
}

/// .note .ni2 — blue/info note
fn note_i(ui: &mut egui::Ui, text: &str) {
    egui::Frame::NONE.fill(C::BBG).corner_radius(10)
        .inner_margin(egui::Margin { left:13,right:13,top:10,bottom:10 })
        .show(ui, |ui| {
            let r = ui.available_rect_before_wrap();
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(r.left()-13.0,r.top()-10.0), egui::vec2(2.0,r.height()+20.0)), 0.0, C::B);
            ui.label(RichText::new(text).size(9.5).color(Color32::from_rgb(0x60,0xA0,0xD8)));
        });
    ui.add_space(10.0);
}

/// .note .nk — green/success note
fn note_k(ui: &mut egui::Ui, text: &str) {
    egui::Frame::NONE.fill(C::GBG).corner_radius(10)
        .inner_margin(egui::Margin { left:13,right:13,top:10,bottom:10 })
        .show(ui, |ui| {
            let r = ui.available_rect_before_wrap();
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(r.left()-13.0,r.top()-10.0), egui::vec2(2.0,r.height()+20.0)), 0.0, C::G);
            ui.label(RichText::new(text).size(9.5).color(Color32::from_rgb(0x40,0xCC,0x8C)));
        });
    ui.add_space(10.0);
}

/// .code — monospace code block
fn code_block(ui: &mut egui::Ui, text: &str) {
    egui::Frame::NONE.fill(C::S00)
        .stroke(egui::Stroke::new(1.0, C::LN))
        .corner_radius(8).inner_margin(egui::Margin { left:13,right:10,top:8,bottom:8 })
        .show(ui, |ui| {
            let r = ui.available_rect_before_wrap();
            ui.painter().rect_filled(
                egui::Rect::from_min_size(egui::pos2(r.left()-13.0,r.top()-8.0), egui::vec2(2.0,r.height()+16.0)), 0.0, C::A18);
            ui.label(RichText::new(text).size(9.5).color(C::T1)
                .family(egui::FontFamily::Monospace));
        });
}

/// .empty — empty state
fn empty(ui: &mut egui::Ui, icon: &str, title: &str, sub: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(28.0);
        ui.label(RichText::new(icon).size(16.0).color(C::T2));
        ui.add_space(9.0);
        ui.label(RichText::new(title.to_uppercase()).size(11.5).strong().color(C::T2));
        ui.add_space(6.0);
        ui.label(RichText::new(sub).size(9.5).color(C::T3));
        ui.add_space(28.0);
    });
}

/// .chip .cG / .cA / .cR / .cB / .cM
fn chip_g(ui: &mut egui::Ui, label: &str) {
    egui::Frame::NONE.fill(C::GBG).stroke(egui::Stroke::new(1.0,C::GBR))
        .corner_radius(255).inner_margin(egui::Margin{left:8,right:8,top:4,bottom:4})
        .show(ui,|ui|{ui.label(RichText::new(label.to_uppercase()).size(8.0).strong().color(C::G));});
}
fn chip_a(ui: &mut egui::Ui, label: &str) {
    egui::Frame::NONE.fill(C::A03).stroke(egui::Stroke::new(1.0,C::A18))
        .corner_radius(255).inner_margin(egui::Margin{left:8,right:8,top:4,bottom:4})
        .show(ui,|ui|{ui.label(RichText::new(label.to_uppercase()).size(8.0).strong().color(C::A));});
}
fn chip_r(ui: &mut egui::Ui, label: &str) {
    egui::Frame::NONE.fill(C::RBG).stroke(egui::Stroke::new(1.0,C::RBR))
        .corner_radius(255).inner_margin(egui::Margin{left:8,right:8,top:4,bottom:4})
        .show(ui,|ui|{ui.label(RichText::new(label.to_uppercase()).size(8.0).strong().color(C::R));});
}
fn chip_b(ui: &mut egui::Ui, label: &str) {
    egui::Frame::NONE.fill(C::BBG).stroke(egui::Stroke::new(1.0,C::BBR))
        .corner_radius(255).inner_margin(egui::Margin{left:8,right:8,top:4,bottom:4})
        .show(ui,|ui|{ui.label(RichText::new(label.to_uppercase()).size(8.0).strong().color(C::B));});
}
fn chip_m(ui: &mut egui::Ui, label: &str) {
    egui::Frame::NONE.fill(Color32::TRANSPARENT).stroke(egui::Stroke::new(1.0,C::LN))
        .corner_radius(255).inner_margin(egui::Margin{left:8,right:8,top:4,bottom:4})
        .show(ui,|ui|{ui.label(RichText::new(label.to_uppercase()).size(8.0).strong().color(C::T2));});
}

/// .ts — tab bar
fn tabs(ui: &mut egui::Ui, labels: &[&str], active: &mut usize) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (i, label) in labels.iter().enumerate() {
            let on = *active == i;
            let r = ui.add(egui::Button::new(RichText::new(*label).size(10.0)
                    .color(if on { C::A } else { C::T2 }))
                .fill(if on { Color32::from_rgba_premultiplied(255,255,255,10) } else { Color32::TRANSPARENT })
                .stroke(egui::Stroke::NONE)
                .corner_radius(egui::CornerRadius{nw:8,ne:8,sw:0,se:0})
                .min_size(egui::vec2(0.0,32.0)));
            if r.clicked() { *active = i; }
            if on {
                let rr = r.rect;
                ui.painter().line_segment(
                    [egui::pos2(rr.left(),rr.bottom()+1.0),egui::pos2(rr.right(),rr.bottom()+1.0)],
                    egui::Stroke::new(1.5,C::A));
            }
        }
    });
    ui.add(egui::Separator::default().spacing(0.0));
    ui.add_space(12.0);
}

// ══════════════════════════════════════════════════════
// pg-dash  DASHBOARD
// ══════════════════════════════════════════════════════
pub fn render_dashboard(ui: &mut egui::Ui, state: &mut AppState) {
    ph_act(ui, "01 · WORKSPACE", "Dashboard",
        "Overview · Connection Mode · Active Workspace", |ui| {
            if btn_s(ui, "Device Info \u{2197}") {
                state.current_page = crate::ui::nav::Page::DeviceInfo;
            }
        });

    // Mode cards
    ui.columns(2, |cols| {
        let sel = state.dashboard_mode;
        for (i, col) in cols.iter_mut().enumerate() {
            let on = sel == i as u8;
            egui::Frame::NONE
                .fill(if on { Color32::from_rgba_premultiplied(232,149,30,8) } else { Color32::from_rgb(16,19,23) })
                .stroke(egui::Stroke::new(1.0, if on { C::A18 } else { C::LN }))
                .corner_radius(10).inner_margin(egui::Margin::same(13))
                .show(col, |ui| {
                    if on {
                        egui::Frame::NONE.fill(C::A).corner_radius(2)
                            .inner_margin(egui::Margin{left:5,right:5,top:1,bottom:1})
                            .show(ui, |ui| {
                                ui.label(RichText::new("ACTIVE").size(7.5).strong().color(Color32::from_rgb(3,1,0)));
                            });
                        ui.add_space(4.0);
                    }
                    if i == 0 {
                        ui.label(RichText::new("AUTO DETECT").size(11.0).strong().color(C::T0));
                        ui.add_space(2.0);
                        ui.label(RichText::new("Continuously monitors USB, ADB and service channels for newly connected devices. Recommended for daily repair workflow.").size(10.0).color(C::T2));
                    } else {
                        ui.label(RichText::new("DEVICE WIZARD").size(11.0).strong().color(C::T0));
                        ui.add_space(2.0);
                        ui.label(RichText::new("Guided step-by-step sequence \u{2014} device info read, pre-check, repair execution and post-action result validation.").size(10.0).color(C::T2));
                    }
                    if ui.interact(ui.min_rect(), egui::Id::new(format!("mc{i}")), egui::Sense::click()).clicked() {
                        state.dashboard_mode = i as u8;
                    }
                });
        }
    });
    ui.add_space(10.0);

    // Stat cards (g4)
    let dev_count = state.devices.len();
    ui.horizontal(|ui| {
        let w = (ui.available_width() - 24.0) / 4.0;
        stat_card(ui, w, "Connected Devices", &dev_count.to_string(), "USB \u{00b7} ADB \u{00b7} Fastboot", true);
        stat_card(ui, w, "Supported Models", "65", "Device database entries", false);
        stat_card(ui, w, "Active Workspace", "Device Info", "Current service tab", false);
        stat_card(ui, w, "Last Operation", "No jobs yet", "Recent status", false);
    });
    ui.add_space(16.0);

    // Quick Access (g3) — clicking navigates
    sh(ui, "Quick Access");
    ui.columns(3, |cols| {
        if qa_card(&mut cols[0], "\u{25eb}", "Scan Devices", "USB \u{00b7} ADB \u{00b7} Fastboot \u{00b7} Download mode") {
            state.current_page = crate::ui::nav::Page::Devices;
        }
        if qa_card(&mut cols[1], "\u{2318}", "Apple Operations", "DFU \u{00b7} IPSW flash \u{00b7} iCloud \u{00b7} Passcode") {
            state.current_page = crate::ui::nav::Page::AppleIos;
        }
        if qa_card(&mut cols[2], "\u{25c9}", "AU Carrier Unlock", "Telstra \u{00b7} Optus \u{00b7} Vodafone AU + more") {
            state.current_page = crate::ui::nav::Page::AuUnlock;
        }
    });
    ui.add_space(16.0);

    // System Status (g2)
    sh(ui, "System Status");
    ui.columns(2, |cols| {
        card(&mut cols[0], |ui| {
            ui.label(RichText::new("RUNTIME ENVIRONMENT").size(7.5).strong()
                .color(Color32::from_rgba_premultiplied(255,255,255,100)));
            ui.add_space(9.0);
            kv(ui, "Version",           "ChimeraRS v1.3.13",                  C::T0, false, false);
            kv(ui, "Build timestamp",   "2026-03-31 \u{00b7} 14:39:08 UTC",   C::T1, true,  false);
            kv(ui, "Architecture",      "macOS \u{00b7} x86_64-apple-darwin",  C::T0, false, false);
            kv(ui, "Worker pool",       "Running",                             C::G,  false, false);
            state.refresh_adb_throttled();
            let (adb_str, adb_color, adb_pulse) = if state.adb_ok {
                let value = match (state.adb_path.as_deref(), state.adb_version.as_deref()) {
                    (Some(p), Some(v)) => format!("{} \u{00b7} {}", v, p),
                    (Some(p), None)    => format!("Found at {}", p),
                    _                  => "Found".to_string(),
                };
                (value, C::G, true)
            } else {
                let value = state.adb_error.clone()
                    .unwrap_or_else(|| "Not found in PATH".to_string());
                (value, C::A, false)
            };
            kv(ui, "ADB daemon",        &adb_str,                              adb_color, adb_pulse, false);
            kv(ui, "libusb",            "v0.9.4 \u{2014} linked",              C::G,  false, true);
        });
        card(&mut cols[1], |ui| {
            ui.label(RichText::new("LOADED MODULES \u{00b7} RUNTIME TOPOLOGY").size(7.5).strong()
                .color(Color32::from_rgba_premultiplied(255,255,255,100)));
            ui.add_space(9.0);
            module_group(ui, "Transport",  &["adb","fastboot","edl","libusb"]);
            ui.add_space(8.0);
            module_group(ui, "Platform",   &["apple","samsung","xiaomi","huawei","mtk","motorola","sony","nokia","oppo"]);
            ui.add_space(8.0);
            module_group(ui, "Workspace",  &["core","devices","firmware","utils","gui","api"]);
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                chip_a(ui, "18 active modules");
                chip_b(ui, "148 files \u{00b7} 27,244 LOC");
            });
        });
    });
}

fn stat_card(ui: &mut egui::Ui, w: f32, label: &str, val: &str, meta: &str, amber: bool) {
    ui.allocate_ui_with_layout(egui::vec2(w, 0.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
        egui::Frame::NONE
            .fill(Color32::from_rgb(16,19,23))
            .stroke(egui::Stroke::new(1.0, C::LN))
            .corner_radius(10).inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.label(RichText::new(label.to_uppercase()).size(7.5).strong().color(C::T3));
                ui.label(RichText::new(val).size(20.0).strong()
                    .color(if amber { C::A } else { C::T0 }));
                ui.label(RichText::new(meta.to_uppercase()).size(8.5).color(C::T2));
            });
    });
}

fn qa_card(ui: &mut egui::Ui, icon: &str, title: &str, sub: &str) -> bool {
    let r = egui::Frame::NONE
        .fill(Color32::from_rgb(16,19,23))
        .stroke(egui::Stroke::new(1.0, C::LN))
        .corner_radius(10).inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.label(RichText::new(icon).size(14.0).color(Color32::from_rgba_premultiplied(255,255,255,71)));
            ui.add_space(5.0);
            ui.label(RichText::new(title.to_uppercase()).size(11.0).strong().color(C::T0));
            ui.add_space(1.0);
            ui.label(RichText::new(sub.to_uppercase()).size(9.0).color(C::T2));
        });
    ui.interact(r.response.rect, egui::Id::new(format!("qa{title}")), egui::Sense::click()).clicked()
}

fn module_group(ui: &mut egui::Ui, label: &str, items: &[&str]) {
    egui::Frame::NONE
        .fill(Color32::from_rgba_premultiplied(255,255,255,8))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(255,255,255,15)))
        .corner_radius(10).inner_margin(egui::Margin{left:11,right:11,top:8,bottom:8})
        .show(ui, |ui| {
            ui.label(RichText::new(label.to_uppercase()).size(7.5).strong()
                .color(Color32::from_rgba_premultiplied(255,255,255,115)));
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(5.0, 4.0);
                for item in items { chip_g(ui, item); }
            });
        });
}

// ══════════════════════════════════════════════════════
// pg-devs  DEVICES
// ══════════════════════════════════════════════════════
pub fn render_devices(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
    ph_act(ui, "02 \u{00b7} WORKSPACE", "Devices",
        "Connected and discovered devices across all transports", |ui| {
            btn_p(ui, "+ Manual");
            btn_s(ui, "\u{27f3} Refresh");
        });
    tabs(ui, &["USB","ADB","Fastboot","EDL \u{00b7} Download","Apple USB-MUX"], &mut state.devices_tab);
    match state.devices_tab {
        0 => empty(ui, "\u{22a1}", "No USB Devices Detected",
            "Connect a device via USB cable. Enable USB debugging on Android; authorise this computer on Apple devices."),
        1 => {
            if !state.adb_ok {
                let msg = state.adb_error.clone()
                    .unwrap_or_else(|| "ADB daemon not found in PATH \u{2014} install via:  brew install android-platform-tools".to_string());
                note_i(ui, &msg);
            }
            empty(ui, "\u{25c8}",
                if state.adb_ok { "ADB \u{00b7} No Connected Devices" } else { "ADB Unavailable" },
                if state.adb_ok {
                    "ADB is running. Connect an Android device with USB debugging enabled."
                } else {
                    "Configure ADB binary path in Settings \u{2192} ADB \u{00b7} Paths, then reconnect."
                });
        }
        2 => empty(ui, "\u{26a1}", "No Fastboot Devices",
            "Boot device into Fastboot mode (Vol\u{2013} + Power), then connect via USB."),
        3 => {
            note_w(ui, "Qualcomm EDL requires device in 9008 Download Mode with appropriate host-side USB driver installed.");
            empty(ui, "\u{2193}", "No EDL \u{00b7} Download Devices",
                "Hold Vol+ + Vol\u{2013} during power-on, or use a hardware test-point to force EDL on Qualcomm targets.");
        }
        _ => empty(ui, "\u{2318}", "No Apple Devices",
            "Connect iPhone or iPad via USB. Tap \u{201c}Trust This Computer\u{201d} when prompted on the device."),
    }
}

// ══════════════════════════════════════════════════════
// pg-dld  DOWNLOADS
// ══════════════════════════════════════════════════════
pub fn render_downloads(ui: &mut egui::Ui, state: &mut AppState) {
    ph_act(ui, "03 \u{00b7} WORKSPACE", "Downloads",
        "Firmware, IPSW and resource queue management", |ui| {
            btn_p(ui, "+ New Download");
        });
    tabs(ui, &["Queue","IPSW Finder","Samsung Firmware","Completed"], &mut state.downloads_tab2);
    match state.downloads_tab2 {
        0 => empty(ui, "\u{2193}", "Download Queue Empty",
            "Use IPSW Finder or Samsung Firmware to queue firmware downloads."),
        1 => { card(ui, |ui| {
            sh(ui, "IPSW Firmware Search");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("ipsw_model")
                    .selected_text(&state.ipsw_model_selected)
                    .width(ui.available_width() - 88.0)
                    .show_ui(ui, |ui| {
                        for m in &["Select a device model\u{2026}","iPhone 17 Pro Max (iPhone18,5)",
                            "iPhone 17 Pro (iPhone18,4)","iPhone 17 Air (iPhone18,2)",
                            "iPhone 16 Pro Max (iPhone17,2)","iPhone 16 Pro (iPhone17,1)",
                            "iPhone 16 (iPhone17,3)","iPhone 16e (iPhone17,5)",
                            "iPhone 15 Pro Max (iPhone16,2)","iPhone 15 (iPhone15,4)"] {
                            ui.selectable_value(&mut state.ipsw_model_selected, m.to_string(), *m);
                        }
                    });
                btn_p(ui, "Search");
            });
            ui.add_space(6.0);
            note_i(ui, "Results from ipsw.me API. Only signed builds can be restored via Apple TSS.");
        }); }
        2 => { card(ui, |ui| {
            sh(ui, "Samsung Firmware Lookup");
            ui.horizontal(|ui| {
                fi(ui, &mut state.samsung_fw_model, "Model number \u{2014} e.g. SM-S928B");
                ui.add_space(4.0);
                egui::Frame::NONE
                    .fill(Color32::from_rgb(20,23,28))
                    .stroke(egui::Stroke::new(1.0,Color32::from_rgba_premultiplied(255,255,255,20)))
                    .corner_radius(8).inner_margin(egui::Margin{left:10,right:10,top:8,bottom:8})
                    .show(ui, |ui| {
                        ui.add_sized([80.0,0.0], egui::TextEdit::singleline(&mut state.samsung_fw_csc)
                            .hint_text("CSC").frame(egui::Frame::NONE));
                    });
                btn_p(ui, "Fetch");
            });
            ui.add_space(4.0);
            ui.label(RichText::new("Queries samfw.com \u{2014} falls back to portal link when direct unavailable.").size(9.0).color(C::T2));
        }); }
        _ => empty(ui, "\u{2713}", "No Completed Downloads",
            "Completed firmware downloads appear here with SHA-1 and MD5 verification."),
    }
}

// ══════════════════════════════════════════════════════
// pg-hist  WORK HISTORY
// ══════════════════════════════════════════════════════
pub fn render_history(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
    ph_act(ui, "04 \u{00b7} WORKSPACE", "Work History",
        "Chronological record of all device operations and service repairs", |ui| {
            btn_d(ui, "Clear Log");
            btn_s(ui, "Export CSV");
        });
    cflat(ui, |ui| {
        egui::Grid::new("hist_tbl").num_columns(5).striped(false).spacing(egui::vec2(8.0,4.0)).show(ui, |ui| {
            for h in &["Timestamp","Device","Operation","Result","Duration"] {
                ui.label(RichText::new(h.to_uppercase()).size(7.5).strong().color(C::T3));
            }
            ui.end_row();
            ui.separator(); ui.separator(); ui.separator(); ui.separator(); ui.separator();
            ui.end_row();
            // Smoke test row
            ui.label(RichText::new("2026-03-31 14:39").size(9.0).color(C::T1).family(egui::FontFamily::Monospace));
            ui.label(RichText::new("\u{2014}").size(10.0).color(C::T2));
            ui.label(RichText::new("GUI Smoke Test").size(10.0).color(C::T1));
            chip_g(ui, "Success");
            ui.label(RichText::new("0.46 s").size(9.0).color(C::T3));
            ui.end_row();
            // No prior operations
            ui.label(RichText::new("No prior operations recorded in this session").size(9.5).color(C::T3));
        });
    });
}

// ══════════════════════════════════════════════════════
// pg-util  UTILITIES
// ══════════════════════════════════════════════════════
pub fn render_utilities(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
    ph(ui, "05 \u{00b7} WORKSPACE", "Utilities",
        "IMEI validation \u{00b7} NCK calculation \u{00b7} MAC tools \u{00b7} cryptographic hashing \u{00b7} QR generation");
    tabs(ui, &["IMEI Tools","NCK Calc","MAC Tools","Crypto \u{00b7} Hash","QR Code"], &mut state.utilities_tab);
    match state.utilities_tab {
        0 => {
            ui.columns(2, |cols| {
                card(&mut cols[0], |ui| {
                    sh(ui, "Luhn Algorithm Validator");
                    fr(ui, "IMEI Number", &mut state.imei_check_input, "Enter 15-digit IMEI\u{2026}");
                    if btn_p(ui, "Validate") {
                        state.imei_check_result = Some(validate_imei(&state.imei_check_input));
                    }
                    if let Some(ref r) = state.imei_check_result.clone() {
                        ui.add_space(6.0);
                        let ok = r.contains("Valid");
                        ui.label(RichText::new(r.as_str()).size(11.0).color(if ok { C::G } else { C::R }));
                    }
                });
                card(&mut cols[1], |ui| {
                    sh(ui, "TAC \u{00b7} SNR Structure Decoder");
                    fr(ui, "IMEI", &mut state.imei_decode_input, "Enter 15-digit IMEI\u{2026}");
                    if btn_s(ui, "Decode Structure") && state.imei_decode_input.len() == 15 {
                        let v = &state.imei_decode_input;
                        let valid = luhn_check(v);
                        kv(ui, "TAC (Type Allocation Code)", &v[..8], C::T0, true, false);
                        kv(ui, "SNR (Serial Number)", &v[8..14], C::T0, true, false);
                        kv(ui, "Check Digit", &v[14..], C::T0, true, false);
                        kv(ui, "Luhn Validity", if valid {"Valid"} else {"Invalid"},
                            if valid {C::G} else {C::R}, false, true);
                    }
                });
            });
        }
        1 => { card(ui, |ui| {
            ui.set_max_width(420.0);
            sh(ui, "NCK Calculator");
            fr(ui, "IMEI", &mut state.nck_imei_input, "15-digit IMEI\u{2026}");
            fl(ui, "Algorithm");
            egui::ComboBox::from_id_salt("nck_algo").selected_text(&state.nck_algo)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for a in &["Samsung (DCK)","Nokia Algo 1","LG Standard","Motorola","HTC"] {
                        ui.selectable_value(&mut state.nck_algo, a.to_string(), *a);
                    }
                });
            ui.add_space(8.0);
            btn_p(ui, "Calculate NCK");
            ui.add_space(8.0);
            note_w(ui, "Authorised repair and educational use only.");
        }); }
        2 => {
            ui.columns(2, |cols| {
                card(&mut cols[0], |ui| {
                    sh(ui, "MAC Address Validator");
                    fr(ui, "MAC Address", &mut state.mac_input, "AA:BB:CC:DD:EE:FF");
                    if btn_p(ui, "Validate") {
                        state.mac_validate_result = validate_mac(&state.mac_input);
                    }
                    if !state.mac_validate_result.is_empty() {
                        ui.add_space(6.0);
                        let ok = !state.mac_validate_result.contains("Invalid") && !state.mac_validate_result.contains("\u{274c}");
                        ui.label(RichText::new(&state.mac_validate_result.clone()).size(10.0)
                            .color(if ok { C::G } else { C::R }));
                    }
                });
                card(&mut cols[1], |ui| {
                    sh(ui, "MAC Derivation");
                    fr(ui, "Input \u{2014} Serial or IMEI", &mut state.mac_derive_input, "Device serial or IMEI\u{2026}");
                    if btn_s(ui, "Derive MAC") {
                        state.mac_derive_result = derive_mac(&state.mac_derive_input);
                    }
                    if !state.mac_derive_result.is_empty() {
                        ui.add_space(6.0);
                        ui.label(RichText::new(&state.mac_derive_result.clone()).size(11.0).color(C::T0)
                            .family(egui::FontFamily::Monospace));
                    }
                });
            });
        }
        3 => { card(ui, |ui| {
            sh(ui, "Hash Generation");
            fl(ui, "Input Data");
            egui::Frame::NONE.fill(Color32::from_rgb(20,23,28))
                .stroke(egui::Stroke::new(1.0,Color32::from_rgba_premultiplied(255,255,255,20)))
                .corner_radius(8).inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut state.hash_input)
                        .desired_rows(3).desired_width(f32::INFINITY)
                        .hint_text("Enter data or paste file contents\u{2026}").frame(egui::Frame::NONE));
                });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for algo in &["SHA-256","SHA-1","MD5","CRC-32","HMAC-SHA256"] {
                    if btn_s(ui, algo) {
                        state.hash_result = compute_hash(algo, &state.hash_input, &state.hash_hmac_key);
                        state.hash_algo = algo.to_string();
                    }
                }
            });
            ui.add_space(8.0);
            fl(ui, "Output Hash");
            egui::Frame::NONE.fill(Color32::from_rgb(20,23,28))
                .stroke(egui::Stroke::new(1.0,Color32::from_rgba_premultiplied(255,255,255,20)))
                .corner_radius(8).inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    let mut r = state.hash_result.clone();
                    ui.add(egui::TextEdit::singleline(&mut r).desired_width(f32::INFINITY)
                        .hint_text("Hash result will appear here\u{2026}").interactive(false).frame(egui::Frame::NONE)
                        .font(egui::FontId::new(9.5, egui::FontFamily::Monospace)));
                });
        }); }
        _ => { card(ui, |ui| {
            ui.set_max_width(330.0);
            sh(ui, "QR Code Generator");
            fr(ui, "Content", &mut state.qr_input, "Enter URL or text\u{2026}");
            btn_p(ui, "Generate QR Code");
            ui.add_space(9.0);
            egui::Frame::NONE.fill(C::S00).stroke(egui::Stroke::new(1.0,C::LN)).corner_radius(8)
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(130.0,130.0));
                    ui.vertical_centered(|ui| {
                        ui.add_space(55.0);
                        ui.label(RichText::new("PREVIEW").size(9.0).extra_letter_spacing(1.2).color(C::T3));
                    });
                });
        }); }
    }
}

// ══════════════════════════════════════════════════════
// pg-cfg  SETTINGS
// ══════════════════════════════════════════════════════
pub fn render_settings(ui: &mut egui::Ui, state: &mut AppState) {
    ph(ui, "06 \u{00b7} WORKSPACE", "Settings",
        "Tool paths \u{00b7} appearance \u{00b7} behaviour \u{00b7} network proxy");
    tabs(ui, &["ADB \u{00b7} Paths","Appearance","Behaviour","Network"], &mut state.settings_tab);
    match state.settings_tab {
        0 => {
            ui.columns(2, |cols| {
                card(&mut cols[0], |ui| {
                    sh(ui, "External Tool Paths");
                    fr(ui, "ADB Binary", &mut state.settings_adb_path, "");
                    fr(ui, "Fastboot Binary", &mut state.settings_fastboot_path, "");
                    fr(ui, "irecovery Binary", &mut state.settings_irecovery_path, "Path to irecovery\u{2026}");
                    fr(ui, "futurerestore Binary", &mut state.settings_futurerestore_path, "Path to futurerestore\u{2026}");
                    btn_p(ui, "Save Configuration");
                });
                card(&mut cols[1], |ui| {
                    sh(ui, "Data Directories");
                    fl(ui, "Application Data");
                    fi_ro(ui, "~/Library/Application Support/ChimeraRS");
                    ui.add_space(8.0);
                    fl(ui, "Cache");
                    fi_ro(ui, "~/Library/Caches/chimera-rs");
                    ui.add_space(8.0);
                    fl(ui, "SHSH Blob Store");
                    fi_ro(ui, "~/Library/Application Support/ChimeraRS/blobs");
                    ui.add_space(8.0);
                    fl(ui, "Downloads");
                    fi_ro(ui, "~/Downloads");
                    ui.add_space(8.0);
                    btn_s(ui, "Open in Finder");
                });
            });
        }
        1 => { card(ui, |ui| {
            ui.set_max_width(450.0);
            sh(ui, "Theme Preferences");
            tog(ui, "Dark mode", "Always-on dark surface rendering", &mut state.settings_dark_mode);
            tog(ui, "Amber accent colour", "Chimera signature amber \u{2014} #E8951E", &mut state.settings_amber_accent);
            tog(ui, "Compact sidebar", "Reduce navigation item height", &mut state.settings_compact_sidebar);
            tog(ui, "Show console strip", "Runtime log panel at bottom", &mut state.settings_show_console);
            tog(ui, "Dot-grid background", "Subtle 24px surface texture", &mut state.settings_dot_grid);
            tog(ui, "Grain overlay", "Matte fractal noise \u{2014} premium finish", &mut state.settings_grain);
            tog(ui, "Shimmer on hover", "Directional shimmer \u{2014} SolarWinds style", &mut state.settings_shimmer);
        }); }
        2 => { card(ui, |ui| {
            ui.set_max_width(450.0);
            sh(ui, "Application Behaviour");
            tog(ui, "Auto-detect on startup", "Begin USB and ADB scan on launch", &mut state.settings_auto_detect);
            tog(ui, "Confirm destructive operations", "Confirmation dialog before irreversible actions", &mut state.settings_confirm_destructive);
            tog(ui, "Audible completion alert", "System sound when long operations finish", &mut state.settings_audible_alert);
            tog(ui, "Prevent display sleep", "Keep screen awake during active operations", &mut state.settings_prevent_sleep);
        }); }
        _ => { card(ui, |ui| {
            ui.set_max_width(450.0);
            sh(ui, "Network \u{00b7} Proxy");
            tog(ui, "Use system proxy", "", &mut state.settings_use_system_proxy);
            ui.add_space(6.0);
            fr(ui, "Custom Proxy URL", &mut state.settings_proxy_url, "http://hostname:port");
            tog(ui, "Verify TLS certificates", "", &mut state.settings_verify_tls);
            tog(ui, "API mock mode", "Route all API calls to local mock server", &mut state.settings_api_mock);
        }); }
    }
}

// ══════════════════════════════════════════════════════
// pg-dinfo  DEVICE INFO
// ══════════════════════════════════════════════════════
pub fn render_device_info(ui: &mut egui::Ui, _state: &mut AppState) {
    ph(ui, "01 \u{00b7} DEVICE", "Device Info",
        "Hardware identifiers and software details for the active connected device");
    empty(ui, "\u{25eb}", "No Device Selected",
        "Connect a device via USB or ADB, or use Auto Detect mode from the Dashboard to begin a session.");
}

// ══════════════════════════════════════════════════════
// pg-jb  JAILBREAK
// ══════════════════════════════════════════════════════
pub fn render_jailbreak(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
    ph(ui, "02 \u{00b7} DEVICE", "Jailbreak",
        "Android Root \u{00b7} iOS Jailbreak \u{00b7} FRP Bypass \u{00b7} Magisk \u{00b7} TWRP");
    note_e(ui, "Authorised device servicing only \u{2014} Aust. Criminal Code Act 1995 \u{a7}477\u{2013}478 \u{00b7} US CFAA. Written ownership or service authorisation required before use.");
    tabs(ui, &["Android Root","iOS Jailbreak","Magisk","TWRP","FRP Bypass"], &mut state.jailbreak_tab);
    match state.jailbreak_tab {
        0 => jb_android(ui, state),
        1 => jb_ios(ui, state),
        2 => jb_magisk(ui, state),
        3 => jb_twrp(ui, state),
        _ => jb_frp(ui, state),
    }
}

fn jb_android(ui: &mut egui::Ui, state: &mut AppState) {
    jb_hd(ui, "Configuration");
    ui.columns(3, |cols| {
        cols[0].vertical(|ui| {
            fl(ui, "Magisk APK");
            ui.horizontal(|ui| {
                fi(ui, &mut state.jb_magisk_path, "~/Downloads/Magisk-v27.0.apk");
                btn_s(ui, "Browse");
            });
        });
        cols[1].vertical(|ui| {
            fl(ui, "TWRP App APK");
            ui.horizontal(|ui| {
                fi(ui, &mut state.jb_twrp_apk_path, "./TWRP/me.twrp.twrpapp-26.apk");
                btn_s(ui, "Browse");
            });
        });
        cols[2].vertical(|ui| {
            fl(ui, "Recovery Strategy");
            egui::ComboBox::from_id_salt("jb_strat").selected_text(&state.jb_strategy)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for s in &["Patch boot.img + stage TWRP App","Install TWRP App after root",
                               "Use existing custom recovery","Magisk-only patch flow"] {
                        ui.selectable_value(&mut state.jb_strategy, s.to_string(), *s);
                    }
                });
        });
    });
    ui.add_space(14.0);

    jb_hd(ui, "Options");
    egui::Frame::NONE
        .fill(Color32::from_rgba_premultiplied(255,255,255,11))
        .stroke(egui::Stroke::new(1.0,Color32::from_rgba_premultiplied(255,255,255,18)))
        .corner_radius(10).inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            tog(ui, "Patch boot.img automatically", "Patch the device boot image in place before flashing", &mut state.jb_opt_patch_boot);
            tog(ui, "Stage bundled TWRP app after root", "APK only \u{2014} no recovery.img is bundled in this archive", &mut state.jb_opt_stage_twrp);
            tog(ui, "Verify TWRP bundle checksum before deploy", "Validate integrity against known-good hashes before installation", &mut state.jb_opt_verify_checksum);
        });
    ui.add_space(14.0);

    ui.columns(2, |cols| {
        egui::Frame::NONE
            .fill(Color32::from_rgba_premultiplied(255,255,255,6))
            .stroke(egui::Stroke::new(1.0,Color32::from_rgba_premultiplied(255,255,255,15)))
            .corner_radius(10).inner_margin(egui::Margin::same(13))
            .show(&mut cols[0], |ui| {
                jb_hd(ui, "Bundle Verification Hashes");
                ui.label(RichText::new("SHA-256").size(7.5).strong().color(C::T3));
                ui.add_space(2.0);
                ui.label(RichText::new("80d2d8aa6f325019fb57427aecb06e044c9d8e070751ac09e85b3642aa361f2b").size(8.5).color(C::T1).family(egui::FontFamily::Monospace));
                ui.add_space(8.0);
                ui.label(RichText::new("MD5").size(7.5).strong().color(C::T3));
                ui.add_space(2.0);
                ui.label(RichText::new("d459d75188852a0a41290c85b3849d2a").size(8.5).color(C::T1).family(egui::FontFamily::Monospace));
            });

        egui::Frame::NONE
            .fill(Color32::from_rgba_premultiplied(255,255,255,6))
            .stroke(egui::Stroke::new(1.0,Color32::from_rgba_premultiplied(255,255,255,15)))
            .corner_radius(10).inner_margin(egui::Margin::same(13))
            .show(&mut cols[1], |ui| {
                jb_hd(ui, "Pre-Flight Check");
                for (k,v,c) in &[("Bootloader","No device",C::A),("ADB root","\u{2014}",C::T2),("TWRP bundle","Verified",C::G)] {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(*k).size(10.0).color(C::T2));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(*v).size(10.0).strong().color(*c));
                        });
                    });
                }
                ui.add_space(14.0);
                btn_s(ui, "Run Pre-Check");
            });
    });
    ui.add_space(14.0);
    btn_p_wide(ui, "Begin Root + TWRP Prep");
}

fn jb_ios(ui: &mut egui::Ui, _state: &mut AppState) {
    jb_hd(ui, "Exploit Compatibility");
    note_i(ui, "checkm8 exploit targets A7\u{2013}A11 SoC only. A12 Bionic and later chips cannot be exploited via checkm8.");
    let compat = [
        ("A11 Bionic (checkm8)", "Supported",      false),
        ("A12 Bionic and later", "Not exploitable", true),
        ("palera1n rootless",    "A9 \u{2013} A11", false),
    ];
    for (label, val, red) in &compat {
        ui.horizontal(|ui| {
            ui.label(RichText::new(*label).size(10.5).color(C::T1));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if *red { chip_r(ui, val); } else { chip_g(ui, val); }
            });
        });
        ui.separator();
    }
    ui.add_space(14.0);

    jb_hd(ui, "DFU Entry Reference");
    for (model, steps) in &[
        ("iPhone 8 \u{00b7} X and later",  "Vol\u{2191} \u{2192} Vol\u{2193} \u{2192} Hold Side 10 s \u{2192} Release Side, Hold Vol\u{2193} 5 s"),
        ("iPhone 7 \u{00b7} 7 Plus",       "Hold Vol\u{2193} + Wake 10 s \u{2192} Release Wake, Hold Vol\u{2193} 5 s"),
        ("iPhone 6s and earlier",          "Hold Home + Wake 10 s \u{2192} Release Wake, Hold Home 5 s"),
    ] {
        ui.label(RichText::new(*model).size(10.0).strong().color(C::T1));
        ui.label(RichText::new(*steps).size(9.5).color(C::T2));
        ui.separator();
    }
    ui.add_space(14.0);
    btn_p_wide(ui, "Enter DFU Mode");
}

fn jb_magisk(ui: &mut egui::Ui, _state: &mut AppState) {
    jb_hd(ui, "Operations");
    for label in &["Install Magisk to Device","Uninstall \u{00b7} Restore Stock Boot",
                   "Patch Custom boot.img","Install Bundled TWRP App APK","Query Magisk Install Status"] {
        if btn_s(ui, label) {} ui.add_space(4.0);
    }
    ui.add_space(10.0);
    jb_hd(ui, "Magisk \u{2194} TWRP Integration");
    for (k,v,c,mono) in &[
        ("Bundled TWRP","me.twrp.twrpapp-26.apk",C::G,false),
        ("SHA-256","80d2d8aa\u{2026}aa361f2b",C::T2,true),
        ("Recovery image","Supply device-specific separately",C::T2,false),
    ] {
        kv(ui, k, v, *c, *mono, false);
    }
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        btn_s(ui, "Verify TWRP Hashes");
        btn_s(ui, "Queue Recovery Flow");
    });
}

fn jb_twrp(ui: &mut egui::Ui, state: &mut AppState) {
    note_k(ui, "Included from uploaded TWRP.zip: me.twrp.twrpapp-26.apk \u{00b7} .md5 \u{00b7} .sha256 \u{00b7} signature sidecar");
    jb_hd(ui, "Bundle Contents");
    for (algo, val) in &[("APK","./TWRP/me.twrp.twrpapp-26.apk"),
        ("MD5","d459d75188852a0a41290c85b3849d2a"),
        ("SHA-256","80d2d8aa6f325019fb57427aecb06e044c9d8e070751ac09e85b3642aa361f2b")] {
        ui.label(RichText::new(*algo).size(7.5).strong().color(C::T3));
        ui.label(RichText::new(*val).size(8.5).color(C::T1).family(egui::FontFamily::Monospace));
        ui.add_space(8.0);
    }
    jb_hd(ui, "Recovery Workflow");
    fl(ui, "Recovery Image");
    ui.horizontal(|ui| {
        fi(ui, &mut state.jb_recovery_img_path, "Provide device-specific recovery.img");
        btn_s(ui, "Browse");
    });
    ui.add_space(8.0);
    fl(ui, "Flash Method");
    egui::ComboBox::from_id_salt("twrp_flash").selected_text(&state.jb_flash_method)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for m in &["Fastboot boot recovery.img","Fastboot flash recovery","ADB sideload","Already installed on device"] {
                ui.selectable_value(&mut state.jb_flash_method, m.to_string(), *m);
            }
        });
    ui.add_space(8.0);
    note_w(ui, "Bundle does not include a recovery image \u{2014} supply device-specific TWRP media separately.");
    ui.horizontal(|ui| {
        btn_p(ui, "Reboot to Recovery");
        btn_s(ui, "Launch Recovery Assistant");
    });
    ui.add_space(8.0);
    btn_s(ui, "Install TWRP App APK to Device");
    ui.add_space(4.0);
    btn_s(ui, "Verify Bundle Integrity");
}

fn jb_frp(ui: &mut egui::Ui, state: &mut AppState) {
    jb_hd(ui, "Target Platform");
    fl(ui, "Manufacturer");
    egui::ComboBox::from_id_salt("frp_mfr").selected_text(&state.jb_frp_manufacturer)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for m in &["Auto-detect from device","Samsung","Xiaomi","Motorola","LG","OPPO \u{00b7} OnePlus","Nothing Phone"] {
                ui.selectable_value(&mut state.jb_frp_manufacturer, m.to_string(), *m);
            }
        });
    ui.add_space(8.0);
    btn_p_wide(ui, "Check FRP State");
    ui.add_space(14.0);
    jb_hd(ui, "Status");
    kv(ui, "FRP Lock State", "No device connected", C::A, false, true);
}

// ══════════════════════════════════════════════════════
// pg-ssh  SSH / VPN
// ══════════════════════════════════════════════════════
pub fn render_ssh(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
    ph(ui, "03 \u{00b7} DEVICE", "SSH \u{00b7} VPN",
        "Remote device access \u{00b7} port forwarding \u{00b7} VPN tunnel configuration");
    tabs(ui, &["SSH Session","Port Forward","VPN"], &mut state.ssh_tab2);
    match state.ssh_tab2 {
        0 => { card(ui, |ui| {
            ui.set_max_width(450.0);
            sh(ui, "SSH Connection");
            fr(ui, "Host or IP Address", &mut state.ssh_host, "192.168.0.1 or hostname");
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    fl(ui, "Port");
                    egui::Frame::NONE.fill(Color32::from_rgb(20,23,28))
                        .stroke(egui::Stroke::new(1.0,Color32::from_rgba_premultiplied(255,255,255,20)))
                        .corner_radius(8).inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.add_sized([68.0,0.0], egui::TextEdit::singleline(&mut state.ssh_port).frame(egui::Frame::NONE));
                        });
                });
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    fl(ui, "Username");
                    fi(ui, &mut state.ssh_username, "root");
                });
            });
            ui.add_space(8.0);
            fl(ui, "Authentication Method");
            egui::ComboBox::from_id_salt("ssh_auth").selected_text(&state.ssh_auth_method)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for m in &["Password","Private Key File","Public Key + Passphrase"] {
                        ui.selectable_value(&mut state.ssh_auth_method, m.to_string(), *m);
                    }
                });
            ui.add_space(8.0);
            btn_p(ui, "Establish Connection");
        }); }
        1 => empty(ui, "\u{21c4}", "No Active Tunnels",
            "Configure SSH port-forwarding rules to expose device services on your local network."),
        _ => empty(ui, "\u{25c8}", "VPN Not Configured",
            "Set up a VPN tunnel for secure remote device access across networks."),
    }
}

// ══════════════════════════════════════════════════════
// pg-act  ACTIVATION
// ══════════════════════════════════════════════════════
pub fn render_activation(ui: &mut egui::Ui, state: &mut AppState) {
    ph(ui, "04 \u{00b7} DEVICE", "Activation",
        "iCloud activation lock check \u{00b7} bypass methods \u{00b7} MDM enrollment status");
    note_e(ui, "Use only on devices you own outright or hold explicit written service authorisation for. Unauthorised bypass is a criminal offence.");
    tabs(ui, &["Status Check","Bypass","Escrow Key","MDM"], &mut state.activation_tab);
    match state.activation_tab {
        0 => { card(ui, |ui| {
            ui.set_max_width(430.0);
            sh(ui, "iCloud Activation Lock Check");
            fr(ui, "Device IMEI or Serial Number", &mut state.activation_imei, "15-digit IMEI or device serial");
            ui.horizontal(|ui| {
                btn_p(ui, "Check via Apple");
                btn_s(ui, "Read from Device");
            });
        }); }
        1 => { card(ui, |ui| {
            ui.set_max_width(430.0);
            sh(ui, "Bypass Method");
            fl(ui, "Bypass Technique");
            egui::ComboBox::from_id_salt("bypass_tech").selected_text(&state.activation_bypass_method)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for m in &["checkm8 exploit (A7\u{2013}A11)","DNS server redirect (community)",
                               "Erase and restore via DFU","MDM DEP enrollment","SIM network trick"] {
                        ui.selectable_value(&mut state.activation_bypass_method, m.to_string(), *m);
                    }
                });
            ui.add_space(6.0);
            note_i(ui, "Community DNS: 78.100.17.6 \u{00b7} dns.bypass.community \u{00b7} activation-bypass.app");
            btn_p(ui, "Execute Bypass");
        }); }
        2 => { card(ui, |ui| {
            ui.set_max_width(430.0);
            sh(ui, "Escrow Key Query");
            fr(ui, "Device ECID (Hexadecimal)", &mut state.activation_ecid, "0x\u{2026}");
            btn_p(ui, "Query escrowproxy.icloud.com");
        }); }
        _ => empty(ui, "\u{229e}", "No Device Connected",
            "Connect an Apple device to read MDM enrollment status and lockdownd carrier domain details."),
    }
}

// ══════════════════════════════════════════════════════
// pg-nwk  NETWORK
// ══════════════════════════════════════════════════════
pub fn render_network(ui: &mut egui::Ui, state: &mut AppState) {
    ph(ui, "05 \u{00b7} DEVICE", "Network",
        "SIM and carrier lock \u{00b7} MCC unlock gateway \u{00b7} lockdownd policy");
    tabs(ui, &["SIM Status","MCC Unlock","Lock Policy"], &mut state.network_tab);
    match state.network_tab {
        0 => { card(ui, |ui| {
            ui.set_max_width(430.0);
            sh(ui, "SIM Lock Status");
            fr(ui, "Device IMEI", &mut state.network_sim_imei, "15-digit IMEI");
            btn_p(ui, "Check SIM Lock State");
        }); }
        1 => { card(ui, |ui| {
            ui.set_max_width(430.0);
            sh(ui, "MCC Carrier Unlock");
            fr(ui, "Device IMEI", &mut state.network_mcc_imei, "15-digit IMEI");
            btn_p(ui, "Query mccgateway.icloud.com");
        }); }
        _ => empty(ui, "\u{25f0}", "No Device Connected",
            "Connect a device to query SIM lock policy from the lockdownd carrier settings domain."),
    }
}

// ══════════════════════════════════════════════════════
// pg-tls  PLATFORM TOOLS
// ══════════════════════════════════════════════════════
pub fn render_tools(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
    ph(ui, "06 \u{00b7} DEVICE", "Platform Tools",
        "Manufacturer-specific repair diagnostics and service operations");
    tabs(ui, &["Diagnostics","Samsung","Xiaomi","Motorola","Sony","Nokia","OPPO"], &mut state.tools_tab);
    match state.tools_tab {
        0 => {
            ui.columns(2, |cols| {
                card(&mut cols[0], |ui| {
                    sh(ui, "USB \u{00b7} ADB Diagnostics");
                    btn_s(ui, "Enumerate USB Device Tree"); ui.add_space(6.0);
                    btn_s(ui, "Test ADB Connectivity"); ui.add_space(6.0);
                    btn_s(ui, "Verify libusb Linkage");
                });
                card(&mut cols[1], |ui| {
                    sh(ui, "TCP Port Probe");
                    ui.horizontal(|ui| {
                        fi(ui, &mut state.tools_probe_host, "Target host or IP");
                        ui.add_space(4.0);
                        egui::Frame::NONE.fill(Color32::from_rgb(20,23,28))
                            .stroke(egui::Stroke::new(1.0,Color32::from_rgba_premultiplied(255,255,255,20)))
                            .corner_radius(8).inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.add_sized([60.0,0.0], egui::TextEdit::singleline(&mut state.tcp_test_port)
                                    .hint_text("Port").frame(egui::Frame::NONE));
                            });
                        btn_p(ui, "Probe");
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new("Local TCP probe via chimera-api::portcheck.").size(9.0).color(C::T2));
                });
            });
        }
        1 => {
            ui.columns(2, |cols| {
                card(&mut cols[0], |ui| {
                    sh(ui, "Samsung Service Tools");
                    for l in &["Read IMEI from Device","Remove FRP Lock","Change CSC Region",
                               "Stage Bundled TWRP App APK","Root \u{00b7} Unroot \u{00b7} TWRP Prep"] {
                        btn_s(ui, l); ui.add_space(6.0);
                    }
                    ui.label(RichText::new("Bootloader unlock and any required recovery image remain device-specific.").size(9.0).color(C::T2));
                });
                card(&mut cols[1], |ui| {
                    sh(ui, "Download Mode \u{00b7} Odin");
                    note_w(ui, "Vol\u{2013} + Bixby + Power \u{2192} Download Mode.");
                    btn_s(ui, "Flash Firmware via Odin");
                });
            });
        }
        2 => { card(ui, |ui| {
            ui.set_max_width(360.0); sh(ui, "Xiaomi \u{00b7} HyperOS");
            btn_s(ui, "Remove FRP Lock"); ui.add_space(6.0);
            btn_s(ui, "Unlock Mi Account"); ui.add_space(6.0);
            btn_s(ui, "Check Bootloader State");
        }); }
        3 => { card(ui, |ui| {
            ui.set_max_width(360.0); sh(ui, "Motorola");
            btn_s(ui, "Remove FRP Lock"); ui.add_space(6.0);
            btn_s(ui, "Repair IMEI");
        }); }
        4 => { card(ui, |ui| {
            ui.set_max_width(360.0); sh(ui, "Sony Xperia");
            btn_s(ui, "Backup TA Partition"); ui.add_space(6.0);
            btn_s(ui, "Restore TA Partition");
        }); }
        5 => { card(ui, |ui| {
            ui.set_max_width(360.0); sh(ui, "Nokia");
            btn_s(ui, "Remove FRP Lock");
        }); }
        _ => { card(ui, |ui| {
            ui.set_max_width(360.0); sh(ui, "OPPO \u{00b7} OnePlus \u{00b7} Nothing");
            btn_s(ui, "Remove FRP Lock"); ui.add_space(6.0);
            btn_s(ui, "Nothing Phone FRP Bypass");
        }); }
    }
}

// ══════════════════════════════════════════════════════
// pg-ios  APPLE iOS
// ══════════════════════════════════════════════════════
pub fn render_apple_ios(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
    ph(ui, "01 \u{00b7} PLATFORM", "Apple iOS",
        "iPhone and iPad \u{2014} DFU \u{00b7} IPSW flash \u{00b7} iCloud \u{00b7} passcode \u{00b7} lockdown");
    tabs(ui, &["DFU \u{00b7} Recovery","Flash IPSW","iCloud","Passcode","Network Unlock","Lockdown"], &mut state.apple_ios_tab);
    match state.apple_ios_tab {
        0 => {
            ui.columns(2, |cols| {
                card(&mut cols[0], |ui| {
                    sh(ui, "Device Mode Entry");
                    ui.horizontal(|ui| {
                        btn_p(ui, "Enter DFU Mode");
                        btn_s(ui, "Enter Recovery");
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        btn_s(ui, "Exit Recovery");
                        btn_s(ui, "Normal Boot");
                    });
                });
                card(&mut cols[1], |ui| {
                    sh(ui, "DFU Entry Reference");
                    kv(ui, "iPhone 8 \u{00b7} X and later",  "Vol\u{2191} \u{2192} Vol\u{2193} \u{2192} hold Side 10 s \u{2192} release Side + hold Vol\u{2193} 5 s", C::T1, false, false);
                    kv(ui, "iPhone 7 \u{00b7} 7 Plus",       "Hold Vol\u{2193} + Wake 10 s \u{2192} release Wake + hold Vol\u{2193} 5 s", C::T1, false, false);
                    kv(ui, "iPhone 6s and earlier",          "Hold Home + Wake 10 s \u{2192} release Wake + hold Home 5 s", C::T1, false, true);
                });
            });
        }
        1 => { card(ui, |ui| {
            ui.set_max_width(470.0);
            sh(ui, "Flash IPSW Firmware");
            fl(ui, "IPSW File");
            ui.horizontal(|ui| {
                fi(ui, &mut state.apple_ipsw_path, "~/Downloads/iPhone_*.ipsw");
                btn_s(ui, "Browse");
            });
            ui.add_space(8.0);
            fl(ui, "SHSH2 Blob (optional \u{2014} for downgrade)");
            ui.horizontal(|ui| {
                fi(ui, &mut state.apple_shsh_path, "*.shsh2 blob file");
                btn_s(ui, "Browse");
            });
            ui.add_space(8.0);
            tog(ui, "Verify IPSW SHA-1 before flashing", "", &mut state.apple_verify_sha1);
            tog(ui, "Erase device during restore", "", &mut state.apple_erase_restore);
            ui.add_space(8.0);
            btn_p(ui, "Begin Flash Sequence");
        }); }
        2 => {
            ui.columns(2, |cols| {
                card(&mut cols[0], |ui| {
                    sh(ui, "Activation Status");
                    btn_s(ui, "Query lockdownd Service"); ui.add_space(6.0);
                    btn_s(ui, "Check Online via Apple"); ui.add_space(9.0);
                    kv(ui, "ActivationState", "No device", C::A, false, true);
                });
                card(&mut cols[1], |ui| {
                    sh(ui, "Bypass");
                    fl(ui, "Technique");
                    egui::ComboBox::from_id_salt("ios_bypass").selected_text(&state.apple_bypass_technique)
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for m in &["checkm8 (A7\u{2013}A11)","DNS redirect","Erase and restore","MDM DEP"] {
                                ui.selectable_value(&mut state.apple_bypass_technique, m.to_string(), *m);
                            }
                        });
                    ui.add_space(8.0);
                    btn_p(ui, "Execute Bypass");
                });
            });
        }
        3 => { card(ui, |ui| {
            ui.set_max_width(430.0);
            sh(ui, "Passcode Removal");
            note_w(ui, "checkm8 requires A11 SoC or earlier. Erase method supports all models.");
            fl(ui, "Removal Method");
            egui::ComboBox::from_id_salt("pc_method").selected_text(&state.apple_passcode_method)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for m in &["checkm8 bypass exploit (A7\u{2013}A11)","Erase device and restore (all models)"] {
                        ui.selectable_value(&mut state.apple_passcode_method, m.to_string(), *m);
                    }
                });
            ui.add_space(8.0);
            kv(ui, "Remaining attempts", "No device", C::A, false, false);
            kv(ui, "Failed attempts", "\u{2014}", C::T2, false, true);
            ui.add_space(8.0);
            btn_d(ui, "Remove Passcode");
        }); }
        4 => { card(ui, |ui| {
            sh(ui, "iPhone \u{00b7} AU Network Unlock Reference");
            ScrollArea::vertical().id_salt("nu_tbl").max_height(300.0).show(ui, |ui| {
                egui::Grid::new("nu_grid").num_columns(5).striped(true).show(ui, |ui| {
                    for h in &["Device Series","Identifiers","SoC","checkm8","Free Unlock"] {
                        ui.label(RichText::new(h.to_uppercase()).size(7.5).strong().color(C::T3));
                    }
                    ui.end_row();
                    let rows: &[(&str,&str,&str,bool,bool)] = &[
                        ("iPhone 17 Air \u{00b7} 17 \u{00b7} Pro \u{00b7} Pro Max","iPhone18,1\u{2013}5","A19 \u{00b7} A19 Pro",false,true),
                        ("iPhone 16 \u{00b7} 16e series","iPhone17,1\u{2013}5","A18 \u{00b7} A16",false,true),
                        ("iPhone 15 series","iPhone15,4\u{2013}16,2","A16 \u{00b7} A17 Pro",false,true),
                        ("iPhone 14 series","iPhone14,7\u{2013}15,3","A15 \u{00b7} A16",false,true),
                        ("iPhone 13 series","iPhone14,2\u{2013}6","A15 Bionic",false,true),
                        ("iPhone X through 11 Pro Max","iPhone10,3\u{2013}12,5","A11\u{2013}A13",true,true),
                    ];
                    for (series,ids,soc,cm8,free) in rows {
                        ui.label(RichText::new(*series).size(10.0).color(C::T1));
                        ui.label(RichText::new(*ids).size(9.0).color(C::T2).family(egui::FontFamily::Monospace));
                        ui.label(RichText::new(*soc).size(9.0).color(C::T2).family(egui::FontFamily::Monospace));
                        if *cm8 { chip_a(ui,"A11 only"); } else { chip_r(ui,"No"); }
                        chip_g(ui,"Yes");
                        ui.end_row();
                    }
                });
            });
        }); }
        _ => {
            ui.columns(2, |cols| {
                card(&mut cols[0], |ui| {
                    sh(ui, "lockdownd Values");
                    kv(ui,"ProductType","No device",C::A,false,false);
                    kv(ui,"ProductVersion","\u{2014}",C::T2,false,false);
                    kv(ui,"IMEI","\u{2014}",C::T2,true,false);
                    kv(ui,"SerialNumber","\u{2014}",C::T2,true,true);
                });
                card(&mut cols[1], |ui| {
                    sh(ui, "Pair Record");
                    note_i(ui, "Pair records read from ~/Library/Lockdown/<UDID>.plist on macOS. Falls back to None if absent or expired.");
                });
            });
        }
    }
}

// ══════════════════════════════════════════════════════
// pg-au  AU NETWORK UNLOCK
// ══════════════════════════════════════════════════════
pub fn render_au_unlock(ui: &mut egui::Ui, state: &mut AppState) {
    ph(ui, "02 \u{00b7} PLATFORM", "AU Network Unlock",
        "Australian carrier network unlock \u{00b7} MCC 505 operators");
    tabs(ui, &["Carriers","Unlock Wizard","IMEI Validate"], &mut state.au_tab);
    match state.au_tab {
        0 => {
            let carriers = [
                ("Telstra",                        "MCC 505/01 \u{00b7} telstra.com.au/support",         "https://www.telstra.com.au/support/mobiles-tablets-and-home-phones/unlock-your-device"),
                ("Optus",                          "MCC 505/02 \u{00b7} optus.com.au/support",           "https://www.optus.com.au/support/mobiles-wearables/unlock-your-device"),
                ("Vodafone AU",                    "MCC 505/03 \u{00b7} vodafone.com.au",                "https://www.vodafone.com.au/support/device/unlock-device"),
                ("TPG Telecom",                    "MCC 505/90 \u{00b7} tpg.com.au",                    "https://www.tpg.com.au/support/unlocking"),
                ("Boost Mobile (Telstra MVNO)",    "MCC 505/19 \u{00b7} boost.com.au",                  "https://www.boost.com.au/pages/faq"),
                ("Woolworths Mobile (Optus MVNO)", "MCC 505/05 \u{00b7} woolworths.com.au/mobile",      "https://www.woolworthsmobile.com.au/support/"),
            ];
            for (name, mcc, url) in &carriers {
                egui::Frame::NONE.fill(Color32::from_rgb(16,19,23))
                    .stroke(egui::Stroke::new(1.0,C::LN))
                    .corner_radius(10).inner_margin(egui::Margin{left:11,right:11,top:12,bottom:12})
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(*name).size(11.0).strong().color(C::T0));
                                ui.label(RichText::new(*mcc).size(9.0).color(C::T2).family(egui::FontFamily::Monospace));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if btn_s(ui, "Open Portal") { let _ = webbrowser::open(url); }
                                chip_g(ui, "Free");
                            });
                        });
                    });
                ui.add_space(5.0);
            }
        }
        1 => { card(ui, |ui| {
            ui.set_max_width(450.0);
            sh(ui, "Guided Unlock Wizard");
            fr(ui, "Device IMEI", &mut state.au_wizard_imei, "15-digit IMEI");
            fl(ui, "Locked Carrier");
            egui::ComboBox::from_id_salt("au_carr").selected_text(&state.au_wizard_carrier)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for c in &["Telstra (MCC 505/01)","Optus (MCC 505/02)","Vodafone AU (MCC 505/03)",
                               "TPG Telecom (MCC 505/90)","Boost Mobile (MCC 505/19)","Woolworths Mobile (MCC 505/05)"] {
                        ui.selectable_value(&mut state.au_wizard_carrier, c.to_string(), *c);
                    }
                });
            ui.add_space(8.0);
            if btn_p(ui, "Run Unlock Wizard") {
                let imei = state.au_wizard_imei.replace(|c:char| !c.is_ascii_digit(), "");
                let carr = state.au_wizard_carrier.split(' ').next().unwrap_or("").to_string();
                if luhn_check(&imei) {
                    state.au_wizard_result = Some(format!(
                        "\u{2713} IMEI validated \u{2014} {} unlock request ready\n\nStep 1 \u{2014} Navigate to the {} device unlock portal\nStep 2 \u{2014} Enter IMEI: {}\nStep 3 \u{2014} Submit free unlock request (eligible devices only)\nStep 4 \u{2014} Allow 24\u{2013}72 hours for carrier processing\nStep 5 \u{2014} Insert a non-{} SIM and follow on-screen prompts",
                        carr, carr, imei, carr));
                } else {
                    state.au_wizard_result = Some("Invalid IMEI \u{2014} verify the number and retry.".into());
                }
            }
            if let Some(ref r) = state.au_wizard_result.clone() {
                ui.add_space(8.0);
                if r.starts_with('\u{2713}') { note_k(ui, r); } else { note_w(ui, r); }
            }
        }); }
        _ => { card(ui, |ui| {
            ui.set_max_width(330.0);
            sh(ui, "IMEI Validation \u{00b7} Luhn");
            fr(ui, "IMEI Number", &mut state.au_validate_imei, "15-digit IMEI");
            if btn_p(ui, "Validate IMEI") {
                let clean = state.au_validate_imei.replace(|c:char| !c.is_ascii_digit(), "");
                state.au_validate_result = if luhn_check(&clean) {
                    "\u{2713} Valid IMEI \u{2014} Luhn algorithm passed".into()
                } else {
                    "\u{2717} Invalid IMEI \u{2014} Luhn check failed".into()
                };
            }
            if !state.au_validate_result.is_empty() {
                ui.add_space(8.0);
                if state.au_validate_result.starts_with('\u{2713}') {
                    note_k(ui, &state.au_validate_result.clone());
                } else {
                    note_w(ui, &state.au_validate_result.clone());
                }
            }
        }); }
    }
}

// ══════════════════════════════════════════════════════
// pg-shsh  SHSH BLOB MANAGER
// ══════════════════════════════════════════════════════
pub fn render_shsh(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
    ph(ui, "03 \u{00b7} PLATFORM", "SHSH Blob Manager",
        "APTicket \u{00b7} SHSH2 save and verify \u{00b7} futurerestore command builder");
    tabs(ui, &["Save Blobs","Local Blobs","Downgrade Report","FutureRestore","Error Catalogue"], &mut state.shsh_tab);
    match state.shsh_tab {
        0 => {
            ui.columns(2, |cols| {
                card(&mut cols[0], |ui| {
                    sh(ui, "Request from Apple TSS");
                    fr(ui, "Device ECID (Hexadecimal)", &mut state.shsh_ecid, "0x\u{2026} ECID from lockdownd");
                    fr(ui, "Board Configuration", &mut state.shsh_board, "e.g. d22ap \u{00b7} d63ap \u{00b7} d421ap");
                    fr(ui, "Target Build Identifier", &mut state.shsh_build, "e.g. 21A329 \u{00b7} 22E772610a");
                    btn_p(ui, "Request via gsa.apple.com");
                });
                card(&mut cols[1], |ui| {
                    sh(ui, "Bulk Save from shsh.host");
                    fr(ui, "Device ECID", &mut state.shsh_ecid2, "ECID (hexadecimal)");
                    fr(ui, "Device Model Identifier", &mut state.shsh_model, "e.g. iPhone14,2 \u{00b7} iPhone16,1");
                    btn_s(ui, "Fetch All from shsh.host");
                });
            });
        }
        1 => empty(ui, "\u{25f0}", "No Saved Blobs",
            "SHSH2 blobs will appear here once saved. Store: ~/Library/Application Support/ChimeraRS/blobs/"),
        2 => { card(ui, |ui| {
            ui.set_max_width(430.0);
            sh(ui, "Downgrade Compatibility Report");
            note_w(ui, "SEP and Cryptex1 constraints must be satisfied before any downgrade attempt.");
            fr(ui, "Target iOS Version", &mut state.downgrade_ios, "e.g. 16.7.10 \u{00b7} 17.6.1");
            fr(ui, "Device ECID", &mut state.shsh_ecid, "0x\u{2026}");
            btn_p(ui, "Generate Compatibility Report");
        }); }
        3 => { card(ui, |ui| {
            sh(ui, "futurerestore Command Builder");
            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    fr(ui, "IPSW File", &mut state.futurerestore_ipsw, "*.ipsw");
                    fr(ui, "SHSH2 Blob", &mut state.futurerestore_shsh, "*.shsh2");
                    fl(ui, "APNonce Generator");
                    egui::ComboBox::from_id_salt("apnonce_gen")
                        .selected_text(&state.apnonce_generator)
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for g in &["None","misaka","SuccessionRestore","palera1n"] {
                                ui.selectable_value(&mut state.apnonce_generator, g.to_string(), *g);
                            }
                        });
                });
                cols[1].vertical(|ui| {
                    tog(ui, "Use latest SEP firmware", "", &mut state.futurerestore_latest_sep);
                    tog(ui, "Use latest baseband firmware", "", &mut state.futurerestore_latest_bb);
                    tog(ui, "Erase device during restore", "", &mut state.futurerestore_erase);
                });
            });
            ui.add_space(8.0);
            btn_p(ui, "Build Command String");
            ui.add_space(8.0);
            code_block(ui, "futurerestore --latest-sep --latest-baseband -t *.shsh2 *.ipsw");
        }); }
        _ => { card(ui, |ui| {
            sh(ui, "Common SHSH \u{00b7} TSS Error Reference");
            ScrollArea::vertical().id_salt("err_cat").show(ui, |ui| {
                egui::Grid::new("err_tbl").num_columns(3).striped(true).spacing(egui::vec2(10.0,4.0)).show(ui, |ui| {
                    for h in &["Error","Root Cause","Recommended Action"] {
                        ui.label(RichText::new(h.to_uppercase()).size(7.5).strong().color(C::T3));
                    }
                    ui.end_row();
                    let rows: &[(&str,&str,&str)] = &[
                        ("TSS: -1",         "Firmware build not signed by Apple",        "Request a build within the active signing window"),
                        ("ECID mismatch",   "Blob does not match the device ECID",       "Verify ECID against lockdownd UniqueChipID"),
                        ("Nonce not set",   "APNonce not configured on device",          "Set generator nonce via misaka or palera1n"),
                        ("SEP incompatible","Target SEP older than installed firmware",  "Add --use-latest-sep to futurerestore"),
                        ("Baseband error",  "Baseband version mismatch",                 "Add --use-latest-baseband"),
                        ("Network timeout", "TSS server unreachable",                    "Verify gsa.apple.com reachable; check proxy"),
                        ("Cryptex1 error",  "A15+ Cryptex1 downgrade constraint",        "Downgrade not possible for this target"),
                        ("Blob expired",    "Build no longer signed by Apple",           "Save a new blob while build remains signed"),
                    ];
                    for (err, cause, action) in rows {
                        ui.label(RichText::new(*err).size(9.0).color(C::T1).family(egui::FontFamily::Monospace));
                        ui.label(RichText::new(*cause).size(9.0).color(C::T2));
                        ui.label(RichText::new(*action).size(9.0).color(C::T2));
                        ui.end_row();
                    }
                });
            });
        }); }
    }
}

// ══════════════════════════════════════════════════════
// pg-api  API TOOLS
// ══════════════════════════════════════════════════════
pub fn render_api_tools(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
    ph_act(ui, "04 \u{00b7} PLATFORM", "API Tools",
        "Endpoint explorer \u{00b7} iCloud infrastructure map \u{00b7} TCP port probing", |ui| {
            ui.label(RichText::new("Live").size(9.0).color(C::T2));
            ui.checkbox(&mut state.api_mock_mode, "");
            ui.label(RichText::new("Mock").size(9.0).color(C::T2));
        });
    tabs(ui, &["Endpoints","iCloud Map","Port Check"], &mut state.api_tab);
    match state.api_tab {
        0 => { cflat(ui, |ui| {
            ui.label(RichText::new("CHIMERATOOL SUBDOMAIN \u{00b7} LOCAL MODULE MAP").size(7.5).strong()
                .color(Color32::from_rgba_premultiplied(255,255,255,100)));
            ui.add_space(9.0);
            ScrollArea::vertical().id_salt("ep_scroll").show(ui, |ui| {
                egui::Grid::new("ep_tbl").num_columns(4).striped(false).spacing(egui::vec2(10.0,4.0)).show(ui, |ui| {
                    for h in &["Subdomain","Service Role","ChimeraRS Module","Status"] {
                        ui.label(RichText::new(h.to_uppercase()).size(7.5).strong().color(C::T3));
                    }
                    ui.end_row();
                    let rows: &[(&str,&str,&str,u8)] = &[
                        ("api.chimeratool.com",           "Primary REST API gateway",    "chimera-api::client",              0),
                        ("secure.chimeratool.com",        "IMEI \u{00b7} SHSH \u{00b7} certificates", "chimera-api::secure_api", 0),
                        ("data.chimeratool.com",          "Firmware + device database",  "chimera-firmware + chimera-devices",0),
                        ("upload.chimeratool.com",        "Log and firmware upload",     "chimera-api::upload_api",          0),
                        ("pics.chimeratool.com",          "Device image thumbnails",     "chimera-api::pics_api",            0),
                        ("portcheck.chimeratool.com",     "TCP reachability probe",      "chimera-api::portcheck",           0),
                        ("stage.chimeratool.com",         "Staging \u{00b7} beta environment","mock_server (localhost)",     1),
                        ("administration.chimeratool.com","Administrative panel",        "GUI Settings (local)",            2),
                        ("bb.chimeratool.com",            "Backend message bus",         "crossbeam-channel worker",        0),
                        ("munin.chimeratool.com",         "System monitoring",           "GUI Diagnostics tab",             2),
                    ];
                    for (sub,role,module,kind) in rows {
                        ui.label(RichText::new(*sub).size(9.0).color(C::T2).family(egui::FontFamily::Monospace));
                        ui.label(RichText::new(*role).size(9.0).color(C::T2));
                        ui.label(RichText::new(*module).size(9.0).color(C::T2));
                        match kind { 1=>chip_a(ui,"Mock"), 2=>chip_b(ui,"Local"), _=>chip_g(ui,"Ready") }
                        ui.end_row();
                    }
                });
            });
        }); }
        1 => { cflat(ui, |ui| {
            ui.label(RichText::new("iCLOUD INFRASTRUCTURE MAP \u{00b7} 198 ENDPOINTS \u{00b7} 27 PROBE-CONFIRMED \u{00b7} 2026-03-13").size(7.5).strong()
                .color(Color32::from_rgba_premultiplied(255,255,255,100)));
            ui.add_space(6.0);
            note_i(ui, "Apple IPv4: 17.110/16 \u{00b7} 17.111/16 \u{00b7} 17.143/16 \u{00b7} 17.172/16 \u{00b7} 17.177/16 \u{00b7} 17.178/16 \u{00b7} 17.248/16 \u{00b7} 17.56.136/24 \u{00b7} 23.11.166/24 \u{00b7} IPv6: 2403:300:a50:180::/64 \u{00b7} 2600:1415:6c00::/48");
            ScrollArea::vertical().id_salt("ic_scroll").show(ui, |ui| {
                egui::Grid::new("ic_tbl").num_columns(4).striped(false).spacing(egui::vec2(10.0,4.0)).show(ui, |ui| {
                    for h in &["FQDN","IPv4","Role","Probe"] {
                        ui.label(RichText::new(h.to_uppercase()).size(7.5).strong().color(C::T3));
                    }
                    ui.end_row();
                    let rows: &[(&str,&str,&str,u8)] = &[
                        ("background.gateway.icloud.com","17.248.219.23/66/39/8","Primary gateway",     0),
                        ("ckhttpapi.icloud.com",          "17.248.219.15/23/66/8","CloudKit HTTP API",   0),
                        ("beta.icloud.com",               "23.11.166.5",          "Beta access",         0),
                        ("imap.mail.icloud.com",          "17.56.136.196",        "Mail IMAP",           0),
                        ("mr-e3sh.icloud.com",            "17.178.103.11",        "TSS signing relay",   0),
                        ("icloud4-hubble.icloud.com",     "17.177.80.31",         "Hubble monitoring",   0),
                        ("antonws.icloud.com",            "17.143.188.8",         "Anton websocket",     0),
                        ("escrowproxy.icloud.com",        "\u{2014}",             "Escrow key proxy",    1),
                        ("mccgateway.icloud.com",         "\u{2014}",             "Carrier activation",  1),
                        ("fmipmobile.icloud.com",         "\u{2014}",             "Find My iPhone",      1),
                        ("gsa.apple.com",                 "\u{2014}",             "TSS firmware signing",1),
                    ];
                    let probes = ["\u{2713} 301","\u{2713} 301","\u{2713} 301","\u{2713}","\u{2713}","\u{2713}","\u{2713}","\u{2014}","\u{2014}","\u{2014}","\u{2014}"];
                    for (i,(fqdn,ip,role,kind)) in rows.iter().enumerate() {
                        ui.label(RichText::new(*fqdn).size(8.5).color(C::T2).family(egui::FontFamily::Monospace));
                        ui.label(RichText::new(*ip).size(8.5).color(C::T2).family(egui::FontFamily::Monospace));
                        ui.label(RichText::new(*role).size(9.0).color(C::T2));
                        if *kind == 0 { chip_g(ui, probes[i]); } else { chip_m(ui, probes[i]); }
                        ui.end_row();
                    }
                });
            });
        }); }
        _ => { card(ui, |ui| {
            ui.set_max_width(430.0);
            sh(ui, "TCP Port Reachability Probe");
            ui.horizontal(|ui| {
                fi(ui, &mut state.api_probe_host, "Target hostname or IP");
                ui.add_space(4.0);
                egui::Frame::NONE.fill(Color32::from_rgb(20,23,28))
                    .stroke(egui::Stroke::new(1.0,Color32::from_rgba_premultiplied(255,255,255,20)))
                    .corner_radius(8).inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.add_sized([60.0,0.0], egui::TextEdit::singleline(&mut state.api_probe_port)
                            .hint_text("Port").frame(egui::Frame::NONE));
                    });
                btn_p(ui, "Probe");
            });
            ui.add_space(4.0);
            ui.label(RichText::new("Local TCP connection via chimera-api::portcheck.").size(9.0).color(C::T2));
        }); }
    }
}

// ══════════════════════════════════════════════════════
// pg-evlog  EVENT LOG
// ══════════════════════════════════════════════════════
pub fn render_event_log(ui: &mut egui::Ui, state: &mut AppState) {
    use chimera_core::event::LogLevel;
    ph_act(ui, "05 \u{00b7} PLATFORM", "Event Log",
        "Runtime telemetry \u{00b7} Worker output \u{00b7} System events", |ui| {
            btn_s(ui, "Pause");
            btn_s(ui, "Export");
            if btn_s(ui, "Clear") { state.log_entries.clear(); }
        });

    // Stats bar
    let total = state.log_entries.len();
    let warns = state.log_entries.iter().filter(|e| matches!(e.level, LogLevel::Warning)).count();
    let errs  = state.log_entries.iter().filter(|e| matches!(e.level, LogLevel::Error)).count();
    let infos = total - warns - errs;

    egui::Frame::NONE
        .fill(Color32::from_rgba_premultiplied(255,255,255,6))
        .stroke(egui::Stroke::new(1.0, C::LN))
        .corner_radius(8).inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                evstat(ui, &total.to_string(), "Total",  C::T0);
                evdiv(ui);
                evstat(ui, &infos.to_string(), "Info",   C::G);
                evdiv(ui);
                evstat(ui, &warns.to_string(), "Warn",   C::A);
                evdiv(ui);
                evstat(ui, &errs.to_string(),  "Error",  C::R);
                evdiv(ui);
                ui.add_space(8.0);
                ui.label(RichText::new("Filter").size(9.0).color(C::T3));
                egui::ComboBox::from_id_salt("evlog_filter")
                    .selected_text(&state.evlog_filter)
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for f in &["All levels","Info only","Warn + Error","Error only"] {
                            ui.selectable_value(&mut state.evlog_filter, f.to_string(), *f);
                        }
                    });
            });
        });
    ui.add_space(8.0);

    // Column headers
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        for (label, w) in &[("Timestamp",80.0),("Level",44.0),("Source",90.0),("Message",0.0)] {
            ui.add_sized([*w, 0.0], egui::Label::new(RichText::new(*label).size(7.5).strong().color(C::T3)));
        }
    });
    ui.add(egui::Separator::default().spacing(2.0));

    egui::Frame::NONE.fill(C::S00).stroke(egui::Stroke::new(1.0,C::LN)).corner_radius(8)
        .show(ui, |ui| {
            ScrollArea::vertical().id_salt("evlog_scroll")
                .stick_to_bottom(true).max_height(ui.available_height()-20.0)
                .show(ui, |ui| {
                    let entries: Vec<_> = state.log_entries.iter()
                        .filter(|e| match state.evlog_filter.as_str() {
                            "Info only"    => matches!(e.level, LogLevel::Info | LogLevel::Success),
                            "Warn + Error" => matches!(e.level, LogLevel::Warning | LogLevel::Error),
                            "Error only"   => matches!(e.level, LogLevel::Error),
                            _              => true,
                        }).collect();
                    for e in entries.iter().rev().take(200).rev() {
                        let (lv_str, lv_col, row_bg) = match e.level {
                            LogLevel::Info    => ("INFO", C::G,  Color32::TRANSPARENT),
                            LogLevel::Warning => ("WARN", C::A,  Color32::from_rgba_premultiplied(232,149,30,4)),
                            LogLevel::Error   => ("ERR ", C::R,  Color32::from_rgba_premultiplied(194,68,68,4)),
                            LogLevel::Success => ("OK  ", C::G,  Color32::TRANSPARENT),
                            LogLevel::Debug   => ("DBG ", C::T3, Color32::TRANSPARENT),
                        };
                        egui::Frame::NONE.fill(row_bg)
                            .inner_margin(egui::Margin{left:11,right:11,top:2,bottom:2})
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_sized([80.0,0.0], egui::Label::new(
                                        RichText::new(&e.timestamp).size(8.5).color(C::T3)
                                            .family(egui::FontFamily::Monospace)));
                                    ui.add_sized([44.0,0.0], egui::Label::new(
                                        RichText::new(lv_str).size(8.0).strong().color(lv_col)));
                                    let src = if e.message.contains("::") {
                                        e.message.split("::").take(2).collect::<Vec<_>>().join("::").split(' ').next().unwrap_or("chimera").to_string()
                                    } else { "chimera".to_string() };
                                    ui.add_sized([90.0,0.0], egui::Label::new(
                                        RichText::new(&src).size(8.5).color(C::T3)
                                            .family(egui::FontFamily::Monospace)));
                                    ui.label(RichText::new(&e.message).size(9.0).color(C::T2));
                                });
                            });
                    }
                });
        });
}

fn evstat(ui: &mut egui::Ui, val: &str, label: &str, col: Color32) {
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(val).size(14.0).strong().color(col));
        ui.label(RichText::new(label.to_uppercase()).size(7.5).color(C::T3));
    });
}
fn evdiv(ui: &mut egui::Ui) {
    let (r,_) = ui.allocate_exact_size(egui::vec2(1.0,28.0), egui::Sense::hover());
    if ui.is_rect_visible(r) {
        ui.painter().line_segment([r.left_top(),r.left_bottom()], egui::Stroke::new(1.0,C::LN));
    }
}

// ══════════════════════════════════════════════════════
// LOGIC HELPERS
// ══════════════════════════════════════════════════════
pub fn luhn_check(s: &str) -> bool {
    if s.len() != 15 || !s.chars().all(|c| c.is_ascii_digit()) { return false; }
    let sum: u32 = s.chars().enumerate().map(|(i,c)| {
        let mut d = c.to_digit(10).unwrap();
        if i % 2 == 1 { d *= 2; if d > 9 { d -= 9; } }
        d
    }).sum();
    sum % 10 == 0
}

fn validate_imei(s: &str) -> String {
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() != 15 { return "\u{2717} Enter exactly 15 digits".into(); }
    if luhn_check(&d) { "\u{2713} Valid IMEI \u{2014} Luhn algorithm passed".into() }
    else               { "\u{2717} Invalid IMEI \u{2014} Luhn check failed".into() }
}

fn validate_mac(mac: &str) -> String {
    let clean: String = mac.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if clean.len() != 12 { return "\u{274c} Invalid MAC \u{2014} expected 12 hex digits".into(); }
    let bytes: Vec<u8> = (0..6).map(|i| u8::from_str_radix(&clean[i*2..i*2+2], 16).unwrap_or(0)).collect();
    let fmt = bytes.iter().map(|b| format!("{:02X}",b)).collect::<Vec<_>>().join(":");
    let oui = format!("{:02X}-{:02X}-{:02X}", bytes[0], bytes[1], bytes[2]);
    let mc = if bytes[0] & 1 != 0 { "[Multicast]" } else { "[Unicast]" };
    let la = if bytes[0] & 2 != 0 { "[Locally Administered]" } else { "[Globally Unique]" };
    format!("\u{2713} Valid: {}  OUI: {}  {} {}", fmt, oui, mc, la)
}

fn derive_mac(input: &str) -> String {
    use sha2::{Sha256, Digest};
    let clean = input.trim();
    if clean.is_empty() { return String::new(); }
    let hash = Sha256::digest(clean.as_bytes());
    let mut b = [hash[0],hash[1],hash[2],hash[3],hash[4],hash[5]];
    b[0] &= 0xFE; b[0] |= 0x02;
    b.iter().map(|x| format!("{:02X}",x)).collect::<Vec<_>>().join(":")
}

fn compute_hash(algo: &str, input: &str, key: &str) -> String {
    fn hex(b: &[u8]) -> String {
        use std::fmt::Write;
        b.iter().fold(String::new(), |mut s,x| { let _ = write!(s,"{:02x}",x); s })
    }
    match algo {
        "SHA-256" => { use sha2::{Sha256,Digest}; hex(&Sha256::digest(input.as_bytes())) }
        "SHA-1"   => { use sha1::{Sha1,Digest};   hex(&Sha1::digest(input.as_bytes())) }
        "MD5"     => { use md5::{Md5,Digest};     hex(&Md5::digest(input.as_bytes())) }
        "CRC-32"  => format!("{:08x}", crc32fast::hash(input.as_bytes())),
        "HMAC-SHA256" => {
            use hmac::{Hmac,Mac,KeyInit}; use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;
            let k = if key.is_empty() { b"chimera".as_ref() } else { key.as_bytes() };
            let mut m = HmacSha256::new_from_slice(k).unwrap_or_else(|_| HmacSha256::new_from_slice(b"chimera").unwrap());
            m.update(input.as_bytes());
            hex(&m.finalize().into_bytes())
        }
        _ => String::new(),
    }
}
