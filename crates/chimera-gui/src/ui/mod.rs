// chimera-gui/src/ui/mod.rs
// UI module wiring — ChimeraTool-matched layout
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

pub mod common;
pub mod menu;
pub mod device_list;
pub mod device_info;
pub mod operations;
pub mod firmware_panel;
pub mod utilities_panel;
pub mod settings_panel;
pub mod log_panel;
pub mod about;
pub mod diagnostics_panel;
pub mod history_panel;
pub mod apple_panel;
pub mod au_unlock_panel;
pub mod api_panel;
pub mod shsh_panel;
pub mod ssh_panel;
pub mod hash_panel;
pub mod settings_network_mac;
pub mod nav;
pub mod dashboard;
pub mod downloads;
pub mod network_tools;
pub mod pages;
pub mod mediatek_panel;

use eframe::egui::{self, RichText, Color32};
use crate::state::{AppState, ActiveTab};
use crate::theme::ChimeraTheme;
use crate::worker::OperationRequest;
use chimera_devices::database::DeviceDatabase;
use crossbeam_channel::Sender;
use chimera_core::VERSION;

// ─── Top header bar ──────────────────────────────────────────────────────────
pub fn header_bar(ui: &mut egui::Ui, state: &mut AppState, _op_tx: &Sender<OperationRequest>) {
  menu::render_header(ui, state);
}

// ─── Left navigation sidebar ─────────────────────────────────────────────────
pub fn nav_sidebar(ui: &mut egui::Ui, state: &mut AppState, op_tx: &Sender<OperationRequest>) {
  // Logo + branding strip
  ui.add_space(0.0);
  let logo_frame = egui::Frame::NONE
    .fill(ChimeraTheme::BG_HEADER)
    .inner_margin(egui::Margin::symmetric(10, 10))
    .stroke(egui::Stroke::new(0.0_f32, Color32::TRANSPARENT));
  logo_frame.show(ui, |ui| {
    ui.set_width(ui.available_width());
    ui.horizontal(|ui| {
      ui.colored_label(
        ChimeraTheme::ACCENT,
        RichText::new("Flash").size(20.0).strong(),
      );
      ui.add_space(4.0);
      ui.vertical(|ui| {
        ui.colored_label(
          ChimeraTheme::ACCENT,
          RichText::new("CHIMERA").strong().size(14.0),
        );
        ui.colored_label(
          ChimeraTheme::TEXT_SECONDARY,
          RichText::new(format!("v{} RS Edition", VERSION)).size(10.0),
        );
      });
    });
  });

  // Thin separator line
  ui.painter().hline(
    0.0..=190.0,
    ui.cursor().top(),
    egui::Stroke::new(1.0_f32, ChimeraTheme::BORDER_SUBTLE),
  );

  egui::ScrollArea::vertical()
    .auto_shrink([false, false])
    .show(ui, |ui| {
      ui.set_width(190.0);
      ui.add_space(6.0);

      // ─── DEVICE section ─────────────────────────────────────────
      ChimeraTheme::nav_section_label(ui, "Device");

      let _dev_count = state.devices.len() as u32;
      if ChimeraTheme::nav_item(ui, "Device", "Phones", state.active_tab == ActiveTab::DeviceInfo).clicked() {
          state.active_tab = ActiveTab::DeviceInfo;
      }
      if ChimeraTheme::nav_item(ui, "Tool", "Operations", state.active_tab == ActiveTab::Operations).clicked() {
          state.active_tab = ActiveTab::Operations;
      }
      if ChimeraTheme::nav_item(ui, "Firmware", "Firmware", state.active_tab == ActiveTab::Firmware).clicked() {
          state.active_tab = ActiveTab::Firmware;
      }
      if ChimeraTheme::nav_item(ui, "Diagnostics", "Diagnostics", state.active_tab == ActiveTab::Diagnostics).clicked() {
          state.active_tab = ActiveTab::Diagnostics;
      }
      if ChimeraTheme::nav_item(ui, "Tools", "Utilities", state.active_tab == ActiveTab::Utilities).clicked() {
          state.active_tab = ActiveTab::Utilities;
      }

      ui.add_space(4.0);

      // ─── APPLE / iOS section ────────────────────────────────────
      ChimeraTheme::nav_section_label(ui, "Apple / iOS");

      if ChimeraTheme::nav_item(ui, "Apple", "Apple Panel", state.active_tab == ActiveTab::Apple).clicked() {
          state.active_tab = ActiveTab::Apple;
      }
      if ChimeraTheme::nav_item(ui, "Key", "SHSH Blobs", state.active_tab == ActiveTab::ShshManager).clicked() {
          state.active_tab = ActiveTab::ShshManager;
      }

      ui.add_space(4.0);

      // ─── NETWORK / UNLOCK section ───────────────────────────────
      ChimeraTheme::nav_section_label(ui, "Unlock");

      if ChimeraTheme::nav_item(ui, "🇦🇺", "AU Network Unlock", state.active_tab == ActiveTab::AuNetworkUnlock).clicked() {
          state.active_tab = ActiveTab::AuNetworkUnlock;
      }
      if ChimeraTheme::nav_item(ui, "", "API / iCloud Tools", state.active_tab == ActiveTab::ApiTools).clicked() {
          state.active_tab = ActiveTab::ApiTools;
      }

      ui.add_space(4.0);

      // ─── GENERAL section ────────────────────────────────────────
      ChimeraTheme::nav_section_label(ui, "General");

      if ChimeraTheme::nav_item(ui, "History", "Work History", state.active_tab == ActiveTab::History).clicked() {
          state.active_tab = ActiveTab::History;
      }
      if ChimeraTheme::nav_item(ui, "Settings", "Settings", state.active_tab == ActiveTab::Settings).clicked() {
          state.active_tab = ActiveTab::Settings;
      }
      if ChimeraTheme::nav_item(ui, "Copy", "Event Log", state.active_tab == ActiveTab::Log).clicked() {
          state.active_tab = ActiveTab::Log;
      }

      ui.add_space(8.0);

      // ─── Device quick-list at bottom of sidebar ─────────────────
      if !state.devices.is_empty() {
          ui.painter().hline(
              4.0..=186.0,
              ui.cursor().top(),
              egui::Stroke::new(1.0_f32, ChimeraTheme::BORDER_SUBTLE),
          );
          ui.add_space(6.0);
          ChimeraTheme::nav_section_label(ui, "Connected");
          device_list::render_sidebar_device_list(ui, state, op_tx);
      }
    }); // closes ScrollArea::show
} // closes nav_sidebar

// ─── Log console (bottom panel) ──────────────────────────────────────────────
pub fn log_panel(ui: &mut egui::Ui, state: &mut AppState) {
  log_panel::render_log(ui, state);
}

// ─── About dialog ─────────────────────────────────────────────────────────────
pub fn about_dialog(ctx: &egui::Context, state: &mut AppState) {
  about::render_about_dialog(ctx, state);
}

// ─── Main content routing ─────────────────────────────────────────────────────
pub fn main_content(
  ui:    &mut egui::Ui,
  state:   &mut AppState,
  op_tx:   &Sender<OperationRequest>,
  device_db: &DeviceDatabase,
) {
  // Content-area top tab strip (golden underline style)
  render_content_tabs(ui, state);

  egui::ScrollArea::vertical()
    .auto_shrink([false, false])
    .show(ui, |ui| {
      ui.add_space(8.0);
      // Pad left/right inside content area
      ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.vertical(|ui| {
          ui.set_width(ui.available_width() - 12.0);
          route_content(ui, state, op_tx, device_db);
        });
      });
    });
}

/// Render the golden-underline tab strip at the top of the content area
fn render_content_tabs(ui: &mut egui::Ui, state: &mut AppState) {
  let frame = egui::Frame::NONE
    .fill(ChimeraTheme::BG_CARD)
    .inner_margin(egui::Margin::symmetric(12, 0))
    .stroke(egui::Stroke::new(
      1.0_f32,
      egui::Color32::from_rgb(0x20, 0x22, 0x30),
    ));

  frame.show(ui, |ui| {
    ui.set_min_height(38.0);
    ui.horizontal(|ui| {
      ui.add_space(4.0);
      // Map each tab to its icon + label
      let tabs: &[(ActiveTab, &str, &str)] = &[
        (ActiveTab::DeviceInfo, "Device", "Device Info"),
        (ActiveTab::Operations, "Tool", "Operations"),
        (ActiveTab::Firmware, "Firmware", "Firmware"),
        (ActiveTab::Utilities, "Tools", "Utilities"),
        (ActiveTab::Diagnostics, "Diagnostics", "Diagnostics"),
        (ActiveTab::Apple, "Apple", "Apple"),
        (ActiveTab::ShshManager, "Key", "SHSH"),
        (ActiveTab::AuNetworkUnlock, "🇦🇺", "AU Unlock"),
        (ActiveTab::ApiTools, "", "API"),
        (ActiveTab::History, "History", "History"),
        (ActiveTab::Settings, "Settings", "Settings"),
        (ActiveTab::Log, "Copy", "Log"),
      ];

      for (tab, icon, label) in tabs {
        let is_active = state.active_tab == *tab;
        let tab_text = format!("{} {}", icon, label);
        
        // Custom tab button with golden underline
        let resp = tab_button(ui, &tab_text, is_active);
        if resp.clicked() {
          state.active_tab = tab.clone();
        }
      }
    });
  });

  // Golden separator line under tab bar
  ui.painter().hline(
    ui.clip_rect().x_range(),
    ui.cursor().top(),
    egui::Stroke::new(1.0_f32, ChimeraTheme::BORDER),
  );
}

/// A single tab button with ChimeraTool-style golden underline indicator
fn tab_button(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
  // Estimate width via layout (works on &Fonts unlike glyph_width which needs &mut)
  let galley = ui.painter().layout_no_wrap(
      label.into(),
      egui::FontId::proportional(13.0),
      egui::Color32::WHITE,
  );
  let desired = egui::vec2(galley.size().x + 18.0, 38.0);
  let desired = egui::vec2(desired.x.max(60.0).min(120.0), 38.0);
  let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

  if ui.is_rect_visible(rect) {
    let painter = ui.painter();

    // Hover / active background
    if active {
      painter.rect_filled(
        rect,
        egui::CornerRadius::same(0),
        ChimeraTheme::TAB_ACTIVE_BG,
      );
    } else if response.hovered() {
      painter.rect_filled(
        rect,
        egui::CornerRadius::same(0),
        egui::Color32::from_rgb(0x1a, 0x1c, 0x28),
      );
    }

    // Label text
    let text_color = if active {
      ChimeraTheme::ACCENT
    } else if response.hovered() {
      ChimeraTheme::TEXT_PRIMARY
    } else {
      ChimeraTheme::TEXT_SECONDARY
    };
    painter.text(
      rect.center(),
      egui::Align2::CENTER_CENTER,
      label,
      egui::FontId::proportional(12.5),
      text_color,
    );

    // Golden underline for active tab
    if active {
      painter.rect_filled(
        egui::Rect::from_min_size(
          rect.left_bottom() - egui::vec2(0.0, 2.5),
          egui::vec2(rect.width(), 2.5),
        ),
        egui::CornerRadius::same(1),
        ChimeraTheme::ACCENT,
      );
    }
  }

  response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Route to the correct panel based on active tab
fn route_content(
  ui:    &mut egui::Ui,
  state:   &mut AppState,
  op_tx:   &Sender<OperationRequest>,
  device_db: &DeviceDatabase,
) {
  match state.active_tab {
    // Panels that don't need a device
    ActiveTab::Apple         => apple_panel::render_apple_panel(ui, state),
    ActiveTab::AuNetworkUnlock => au_unlock_panel::render_au_unlock_panel(ui, state),
    ActiveTab::ApiTools      => api_panel::render_api_panel(ui, state, op_tx),
    ActiveTab::ShshManager   => shsh_panel::render_shsh_panel(ui, state, op_tx),
    ActiveTab::Settings      => settings_panel::render_settings(ui, state),
    ActiveTab::History       => history_panel::render_history(ui, state, op_tx),
    ActiveTab::Log           => log_panel::render_log(ui, state),
    ActiveTab::MediaTek      => mediatek_panel::render_mediatek_panel(ui, state),

    // Panels that show device-specific content
    ActiveTab::DeviceInfo => {
      if let Some(id) = &state.selected_device_id.clone() {
        device_info::render_device_info(ui, state, id, op_tx);
      } else {
        no_device_hint(ui);
      }
    }
    ActiveTab::Operations => {
      if let Some(id) = &state.selected_device_id.clone() {
        operations::render_operations(ui, state, id, op_tx);
      } else {
        no_device_hint(ui);
      }
    }
    ActiveTab::Firmware => {
      firmware_panel::render_firmware(ui, state, op_tx, device_db)
    }
    ActiveTab::Utilities => {
      utilities_panel::render_utilities(ui, state, op_tx)
    }
    ActiveTab::Diagnostics => {
      if let Some(id) = &state.selected_device_id.clone() {
        diagnostics_panel::render_diagnostics(ui, state, id, op_tx);
      } else {
        no_device_hint(ui);
      }
    }
  }
}

fn no_device_hint(ui: &mut egui::Ui) {
  ui.add_space(60.0);
  ui.vertical_centered(|ui| {
    ui.colored_label(
      ChimeraTheme::ACCENT,
      RichText::new("Device").size(48.0),
    );
    ui.add_space(12.0);
    ui.colored_label(
      ChimeraTheme::TEXT_SECONDARY,
      RichText::new("No device selected").size(16.0),
    );
    ui.add_space(6.0);
    ui.colored_label(
      ChimeraTheme::TEXT_DISABLED, "Connect a device via USB and select it from the sidebar.",
    );
    ui.add_space(6.0);
    ui.colored_label(
      ChimeraTheme::TEXT_DISABLED, "Apple devices connect via USB without ADB.",
    );
  });
}
pub mod pages_additions;
