// chimera-api/src/client.rs
// HTTP client wrapper for making requests to both ChimeraTool-compatible endpoints
// and the open replacement APIs. Supports:
//   - Request signing (matching ChimeraTool's HMAC-SHA256 signature scheme)
//   - Automatic endpoint routing (legacy vs open)
//   - Retry logic with exponential backoff
//   - Response decoding (JSON + encrypted binary payloads)

use anyhow::{anyhow, Result};
use log::warn;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};

type HmacSha256 = Hmac<Sha256>;

/// Base URLs for all ChimeraTool service endpoints
pub mod base_url {
    pub const API:        &str = "https://api.chimeratool.com";
    pub const SECURE:     &str = "https://secure.chimeratool.com";
    pub const DATA:       &str = "https://data.chimeratool.com";
    pub const UPLOAD:     &str = "https://upload.chimeratool.com";
    pub const PICS:       &str = "https://pics.chimeratool.com";
    pub const PORTCHECK:  &str = "https://portcheck.chimeratool.com";
    pub const BB:         &str = "https://bb.chimeratool.com";

    // Open replacement base URLs (used when `use_open_alternatives = true`)
    pub const OPEN_FIRMWARE_APPLE:   &str = "https://api.ipsw.me/v4";
    pub const OPEN_FIRMWARE_SAMSUNG: &str = "https://api.samfw.com";
    pub const OPEN_IMEI_CHECK:       &str = "https://api.imeicheck.net/v1";
}

/// Known API path patterns observed/inferred from ChimeraTool binary analysis
pub mod api_paths {
    // ── api.chimeratool.com ──────────────────────────────────────────────
    pub const AUTH_LOGIN:          &str = "/v2/auth/login";
    pub const AUTH_REFRESH:        &str = "/v2/auth/refresh";
    pub const USER_PROFILE:        &str = "/v2/user/profile";
    pub const USER_CREDITS:        &str = "/v2/user/credits";
    pub const DEVICE_REGISTER:     &str = "/v2/device/register";
    pub const DEVICE_LIST:         &str = "/v2/device/list";
    pub const OP_DISPATCH:         &str = "/v2/operation/dispatch";
    pub const OP_STATUS:           &str = "/v2/operation/status/{id}";
    pub const OP_HISTORY:          &str = "/v2/operation/history";
    pub const LICENSE_CHECK:       &str = "/v2/license/check";
    pub const LICENSE_ACTIVATE:    &str = "/v2/license/activate";
    pub const UPDATE_CHECK:        &str = "/v2/update/check";
    pub const UPDATE_DOWNLOAD:     &str = "/v2/update/download";
    // ── secure.chimeratool.com ───────────────────────────────────────────
    pub const SECURE_CERT_READ:    &str = "/v1/cert/read";
    pub const SECURE_CERT_WRITE:   &str = "/v1/cert/write";
    pub const SECURE_IMEI_CHECK:   &str = "/v1/imei/check";
    pub const SECURE_IMEI_REPAIR:  &str = "/v1/imei/repair";
    pub const SECURE_FRP_TICKET:   &str = "/v1/frp/ticket";
    pub const SECURE_KNOX_TICKET:  &str = "/v1/knox/ticket";
    pub const SECURE_NCK:          &str = "/v1/nck/calculate";
    pub const SECURE_SAMSUNG_EFS:  &str = "/v1/samsung/efs";
    pub const SECURE_APPLE_ACT:    &str = "/v1/apple/activation";
    // ── data.chimeratool.com ─────────────────────────────────────────────
    pub const DATA_MODELS:         &str = "/v1/models/{brand}";
    pub const DATA_FIRMWARE:       &str = "/v1/firmware/{brand}/{model}";
    pub const DATA_FW_DOWNLOAD:    &str = "/v1/firmware/download/{id}";
    pub const DATA_CHANGELOG:      &str = "/v1/changelog";
    pub const DATA_SUPPORT_MATRIX: &str = "/v1/support/matrix";
    // ── upload.chimeratool.com ───────────────────────────────────────────
    pub const UPLOAD_FILE:         &str = "/v1/upload";
    pub const UPLOAD_DIAGNOSTIC:   &str = "/v1/diagnostic";
    // ── pics.chimeratool.com ─────────────────────────────────────────────
    pub const PICS_DEVICE:         &str = "/devices/{model}.jpg";
    pub const PICS_BRAND:          &str = "/brands/{brand}.png";
    // ── portcheck.chimeratool.com ────────────────────────────────────────
    pub const PORTCHECK_TCP:       &str = "/check?host={host}&port={port}&proto=tcp";
    pub const PORTCHECK_ADB:       &str = "/adb?host={host}&port=5555";
}

/// Standard request headers observed in ChimeraTool traffic
pub mod ct_headers {
    pub const APP_VERSION:  &str = "X-Chimera-Version";
    pub const APP_BUILD:    &str = "X-Chimera-Build";
    pub const DEVICE_TOKEN: &str = "X-Device-Token";
    pub const REQUEST_ID:   &str = "X-Request-Id";
    pub const SIGNATURE:    &str = "X-Signature";
    pub const TIMESTAMP:    &str = "X-Timestamp";
    pub const CLIENT_ID:    &str = "X-Client-Id";
    pub const SESSION_TOKEN:&str = "Authorization";
}

/// ChimeraTool API request signature (HMAC-SHA256 scheme)
/// Derived from observed request patterns. Signing key appears to be a
/// per-installation secret derived from hardware fingerprint + app key.
pub fn sign_request(
    method: &str,
    path: &str,
    body: &[u8],
    timestamp: u64,
    signing_key: &[u8],
) -> String {
    // Observed signing string format:
    //   METHOD\n PATH\n TIMESTAMP\n SHA256(BODY)
    let body_hash = hex::encode(Sha256::digest(body));
    let signing_string = format!("{}\n{}\n{}\n{}", method.to_uppercase(), path, timestamp, body_hash);

    let mut mac = HmacSha256::new_from_slice(signing_key)
        .expect("HMAC key init failed");
    mac.update(signing_string.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Standard JSON response envelope from api.chimeratool.com
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub code: i32,
    pub message: Option<String>,
    pub data: Option<T>,
    pub credits_used: Option<u32>,
    pub credits_remaining: Option<u32>,
    pub request_id: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn is_ok(&self) -> bool { self.success && self.code == 200 }
    pub fn error_msg(&self) -> String {
        self.message.clone().unwrap_or_else(|| format!("Error code {}", self.code))
    }
}

/// Known API error codes from ChimeraTool
pub mod error_code {
    pub const OK:                   i32 = 200;
    pub const UNAUTHORIZED:         i32 = 401;
    pub const INSUFFICIENT_CREDITS: i32 = 402;
    pub const FORBIDDEN:            i32 = 403;
    pub const NOT_FOUND:            i32 = 404;
    pub const DEVICE_UNSUPPORTED:   i32 = 422;
    pub const RATE_LIMITED:         i32 = 429;
    pub const SERVER_ERROR:         i32 = 500;
    pub const LICENSE_EXPIRED:      i32 = 4001;
    pub const LICENSE_SUSPENDED:    i32 = 4002;
    pub const CREDIT_REQUIRED:      i32 = 4010;
    pub const OPERATION_FAILED:     i32 = 5001;
    pub const DEVICE_OFFLINE:       i32 = 5002;
    pub const TIMEOUT:              i32 = 5003;
}

/// Main API client — routes requests to open alternatives by default
pub struct ApiClient {
    http: reqwest::Client,
    /// If true: attempt ChimeraTool endpoints (for compatibility research only)
    /// If false (default): use open/local alternatives for all operations
    use_legacy_endpoints: bool,
    /// Version string sent in X-App-Version header.

    pub app_version: String,
    /// HMAC-SHA256 key for request signing.

    pub signing_key: Vec<u8>,
}

impl ApiClient {
    /// Create a client configured to use ONLY open alternatives (default)
    pub fn open() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ChimeraRS/1.2.0 (open-source)")
                .build()
                .expect("HTTP client build failed"),
            use_legacy_endpoints: false,
            app_version: "1.2.0".into(),
            signing_key: vec![],
        }
    }

    /// Create a client for legacy endpoint research (NOT for production use)
    #[doc(hidden)]
    pub fn legacy_research(signing_key: Vec<u8>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ChimeraTool/3.0.0")
                .build()
                .expect("HTTP client build failed"),
            use_legacy_endpoints: true,
            app_version: "3.0.0".into(),
            signing_key,
        }
    }

    /// GET request with automatic retry (3 attempts, exponential backoff)
    pub async fn get<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let mut attempts = 0u32;
        loop {
            attempts += 1;
            match self.http.get(url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return Ok(resp.json::<T>().await?);
                    }
                    if status.as_u16() == 429 && attempts < 3 {
                        let wait = Duration::from_millis(500 * 2u64.pow(attempts));
                        tokio::time::sleep(wait).await;
                        continue;
                    }
                    return Err(anyhow!("HTTP {} from {}", status, url));
                }
                Err(e) if attempts < 3 => {
                    let wait = Duration::from_millis(500 * 2u64.pow(attempts));
                    warn!("Request failed (attempt {}): {}. Retrying…", attempts, e);
                    tokio::time::sleep(wait).await;
                }
                Err(e) => return Err(anyhow!("HTTP GET failed after {} attempts: {}", attempts, e)),
            }
        }
    }

    /// POST JSON request
    pub async fn post_json<Req: Serialize, Res: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        body: &Req,
    ) -> Result<Res> {
        let resp = self.http.post(url)
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() {
            Ok(resp.json::<Res>().await?)
        } else {
            Err(anyhow!("HTTP {} from POST {}", status, url))
        }
    }

    pub fn is_open_mode(&self) -> bool { !self.use_legacy_endpoints }
}
