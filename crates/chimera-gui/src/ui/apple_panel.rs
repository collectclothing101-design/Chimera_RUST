// chimera-gui/src/ui/apple_panel.rs
// Apple iPhone/iPad operations panel for ChimeraRS GUI.
// Tabs: Device Info | Flash (IPSW) | iCloud & Bypass | Passcode | Network Unlock
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui::{self, Color32, RichText, Ui, ScrollArea, ComboBox};
use crate::state::AppState;
use crate::worker::OperationRequest;

/// Render the full Apple device panel
pub fn render_apple_panel(ui: &mut Ui, state: &mut AppState) {
    let device_id = match state.selected_device_id.clone() {
        Some(id) => id,
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(RichText::new("🍎  No Apple device selected").size(16.0).color(Color32::GRAY));
                ui.add_space(8.0);
                ui.label(RichText::new("Connect an iPhone/iPad via USB and wait for detection").color(Color32::DARK_GRAY));
            });
            return;
        }
    };

    // Top-level tab bar for Apple subsections
    ui.horizontal(|ui| {
        for (label, tab) in &[
            ("📋 Info",        AppleTab::Info),
            ("⚡ Flash IPSW",  AppleTab::Flash),
            ("☁️  iCloud",      AppleTab::ICloud),
            ("🔐 Passcode",    AppleTab::Passcode),
            ("📡 Network",     AppleTab::Network),
        ] {
            let selected = state.apple_tab == *tab;
            let text = if selected {
                RichText::new(*label).strong().color(Color32::WHITE)
            } else {
                RichText::new(*label).color(Color32::LIGHT_GRAY)
            };
            if ui.selectable_label(selected, text).clicked() {
                state.apple_tab = tab.clone();
            }
            ui.separator();
        }
    });
    ui.separator();

    match &state.apple_tab {
        AppleTab::Info    => render_apple_info(ui, state, &device_id),
        AppleTab::Flash   => render_apple_flash(ui, state, &device_id),
        AppleTab::ICloud  => render_apple_icloud(ui, state, &device_id),
        AppleTab::Passcode => render_apple_passcode(ui, state, &device_id),
        AppleTab::Network => render_apple_network(ui, state, &device_id),
    }
}

// ─── Device Info Tab ────────────────────────────────────────────────────────

fn render_apple_info(ui: &mut Ui, state: &mut AppState, device_id: &str) {
    ui.heading("📱 Apple Device Information");
    ui.add_space(8.0);

    if ui.button("🔄 Read Device Info").clicked() {
        state.send_operation(OperationRequest::AppleGetInfo {
            device_id: device_id.to_owned(),
        });
    }
    ui.add_space(8.0);
    ui.separator();

    if let Some(info) = state.apple_device_info.get(device_id) {
        egui::Grid::new("apple_info_grid")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                info_row(ui, "Model",       &info.model_name);
                info_row(ui, "Identifier",  &info.model_identifier);
                info_row(ui, "Chipset",     &format!("{:?}", info.chipset));
                info_row(ui, "iOS Version", info.ios_version.as_deref().unwrap_or("—"));
                info_row(ui, "Build",       info.build_version.as_deref().unwrap_or("—"));
                info_row(ui, "Serial",      &info.serial_number);
                info_row(ui, "UDID",        &info.udid);
                info_row(ui, "IMEI",        info.imei.as_deref().unwrap_or("—"));
                info_row(ui, "IMEI 2",      info.imei2.as_deref().unwrap_or("—"));
                info_row(ui, "ICCID",       info.iccid.as_deref().unwrap_or("—"));
                info_row(ui, "Phone #",     info.phone_number.as_deref().unwrap_or("—"));
                info_row(ui, "Wi-Fi MAC",   info.wifi_address.as_deref().unwrap_or("—"));
                info_row(ui, "Bluetooth",   info.bluetooth_address.as_deref().unwrap_or("—"));
                info_row(ui, "Carrier",     info.carrier.as_deref().unwrap_or("—"));
                info_row(ui, "Region",      info.region.as_deref().unwrap_or("—"));

                // Activation lock status
                ui.label(RichText::new("Activation Lock").strong());
                let (label, color) = if info.is_activation_locked {
                    ("🔒 LOCKED", Color32::RED)
                } else {
                    ("🔓 Unlocked", Color32::GREEN)
                };
                ui.label(RichText::new(label).color(color));
                ui.end_row();

                // Passcode
                ui.label(RichText::new("Passcode").strong());
                let (p_label, p_color) = if info.is_passcode_set {
                    ("🔑 Set", Color32::YELLOW)
                } else {
                    ("✅ Not Set", Color32::GREEN)
                };
                ui.label(RichText::new(p_label).color(p_color));
                ui.end_row();

                // Connection mode
                ui.label(RichText::new("Connection Mode").strong());
                ui.label(&format!("{:?}", info.connection_mode));
                ui.end_row();
            });

        // Checkm8 status badge
        ui.add_space(8.0);
        ui.separator();
        let is_checkm8 = info.chipset.is_checkm8_vulnerable();
        let (cm8_label, cm8_color, cm8_desc) = if is_checkm8 {
            ("⚡ checkm8 VULNERABLE", Color32::from_rgb(255, 160, 0),
             "This device's bootrom contains the unpatchable checkm8 exploit.\nBypass/flash operations are available without official credentials.")
        } else {
            ("🛡️  checkm8 Not Applicable", Color32::GRAY,
             "A12+ chips are not vulnerable to checkm8. Only official or MDM-based operations available.")
        };
        ui.label(RichText::new(cm8_label).strong().color(cm8_color));
        ui.label(RichText::new(cm8_desc).small().color(Color32::GRAY));
    } else {
        ui.label(RichText::new("Device info not yet loaded. Click 'Read Device Info'.").color(Color32::GRAY));
    }
}

// ─── Flash IPSW Tab ─────────────────────────────────────────────────────────

fn render_apple_flash(ui: &mut Ui, state: &mut AppState, device_id: &str) {
    ui.heading("⚡ Flash IPSW Firmware");
    ui.add_space(4.0);
    ui.label(RichText::new(
        "Flash a signed Apple IPSW firmware file to the device.\n\
         Update mode preserves user data. Restore mode wipes everything."
    ).color(Color32::GRAY).small());
    ui.add_space(8.0);

    // IPSW file path
    ui.horizontal(|ui| {
        ui.label("IPSW File:");
        ui.text_edit_singleline(&mut state.apple_ipsw_path);
        if ui.button("📂 Browse…").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("IPSW Firmware", &["ipsw"])
                .pick_file()
            {
                state.apple_ipsw_path = path.to_string_lossy().to_string();
            }
        }
    });

    ui.add_space(8.0);

    // Mode selection
    ui.horizontal(|ui| {
        ui.label("Restore Mode:");
        ui.radio_value(&mut state.apple_erase_mode, false, "🔄 Update (keep data)");
        ui.radio_value(&mut state.apple_erase_mode, true,  "🗑️  Restore (erase all)");
    });

    ui.add_space(4.0);

    // TSS verification toggle
    ui.checkbox(&mut state.apple_verify_tss, "✅ Verify SHSH blobs with Apple TSS");
    ui.checkbox(&mut state.apple_skip_baseband, "⚠️  Skip baseband update");

    ui.add_space(8.0);
    ui.separator();

    // Download latest IPSW section
    ui.label(RichText::new("Or download the latest signed firmware:").strong());
    ui.horizontal(|ui| {
        ui.label("Download to:");
        ui.text_edit_singleline(&mut state.apple_download_dir);
        if ui.button("📂").clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                state.apple_download_dir = path.to_string_lossy().to_string();
            }
        }
        if ui.button("⬇️  Download Latest IPSW").clicked() {
            state.send_operation(OperationRequest::AppleDownloadIpsw {
                device_id: device_id.to_owned(),
                dest_dir: state.apple_download_dir.clone(),
                model: state.ipsw_model_selected.clone(),
            });
        }
    });

    ui.add_space(8.0);

    // Validate IPSW button
    if ui.button("🔍 Validate IPSW").clicked() && !state.apple_ipsw_path.is_empty() {
        state.send_operation(OperationRequest::AppleValidateIpsw {
            ipsw_path: state.apple_ipsw_path.clone(),
        });
    }

    ui.add_space(8.0);

    // Flash button with warning
    let flash_label = if state.apple_erase_mode {
        RichText::new("⚡ RESTORE (ERASE ALL DATA)").color(Color32::RED).strong()
    } else {
        RichText::new("⚡ Flash IPSW Update").color(Color32::GREEN).strong()
    };

    if ui.add_sized([220.0, 36.0], egui::Button::new(flash_label)).clicked() {
        if state.apple_ipsw_path.is_empty() {
            state.add_log(crate::state::LogEntry::error("Please select an IPSW file first."));
        } else {
            state.pending_confirm = Some(crate::state::ConfirmDialog {
                title: if state.apple_erase_mode { "Confirm Full Erase & Restore" } else { "Confirm IPSW Flash" }.into(),
                message: if state.apple_erase_mode {
                    "⚠️  ALL USER DATA WILL BE PERMANENTLY ERASED!\nThis cannot be undone. Are you sure?".into()
                } else {
                    "Flash IPSW update to device? User data will be preserved.".into()
                },
                on_confirm: OperationRequest::AppleFlashIpsw {
                    device_id: device_id.to_owned(),
                    ipsw_path: state.apple_ipsw_path.clone(),
                    erase: state.apple_erase_mode,
                },
            });
        }
    }

    // Recovery mode controls
    ui.add_space(12.0);
    ui.separator();
    ui.label(RichText::new("Recovery Mode Controls:").strong());
    ui.horizontal(|ui| {
        if ui.button("📲 Enter Recovery Mode").clicked() {
            state.send_operation(OperationRequest::AppleEnterRecovery {
                device_id: device_id.to_owned(),
            });
        }
        if ui.button("🔙 Exit Recovery Mode").clicked() {
            state.send_operation(OperationRequest::AppleExitRecovery {
                device_id: device_id.to_owned(),
            });
        }
        if ui.button("🔄 Reboot Device").clicked() {
            state.send_operation(OperationRequest::AppleReboot {
                device_id: device_id.to_owned(),
            });
        }
    });
}

// ─── iCloud & Bypass Tab ────────────────────────────────────────────────────

fn render_apple_icloud(ui: &mut Ui, state: &mut AppState, device_id: &str) {
    ui.heading("☁️  iCloud & Activation Lock");
    ui.add_space(4.0);

    // Legal warning banner
    egui::Frame::NONE
        .fill(Color32::from_rgb(60, 30, 0))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(
                "⚠️  LEGAL NOTICE: Only use these features on devices you own or have \
                 written authorisation for. Bypassing activation lock on a stolen device \
                 is a criminal offence in Australia (Criminal Code Act 1995 §477-478)."
            ).color(Color32::from_rgb(255, 200, 0)).small());
        });
    ui.add_space(8.0);

    // Check iCloud status
    if ui.button("🔍 Check Activation Lock Status").clicked() {
        state.send_operation(OperationRequest::AppleCheckIcloud {
            device_id: device_id.to_owned(),
        });
    }

    if let Some(activation_info) = state.apple_activation_info.get(device_id) {
        ui.add_space(6.0);
        let (status_text, status_color) = match &activation_info.status {
            crate::state::AppleActivationStatus::Activated =>
                ("✅ Activated – iCloud account linked", Color32::GREEN),
            crate::state::AppleActivationStatus::ActivationRequired =>
                ("🔒 ACTIVATION LOCK – Requires iCloud credentials", Color32::RED),
            crate::state::AppleActivationStatus::Unactivated =>
                ("📱 Unactivated – No lock (factory reset)", Color32::YELLOW),
            crate::state::AppleActivationStatus::Unknown =>
                ("❓ Unknown status", Color32::GRAY),
        };
        ui.label(RichText::new(status_text).strong().color(status_color));
        if let Some(hint) = &activation_info.account_hint {
            ui.label(format!("iCloud Account: {}", hint));
        }
        if activation_info.is_supervised {
            if let Some(org) = &activation_info.mdm_org {
                ui.label(format!("👔 Supervised by: {}", org));
            }
        }
    }

    ui.add_space(8.0);
    ui.separator();

    // Bypass section
    ui.label(RichText::new("Activation Lock Bypass:").strong());
    ui.add_space(4.0);

    // Bypass method selector
    ui.horizontal(|ui| {
        ui.label("Method:");
        ComboBox::from_id_salt("bypass_method")
            .selected_text(state.apple_bypass_method.label())
            .show_ui(ui, |ui| {
                for method in &[
                    crate::state::AppleBypassMethodUI::Checkm8,
                    crate::state::AppleBypassMethodUI::Palera1n,
                    crate::state::AppleBypassMethodUI::EraseRestore,
                    crate::state::AppleBypassMethodUI::DnsServer,
                    crate::state::AppleBypassMethodUI::MdmDep,
                ] {
                    ui.selectable_value(
                        &mut state.apple_bypass_method,
                        method.clone(),
                        method.label(),
                    );
                }
            });
    });

    // Method description
    ui.label(RichText::new(state.apple_bypass_method.description()).small().color(Color32::GRAY));
    ui.add_space(4.0);

    if state.apple_bypass_method == crate::state::AppleBypassMethodUI::DnsServer {
        ui.horizontal(|ui| {
            ui.label("DNS Server IP:");
            ui.text_edit_singleline(&mut state.apple_dns_server);
        });
    }

    ui.add_space(8.0);
    if ui.add_sized([200.0, 32.0], egui::Button::new(
        RichText::new("🔓 Execute Bypass").color(Color32::YELLOW).strong()
    )).clicked() {
        state.pending_confirm = Some(crate::state::ConfirmDialog {
            title: "Confirm iCloud Bypass".into(),
            message: format!(
                "Execute {} on this device?\n\nEnsure you are authorised to do this.",
                state.apple_bypass_method.label()
            ),
            on_confirm: OperationRequest::AppleBypassIcloud {
                device_id: device_id.to_owned(),
                method: state.apple_bypass_method.clone(),
            },
        });
    }

    ui.add_space(12.0);
    ui.separator();

    // iCloud wipe section
    ui.label(RichText::new("iCloud Wipe (Sign out + Erase):").strong());
    ui.label(RichText::new(
        "Sign the device out of iCloud and perform a full factory reset.\n\
         Requires device to be unlocked or bypass to be active."
    ).small().color(Color32::GRAY));
    ui.add_space(4.0);

    if ui.add_sized([200.0, 32.0], egui::Button::new(
        RichText::new("☁️  Wipe & Sign Out iCloud").color(Color32::RED)
    )).clicked() {
        state.pending_confirm = Some(crate::state::ConfirmDialog {
            title: "Confirm iCloud Wipe".into(),
            message: "⚠️  This will ERASE ALL DATA and sign out of iCloud!\nThis cannot be undone. Proceed?".into(),
            on_confirm: OperationRequest::AppleIcloudWipe {
                device_id: device_id.to_owned(),
                ipsw_path: if state.apple_ipsw_path.is_empty() {
                    None
                } else {
                    Some(state.apple_ipsw_path.clone())
                },
            },
        });
    }
}

// ─── Passcode Tab ───────────────────────────────────────────────────────────

fn render_apple_passcode(ui: &mut Ui, state: &mut AppState, device_id: &str) {
    ui.heading("🔐 Passcode & Screen Lock");
    ui.add_space(4.0);

    egui::Frame::NONE
        .fill(Color32::from_rgb(30, 30, 60))
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.label(RichText::new(
                "ℹ️  Removing a passcode without the owner's consent is prohibited.\n\
                 Authorised uses: forgotten PIN recovery, enterprise device management."
            ).color(Color32::LIGHT_BLUE).small());
        });
    ui.add_space(8.0);

    // checkm8 bypass passcode removal (A5–A11)
    ui.label(RichText::new("Method 1 – checkm8 Bypass (A5–A11, DATA PRESERVED)").strong().color(Color32::GREEN));
    ui.label(RichText::new(
        "Uses the bootrom exploit to load a custom ramdisk that patches the keybag.\n\
         Works on iPhone 4S through iPhone X (A5–A11 chips only).\n\
         Device data is typically preserved."
    ).small().color(Color32::GRAY));
    ui.add_space(4.0);
    if ui.button("⚡ Bypass Passcode via checkm8").clicked() {
        state.pending_confirm = Some(crate::state::ConfirmDialog {
            title: "Confirm Passcode Bypass".into(),
            message: "Execute checkm8 passcode bypass?\nRequires device in DFU mode.".into(),
            on_confirm: OperationRequest::AppleRemovePasscode {
                device_id: device_id.to_owned(),
                use_checkm8: true,
                ipsw_path: None,
                chipset: state.apple_bypass_technique.clone(),
            },
        });
    }

    ui.add_space(12.0);
    ui.separator();

    // Erase restore passcode removal
    ui.label(RichText::new("Method 2 – Erase & Restore (ALL DEVICES, DATA LOST)").strong().color(Color32::YELLOW));
    ui.label(RichText::new(
        "Performs a full device erase via DFU mode restore.\n\
         Works on all iPhone/iPad models but ALL user data is permanently lost."
    ).small().color(Color32::GRAY));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label("IPSW (optional):");
        ui.text_edit_singleline(&mut state.apple_ipsw_path);
        if ui.button("📂").clicked() {
            if let Some(p) = rfd::FileDialog::new().add_filter("IPSW", &["ipsw"]).pick_file() {
                state.apple_ipsw_path = p.to_string_lossy().to_string();
            }
        }
    });
    ui.label(RichText::new("Leave blank to enter recovery mode for manual Finder/iTunes restore.").small().color(Color32::GRAY));
    ui.add_space(4.0);

    if ui.button("🗑️  Erase Device (Remove Passcode + All Data)").clicked() {
        state.pending_confirm = Some(crate::state::ConfirmDialog {
            title: "Confirm Device Erase".into(),
            message: "⚠️  ALL DATA WILL BE PERMANENTLY ERASED!\nThis removes the passcode but cannot be undone.".into(),
            on_confirm: OperationRequest::AppleRemovePasscode {
                device_id: device_id.to_owned(),
                use_checkm8: false,
                ipsw_path: if state.apple_ipsw_path.is_empty() {
                    None
                } else {
                    Some(state.apple_ipsw_path.clone())
                },
                chipset: state.apple_bypass_technique.clone(),
            },
        });
    }

    ui.add_space(12.0);
    ui.separator();

    // Recovery mode shortcut
    ui.label(RichText::new("Method 3 – Recovery Mode (Manual)").strong().color(Color32::LIGHT_GRAY));
    ui.label(RichText::new("Enter recovery mode so you can restore via Finder/iTunes on your computer.").small().color(Color32::GRAY));
    if ui.button("📲 Enter Recovery Mode for Manual Restore").clicked() {
        state.send_operation(OperationRequest::AppleEnterRecovery {
            device_id: device_id.to_owned(),
        });
    }
}

// ─── Network Unlock Tab ─────────────────────────────────────────────────────

fn render_apple_network(ui: &mut Ui, state: &mut AppState, device_id: &str) {
    ui.heading("📡 Network / Carrier Unlock");
    ui.add_space(4.0);
    ui.label(RichText::new(
        "Request carrier unlocking for iPhone locked to an Australian carrier.\n\
         Official carrier unlock is the only legitimate method for A12+ devices."
    ).color(Color32::GRAY).small());
    ui.add_space(8.0);

    // Check current lock status
    if ui.button("🔍 Check Carrier Lock Status").clicked() {
        state.send_operation(OperationRequest::AppleCheckNetworkLock {
            device_id: device_id.to_owned(),
        });
    }

    if let Some(locked) = state.apple_network_locked.get(device_id) {
        let (txt, clr) = if *locked {
            ("🔒 SIM LOCKED to a specific carrier", Color32::RED)
        } else {
            ("🔓 Carrier Unlocked (any SIM)", Color32::GREEN)
        };
        ui.add_space(4.0);
        ui.label(RichText::new(txt).strong().color(clr));
    }

    ui.add_space(8.0);
    ui.separator();

    // Australian carrier unlock portal
    ui.label(RichText::new("🇦🇺 Australian Carrier Unlock Request:").strong());

    ui.horizontal(|ui| {
        ui.label("Carrier:");
        ComboBox::from_id_salt("au_carrier_select_iphone")
            .selected_text(&state.apple_au_carrier)
            .show_ui(ui, |ui| {
                for carrier in &[
                    "Telstra", "Optus", "Vodafone Australia", "TPG Mobile",
                    "Boost Mobile Australia", "Woolworths Mobile", "amaysim",
                    "Belong", "Circles.Life Australia", "Aldi Mobile",
                    "Dodo Mobile", "Kogan Mobile", "Southern Phone",
                ] {
                    ui.selectable_value(&mut state.apple_au_carrier, carrier.to_string(), *carrier);
                }
            });
    });

    ui.horizontal(|ui| {
        ui.label("Account Number (optional):");
        ui.text_edit_singleline(&mut state.apple_carrier_account);
    });

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button("📋 Generate Unlock Instructions").clicked() {
            state.send_operation(OperationRequest::AppleGetUnlockInstructions {
                device_id: device_id.to_owned(),
                carrier: state.apple_au_carrier.clone(),
            });
        }
        if ui.button("🌐 Submit Unlock Request").clicked() {
            state.send_operation(OperationRequest::AppleSubmitCarrierUnlock {
                device_id: device_id.to_owned(),
                carrier: state.apple_au_carrier.clone(),
                account_number: if state.apple_carrier_account.is_empty() {
                    None
                } else {
                    Some(state.apple_carrier_account.clone())
                },
            });
        }
    });

    // Show instructions if available
    if let Some(instructions) = state.apple_unlock_instructions.get(device_id) {
        ui.add_space(8.0);
        ui.separator();
        ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
            ui.label(RichText::new(instructions).monospace().size(11.0));
        });
        if ui.button("📋 Copy to Clipboard").clicked() {
            ui.output_mut(|o| o.commands.push(egui::output::OutputCommand::CopyText(instructions.clone())));
        }
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("ℹ️  After carrier approval:").strong());
    ui.label(RichText::new(
        "1. Insert a SIM from a different carrier\n\
         2. Power on the device\n\
         3. Connect to iTunes/Finder to trigger re-activation\n\
         4. Device will activate without the old carrier lock"
    ).small().color(Color32::GRAY));
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn info_row(ui: &mut Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).strong());
    ui.label(value);
    ui.end_row();
}

/// Apple-specific sub-tabs
#[derive(Debug, Clone, PartialEq)]
pub enum AppleTab {
    Info,
    Flash,
    ICloud,
    Passcode,
    Network,
}

impl Default for AppleTab {
    fn default() -> Self { AppleTab::Info }
}
