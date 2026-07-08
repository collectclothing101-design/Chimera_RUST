// crates/chimera-gui/src/ui/network_page.rs
// Network page — matches HTML pg-nwk (DEVICE group)
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
use egui::RichText;
use crate::theme::ChimeraTheme;
use crate::state::AppState;

pub fn render_network_page(ui: &mut egui::Ui, state: &mut AppState) {
    crate::app::page_header(ui, "05 · DEVICE", "Network",
        "Wi-Fi status · ADB over TCP · TCP port test");
    ui.columns(2, |cols| {
        // ADB TCP
        crate::app::card_frame().show(&mut cols[0], |ui| {
            crate::app::section_hd(ui, "ADB TCP Connect");
            crate::app::field_lbl(ui, "Host", &mut state.adb_tcp_host, "192.168.0.1");
            crate::app::field_lbl(ui, "Port", &mut state.adb_tcp_port, "5555");
            ui.horizontal(|ui| {
                crate::app::btn_p(ui, "Connect");
                crate::app::btn_s(ui, "Disconnect");
            });
        });
        // TCP port test
        crate::app::card_frame().show(&mut cols[1], |ui| {
            crate::app::section_hd(ui, "Custom Port Test");
            ui.label(RichText::new("Test an arbitrary TCP port for device connectivity.")
                .size(9.5).color(ChimeraTheme::T2));
            ui.add_space(6.0);
            crate::app::field_lbl(ui, "Host", &mut state.tcp_test_host, "10.0.0.1");
            crate::app::field_lbl(ui, "Port", &mut state.tcp_test_port, "9001");
            crate::app::btn_s(ui, "Test Port");
        });
    });
}

// ════════════════════════════════════════════════════════════════
// crates/chimera-gui/src/ui/tools_page.rs
// Tools page — matches HTML pg-tls (DEVICE group)
// ════════════════════════════════════════════════════════════════
pub fn render_tools_page(ui: &mut egui::Ui, _state: &mut AppState) {
    crate::app::page_header(ui, "06 · DEVICE", "Tools",
        "Device certificate · escrow proxy · attestation tools");
    crate::app::note_i(ui, "Apple GSX / AL repair check, attestation, and escrow proxy tools appear here when a device is connected.");
    crate::app::empty_state(ui, "⚒", "No Device Connected",
        "Connect an Apple or Android device to access device certificate and attestation tools.");
}

// State additions for network page:
// pub tcp_test_host:  String,
// pub tcp_test_port:  String,
