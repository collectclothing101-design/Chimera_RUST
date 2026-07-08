// crates/chimera-gui/src/ui/pages_additions.rs
// Real implementations for previously-stubbed page sections.
// These replace or supplement the placeholder blocks in pages.rs.
// Phases 5-11: SSH terminal, encode/decode, network tools, history,
//              downloads, settings wiring, SHSH/futurerestore, API tools.
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_mut)]
use eframe::egui::{self, Color32, RichText, ScrollArea};
use crate::theme::C;
use crate::state::AppState;
use crate::worker::OperationRequest;
use crossbeam_channel::Sender;

// ══════════════════════════════════════════════════════════════════════════
// HISTORY PAGE — real data from state.history
// ══════════════════════════════════════════════════════════════════════════

pub fn render_history_content(ui: &mut egui::Ui, state: &mut AppState) {
    // Filter bar
    ui.horizontal(|ui| {
        ui.label(RichText::new("🔍").size(11.0).color(C::T2));
        ui.add(egui::TextEdit::singleline(&mut state.history_filter)
            .hint_text("Filter by device, operation, result…")
            .desired_width(280.0));
        if !state.history_filter.is_empty() {
            if ui.small_button("✕").clicked() {
                state.history_filter.clear();
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("🗑 Clear").clicked() {
                state.history.clear();
                let _ = crate::persistence::save_history(&[]);
            }
            ui.label(RichText::new(format!("{} entries", state.history.len()))
                .size(9.0).color(C::T3));
        });
    });
    ui.add_space(8.0);

    if state.history.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new("No history yet — completed operations appear here.")
                .size(11.0).color(C::T3));
        });
        return;
    }

    let filter = state.history_filter.clone();
    let entries: Vec<_> = state.history.iter().rev()
        .filter(|e| e.matches_filter(&filter))
        .collect();

    ScrollArea::vertical().id_salt("hist_scroll").show(ui, |ui| {
        for entry in entries {
            history_row(ui, entry);
        }
    });
}

fn history_row(ui: &mut egui::Ui, entry: &crate::history::HistoryEntry) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(0x10, 0x13, 0x17))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 12)))
        .corner_radius(6)
        .inner_margin(egui::Margin { left: 10, right: 10, top: 6, bottom: 6 })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Result badge
                let col  = entry.result.color();
                let lbl  = entry.result.label();
                egui::Frame::NONE
                    .fill(Color32::from_rgba_premultiplied(col.r(), col.g(), col.b(), 20))
                    .stroke(egui::Stroke::new(1.0_f32, col))
                    .corner_radius(4)
                    .inner_margin(egui::Margin { left:6, right:6, top:2, bottom:2 })
                    .show(ui, |ui| {
                        ui.label(RichText::new(lbl).size(8.5).strong().color(col));
                    });

                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(&entry.operation).size(11.0).strong().color(C::T0));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(
                            format!("{} {}  ·  {}", entry.device_brand, entry.device_model, entry.serial)
                        ).size(9.5).color(C::T2));
                    });
                    if !entry.notes.is_empty() {
                        ui.label(RichText::new(&entry.notes).size(9.5).color(C::T3));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    ui.label(RichText::new(&entry.timestamp).size(9.0).color(C::T3)
                        .family(egui::FontFamily::Monospace));
                });
            });
        });
    ui.add_space(4.0);
}

// ══════════════════════════════════════════════════════════════════════════
// SSH TERMINAL (Phase 5)
// ══════════════════════════════════════════════════════════════════════════

pub fn render_ssh_terminal(
    ui: &mut egui::Ui,
    state: &mut AppState,
    op_tx: &Sender<OperationRequest>,
) {
    // Connection form (shown when not connected)
    if !state.ssh_connected {
        ui.columns(2, |cols| {
            cols[0].add(egui::TextEdit::singleline(&mut state.ssh_host)
                .hint_text("Host / IP").desired_width(f32::INFINITY));
            cols[1].horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut state.ssh_port)
                    .hint_text("Port").desired_width(68.0));
                ui.add_space(4.0);
                ui.add(egui::TextEdit::singleline(&mut state.ssh_username)
                    .hint_text("User").desired_width(f32::INFINITY));
            });
        });
        ui.add_space(6.0);

        // Auth method
        ui.horizontal(|ui| {
            ui.label(RichText::new("Auth:").size(10.0).color(C::T2));
            for m in &["Password", "Key File"] {
                if ui.selectable_label(state.ssh_auth_method == *m, *m).clicked() {
                    state.ssh_auth_method = m.to_string();
                }
            }
        });
        ui.add_space(4.0);

        if state.ssh_auth_method == "Password" {
            ui.add(egui::TextEdit::singleline(&mut state.ssh_password)
                .hint_text("Password")
                .password(true)
                .desired_width(f32::INFINITY));
        } else {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut state.ssh_key_path)
                    .hint_text("Key file path").desired_width(f32::INFINITY));
                if ui.small_button("📂").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_file() {
                        state.ssh_key_path = p.to_string_lossy().to_string();
                    }
                }
            });
            ui.add(egui::TextEdit::singleline(&mut state.ssh_passphrase)
                .hint_text("Passphrase (optional)")
                .password(true)
                .desired_width(f32::INFINITY));
        }
        ui.add_space(10.0);

        let port: u16 = state.ssh_port.parse().unwrap_or(22);
        if ui.add(
            egui::Button::new(RichText::new("⟶ Connect SSH").size(10.5).strong().color(Color32::BLACK))
                .fill(C::A).corner_radius(6).min_size(egui::vec2(130.0, 30.0))
        ).clicked() {
            let _ = op_tx.send(OperationRequest::SshConnect {
                host:        state.ssh_host.clone(),
                port,
                username:    state.ssh_username.clone(),
                auth_method: state.ssh_auth_method.clone(),
                password:    state.ssh_password.clone(),
                key_path:    state.ssh_key_path.clone(),
                passphrase:  state.ssh_passphrase.clone(),
            });
        }
        return;
    }

    // ── Connected: show terminal ──────────────────────────────────────
    ui.horizontal(|ui| {
        egui::Frame::NONE
            .fill(Color32::from_rgb(0x06, 0x12, 0x06))
            .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(0x20, 0x70, 0x20)))
            .corner_radius(4)
            .inner_margin(egui::Margin { left:6, right:6, top:2, bottom:2 })
            .show(ui, |ui| {
                ui.label(RichText::new("● CONNECTED").size(8.5).strong()
                    .color(Color32::from_rgb(0x40, 0xff, 0x40)));
            });
        ui.label(RichText::new(format!("{}@{}:{}", state.ssh_username, state.ssh_host, state.ssh_port))
            .size(10.0).color(C::T1).family(egui::FontFamily::Monospace));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Disconnect").clicked() {
                let _ = op_tx.send(OperationRequest::SshDisconnect);
            }
        });
    });
    ui.add_space(6.0);

    // Terminal output
    let avail_h = ui.available_height() - 44.0;
    egui::Frame::NONE
        .fill(Color32::from_rgb(0x08, 0x0a, 0x08))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(0x18, 0x28, 0x18)))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("ssh_term")
                .max_height(avail_h)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut state.ssh_terminal_output.clone())
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .interactive(false)
                    );
                });
        });
    ui.add_space(6.0);

    // Command input
    let mut send_cmd = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("❯").size(11.0).color(Color32::from_rgb(0x40, 0xff, 0x40)));
        let resp = ui.add(
            egui::TextEdit::singleline(&mut state.ssh_command_input)
                .desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace)
                .hint_text("Enter command…")
        );
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            send_cmd = true;
        }
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
            if state.ssh_history_idx < state.ssh_command_history.len() {
                state.ssh_history_idx += 1;
                let idx = state.ssh_command_history.len() - state.ssh_history_idx;
                state.ssh_command_input = state.ssh_command_history.get(idx)
                    .cloned().unwrap_or_default();
            }
        }
        if ui.small_button("Send").clicked() { send_cmd = true; }
    });

    if send_cmd && !state.ssh_command_input.is_empty() {
        let cmd = state.ssh_command_input.clone();
        // Send to SSH worker via the live channel
        if let Some(tx) = &state.ssh_input_tx {
            let _ = tx.send(cmd.clone());
        }
        // Append to local echo
        state.ssh_terminal_output.push_str(&format!("❯ {}\n", cmd));
        state.ssh_command_history.push(cmd);
        state.ssh_command_input.clear();
        state.ssh_history_idx = 0;
    }
}

// ══════════════════════════════════════════════════════════════════════════
// ENCODE / DECODE TOOLS (Phase 10)
// ══════════════════════════════════════════════════════════════════════════

pub fn render_encode_decode(ui: &mut egui::Ui, state: &mut AppState) {
    // Tab bar
    ui.horizontal(|ui| {
        for (idx, lbl) in ["Base64", "Hex", "URL", "JWT", "NumConv"].iter().enumerate() {
            if ui.selectable_label(state.encode_tab == idx as u8, *lbl).clicked() {
                state.encode_tab = idx as u8;
                state.encode_input.clear();
                state.encode_output.clear();
            }
        }
    });
    ui.separator();
    ui.add_space(8.0);

    match state.encode_tab {
        // ── Base64 ───────────────────────────────────────────────────
        0 => {
            kv_label(ui, "Input");
            ui.add(egui::TextEdit::multiline(&mut state.encode_input)
                .desired_rows(4).desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("⇒ Encode Base64").clicked() {
                    use base64::Engine as _;
                    state.encode_output = base64::engine::general_purpose::STANDARD
                        .encode(state.encode_input.as_bytes());
                }
                if ui.button("⇐ Decode Base64").clicked() {
                    use base64::Engine as _;
                    state.encode_output = base64::engine::general_purpose::STANDARD
                        .decode(state.encode_input.trim().as_bytes())
                        .map(|b| String::from_utf8_lossy(&b).to_string())
                        .unwrap_or_else(|e| format!("Error: {}", e));
                }
                if !state.encode_output.is_empty() {
                    if ui.small_button("📋 Copy").clicked() {
                        ui.ctx().copy_text(state.encode_output.clone());
                    }
                }
            });
            if !state.encode_output.is_empty() {
                ui.add_space(6.0);
                kv_label(ui, "Output");
                egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                    ui.add(egui::TextEdit::multiline(&mut state.encode_output)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace));
                });
            }
        }
        // ── Hex ──────────────────────────────────────────────────────
        1 => {
            kv_label(ui, "Input (text or hex string)");
            ui.add(egui::TextEdit::multiline(&mut state.encode_input)
                .desired_rows(4).desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("String → Hex").clicked() {
                    state.encode_output = hex::encode(state.encode_input.as_bytes());
                }
                if ui.button("Hex → String").clicked() {
                    let clean: String = state.encode_input.chars()
                        .filter(|c| c.is_ascii_hexdigit()).collect();
                    state.encode_output = hex::decode(&clean).map(|b| String::from_utf8_lossy(b.as_slice()).to_string())
                        .unwrap_or_else(|e| format!("Error: {}", e));
                }
                if !state.encode_output.is_empty() && ui.small_button("📋 Copy").clicked() {
                    ui.ctx().copy_text(state.encode_output.clone());
                }
            });
            if !state.encode_output.is_empty() {
                ui.add_space(6.0);
                kv_label(ui, "Output");
                ui.add(egui::TextEdit::multiline(&mut state.encode_output)
                    .desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
            }
        }
        // ── URL ──────────────────────────────────────────────────────
        2 => {
            kv_label(ui, "Input (string or URL-encoded)");
            ui.add(egui::TextEdit::multiline(&mut state.encode_input)
                .desired_rows(3).desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("URL Encode").clicked() {
                    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
                    state.encode_output = utf8_percent_encode(
                        &state.encode_input, NON_ALPHANUMERIC).to_string();
                }
                if ui.button("URL Decode").clicked() {
                    use percent_encoding::percent_decode_str;
                    state.encode_output = percent_decode_str(&state.encode_input)
                        .decode_utf8_lossy().to_string();
                }
                if !state.encode_output.is_empty() && ui.small_button("📋 Copy").clicked() {
                    ui.ctx().copy_text(state.encode_output.clone());
                }
            });
            if !state.encode_output.is_empty() {
                ui.add_space(6.0);
                kv_label(ui, "Output");
                ui.add(egui::TextEdit::multiline(&mut state.encode_output)
                    .desired_width(f32::INFINITY).font(egui::TextStyle::Monospace));
            }
        }
        // ── JWT ──────────────────────────────────────────────────────
        3 => {
            kv_label(ui, "JWT Token");
            ui.add(egui::TextEdit::singleline(&mut state.jwt_input)
                .desired_width(f32::INFINITY)
                .hint_text("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9…")
                .font(egui::TextStyle::Monospace));
            ui.add_space(6.0);
            if ui.button("Decode JWT (no verify)").clicked() {
                decode_jwt(state);
            }
            ui.add_space(8.0);
            if !state.jwt_header.is_empty() {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        kv_label(ui, "Header");
                        ui.add(egui::TextEdit::multiline(&mut state.jwt_header)
                            .desired_rows(4).desired_width(240.0)
                            .font(egui::TextStyle::Monospace));
                    });
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        kv_label(ui, "Payload");
                        ui.add(egui::TextEdit::multiline(&mut state.jwt_payload)
                            .desired_rows(4).desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace));
                    });
                });
            }
        }
        // ── NumConv ──────────────────────────────────────────────────
        _ => {
            kv_label(ui, "Number (decimal, hex 0x…, or binary 0b…)");
            let changed = ui.add(
                egui::TextEdit::singleline(&mut state.numconv_input)
                    .desired_width(280.0)
                    .font(egui::TextStyle::Monospace)
            ).changed();
            if changed {
                state.encode_output = numconv_all(&state.numconv_input);
            }
            if !state.encode_output.is_empty() {
                ui.add_space(8.0);
                for line in state.encode_output.clone().lines() {
                    ui.horizontal(|ui| {
                        let parts: Vec<&str> = line.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            ui.label(RichText::new(format!("{}:", parts[0]))
                                .size(10.0).color(C::T2));
                            ui.label(RichText::new(parts[1].trim())
                                .size(10.0).color(C::T0)
                                .family(egui::FontFamily::Monospace));
                        }
                    });
                }
            }
        }
    }
}

fn decode_jwt(state: &mut AppState) {
    use base64::Engine as _;
    let parts: Vec<&str> = state.jwt_input.trim().splitn(3, '.').collect();
    if parts.len() < 2 {
        state.jwt_header  = "Error: not a valid JWT (need 3 parts)".into();
        state.jwt_payload = String::new();
        return;
    }
    let decode_part = |s: &str| -> String {
        let padded = match s.len() % 4 {
            2 => format!("{}==", s),
            3 => format!("{}=", s),
            _ => s.to_string(),
        };
        let url = padded.replace('-', "+").replace('_', "/");
        base64::engine::general_purpose::STANDARD
            .decode(url.as_bytes())
            .map(|b| {
                let raw = String::from_utf8_lossy(&b).to_string();
                // Pretty print JSON
                serde_json::from_str::<serde_json::Value>(&raw)
                    .map(|v| serde_json::to_string_pretty(&v).unwrap_or(raw.clone()))
                    .unwrap_or(raw)
            })
            .unwrap_or_else(|e| format!("Decode error: {}", e))
    };
    state.jwt_header  = decode_part(parts[0]);
    state.jwt_payload = decode_part(parts[1]);
}

fn numconv_all(s: &str) -> String {
    let s = s.trim();
    let val: Option<i64> = if s.starts_with("0x") || s.starts_with("0X") {
        i64::from_str_radix(&s[2..], 16).ok()
    } else if s.starts_with("0b") || s.starts_with("0B") {
        i64::from_str_radix(&s[2..], 2).ok()
    } else {
        s.parse().ok()
    };
    match val {
        Some(n) => format!(
            "Decimal: {}\nHex:     0x{:X}\nBinary:  0b{:b}\nOctal:   0o{:o}",
            n, n, n, n
        ),
        None => "Invalid number".into(),
    }
}

// ══════════════════════════════════════════════════════════════════════════
// NETWORK TOOLS (Phase 8)
// ══════════════════════════════════════════════════════════════════════════

pub fn render_network_tools_content(
    ui: &mut egui::Ui,
    state: &mut AppState,
    op_tx: &Sender<OperationRequest>,
) {
    // Tab bar
    ui.horizontal(|ui| {
        for (i, lbl) in ["TCP Test", "DNS Lookup", "Proxy"].iter().enumerate() {
            if ui.selectable_label(state.network_tab == i, *lbl).clicked() {
                state.network_tab = i;
            }
        }
    });
    ui.separator();
    ui.add_space(10.0);

    match state.network_tab {
        // ── TCP test ─────────────────────────────────────────────────
        0 => {
            kv_label(ui, "Host / IP");
            ui.add(egui::TextEdit::singleline(&mut state.tcp_test_host)
                .hint_text("192.168.0.1 or hostname").desired_width(280.0));
            ui.add_space(4.0);
            kv_label(ui, "Port");
            ui.add(egui::TextEdit::singleline(&mut state.tcp_test_port)
                .hint_text("9001").desired_width(80.0));
            ui.add_space(8.0);
            if ui.add(
                egui::Button::new(RichText::new("▶ Test Connection").size(10.5).strong())
                    .fill(C::A).corner_radius(6).min_size(egui::vec2(140.0, 30.0))
            ).clicked() {
                let port: u16 = state.tcp_test_port.parse().unwrap_or(80);
                let _ = op_tx.send(OperationRequest::TcpTest {
                    host: state.tcp_test_host.clone(), port,
                });
                state.tcp_test_result = "Testing…".into();
            }
            if !state.tcp_test_result.is_empty() {
                ui.add_space(8.0);
                let col = if state.tcp_test_result.contains("OPEN") { C::G } else { C::R };
                egui::Frame::NONE
                    .fill(Color32::from_rgba_premultiplied(col.r(), col.g(), col.b(), 15))
                    .stroke(egui::Stroke::new(1.0_f32, col))
                    .corner_radius(6)
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(RichText::new(&state.tcp_test_result)
                            .size(11.0).color(col).family(egui::FontFamily::Monospace));
                    });
            }
        }
        // ── DNS lookup ───────────────────────────────────────────────
        1 => {
            kv_label(ui, "Hostname");
            ui.add(egui::TextEdit::singleline(&mut state.dns_lookup_host)
                .hint_text("api.chimeratool.com").desired_width(280.0));
            ui.add_space(8.0);
            if ui.add(
                egui::Button::new(RichText::new("▶ Resolve").size(10.5).strong())
                    .fill(C::A).corner_radius(6).min_size(egui::vec2(100.0, 30.0))
            ).clicked() {
                let _ = op_tx.send(OperationRequest::DnsLookup {
                    hostname: state.dns_lookup_host.clone(),
                });
                state.dns_lookup_result = "Resolving…".into();
            }
            if !state.dns_lookup_result.is_empty() {
                ui.add_space(8.0);
                let col = if state.dns_lookup_result.contains("Resolving") { C::T1 } else { C::G };
                egui::Frame::NONE
                    .fill(Color32::from_rgba_premultiplied(col.r(), col.g(), col.b(), 15))
                    .stroke(egui::Stroke::new(1.0_f32, col))
                    .corner_radius(6).inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(RichText::new(
                            format!("{} → {}", state.dns_lookup_host, state.dns_lookup_result))
                            .size(11.0).color(col).family(egui::FontFamily::Monospace));
                    });
            }
        }
        // ── Proxy settings ───────────────────────────────────────────
        _ => {
            let dirty = proxy_settings_panel(ui, state);
            if dirty { state.mark_settings_dirty(); }
        }
    }
}

fn proxy_settings_panel(ui: &mut egui::Ui, state: &mut AppState) -> bool {
    let mut dirty = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Use system proxy").size(10.5).color(C::T1));
        if ui.checkbox(&mut state.settings.use_system_proxy, "").changed() { dirty = true; }
    });
    ui.add_space(4.0);
    kv_label(ui, "Proxy URL (leave empty to bypass)");
    let r = ui.add(egui::TextEdit::singleline(&mut state.settings.proxy_url)
        .hint_text("http://proxy:8080").desired_width(280.0));
    if r.changed() { dirty = true; }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Verify TLS certificates").size(10.5).color(C::T1));
        if ui.checkbox(&mut state.settings.verify_tls, "").changed() { dirty = true; }
    });
    dirty
}

// ══════════════════════════════════════════════════════════════════════════
// DOWNLOADS PAGE — IPSW search + active downloads (Phase 6)
// ══════════════════════════════════════════════════════════════════════════

pub fn render_downloads_content(
    ui: &mut egui::Ui,
    state: &mut AppState,
    op_tx: &Sender<OperationRequest>,
) {
    // Tab bar
    ui.horizontal(|ui| {
        for (i, lbl) in ["iOS IPSW", "Samsung FW", "Active Downloads"].iter().enumerate() {
            if ui.selectable_label(state.downloads_tab2 == i, *lbl).clicked() {
                state.downloads_tab2 = i;
            }
        }
    });
    ui.separator();
    ui.add_space(10.0);

    match state.downloads_tab2 {
        0 => ipsw_search_tab(ui, state, op_tx),
        1 => samsung_fw_tab(ui, state, op_tx),
        _ => active_downloads_tab(ui, state),
    }
}

fn ipsw_search_tab(ui: &mut egui::Ui, state: &mut AppState, op_tx: &Sender<OperationRequest>) {
    kv_label(ui, "Device identifier (e.g. iPhone14,2 or just 'iPhone14')");
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut state.ipsw_search_query)
            .hint_text("iPhone14,2").desired_width(220.0));
        let searching = state.ipsw_searching;
        if ui.add_enabled(!searching,
            egui::Button::new(if searching { "Searching…" } else { "🔍 Search" })
                .fill(C::A).corner_radius(6)
        ).clicked() {
            state.ipsw_searching = true;
            state.ipsw_search_results.clear();
            let _ = op_tx.send(OperationRequest::SearchIpsw {
                model: state.ipsw_search_query.clone(),
            });
        }
    });
    ui.add_space(8.0);

    if state.ipsw_search_results.is_empty() && !state.ipsw_searching {
        ui.label(RichText::new("Search for a device identifier to see available firmware.")
            .size(10.5).color(C::T3));
        return;
    }

    let dest = state.settings.download_dir.clone();
    let results = state.ipsw_search_results.clone();
    for entry in &results {
        ipsw_row(ui, entry, &dest, op_tx);
        ui.add_space(4.0);
    }
}

fn ipsw_row(
    ui: &mut egui::Ui,
    entry: &crate::local_event::IpswEntry,
    dest: &str,
    op_tx: &Sender<OperationRequest>,
) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(0x10, 0x13, 0x17))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 12)))
        .corner_radius(6).inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Signed badge
                let (badge_col, badge_txt) = if entry.signed {
                    (Color32::from_rgb(0x20, 0xaa, 0x40), "SIGNED")
                } else {
                    (Color32::from_rgb(0x80, 0x80, 0x80), "UNSIGNED")
                };
                egui::Frame::NONE
                    .fill(Color32::from_rgba_premultiplied(
                        badge_col.r(), badge_col.g(), badge_col.b(), 25))
                    .stroke(egui::Stroke::new(1.0_f32, badge_col))
                    .corner_radius(4)
                    .inner_margin(egui::Margin { left:5, right:5, top:1, bottom:1 })
                    .show(ui, |ui| {
                        ui.label(RichText::new(badge_txt).size(8.0).color(badge_col));
                    });
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("iOS {}  —  {}", entry.version, entry.build_id))
                        .size(11.5).strong().color(C::T0));
                    ui.label(RichText::new(format!("{:.1} MB  ·  SHA1: {}",
                        entry.filesize as f64 / 1_048_576.0,
                        &entry.sha1sum[..entry.sha1sum.len().min(12)]))
                        .size(9.0).color(C::T3).family(egui::FontFamily::Monospace));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_enabled(entry.signed,
                        egui::Button::new("⬇ Download")
                            .fill(C::A).corner_radius(5)
                    ).clicked() {
                        let dest_file = format!("{}/{}_{}_Restore.ipsw",
                            dest, entry.identifier, entry.build_id);
                        let _ = op_tx.send(OperationRequest::DownloadFile {
                            id:          format!("ipsw_{}_{}", entry.identifier, entry.build_id),
                            url:         entry.url.clone(),
                            dest:        dest_file,
                            verify_sha1: Some(entry.sha1sum.clone()),
                        });
                    }
                });
            });
        });
}

fn samsung_fw_tab(ui: &mut egui::Ui, state: &mut AppState, op_tx: &Sender<OperationRequest>) {
    kv_label(ui, "Model number");
    ui.add(egui::TextEdit::singleline(&mut state.samsung_fw_model)
        .hint_text("SM-S928B").desired_width(160.0));
    ui.add_space(4.0);
    kv_label(ui, "CSC / Region");
    ui.add(egui::TextEdit::singleline(&mut state.samsung_fw_csc)
        .hint_text("OXM").desired_width(100.0));
    ui.add_space(8.0);
    if ui.button("🔍 Search Samsung Firmware").clicked() {
        // Placeholder — wire to chimera-samsung firmware DB query
        state.add_log(crate::state::LogEntry::info(
            format!("Samsung firmware search: {} / {}", state.samsung_fw_model, state.samsung_fw_csc)));
    }
}

fn active_downloads_tab(ui: &mut egui::Ui, state: &mut AppState) {
    if state.active_downloads.is_empty() {
        ui.label(RichText::new("No active downloads.")
            .size(10.5).color(C::T3));
        return;
    }
    let tasks = state.active_downloads.clone();
    for task in &tasks {
        download_row(ui, task);
        ui.add_space(4.0);
    }
}

fn download_row(ui: &mut egui::Ui, task: &crate::local_event::DownloadTask) {
    use crate::local_event::DownloadStatus;
    egui::Frame::NONE
        .fill(Color32::from_rgb(0x10, 0x13, 0x17))
        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 12)))
        .corner_radius(6).inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(&task.name).size(11.0).color(C::T0));
            ui.add_space(4.0);
            let pct = task.progress();
            ui.add(egui::ProgressBar::new(pct)
                .desired_width(f32::INFINITY)
                .text(match &task.status {
                    DownloadStatus::Running   => format!("{:.1}%", pct * 100.0),
                    DownloadStatus::Done      => "Complete".into(),
                    DownloadStatus::Failed(e) => format!("Failed: {}", e),
                    DownloadStatus::Verifying => "Verifying…".into(),
                    DownloadStatus::Queued    => "Queued".into(),
                    DownloadStatus::Cancelled => "Cancelled".into(),
                })
            );
        });
}

// ══════════════════════════════════════════════════════════════════════════
// SETTINGS PAGE — wired to state.settings + settings_dirty flag (Phase 1-C)
// ══════════════════════════════════════════════════════════════════════════

pub fn render_settings_content(ui: &mut egui::Ui, state: &mut AppState) {
    // Tab bar
    ui.horizontal(|ui| {
        for (i, lbl) in ["General", "Tools", "Network", "Appearance", "Developer"].iter().enumerate() {
            if state.settings.show_developer_options || i < 4 {
                if ui.selectable_label(state.settings_tab == i, *lbl).clicked() {
                    state.settings_tab = i;
                }
            }
        }
    });
    ui.separator();
    ui.add_space(10.0);

    let mut dirty = false;
    match state.settings_tab {
        0 => dirty = settings_general(ui, state),
        1 => dirty = settings_tools(ui, state),
        2 => dirty = settings_network(ui, state),
        3 => dirty = settings_appearance(ui, state),
        4 => dirty = settings_developer(ui, state),
        _ => {}
    }
    if dirty { state.mark_settings_dirty(); }
}

fn settings_general(ui: &mut egui::Ui, state: &mut AppState) -> bool {
    let mut dirty = false;
    section(ui, "Scan & Devices");
    dirty |= bool_row(ui, "Auto-scan on startup", &mut state.settings.auto_scan);
    dirty |= bool_row(ui, "Prevent sleep during operations", &mut state.settings.prevent_sleep);
    dirty |= bool_row(ui, "Audible alert on completion", &mut state.settings.audible_alert);
    dirty |= bool_row(ui, "Confirm dangerous operations", &mut state.settings.confirm_dangerous_ops);
    dirty |= bool_row(ui, "Auto backup before operations", &mut state.settings.auto_backup_before_ops);
    ui.add_space(8.0);

    section(ui, "Storage");
    kv_label(ui, "Download directory");
    ui.horizontal(|ui| {
        let r = ui.add(egui::TextEdit::singleline(&mut state.settings.download_dir)
            .desired_width(f32::INFINITY));
        if r.changed() { dirty = true; }
        if ui.small_button("📂").clicked() {
            if let Some(p) = rfd::FileDialog::new().pick_folder() {
                state.settings.download_dir = p.to_string_lossy().to_string();
                dirty = true;
            }
        }
    });
    kv_label(ui, "Backup directory");
    ui.horizontal(|ui| {
        let r = ui.add(egui::TextEdit::singleline(&mut state.settings.backup_dir)
            .desired_width(f32::INFINITY));
        if r.changed() { dirty = true; }
        if ui.small_button("📂").clicked() {
            if let Some(p) = rfd::FileDialog::new().pick_folder() {
                state.settings.backup_dir = p.to_string_lossy().to_string();
                dirty = true;
            }
        }
    });
    ui.add_space(8.0);

    section(ui, "Log");
    dirty |= bool_row(ui, "Write log to file", &mut state.settings.log_to_file);
    dirty |= bool_row(ui, "Show console panel", &mut state.settings.show_console);
    kv_label(ui, "Max log lines");
    let r = ui.add(egui::Slider::new(&mut state.settings.max_log_lines, 100..=10000).text("lines"));
    if r.changed() { dirty = true; }

    dirty
}

fn settings_tools(ui: &mut egui::Ui, state: &mut AppState) -> bool {
    let mut dirty = false;
    section(ui, "Tool Paths");
    dirty |= path_row(ui, "ADB binary",           &mut state.settings.adb_path,            "adb");
    dirty |= path_row(ui, "Fastboot binary",       &mut state.settings.fastboot_path,       "fastboot");
    dirty |= path_row(ui, "futurerestore binary",  &mut state.settings.futurerestore_path,  "futurerestore");
    dirty |= path_row(ui, "irecovery binary",      &mut state.settings.irecovery_path,      "irecovery");
    dirty
}

fn settings_network(ui: &mut egui::Ui, state: &mut AppState) -> bool {
    let mut dirty = false;
    section(ui, "Proxy");
    dirty |= bool_row(ui, "Use system proxy", &mut state.settings.use_system_proxy);
    kv_label(ui, "Custom proxy URL");
    let r = ui.add(egui::TextEdit::singleline(&mut state.settings.proxy_url)
        .hint_text("http://proxy:8080").desired_width(280.0));
    if r.changed() { dirty = true; }
    dirty |= bool_row(ui, "Verify TLS certificates", &mut state.settings.verify_tls);
    ui.add_space(8.0);
    section(ui, "ADB Server");
    kv_label(ui, "Host");
    let r = ui.add(egui::TextEdit::singleline(&mut state.settings.adb_server_host)
        .desired_width(160.0));
    if r.changed() { dirty = true; }
    kv_label(ui, "Port");
    let r = ui.add(egui::DragValue::new(&mut state.settings.adb_server_port)
        .range(1024u16..=65535u16));
    if r.changed() { dirty = true; }
    dirty
}

fn settings_appearance(ui: &mut egui::Ui, state: &mut AppState) -> bool {
    let mut dirty = false;
    section(ui, "Interface");
    dirty |= bool_row(ui, "Compact sidebar", &mut state.settings.compact_sidebar);
    kv_label(ui, "Font size");
    let r = ui.add(egui::Slider::new(&mut state.settings.font_size, 10.0_f32..=18.0).text("pt"));
    if r.changed() { dirty = true; }
    dirty
}

fn settings_developer(ui: &mut egui::Ui, state: &mut AppState) -> bool {
    let mut dirty = false;
    section(ui, "Developer");
    dirty |= bool_row(ui, "Show developer panel", &mut state.settings.show_developer_options);
    dirty |= bool_row(ui, "Verbose logging",       &mut state.settings.log_verbose);
    dirty |= bool_row(ui, "API mock mode",         &mut state.settings.api_mock_mode);
    ui.add_space(8.0);
    ui.label(RichText::new(format!(
        "Settings file: {}",
        crate::persistence::settings_path().display()))
        .size(9.5).color(C::T3).family(egui::FontFamily::Monospace));
    dirty
}

// ══════════════════════════════════════════════════════════════════════════
// FUTURERESTORE UI (Phase 9)
// ══════════════════════════════════════════════════════════════════════════

pub fn render_futurerestore(
    ui: &mut egui::Ui,
    state: &mut AppState,
    op_tx: &Sender<OperationRequest>,
) {
    kv_label(ui, "futurerestore binary");
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut state.settings.futurerestore_path)
            .hint_text("/usr/local/bin/futurerestore").desired_width(f32::INFINITY));
        if ui.small_button("📂").clicked() {
            if let Some(p) = rfd::FileDialog::new().pick_file() {
                state.settings.futurerestore_path = p.to_string_lossy().to_string();
                state.mark_settings_dirty();
            }
        }
    });
    ui.add_space(4.0);
    kv_label(ui, "IPSW file");
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut state.futurerestore_ipsw)
            .hint_text("iPhone14,2_17.4.1_21E236_Restore.ipsw").desired_width(f32::INFINITY));
        if ui.small_button("📂").clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("IPSW", &["ipsw"]).pick_file()
            {
                state.futurerestore_ipsw = p.to_string_lossy().to_string();
            }
        }
    });
    ui.add_space(4.0);
    kv_label(ui, "SHSH2 blob");
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut state.futurerestore_shsh)
            .hint_text("path/to/blob.shsh2").desired_width(f32::INFINITY));
        if ui.small_button("📂").clicked() {
            if let Some(p) = rfd::FileDialog::new()
                .add_filter("SHSH2", &["shsh2","shsh"]).pick_file()
            {
                state.futurerestore_shsh = p.to_string_lossy().to_string();
            }
        }
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.futurerestore_latest_sep, "Latest SEP");
        ui.add_space(8.0);
        ui.checkbox(&mut state.futurerestore_latest_bb,  "Latest baseband");
        ui.add_space(8.0);
        ui.checkbox(&mut state.futurerestore_erase,       "Erase (–e)");
    });
    ui.add_space(10.0);

    let running = state.futurerestore_running;
    if ui.add_enabled(!running, egui::Button::new(
        RichText::new(if running { "⟳ Running…" } else { "▶ Run futurerestore" })
            .size(11.0).strong().color(Color32::BLACK))
        .fill(C::A).corner_radius(6).min_size(egui::vec2(180.0, 32.0))
    ).clicked() {
        state.futurerestore_running = true;
        state.futurerestore_log.clear();
        let _ = op_tx.send(OperationRequest::RunFuturerestore {
            futurerestore_path: state.settings.futurerestore_path.clone(),
            ipsw_path:          state.futurerestore_ipsw.clone(),
            shsh_path:          state.futurerestore_shsh.clone(),
            latest_sep:         state.futurerestore_latest_sep,
            latest_baseband:    state.futurerestore_latest_bb,
            erase:              state.futurerestore_erase,
        });
    }

    if !state.futurerestore_log.is_empty() {
        ui.add_space(10.0);
        kv_label(ui, "Output");
        egui::ScrollArea::vertical()
            .id_salt("fr_log")
            .max_height(200.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut state.futurerestore_log)
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .interactive(false));
            });
    }
}

// ══════════════════════════════════════════════════════════════════════════
// SMALL HELPERS
// ══════════════════════════════════════════════════════════════════════════

fn kv_label(ui: &mut egui::Ui, label: &str) {
    ui.label(RichText::new(label).size(9.5).color(Color32::from_rgb(148, 150, 168)));
}

fn section(ui: &mut egui::Ui, label: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(2.0, 9.0), egui::Sense::hover());
        if ui.is_rect_visible(r) { ui.painter().rect_filled(r, 1.0, C::A); }
        ui.add_space(4.0);
        ui.label(RichText::new(label.to_uppercase()).size(9.0).strong().color(C::T1));
    });
    ui.add_space(8.0);
}

fn bool_row(ui: &mut egui::Ui, label: &str, val: &mut bool) -> bool {
    let old = *val;
    ui.horizontal(|ui| {
        ui.checkbox(val, "");
        ui.label(RichText::new(label).size(10.5).color(C::T1));
    });
    ui.add_space(2.0);
    *val != old
}

fn path_row(ui: &mut egui::Ui, label: &str, val: &mut String, hint: &str) -> bool {
    kv_label(ui, label);
    let mut dirty = false;
    ui.horizontal(|ui| {
        let r = ui.add(egui::TextEdit::singleline(val)
            .hint_text(hint).desired_width(f32::INFINITY));
        if r.changed() { dirty = true; }
        if ui.small_button("📂").clicked() {
            if let Some(p) = rfd::FileDialog::new().pick_file() {
                *val = p.to_string_lossy().to_string();
                dirty = true;
            }
        }
        // Validate
        let exists = !val.is_empty() && std::path::Path::new(val.as_str()).exists();
        ui.label(if val.is_empty() { "—" }
            else if exists { "✓" } else { "✗" });
    });
    ui.add_space(4.0);
    dirty
}
