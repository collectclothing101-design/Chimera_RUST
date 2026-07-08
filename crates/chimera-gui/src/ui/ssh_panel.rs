#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
use crossbeam_channel::Sender;
// chimera-gui/src/ui/ssh_panel.rs
// SSH · VPN panel — matches pg-ssh from chimera-gui.html
// Tabs: SSH Session | Port Forward | VPN

use eframe::egui::{self, RichText, Color32, ScrollArea};
use crate::state::AppState;
use crate::worker::OperationRequest;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum SshTab { #[default] Session, PortForward, Vpn }

pub fn render_ssh_panel(
    ui: &mut egui::Ui,
    state: &mut AppState,
    op_tx: &Sender<OperationRequest>,
) {
    // Page header
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("03 · DEVICE").size(8.0).color(Color32::from_rgb(50, 58, 70)));
            ui.label(RichText::new("SSH · VPN").size(18.0).strong());
            ui.label(RichText::new("Remote device access · port forwarding · VPN tunnel configuration")
                .size(10.0).color(Color32::from_rgb(89, 100, 114)));
        });
    });
    ui.separator();
    ui.add_space(10.0);

    // Tab bar
    ui.horizontal(|ui| {
        if ui.selectable_label(state.ssh_tab == SshTab::Session, "SSH Session").clicked() {
            state.ssh_tab = SshTab::Session;
        }
        if ui.selectable_label(state.ssh_tab == SshTab::PortForward, "Port Forward").clicked() {
            state.ssh_tab = SshTab::PortForward;
        }
        if ui.selectable_label(state.ssh_tab == SshTab::Vpn, "VPN").clicked() {
            state.ssh_tab = SshTab::Vpn;
        }
    });
    ui.separator();
    ui.add_space(12.0);

    match state.ssh_tab {
        SshTab::Session    => render_ssh_session(ui, state, op_tx),
        SshTab::PortForward => render_port_forward(ui, state),
        SshTab::Vpn        => render_vpn(ui, state),
    }
}

fn render_ssh_session(
    ui: &mut egui::Ui,
    state: &mut AppState,
    op_tx: &Sender<OperationRequest>,
) {
    ui.set_max_width(450.0);

    section_header(ui, "SSH Connection");

    ui.vertical(|ui| {
        field_row(ui, "Host or IP Address", |ui| {
            ui.text_edit_singleline(&mut state.ssh_host)
                .on_hover_text("192.168.0.1 or hostname");
        });

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Port").size(9.5).color(Color32::from_rgb(148, 150, 168)));
                ui.add_sized([68.0, 24.0], egui::TextEdit::singleline(&mut state.ssh_port));
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("Username").size(9.5).color(Color32::from_rgb(148, 150, 168)));
                ui.add_sized([ui.available_width(), 24.0],
                    egui::TextEdit::singleline(&mut state.ssh_username)
                        .hint_text("root"));
            });
        });

        field_row(ui, "Authentication Method", |ui| {
            egui::ComboBox::from_id_salt("ssh_auth")
                .selected_text(&state.ssh_auth_method)
                .width(ui.available_width() - 4.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.ssh_auth_method,
                        "Password".to_string(), "Password");
                    ui.selectable_value(&mut state.ssh_auth_method,
                        "Private Key File".to_string(), "Private Key File");
                    ui.selectable_value(&mut state.ssh_auth_method,
                        "Public Key + Passphrase".to_string(), "Public Key + Passphrase");
                });
        });

        if state.ssh_auth_method == "Password" {
            field_row(ui, "Password", |ui| {
                ui.add(egui::TextEdit::singleline(&mut state.ssh_password)
                    .password(true)
                    .desired_width(ui.available_width()));
            });
        } else {
            field_row(ui, "Private Key Path", |ui| {
                ui.horizontal(|ui| {
                    ui.add_sized([ui.available_width() - 70.0, 24.0],
                        egui::TextEdit::singleline(&mut state.ssh_key_path)
                            .hint_text("~/.ssh/id_rsa"));
                    if ui.button("Browse").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("SSH Key", &["pem", "key", ""])
                            .pick_file() {
                            state.ssh_key_path = path.display().to_string();
                        }
                    }
                });
            });
            if state.ssh_auth_method.contains("Passphrase") {
                field_row(ui, "Passphrase", |ui| {
                    ui.add(egui::TextEdit::singleline(&mut state.ssh_passphrase)
                        .password(true)
                        .desired_width(ui.available_width()));
                });
            }
        }

        ui.add_space(8.0);
        let connected = state.ssh_connected;
        if connected {
            ui.horizontal(|ui| {
                ui.label(RichText::new("● Connected").color(Color32::from_rgb(26, 184, 106)).size(10.5));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Disconnect").clicked() {
                        let _ = op_tx.send(OperationRequest::SshDisconnect);
                        state.ssh_connected = false;
                        state.add_log(crate::state::LogEntry::info("SSH disconnect requested.".to_string()));
                    }
                });
            });
        } else {
            if ui.add_sized([ui.available_width(), 32.0],
                egui::Button::new(RichText::new("Establish Connection").strong())).clicked()
            {
                if state.ssh_host.trim().is_empty() {
                    state.add_log(crate::state::LogEntry::warn("SSH: host is required.".to_string()));
                } else if state.ssh_username.trim().is_empty() {
                    state.add_log(crate::state::LogEntry::warn("SSH: username is required.".to_string()));
                } else {
                    state.add_log(crate::state::LogEntry::info(format!(
                        "SSH connecting to {}@{}:{} …",
                        state.ssh_username, state.ssh_host, state.ssh_port
                    )));
                    // Dispatch the real SshConnect operation — worker.rs uses ssh2 to
                    // establish a live session. The worker emits LocalEvent::SshConnected
                    // / SshDisconnected which AppState consumes to flip ssh_connected.
                    let auth_method = state.ssh_auth_method.clone();
                    let _ = op_tx.send(OperationRequest::SshConnect {
                        host:        state.ssh_host.trim().to_string(),
                        port:        state.ssh_port.trim().parse::<u16>().unwrap_or(22),
                        username:    state.ssh_username.trim().to_string(),
                        auth_method: auth_method.clone(),
                        password:    if auth_method.contains("Password")   { state.ssh_password.clone()   } else { String::new() },
                        key_path:    if auth_method.contains("Key")        { state.ssh_key_path.clone()   } else { String::new() },
                        passphrase:  if auth_method.contains("Passphrase") { state.ssh_passphrase.clone() } else { String::new() },
                    });
                }
            }
        }

        // Output terminal
        if connected {
            ui.add_space(10.0);
            section_header(ui, "Terminal Output");
            ScrollArea::vertical().max_height(180.0).id_salt("ssh_term").show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut state.ssh_terminal_output)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace));
            });
            ui.horizontal(|ui| {
                let send = ui.text_edit_singleline(&mut state.ssh_command_input)
                    .on_hover_text("Enter command and press Enter");
                if send.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let cmd = state.ssh_command_input.clone();
                    state.ssh_terminal_output.push_str(&format!("\n$ {}\n", cmd));
                    state.ssh_command_input.clear();
                    state.add_log(crate::state::LogEntry::info(format!("SSH cmd: {}", cmd)));
                }
                if ui.button("Send").clicked() {
                    let cmd = state.ssh_command_input.clone();
                    state.ssh_terminal_output.push_str(&format!("\n$ {}\n", cmd));
                    state.ssh_command_input.clear();
                }
            });
        }
    });
}

fn render_port_forward(ui: &mut egui::Ui, state: &mut AppState) {
    if state.ssh_tunnels.is_empty() {
        // Empty state
        ui.vertical_centered(|ui| {
            ui.add_space(44.0);
            ui.label(RichText::new("⇄").size(24.0).color(Color32::from_rgb(89, 100, 114)));
            ui.add_space(9.0);
            ui.label(RichText::new("No Active Tunnels").size(11.5)
                .color(Color32::from_rgb(89, 100, 114)));
            ui.add_space(4.0);
            ui.label(RichText::new(
                "Configure SSH port-forwarding rules to expose device services\non your local network.")
                .size(9.5).color(Color32::from_rgb(50, 58, 70)));
        });
        ui.add_space(20.0);
    } else {
        section_header(ui, "Active Tunnels");
        let _to_remove: Vec<usize> = Vec::new();
        for (_i, tunnel) in state.ssh_tunnels.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.monospace(format!("localhost:{} → {}:{}", tunnel.local_port, tunnel.remote_host, tunnel.remote_port));
                if ui.small_button("Remove").clicked() {
                    // handled below
                }
            });
        }
    }

    ui.separator();
    section_header(ui, "Add Tunnel");
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("Local Port").size(9.5).color(Color32::from_rgb(148, 150, 168)));
            ui.add_sized([80.0, 24.0], egui::TextEdit::singleline(&mut state.ssh_new_local_port));
        });
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.label(RichText::new("Remote Host").size(9.5).color(Color32::from_rgb(148, 150, 168)));
            ui.add_sized([160.0, 24.0],
                egui::TextEdit::singleline(&mut state.ssh_new_remote_host).hint_text("127.0.0.1"));
        });
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.label(RichText::new("Remote Port").size(9.5).color(Color32::from_rgb(148, 150, 168)));
            ui.add_sized([80.0, 24.0], egui::TextEdit::singleline(&mut state.ssh_new_remote_port));
        });
    });
    ui.add_space(6.0);
    if ui.button("Add Tunnel").clicked() {
        let local: u16 = state.ssh_new_local_port.trim().parse().unwrap_or(0);
        let remote: u16 = state.ssh_new_remote_port.trim().parse().unwrap_or(0);
        if local == 0 || remote == 0 || state.ssh_new_remote_host.trim().is_empty() {
            state.add_log(crate::state::LogEntry::warn("Port forward: invalid port or host.".to_string()));
        } else {
            state.ssh_tunnels.push(SshTunnel {
                local_port: local,
                remote_host: state.ssh_new_remote_host.clone(),
                remote_port: remote,
            });
            state.ssh_new_local_port.clear();
            state.ssh_new_remote_host.clear();
            state.ssh_new_remote_port.clear();
            state.add_log(crate::state::LogEntry::info(format!(
                "SSH tunnel added: localhost:{} → {}:{}", local,
                state.ssh_tunnels.last().unwrap().remote_host, remote
            )));
        }
    }
}

fn render_vpn(ui: &mut egui::Ui, _state: &mut AppState) {
    ui.vertical_centered(|ui| {
        ui.add_space(44.0);
        ui.label(RichText::new("◈").size(24.0).color(Color32::from_rgb(89, 100, 114)));
        ui.add_space(9.0);
        ui.label(RichText::new("VPN Not Configured").size(11.5)
            .color(Color32::from_rgb(89, 100, 114)));
        ui.add_space(4.0);
        ui.label(RichText::new(
            "Set up a VPN tunnel for secure remote device access across networks.")
            .size(9.5).color(Color32::from_rgb(50, 58, 70)));
    });
}

// ── State additions (add these fields to AppState) ──────────────────────────
// pub ssh_tab:                SshTab,
// pub ssh_host:               String,
// pub ssh_port:               String,       // default "22"
// pub ssh_username:           String,       // default "root"
// pub ssh_auth_method:        String,       // default "Password"
// pub ssh_password:           String,
// pub ssh_key_path:           String,
// pub ssh_passphrase:         String,
// pub ssh_connected:          bool,
// pub ssh_terminal_output:    String,
// pub ssh_command_input:      String,
// pub ssh_tunnels:            Vec<SshTunnel>,
// pub ssh_new_local_port:     String,
// pub ssh_new_remote_host:    String,
// pub ssh_new_remote_port:    String,

#[derive(Debug, Clone)]
pub struct SshTunnel {
    pub local_port:  u16,
    pub remote_host: String,
    pub remote_port: u16,
}

// ── Helpers ────────────────────────────────────────────────────────────────
fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(9.0).strong()
            .color(Color32::from_rgb(89, 100, 114)));
        let r = ui.available_rect_before_wrap();
        ui.painter().line_segment(
            [egui::pos2(r.left(), r.center().y), egui::pos2(r.right() - 4.0, r.center().y)],
            egui::Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255,255,255,10)),
        );
    });
    ui.add_space(6.0);
}

fn field_row(ui: &mut egui::Ui, label: &str, content: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).size(9.5).color(Color32::from_rgb(148, 150, 168)));
        content(ui);
        ui.add_space(8.0);
    });
}
