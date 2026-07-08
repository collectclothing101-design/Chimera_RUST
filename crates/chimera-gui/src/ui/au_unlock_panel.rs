// chimera-gui/src/ui/au_unlock_panel.rs
// Australian Network Unlock panel for Android devices.
// Covers: carrier database lookup, NCK calculation, step-by-step instructions,
// and ADB-based NCK application.
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui::{self, Color32, RichText, Ui, ScrollArea, ComboBox};
use crate::state::AppState;
use crate::worker::OperationRequest;

/// Render the Australian Network Unlock panel
pub fn render_au_unlock_panel(ui: &mut Ui, state: &mut AppState) {
    ui.heading("🇦🇺  Australian Network Unlock");
    ui.add_space(4.0);
    ui.label(RichText::new(
        "Carrier unlock Android devices locked to Australian networks (Telstra, Optus, Vodafone AU, etc.).\n\
         Supports NCK code calculation for Samsung, LG, and Motorola devices, plus portal submission guidance."
    ).small().color(Color32::GRAY));
    ui.add_space(8.0);
    ui.separator();

    // ── Device IMEI Input ──────────────────────────────────────────────────
    ui.label(RichText::new("Device Identification").strong());
    ui.add_space(4.0);

    egui::Grid::new("au_unlock_inputs")
        .num_columns(2)
        .spacing([10.0, 6.0])
        .show(ui, |ui| {
            ui.label("IMEI:");
            ui.text_edit_singleline(&mut state.au_unlock_imei);
            ui.end_row();

            ui.label("Device Brand:");
            ComboBox::from_id_salt("au_brand_select")
                .selected_text(&state.au_unlock_brand)
                .show_ui(ui, |ui| {
                    for brand in &[
                        "Samsung", "LG", "Motorola", "Huawei", "Xiaomi",
                        "OnePlus", "Oppo", "Nokia", "Sony", "HTC",
                        "iPhone (use Apple tab)", "Other"
                    ] {
                        ui.selectable_value(&mut state.au_unlock_brand, brand.to_string(), *brand);
                    }
                });
            ui.end_row();

            ui.label("Current Carrier:");
            ComboBox::from_id_salt("au_carrier_select_android")
                .selected_text(&state.au_unlock_carrier)
                .show_ui(ui, |ui| {
                    for carrier in &[
                        "Telstra", "Optus", "Vodafone Australia", "TPG Mobile",
                        "Boost Mobile Australia", "Woolworths Mobile", "amaysim",
                        "Belong", "Circles.Life Australia", "Aldi Mobile",
                        "Dodo Mobile", "Kogan Mobile", "Southern Phone",
                        "Unknown / Auto-Detect"
                    ] {
                        ui.selectable_value(&mut state.au_unlock_carrier, carrier.to_string(), *carrier);
                    }
                });
            ui.end_row();

            ui.label("MCC+MNC (auto if blank):");
            ui.text_edit_singleline(&mut state.au_unlock_mccmnc);
            ui.end_row();
        });

    ui.add_space(8.0);

    // Auto-fill from connected device
    if let Some(dev_id) = &state.selected_device_id {
        if ui.button("📱 Read from Connected Device").clicked() {
            state.send_operation(OperationRequest::AuReadDeviceImeiCarrier {
                device_id: dev_id.clone(),
            });
        }
    }

    ui.add_space(8.0);
    ui.separator();

    // ── NCK Code Calculation ───────────────────────────────────────────────
    ui.label(RichText::new("Network Unlock Code (NCK) Calculator").strong());
    ui.label(RichText::new(
        "Calculates the unlock code using the device IMEI and carrier MCC/MNC.\n\
         NOTE: Algorithmic codes work for most Samsung/LG/Motorola devices.\n\
         If the calculated code doesn't work, use the official carrier portal below."
    ).small().color(Color32::GRAY));
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        if ui.add_sized([160.0, 28.0], egui::Button::new("🔢 Calculate NCK")).clicked() {
            if state.au_unlock_imei.len() == 15 {
                state.send_operation(OperationRequest::AuCalculateNck {
                    imei: state.au_unlock_imei.clone(),
                    brand: state.au_unlock_brand.clone(),
                    carrier: state.au_unlock_carrier.clone(),
                    mccmnc: if state.au_unlock_mccmnc.is_empty() {
                        None
                    } else {
                        Some(state.au_unlock_mccmnc.clone())
                    },
                });
            } else {
                state.add_log(crate::state::LogEntry::error("Please enter a valid 15-digit IMEI first."));
            }
        }

        if let Some(nck) = &state.au_unlock_nck_result {
            ui.add_space(8.0);
            ui.label(RichText::new("NCK:").strong());
            let code_text = RichText::new(nck).monospace().size(18.0).color(Color32::from_rgb(100, 220, 100)).strong();
            ui.label(code_text);
            if ui.button("📋 Copy").clicked() {
                ui.output_mut(|o| o.commands.push(egui::output::OutputCommand::CopyText(nck.clone())));
            }
        }
    });

    ui.add_space(4.0);
    if let Some(_nck) = &state.au_unlock_nck_result {
        egui::Frame::NONE
            .fill(Color32::from_rgb(0, 50, 0))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.label(RichText::new("How to enter the NCK:").strong().color(Color32::GREEN));
                ui.label(RichText::new(
                    "1. Power off the device\n\
                     2. Insert a SIM card from a DIFFERENT carrier (non-Telstra/Optus/etc.)\n\
                     3. Power on the device\n\
                     4. When prompted 'SIM Network Unlock PIN', enter the NCK code above\n\
                     5. Tap 'Unlock' — device will show 'Network Unlock Successful'\n\
                     6. If PIN is rejected, use the official carrier portal to get the correct code"
                ).small().color(Color32::LIGHT_GRAY));
            });
    }

    ui.add_space(8.0);

    // Apply NCK via ADB (for supported devices)
    if let Some(dev_id) = &state.selected_device_id {
        if let Some(nck) = &state.au_unlock_nck_result {
            let nck_clone = nck.clone();
            let dev_clone = dev_id.clone();
            ui.horizontal(|ui| {
                if ui.button("📲 Apply NCK via ADB").clicked() {
                    state.send_operation(OperationRequest::AuApplyNckAdb {
                        device_id: dev_clone,
                        nck: nck_clone,
                    });
                }
                ui.label(RichText::new("(Requires ADB access and supported device/ROM)").small().color(Color32::GRAY));
            });
        }
    }

    ui.add_space(8.0);
    ui.separator();

    // ── Carrier Portal Instructions ────────────────────────────────────────
    ui.label(RichText::new("Official Carrier Portal Instructions").strong());
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        if ui.button("📋 Generate Full Instructions").clicked() {
            state.send_operation(OperationRequest::AuGenerateUnlockInstructions {
                imei: state.au_unlock_imei.clone(),
                carrier: state.au_unlock_carrier.clone(),
                brand: state.au_unlock_brand.clone(),
            });
        }
    });

    if let Some(instructions) = &state.au_unlock_instructions {
        ui.add_space(6.0);
        ScrollArea::vertical()
            .id_salt("au_instructions_scroll")
            .max_height(280.0)
            .show(ui, |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_rgb(20, 20, 40))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(RichText::new(instructions).monospace().size(11.5));
                    });
            });
        ui.horizontal(|ui| {
            if ui.button("📋 Copy to Clipboard").clicked() {
                ui.output_mut(|o| o.commands.push(egui::output::OutputCommand::CopyText(instructions.clone())));
            }
        });
    }

    ui.add_space(8.0);
    ui.separator();

    // ── Quick Reference: All Australian Carriers ──────────────────────────
    ui.label(RichText::new("📊 Australian Carrier Quick Reference").strong());
    if ui.small_button(if state.au_show_carrier_table { "▲ Hide" } else { "▼ Show All Carriers" }).clicked() {
        state.au_show_carrier_table = !state.au_show_carrier_table;
    }

    if state.au_show_carrier_table {
        ui.add_space(4.0);
        ScrollArea::vertical()
            .id_salt("au_carrier_table")
            .max_height(240.0)
            .show(ui, |ui| {
                egui::Grid::new("au_carrier_grid")
                    .num_columns(5)
                    .spacing([8.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Header
                        for h in &["Carrier", "MCC/MNC", "Type", "Fee", "Wait"] {
                            ui.label(RichText::new(*h).strong().color(Color32::WHITE));
                        }
                        ui.end_row();

                        // Data rows
                        for carrier in AU_CARRIER_QUICK_REF {
                            ui.label(carrier.0);
                            ui.label(carrier.1);
                            ui.label(carrier.2);
                            ui.label(carrier.3);
                            ui.label(carrier.4);
                            ui.end_row();
                        }
                    });
            });
    }
}

/// Quick reference table data (name, mcc/mnc, type, fee, wait)
const AU_CARRIER_QUICK_REF: &[(&str, &str, &str, &str, &str)] = &[
    ("Telstra",           "505/01", "MNO",  "Free",      "1–3 days"),
    ("Optus",             "505/02", "MNO",  "Free",      "3–5 days"),
    ("Vodafone AU",       "505/03", "MNO",  "Free",      "2–5 days"),
    ("TPG Mobile",        "505/90", "MVNO", "Free",      "3–7 days"),
    ("Boost Mobile",      "505/19", "MVNO", "Free*",     "2–5 days"),
    ("Woolworths Mobile", "505/05", "MVNO", "Free",      "3–5 days"),
    ("amaysim",           "505/02", "MVNO", "Free",      "1–5 days"),
    ("Belong",            "505/01", "MVNO", "Free",      "3–5 days"),
    ("Circles.Life",      "505/90", "MVNO", "Free",      "5–7 days"),
    ("Aldi Mobile",       "505/01", "MVNO", "Free",      "3–5 days"),
    ("Dodo Mobile",       "505/02", "MVNO", "Free",      "3–7 days"),
    ("Kogan Mobile",      "505/01", "MVNO", "Free",      "3–5 days"),
    ("Southern Phone",    "505/01", "MVNO", "Free",      "3–7 days"),
];
