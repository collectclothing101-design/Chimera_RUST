// chimera-api/src/firmware_api.rs
// Firmware metadata/download API client.
// Maps data.chimeratool.com endpoints to open replacement APIs.
// Primary open sources: ipsw.me (Apple), SamFW.com / SamMobile (Samsung),
//   firmware.mobi, LG Flash Tool, Motorola Rescue, etc.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use log::{info, warn};
use crate::client::{ApiClient, base_url};

// ── Apple firmware via ipsw.me ───────────────────────────────────────────────

/// IPSW.me device response
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IpswDevice {
    pub name: String,
    pub identifier: String,  // e.g. "iPhone14,3"
    pub boards: Vec<String>,
    pub bdid: Option<u32>,
    pub firmwares: Vec<IpswFirmwareEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IpswFirmwareEntry {
    pub identifier: String,
    pub version: String,
    pub buildid: String,
    pub sha1sum: String,
    pub sha256sum: Option<String>,
    pub md5sum: Option<String>,
    pub filesize: u64,
    pub url: String,
    pub releasedate: Option<String>,
    pub uploaddate: Option<String>,
    pub signed: bool,
}

/// Fetch all available firmwares for an Apple device identifier
pub async fn fetch_apple_firmwares(device_identifier: &str) -> Result<Vec<IpswFirmwareEntry>> {
    let url = format!("{}/device/{}?type=ipsw", base_url::OPEN_FIRMWARE_APPLE, device_identifier);
    info!("Fetching Apple firmwares from ipsw.me: {}", url);
    let client = ApiClient::open();
    let device: IpswDevice = client.get(&url).await
        .map_err(|e| anyhow!("ipsw.me fetch failed: {}", e))?;
    Ok(device.firmwares)
}

/// Get only SIGNED (installable) firmwares for a device
pub async fn fetch_signed_apple_firmwares(device_identifier: &str) -> Result<Vec<IpswFirmwareEntry>> {
    let all = fetch_apple_firmwares(device_identifier).await?;
    Ok(all.into_iter().filter(|f| f.signed).collect())
}

/// Get the latest signed firmware URL for a device
pub async fn get_latest_ipsw_url(device_identifier: &str) -> Result<IpswFirmwareEntry> {
    let signed = fetch_signed_apple_firmwares(device_identifier).await?;
    signed.into_iter().next()
        .ok_or_else(|| anyhow!("No signed firmwares available for {}", device_identifier))
}

// ── Samsung firmware via SamFW ───────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SamsungFirmwareEntry {
    pub model: String,
    pub region: String,       // CSC/region code e.g. "XSA" (Australia)
    pub version: String,      // PDA/CSC/modem version
    pub os_version: String,   // Android version
    pub release_date: String,
    pub size_mb: u64,
    pub download_url: Option<String>,
    pub changelog: Option<String>,
}

/// Australian Samsung firmware regions
pub const AU_SAMSUNG_REGIONS: &[(&str, &str)] = &[
    ("XSA", "Australia (Generic)"),
    ("OPP", "Optus Australia"),
    ("TEL", "Telstra Australia"),
    ("VFN", "Vodafone Australia"),
    ("TPG", "TPG Mobile Australia"),
];

/// Fetch Samsung firmware list using the open SamFW API endpoint.
///
/// Queries https://samfw.com/api/v4/firmware/{model}/{region} which returns a JSON
/// array. Falls back to a single informational entry if the API is unavailable.
pub async fn fetch_samsung_firmwares(model: &str, region: &str) -> Result<Vec<SamsungFirmwareEntry>> {
    // Primary: SamFW JSON API (free, no auth)
    let api_url = format!("https://samfw.com/api/v4/firmware/{}/{}", model, region);
    info!("Fetching Samsung firmwares: model={} region={} url={}", model, region, api_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("ChimeraRS/1.0")
        .build()
        .map_err(|e| anyhow!("HTTP client error: {}", e))?;

    match client.get(&api_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            // SamFW API returns: [{"pda":"...","csc":"...","modem":"...","os":"...","date":"...","filename":"...","size":...}]
            #[derive(serde::Deserialize)]
            struct SamFwEntry {
                pda: Option<String>,
                csc: Option<String>,
                os: Option<String>,
                date: Option<String>,
                size: Option<u64>,
            }

            if let Ok(entries) = resp.json::<Vec<SamFwEntry>>().await {
                if !entries.is_empty() {
                    return Ok(entries.into_iter().map(|e| {
                        let pda = e.pda.unwrap_or_default();
                        let csc_ver = e.csc.unwrap_or_default();
                        let version = format!("{}/{}", pda, csc_ver);
                        SamsungFirmwareEntry {
                            model: model.to_owned(),
                            region: region.to_owned(),
                            version,
                            os_version: e.os.unwrap_or_else(|| "Unknown".into()),
                            release_date: e.date.unwrap_or_else(|| "Unknown".into()),
                            size_mb: e.size.unwrap_or(0) / 1_048_576,
                            download_url: Some(format!("https://samfw.com/firmware/{}/{}", model, region)),
                            changelog: None,
                        }
                    }).collect());
                }
            }
        }
        Ok(resp) => warn!("SamFW API returned HTTP {}", resp.status()),
        Err(e) => warn!("SamFW API request failed: {}", e),
    }

    // Fallback: return a descriptive placeholder pointing to the SamFW web page
    Ok(vec![SamsungFirmwareEntry {
        model: model.to_owned(),
        region: region.to_owned(),
        version: "See samfw.com for latest version".to_owned(),
        os_version: "Unknown".to_owned(),
        release_date: "Unknown".to_owned(),
        size_mb: 0,
        download_url: Some(format!("https://samfw.com/firmware/{}/{}", model, region)),
        changelog: Some("Visit samfw.com to download the latest firmware for this model/region.".into()),
    }])
}

// ── firmware.mobi (multi-brand) ──────────────────────────────────────────────

/// Supported brands on firmware.mobi
pub const FIRMWARE_MOBI_BRANDS: &[&str] = &[
    "Samsung", "LG", "Motorola", "HTC", "Huawei", "Nokia", "Sony",
];

/// Fetch firmware links from firmware.mobi (multi-brand open source)
pub async fn fetch_firmware_mobi(brand: &str, model: &str) -> Result<Vec<String>> {
    let api_url = format!("https://firmware.mobi/api/firmware/{}/{}", brand.to_lowercase(), model);
    info!("Fetching firmware from firmware.mobi: {}/{}", brand, model);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("ChimeraRS/1.0")
        .build()
        .map_err(|e| anyhow!("HTTP client: {}", e))?;

    match client.get(&api_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            // firmware.mobi returns JSON array of download URLs
            if let Ok(urls) = resp.json::<Vec<String>>().await {
                if !urls.is_empty() {
                    return Ok(urls);
                }
            }
        }
        Ok(resp) => warn!("firmware.mobi returned HTTP {}", resp.status()),
        Err(e) => warn!("firmware.mobi request failed: {}", e),
    }

    // Fallback: return the search page URL
    Ok(vec![format!("https://firmware.mobi/firmware/{}/{}", brand.to_lowercase(), model)])
}

// ── Chimera data.chimeratool.com replacement ─────────────────────────────────

/// Unified firmware search request (replaces data.chimeratool.com /v1/firmware/*)
#[derive(Debug, Serialize, Deserialize)]
pub struct FirmwareSearchRequest {
    pub brand: String,
    pub model: String,
    pub region: Option<String>,
    pub android_version: Option<String>,
}

/// Unified firmware search result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FirmwareResult {
    pub brand: String,
    pub model: String,
    pub version: String,
    pub region: Option<String>,
    pub size_mb: u64,
    pub download_url: String,
    pub is_latest: bool,
    pub signed: bool,
    pub source: &'static str, // "ipsw.me", "samfw.com", "firmware.mobi"
}

/// Search for firmware using the appropriate open source
pub async fn search_firmware(req: &FirmwareSearchRequest) -> Result<Vec<FirmwareResult>> {
    let brand_lower = req.brand.to_lowercase();
    if brand_lower == "apple" || brand_lower == "iphone" || brand_lower == "ipad" {
        let entries = fetch_signed_apple_firmwares(&req.model).await?;
        Ok(entries.into_iter().enumerate().map(|(i, e)| FirmwareResult {
            brand: "Apple".into(),
            model: req.model.clone(),
            version: format!("iOS {} ({})", e.version, e.buildid),
            region: None,
            size_mb: e.filesize / 1_048_576,
            download_url: e.url,
            is_latest: i == 0,
            signed: e.signed,
            source: "ipsw.me",
        }).collect())
    } else if brand_lower == "samsung" {
        let region = req.region.as_deref().unwrap_or("XSA"); // Default AU region
        let entries = fetch_samsung_firmwares(&req.model, region).await?;
        Ok(entries.into_iter().enumerate().map(|(i, e)| FirmwareResult {
            brand: "Samsung".into(),
            model: e.model,
            version: e.version,
            region: Some(e.region),
            size_mb: e.size_mb,
            download_url: e.download_url.unwrap_or_default(),
            is_latest: i == 0,
            signed: true,
            source: "samfw.com",
        }).collect())
    } else {
        Err(anyhow!("No open firmware source available for brand: {}", req.brand))
    }
}
