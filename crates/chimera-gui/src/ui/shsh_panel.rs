// chimera-gui/src/ui/shsh_panel.rs
//
// SHSH Blob Manager Panel
//
// Tabs:
//  1. Save Blobs      — Request APTicket from Apple TSS or shsh.host for the connected device
//  2. Local Blobs     — Browse blobs saved in ~/Library/Application Support/ChimeraRS/blobs/
//  3. Downgrade Report — Compatibility matrix for current device vs. target iOS
//  4. FutureRestore   — Build futurerestore command with all required flags
//  5. Error Catalogue — Common SHSH/restore error messages and their fixes

use crate::state::{AppState, ShshTab};
use crate::worker::OperationRequest;
use crossbeam_channel::Sender;
use egui::{Color32, RichText, ScrollArea, TextEdit};

// ─── Tab bar colours ──────────────────────────────────────────────────────────
const TAB_ACTIVE:   Color32 = Color32::from_rgb(100, 180, 100);
const WARN_COLOR:   Color32 = Color32::from_rgb(240, 180,  40);
const ERR_COLOR:    Color32 = Color32::from_rgb(220,  80,  80);
const INFO_COLOR:   Color32 = Color32::from_rgb(100, 180, 240);
const CODE_COLOR:   Color32 = Color32::from_rgb(180, 230, 180);

// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn render_shsh_panel(
    ui: &mut egui::Ui,
    state: &mut AppState,
    _op_tx: &Sender<OperationRequest>,
) {
    ui.heading(RichText::new("🔑  SHSH Blob Manager").size(18.0).strong());
    ui.label(RichText::new(
        "Save, manage and replay Apple SHSH2 blobs for iOS downgrade operations."
    ).weak().size(12.0));
    ui.separator();

    // ── Tab bar ───────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        let tabs = [
            (ShshTab::SaveBlobs,        "💾 Save Blobs"),
            (ShshTab::LocalBlobs,       "📁 Local Blobs"),
            (ShshTab::DowngradeReport,  "🔍 Downgrade Report"),
            (ShshTab::FutureRestore,    "⚙️  FutureRestore"),
            (ShshTab::ErrorCatalogue,   "❌ Error Catalogue"),
        ];
        for (tab, label) in &tabs {
            let active = &state.shsh_active_tab == tab;
            let text = if active {
                RichText::new(*label).color(TAB_ACTIVE).strong()
            } else {
                RichText::new(*label)
            };
            if ui.selectable_label(active, text).clicked() {
                state.shsh_active_tab = tab.clone();
            }
        }
    });
    ui.separator();

    ScrollArea::vertical().id_salt("shsh_scroll").show(ui, |ui| {
        match &state.shsh_active_tab.clone() {
            ShshTab::SaveBlobs        => render_save_blobs(ui, state),
            ShshTab::LocalBlobs       => render_local_blobs(ui, state),
            ShshTab::DowngradeReport  => render_downgrade_report(ui, state),
            ShshTab::FutureRestore    => render_futurerestore(ui, state),
            ShshTab::ErrorCatalogue   => render_error_catalogue(ui),
        }
    });
}

// ─── Tab 1: Save Blobs ───────────────────────────────────────────────────────

fn render_save_blobs(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(RichText::new("Request a signed APTicket from Apple TSS for the current iOS version.").weak());
    ui.add_space(6.0);

    egui::Grid::new("shsh_save_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Device ECID (hex or decimal):");
            ui.add(TextEdit::singleline(&mut state.shsh_ecid_input)
                .hint_text("e.g. 0x1A2B3C4D5E6F  or  45678901234567")
                .desired_width(260.0));
            ui.end_row();

            ui.label("Model identifier:");
            ui.add(TextEdit::singleline(&mut state.shsh_model_input)
                .hint_text("e.g. iPhone14,3  (leave blank = auto from device)")
                .desired_width(260.0));
            ui.end_row();

            ui.label("Build number:");
            ui.add(TextEdit::singleline(&mut state.shsh_build_input)
                .hint_text("e.g. 21C62  (leave blank = current device build)")
                .desired_width(260.0));
            ui.end_row();
        });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button(RichText::new("📡 Save from Apple TSS").size(13.0)).clicked() {
            state.shsh_report = format!(
                "→ Would POST to gs.apple.com/TSS/controller?action=2\n\
                 ECID: \n\
                 Model: \n\
                 Build: \n\n\
                 [Stub] In production, TssClient::request_ticket() sends the\n\
                 device parameters and receives a signed plist APTicket.\n\
                 The blob is stored at: ~/Library/Application Support/ChimeraRS/blobs/
ECID: {}
Model: {}
Build: {}",
                if state.shsh_ecid_input.is_empty() { "<from device>" } else { &state.shsh_ecid_input },
                if state.shsh_model_input.is_empty() { "<from device>" } else { &state.shsh_model_input },
                if state.shsh_build_input.is_empty() { "<from device>" } else { &state.shsh_build_input },
            );
        }

        if ui.button(RichText::new("🌐 Fetch from shsh.host").size(13.0)).clicked() {
            state.shsh_report = format!(
                "→ Would GET https://shsh.host/api/blobs?ecid={}&model={}\n\n\
                 shsh.host archives publicly saved blobs.\n\
                 Use ShshHostClient::fetch_all() in chimera-apple/src/shsh.rs\n\
                 to retrieve all available version blobs for this ECID.",
                state.shsh_ecid_input, state.shsh_model_input,
            );
        }

        if ui.button(RichText::new("🌐 Fetch from blobsaver API").size(13.0)).clicked() {
            state.shsh_report = "→ blobsaver uses the same Apple TSS endpoint.\n\
                Compatible .shsh2 files can be loaded via the 'Local Blobs' tab."
                .to_string();
        }
    });

    ui.add_space(6.0);
    ui.collapsing("ℹ️  How SHSH saving works", |ui| {
        ui.label(RichText::new(
            "1. Apple's TSS server (gs.apple.com/TSS/controller?action=2) signs APTickets \
             for firmware versions it is CURRENTLY authorising.\n\
             2. Once Apple stops signing a version, new blob requests are rejected.\n\
             3. You MUST save blobs BEFORE Apple stops signing the version you want to keep.\n\
             4. Tools: TSSSaver (online), blobsaver (desktop), ChimeraRS (here).\n\
             5. Blobs are saved to: ~/Library/Application Support/ChimeraRS/blobs/<model>/<ecid>/"
        ).weak().size(11.0));
    });

    if !state.shsh_report.is_empty() {
        ui.separator();
        ui.label(RichText::new("Result:").strong());
        ui.add(TextEdit::multiline(&mut state.shsh_report.clone())
            .desired_rows(8)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace)
            .interactive(false));
    }
}

// ─── Tab 2: Local Blobs ──────────────────────────────────────────────────────

fn render_local_blobs(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label("Blob storage path:");
        ui.label(RichText::new(
            "~/Library/Application Support/ChimeraRS/blobs/"
        ).monospace().color(CODE_COLOR));
    });
    ui.add_space(4.0);

    if ui.button("🔄 Refresh Blob List").clicked() {
        // Enumerate blobs from BlobStore::default_path()
        let store_path = dirs::data_dir()
            .map(|d| d.join("ChimeraRS").join("blobs"))
            .unwrap_or_default();

        state.shsh_saved_blobs.clear();
        if let Ok(model_dirs) = std::fs::read_dir(&store_path) {
            for model_entry in model_dirs.flatten() {
                let model = model_entry.file_name().to_string_lossy().to_string();
                if let Ok(ecid_dirs) = std::fs::read_dir(model_entry.path()) {
                    for ecid_entry in ecid_dirs.flatten() {
                        let ecid = ecid_entry.file_name().to_string_lossy().to_string();
                        if let Ok(files) = std::fs::read_dir(ecid_entry.path()) {
                            for file in files.flatten() {
                                let fname = file.file_name().to_string_lossy().to_string();
                                state.shsh_saved_blobs.push(
                                    format!("{}/{}/{}", model, ecid, fname)
                                );
                            }
                        }
                    }
                }
            }
        }

        if state.shsh_saved_blobs.is_empty() {
            state.shsh_saved_blobs.push("(No blobs found — save some first)".to_string());
        }
    }

    if !state.shsh_saved_blobs.is_empty() {
        ui.separator();
        ui.label(RichText::new(format!("{} blob(s) found:", state.shsh_saved_blobs.len())).strong());
        egui::Grid::new("blob_list_grid")
            .num_columns(1)
            .striped(true)
            .show(ui, |ui| {
                for blob in &state.shsh_saved_blobs {
                    ui.label(RichText::new(blob).monospace().size(11.0).color(CODE_COLOR));
                    ui.end_row();
                }
            });
    }

    ui.add_space(8.0);
    ui.collapsing("ℹ️  Compatible tools", |ui| {
        ui.label(RichText::new(
            "ChimeraRS .shsh2 files are compatible with:\n\
             • futurerestore -t <blob.shsh2> --latest-sep <firmware.ipsw>\n\
             • blobsaver (import/export)\n\
             • iSHSHit\n\
             • TSSSaver\n\n\
             Blob file format: JSON (ChimeraRS extended) or raw plist (standard .shsh2)\n\
             Both formats are accepted by FutureRestore."
        ).weak().size(11.0));
    });
}

// ─── Tab 3: Downgrade Report ──────────────────────────────────────────────────

fn render_downgrade_report(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(RichText::new("Check if a downgrade is feasible for your device.").weak());
    ui.add_space(6.0);

    egui::Grid::new("dgr_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Current iOS version:");
            ui.add(TextEdit::singleline(&mut state.shsh_model_input)
                .hint_text("e.g. 17.4.1 (auto-filled from device)")
                .desired_width(200.0));
            ui.end_row();

            ui.label("Target iOS version:");
            ui.add(TextEdit::singleline(&mut state.shsh_build_input)
                .hint_text("e.g. 16.7.8")
                .desired_width(200.0));
            ui.end_row();

            ui.label("Device ECID:");
            ui.add(TextEdit::singleline(&mut state.shsh_ecid_input)
                .hint_text("e.g. 0x1A2B3C4D5E6F")
                .desired_width(200.0));
            ui.end_row();
        });

    ui.add_space(4.0);
    if ui.button(RichText::new("🔍 Generate Downgrade Report").size(13.0)).clicked() {
        state.shsh_report = render_downgrade_matrix(
            &state.shsh_model_input,  // current (reusing for demo)
            &state.shsh_build_input,  // target
        );
    }

    if !state.shsh_report.is_empty() {
        ui.separator();
        ScrollArea::vertical().id_salt("dgr_scroll").max_height(300.0).show(ui, |ui| {
            ui.add(TextEdit::multiline(&mut state.shsh_report.clone())
                .desired_rows(12)
                .desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace)
                .interactive(false));
        });
    }

    ui.add_space(6.0);
    ui.collapsing("📊  SEP / Chipset Compatibility Matrix", |ui| {
        egui::Grid::new("sep_matrix")
            .num_columns(5)
            .striped(true)
            .spacing([10.0, 3.0])
            .show(ui, |ui| {
                for label in &["Device", "Chip", "iOS Gap", "Has Blob", "Can Restore?"] {
                    ui.label(RichText::new(*label).strong());
                }
                ui.end_row();

                let rows: &[(&str, &str, &str, &str, &str, Color32)] = &[
                    ("iPhone X",     "A11",  "14 → 15", "✅ Yes", "✅ Yes (nonce must match)", TAB_ACTIVE),
                    ("iPhone X",     "A11",  "14 → 15", "✅ Yes", "❌ No (wrong nonce)",       ERR_COLOR),
                    ("iPhone XS–11", "A12",  "14 → 15", "✅ Yes", "⚠️  Maybe (check SEP)",    WARN_COLOR),
                    ("iPhone 12–14", "A14+", "14 → 16", "✅ Yes", "⚠️  Very unlikely",         WARN_COLOR),
                    ("iPhone 15–17", "A16+", "any",      "✅ Yes", "❌ No (Cryptex1/SEP)",      ERR_COLOR),
                    ("Any",          "any",  "any",      "❌ No",  "❌ No (no blob)",           ERR_COLOR),
                ];
                for (dev, chip, gap, blob, result, color) in rows {
                    ui.label(*dev);
                    ui.label(RichText::new(*chip).monospace());
                    ui.label(*gap);
                    ui.label(*blob);
                    ui.label(RichText::new(*result).color(*color));
                    ui.end_row();
                }
            });
    });
}

fn render_downgrade_matrix(current: &str, target: &str) -> String {
    format!(
        "Downgrade Compatibility Report\n\
         ══════════════════════════════════════\n\
         Current iOS : {current}\n\
         Target  iOS : {target}\n\n\
         ── Blob Requirements ──────────────────\n\
         • You need a valid SHSH2 blob for iOS {target} saved BEFORE Apple\n\
           stopped signing it.\n\
         • The blob must contain the correct APNonce/generator pair.\n\
         • The blob ECID must match this device's ECID exactly.\n\n\
         ── SEP Compatibility ───────────────────\n\
         • A11 (iPhone X/8/8+): Compatible with most iOS 12–16 downgrades\n\
           if blob + nonce are correct.\n\
         • A12–A15 (XS – iPhone 14): Use --latest-sep flag in futurerestore.\n\
           The SEP from the CURRENT signed version is used, bridging the gap.\n\
         • A16+ (iPhone 15+): BLOCKED. Cryptex1 + new SEP architecture\n\
           makes downgrades practically impossible regardless of blobs.\n\n\
         ── Recommended Command ─────────────────\n\
         futurerestore \\\n\
           -t /path/to/blob.shsh2 \\\n\
           --latest-sep \\\n\
           /path/to/iOS_{target}_<Model>.ipsw\n\n\
         See the FutureRestore tab to auto-generate this command.",
        current = if current.is_empty() { "unknown" } else { current },
        target  = if target.is_empty()  { "unknown" } else { target  },
    )
}

// ─── Tab 4: FutureRestore Builder ────────────────────────────────────────────

fn render_futurerestore(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(RichText::new(
        "Build the futurerestore command for a downgrade or restore with saved blobs."
    ).weak());
    ui.add_space(6.0);

    egui::Grid::new("fr_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("IPSW path:");
            ui.add(TextEdit::singleline(&mut state.apple_ipsw_path)
                .hint_text("/path/to/firmware.ipsw")
                .desired_width(320.0));
            ui.end_row();

            ui.label("SHSH2 blob path:");
            ui.add(TextEdit::singleline(&mut state.shsh_blob_path)
                .hint_text("/path/to/blob.shsh2")
                .desired_width(320.0));
            ui.end_row();

            ui.label("Nonce generator:");
            ui.add(TextEdit::singleline(&mut state.shsh_nonce_gen)
                .hint_text("0xBD34A960BF0D087F  (leave blank if not needed)")
                .desired_width(260.0));
            ui.end_row();

            ui.label("Use --latest-sep:");
            ui.checkbox(&mut state.shsh_use_latest_sep,
                "Recommended for A12+ devices");
            ui.end_row();

            ui.label("Use --latest-baseband:");
            ui.checkbox(&mut state.shsh_use_latest_baseband,
                "Use if baseband version mismatch");
            ui.end_row();
        });

    ui.add_space(4.0);
    if ui.button(RichText::new("⚙️  Generate futurerestore Command").size(13.0)).clicked() {
        let mut cmd = String::from("futurerestore");
        if !state.shsh_blob_path.is_empty() {
            cmd.push_str(&format!(" \\\n  -t \"{}\"", state.shsh_blob_path));
        }
        if state.shsh_use_latest_sep {
            cmd.push_str(" \\\n  --latest-sep");
        }
        if state.shsh_use_latest_baseband {
            cmd.push_str(" \\\n  --latest-baseband");
        }
        if !state.shsh_nonce_gen.is_empty() {
            cmd.push_str(&format!(" \\\n  --generator \"{}\"", state.shsh_nonce_gen));
        }
        if !state.apple_ipsw_path.is_empty() {
            cmd.push_str(&format!(" \\\n  \"{}\"", state.apple_ipsw_path));
        } else {
            cmd.push_str(" \\\n  /path/to/firmware.ipsw");
        }
        state.shsh_futurerestore_cmd = cmd;
    }

    if !state.shsh_futurerestore_cmd.is_empty() {
        ui.separator();
        ui.label(RichText::new("Generated Command:").strong());
        ui.add(TextEdit::multiline(&mut state.shsh_futurerestore_cmd.clone())
            .desired_rows(6)
            .desired_width(f32::INFINITY)
            .font(egui::TextStyle::Monospace)
            .interactive(false));
        if ui.button("📋 Copy Command").clicked() {
            ui.output_mut(|o| o.commands.push(egui::output::OutputCommand::CopyText(state.shsh_futurerestore_cmd.clone())));
        }
    }

    ui.add_space(8.0);
    ui.collapsing("📖  futurerestore Guide", |ui| {
        ui.label(RichText::new(
            "Prerequisites:\n\
             1. Device must be in DFU mode\n\
             2. A valid SHSH2 blob saved when the target iOS was being signed\n\
             3. The correct generator/nonce set on the device (jailbreak required)\n\
             4. The IPSW for the target iOS version\n\
             5. futurerestore binary installed (brew install futurerestore or compile from source)\n\n\
             Nonce setter tools:\n\
             • misaka (iOS 15–17, A12–A17 Pro)\n\
             • SuccessionRestore (iOS 14–16)\n\
             • palera1n (A8–A11, checkm8)\n\n\
             Common flags:\n\
             --latest-sep     Use current SEP firmware (bridges SEP version gaps)\n\
             --latest-baseband Use current baseband (bridges BB version gaps)\n\
             -t <blob>        Path to your SHSH2 file\n\
             --generator      Set the APNonce generator seed"
        ).weak().size(11.0));
    });
}

// ─── Tab 5: Error Catalogue ───────────────────────────────────────────────────

fn render_error_catalogue(ui: &mut egui::Ui) {
    ui.label(RichText::new(
        "Common SHSH / restore error messages and their resolutions."
    ).weak());
    ui.add_space(8.0);

    let errors: &[(&str, &str, &str)] = &[
        (
            "This device isn't eligible for the requested build",
            "Apple has stopped signing this iOS version. You CANNOT restore \
             to it without a pre-saved SHSH2 blob.",
            "Save blobs NOW for the version you want to keep using TSSSaver, \
             blobsaver, or ChimeraRS (Save Blobs tab). Once Apple stops signing, \
             it's too late.",
        ),
        (
            "Missing SHSH2 Blobs",
            "No blob exists for this device + iOS combination. The restore \
             cannot be verified against Apple TSS.",
            "Use IpswMeClient or shsh.host to check if any third-party archives \
             have your blobs. If not, only the CURRENT signed version is restorable.",
        ),
        (
            "SEP Incompatibility / SEP firmware is too new",
            "The Secure Enclave firmware in your saved blob is older than the \
             SEP that was flashed by a later iOS update. The gap is too large.",
            "Try futurerestore with --latest-sep to use the current SEP. \
             If the gap exceeds Apple's allowed range, the restore will fail regardless. \
             A12+ devices are most affected.",
        ),
        (
            "Incorrect Nonce Generator / APNonce mismatch",
            "The blob was saved with a specific APNonce. The device is generating \
             a different nonce at boot, so the blob is rejected.",
            "Set the matching generator on the device using misaka, SuccessionRestore, \
             or palera1n BEFORE entering DFU. The generator is stored inside the blob file.",
        ),
        (
            "SHSH blobs are corrupted",
            "The blob file is incomplete, truncated, or was saved incorrectly.",
            "Re-save the blob using a fresh TSS request. If Apple is still signing \
             the version, use TSSSaver or blobsaver to get a clean copy. \
             Verify the blob with futurerestore --verify.",
        ),
        (
            "Unsigned iOS Version",
            "Apple is not signing this version. Any restore attempt will be blocked \
             at the TSS check unless you have a pre-saved blob.",
            "Check https://ipsw.me/signing to see what Apple is currently signing. \
             Use a saved blob + futurerestore for unsigned versions.",
        ),
        (
            "iPhone X (and earlier A11) — iOS 16+ blob useless",
            "A11 and older devices CAN use blobs for downgrading, but only to versions \
             that Apple signed when the blob was saved. iOS 16+ blobs are valid but \
             downgrades still require correct nonce.",
            "Ensure the generator is set correctly. A11 devices are the LAST generation \
             where downgrading with blobs is reliably possible.",
        ),
        (
            "iPhone 12+ (A14+) — blobs largely useless",
            "A14+ devices running iOS 16+ have Cryptex1 and a new SEP architecture. \
             Even with valid blobs, the SEP and baseband firmware are forward-only \
             and will reject older iOS combinations.",
            "This is a hardware/firmware limitation. No known bypass exists. \
             The only option is restoring to a currently signed iOS version.",
        ),
    ];

    for (error, cause, fix) in errors {
        egui::CollapsingHeader::new(
            RichText::new(format!("⚠️  {}", error))
                .color(WARN_COLOR)
                .size(13.0)
        )
        .id_salt(*error)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Cause:").strong().color(INFO_COLOR));
                ui.label(*cause);
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Fix:").strong().color(TAB_ACTIVE));
                ui.label(*fix);
            });
        });
        ui.add_space(2.0);
    }

    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("Further resources:").strong());
    ui.horizontal(|ui| {
        ui.hyperlink_to("ipsw.me signing status", "https://ipsw.me/signing");
        ui.label("•");
        ui.hyperlink_to("shsh.host blob archive", "https://shsh.host");
        ui.label("•");
        ui.hyperlink_to("TSSSaver", "https://tsssaver.1conan.com");
        ui.label("•");
        ui.hyperlink_to("futurerestore GitHub", "https://github.com/futurerestore/futurerestore");
    });
}
