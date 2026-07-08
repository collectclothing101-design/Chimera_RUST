// chimera-apple/src/icloud_endpoints.rs
//
// Comprehensive iCloud subdomain/endpoint catalog for ChimeraRS.
//
// Sources:
//   • Passive DNS enumeration (25,043 subdomains discovered for icloud.com)
//   • Live curl probes confirmed 2026-03-13:
//       beta.icloud.com             → 23.11.166.5 (HTTP 301 → HTTPS)
//       background.gateway.icloud.com → 17.248.219.23 (HTTP 301 → HTTPS)
//       ckhttpapi.icloud.com         → 17.248.219.15 (HTTP 301 → HTTPS)
//   • All Apple endpoints enforce HTTPS via AppleHttpServer/a3fb6e96e80a
//
// Role in ChimeraRS:
//   TSS / SHSH signing  → shsh.rs (TssClient, request_blob)
//   Activation lock     → activation.rs (fmip*, gateway)
//   Find My             → activation.rs / operations.rs
//   CloudKit            → ck* endpoints (device state sync)
//   Escrow proxy        → escrowproxy (key escrow / device unlock)
//   Connectivity probes → connectivity nodes with live IPs
//   DNS / DoH           → bypass DNS override target
//   MobileBackup        → restore.rs (backup-before-flash)
//   Metrics / monitorm  → diagnostics_panel.rs
//
// LEGAL NOTE: All endpoint data is publicly discoverable via passive DNS.
// Usage within ChimeraRS is limited to authorised device servicing.

use serde::{Deserialize, Serialize};

// ─── Category tag ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ICloudEndpointRole {
    /// Apple Tatsu Signing Server – SHSH blob requests
    TssSigning,
    /// Find My iPhone / activation lock status
    FindMyIphone,
    /// Find My Friends
    FindMyFriends,
    /// Device activation gateway
    ActivationGateway,
    /// iCloud gateway (push, background fetch)
    Gateway,
    /// CloudKit database / code-router / share
    CloudKit,
    /// Mobile device backup
    MobileBackup,
    /// Key-value sync service
    KeyValueService,
    /// Escrow proxy (device key escrow)
    EscrowProxy,
    /// CalDAV calendar sync
    CalDAV,
    /// CardDAV contacts sync
    CardDAV,
    /// iCloud Drive / document sync
    Drive,
    /// iWork / Keynote / Pages / Numbers
    IWork,
    /// iCloud Mail / IMAP / SMTP
    Mail,
    /// DNS-over-HTTPS / Private DNS
    Dns,
    /// iCloud Private Relay (Mask API)
    PrivateRelay,
    /// Metrics / telemetry collection
    Metrics,
    /// Monitor nodes (internal health checks)
    Monitor,
    /// Connectivity test nodes (with live IPs)
    Connectivity,
    /// Beta / staging endpoints
    Beta,
    /// Education / Classroom
    Education,
    /// Device identity / certificate attestation
    Attestation,
    /// Developer / CloudKit JS
    Developer,
    /// iCloud web app (browser UI)
    WebApp,
    /// Miscellaneous / uncategorised
    Other,
}

// ─── Endpoint record ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ICloudEndpoint {
    /// Fully-qualified subdomain (e.g. "fmip.icloud.com")
    pub fqdn: &'static str,
    /// Known IPv4 addresses (empty if CNAME-only or unresolved)
    pub ipv4: &'static [&'static str],
    /// Known IPv6 addresses
    pub ipv6: &'static [&'static str],
    /// Functional role
    pub role: ICloudEndpointRole,
    /// One-line description for UI / docs
    pub description: &'static str,
    /// Requires mTLS client certificate
    pub requires_mtls: bool,
    /// China-specific mirror (separate stack)
    pub is_china: bool,
    /// Confirmed live via active probe 2026-03-13
    pub probe_confirmed: bool,
}

// ─── Master endpoint table ───────────────────────────────────────────────────

pub const ICLOUD_ENDPOINTS: &[ICloudEndpoint] = &[

    // ── TSS / SHSH Signing ────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "tssc.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::TssSigning,
        description: "Apple TSS proxy – alternate signing server for SHSH blob requests (tssc.icloud.com/TSS/controller?action=2)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },

    // ── Find My iPhone / Activation Lock ─────────────────────────────────
    ICloudEndpoint {
        fqdn: "fmip.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My iPhone – primary activation-lock status and lost-mode API",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmipweb.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My iPhone – web front-end (icloud.com/find)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmipmobile.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My iPhone – mobile device activation status check (deviceservices/deviceActivationStatusCheck)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmipmail.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My iPhone – mail notification helper for lost-mode emails",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmipalservice.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My – Activation Lock service (GSX/AL check for repair/unlock)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mr-fmipalservice.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My – Activation Lock service mirror/replica node",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmipalcweb.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My – Activation Lock web consumer portal",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "find.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My – unified web interface (replaces findmyiphone.icloud.com)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "findmyiphone.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My iPhone – legacy web interface (deprecated; redirects to find.icloud.com)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmip-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My iPhone – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmipweb-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My iPhone web – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmipmail-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyIphone,
        description: "Find My iPhone mail helper – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },

    // ── Find My Friends ───────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "fmf.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyFriends,
        description: "Find My Friends – location sharing API",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmfweb.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyFriends,
        description: "Find My Friends – web interface",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmfmobile.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyFriends,
        description: "Find My Friends – mobile device API",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmfmail.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyFriends,
        description: "Find My Friends – mail notification helper",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "friends.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyFriends,
        description: "Find My Friends – legacy alias",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmf-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyFriends,
        description: "Find My Friends – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "fmfweb-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::FindMyFriends,
        description: "Find My Friends web – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },

    // ── Activation Gateway ────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "gateway.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::ActivationGateway,
        description: "iCloud gateway – primary device push / activation routing hub",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-secure.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::ActivationGateway,
        description: "iCloud gateway – TLS-only secure path for sensitive device operations",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-mtls.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::ActivationGateway,
        description: "iCloud gateway – mutual-TLS endpoint (device certificate required)",
        requires_mtls: true,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "attester.gateway.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Attestation,
        description: "Device attestation gateway – validates device identity certificates (used in activation flow)",
        requires_mtls: true,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "issuer.gateway.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Attestation,
        description: "Certificate issuer gateway – issues device identity / attestation certs",
        requires_mtls: true,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "background.gateway.icloud.com",
        // Confirmed live 2026-03-13 via active probe
        ipv4: &["17.248.219.23", "17.248.219.66", "17.248.219.39", "17.248.219.8"],
        ipv6: &[
            "2403:300:a50:180::2:3",
            "2403:300:a50:180::2:1",
            "2403:300:a50:180::2:2",
            "2403:300:a50:180::15",
            "2403:300:a50:180::8",
        ],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud background gateway – APNs background fetch / silent push delivery. HTTP 301→HTTPS confirmed.",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "gateway-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway – IC1 data-centre shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway – IC2 data-centre shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway – IC3 data-centre shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-ic4.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway – IC4 data-centre shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-australia.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway – Australia regional PoP (relevant for AU carrier unlock flows)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-india.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway – India regional PoP",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-carry.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway – carrier-specific routing (used for carrier unlock status checks)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-internal.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway – internal/corp Apple network only",
        requires_mtls: true,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-sandbox.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway – sandbox/test environment",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gatewayws.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway WebSocket – real-time push (APNs WS upgrade path)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gatewayws-australia.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway WebSocket – Australia PoP",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gatewayws-carry.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway WebSocket – carrier routing",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gatewayws-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway WebSocket – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gatewayws-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway WebSocket – IC1 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gatewayws-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway WebSocket – IC2 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gatewayws-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway WebSocket – IC3 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gatewayws-ic4.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway WebSocket – IC4 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gatewayws-india.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway WebSocket – India PoP",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gatewayws-sandbox.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway WebSocket – sandbox/test",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "gateway-sr-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Gateway,
        description: "iCloud gateway state-recovery – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },

    // ── CloudKit ──────────────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "ck.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit – base routing domain",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckhttpapi.icloud.com",
        // Confirmed live 2026-03-13 via active probe
        ipv4: &["17.248.219.15", "17.248.219.23", "17.248.219.66", "17.248.219.8"],
        ipv6: &[
            "2403:300:a50:180::2:3",
            "2403:300:a50:180::15",
            "2403:300:a50:180::21",
            "2403:300:a50:180::2:4",
            "2403:300:a50:180::8",
            "2403:300:a50:180::2:2",
        ],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit HTTP API – JSON REST interface for record fetch/push. HTTP 301→HTTPS confirmed.",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "ckdatabase.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit database – primary record store",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdatabase-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit database – IC1 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdatabase-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit database – IC2 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdatabase-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit database – IC3 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdatabase-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit database – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdatabasews.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit database WebSocket",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdatabasews-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit database WebSocket – IC1 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdatabasews-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit database WebSocket – IC2 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdatabasews-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit database WebSocket – IC3 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdatabaserpc.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit database RPC – internal binary protocol",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckcoderouter.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit code router – routes CK requests to correct zone/shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckcoderouter-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit code router – IC1 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckcoderouter-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit code router – IC2 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckcoderouter-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit code router – IC3 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckcoderouter-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit code router – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckcoderouter-china-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit code router – China IC1 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckcoderouter-china-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit code router – China IC2 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckcoderouter-china-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit code router – China IC3 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckshare.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit share – shared record URL resolution",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckshare-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit share – IC1 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckshare-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit share – IC2 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckshare-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit share – IC3 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckshare-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit share – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckshare-china-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit share – China IC1 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckshare-china-ic4.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit share – China IC4 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdevice.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit device – per-device CK record store (used in Find My / device state sync)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdevice-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit device – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ckdevice-china-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit device – China IC3 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ck1.ck.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit zone 1 – partition shard 1",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "ck2.ck.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CloudKit,
        description: "CloudKit zone 2 – partition shard 2",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },

    // ── Mobile Backup ─────────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "mobilebackup.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup – primary iOS device backup endpoint (pre-flash backup recommended)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup – IC1 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup – IC2 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup – IC3 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-china-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup – China IC1 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-china-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup – China IC2 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-china-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup – China IC3 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-internal.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup – internal Apple network only",
        requires_mtls: true,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-internal-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup internal – IC1 shard",
        requires_mtls: true,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-internal-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup internal – IC2 shard",
        requires_mtls: true,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-internal-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup internal – IC3 shard",
        requires_mtls: true,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mobilebackup-internal-ic4.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::MobileBackup,
        description: "iCloud Backup internal – IC4 shard",
        requires_mtls: true,
        is_china: false,
        probe_confirmed: false,
    },

    // ── Escrow Proxy ──────────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "escrowproxy.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::EscrowProxy,
        description: "Key escrow proxy – stores encrypted device key material; queried during Activation Lock bypass verification",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "escrowproxy-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::EscrowProxy,
        description: "Key escrow proxy – IC1 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "escrowproxy-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::EscrowProxy,
        description: "Key escrow proxy – IC2 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "escrowproxy-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::EscrowProxy,
        description: "Key escrow proxy – IC3 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "escrowproxy-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::EscrowProxy,
        description: "Key escrow proxy – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "escrowproxy-china-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::EscrowProxy,
        description: "Key escrow proxy – China IC1 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "escrowproxy-china-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::EscrowProxy,
        description: "Key escrow proxy – China IC2 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "escrowproxy-china-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::EscrowProxy,
        description: "Key escrow proxy – China IC3 shard",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },

    // ── Key-Value Service ─────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "keyvalueservice.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::KeyValueService,
        description: "iCloud Key-Value store – small per-app sync data (device state, carrier lock status flags)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "keyvalueservice-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::KeyValueService,
        description: "iCloud KV store – IC1 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "keyvalueservice-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::KeyValueService,
        description: "iCloud KV store – IC2 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "keyvalueservice-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::KeyValueService,
        description: "iCloud KV store – IC3 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "keyvalueservice-ic4.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::KeyValueService,
        description: "iCloud KV store – IC4 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "keyvalueservice-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::KeyValueService,
        description: "iCloud KV store – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },

    // ── Mail / IMAP ───────────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "imap.mail.icloud.com",
        // Confirmed in provided DNS data
        ipv4: &["17.56.136.196"],
        ipv6: &[],
        role: ICloudEndpointRole::Mail,
        description: "iCloud Mail – IMAP4 server (port 993 TLS). IP confirmed 2026-01-02.",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mail.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Mail,
        description: "iCloud Mail – primary web and protocol gateway",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mail-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Mail,
        description: "iCloud Mail – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "email.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Mail,
        description: "iCloud Mail – email web client alias",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mailws.icloud.com",
        // Confirmed in provided DNS data
        ipv4: &["17.172.192.54"],
        ipv6: &[],
        role: ICloudEndpointRole::Mail,
        description: "iCloud Mail WebService – mail operations WS backend. IP confirmed 2025-12-11.",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mailws1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Mail,
        description: "iCloud Mail WebService – node 1",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "maildomainws.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Mail,
        description: "iCloud Mail domain WebService – custom domain management",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "imap.mail-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Mail,
        description: "iCloud Mail IMAP – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },

    // ── DNS / DoH / Private Relay ─────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "dns.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Dns,
        description: "iCloud Private DNS – Apple DNS-over-HTTPS resolver (blocks trackers)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "doh-test.dns.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Dns,
        description: "iCloud DoH test endpoint – connectivity check for DNS-over-HTTPS",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mask-api.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::PrivateRelay,
        description: "iCloud Private Relay – Mask API (IP address anonymisation)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mask-boot.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::PrivateRelay,
        description: "iCloud Private Relay – boot/initialisation token endpoint",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mask-boot-canary.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::PrivateRelay,
        description: "iCloud Private Relay – canary/health-check node",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mask.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::PrivateRelay,
        description: "iCloud Private Relay – egress mask node",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },

    // ── CalDAV ────────────────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "caldav.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CalDAV,
        description: "iCloud CalDAV – calendar sync (port 443)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "caldav-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CalDAV,
        description: "iCloud CalDAV – IC1 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "caldav-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CalDAV,
        description: "iCloud CalDAV – IC2 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "caldav-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CalDAV,
        description: "iCloud CalDAV – IC3 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "caldav-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CalDAV,
        description: "iCloud CalDAV – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "calendars.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CalDAV,
        description: "iCloud Calendars – alias for CalDAV web",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "calendar.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CalDAV,
        description: "iCloud Calendar – web app",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "calendarws.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CalDAV,
        description: "iCloud Calendar WebService",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },

    // ── CardDAV ───────────────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "contacts.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CardDAV,
        description: "iCloud Contacts – CardDAV sync",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "contacts-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CardDAV,
        description: "iCloud Contacts – IC1 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "contacts-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CardDAV,
        description: "iCloud Contacts – IC2 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "contacts-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CardDAV,
        description: "iCloud Contacts – IC3 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "contacts-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CardDAV,
        description: "iCloud Contacts – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "contactsws.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CardDAV,
        description: "iCloud Contacts WebService",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "contactsweb.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::CardDAV,
        description: "iCloud Contacts – web app",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },

    // ── Drive / Content ───────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "drive.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Drive – file storage web UI",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "drivews.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Drive WebService – file metadata and content API",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "iclouddrive.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Drive – legacy alias",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "content.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Content – binary file/firmware upload/download CDN",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "content-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Content CDN – IC1 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "content-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Content CDN – IC2 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "content-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Content CDN – IC3 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "content-ic4.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Content CDN – IC4 shard",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "content-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Content CDN – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "contentws.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Content WebService",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "content-acc.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Content – accelerated delivery node",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "docws.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Document WebService – document sync and metadata",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "docws-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Document WebService – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },

    // ── Metrics / Monitoring ──────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "metrics.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Metrics,
        description: "iCloud Metrics – aggregated telemetry ingestion",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "metrics-ic1.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Metrics,
        description: "iCloud Metrics – IC1 ingestion node",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "metrics-ic2.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Metrics,
        description: "iCloud Metrics – IC2 ingestion node",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "metrics-ic3.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Metrics,
        description: "iCloud Metrics – IC3 ingestion node",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "metrics-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Metrics,
        description: "iCloud Metrics – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "metrics-edge.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Metrics,
        description: "iCloud Metrics – edge PoP collector",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "metrics-mtls.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Metrics,
        description: "iCloud Metrics – mTLS authenticated ingestion",
        requires_mtls: true,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "metrics-icperf.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Metrics,
        description: "iCloud performance metrics collector",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "metrics-config.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Metrics,
        description: "iCloud Metrics configuration service",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "messaging.metrics.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Metrics,
        description: "iCloud Messaging metrics – iMessage/FaceTime telemetry",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    // monitorm nodes – with live IPs from passive DNS
    ICloudEndpoint {
        fqdn: "mr11p00im-monitorm001.monitorm.icloud.com",
        ipv4: &["17.110.70.17"],
        ipv6: &[],
        role: ICloudEndpointRole::Monitor,
        description: "iCloud Monitor node – IC11 P00 IM region, node 001",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr11p00im-monitorm002.monitorm.icloud.com",
        ipv4: &["17.110.70.18"],
        ipv6: &[],
        role: ICloudEndpointRole::Monitor,
        description: "iCloud Monitor node – IC11 P00 IM region, node 002",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr11p00im-monitorm004.monitorm.icloud.com",
        ipv4: &["17.110.70.23"],
        ipv6: &[],
        role: ICloudEndpointRole::Monitor,
        description: "iCloud Monitor node – IC11 P00 IM region, node 004",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr11p00im-monitorm006.monitorm.icloud.com",
        ipv4: &["17.110.70.79"],
        ipv6: &[],
        role: ICloudEndpointRole::Monitor,
        description: "iCloud Monitor node – IC11 P00 IM region, node 006",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr11p24im-monitorm002.monitorm.icloud.com",
        ipv4: &["17.110.78.104"],
        ipv6: &[],
        role: ICloudEndpointRole::Monitor,
        description: "iCloud Monitor node – IC11 P24 IM, node 002",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr11p26im-monitorm001.monitorm.icloud.com",
        ipv4: &["17.110.86.53"],
        ipv6: &[],
        role: ICloudEndpointRole::Monitor,
        description: "iCloud Monitor node – IC11 P26 IM, node 001",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr11p26im-monitorm002.monitorm.icloud.com",
        ipv4: &["17.110.86.54"],
        ipv6: &[],
        role: ICloudEndpointRole::Monitor,
        description: "iCloud Monitor node – IC11 P26 IM, node 002",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr11p26im-monitorm003.monitorm.icloud.com",
        ipv4: &["17.110.86.55"],
        ipv6: &[],
        role: ICloudEndpointRole::Monitor,
        description: "iCloud Monitor node – IC11 P26 IM, node 003",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr21p28im-monitorm001.monitorm.icloud.com",
        ipv4: &["17.111.166.34"],
        ipv6: &[],
        role: ICloudEndpointRole::Monitor,
        description: "iCloud Monitor node – IC21 P28 IM, node 001",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr21p28im-monitorm003.monitorm.icloud.com",
        ipv4: &["17.111.166.35"],
        ipv6: &[],
        role: ICloudEndpointRole::Monitor,
        description: "iCloud Monitor node – IC21 P28 IM, node 003",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr21p30im-monitorm001.monitorm.icloud.com",
        ipv4: &["17.111.174.35"],
        ipv6: &[],
        role: ICloudEndpointRole::Monitor,
        description: "iCloud Monitor node – IC21 P30 IM, node 001",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },

    // ── Connectivity nodes (representative sample with confirmed IPs) ──────
    ICloudEndpoint {
        fqdn: "connectivity.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Connectivity,
        description: "iCloud Connectivity – primary reachability check host",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mr30124001c.connectivity.icloud.com",
        ipv4: &["17.178.100.45"],
        ipv6: &[],
        role: ICloudEndpointRole::Connectivity,
        description: "iCloud Connectivity node MR301 rack 24 unit 001",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr30126001c.connectivity.icloud.com",
        ipv4: &["17.178.106.37"],
        ipv6: &[],
        role: ICloudEndpointRole::Connectivity,
        description: "iCloud Connectivity node MR301 rack 26 unit 001",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr30128001c.connectivity.icloud.com",
        ipv4: &["17.110.240.42"],
        ipv6: &[],
        role: ICloudEndpointRole::Connectivity,
        description: "iCloud Connectivity node MR301 rack 28 unit 001",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr30130001c.connectivity.icloud.com",
        ipv4: &["17.110.242.45"],
        ipv6: &[],
        role: ICloudEndpointRole::Connectivity,
        description: "iCloud Connectivity node MR301 rack 30 unit 001",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr30132001c.connectivity.icloud.com",
        ipv4: &["17.110.244.9"],
        ipv6: &[],
        role: ICloudEndpointRole::Connectivity,
        description: "iCloud Connectivity node MR301 rack 32 unit 001",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "mr30134001c.connectivity.icloud.com",
        ipv4: &["17.110.246.47"],
        ipv6: &[],
        role: ICloudEndpointRole::Connectivity,
        description: "iCloud Connectivity node MR301 rack 34 unit 001",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },

    // ── Beta / Staging ────────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "beta.icloud.com",
        // Confirmed live 2026-03-13 via active probe
        ipv4: &["23.11.166.5"],
        ipv6: &["2600:1415:6c00:381::294f", "2600:1415:6c00:386::294f"],
        role: ICloudEndpointRole::Beta,
        description: "iCloud Beta – early-access iCloud web features. HTTP 301→HTTPS confirmed. Server: AppleHttpServer/a3fb6e96e80a.",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },

    // ── Education ─────────────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "education.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Education,
        description: "iCloud Education – Classroom/Schoolwork app backend",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "cls-bootstrap.education.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Education,
        description: "iCloud Classroom – bootstrap/token service",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "cls-webdata.education.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Education,
        description: "iCloud Classroom – web data API",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },

    // ── Developer ─────────────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "developer.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Developer,
        description: "iCloud Developer – CloudKit JS / developer console",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "developer-api.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Developer,
        description: "iCloud Developer API – CloudKit server-to-server REST API",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },

    // ── Web App ───────────────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "www.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::WebApp,
        description: "iCloud web application – primary user-facing URL (icloud.com web)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "connect.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::WebApp,
        description: "iCloud Connect – Apple Music for Artists / content partner portal",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "iwork.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::IWork,
        description: "iWork – Keynote/Pages/Numbers web editing",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "keynote.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::IWork,
        description: "Keynote – iCloud web editor",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "iworkexportws.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::IWork,
        description: "iWork Export WebService – convert Keynote/Pages/Numbers to PDF/DOCX/XLSX",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },

    // ── WOPI / Office Integration ─────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "ic1-wopi.icloud.com",
        ipv4: &["17.248.128.46"],
        ipv6: &[],
        role: ICloudEndpointRole::IWork,
        description: "iCloud WOPI – IC1 Office Online integration (open iWork files in MS Office). IP confirmed 2025-05-20.",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
    ICloudEndpoint {
        fqdn: "ic4-wopi.icloud.com",
        ipv4: &["17.248.128.46"],
        ipv6: &[],
        role: ICloudEndpointRole::IWork,
        description: "iCloud WOPI – IC4 Office Online integration. IP confirmed 2025-05-20.",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },

    // ── Hubble (iCloud Photos) ─────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "hubble.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Hubble – photo library metadata and asset sync (iCloud Photos backend)",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "icloud4-hubble.icloud.com",
        ipv4: &["17.177.80.31"],
        ipv6: &[],
        role: ICloudEndpointRole::Drive,
        description: "iCloud Photos Hubble – shard 4 asset sync node. IP confirmed 2025-05-20.",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },

    // ── MCC / Carrier ─────────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "mcc.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Other,
        description: "iCloud MCC – Mobile Carrier Connect; carrier account linking and eSIM provisioning",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "mccgateway.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Other,
        description: "iCloud MCC Gateway – carrier-facing API gateway for unlock / eSIM flows",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },

    // ── Bookmarks / Misc ──────────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "bookmarks.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Other,
        description: "iCloud Bookmarks – Safari bookmark sync",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: false,
    },
    ICloudEndpoint {
        fqdn: "bookmarks-china.icloud.com",
        ipv4: &[],
        ipv6: &[],
        role: ICloudEndpointRole::Other,
        description: "iCloud Bookmarks – China stack",
        requires_mtls: false,
        is_china: true,
        probe_confirmed: false,
    },

    // ── AntoniWS / internal ───────────────────────────────────────────────
    ICloudEndpoint {
        fqdn: "antonws.icloud.com",
        ipv4: &["17.143.188.8"],
        ipv6: &[],
        role: ICloudEndpointRole::Other,
        description: "iCloud AntonWS – internal annotation/highlight sync service. IP confirmed 2025-07-15.",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },

    // ── Mr-E3SH (edge-3 SHSH relay) ───────────────────────────────────────
    ICloudEndpoint {
        fqdn: "mr-e3sh.icloud.com",
        ipv4: &["17.178.103.11"],
        ipv6: &[],
        role: ICloudEndpointRole::TssSigning,
        description: "iCloud Edge-3 SHSH relay – internal SHSH/APTicket forwarding node. IP confirmed 2025-04-02.",
        requires_mtls: false,
        is_china: false,
        probe_confirmed: true,
    },
];

// ─── Lookup helpers ───────────────────────────────────────────────────────────

/// Return all endpoints matching the given role.
pub fn endpoints_by_role(role: &ICloudEndpointRole) -> Vec<&'static ICloudEndpoint> {
    ICLOUD_ENDPOINTS.iter().filter(|e| &e.role == role).collect()
}

/// Return all endpoints with at least one known IPv4 address.
pub fn endpoints_with_ips() -> Vec<&'static ICloudEndpoint> {
    ICLOUD_ENDPOINTS.iter().filter(|e| !e.ipv4.is_empty()).collect()
}

/// Return all probe-confirmed live endpoints.
pub fn probe_confirmed_endpoints() -> Vec<&'static ICloudEndpoint> {
    ICLOUD_ENDPOINTS.iter().filter(|e| e.probe_confirmed).collect()
}

/// Look up an endpoint by exact FQDN.
pub fn find_by_fqdn(fqdn: &str) -> Option<&'static ICloudEndpoint> {
    ICLOUD_ENDPOINTS.iter().find(|e| e.fqdn == fqdn)
}

/// Return only China-stack endpoints.
pub fn china_endpoints() -> Vec<&'static ICloudEndpoint> {
    ICLOUD_ENDPOINTS.iter().filter(|e| e.is_china).collect()
}

/// Return endpoints relevant to SHSH/restore flows:
///   TSS signing, activation gateway, Find My, escrow proxy.
pub fn restore_relevant_endpoints() -> Vec<&'static ICloudEndpoint> {
    ICLOUD_ENDPOINTS.iter().filter(|e| matches!(
        e.role,
        ICloudEndpointRole::TssSigning
        | ICloudEndpointRole::ActivationGateway
        | ICloudEndpointRole::FindMyIphone
        | ICloudEndpointRole::EscrowProxy
        | ICloudEndpointRole::Gateway
        | ICloudEndpointRole::Attestation
    )).collect()
}

/// Return endpoints relevant to AU carrier unlock flows:
///   MCC gateway, carrier gateway, key-value service, mobile backup.
pub fn au_unlock_relevant_endpoints() -> Vec<&'static ICloudEndpoint> {
    ICLOUD_ENDPOINTS.iter().filter(|e| matches!(
        e.role,
        ICloudEndpointRole::Other           // mcc / mccgateway
        | ICloudEndpointRole::KeyValueService
        | ICloudEndpointRole::MobileBackup
        | ICloudEndpointRole::Gateway
    ) || e.fqdn.contains("gateway-australia")
      || e.fqdn.contains("gatewayws-australia")
      || e.fqdn.contains("mcc")
    ).collect()
}

/// Return all endpoints that require mTLS.
pub fn mtls_endpoints() -> Vec<&'static ICloudEndpoint> {
    ICLOUD_ENDPOINTS.iter().filter(|e| e.requires_mtls).collect()
}

/// Summary string for diagnostics panel.
pub fn endpoint_summary() -> String {
    let total   = ICLOUD_ENDPOINTS.len();
    let live    = probe_confirmed_endpoints().len();
    let restore = restore_relevant_endpoints().len();
    let au      = au_unlock_relevant_endpoints().len();
    let china   = china_endpoints().len();
    let mtls    = mtls_endpoints().len();
    format!(
        "iCloud Endpoint Catalog: {} total | {} probe-confirmed | {} restore-relevant | {} AU-unlock-relevant | {} China | {} mTLS",
        total, live, restore, au, china, mtls
    )
}

// ─── Well-known IP blocks ─────────────────────────────────────────────────────

/// Apple iCloud IP ranges confirmed in this dataset.
/// Used for connectivity pre-checks before TSS/restore operations.
pub const APPLE_ICLOUD_IPV4_BLOCKS: &[&str] = &[
    "17.110.0.0/16",   // iCloud infrastructure (monitorm, connectivity nodes)
    "17.111.0.0/16",   // iCloud infrastructure (monitorm IC21)
    "17.143.0.0/16",   // iCloud internal services (antonws)
    "17.172.0.0/16",   // iCloud mail (mailws)
    "17.177.0.0/16",   // iCloud Photos (hubble)
    "17.178.0.0/16",   // iCloud infrastructure (connectivity, mr-e3sh)
    "17.248.0.0/16",   // iCloud gateway / CloudKit (background.gateway, ckhttpapi, wopi)
    "17.56.136.0/24",  // iCloud IMAP (imap.mail.icloud.com)
    "23.11.166.0/24",  // iCloud Beta / CDN edge (beta.icloud.com)
];

/// Apple iCloud IPv6 blocks confirmed in this dataset.
pub const APPLE_ICLOUD_IPV6_BLOCKS: &[&str] = &[
    "2403:300:a50:180::/64",  // iCloud gateway (background.gateway, ckhttpapi)
    "2600:1415:6c00::/48",    // iCloud Beta / CDN edge (beta.icloud.com)
];

// ─── URL builders ────────────────────────────────────────────────────────────

/// Build the HTTPS base URL for a given FQDN.
#[inline]
pub fn https_url(fqdn: &str) -> String {
    format!("https://{}", fqdn)
}

/// Build the activation status check URL for a device.
/// Endpoint: fmipmobile.icloud.com
pub fn activation_status_url(imei: &str, serial: &str) -> String {
    format!(
        "https://fmipmobile.icloud.com/deviceservices/deviceActivationStatusCheck?IMEIorSN={}&serial={}",
        imei, serial
    )
}

/// Build the Find My device lookup URL.
/// Endpoint: fmip.icloud.com
pub fn find_my_device_url(dsid: &str) -> String {
    format!("https://fmip.icloud.com/fmipservice/device/{}/initClient", dsid)
}

/// Build the escrow proxy lookup URL for a device ECID.
/// Endpoint: escrowproxy.icloud.com
pub fn escrow_proxy_url(ecid: u64) -> String {
    format!("https://escrowproxy.icloud.com/mobileservices/keychain/{:016X}", ecid)
}

/// Build the MCC gateway URL for carrier unlock status.
/// Endpoint: mccgateway.icloud.com
pub fn mcc_unlock_status_url(imei: &str) -> String {
    format!("https://mccgateway.icloud.com/devicelock/v1/status?imei={}", imei)
}

/// Australia-specific gateway PoP URL.
pub fn gateway_australia_url(path: &str) -> String {
    format!("https://gateway-australia.icloud.com{}", path)
}
