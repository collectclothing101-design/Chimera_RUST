// crates/chimera-gui/src/app.rs
// ChimeraRS — main app shell (Phase 0-11, all errors fixed)
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui;
use crate::state::AppState;
use crate::theme::C;
use crate::ui::nav::Page;
use crate::worker::OperationRequest;

pub struct ChimeraApp {
    pub state:   AppState,
    pub op_tx:   crossbeam_channel::Sender<OperationRequest>,
    start_time:  std::time::Instant,
}

impl ChimeraApp {
    pub fn new(
        cc:    &eframe::CreationContext<'_>,
        op_tx: crossbeam_channel::Sender<OperationRequest>,
    ) -> Self {
        crate::theme::apply(&cc.egui_ctx);
        let mut state = AppState::new();
        state.op_tx = Some(op_tx.clone());
        apply_font_size(&cc.egui_ctx, state.settings.font_size);
        use crate::state::LogEntry;
        state.log_entries.push(LogEntry::info("Starting ChimeraRS v1.3.13 on macOS (x86_64-apple-darwin)"));
        state.log_entries.push(LogEntry::info("chimera::worker GUI worker pool started — 22 crates · 148 files · 27,244 LOC"));
        let _ = op_tx.send(OperationRequest::QuickScan);
        Self { state, op_tx, start_time: std::time::Instant::now() }
    }
}

pub fn apply_font_size(ctx: &egui::Context, size: f32) {
    let mut style = (*ctx.global_style()).clone();
    for (_, id) in style.text_styles.iter_mut() { id.size = size; }
    ctx.set_global_style(style);
}

// ── Helper widgets (pub so dashboard.rs / downloads.rs / network_tools.rs can use them) ──

pub fn btn_p(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add(egui::Button::new(egui::RichText::new(label).size(10.0).strong()
        .color(egui::Color32::from_rgb(3,1,0)))
        .fill(C::A).stroke(egui::Stroke::new(1.0_f32,C::AH))
        .corner_radius(8).min_size(egui::vec2(0.0,30.0))
    ).clicked()
}
pub fn btn_s(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add(egui::Button::new(egui::RichText::new(label).size(10.0).color(C::T1))
        .fill(egui::Color32::from_rgb(0x16,0x18,0x1e))
        .stroke(egui::Stroke::new(1.0_f32,C::LNH))
        .corner_radius(8).min_size(egui::vec2(0.0,30.0))
    ).clicked()
}
pub fn section_hd(ui: &mut egui::Ui, label: &str) {
    ui.horizontal(|ui| {
        let (r,_) = ui.allocate_exact_size(egui::vec2(2.0,9.0), egui::Sense::hover());
        if ui.is_rect_visible(r) { ui.painter().rect_filled(r, 1.0, C::A); }
        ui.add_space(4.0);
        ui.label(egui::RichText::new(label.to_uppercase()).size(9.0).strong().color(C::T1));
    });
    ui.add_space(8.0);
}
pub fn card_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(egui::Color32::from_rgb(16, 19, 23))
        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_premultiplied(255, 255, 255, 15)))
        .corner_radius(10)
        .inner_margin(egui::Margin::same(15))
}
pub fn kv(ui: &mut egui::Ui, key: &str, val: &str, mono: bool) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{}: ", key)).size(9.5).color(C::T2));
        let rt = egui::RichText::new(val).size(9.5).color(C::T0);
        ui.label(if mono { rt.family(egui::FontFamily::Monospace) } else { rt });
    });
}
pub fn note_i(ui: &mut egui::Ui, text: &str) {
    egui::Frame::NONE
        .fill(egui::Color32::from_rgb(0x0c, 0x10, 0x18))
        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x20, 0x40, 0x80)))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(9.5).color(egui::Color32::from_rgb(0x90, 0xb0, 0xff)));
        });
}
pub fn chip_a(ui: &mut egui::Ui, label: &str) {
    egui::Frame::NONE
        .fill(egui::Color32::from_rgb(0x1a, 0x14, 0x06))
        .stroke(egui::Stroke::new(1.0_f32, C::A))
        .corner_radius(4)
        .inner_margin(egui::Margin { left:6, right:6, top:2, bottom:2 })
        .show(ui, |ui| { ui.label(egui::RichText::new(label).size(8.5).color(C::A)); });
}
pub fn chip_b(ui: &mut egui::Ui, label: &str) {
    egui::Frame::NONE
        .fill(egui::Color32::from_rgb(0x08, 0x10, 0x20))
        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(0x40, 0x80, 0xff)))
        .corner_radius(4)
        .inner_margin(egui::Margin { left:6, right:6, top:2, bottom:2 })
        .show(ui, |ui| {
            ui.label(egui::RichText::new(label).size(8.5)
                .color(egui::Color32::from_rgb(0x80, 0xb0, 0xff)));
        });
}
pub fn chip_g(ui: &mut egui::Ui, label: &str) {
    egui::Frame::NONE
        .fill(C::GBG)
        .stroke(egui::Stroke::new(1.0_f32, C::GBR))
        .corner_radius(4)
        .inner_margin(egui::Margin { left:6, right:6, top:2, bottom:2 })
        .show(ui, |ui| { ui.label(egui::RichText::new(label).size(8.5).color(C::G)); });
}
pub fn page_header(ui: &mut egui::Ui, idx: &str, title: &str, sub: &str) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(idx).size(7.5).strong().color(C::A48)
                .family(egui::FontFamily::Monospace));
            ui.label(egui::RichText::new(title.to_uppercase()).size(15.0).strong().color(C::T0));
            ui.label(egui::RichText::new(sub).size(10.0).color(C::T2));
        });
    });
    ui.add_space(13.0);
    ui.separator();
    ui.add_space(11.0);
}
pub fn empty_state(ui: &mut egui::Ui, icon: &str, title: &str, sub: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(32.0);
        ui.label(egui::RichText::new(icon).size(28.0).color(C::T3));
        ui.add_space(8.0);
        ui.label(egui::RichText::new(title).size(13.0).strong().color(C::T1));
        ui.label(egui::RichText::new(sub).size(10.5).color(C::T3));
        ui.add_space(32.0);
    });
}
pub fn field_lbl(ui: &mut egui::Ui, label: &str, val: &mut String, hint: &str) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).size(9.5).color(C::T2));
        ui.add(egui::TextEdit::singleline(val).hint_text(hint).desired_width(f32::INFINITY));
    });
}

// ─────────────────────────────────────────────────────────────────────────────

#[allow(deprecated)]
impl eframe::App for ChimeraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.process_events();
        self.state.app_uptime_secs = self.start_time.elapsed().as_secs();
        let uptime = fmt_uptime(self.state.app_uptime_secs);

        if self.state.settings_dirty {
            let _ = crate::persistence::save_settings(&self.state.settings);
            self.state.settings_dirty = false;
        }
        if (self.state.last_font_size - self.state.settings.font_size).abs() > 0.01 {
            apply_font_size(ctx, self.state.settings.font_size);
            self.state.last_font_size = self.state.settings.font_size;
        }
        let max = self.state.settings.max_log_lines;
        if self.state.log_entries.len() > max {
            let n = self.state.log_entries.len() - max;
            self.state.log_entries.drain(..n);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // ── HEADER ────────────────────────────────────────────────────
        egui::Panel::top("hdr")
            .exact_size(43.0)
            .frame(egui::Frame::NONE
                .fill(egui::Color32::from_rgb(8, 10, 12))
                .stroke(egui::Stroke::new(1.0_f32, C::LN)))
            .show(ctx, |ui| {
                render_header(ui, &mut self.state, &self.op_tx, &uptime);
            });

        // ── CONSOLE ───────────────────────────────────────────────────
        if self.state.settings.show_console {
            egui::Panel::bottom("con")
                .exact_size(90.0)
                .frame(egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(8, 10, 12))
                    .stroke(egui::Stroke::new(1.0_f32, C::LN)))
                .show(ctx, |ui| {
                    crate::ui::log_panel::render_log(ui, &mut self.state);
                });
        }

        // ── SIDEBAR ───────────────────────────────────────────────────
        egui::Panel::left("sb")
            .exact_size(236.0)
            .frame(egui::Frame::NONE
                .fill(egui::Color32::from_rgb(8, 10, 12))
                .stroke(egui::Stroke::new(1.0_f32, C::LN)))
            .show(ctx, |ui| {
                // Re-probe ADB so the sidebar pill matches the dashboard card.
                self.state.refresh_adb_throttled();
                crate::ui::nav::render_sidebar(
                    ui,
                    &mut self.state.current_page,
                    &self.state.hostname.to_uppercase(),
                    self.state.worker_ok,
                    self.state.adb_ok,
                    self.state.usb_ok,
                    &uptime,
                    "2026-03-31",
                    "1.3.13",
                );
            });

        // ── CONTENT ───────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE
                .fill(C::S00)
                .inner_margin(egui::Margin::symmetric(20, 18)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("content_scroll")
                    .show(ui, |ui| {
                        dispatch_page(ui, &mut self.state, &self.op_tx);
                    });
            });

        // ── ABOUT ─────────────────────────────────────────────────────
        if self.state.show_about_modal {
            render_about(ctx, &mut self.state.show_about_modal);
        }

        // ── CONFIRM ───────────────────────────────────────────────────
        render_confirm_dialog(ctx, &mut self.state, &self.op_tx);
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
}

// ── Confirmation dialog ────────────────────────────────────────────────────

fn render_confirm_dialog(
    ctx: &egui::Context,
    state: &mut AppState,
    op_tx: &crossbeam_channel::Sender<OperationRequest>,
) {
    if state.pending_confirm.is_none() { return; }
    let dialog = state.pending_confirm.clone().unwrap();

    egui::Window::new(&dialog.title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(egui::Frame::NONE
            .fill(egui::Color32::from_rgb(0x14, 0x10, 0x10))
            .stroke(egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(0x80, 0x20, 0x20)))
            .corner_radius(10)
            .inner_margin(egui::Margin::same(20)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("⚠").size(22.0).color(C::A));
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&dialog.title).size(14.0).strong().color(C::T0));
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(&dialog.message).size(11.0).color(C::T1));
                });
            });
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(
                    egui::RichText::new("⚠ CONFIRM").size(11.0).strong().color(C::R))
                    .fill(egui::Color32::from_rgb(0x28, 0x0a, 0x0a))
                    .stroke(egui::Stroke::new(1.0_f32, C::R))
                    .corner_radius(6)
                    .min_size(egui::vec2(110.0, 30.0))
                ).clicked() {
                    let _ = op_tx.send(dialog.on_confirm.clone());
                    state.pending_confirm = None;
                }
                ui.add_space(8.0);
                if ui.add(egui::Button::new(
                    egui::RichText::new("Cancel").size(11.0).color(C::T1))
                    .fill(egui::Color32::from_rgb(0x16, 0x18, 0x1e))
                    .stroke(egui::Stroke::new(1.0_f32, C::LNH))
                    .corner_radius(6)
                    .min_size(egui::vec2(80.0, 30.0))
                ).clicked() {
                    state.pending_confirm = None;
                }
            });
        });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.pending_confirm = None;
    }
}

// ── Header ─────────────────────────────────────────────────────────────────

fn render_header(
    ui: &mut egui::Ui,
    state: &mut AppState,
    op_tx: &crossbeam_channel::Sender<OperationRequest>,
    uptime: &str,
) {
    ui.horizontal(|ui| {
        ui.set_min_height(43.0);
        ui.spacing_mut().item_spacing.x = 10.0;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(egui::RichText::new("⚡").size(13.0).color(C::A));
            ui.add_space(5.0);
            ui.label(egui::RichText::new("CHIMERA").size(12.5).strong().color(C::A));
            ui.add_space(6.0);
            egui::Frame::NONE
                .fill(C::S03).stroke(egui::Stroke::new(1.0_f32, C::LN)).corner_radius(2)
                .inner_margin(egui::Margin { left:5, right:5, top:1, bottom:1 })
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("RS EDITION").size(8.0).strong().color(C::T2));
                });
            ui.add_space(4.0);
            ui.label(egui::RichText::new("1.3.13").size(8.5).color(C::T3)
                .family(egui::FontFamily::Monospace));
        });

        // Device count
        let dc = state.devices.len();
        egui::Frame::NONE
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::new(1.0_f32, C::LN)).corner_radius(255)
            .inner_margin(egui::Margin { left:7, right:7, top:2, bottom:2 })
            .show(ui, |ui| {
                ui.label(egui::RichText::new(if dc == 0 { "0 DEVICES".into() }
                    else { format!("{} DEVICE{}", dc, if dc == 1 {""} else {"S"}) })
                    .size(8.5).color(C::T2));
            });

        ui.add(egui::Separator::default().vertical().spacing(0.0));

        ui.vertical(|ui| {
            ui.add_space(5.0);
            ui.label(egui::RichText::new("ADMIN_USER").size(10.5).strong().color(C::T0));
            ui.label(egui::RichText::new(format!("ADMIN-MAC · X86_64 · {}", uptime))
                .size(8.5).color(C::T1).family(egui::FontFamily::Monospace));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            let scanning = state.is_scanning;
            if ui.add_enabled(!scanning, egui::Button::new(
                egui::RichText::new(if scanning { "⟳ SCANNING…" } else { "⟳ QUICK SCAN" })
                    .size(10.0).strong().color(egui::Color32::BLACK))
                .fill(C::A).corner_radius(6).min_size(egui::vec2(100.0, 26.0))
            ).clicked() {
                state.is_scanning = true;
                state.add_log(crate::state::LogEntry::info("Quick scan initiated."));
                let _ = op_tx.send(OperationRequest::QuickScan);
            }
            ui.add_space(6.0);
            if ui.add(egui::Button::new(egui::RichText::new("ABOUT").size(9.0).color(C::T1))
                .fill(egui::Color32::TRANSPARENT).stroke(egui::Stroke::new(1.0_f32, C::LN))
                .corner_radius(4)
            ).clicked() {
                state.show_about_modal = true;
            }
        });
    });
}

// ── Page dispatch ─────────────────────────────────────────────────────────

fn dispatch_page(
    ui: &mut egui::Ui, state: &mut AppState,
    op_tx: &crossbeam_channel::Sender<OperationRequest>,
) {
    use crate::ui::pages::*;
    match &state.current_page.clone() {
        Page::Dashboard  => render_dashboard(ui, state),
        Page::Devices    => render_devices(ui, state, op_tx),
        Page::Downloads  => render_downloads(ui, state),
        Page::History    => render_history(ui, state, op_tx),
        Page::Utilities  => render_utilities(ui, state, op_tx),
        Page::Settings   => render_settings(ui, state),
        Page::DeviceInfo => render_device_info(ui, state),
        Page::Jailbreak  => render_jailbreak(ui, state, op_tx),
        Page::SshVpn     => render_ssh(ui, state, op_tx),
        Page::Activation => render_activation(ui, state),
        Page::Network    => render_network(ui, state),
        Page::Tools      => render_tools(ui, state, op_tx),
        Page::AppleIos   => crate::ui::apple_panel::render_apple_panel(ui, state),
        Page::MediaTek   => crate::ui::mediatek_panel::render_mediatek_panel(ui, state),
        Page::AuUnlock   => render_au_unlock(ui, state),
        Page::ShshBlobs  => crate::ui::shsh_panel::render_shsh_panel(ui, state, op_tx),
        Page::ApiTools   => crate::ui::api_panel::render_api_panel(ui, state, op_tx),
        Page::EventLog   => render_event_log(ui, state),
    }
}

// ── About modal ───────────────────────────────────────────────────────────

fn render_about(ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("About ChimeraRS")
        .collapsible(false).resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.label(egui::RichText::new("⚡ ChimeraRS").size(22.0).strong().color(C::A));
                ui.label(egui::RichText::new("Professional Mobile Device Repair Tool")
                    .size(11.0).color(C::T2));
                ui.add_space(8.0);
            });
            for (k, v) in &[
                ("Version",       "1.3.13"),
                ("GUI Framework", "eframe 0.34 · egui · Metal + AppKit"),
                ("Architecture",  "macOS · x86_64-apple-darwin"),
                ("Rust Edition",  "2021 (nightly)"),
                ("Crates",        "22 workspace crates"),
            ] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("{}: ", k)).size(10.5).color(C::T2));
                    ui.label(egui::RichText::new(*v).size(10.5).color(C::T0)
                        .family(egui::FontFamily::Monospace));
                });
            }
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                if ui.add(egui::Button::new("Close")
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(egui::Stroke::new(1.0_f32, C::LN))
                    .corner_radius(6).min_size(egui::vec2(80.0, 28.0))
                ).clicked() { *open = false; }
            });
        });
}

// ── Utilities ────────────────────────────────────────────────────────────────

fn fmt_uptime(secs: u64) -> String {
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60)
}
