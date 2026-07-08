// chimera-api/src/mock_server.rs
// Optional local mock server that emulates the ChimeraTool API surface.
// Useful for:
//   - Integration testing without internet
//   - Reverse-engineering API payloads by inspecting traffic
//   - Running ChimeraTool clients (not ChimeraRS) in offline mode
//
// Starts a local HTTP server on 127.0.0.1:8742 that handles all known
// api.chimeratool.com / secure.chimeratool.com / data.chimeratool.com paths.

use log::warn;
use serde_json::{json, Value};

/// Mock server configuration
pub struct MockServerConfig {
    pub bind_addr: String,   // default "127.0.0.1:8742"
    pub log_requests: bool,
    pub simulate_credits: bool,
    pub credits_per_account: u32,
}

impl Default for MockServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8742".into(),
            log_requests: true,
            simulate_credits: false, // ChimeraRS: always unlimited
            credits_per_account: u32::MAX,
        }
    }
}

/// Generate a mock JSON response for any known ChimeraTool API path
pub fn mock_response(method: &str, path: &str, body: Option<&Value>) -> Value {
    match path {
        // ── Auth ────────────────────────────────────────────────────────
        "/v2/auth/login" => json!({
            "success": true,
            "code": 200,
            "data": {
                "token": "mock-token-chimera-rs",
                "refresh_token": "mock-refresh-token",
                "user_id": 1,
                "username": "chimera-rs-local",
                "credits": u32::MAX,
                "expiry": 9999999999u64,
                "features": ["all_operations", "unlimited_credits", "no_ads"]
            },
            "credits_used": 0,
            "credits_remaining": u32::MAX
        }),
        "/v2/auth/refresh" => json!({
            "success": true, "code": 200,
            "data": { "token": "mock-token-refreshed", "expiry": 9999999999u64 }
        }),
        "/v2/user/credits" => json!({
            "success": true, "code": 200,
            "data": { "credits": u32::MAX, "plan": "unlimited", "expiry": null }
        }),

        // ── Operations ──────────────────────────────────────────────────
        "/v2/operation/dispatch" => {
            let op_type = body.and_then(|b| b["operation"].as_str()).unwrap_or("unknown");
            json!({
                "success": true, "code": 200,
                "data": {
                    "operation_id": format!("mock-op-{}", uuid::Uuid::new_v4()),
                    "status": "success",
                    "credits_deducted": 0,
                    "message": format!("Operation '{}' executed locally (no server needed)", op_type),
                    "result": {}
                },
                "credits_used": 0,
                "credits_remaining": u32::MAX
            })
        },

        // ── Secure: IMEI ────────────────────────────────────────────────
        "/v1/imei/check" => {
            let imei = body.and_then(|b| b["imei"].as_str()).unwrap_or("");
            json!({
                "success": true, "code": 200,
                "data": {
                    "imei": imei,
                    "valid": imei.len() == 15,
                    "blacklisted": false,
                    "brand": null, "carrier": null, "country": null
                }
            })
        },

        // ── Secure: NCK ─────────────────────────────────────────────────
        "/v1/nck/calculate" => {
            let imei = body.and_then(|b| b["imei"].as_str()).unwrap_or("");
            let mccmnc = body.and_then(|b| b["mccmnc"].as_str()).unwrap_or("50501");
            let brand = body.and_then(|b| b["brand"].as_str()).unwrap_or("Samsung");
            let result = crate::secure_api::calculate_nck_local(imei, mccmnc, brand);
            json!({
                "success": true, "code": 200,
                "data": {
                    "imei": imei,
                    "carrier_mccmnc": mccmnc,
                    "nck1": result.nck1,
                    "algorithm": result.algorithm,
                    "note": "Calculated locally by ChimeraRS mock server"
                }
            })
        },

        // ── Secure: FRP ticket ──────────────────────────────────────────
        "/v1/frp/ticket" => json!({
            "success": true, "code": 200,
            "data": {
                "ticket_id": format!("mock-{}", uuid::Uuid::new_v4()),
                "signed_payload": [],
                "note": "ChimeraRS performs FRP removal locally without server tickets"
            }
        }),

        // ── Data: Models ────────────────────────────────────────────────
        p if p.starts_with("/v1/models/") => {
            let brand = p.trim_start_matches("/v1/models/");
            json!({
                "success": true, "code": 200,
                "data": {
                    "brand": brand,
                    "models": ["See chimera-devices/database.rs for full local device database"],
                    "source": "chimera-devices (local)"
                }
            })
        },

        // ── Port check ──────────────────────────────────────────────────
        p if p.starts_with("/check") => json!({
            "open": false,
            "latency_ms": 0,
            "note": "Use chimera-api/portcheck.rs for local TCP port checking"
        }),

        // ── Update check ────────────────────────────────────────────────
        "/v2/update/check" => json!({
            "success": true, "code": 200,
            "data": {
                "current_version": env!("CARGO_PKG_VERSION"),
                "latest_version": env!("CARGO_PKG_VERSION"),
                "update_available": false,
                "changelog_url": "https://github.com/your-org/chimera-rs/releases"
            }
        }),

        // ── Fallback ────────────────────────────────────────────────────
        _ => {
            warn!("Mock server: unhandled path {} {}", method, path);
            json!({
                "success": false,
                "code": 404,
                "message": format!("Unknown endpoint: {} {}. See chimera-api/endpoints.rs for full map.", method, path)
            })
        }
    }
}

/// Start instructions for running the mock server
pub fn print_mock_server_instructions() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  ChimeraRS Mock Server (replaces chimeratool.com APIs)     ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║  Bind: http://127.0.0.1:8742                               ║");
    println!("║                                                            ║");
    println!("║  To route ChimeraTool client to this mock:                 ║");
    println!("║    macOS/Linux: Add to /etc/hosts:                         ║");
    println!("║      127.0.0.1  api.chimeratool.com                        ║");
    println!("║      127.0.0.1  secure.chimeratool.com                     ║");
    println!("║      127.0.0.1  data.chimeratool.com                       ║");
    println!("║    (sudo nano /private/etc/hosts  on macOS)                ║");
    println!("║                                                            ║");
    println!("║  All operations return SUCCESS with unlimited credits.     ║");
    println!("║  No authentication required.                               ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
}
