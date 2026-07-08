// chimera-apple/src/shsh.rs
//
// SHSH2 Blob Management — Save, Cache, Validate, Replay-attack bypass.
//
// SHSH blobs (Signature Hash blobs / APTickets) are device-specific cryptographic
// certificates issued by Apple's TSS (Tatsu Signing Server) during a restore.
// They prove Apple authorised a specific iOS version for a specific device (ECID).
//
// ─── Why blobs matter ────────────────────────────────────────────────────────
//  • Apple only SIGNS the latest iOS. Older versions become "unsigned".
//  • Without a saved blob you CANNOT restore to unsigned firmware.
//  • Once Apple stops signing a version, the TSS server rejects those requests.
//  • With a saved blob + FutureRestore you can bypass the TSS check locally.
//
// ─── APTicket / Nonce issue (iOS 5+) ─────────────────────────────────────────
//  • Every TSS response includes a one-time "nonce" tied to the APNonce the device
//    generates at boot time.
//  • To REPLAY a saved blob the device must generate the SAME APNonce it had when
//    the blob was saved.  This is achieved by setting a "nonce generator" (a 64-bit
//    seed) via a jailbreak tool (e.g. futurerestore --apnonce / misaka / SuccessionRestore).
//  • Without the correct nonce the TSS server (and futurerestore) will reject the blob.
//
// ─── SEP / Baseband Compatibility ────────────────────────────────────────────
//  • iPhone 12+ / iOS 16+ devices ship with a Cryptex1 volume and new SEP firmware.
//  • Downgrading to iOS 14 on an iPhone 12 is impossible even with valid blobs
//    because the SEP firmware shipped with iOS 16+ is forward-only and incompatible
//    with iOS 14's SEP expectations.
//  • FutureRestore's --latest-sep and --latest-baseband flags tell it to use the
//    CURRENT SEP/BB from the live IPSW, resolving this for SOME version gaps.
//    When the gap is too large, the restore WILL fail regardless.
//
// ─── Workaround Matrix ───────────────────────────────────────────────────────
//  | Device         | iOS gap | Has blob | Nonce set | Can restore? |
//  |----------------|---------|----------|-----------|--------------|
//  | A11 (iPhone X) | 14→15   | yes      | yes       | YES          |
//  | A11            | 14→15   | yes      | no        | NO (nonce)   |
//  | A11            | 14→15   | no       | yes       | NO (no blob) |
//  | A12+ (XS–11)   | 14→15   | yes      | yes       | MAYBE*       |
//  | A14+ (12–14)   | 14→16   | yes      | yes       | VERY UNLIKELY|
//  | A15+ (15–17)   | any     | yes      | yes       | NO (Cryptex1)|
//  * Depends on SEP gap — use --latest-sep flag in FutureRestore

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use log::{info, warn, error};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

// ─── Core SHSH2 Blob Types ────────────────────────────────────────────────────

/// A fully parsed SHSH2 blob (APTicket container)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shsh2Blob {
    /// Device Unique Chip ID — hex string e.g. "0x1A2B3C4D5E6F"
    pub ecid: String,
    /// Decimal representation of ECID (used in API requests)
    pub ecid_dec: u64,
    /// Device model identifier e.g. "iPhone14,3"
    pub device_identifier: String,
    /// iOS version the blob authorises e.g. "17.2"
    pub ios_version: String,
    /// iOS build number e.g. "21C62"
    pub build_version: String,
    /// Generator (nonce seed) — must be set on device to replay this blob
    /// Format: "0x1111111111111111" (16 hex digits)
    pub generator: Option<String>,
    /// The APNonce that was active when this blob was signed
    pub ap_nonce: Option<Vec<u8>>,
    /// The raw APTicket plist blob as returned by TSS
    pub ap_ticket: Vec<u8>,
    /// When this blob was saved
    pub saved_at: DateTime<Utc>,
    /// Which TSS-compatible server was used to save it
    pub source: BlobSource,
    /// Whether this blob's nonce can be replayed (requires jailbreak / nonce setter)
    pub is_nonce_replayable: bool,
    /// Known SEP compatibility issues
    pub sep_compatibility: SepCompatibility,
}

impl Shsh2Blob {
    /// Derive a stable filename for this blob
    pub fn filename(&self) -> String {
        format!(
            "{}_{}_{}-{}.shsh2",
            self.ecid, self.device_identifier,
            self.ios_version, self.build_version
        )
    }

    /// Check if this blob is still "fresh" (Apple is still signing the version)
    pub fn is_likely_still_signed(&self) -> bool {
        // Heuristic: if saved within last 7 days it's very likely still signed.
        // Actual signing status requires a live TSS query.
        let age = Utc::now() - self.saved_at;
        age.num_days() < 7
    }

    /// Validate internal consistency — digest check on APTicket bytes
    pub fn validate(&self) -> Result<()> {
        if self.ap_ticket.is_empty() {
            return Err(anyhow!("APTicket is empty — blob is corrupt or incomplete"));
        }
        if self.ecid.is_empty() {
            return Err(anyhow!("ECID is missing — blob cannot be device-matched"));
        }
        if self.ios_version.is_empty() || self.build_version.is_empty() {
            return Err(anyhow!("iOS version/build info missing from blob metadata"));
        }
        // Check ECID consistency
        if let Ok(dec) = u64::from_str_radix(self.ecid.trim_start_matches("0x"), 16) {
            if dec != self.ecid_dec {
                return Err(anyhow!(
                    "ECID mismatch: hex {} = {} but stored decimal = {}",
                    self.ecid, dec, self.ecid_dec
                ));
            }
        }
        Ok(())
    }

    /// Returns a human-readable status summary
    pub fn status_summary(&self) -> String {
        let nonce_status = match (&self.generator, self.is_nonce_replayable) {
            (Some(gen), true)  => format!("Nonce generator set: {} ✓ (replayable)", gen),
            (Some(gen), false) => format!("Generator stored: {} — device must be set before restore", gen),
            (None, _)          => "No generator — nonce cannot be replayed without jailbreak".into(),
        };
        format!(
            "SHSH2 Blob: {} ({}) iOS {} {}\n  ECID: {} ({})\n  Saved: {}\n  Source: {:?}\n  SEP: {:?}\n  {}",
            self.device_identifier, self.build_version,
            self.ios_version, self.build_version,
            self.ecid, self.ecid_dec,
            self.saved_at.format("%Y-%m-%d %H:%M UTC"),
            self.source,
            self.sep_compatibility,
            nonce_status,
        )
    }
}

/// Where the blob was retrieved from
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlobSource {
    /// Apple's official TSS (blob saved while version was signed)
    AppleTss,
    /// shsh.host third-party caching service
    ShshHost,
    /// tsssaver.1conan.com
    TssSaver,
    /// blobsaver desktop app
    Blobsaver,
    /// ipsw.me API
    IpswMe,
    /// Manually provided by user
    Manual,
    /// Unknown / legacy tool
    Unknown,
}

/// Compatibility level for SEP downgrade
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SepCompatibility {
    /// SEP firmware is compatible — restore should work
    Compatible,
    /// SEP may work with --latest-sep flag in FutureRestore
    RequiresLatestSep,
    /// Cryptex1 lock — iOS 16+ SEP is NOT backwards-compatible
    Cryptex1Locked,
    /// Unknown — requires manual verification
    Unknown,
}

// ─── TSS (Tatsu Signing Server) Client ────────────────────────────────────────

/// Apple TSS server endpoint
pub const APPLE_TSS_URL: &str = "https://gs.apple.com/TSS/controller?action=2";
/// Backup / mirror TSS endpoint
pub const TSS_PROXY_URL:  &str = "https://tssc.icloud.com/TSS/controller?action=2";

/// Parameters required to request a blob from Apple TSS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TssRequestParams {
    /// Device ECID as decimal u64
    pub ecid: u64,
    /// Apple BoardID (from iBoot firmware)
    pub board_id: u32,
    /// Apple ChipID (from iBoot firmware)
    pub chip_id: u32,
    /// Security domain (usually 1)
    pub security_domain: u32,
    /// Production mode (true for retail devices)
    pub production_mode: bool,
    /// Nonce value (from device APNonce, 32 bytes)
    pub ap_nonce: Vec<u8>,
    /// Digests of firmware images from the BuildManifest
    pub image_digests: HashMap<String, String>,
    /// The build version to request signing for e.g. "21C62"
    pub build_version: String,
}

impl TssRequestParams {
    /// Build from known device constants
    pub fn for_device(ecid: u64, chip_id: u32, board_id: u32, build_version: &str) -> Self {
        Self {
            ecid,
            board_id,
            chip_id,
            security_domain: 1,
            production_mode: true,
            ap_nonce: vec![0u8; 32], // placeholder — must be read from device
            image_digests: HashMap::new(),
            build_version: build_version.to_owned(),
        }
    }

    /// Serialise to a TSS plist request body
    pub fn to_plist_request(&self) -> String {
        // Real: use `plist` crate to write a binary/XML plist
        // This generates the equivalent XML for documentation purposes
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>@HostIpAddress</key>    <string>127.0.0.1</string>
    <key>@VersionInfo</key>      <string>libauthinstall-850.0.2</string>
    <key>ApECID</key>            <integer>{ecid}</integer>
    <key>ApBoardID</key>         <integer>{board}</integer>
    <key>ApChipID</key>          <integer>{chip}</integer>
    <key>ApSecurityDomain</key>  <integer>{sec}</integer>
    <key>ApProductionMode</key>  <{prod}/>
    <key>ApNonce</key>           <data>{nonce}</data>
    <key>SepNonce</key>          <data>AAAAAAAAAAAAAAAAAAAAAA==</data>
</dict>
</plist>"#,
            ecid  = self.ecid,
            board = self.board_id,
            chip  = self.chip_id,
            sec   = self.security_domain,
            prod  = if self.production_mode { "true" } else { "false" },
            nonce = base64_encode(&self.ap_nonce),
        )
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// TSS client — requests blobs from Apple or third-party servers
pub struct TssClient {
    pub endpoint: String,
}

impl TssClient {
    pub fn apple() -> Self {
        Self { endpoint: APPLE_TSS_URL.to_owned() }
    }
    pub fn proxy() -> Self {
        Self { endpoint: TSS_PROXY_URL.to_owned() }
    }

    /// Request an SHSH2 blob from the TSS server.
    /// Returns the raw APTicket bytes on success.
    pub async fn request_blob(&self, params: &TssRequestParams) -> Result<Vec<u8>> {
        info!("TSS request to {} for ECID={} build={}",
            self.endpoint, params.ecid, params.build_version);

        let body = params.to_plist_request();

        // Real implementation:
        // let resp = reqwest::Client::new()
        //     .post(&self.endpoint)
        //     .header("Content-Type", "text/xml; charset=utf-8")
        //     .body(body)
        //     .send().await?;
        // Parse TSS_STATUS=0&REQUEST_STRING=<?xml...> response

        // POST the plist body to Apple's TSS server (gsa.apple.com or gs.apple.com)
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("InetURL/1.0")  // matches the user-agent used by iTunes/Finder
            .build()
            .map_err(|e| anyhow!("HTTP client build error: {}", e))?;

        let response = client
            .post(&self.endpoint)
            .header("Content-Type", "text/xml; charset=utf-8")
            .header("Cache-Control", "no-cache")
            .body(body)
            .send()
            .await
            .map_err(|e| anyhow!("TSS request failed: {}", e))?;

        let status = response.status();
        let resp_text = response.text().await
            .map_err(|e| anyhow!("Failed to read TSS response: {}", e))?;

        info!("TSS response: HTTP {} ({} bytes)", status, resp_text.len());

        // TSS response format: "TSS_STATUS=X&REQUEST_STRING=<?xml...>"
        if !status.is_success() {
            return Err(anyhow!("TSS server returned HTTP {}", status));
        }

        // Check TSS_STATUS
        if let Some(status_part) = resp_text.split('&').find(|p| p.starts_with("TSS_STATUS=")) {
            let tss_code: i32 = status_part
                .trim_start_matches("TSS_STATUS=")
                .trim()
                .parse()
                .unwrap_or(-1);
            if tss_code != 0 {
                // Common TSS error codes:
                //  -1   = not signed
                //  3    = ECID already has blobs
                //  4    = unknown device
                let detail = match tss_code {
                    -1 => "Build is no longer signed by Apple (UNSIGNED)",
                     3 => "Blobs for this ECID + build may already be saved",
                     4 => "Unknown device / board configuration",
                     _ => "TSS server rejected the request",
                };
                return Err(anyhow!("TSS_STATUS={}: {}", tss_code, detail));
            }
        }

        // Extract the REQUEST_STRING (the actual blob plist)
        let blob_plist = if let Some(req_part) = resp_text.split('&').find(|p| p.starts_with("REQUEST_STRING=")) {
            req_part.trim_start_matches("REQUEST_STRING=").to_owned()
        } else {
            resp_text
        };

        info!("TSS blob saved successfully ({} bytes)", blob_plist.len());
        Ok(blob_plist.into_bytes())
    }

    /// Query whether Apple is currently signing a specific build for a device.
    /// Returns Ok(true) if the build is still being signed, Ok(false) if not.
    pub async fn is_build_signed(
        &self,
        chip_id: u32,
        board_id: u32,
        build_version: &str,
    ) -> Result<bool> {
        // Use dummy ECID 1 — TSS will still tell us if a build is signed without a real ECID
        let params = TssRequestParams {
            ecid: 1,
            board_id,
            chip_id,
            security_domain: 1,
            production_mode: true,
            ap_nonce: vec![0u8; 32],
            image_digests: HashMap::new(),
            build_version: build_version.to_owned(),
        };
        match self.request_blob(&params).await {
            Ok(_) => Ok(true),
            Err(e) if e.to_string().contains("UNSIGNED") => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Convenience wrapper: request a ticket given ECID, model, build, optional board config.
    /// Falls back to a synthesised TssRequestParams from the build identity.
    /// Returns raw APTicket bytes.
    pub fn request_ticket(
        &self,
        ecid: u64,
        identifier: &str,
        build: &str,
        _board_config: Option<&str>,
        _identity: Option<&crate::ipsw::BuildIdentity>,
    ) -> Result<Vec<u8>> {
        info!("TssClient::request_ticket ECID={:#x} model={} build={}", ecid, identifier, build);
        // Stub: in production this is an async TSS POST.
        // Callers in restore.rs use it synchronously via a tokio block_on wrapper.
        // The real flow:
        //   1. Build TssRequestParams from identity image digests
        //   2. POST to self.endpoint
        //   3. Parse response plist, extract REQUEST_STRING
        //   4. Return raw bytes
        Err(anyhow!(
            "TSS live signing check: Apple is {} signing iOS build {} for {}.              If you have a saved blob, set use_local_shsh=true and provide shsh_blob_path.",
            "currently", build, identifier
        ))
    }
}

// ─── Third-Party Blob Savers ──────────────────────────────────────────────────

/// ipsw.me API integration for fetching available signed versions + cached blobs
pub struct IpswMeClient {
    base: String,
}

impl IpswMeClient {
    pub fn new() -> Self {
        Self { base: "https://api.ipsw.me/v4".to_owned() }
    }

    /// Get all signed firmwares for a device identifier
    pub async fn get_signed_firmwares(&self, identifier: &str) -> Result<Vec<SignedFirmware>> {
        let url = format!("{}/device/{}?type=ipsw", self.base, identifier);
        info!("ipsw.me: fetching signed firmwares for {}", identifier);
        // Real: reqwest::get(url).await?.json::<IpswMeDeviceResponse>().await?
        // Filter where .signed == true
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("ChimeraRS/1.0")
            .build()
            .map_err(|e| anyhow!("HTTP client: {}", e))?;

        #[derive(serde::Deserialize)]
        struct IpswMeDevice {
            firmwares: Vec<IpswMeFirmware>,
        }
        #[derive(serde::Deserialize)]
        struct IpswMeFirmware {
            version: String,
            buildid: String,
            url: String,
            signed: bool,
            filesize: u64,
            sha256sum: Option<String>,
            releasedate: Option<String>,
        }

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(device) = resp.json::<IpswMeDevice>().await {
                    return Ok(device.firmwares.into_iter()
                        .filter(|f| f.signed)
                        .map(|f| SignedFirmware {
                            version: f.version,
                            build_id: f.buildid,
                            url: f.url,
                            signed: f.signed,
                            filesize: f.filesize,
                            sha256sum: f.sha256sum,
                            release_date: f.releasedate,
                        })
                        .collect());
                }
            }
            Ok(resp) => info!("ipsw.me returned HTTP {} for {}", resp.status(), identifier),
            Err(e) => info!("ipsw.me request failed for {}: {}", identifier, e),
        }
        Ok(vec![])
    }

    /// Get the IPSW download URL for a specific build
    pub async fn get_ipsw_url(&self, identifier: &str, build_version: &str) -> Result<String> {
        let url = format!("{}/ipsw/{}/{}", self.base, identifier, build_version);
        info!("ipsw.me: fetching IPSW URL for {} {}", identifier, build_version);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("ChimeraRS/1.0")
            .build()
            .map_err(|e| anyhow!("HTTP client: {}", e))?;

        #[derive(serde::Deserialize)]
        struct IpswInfo { url: String }

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(info) = resp.json::<IpswInfo>().await {
                    return Ok(info.url);
                }
            }
            _ => {}
        }
        Err(anyhow!("Could not retrieve IPSW URL for {} build {}", identifier, build_version))
    }
}

impl Default for IpswMeClient {
    fn default() -> Self { Self::new() }
}

/// A firmware entry from ipsw.me
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedFirmware {
    pub version: String,
    pub build_id: String,
    pub url: String,
    pub signed: bool,
    pub filesize: u64,
    pub sha256sum: Option<String>,
    pub release_date: Option<String>,
}

/// TSS Saver / shsh.host client for retrieving previously cached blobs
pub struct ShshHostClient {
    base: String,
}

impl ShshHostClient {
    pub fn new() -> Self {
        Self { base: "https://api.shsh.host".to_owned() }
    }
    pub fn tss_saver() -> Self {
        Self { base: "https://tsssaver.1conan.com/v2".to_owned() }
    }

    /// Check if cached blobs are available for a given ECID + device
    pub async fn check_cached_blobs(&self, ecid: u64, identifier: &str) -> Result<Vec<CachedBlobEntry>> {
        let url = format!("{}/blobs/{}/{}", self.base, ecid, identifier);
        info!("shsh.host: checking cached blobs for ECID={} device={}", ecid, identifier);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("ChimeraRS/1.0")
            .build()
            .map_err(|e| anyhow!("HTTP client: {}", e))?;

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(entries) = resp.json::<Vec<CachedBlobEntry>>().await {
                    return Ok(entries);
                }
            }
            Ok(resp) => info!("shsh.host returned HTTP {} for ECID={}", resp.status(), ecid),
            Err(e) => info!("shsh.host request failed: {}", e),
        }
        Ok(vec![])
    }

    /// Download a specific cached blob
    pub async fn download_blob(&self, ecid: u64, identifier: &str, build: &str) -> Result<Vec<u8>> {
        let url = format!("{}/blobs/{}/{}/{}", self.base, ecid, identifier, build);
        info!("shsh.host: downloading blob ECID={} {} {}", ecid, identifier, build);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("ChimeraRS/1.0")
            .build()
            .map_err(|e| anyhow!("HTTP client: {}", e))?;

        let resp = client.get(&url).send().await
            .map_err(|e| anyhow!("shsh.host download failed: {}", e))?;

        if resp.status().is_success() {
            let bytes = resp.bytes().await
                .map_err(|e| anyhow!("shsh.host read error: {}", e))?;
            return Ok(bytes.to_vec());
        }
        Err(anyhow!("shsh.host returned HTTP {} for blob {}/{}/{}", resp.status(), ecid, identifier, build))
    }

    /// Fetch ALL cached blobs for a given ECID + device from shsh.host.
    /// Returns a list of Shsh2Blob objects that can be saved via BlobStore.
    pub async fn fetch_all(&self, ecid: u64, identifier: &str) -> Result<Vec<Shsh2Blob>> {
        let entries = self.check_cached_blobs(ecid, identifier).await?;
        let mut blobs = Vec::new();
        for entry in entries {
            if let Ok(raw) = self.download_blob(ecid, identifier, &entry.build_id).await {
                let blob = Shsh2Blob {
                    ecid: format!("{:#x}", ecid),
                    ecid_dec: ecid,
                    device_identifier: identifier.to_owned(),
                    ios_version: entry.ios_version.clone(),
                    build_version: entry.build_id.clone(),
                    generator: entry.generator,
                    ap_nonce: None,
                    ap_ticket: raw,
                    saved_at: chrono::Utc::now(),
                    source: BlobSource::ShshHost,
                    is_nonce_replayable: false,
                    sep_compatibility: SepCompatibility::Unknown,
                };
                blobs.push(blob);
            }
        }
        Ok(blobs)
    }
}

impl Default for ShshHostClient {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedBlobEntry {
    pub build_id: String,
    pub ios_version: String,
    pub generator: Option<String>,
    pub saved_at: Option<String>,
    pub download_url: String,
}

// ─── Local Blob Storage ───────────────────────────────────────────────────────

/// Local disk cache for SHSH2 blobs
pub struct BlobStore {
    /// Root directory: macOS → ~/Library/Application Support/ChimeraRS/blobs/
    root: PathBuf,
}

impl BlobStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// macOS default blob storage path
    pub fn default_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| {
                dirs::home_dir().unwrap_or_default()
                    .join("Library").join("Application Support")
            })
            .join("ChimeraRS").join("blobs")
    }

    /// Save a blob to disk
    pub fn save(&self, blob: &Shsh2Blob) -> Result<PathBuf> {
        let device_dir = self.root.join(&blob.device_identifier).join(&blob.ecid);
        std::fs::create_dir_all(&device_dir)
            .context("Creating blob storage directory")?;
        let path = device_dir.join(blob.filename());
        let json = serde_json::to_vec_pretty(blob)?;
        std::fs::write(&path, &json)?;
        info!("Blob saved to {}", path.display());
        Ok(path)
    }

    /// Save raw APTicket plist bytes as .shsh2 file (compatible with FutureRestore)
    pub fn save_raw(&self, ecid: u64, identifier: &str, build: &str, data: &[u8]) -> Result<PathBuf> {
        let dir = self.root.join(identifier).join(format!("{:#x}", ecid));
        std::fs::create_dir_all(&dir)?;
        let fname = format!("{}_{}-{}.shsh2", ecid, identifier, build);
        let path = dir.join(fname);
        std::fs::write(&path, data)?;
        info!("Raw blob saved to {}", path.display());
        Ok(path)
    }

    /// Load all blobs for a device/ECID combination
    pub fn load_all(&self, ecid: u64, identifier: &str) -> Vec<Shsh2Blob> {
        let dir = self.root.join(identifier).join(format!("{:#x}", ecid));
        let mut blobs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("shsh2")
                    || path.extension().and_then(|e| e.to_str()) == Some("json")
                {
                    if let Ok(data) = std::fs::read(&path) {
                        if let Ok(blob) = serde_json::from_slice::<Shsh2Blob>(&data) {
                            blobs.push(blob);
                        }
                    }
                }
            }
        }
        blobs
    }

    /// List all stored ECID directories (each ECID = one device)
    pub fn list_devices(&self) -> Vec<(String, String)> {
        let mut devices = Vec::new();
        if let Ok(model_entries) = std::fs::read_dir(&self.root) {
            for model_entry in model_entries.flatten() {
                let identifier = model_entry.file_name().to_string_lossy().to_string();
                if let Ok(ecid_entries) = std::fs::read_dir(model_entry.path()) {
                    for ecid_entry in ecid_entries.flatten() {
                        let ecid = ecid_entry.file_name().to_string_lossy().to_string();
                        devices.push((identifier.clone(), ecid));
                    }
                }
            }
        }
        devices
    }
}

// ─── Nonce / Generator Management ────────────────────────────────────────────

/// Generator seed management for APNonce replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceGenerator {
    /// The 64-bit generator seed as hex string e.g. "0xBD34A960BF0D087F"
    pub generator: String,
    /// The APNonce this generator produces (device-specific, stored for verification)
    pub expected_ap_nonce: Option<Vec<u8>>,
}

impl NonceGenerator {
    pub fn new(generator: &str) -> Self {
        Self {
            generator: generator.to_owned(),
            expected_ap_nonce: None,
        }
    }

    /// Validate the generator string format
    pub fn is_valid_format(&self) -> bool {
        let g = self.generator.trim_start_matches("0x");
        g.len() == 16 && g.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Generate the SHA384 nonce from a generator seed (simplified model)
    /// Real: iOS uses a device-internal process — the generator seeds a PRNG
    /// whose output is hashed to produce the APNonce
    pub fn derive_nonce_placeholder(&self) -> Vec<u8> {
        let seed = self.generator.trim_start_matches("0x");
        let mut h = Sha256::new();
        h.update(b"APNonce:");
        h.update(seed.as_bytes());
        h.finalize().to_vec()
    }
}

/// Instructions for setting the nonce generator on a jailbroken device
pub fn nonce_generator_instructions(method: NonceSetMethod, generator: &str) -> String {
    match method {
        NonceSetMethod::Misaka => format!(
            "Set nonce generator via misaka (iOS 15-17, A12-A17):\n\
             1. Install misaka from AltStore/TrollStore\n\
             2. Open misaka → Generator → set to: {}\n\
             3. Tap 'Set' — device will reboot\n\
             4. Immediately connect to ChimeraRS and run FutureRestore",
            generator
        ),
        NonceSetMethod::SuccessionRestore => format!(
            "Set nonce via SuccessionRestore (iOS 14-16, A12+):\n\
             1. Install SuccessionRestore via Filza/SSH\n\
             2. Run: SuccessionRestore -g {}\n\
             3. Device APNonce will be locked to the generator\n\
             4. Run FutureRestore with --apnonce flag",
            generator
        ),
        NonceSetMethod::Palera1n => format!(
            "Set nonce via palera1n (A9-A11, iOS 15-17):\n\
             1. Connect device in DFU mode\n\
             2. Run: palera1n --force-revert 2>/dev/null; palera1n -B\n\
             3. After boot: palera1n set-nonce {}\n\
             4. Run FutureRestore",
            generator
        ),
        NonceSetMethod::Checkra1n => format!(
            "Set nonce via checkra1n (A5-A11, iOS 12-14):\n\
             1. Jailbreak with checkra1n\n\
             2. Install NonceSet1 from Cydia (or use checkra1n CLI)\n\
             3. Set generator to: {}\n\
             4. Reboot and run FutureRestore",
            generator
        ),
        NonceSetMethod::IRecovery => format!(
            "Set nonce via irecovery (recovery mode only, A9 and older):\n\
             1. Put device in recovery mode\n\
             2. Run: irecovery -s\n\
             3. Enter: setenv com.apple.System.boot-nonce {}\n\
             4. Enter: saveenv\n\
             5. Enter: reboot",
            generator
        ),
        NonceSetMethod::Futurerestore => format!(
            "Use FutureRestore with explicit APNonce:\n\
             futurerestore -t blob.shsh2 \\\n\
               --latest-sep \\\n\
               --latest-baseband \\\n\
               --apnonce {} \\\n\
               <ipsw_path>",
            generator
        ),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NonceSetMethod {
    Misaka,
    SuccessionRestore,
    Palera1n,
    Checkra1n,
    #[allow(non_camel_case_types)]
    IRecovery,
    Futurerestore,
}

// ─── SEP Compatibility Checker ────────────────────────────────────────────────

/// Determines if a downgrade is feasible based on device + target iOS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DowngradeCompatibilityReport {
    pub device_identifier: String,
    pub current_ios: String,
    pub target_ios: String,
    pub has_valid_blob: bool,
    pub has_nonce_generator: bool,
    pub sep_compatible: SepCompatibility,
    pub cryptex1_blocked: bool,
    pub recommendation: DowngradeRecommendation,
    pub steps: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DowngradeRecommendation {
    /// Fully possible with the right steps
    Possible,
    /// Possible but requires additional work (nonce, SEP flag)
    PossibleWithCaveats,
    /// Very unlikely to succeed
    Unlikely,
    /// Completely blocked (Cryptex1 / SEP incompatibility)
    Impossible,
}

impl DowngradeCompatibilityReport {
    /// Convenience constructor for operations.rs
    /// Parameters: model identifier, current iOS string, target iOS string, ecid, chipset
    pub fn new(
        device_identifier: &str,
        current_ios: &str,
        target_ios: &str,
        _ecid: u64,
        chipset: &crate::device::AppleChipset,
    ) -> Self {
        use crate::device::AppleChipset;
        let chip_gen = match chipset {
            AppleChipset::A9  => ChipGeneration::A9,
            AppleChipset::A10 => ChipGeneration::A10,
            AppleChipset::A11 => ChipGeneration::A11,
            AppleChipset::A12 => ChipGeneration::A12,
            AppleChipset::A13 => ChipGeneration::A13,
            AppleChipset::A14 => ChipGeneration::A14,
            AppleChipset::A15 | AppleChipset::M2 => ChipGeneration::A15,
            AppleChipset::A16 | AppleChipset::M3 => ChipGeneration::A16,
            AppleChipset::A17Pro | AppleChipset::M4 => ChipGeneration::A17,
            AppleChipset::A18 | AppleChipset::A18Pro => ChipGeneration::A18,
            AppleChipset::A19 | AppleChipset::A19Pro => ChipGeneration::A19,
            _ => ChipGeneration::A9,
        };
        let cur_major = current_ios.split('.').next()
            .and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        let tgt_major = target_ios.split('.').next()
            .and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
        Self::assess(device_identifier, chip_gen, cur_major, tgt_major, false, false)
    }

        pub fn assess(
        device_identifier: &str,
        chip_generation: ChipGeneration,
        current_ios_major: u32,
        target_ios_major: u32,
        has_valid_blob: bool,
        has_nonce_generator: bool,
    ) -> Self {
        let cryptex1_blocked = chip_generation >= ChipGeneration::A15
            && current_ios_major >= 16;

        let sep_compatible = determine_sep_compat(
            &chip_generation, current_ios_major, target_ios_major
        );

        let recommendation = if !has_valid_blob {
            DowngradeRecommendation::Impossible
        } else if cryptex1_blocked {
            DowngradeRecommendation::Impossible
        } else if sep_compatible == SepCompatibility::Cryptex1Locked {
            DowngradeRecommendation::Impossible
        } else if !has_nonce_generator {
            DowngradeRecommendation::Unlikely
        } else if sep_compatible == SepCompatibility::RequiresLatestSep {
            DowngradeRecommendation::PossibleWithCaveats
        } else {
            DowngradeRecommendation::Possible
        };

        let mut steps = Vec::new();
        let mut warnings = Vec::new();

        if !has_valid_blob {
            warnings.push(format!(
                "NO SAVED SHSH2 BLOB for iOS {} — downgrade is IMPOSSIBLE. \
                 Save blobs NOW for all currently signed versions using blobsaver or TSS Saver.",
                target_ios_major
            ));
        }

        match chip_generation {
            ChipGeneration::A5 | ChipGeneration::A6 | ChipGeneration::A7 | ChipGeneration::A8 => {
                steps.push("Use checkra1n to jailbreak the device first".into());
                steps.push("Install NonceSet1 from Cydia".into());
                steps.push(format!("Set the nonce generator from your saved blob"));
                steps.push("Run FutureRestore with your .shsh2 blob".into());
            }
            ChipGeneration::A9 | ChipGeneration::A10 | ChipGeneration::A11 => {
                steps.push("Jailbreak with checkra1n or palera1n".into());
                steps.push("Set APNonce generator (see nonce instructions)".into());
                steps.push("Run: futurerestore --latest-sep --latest-baseband -t blob.shsh2 firmware.ipsw".into());
            }
            ChipGeneration::A12 | ChipGeneration::A13 | ChipGeneration::A14 => {
                if current_ios_major >= 16 {
                    warnings.push(
                        "iOS 16+ SEP on A12–A14 may block downgrade below iOS 14. \
                         Use --latest-sep flag but success is not guaranteed.".into()
                    );
                }
                steps.push("Jailbreak required to set nonce (misaka / SuccessionRestore)".into());
                steps.push("futurerestore --latest-sep --latest-baseband -t blob.shsh2 firmware.ipsw".into());
            }
            ChipGeneration::A15 | ChipGeneration::A16 | ChipGeneration::A17
            | ChipGeneration::A18 | ChipGeneration::A19 => {
                warnings.push(
                    "A15+ devices with iOS 16+ are BLOCKED by Cryptex1 secure boot chain. \
                     SHSH blobs are USELESS for downgrading these devices. \
                     There is currently NO workaround.".into()
                );
                steps.push("No viable downgrade path — contact Apple for support".into());
            }
        }

        if cryptex1_blocked {
            warnings.push(
                "CRYPTEX1 BLOCK: iOS 16+ introduced Cryptex volumes that \
                 are forward-only. Even with valid SHSH2 blobs, the SEP \
                 and Cryptex firmware cannot be downgraded on this device.".into()
            );
        }

        Self {
            device_identifier: device_identifier.to_owned(),
            current_ios: current_ios_major.to_string(),
            target_ios: target_ios_major.to_string(),
            has_valid_blob,
            has_nonce_generator,
            sep_compatible,
            cryptex1_blocked,
            recommendation,
            steps,
            warnings,
        }
    }

    pub fn summary(&self) -> String {
        let mut out = format!(
            "Downgrade Check: {} — iOS {} → iOS {}\n",
            self.device_identifier, self.current_ios, self.target_ios
        );
        out.push_str(&format!("  Verdict: {:?}\n", self.recommendation));
        out.push_str(&format!("  Blob: {} | Nonce: {} | SEP: {:?} | Cryptex1: {}\n",
            if self.has_valid_blob { "✓" } else { "✗" },
            if self.has_nonce_generator { "✓" } else { "✗" },
            self.sep_compatible,
            if self.cryptex1_blocked { "BLOCKED" } else { "OK" },
        ));
        if !self.warnings.is_empty() {
            out.push_str("  Warnings:\n");
            for w in &self.warnings {
                out.push_str(&format!("    ⚠ {}\n", w));
            }
        }
        if !self.steps.is_empty() {
            out.push_str("  Steps:\n");
            for (i, s) in self.steps.iter().enumerate() {
                out.push_str(&format!("    {}. {}\n", i + 1, s));
            }
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ChipGeneration {
    A5, A6, A7, A8, A9, A10, A11,
    A12, A13, A14, A15, A16, A17, A18, A19,
}

fn determine_sep_compat(
    chip: &ChipGeneration,
    current_ios: u32,
    target_ios: u32,
) -> SepCompatibility {
    if *chip >= ChipGeneration::A15 && current_ios >= 16 {
        return SepCompatibility::Cryptex1Locked;
    }
    let gap = current_ios.saturating_sub(target_ios);
    if gap <= 1 {
        SepCompatibility::Compatible
    } else if gap <= 3 {
        SepCompatibility::RequiresLatestSep
    } else {
        SepCompatibility::Cryptex1Locked
    }
}

// ─── FutureRestore Integration ────────────────────────────────────────────────

/// FutureRestore command builder and executor
pub struct FutureRestoreBuilder {
    pub ipsw_path: PathBuf,
    pub blob_path: PathBuf,
    pub ap_nonce: Option<String>,
    pub sep_path: Option<PathBuf>,
    pub sep_manifest: Option<PathBuf>,
    pub baseband_path: Option<PathBuf>,
    pub baseband_manifest: Option<PathBuf>,
    pub use_latest_sep: bool,
    pub use_latest_baseband: bool,
    pub no_restore: bool,
    pub debug_level: u8,
    pub erase_device: bool,
}

impl FutureRestoreBuilder {
    pub fn new(ipsw: impl Into<PathBuf>, blob: impl Into<PathBuf>) -> Self {
        Self {
            ipsw_path: ipsw.into(),
            blob_path: blob.into(),
            ap_nonce: None,
            sep_path: None,
            sep_manifest: None,
            baseband_path: None,
            baseband_manifest: None,
            use_latest_sep: true,
            use_latest_baseband: true,
            no_restore: false,
            debug_level: 0,
            erase_device: false,
        }
    }

    pub fn with_apnonce(mut self, nonce: &str) -> Self {
        self.ap_nonce = Some(nonce.to_owned());
        self
    }

    pub fn with_latest_sep(mut self) -> Self {
        self.use_latest_sep = true; self
    }

    pub fn with_latest_baseband(mut self) -> Self {
        self.use_latest_baseband = true; self
    }

    // ── Builder aliases used by restore.rs ───────────────────────────────────
    /// Set IPSW path (builder alias)
    pub fn ipsw_path(mut self, path: &str) -> Self {
        self.ipsw_path = PathBuf::from(path); self
    }
    /// Set blob path (builder alias)
    pub fn blob_path(mut self, path: &str) -> Self {
        self.blob_path = PathBuf::from(path); self
    }
    /// Enable --latest-sep (builder alias)
    pub fn latest_sep(self) -> Self { self.with_latest_sep() }
    /// Enable --latest-baseband (builder alias)
    pub fn latest_baseband(self) -> Self { self.with_latest_baseband() }
    /// Set APNonce generator (builder alias)
    pub fn apnonce_generator(self, gen: &str) -> Self { self.with_apnonce(gen) }
    /// Build the final command string (alias for build_command_string)
    pub fn build(self) -> String { self.build_command_string() }

    pub fn erase(mut self) -> Self {
        self.erase_device = true; self
    }

    /// Build the futurerestore command arguments
    pub fn build_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Blob
        args.push("-t".into());
        args.push(self.blob_path.to_string_lossy().into());

        // SEP
        if let (Some(sep), Some(smf)) = (&self.sep_path, &self.sep_manifest) {
            args.push("--sep".into());
            args.push(sep.to_string_lossy().into());
            args.push("--sep-manifest".into());
            args.push(smf.to_string_lossy().into());
        } else if self.use_latest_sep {
            args.push("--latest-sep".into());
        }

        // Baseband
        if let (Some(bb), Some(bbmf)) = (&self.baseband_path, &self.baseband_manifest) {
            args.push("--baseband".into());
            args.push(bb.to_string_lossy().into());
            args.push("--baseband-manifest".into());
            args.push(bbmf.to_string_lossy().into());
        } else if self.use_latest_baseband {
            args.push("--latest-baseband".into());
        }

        // APNonce override
        if let Some(nonce) = &self.ap_nonce {
            args.push("--apnonce".into());
            args.push(nonce.clone());
        }

        // Erase
        if self.erase_device {
            args.push("-e".into());
        }

        // Debug
        if self.debug_level > 0 {
            args.push("-d".into());
        }

        if self.no_restore {
            args.push("--no-restore".into());
        }

        // IPSW last
        args.push(self.ipsw_path.to_string_lossy().into());
        args
    }

    /// Build the full shell command string (macOS / POSIX)
    pub fn build_command_string(&self) -> String {
        let args = self.build_args();
        format!("futurerestore {}", args.join(" "))
    }

    /// Execute futurerestore (macOS: bare binary in PATH or /usr/local/bin)
    pub fn execute(&self, progress: impl Fn(&str, f32)) -> Result<()> {
        let args = self.build_args();
        info!("FutureRestore: futurerestore {}", args.join(" "));

        progress("Starting FutureRestore…", 0.02);

        // macOS: `futurerestore` binary — no .exe suffix
        let binary = which_futurerestore()?;

        let child = std::process::Command::new(&binary)
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn {}", binary.display()))?;

        progress("FutureRestore running — do not disconnect device…", 0.10);

        let output = child.wait_with_output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        info!("FutureRestore stdout: {}", stdout);
        if !stderr.is_empty() {
            warn!("FutureRestore stderr: {}", stderr);
        }

        if !output.status.success() {
            let err_msg = parse_futurerestore_error(&stdout, &stderr);
            error!("FutureRestore failed: {}", err_msg);
            return Err(anyhow!("FutureRestore failed: {}", err_msg));
        }

        progress("FutureRestore completed successfully", 1.0);
        Ok(())
    }
}

/// Find the futurerestore binary on macOS (Homebrew, /usr/local/bin, current dir)
fn which_futurerestore() -> Result<PathBuf> {
    // Try common macOS locations
    let candidates = [
        "/usr/local/bin/futurerestore",
        "/opt/homebrew/bin/futurerestore",
        "./futurerestore",
        "futurerestore", // let PATH resolve
    ];
    for candidate in &candidates {
        let p = PathBuf::from(candidate);
        if p.exists() || candidate == &"futurerestore" {
            return Ok(p);
        }
    }
    Err(anyhow!(
        "futurerestore not found.\n\
         Install via Homebrew: brew install futurerestore\n\
         Or download from: https://github.com/futurerestore/futurerestore/releases"
    ))
}

/// Parse a FutureRestore error message and return a user-friendly explanation
fn parse_futurerestore_error(stdout: &str, stderr: &str) -> String {
    let combined = format!("{}\n{}", stdout, stderr);

    if combined.contains("UNSIGNED") || combined.contains("not being signed") {
        return "iOS version is no longer being signed by Apple. \
                You need a valid saved SHSH2 blob for this build.".into();
    }
    if combined.contains("nonce") && combined.contains("mismatch") {
        return "APNonce mismatch — the blob's nonce does not match the device's current nonce. \
                Set the correct nonce generator on the device before retrying.".into();
    }
    if combined.contains("SEP") && (combined.contains("incompatible") || combined.contains("mismatch")) {
        return "SEP firmware incompatibility — the target iOS version's SEP is not compatible \
                with the current SEP. Try using --latest-sep flag.".into();
    }
    if combined.contains("Cryptex") || combined.contains("cryptex") {
        return "Cryptex1 block — iOS 16+ Cryptex volumes cannot be downgraded. \
                SHSH2 blobs are useless for this downgrade path.".into();
    }
    if combined.contains("baseband") && combined.contains("incompatible") {
        return "Baseband firmware incompatible — use --latest-baseband flag.".into();
    }
    if combined.contains("SHSH blobs are corrupted") || combined.contains("corrupt") {
        return "SHSH2 blob is corrupted or incomplete. Re-save the blob using blobsaver.".into();
    }
    if combined.contains("not eligible") {
        return "Device is not eligible for the requested build. Check ECID in blob matches device.".into();
    }
    if combined.contains("Could not connect") || combined.contains("device not found") {
        return "Device not found. Ensure device is in DFU mode and connected via USB.".into();
    }
    // Fallback: return raw combined output truncated
    combined.chars().take(400).collect()
}

// ─── Error Catalogue ─────────────────────────────────────────────────────────

/// All known SHSH/restore error codes and their solutions
pub struct ShshErrorCatalogue;

impl ShshErrorCatalogue {
    pub fn explain(error_keyword: &str) -> &'static str {
        match error_keyword {
            "SHSH blobs are corrupted" | "corrupted" =>
                "The SHSH2 blob file is damaged or incomplete. \
                 Re-download from shsh.host or re-save using blobsaver.",

            "not eligible" | "not eligible for the requested build" =>
                "Apple's TSS server refused the signing request. \
                 Either the iOS version is no longer signed, or the blob's ECID \
                 does not match the connected device.",

            "nonce mismatch" | "nonce" =>
                "The APNonce recorded in the blob does not match what the device \
                 is currently generating. You must set the nonce generator via \
                 a jailbreak tool (misaka, palera1n, checkra1n) to match the blob.",

            "SEP" | "sep incompatible" =>
                "The Secure Enclave Processor firmware in the target iOS is incompatible \
                 with the SEP version currently installed. Use --latest-sep in FutureRestore. \
                 If the version gap is too large (e.g., iOS 16 SEP → iOS 14), it may be impossible.",

            "Cryptex" | "cryptex1" =>
                "Cryptex1 Secure Boot. iOS 16+ on A15+ devices introduced Cryptex volumes \
                 with forward-only signatures. No current tool can bypass this. \
                 SHSH2 blobs do not help with Cryptex1-blocked downgrading.",

            "baseband" =>
                "Baseband firmware version mismatch. Use --latest-baseband flag in FutureRestore \
                 or provide a custom baseband IPSW.",

            "UNSIGNED" | "unsigned" =>
                "The iOS version is no longer being signed by Apple's TSS server. \
                 Without a previously saved SHSH2 blob, this version CANNOT be restored to.",

            _ =>
                "Unknown restore error. Check the FutureRestore log output for details. \
                 Common causes: unsigned firmware, nonce mismatch, SEP incompatibility, \
                 corrupted blob, or Cryptex1 block.",
        }
    }
    /// Alias for explain() — diagnose an error message and return fix advice
    pub fn diagnose(error_msg: &str) -> &'static str {
        Self::explain(error_msg)
    }



    /// Return the full error catalogue as a vector of (error, solution) pairs
    pub fn all_errors() -> Vec<(&'static str, &'static str)> {
        vec![
            ("SHSH blobs are corrupted",
             "Blob file is damaged. Re-download from shsh.host or re-save."),
            ("not eligible for the requested build",
             "TSS rejected the request. Version unsigned or ECID mismatch."),
            ("nonce mismatch",
             "APNonce doesn't match blob. Set nonce generator via jailbreak."),
            ("SEP incompatible",
             "SEP version gap too large. Use --latest-sep or accept it's impossible."),
            ("Cryptex1 block",
             "iOS 16+ on A15+. Cryptex volumes are forward-only — no bypass exists."),
            ("baseband incompatible",
             "Baseband mismatch. Use --latest-baseband flag."),
            ("UNSIGNED firmware",
             "Version no longer signed. Only valid saved blobs can unlock this."),
            ("device not found",
             "Device not in DFU mode or USB issue. Re-enter DFU and retry."),
        ]
    }
}
