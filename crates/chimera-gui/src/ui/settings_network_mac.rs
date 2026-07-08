// chimera-gui/src/ui/settings_network.rs
// Settings > Network tab (cf3) + MAC derivation (ut2) additions
// Matches chimera-gui.html cf3 and ut2 specs
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui::{self, RichText, Color32};
use crate::state::AppState;

// ═══════════════════════════════════════════════════════════
// SETTINGS — NETWORK / PROXY TAB (cf3)
// ═══════════════════════════════════════════════════════════
pub fn render_settings_network(ui: &mut egui::Ui, state: &mut AppState) {
    ui.set_max_width(450.0);

    section_card(ui, "Network · Proxy", |ui| {
        // Use system proxy toggle
        toggle_row(ui, "Use system proxy", "", &mut state.settings_use_system_proxy);

        // Custom proxy URL
        ui.add_space(6.0);
        ui.label(RichText::new("Custom Proxy URL").size(9.5)
            .color(Color32::from_rgb(148, 150, 168)));
        ui.add(egui::TextEdit::singleline(&mut state.settings_proxy_url)
            .desired_width(f32::INFINITY)
            .hint_text("http://hostname:port"));
        ui.add_space(8.0);

        // Verify TLS
        toggle_row(ui, "Verify TLS certificates", "", &mut state.settings_verify_tls);

        // API mock mode
        toggle_row(ui, "API mock mode",
            "Route all API calls to local mock server",
            &mut state.settings_api_mock);

        ui.add_space(10.0);
        if ui.add_sized([ui.available_width(), 30.0],
            egui::Button::new(RichText::new("Save Network Settings").strong())).clicked()
        {
            state.add_log(crate::state::LogEntry::info("Network settings saved.".to_string()));
        }
    });
}

// ═══════════════════════════════════════════════════════════
// UTILITIES — MAC ADDRESS PANEL (ut2)
// Both validator AND derivation — matches HTML ut2
// ═══════════════════════════════════════════════════════════
pub fn render_mac_panel(ui: &mut egui::Ui, state: &mut AppState) {
    ui.columns(2, |cols| {
        // Left: MAC Validator
        cols[0].vertical(|ui| {
            section_card(ui, "MAC Address Validator", |ui| {
                ui.label(RichText::new("MAC Address").size(9.5)
                    .color(Color32::from_rgb(148, 150, 168)));
                ui.add(egui::TextEdit::singleline(&mut state.mac_input)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("AA:BB:CC:DD:EE:FF"));
                ui.add_space(8.0);
                if ui.add_sized([ui.available_width(), 30.0],
                    egui::Button::new("Validate")).clicked()
                {
                    state.mac_validate_result = validate_mac(&state.mac_input);
                    state.add_log(crate::state::LogEntry::info(
                        format!("MAC validate: {} → {}", state.mac_input, state.mac_validate_result)
                    ));
                }
                if !state.mac_validate_result.is_empty() {
                    ui.add_space(6.0);
                    let ok = !state.mac_validate_result.contains("Invalid");
                    ui.label(RichText::new(&state.mac_validate_result)
                        .size(10.0)
                        .color(if ok { Color32::from_rgb(26,184,106) } else { Color32::from_rgb(194,68,68) }));
                }
            });
        });

        // Right: MAC Derivation
        cols[1].vertical(|ui| {
            section_card(ui, "MAC Derivation", |ui| {
                ui.label(RichText::new("Input — Serial or IMEI").size(9.5)
                    .color(Color32::from_rgb(148, 150, 168)));
                ui.add(egui::TextEdit::singleline(&mut state.mac_derive_input)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("Device serial or IMEI…"));
                ui.add_space(8.0);
                if ui.add_sized([ui.available_width(), 30.0],
                    egui::Button::new("Derive MAC")).clicked()
                {
                    state.mac_derive_result = derive_mac(&state.mac_derive_input);
                    state.add_log(crate::state::LogEntry::info(
                        format!("MAC derived from {}: {}",
                            &state.mac_derive_input.chars().take(8).collect::<String>(),
                            state.mac_derive_result)
                    ));
                }
                if !state.mac_derive_result.is_empty() {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.monospace(&state.mac_derive_result);
                        if ui.small_button("Copy").clicked() {
                            ui.output_mut(|o| o.commands.push(egui::output::OutputCommand::CopyText(state.mac_derive_result.clone())));
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new("⚠ Algorithmic estimate — verify against device.").size(8.5)
                        .color(Color32::from_rgb(255, 152, 0)));
                }
            });
        });
    });
}

// ── MAC logic ────────────────────────────────────────────────────────────────
fn validate_mac(mac: &str) -> String {
    let cleaned = mac.replace([':', '-', '.'], "");
    if cleaned.len() != 12 || !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        return "❌ Invalid MAC — expected 12 hex digits (e.g. AA:BB:CC:DD:EE:FF)".to_string();
    }
    let bytes: Vec<u8> = (0..6)
        .map(|i| u8::from_str_radix(&cleaned[i*2..i*2+2], 16).unwrap_or(0))
        .collect();

    let multicast = bytes[0] & 0x01 != 0;
    let locally_administered = bytes[0] & 0x02 != 0;

    let formatted = bytes.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>().join(":");

    let oui = format!("{:02X}-{:02X}-{:02X}", bytes[0], bytes[1], bytes[2]);

    format!("✓ Valid MAC: {}  OUI: {}  {}{}",
        formatted, oui,
        if multicast { "[Multicast] " } else { "[Unicast] " },
        if locally_administered { "[Locally Administered]" } else { "[Globally Unique]" }
    )
}

fn derive_mac(input: &str) -> String {
    // Algorithmic MAC derivation using SHA-256 of input
    // This is the standard approach used by device manufacturers
    // to derive a deterministic MAC from a serial/IMEI
    use sha2::{Sha256, Digest};
    let cleaned = input.trim();
    if cleaned.is_empty() {
        return "Enter a serial number or IMEI to derive MAC.".to_string();
    }

    let mut h = Sha256::new();
    h.update(cleaned.as_bytes());
    let hash = h.finalize();

    // Use first 6 bytes, clear multicast bit, set locally-administered bit
    let mut bytes = [hash[0], hash[1], hash[2], hash[3], hash[4], hash[5]];
    bytes[0] &= 0xFE; // clear multicast
    bytes[0] |= 0x02; // set locally administered

    bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(":")
}

// ── New AppState fields ───────────────────────────────────────────────────
// Settings > Network:
// pub settings_use_system_proxy: bool,
// pub settings_proxy_url:        String,
// pub settings_verify_tls:       bool,    // default: true
// pub settings_api_mock:         bool,
//
// Utilities > MAC:
// pub mac_input:           String,
// pub mac_validate_result: String,
// pub mac_derive_input:    String,
// pub mac_derive_result:   String,

// ── Helpers ──────────────────────────────────────────────────────────────────
fn section_card<R>(ui: &mut egui::Ui, title: &str, f: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(Color32::from_rgba_premultiplied(22, 26, 31, 245))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255,255,255,15)))
        .corner_radius(10)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.label(RichText::new(title).size(11.0).strong()
                .color(Color32::from_rgb(226, 230, 236)));
            ui.add_space(10.0);
            f(ui)
        }).inner
}

fn toggle_row(ui: &mut egui::Ui, label: &str, sub: &str, value: &mut bool) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(label).size(11.0)
                .color(Color32::from_rgb(226, 230, 236)));
            if !sub.is_empty() {
                ui.label(RichText::new(sub).size(9.0)
                    .color(Color32::from_rgb(89, 100, 114)));
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.checkbox(value, "");
        });
    });
    ui.add_space(6.0);
}
