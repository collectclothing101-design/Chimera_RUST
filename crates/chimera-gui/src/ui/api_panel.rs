// chimera-gui/src/ui/api_panel.rs
//
// API Tools Panel — maps ChimeraTool subdomains to local/mock equivalents.
// Subdomains mapped:
//   api.chimeratool.com          -> ChimeraRS REST API client (auth, device info, credits)
//   secure.chimeratool.com       -> IMEI verification, SHSH blob, cert checks
//   data.chimeratool.com         -> Firmware metadata, device DB queries
//   upload.chimeratool.com       -> Log/firmware upload
//   pics.chimeratool.com         -> Device images/thumbnails
//   portcheck.chimeratool.com    -> ADB TCP / network port reachability tests
//   stage.chimeratool.com        -> Staging/beta endpoint testing
//   administration.chimeratool.com -> Admin-equivalent local settings
//   bb.chimeratool.com           -> Backend bus / message broker proxy
//   munin.chimeratool.com        -> Monitoring / health dashboard
//
// In "Mock Mode" all requests are intercepted locally — no real network calls.
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

use eframe::egui;
use crate::state::AppState;
use crate::worker::OperationRequest;
use crossbeam_channel::Sender;

// ─── Endpoint catalogue ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct EndpointEntry {
    label:       &'static str,
    subdomain:   &'static str,
    path:        &'static str,
    method:      &'static str,
    description: &'static str,
    sample_body: &'static str,
}

const ENDPOINTS: &[EndpointEntry] = &[
    EndpointEntry {
        label:       "Auth — Login",
        subdomain:   "api",
        path:        "/v1/auth/login",
        method:      "POST",
        description: "Authenticate with username + password. Returns session token.",
        sample_body: r#"{"username":"user@example.com","password":"secret"}"#,
    },
    EndpointEntry {
        label:       "Auth — Logout",
        subdomain:   "api",
        path:        "/v1/auth/logout",
        method:      "POST",
        description: "Invalidate current session token.",
        sample_body: r#"{}"#,
    },
    EndpointEntry {
        label:       "Account — Credits",
        subdomain:   "api",
        path:        "/v1/account/credits",
        method:      "GET",
        description: "Fetch remaining credits for the current account.",
        sample_body: r#"{}"#,
    },
    EndpointEntry {
        label:       "Device — Info",
        subdomain:   "api",
        path:        "/v1/device/info",
        method:      "POST",
        description: "Read device information (model, IMEI, serial) via IMEI lookup.",
        sample_body: r#"{"imei":"353879234567890"}"#,
    },
    EndpointEntry {
        label:       "Device — Supported Operations",
        subdomain:   "api",
        path:        "/v1/device/supported_ops",
        method:      "POST",
        description: "List supported operations for a given device model.",
        sample_body: r#"{"model":"SM-G991B","brand":"Samsung"}"#,
    },
    EndpointEntry {
        label:       "Secure — IMEI Check",
        subdomain:   "secure",
        path:        "/v1/imei/check",
        method:      "POST",
        description: "Validate IMEI via GSMA / carrier database. Returns carrier, blacklist status.",
        sample_body: r#"{"imei":"353879234567890"}"#,
    },
    EndpointEntry {
        label:       "Secure — SHSH Blob",
        subdomain:   "secure",
        path:        "/v1/shsh/fetch",
        method:      "POST",
        description: "Fetch SHSH blobs for an Apple device (ECID + model).",
        sample_body: r#"{"ecid":"0x1A2B3C4D5E6F","model":"iPhone14,5","board":"d53g"}"#,
    },
    EndpointEntry {
        label:       "Secure — Certificate Verify",
        subdomain:   "secure",
        path:        "/v1/cert/verify",
        method:      "POST",
        description: "Verify an Apple activation certificate chain.",
        sample_body: r#"{"cert_base64":"<base64>"}"#,
    },
    EndpointEntry {
        label:       "Data — Firmware Search",
        subdomain:   "data",
        path:        "/v1/firmware/search",
        method:      "POST",
        description: "Search firmware by model + region. Returns CSC, PDA, AP versions.",
        sample_body: r#"{"model":"SM-G991B","region":"OXM"}"#,
    },
    EndpointEntry {
        label:       "Data — Device DB Query",
        subdomain:   "data",
        path:        "/v1/devices/query",
        method:      "POST",
        description: "Query device specifications from the ChimeraRS device database.",
        sample_body: r#"{"brand":"Apple","model":"iPhone16,1"}"#,
    },
    EndpointEntry {
        label:       "Data — IPSW Latest",
        subdomain:   "data",
        path:        "/v1/ipsw/latest",
        method:      "POST",
        description: "Fetch the latest signed IPSW URL for a given Apple identifier.",
        sample_body: r#"{"identifier":"iPhone16,1","signed_only":true}"#,
    },
    EndpointEntry {
        label:       "Upload — Diagnostics Log",
        subdomain:   "upload",
        path:        "/v1/upload/log",
        method:      "POST",
        description: "Upload a diagnostics log bundle (multipart/form-data).",
        sample_body: r#"{"filename":"diag_20250313.zip","note":"Crash on restore"}"#,
    },
    EndpointEntry {
        label:       "Pics — Device Image",
        subdomain:   "pics",
        path:        "/v1/device/image",
        method:      "GET",
        description: "Fetch device thumbnail PNG by model identifier.",
        sample_body: r#"{"model":"iPhone16,1"}"#,
    },
    EndpointEntry {
        label:       "Port Check — ADB TCP",
        subdomain:   "portcheck",
        path:        "/v1/check",
        method:      "POST",
        description: "Test whether a remote host:port is reachable for ADB TCP connections.",
        sample_body: r#"{"host":"192.168.1.50","port":5555}"#,
    },
    EndpointEntry {
        label:       "Port Check — Custom Port",
        subdomain:   "portcheck",
        path:        "/v1/check",
        method:      "POST",
        description: "Test an arbitrary TCP port for device connectivity.",
        sample_body: r#"{"host":"10.0.0.1","port":9001}"#,
    },
    EndpointEntry {
        label:       "AU Unlock — NCK Request",
        subdomain:   "api",
        path:        "/v1/unlock/au/nck",
        method:      "POST",
        description: "Request an NCK network unlock code for an Australian carrier IMEI.",
        sample_body: r#"{"imei":"353879234567890","carrier_mnc":"501","brand":"Samsung"}"#,
    },
    EndpointEntry {
        label:       "Apple — iCloud Status",
        subdomain:   "api",
        path:        "/v1/apple/icloud/status",
        method:      "POST",
        description: "Check activation lock / iCloud lock status for an Apple device by serial.",
        sample_body: r#"{"serial":"F4GT8H91XXXX","imei":"353879234567890"}"#,
    },
    EndpointEntry {
        label:       "Apple — Carrier Unlock",
        subdomain:   "api",
        path:        "/v1/apple/unlock/carrier",
        method:      "POST",
        description: "Submit carrier unlock request for an iPhone by IMEI + carrier.",
        sample_body: r#"{"imei":"353879234567890","carrier":"Telstra","country":"AU"}"#,
    },
    EndpointEntry {
        label:       "Stage — Endpoint Test",
        subdomain:   "stage",
        path:        "/v1/ping",
        method:      "GET",
        description: "Ping the staging environment to verify connectivity.",
        sample_body: r#"{}"#,
    },
    EndpointEntry {
        label:       "Admin — System Status",
        subdomain:   "administration",
        path:        "/v1/status",
        method:      "GET",
        description: "Retrieve local ChimeraRS system status (crate versions, DB health).",
        sample_body: r#"{}"#,
    },
];

// ─── Mock responses ────────────────────────────────────────────────────────

fn mock_response(endpoint: &EndpointEntry, state: &AppState) -> String {
    match endpoint.path {
        "/v1/auth/login" =>
            r#"{"status":"ok","token":"chimera_mock_token_abc123","expires_in":3600}"#.to_string(),
        "/v1/account/credits" =>
            r#"{"status":"ok","credits":9999,"plan":"ChimeraRS_Offline","expires":"2099-12-31"}"#.to_string(),
        "/v1/device/info" => {
            let imei = if state.api_imei_query.is_empty() { "353879234567890" } else { &state.api_imei_query };
            format!(r#"{{"status":"ok","imei":"{}","brand":"Samsung","model":"SM-G991B","carrier":"Telstra","country":"AU","blacklisted":false}}"#, imei)
        },
        "/v1/device/supported_ops" =>
            r#"{"status":"ok","ops":["get_info","flash_firmware","remove_frp","repair_imei","network_unlock","samsung_root"]}"#.to_string(),
        "/v1/imei/check" => {
            let imei = if state.api_imei_query.is_empty() { "353879234567890" } else { &state.api_imei_query };
            format!(r#"{{"status":"ok","imei":"{}","valid":true,"carrier":"Telstra AU","mccmnc":"50501","blacklisted":false,"country":"Australia"}}"#, imei)
        },
        "/v1/shsh/fetch" =>
            r#"{"status":"ok","blobs":[{"version":"17.4.1","generator":"0xBD34A960BF0D087F","blob_url":"https://shsh.host/blob/ecid_17.4.1.shsh2"}]}"#.to_string(),
        "/v1/firmware/search" => {
            let model = if state.api_firmware_model.is_empty() { "SM-G991B" } else { &state.api_firmware_model };
            let region = if state.api_firmware_region.is_empty() { "OXM" } else { &state.api_firmware_region };
            format!(r#"{{"status":"ok","model":"{}","region":"{}","pda":"G991BXXS9EXA1","csc":"G991BOXM9EXA1","modem":"G991BXXS9EXA1","android":"14","size_mb":4821}}"#, model, region)
        },
        "/v1/devices/query" =>
            r#"{"status":"ok","brand":"Apple","identifier":"iPhone16,1","name":"iPhone 15","chip":"A16 Bionic","checkm8_support":false,"ios_latest":"17.4.1"}"#.to_string(),
        "/v1/ipsw/latest" =>
            r#"{"status":"ok","identifier":"iPhone16,1","version":"17.4.1","url":"https://updates.cdn-apple.com/2024/mobileassets/071-06570/iPhone16,1_17.4.1_21E236_Restore.ipsw","signed":true,"size_gb":6.8}"#.to_string(),
        "/v1/upload/log" =>
            r#"{"status":"ok","upload_id":"log_20250313_abc","message":"Log received and queued for analysis."}"#.to_string(),
        "/v1/device/image" =>
            r#"{"status":"ok","image_url":"https://pics.chimeratool.com/devices/iPhone16,1.png","resolution":"512x512"}"#.to_string(),
        "/v1/check" =>
            r#"{"status":"ok","reachable":true,"latency_ms":12,"message":"Port 5555 open on 192.168.1.50"}"#.to_string(),
        "/v1/unlock/au/nck" =>
            r#"{"status":"ok","nck":"12345678","carrier":"Telstra","unlock_method":"dial_*#7465625*638*NCK#","wait_days":1}"#.to_string(),
        "/v1/apple/icloud/status" =>
            r#"{"status":"ok","activation_locked":false,"find_my_enabled":false,"carrier_locked":true,"carrier":"Optus","mdm_enrolled":false}"#.to_string(),
        "/v1/apple/unlock/carrier" =>
            r#"{"status":"ok","submitted":true,"reference":"CHM-AU-2025-0042","estimated_days":2,"message":"Carrier unlock request submitted to Telstra AU."}"#.to_string(),
        "/v1/ping" =>
            r#"{"status":"ok","env":"stage","version":"1.2.0","timestamp":"2025-03-13T00:00:00Z"}"#.to_string(),
        "/v1/status" =>
            r#"{"status":"ok","chimera_rs":"1.2.0","crates":21,"source_files":132,"db_devices":1284,"uptime_s":3600}"#.to_string(),
        _ => format!(r#"{{"status":"ok","mock":true,"endpoint":"{}"}}"#, endpoint.path),
    }
}

// ─── Render ────────────────────────────────────────────────────────────────

pub fn render_api_panel(
    ui: &mut egui::Ui,
    state: &mut AppState,
    _op_tx: &Sender<OperationRequest>,
) {
    ui.heading("🌐 API Tools — ChimeraTool Endpoint Explorer");
    ui.label(
        egui::RichText::new(
            "Maps ChimeraTool subdomains (api / secure / data / upload / pics / portcheck) to \
             ChimeraRS equivalents. In Mock Mode all responses are generated locally."
        ).weak().size(12.0)
    );
    ui.separator();

    // ── Top controls ──────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Base URL:");
        ui.add(egui::TextEdit::singleline(&mut state.api_base_url).desired_width(320.0));
        ui.separator();
        ui.label("Token:");
        ui.add(egui::TextEdit::singleline(&mut state.api_token)
            .password(true)
            .desired_width(200.0)
            .hint_text("(empty = mock mode)"));
    });

    ui.horizontal(|ui| {
        ui.checkbox(&mut state.api_mock_mode, "🔵 Mock Mode (offline / no real requests)");
        if state.api_mock_mode {
            ui.label(egui::RichText::new("All responses are synthetic — no network calls made.").color(egui::Color32::from_rgb(80, 160, 255)));
        } else {
            ui.label(egui::RichText::new("⚠ Live mode — real HTTP requests will be sent.").color(egui::Color32::YELLOW));
        }
    });

    ui.separator();

    // ── Quick-fill helpers ─────────────────────────────────────────────────
    ui.collapsing("🔎 Quick-fill Query Fields", |ui| {
        ui.horizontal(|ui| {
            ui.label("IMEI:");
            ui.add(egui::TextEdit::singleline(&mut state.api_imei_query)
                .desired_width(200.0)
                .hint_text("e.g. 353879234567890"));
        });
        ui.horizontal(|ui| {
            ui.label("Firmware Model:");
            ui.add(egui::TextEdit::singleline(&mut state.api_firmware_model)
                .desired_width(150.0)
                .hint_text("e.g. SM-G991B"));
            ui.label("Region:");
            ui.add(egui::TextEdit::singleline(&mut state.api_firmware_region)
                .desired_width(80.0)
                .hint_text("OXM"));
        });
    });

    ui.separator();

    // ── Endpoint selector ─────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Endpoint:");
        egui::ComboBox::from_id_salt("api_endpoint_combo")
            .selected_text(
                ENDPOINTS.get(state.api_selected_endpoint)
                    .map(|e| e.label)
                    .unwrap_or("Select…")
            )
            .width(420.0)
            .show_ui(ui, |ui| {
                for (i, ep) in ENDPOINTS.iter().enumerate() {
                    let label = format!("[{}] {}.chimeratool.com{}", ep.method, ep.subdomain, ep.path);
                    ui.selectable_value(&mut state.api_selected_endpoint, i, label);
                }
            });
    });

    if let Some(ep) = ENDPOINTS.get(state.api_selected_endpoint) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("▶ {}.chimeratool.com{}", ep.subdomain, ep.path))
                .color(egui::Color32::from_rgb(130, 220, 130))
                .monospace());
            ui.separator();
            ui.label(egui::RichText::new(ep.method).strong());
        });
        ui.label(ep.description);
        ui.separator();

        // Request body
        ui.label("Request Body (JSON):");
        // Pre-fill if empty
        if state.api_request_body.is_empty() {
            state.api_request_body = ep.sample_body.to_string();
        }
        ui.add(
            egui::TextEdit::multiline(&mut state.api_request_body)
                .desired_rows(4)
                .desired_width(f32::INFINITY)
                .font(egui::TextStyle::Monospace)
        );

        ui.horizontal(|ui| {
            let btn_label = if state.api_mock_mode { "⚡ Execute (Mock)" } else { "🚀 Send Request" };
            if ui.button(egui::RichText::new(btn_label).size(14.0)).clicked() {
                let resp = if state.api_mock_mode {
                    mock_response(ep, state)
                } else {
                    format!(
                        "// Live HTTP not wired in GUI yet.\n// Would POST to: {}.chimeratool.com{}\n// Body: {}",
                        ep.subdomain, ep.path, state.api_request_body
                    )
                };
                state.api_response = resp;
                state.api_last_status = if state.api_mock_mode {
                    "200 OK (mock)".to_string()
                } else {
                    "Not connected".to_string()
                };
            }

            if ui.button("🔄 Reset Body").clicked() {
                state.api_request_body = ep.sample_body.to_string();
            }

            if ui.button("🗑 Clear Response").clicked() {
                state.api_response.clear();
                state.api_last_status.clear();
            }
        });
    }

    ui.separator();

    // ── Response viewer ───────────────────────────────────────────────────
    if !state.api_last_status.is_empty() {
        let status_color = if state.api_last_status.contains("200") {
            egui::Color32::from_rgb(80, 200, 80)
        } else {
            egui::Color32::YELLOW
        };
        ui.horizontal(|ui| {
            ui.label("Status:");
            ui.label(egui::RichText::new(&state.api_last_status).color(status_color).strong());
        });
    }

    if !state.api_response.is_empty() {
        ui.label("Response:");
        egui::ScrollArea::vertical()
            .id_salt("api_resp_scroll")
            .max_height(260.0)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut state.api_response.clone())
                        .desired_rows(10)
                        .desired_width(f32::INFINITY)
                        .font(egui::TextStyle::Monospace)
                        .interactive(false)
                );
            });
        if ui.button("📋 Copy Response").clicked() {
            ui.output_mut(|o| o.commands.push(egui::output::OutputCommand::CopyText(state.api_response.clone())));
        }
    }

    ui.separator();

    // ── Subdomain reference table ──────────────────────────────────────────
    ui.collapsing("📡 Subdomain Map — chimeratool.com", |ui| {
        egui::Grid::new("subdomain_grid")
            .num_columns(4)
            .striped(true)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Subdomain").strong());
                ui.label(egui::RichText::new("IP(s)").strong());
                ui.label(egui::RichText::new("Role").strong());
                ui.label(egui::RichText::new("ChimeraRS Equivalent").strong());
                ui.end_row();

                let rows: &[(&str, &str, &str, &str)] = &[
                    ("api",            "104.18.14.248 / .15.248",           "Main REST API",                  "chimera-api / local HTTP server"),
                    ("secure",         "104.18.14.248 / .15.248",           "IMEI / SHSH / certs",            "chimera-api::secure_api"),
                    ("data",           "104.18.14.248 / .15.248",           "Firmware DB / device data",      "chimera-firmware + chimera-devices"),
                    ("upload",         "188.114.98.228 / .99.228",          "Log & firmware uploads",         "chimera-api::upload_api"),
                    ("pics",           "104.18.14.248 / .15.248",           "Device thumbnails",              "chimera-api::pics_api"),
                    ("portcheck",      "104.18.14.248 / .15.248",           "TCP reachability tests",         "chimera-api::portcheck"),
                    ("stage",          "104.18.14.248 / .15.248",           "Staging / beta",                 "mock_server (local)"),
                    ("administration", "104.18.14.248 / .15.248",           "Admin dashboard",                "chimera-gui settings panel"),
                    ("bb",             "104.18.14.248 / .15.248",           "Backend bus / broker",           "crossbeam-channel worker"),
                    ("munin",          "104.18.14.248 / .15.248",           "Monitoring / Munin graphs",      "chimera-gui diagnostics"),
                    ("chat",           "188.114.99.228 / .98.228",          "Support chat",                   "N/A (external)"),
                    ("dev",            "88.151.102.23",                      "Dev / internal server",          "localhost dev build"),
                    ("mail[1-5]",      "78.24.x / 88.151.x / 37.48.x",     "Mail infrastructure",            "N/A"),
                    ("www",            "172.66.130.193 / .194",             "Public website",                 "N/A"),
                ];
                for (sub, ip, role, equiv) in rows {
                    ui.label(egui::RichText::new(format!("{}.chimeratool.com", sub)).monospace().color(egui::Color32::from_rgb(130, 200, 255)));
                    ui.label(egui::RichText::new(*ip).monospace().weak());
                    ui.label(*role);
                    ui.label(egui::RichText::new(*equiv).weak());
                    ui.end_row();
                }
            });
    });
    ui.separator();

    // ── iCloud Endpoint Reference (probe-confirmed + restore-relevant) ─────
    ui.collapsing("🍎 iCloud Endpoint Map — Probe-Confirmed & Restore-Relevant", |ui| {
        egui::ScrollArea::vertical()
            .id_salt("icloud_endpoint_scroll")
            .max_height(340.0)
            .show(ui, |ui| {
                egui::Grid::new("icloud_endpoint_grid")
                    .num_columns(4)
                    .striped(true)
                    .spacing([8.0, 3.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("FQDN").strong());
                        ui.label(egui::RichText::new("IPv4").strong());
                        ui.label(egui::RichText::new("Role").strong());
                        ui.label(egui::RichText::new("Description").strong());
                        ui.end_row();

                        // Probe-confirmed + restore-relevant entries, sourced from
                        // icloud_endpoints.rs (ICLOUD_ENDPOINTS catalog).
                        // Curl probes 2026-03-13: all endpoints return HTTP 301 → HTTPS
                        // via AppleHttpServer/a3fb6e96e80a.
                        let rows: &[(&str, &str, &str, &str)] = &[
                            // ── Probe-Confirmed ─────────────────────────────────────
                            ("beta.icloud.com",
                             "23.11.166.5",
                             "Beta / Web",
                             "iCloud Beta web; 301→HTTPS confirmed 2026-03-13"),
                            ("background.gateway.icloud.com",
                             "17.248.219.23/66/39/8",
                             "Gateway",
                             "APNs background fetch; 301→HTTPS confirmed 2026-03-13"),
                            ("ckhttpapi.icloud.com",
                             "17.248.219.15/23/66/8",
                             "CloudKit",
                             "CloudKit HTTP API; 301→HTTPS confirmed 2026-03-13"),
                            ("imap.mail.icloud.com",
                             "17.56.136.196",
                             "Mail / IMAP",
                             "iCloud IMAP4 server (port 993 TLS). Confirmed 2026-01-02"),
                            ("mailws.icloud.com",
                             "17.172.192.54",
                             "Mail WS",
                             "iCloud Mail WebService backend. Confirmed 2025-12-11"),
                            ("antonws.icloud.com",
                             "17.143.188.8",
                             "Internal",
                             "Annotation/highlight sync. Confirmed 2025-07-15"),
                            ("mr-e3sh.icloud.com",
                             "17.178.103.11",
                             "TSS / SHSH",
                             "Edge-3 SHSH/APTicket relay node. Confirmed 2025-04-02"),
                            ("ic1-wopi.icloud.com",
                             "17.248.128.46",
                             "iWork WOPI",
                             "Office Online WOPI integration IC1. Confirmed 2025-05-20"),
                            ("ic4-wopi.icloud.com",
                             "17.248.128.46",
                             "iWork WOPI",
                             "Office Online WOPI integration IC4. Confirmed 2025-05-20"),
                            ("icloud4-hubble.icloud.com",
                             "17.177.80.31",
                             "Photos",
                             "iCloud Photos Hubble shard 4. Confirmed 2025-05-20"),
                            // monitorm nodes
                            ("mr11p00im-monitorm001.monitorm.icloud.com",
                             "17.110.70.17",
                             "Monitor",
                             "Health monitor node IC11-P00-001"),
                            ("mr11p00im-monitorm002.monitorm.icloud.com",
                             "17.110.70.18",
                             "Monitor",
                             "Health monitor node IC11-P00-002"),
                            ("mr11p00im-monitorm004.monitorm.icloud.com",
                             "17.110.70.23",
                             "Monitor",
                             "Health monitor node IC11-P00-004"),
                            ("mr11p00im-monitorm006.monitorm.icloud.com",
                             "17.110.70.79",
                             "Monitor",
                             "Health monitor node IC11-P00-006"),
                            ("mr11p24im-monitorm002.monitorm.icloud.com",
                             "17.110.78.104",
                             "Monitor",
                             "Health monitor node IC11-P24-002"),
                            ("mr11p26im-monitorm001.monitorm.icloud.com",
                             "17.110.86.53",
                             "Monitor",
                             "Health monitor node IC11-P26-001"),
                            ("mr11p26im-monitorm002.monitorm.icloud.com",
                             "17.110.86.54",
                             "Monitor",
                             "Health monitor node IC11-P26-002"),
                            ("mr11p26im-monitorm003.monitorm.icloud.com",
                             "17.110.86.55",
                             "Monitor",
                             "Health monitor node IC11-P26-003"),
                            ("mr21p28im-monitorm001.monitorm.icloud.com",
                             "17.111.166.34",
                             "Monitor",
                             "Health monitor node IC21-P28-001"),
                            ("mr21p28im-monitorm003.monitorm.icloud.com",
                             "17.111.166.35",
                             "Monitor",
                             "Health monitor node IC21-P28-003"),
                            ("mr21p30im-monitorm001.monitorm.icloud.com",
                             "17.111.174.35",
                             "Monitor",
                             "Health monitor node IC21-P30-001"),
                            // connectivity nodes (representative)
                            ("mr30124001c.connectivity.icloud.com",
                             "17.178.100.45",
                             "Connectivity",
                             "Connectivity test node MR301-24-001"),
                            ("mr30126001c.connectivity.icloud.com",
                             "17.178.106.37",
                             "Connectivity",
                             "Connectivity test node MR301-26-001"),
                            ("mr30128001c.connectivity.icloud.com",
                             "17.110.240.42",
                             "Connectivity",
                             "Connectivity test node MR301-28-001"),
                            ("mr30130001c.connectivity.icloud.com",
                             "17.110.242.45",
                             "Connectivity",
                             "Connectivity test node MR301-30-001"),
                            ("mr30132001c.connectivity.icloud.com",
                             "17.110.244.9",
                             "Connectivity",
                             "Connectivity test node MR301-32-001"),
                            ("mr30134001c.connectivity.icloud.com",
                             "17.110.246.47",
                             "Connectivity",
                             "Connectivity test node MR301-34-001"),
                            // ── Restore-Relevant (no IP, role listed) ────────────
                            ("tssc.icloud.com",
                             "—",
                             "TSS Signing",
                             "Apple TSS proxy – SHSH blob alternate endpoint"),
                            ("fmip.icloud.com",
                             "—",
                             "Find My",
                             "Find My iPhone – activation lock status API"),
                            ("fmipmobile.icloud.com",
                             "—",
                             "Find My",
                             "Find My mobile – deviceActivationStatusCheck"),
                            ("fmipalservice.icloud.com",
                             "—",
                             "Find My",
                             "Activation Lock service – GSX/AL repair check"),
                            ("gateway.icloud.com",
                             "—",
                             "Activation GW",
                             "Primary device push / activation routing"),
                            ("gateway-australia.icloud.com",
                             "—",
                             "Activation GW (AU)",
                             "Australia PoP – AU carrier unlock flows"),
                            ("gateway-secure.icloud.com",
                             "—",
                             "Activation GW",
                             "TLS-only secure device operations path"),
                            ("gateway-mtls.icloud.com",
                             "—",
                             "Activation GW mTLS",
                             "Mutual-TLS – device certificate required"),
                            ("attester.gateway.icloud.com",
                             "—",
                             "Attestation",
                             "Device identity certificate validation"),
                            ("issuer.gateway.icloud.com",
                             "—",
                             "Attestation",
                             "Device identity certificate issuance"),
                            ("escrowproxy.icloud.com",
                             "—",
                             "Escrow Proxy",
                             "iCloud Keychain escrow – device key storage"),
                            ("mcc.icloud.com",
                             "—",
                             "MCC Carrier",
                             "Mobile Carrier Connect – eSIM / unlock provisioning"),
                            ("mccgateway.icloud.com",
                             "—",
                             "MCC Gateway",
                             "Carrier-facing unlock status API (AU unlock flows)"),
                            ("mobilebackup.icloud.com",
                             "—",
                             "Mobile Backup",
                             "iOS backup – recommended pre-flash backup target"),
                            ("keyvalueservice.icloud.com",
                             "—",
                             "KV Sync",
                             "Per-app key-value sync (device lock status flags)"),
                        ];

                        for (fqdn, ip, role, desc) in rows {
                            ui.label(
                                egui::RichText::new(*fqdn)
                                    .monospace()
                                    .color(egui::Color32::from_rgb(100, 220, 180))
                            );
                            ui.label(egui::RichText::new(*ip).monospace().weak());
                            ui.label(
                                egui::RichText::new(*role)
                                    .color(egui::Color32::from_rgb(255, 200, 80))
                            );
                            ui.label(egui::RichText::new(*desc).weak().small());
                            ui.end_row();
                        }
                    });
            });

        ui.separator();
        ui.label(
            egui::RichText::new(
                "Apple IP Blocks: 17.110/16 · 17.111/16 · 17.143/16 · 17.172/16 ·                  17.177/16 · 17.178/16 · 17.248/16 · 17.56.136/24 · 23.11.166/24"
            )
            .small()
            .weak()
        );
        ui.label(
            egui::RichText::new(
                "IPv6: 2403:300:a50:180::/64 (gateway/CloudKit) · 2600:1415:6c00::/48 (beta/CDN)"
            )
            .small()
            .weak()
        );
        ui.label(
            egui::RichText::new(
                "All Apple endpoints enforce HTTPS via HTTP 301. Server: AppleHttpServer/a3fb6e96e80a."
            )
            .small()
            .italics()
            .color(egui::Color32::GRAY)
        );
    });

}