// chimera-api/src/lib.rs
// Reverse-engineered ChimeraTool API infrastructure.
//
// Based on subdomain enumeration of chimeratool.com (24 subdomains):
//   api.chimeratool.com      → Main REST API  (Cloudflare 104.18.x.x)
//   secure.chimeratool.com   → Certificate/IMEI secure ops (Cloudflare)
//   data.chimeratool.com     → Firmware metadata, device DB (Cloudflare)
//   upload.chimeratool.com   → File upload endpoint (Cloudflare Workers)
//   pics.chimeratool.com     → Device images CDN (Cloudflare)
//   portcheck.chimeratool.com→ ADB/Fastboot TCP port check (Cloudflare)
//   bb.chimeratool.com       → Binary backend / bus (Cloudflare)
//   chat.chimeratool.com     → Support WebSocket chat (Cloudflare Workers)
//   administration.chimeratool.com → Admin panel (Cloudflare)
//   stage.chimeratool.com    → Staging env (Cloudflare)
//   dev.chimeratool.com      → Dev server (88.151.102.23 – bare IP, no CDN)
//   munin.chimeratool.com    → Munin server monitoring (Cloudflare proxy)
//   mail*.chimeratool.com    → Mail infrastructure (multiple IPs)
//   adr65/fywc6/hvdt8/bgtw2  → Rotating worker/shard subdomains
//
// ChimeraRS replaces ALL remote dependencies with:
//   1. Local algorithmic equivalents  (no server needed)
//   2. Free public APIs               (ipsw.me, Samsung Open API, etc.)
//   3. Self-hostable mock server      (chimera-api/mock_server.rs)
//
// Zero logins. Zero credits. Zero phoning home.

pub mod endpoints;
pub mod client;
pub mod auth;
pub mod device_api;
pub mod firmware_api;
pub mod secure_api;
pub mod portcheck;
pub mod pics_api;
pub mod upload_api;
pub mod mock_server;
pub mod open_alternatives;

pub use client::ApiClient;
pub use endpoints::{CHIMERA_ENDPOINTS, EndpointMap};
pub use open_alternatives::OpenApiRouter;
