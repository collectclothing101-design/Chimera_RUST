// chimera-api/src/endpoints.rs
// Complete map of all known ChimeraTool subdomains/endpoints derived from
// DNS enumeration. Each entry documents the inferred purpose, protocol,
// and the ChimeraRS open replacement strategy.

use serde::Serialize;
use std::collections::HashMap;

/// A single known endpoint in the ChimeraTool infrastructure
#[derive(Debug, Clone, Serialize)]
pub struct EndpointInfo {
    /// Fully-qualified subdomain
    pub subdomain: &'static str,
    /// Resolved IP addresses (as observed)
    pub ips: &'static [&'static str],
    /// Last DNS resolution date observed
    pub last_resolved: &'static str,
    /// CDN / hosting provider
    pub provider: InfraProvider,
    /// Inferred purpose of this endpoint
    pub purpose: &'static str,
    /// Observed or inferred transport protocol
    pub protocol: &'static str,
    /// ChimeraRS open replacement approach
    pub chimera_rs_replacement: &'static str,
    /// Whether ChimeraRS has fully replaced this endpoint
    pub replaced: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum InfraProvider {
    /// Cloudflare CDN/proxy (104.18.x.x or 172.66.x.x)
    Cloudflare,
    /// Cloudflare Workers / Pages (188.114.x.x)
    CloudflareWorkers,
    /// Direct/bare server IP (not behind CDN)
    Bare,
    /// No current IP resolution
    Inactive,
}

/// Complete endpoint map for chimeratool.com (24 subdomains enumerated)
pub const CHIMERA_ENDPOINTS: &[EndpointInfo] = &[
    // ── Core API ────────────────────────────────────────────────────────────
    EndpointInfo {
        subdomain: "api.chimeratool.com",
        ips: &["104.18.14.248", "104.18.15.248"],
        last_resolved: "2025-02-17",
        provider: InfraProvider::Cloudflare,
        purpose: "Main REST API: user auth, credit management, operation dispatch, license checks",
        protocol: "HTTPS/REST JSON",
        chimera_rs_replacement: "All operations run locally. No auth or credits needed. \
                                  Credits system removed entirely.",
        replaced: true,
    },
    EndpointInfo {
        subdomain: "secure.chimeratool.com",
        ips: &["104.18.15.248", "104.18.14.248"],
        last_resolved: "2025-02-17",
        provider: InfraProvider::Cloudflare,
        purpose: "Secure server-side ops: certificate signing, IMEI server queries, \
                  license token validation, Samsung KG/FRP server",
        protocol: "HTTPS/REST + AES-encrypted payloads",
        chimera_rs_replacement: "chimera-core/crypto.rs: local AES-256 key derivation. \
                                  chimera-core/certificate.rs: local cert ops. \
                                  chimera-samsung: local Knox/FRP removal without server.",
        replaced: true,
    },
    EndpointInfo {
        subdomain: "data.chimeratool.com",
        ips: &["104.18.14.248", "104.18.15.248"],
        last_resolved: "2025-02-17",
        provider: InfraProvider::Cloudflare,
        purpose: "Firmware metadata database: model lists, firmware URLs, changelog, \
                  device support matrix updates",
        protocol: "HTTPS/REST JSON + binary manifests",
        chimera_rs_replacement: "chimera-firmware: local firmware DB. \
                                  open_alternatives: ipsw.me API, SamFW.com, samfrew.com, \
                                  firmware.mobi for Samsung. Device DB is embedded in \
                                  chimera-devices/database.rs and updated via git.",
        replaced: true,
    },
    EndpointInfo {
        subdomain: "upload.chimeratool.com",
        ips: &["188.114.99.228", "188.114.98.228"],
        last_resolved: "2025-02-17",
        provider: InfraProvider::CloudflareWorkers,
        purpose: "File upload: firmware files, crash logs, diagnostic data, \
                  certificate/EFS backup uploads to user cloud account",
        protocol: "HTTPS multipart/form-data + chunked upload",
        chimera_rs_replacement: "chimera-core/backup.rs: local backup to user filesystem. \
                                  No cloud upload. All files stay on device/local machine.",
        replaced: true,
    },
    EndpointInfo {
        subdomain: "pics.chimeratool.com",
        ips: &["104.18.14.248", "104.18.15.248"],
        last_resolved: "2025-05-20",
        provider: InfraProvider::Cloudflare,
        purpose: "Device image CDN: thumbnails and full-size photos for GUI device list, \
                  brand logos, operation icons",
        protocol: "HTTPS CDN (static assets)",
        chimera_rs_replacement: "chimera-gui/assets/: bundled brand icons via egui. \
                                  open_alternatives::pics: fallback fetch from GSMArena/FCC. \
                                  Device images cached locally in ~/.chimera-rs/cache/pics/",
        replaced: true,
    },
    EndpointInfo {
        subdomain: "portcheck.chimeratool.com",
        ips: &["104.18.14.248", "104.18.15.248"],
        last_resolved: "2025-08-06",
        provider: InfraProvider::Cloudflare,
        purpose: "External TCP port reachability check: validates ADB-over-TCP connectivity, \
                  checks if device port is reachable from internet for remote session",
        protocol: "HTTPS GET with port/host params → JSON {open: bool, latency_ms: u32}",
        chimera_rs_replacement: "chimera-api/portcheck.rs: local TCP connect attempt. \
                                  No external server needed for LAN ADB. For WAN: use \
                                  portcheck.local() or portcheck.stun() with STUN reflection.",
        replaced: true,
    },
    EndpointInfo {
        subdomain: "bb.chimeratool.com",
        ips: &["104.18.14.248", "104.18.15.248"],
        last_resolved: "2025-02-17",
        provider: InfraProvider::Cloudflare,
        purpose: "Binary Backend / Broadcast bus: suspected real-time operation status, \
                  WebSocket push notifications for long-running ops, credit deduction events",
        protocol: "WebSocket (ws/wss) over HTTPS upgrade",
        chimera_rs_replacement: "chimera-core/event.rs: local EventBus (crossbeam-channel). \
                                  All progress events are in-process. No remote bus needed.",
        replaced: true,
    },
    EndpointInfo {
        subdomain: "chat.chimeratool.com",
        ips: &["188.114.99.228", "188.114.98.228"],
        last_resolved: "2025-02-17",
        provider: InfraProvider::CloudflareWorkers,
        purpose: "Live customer support chat (Cloudflare Workers + Durable Objects)",
        protocol: "WebSocket / Cloudflare Durable Objects",
        chimera_rs_replacement: "Not replicated (support channel). \
                                  ChimeraRS users: GitHub Issues, community forum.",
        replaced: false,
    },
    EndpointInfo {
        subdomain: "administration.chimeratool.com",
        ips: &["104.18.15.248", "104.18.14.248"],
        last_resolved: "2025-02-17",
        provider: InfraProvider::Cloudflare,
        purpose: "Internal admin panel: user management, credit topups, device whitelist, \
                  feature flag toggles, operation audit logs",
        protocol: "HTTPS web app (likely React + REST)",
        chimera_rs_replacement: "N/A – ChimeraRS has no admin/server component. \
                                  All configuration is local via settings_panel.rs.",
        replaced: false,
    },
    EndpointInfo {
        subdomain: "munin.chimeratool.com",
        ips: &["104.18.14.248", "104.18.15.248"],
        last_resolved: "2025-02-17",
        provider: InfraProvider::Cloudflare,
        purpose: "Munin network/system monitoring for ChimeraTool server infrastructure",
        protocol: "HTTP (Munin CGI web interface)",
        chimera_rs_replacement: "N/A – server-side monitoring. ChimeraRS: no servers.",
        replaced: false,
    },
    EndpointInfo {
        subdomain: "stage.chimeratool.com",
        ips: &["104.18.14.248", "104.18.15.248"],
        last_resolved: "2025-02-17",
        provider: InfraProvider::Cloudflare,
        purpose: "Staging environment mirroring production API for pre-release testing",
        protocol: "HTTPS (same as api.chimeratool.com)",
        chimera_rs_replacement: "N/A – ChimeraRS uses cargo test + CI for testing.",
        replaced: false,
    },
    EndpointInfo {
        subdomain: "dev.chimeratool.com",
        ips: &["88.151.102.23"],
        last_resolved: "2025-02-17",
        provider: InfraProvider::Bare,
        purpose: "Development server — bare IP (not Cloudflare). Likely internal developer API, \
                  debug endpoints, possibly exposes raw API without rate limiting",
        protocol: "HTTP/HTTPS on non-standard port or standard",
        chimera_rs_replacement: "Useful for API structure discovery. \
                                  See chimera-api/device_api.rs for inferred request shapes.",
        replaced: false,
    },
    // ── Mail infrastructure ──────────────────────────────────────────────────
    EndpointInfo {
        subdomain: "mail.chimeratool.com",
        ips: &["78.24.184.253"],
        last_resolved: "2025-05-20",
        provider: InfraProvider::Bare,
        purpose: "Primary mail server (SMTP/IMAP for transactional email)",
        protocol: "SMTP/IMAP",
        chimera_rs_replacement: "N/A – ChimeraRS sends no email.",
        replaced: false,
    },
    EndpointInfo {
        subdomain: "mail2.chimeratool.com",
        ips: &["88.151.101.136"],
        last_resolved: "2025-03-26",
        provider: InfraProvider::Bare,
        purpose: "Mail server 2 (redundancy/backup MX)",
        protocol: "SMTP",
        chimera_rs_replacement: "N/A",
        replaced: false,
    },
    EndpointInfo {
        subdomain: "mail3.chimeratool.com",
        ips: &["37.48.87.204"],
        last_resolved: "2025-03-26",
        provider: InfraProvider::Bare,
        purpose: "Mail server 3",
        protocol: "SMTP",
        chimera_rs_replacement: "N/A",
        replaced: false,
    },
    EndpointInfo {
        subdomain: "mailx.chimeratool.com",
        ips: &["37.48.87.204", "81.17.60.229", "108.62.121.121"],
        last_resolved: "2025-02-17",
        provider: InfraProvider::Bare,
        purpose: "Mail relay / outbound SMTP relay (multiple A records = round-robin relay)",
        protocol: "SMTP relay",
        chimera_rs_replacement: "N/A",
        replaced: false,
    },
    // ── Rotating shard / worker subdomains ──────────────────────────────────
    EndpointInfo {
        subdomain: "adr65.chimeratool.com",
        ips: &["188.114.98.228", "188.114.99.228"],
        last_resolved: "2025-02-13",
        provider: InfraProvider::CloudflareWorkers,
        purpose: "Worker shard or address-routing endpoint. \
                  'adr' prefix suggests address/routing. \
                  Likely: geographic routing shard or A/B test bucket for API traffic",
        protocol: "HTTPS (Cloudflare Worker)",
        chimera_rs_replacement: "Not needed. Routing is irrelevant in local operation.",
        replaced: false,
    },
    EndpointInfo {
        subdomain: "fywc6.chimeratool.com",
        ips: &["188.114.98.228", "188.114.99.228"],
        last_resolved: "2025-02-13",
        provider: InfraProvider::CloudflareWorkers,
        purpose: "Obfuscated Cloudflare Worker endpoint — likely a versioned binary delivery \
                  endpoint or encrypted callback URL embedded in ChimeraTool binary",
        protocol: "HTTPS (Cloudflare Worker)",
        chimera_rs_replacement: "Identified as likely firmware binary download shard. \
                                  Replaced by chimera-firmware/downloader.rs with open firmware sources.",
        replaced: true,
    },
    // ── Inactive / no current IP ─────────────────────────────────────────────
    EndpointInfo {
        subdomain: "bgtw2.chimeratool.com",
        ips: &[],
        last_resolved: "N/A",
        provider: InfraProvider::Inactive,
        purpose: "Background gateway 2 — decommissioned or reserved. \
                  'bgtw' = background gateway. Likely a previous billing/gateway server.",
        protocol: "Unknown (decommissioned)",
        chimera_rs_replacement: "Was likely the billing/credit-deduction gateway. \
                                  Fully removed in ChimeraRS — no billing required.",
        replaced: true,
    },
    EndpointInfo {
        subdomain: "blog.chimeratool.com",
        ips: &[],
        last_resolved: "N/A",
        provider: InfraProvider::Inactive,
        purpose: "ChimeraTool blog (currently inactive / DNS removed)",
        protocol: "HTTPS (WordPress/Ghost blog)",
        chimera_rs_replacement: "ChimeraRS changelog in CHANGELOG.md and GitHub releases.",
        replaced: false,
    },
    EndpointInfo {
        subdomain: "hvdt8.chimeratool.com",
        ips: &[],
        last_resolved: "N/A",
        provider: InfraProvider::Inactive,
        purpose: "Obfuscated inactive shard. Likely a previously active Worker endpoint \
                  (same pattern as fywc6/adr65). Could be a canary/health-check shard.",
        protocol: "Unknown (decommissioned)",
        chimera_rs_replacement: "N/A – inactive endpoint.",
        replaced: false,
    },
    EndpointInfo {
        subdomain: "www.chimeratool.com",
        ips: &["172.66.130.194", "172.66.130.193"],
        last_resolved: "2025-08-06",
        provider: InfraProvider::Cloudflare,
        purpose: "Main marketing website and web application front-end",
        protocol: "HTTPS (web app)",
        chimera_rs_replacement: "ChimeraRS ships as a native desktop app. No web dependency.",
        replaced: false,
    },
];

/// Look up an endpoint by subdomain
pub fn lookup_endpoint(subdomain: &str) -> Option<&'static EndpointInfo> {
    CHIMERA_ENDPOINTS.iter().find(|e| e.subdomain == subdomain)
}

/// Get all endpoints that ChimeraRS has successfully replaced
pub fn replaced_endpoints() -> Vec<&'static EndpointInfo> {
    CHIMERA_ENDPOINTS.iter().filter(|e| e.replaced).collect()
}

/// Get all endpoints still not replaced (support/admin/monitoring)
pub fn unreplaced_endpoints() -> Vec<&'static EndpointInfo> {
    CHIMERA_ENDPOINTS.iter().filter(|e| !e.replaced).collect()
}

/// Summary stats
pub struct EndpointStats {
    pub total: usize,
    pub replaced: usize,
    pub cloudflare: usize,
    pub cloudflare_workers: usize,
    pub bare: usize,
    pub inactive: usize,
}

impl EndpointStats {
    pub fn compute() -> Self {
        let total = CHIMERA_ENDPOINTS.len();
        let replaced = CHIMERA_ENDPOINTS.iter().filter(|e| e.replaced).count();
        let cloudflare = CHIMERA_ENDPOINTS.iter().filter(|e| e.provider == InfraProvider::Cloudflare).count();
        let cloudflare_workers = CHIMERA_ENDPOINTS.iter().filter(|e| e.provider == InfraProvider::CloudflareWorkers).count();
        let bare = CHIMERA_ENDPOINTS.iter().filter(|e| e.provider == InfraProvider::Bare).count();
        let inactive = CHIMERA_ENDPOINTS.iter().filter(|e| e.provider == InfraProvider::Inactive).count();
        Self { total, replaced, cloudflare, cloudflare_workers, bare, inactive }
    }

    pub fn replacement_pct(&self) -> f32 {
        if self.total == 0 { 0.0 } else { self.replaced as f32 / self.total as f32 * 100.0 }
    }
}

pub type EndpointMap = HashMap<&'static str, &'static EndpointInfo>;
