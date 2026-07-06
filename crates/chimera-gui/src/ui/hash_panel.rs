// chimera-gui/src/ui/hash_panel.rs
// Hash Generation tab (ut3) — matches chimera-gui.html utilities tab 3
// SHA-256 · SHA-1 · MD5 · CRC-32 · HMAC-SHA256
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui::{self, RichText, Color32};
use crate::state::AppState;

pub fn render_hash_tab(ui: &mut egui::Ui, state: &mut AppState) {
    ui.set_max_width(540.0);

    section_header(ui, "Hash Generation");

    // Input field
    ui.label(RichText::new("Input Data").size(9.5)
        .color(Color32::from_rgb(148, 150, 168)));
    ui.add(egui::TextEdit::multiline(&mut state.hash_input)
        .desired_rows(4)
        .desired_width(f32::INFINITY)
        .hint_text("Enter data or paste file contents…"));
    ui.add_space(8.0);

    // Algorithm buttons
    ui.horizontal_wrapped(|ui| {
        for algo in &["SHA-256", "SHA-1", "MD5", "CRC-32", "HMAC-SHA256"] {
            if ui.button(*algo).clicked() {
                state.hash_result = compute_hash(algo, &state.hash_input, &state.hash_hmac_key);
                state.hash_algo = algo.to_string();
                state.add_log(crate::state::LogEntry::info(
                    format!("{} computed: {}…", algo, &state.hash_result.chars().take(16).collect::<String>())
                ));
            }
        }
    });

    // HMAC key field (shown only when HMAC selected)
    if state.hash_algo == "HMAC-SHA256" {
        ui.add_space(6.0);
        ui.label(RichText::new("HMAC Key (hex or text)").size(9.5)
            .color(Color32::from_rgb(148, 150, 168)));
        ui.add(egui::TextEdit::singleline(&mut state.hash_hmac_key)
            .desired_width(f32::INFINITY)
            .hint_text("secret key…"));
    }

    ui.add_space(10.0);

    // Output
    ui.label(RichText::new("Output Hash").size(9.5)
        .color(Color32::from_rgb(148, 150, 168)));
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut state.hash_result.clone())
            .desired_width(ui.available_width() - 70.0)
            .font(egui::TextStyle::Monospace));
        if ui.button("Copy").clicked() {
            ui.output_mut(|o| o.commands.push(egui::output::OutputCommand::CopyText(state.hash_result.clone())));
        }
    });

    if !state.hash_result.is_empty() && !state.hash_algo.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new(format!("Algorithm: {}  ·  {} chars",
            state.hash_algo, state.hash_result.len()))
            .size(9.0).color(Color32::from_rgb(89, 100, 114)));
    }
}

fn compute_hash(algo: &str, input: &str, hmac_key: &str) -> String {
    use std::fmt::Write;

    match algo {
        "SHA-256" => {
            use sha2::{Sha256, Digest};
            let mut h = Sha256::new();
            h.update(input.as_bytes());
            let result = h.finalize();
            result.iter().fold(String::new(), |mut s, b| { write!(s, "{:02x}", b).ok(); s })
        }
        "SHA-1" => {
            use sha1::{Sha1, Digest};
            let mut h = Sha1::new();
            h.update(input.as_bytes());
            let result = h.finalize();
            result.iter().fold(String::new(), |mut s, b| { write!(s, "{:02x}", b).ok(); s })
        }
        "MD5" => {
            use md5::{Md5, Digest};
            let mut h = Md5::new();
            h.update(input.as_bytes());
            let result = h.finalize();
            result.iter().fold(String::new(), |mut s, b| { write!(s, "{:02x}", b).ok(); s })
        }
        "CRC-32" => {
            let crc = crc32fast::hash(input.as_bytes());
            format!("{:08x}", crc)
        }
        "HMAC-SHA256" => {
            use hmac::{Hmac, Mac};
            use sha2::Sha256;
            type HmacSha256 = Hmac<Sha256>;
            let key = if hmac_key.is_empty() { b"chimera".as_ref() } else { hmac_key.as_bytes() };
            let mut mac = HmacSha256::new_from_slice(key)
                .unwrap_or_else(|_| HmacSha256::new_from_slice(b"chimera").unwrap());
            mac.update(input.as_bytes());
            let result = mac.finalize().into_bytes();
            result.iter().fold(String::new(), |mut s, b| { write!(s, "{:02x}", b).ok(); s })
        }
        _ => String::new(),
    }
}

// ── State fields to add to AppState ────────────────────────────────────────
// pub hash_input:     String,
// pub hash_result:    String,
// pub hash_algo:      String,
// pub hash_hmac_key:  String,

fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(9.0).strong()
            .color(Color32::from_rgb(89, 100, 114)));
    });
    ui.add_space(6.0);
}
