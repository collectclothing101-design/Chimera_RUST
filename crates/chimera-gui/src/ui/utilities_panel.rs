// chimera-gui/src/ui/utilities_panel.rs
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
use eframe::egui::{self, RichText};
use crate::state::AppState;
use crate::worker::OperationRequest;
use crate::ui::common::*;
use crossbeam_channel::Sender;

pub fn render_utilities(ui: &mut egui::Ui, state: &mut AppState, op_tx: &Sender<OperationRequest>) {
    ui.columns(2, |cols| {
        // LEFT - IMEI Tools
        let ui = &mut cols[0];
        section_header(ui, "📋 IMEI Checker");
        
        ui.horizontal(|ui| {
            ui.label("IMEI:");
            ui.text_edit_singleline(&mut state.imei_check_input);
        });
        
        ui.horizontal(|ui| {
            if ui.button("✅ Validate").clicked() {
                let result = match chimera_core::imei::validate_imei(&state.imei_check_input) {
                    Ok(_) => format!("✅ IMEI {} is VALID (Luhn OK)", state.imei_check_input),
                    Err(_e) => format!("❌ IMEI INVALID: {}", _e),
                };
                state.imei_check_result = Some(result);
            }
            if ui.button("🔍 Check Online").clicked() {
                let _ = op_tx.send(OperationRequest::CheckImei { imei: state.imei_check_input.clone() });
            }
        });
        
        if let Some(result) = &state.imei_check_result {
            if result.starts_with("✅") {
                success_box(ui, result);
            } else {
                error_box(ui, result);
            }
        }
        
        // Show IMEI details
        if !state.imei_check_input.is_empty() {
            ui.add_space(4.0);
            ui.label(format!("TAC: {}", &state.imei_check_input[..8.min(state.imei_check_input.len())]));
            ui.label(format!("Formatted: {}", chimera_core::imei::format_imei(&state.imei_check_input)));
        }
        
        ui.add_space(12.0);
        section_header(ui, "📡 Network Code Calculator");
        
        ui.horizontal(|ui| {
            ui.label("IMEI:");
            ui.text_edit_singleline(&mut state.network_code_input);
        });
        
        if ui.button("🔢 Calculate NCK").clicked() {
            let _ = op_tx.send(OperationRequest::CalculateNetworkCode { imei: state.network_code_input.clone() });
        }
        
        if let Some(result) = &state.network_code_result {
            ui.label(RichText::new(result).monospace().size(16.0));
            if ui.small_button("📋 Copy").clicked() {
                ui.output_mut(|o| o.commands.push(egui::OutputCommand::CopyText(result.clone())));
            }
        }
        
        ui.add_space(12.0);
        section_header(ui, "📲 Enable ADB (QR Code)");
        ui.label("Generate QR code to enable ADB on Android 11+");
        if ui.button("📲 Generate QR Code").clicked() {
            let _qr_data = "WIFI:T:ADB;S:ChimeraRS;P:chimera_pair;;";
            state.add_log(crate::state::LogEntry::info(format!("QR Data: {}", _qr_data)));
            state.add_log(crate::state::LogEntry::info("Scan with 'pair device via QR code' in Android Developer Options > Wireless Debugging"));
        }
        
        // RIGHT - More tools
        let ui = &mut cols[1];
        section_header(ui, "🔧 Additional Utilities");
        
        if ui.button("🔢 IMEI Luhn Fixer").clicked() {
            if state.imei_check_input.len() == 14 {
                if let Ok(_result) = chimera_core::imei::complete_imei(&state.imei_check_input) {
                    state.imei_check_result = Some(format!("✅ Completed IMEI: {}", _result));
                }
            }
        }
        
        ui.add_space(8.0);
        section_header(ui, "ℹ️ Connection Guide");
        
        egui::CollapsingHeader::new("📱 Samsung Download Mode").show(ui, |ui| {
            ui.label("• Hold Volume Down + Home + Power simultaneously");
            ui.label("• Connect USB cable when in Download Mode");
            ui.label("• For Exynos: EUB mode requires test points");
        });
        
        egui::CollapsingHeader::new("📱 Qualcomm EDL Mode").show(ui, |ui| {
            ui.label("• Power off the device completely");
            ui.label("• Short the EDL test points on the motherboard");
            ui.label("• Connect USB cable - PC shows COM port or 9008");
        });
        
        egui::CollapsingHeader::new("📱 ADB Mode (All devices)").show(ui, |ui| {
            ui.label("• Settings > About Phone > Tap Build Number 7x");
            ui.label("• Settings > Developer Options > USB Debugging: ON");
            ui.label("• Connect USB > Allow debugging prompt");
        });
        
        egui::CollapsingHeader::new("📱 MTK BootROM Mode").show(ui, |ui| {
            ui.label("• Power off device");
            ui.label("• Hold Vol Down + Vol Up and connect USB");
            ui.label("• Device appears as MTK USB device (VID:0E8D)");
        });
        
        egui::CollapsingHeader::new("📱 Xiaomi EDL Mode").show(ui, |ui| {
            ui.label("• Remove battery if possible");
            ui.label("• Short test points on motherboard");
            ui.label("• Connect USB - appears as Qualcomm 9008");
        });
        
        egui::CollapsingHeader::new("📱 Huawei Factory Fastboot").show(ui, |ui| {
            ui.label("• Power off device");
            ui.label("• Hold Vol Down and connect USB cable");
            ui.label("• Or: Hold Vol Down + Power");
        });
    });
}
