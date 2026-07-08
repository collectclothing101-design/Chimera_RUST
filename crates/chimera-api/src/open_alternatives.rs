// chimera-api/src/open_alternatives.rs
// The OpenApiRouter: single entry point that routes every ChimeraTool
// API call to the appropriate open/free/local alternative.
// This is the "no login, no credits" replacement layer.

use anyhow::Result;
use log::info;
use serde::{Deserialize, Serialize};

use crate::firmware_api::{search_firmware, FirmwareSearchRequest, FirmwareResult};
use crate::secure_api::{check_imei_online, calculate_nck_local, ImeiCheckResult, NckResult};
use crate::portcheck::{check_adb_tcp, PortCheckResult};

/// The open API router — routes operations to free/local implementations
pub struct OpenApiRouter;

impl OpenApiRouter {
    // ── IMEI operations ───────────────────────────────────────────────────

    /// Check IMEI validity and metadata (replaces secure.chimeratool.com /v1/imei/check)
    pub async fn check_imei(imei: &str) -> Result<ImeiCheckResult> {
        info!("OpenApiRouter: check_imei {}", imei);
        check_imei_online(imei).await
    }

    // ── NCK / Network Unlock ──────────────────────────────────────────────

    /// Calculate network unlock code (replaces secure.chimeratool.com /v1/nck/calculate)
    pub fn calculate_nck(imei: &str, mccmnc: &str, brand: &str) -> NckResult {
        info!("OpenApiRouter: calculate_nck imei={} mccmnc={} brand={}", imei, mccmnc, brand);
        calculate_nck_local(imei, mccmnc, brand)
    }

    // ── Firmware ──────────────────────────────────────────────────────────

    /// Search firmware (replaces data.chimeratool.com /v1/firmware/*)
    pub async fn search_firmware(brand: &str, model: &str, region: Option<&str>) -> Result<Vec<FirmwareResult>> {
        info!("OpenApiRouter: search_firmware brand={} model={} region={:?}", brand, model, region);
        search_firmware(&FirmwareSearchRequest {
            brand: brand.to_owned(),
            model: model.to_owned(),
            region: region.map(str::to_owned),
            android_version: None,
        }).await
    }

    // ── Port Check ────────────────────────────────────────────────────────

    /// Check ADB TCP port (replaces portcheck.chimeratool.com)
    pub fn check_adb_port(host: &str) -> PortCheckResult {
        info!("OpenApiRouter: check_adb_port host={}", host);
        check_adb_tcp(host)
    }

    // ── Auth bypass ───────────────────────────────────────────────────────

    /// Authentication — always succeeds with unlimited credits
    pub fn authenticate(_email: &str, _password: &str) -> AuthResult {
        AuthResult {
            success: true,
            token: "chimera-rs-local-no-auth".into(),
            credits: u32::MAX,
            message: "ChimeraRS: no authentication required. All operations are free and local.".into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResult {
    pub success: bool,
    pub token: String,
    pub credits: u32,
    pub message: String,
}

/// Summary of which ChimeraTool endpoints are replaced vs not
pub fn print_replacement_status() {
    use crate::endpoints::EndpointStats;
    let stats = EndpointStats::compute();
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║  ChimeraTool Endpoint Replacement Status               ║");
    println!("╠════════════════════════════════════════════════════════╣");
    println!("║  Total subdomains mapped:  {:>3}                         ║", stats.total);
    println!("║  Fully replaced (local):   {:>3}  ({:.0}%)                ║", stats.replaced, stats.replacement_pct());
    println!("║  Support/admin (N/A):      {:>3}                         ║", stats.total - stats.replaced);
    println!("║  Cloudflare CDN:           {:>3}                         ║", stats.cloudflare);
    println!("║  Cloudflare Workers:       {:>3}                         ║", stats.cloudflare_workers);
    println!("║  Bare servers:             {:>3}                         ║", stats.bare);
    println!("║  Inactive/decommissioned:  {:>3}                         ║", stats.inactive);
    println!("╚════════════════════════════════════════════════════════╝");

    println!("\n✅ REPLACED endpoints:");
    for e in crate::endpoints::replaced_endpoints() {
        println!("  • {} → {}", e.subdomain, &e.chimera_rs_replacement[..60.min(e.chimera_rs_replacement.len())]);
    }
    println!("\n❌ NOT replaced (support/admin/monitoring — not needed):");
    for e in crate::endpoints::unreplaced_endpoints() {
        println!("  • {} — {}", e.subdomain, e.purpose);
    }
}
