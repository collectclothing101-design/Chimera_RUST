// chimera-gui/src/ui/device_list.rs
// Connected phone cards — ChimeraTool-style compact cards in the sidebar
// and full device list panel when the "Phones" nav item is active.
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui::{self, RichText, Color32};
use crate::state::{AppState, ActiveTab};
use crate::worker::OperationRequest;
use crate::theme::ChimeraTheme;
use chimera_core::device::{ConnectionMode, DeviceState};
use crossbeam_channel::Sender;

// ─── Sidebar compact cards (called from mod.rs nav_sidebar) ──────────────────
pub fn render_sidebar_device_list(
    ui:     &mut egui::Ui,
    state:  &mut AppState,
    __op_tx: &Sender<OperationRequest>,
) {
    let device_entries: Vec<(String, String, String, DeviceState)> = state
        .devices
        .values()
        .map(|d| (
            d.device.id.clone(),
            d.device.model.clone(),
            format!("{:?}", d.device.brand),
            d.device.state.clone(),
        ))
        .collect();

    let selected_id = state.selected_device_id.clone();

    for (id, model, brand, dev_state) in &device_entries {
        let is_selected = selected_id.as_ref().map_or(false, |s| s == id);
        let is_active   = matches!(dev_state, DeviceState::Connected | DeviceState::Authorized);

        let bg = if is_selected {
            ChimeraTheme::BG_ACTIVE
        } else {
            Color32::TRANSPARENT
        };

        let frame = egui::Frame::NONE
            .fill(bg)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .corner_radius(egui::CornerRadius::same(5))
            .stroke(if is_selected {
                egui::Stroke::new(1.0_f32, ChimeraTheme::ACCENT)
            } else {
                egui::Stroke::NONE
            });

        let resp = frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // State dot
                let dot_color = if is_active {
                    ChimeraTheme::SUCCESS
                } else {
                    ChimeraTheme::TEXT_DISABLED
                };
                ChimeraTheme::status_dot(ui, dot_color);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(model.as_str())
                            .strong()
                            .size(12.0)
                            .color(if is_selected {
                                ChimeraTheme::TEXT_HEADING
                            } else {
                                ChimeraTheme::TEXT_PRIMARY
                            }),
                    );
                    ui.colored_label(
                        ChimeraTheme::TEXT_DISABLED,
                        RichText::new(brand.as_str()).size(10.5),
                    );
                });
            });
        });

        if resp.response.interact(egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            state.selected_device_id = Some(id.clone());
            state.active_tab = ActiveTab::DeviceInfo;
        }
    }
}

// ─── Full device list (DeviceInfo tab — when no specific device selected) ─────
#[allow(dead_code)]
pub fn render_device_list(
    ui:    &mut egui::Ui,
    state: &mut AppState,
    _op_tx: &Sender<OperationRequest>,
) {
    // Header
    ui.horizontal(|ui| {
        ui.colored_label(
            ChimeraTheme::ACCENT,
            RichText::new("📱  PHONES").strong().size(14.0),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("🔄").on_hover_text("Refresh").clicked() {
                state.add_log(crate::state::LogEntry::info("Refreshing device list…"));
            }
        });
    });

    ui.add_space(8.0);

    if state.devices.is_empty() {
        // Empty state card
        ChimeraTheme::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.colored_label(ChimeraTheme::TEXT_DISABLED, RichText::new("📵").size(32.0));
                ui.add_space(8.0);
                ui.colored_label(
                    ChimeraTheme::TEXT_SECONDARY,
                    "No devices detected",
                );
                ui.add_space(4.0);
                ui.colored_label(
                    ChimeraTheme::TEXT_DISABLED,
                    RichText::new("Connect a phone via USB and enable USB Debugging").size(11.0),
                );
                ui.add_space(20.0);
            });
        });
        return;
    }

    let device_entries: Vec<(String, String, String, ConnectionMode, DeviceState)> = state
        .devices
        .values()
        .map(|d| (
            d.device.id.clone(),
            d.device.model.clone(),
            format!("{:?}", d.device.brand),
            d.device.connection_mode.clone(),
            d.device.state.clone(),
        ))
        .collect();

    let selected_id = state.selected_device_id.clone();

    for (id, model, brand, mode, dev_state) in &device_entries {
        let is_selected = selected_id.as_ref().map_or(false, |s| s == id);
        let is_active   = matches!(dev_state, DeviceState::Connected | DeviceState::Authorized);

        let border = if is_selected {
            egui::Stroke::new(1.5_f32, ChimeraTheme::ACCENT)
        } else {
            egui::Stroke::new(1.0_f32, ChimeraTheme::BORDER)
        };
        let bg = if is_selected {
            ChimeraTheme::BG_ACTIVE
        } else {
            ChimeraTheme::BG_CARD
        };

        let frame = egui::Frame::NONE
            .fill(bg)
            .inner_margin(egui::Margin::same(10))
            .corner_radius(egui::CornerRadius::same(7))
            .stroke(border);

        let resp = frame.show(ui, |ui| {
            ui.set_width(ui.available_width());

            ui.horizontal(|ui| {
                // Brand colour dot
                let brand_color = ChimeraTheme::brand_color(&brand.to_lowercase());
                ui.painter_at(ui.cursor().expand(6.0)).circle_filled(
                    ui.cursor().left_center() + egui::vec2(6.0, 0.0),
                    5.0,
                    brand_color,
                );
                ui.add_space(14.0);

                ui.vertical(|ui| {
                    // Model + brand
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            if is_selected { ChimeraTheme::ACCENT } else { ChimeraTheme::TEXT_HEADING },
                            RichText::new(model.as_str()).strong().size(13.5),
                        );
                        ui.colored_label(
                            ChimeraTheme::TEXT_DISABLED,
                            RichText::new(format!("  {}", brand)).size(11.0),
                        );
                    });

                    // Connection info row
                    ui.horizontal(|ui| {
                        let state_color = if is_active {
                            ChimeraTheme::SUCCESS
                        } else {
                            ChimeraTheme::WARNING
                        };
                        let state_str = format!("{:?}", dev_state);
                        ui.colored_label(state_color, RichText::new(state_str).size(11.0));
                        ui.colored_label(
                            ChimeraTheme::TEXT_DISABLED,
                            RichText::new(format!("  via {}", mode)).size(11.0),
                        );
                    });
                });

                // Right: selected badge
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_selected {
                        ChimeraTheme::golden_badge(ui, "ACTIVE");
                    } else {
                        ui.colored_label(
                            ChimeraTheme::TEXT_DISABLED,
                            RichText::new("Select →").size(11.0),
                        );
                    }
                });
            });
        });

        let click_resp = resp.response.interact(egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if click_resp.clicked() {
            state.selected_device_id = Some(id.clone());
            state.active_tab = ActiveTab::DeviceInfo;
        }

        ui.add_space(5.0);
    }
}
