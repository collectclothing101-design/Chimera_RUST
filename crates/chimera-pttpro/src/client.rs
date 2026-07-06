//! HTTP client with OAuth2 bearer auth, automatic refresh, and
//! exponential-backoff retries on rate-limit / 5xx responses.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::{Error, Result};

/// Default API base URL for Zebra's hosted PTT Pro service.
pub const DEFAULT_BASE: &str = "https://api.ptt.zebra.com";

/// Per-environment auth strategy.
///
/// PTT Pro accepts two flows:
///
/// 1. **OAuth2 client-credentials** — for server-to-server automation.
///    The "right" answer for fleet provisioning.
///
/// 2. **Static bearer token** — for short-lived scripts or test fixtures.
///    Token comes from the management portal and lasts ~24 h.
#[derive(Debug, Clone)]
pub enum Credentials {
    /// `client_credentials` OAuth2 flow. The client posts client_id +
    /// client_secret to `/oauth2/token` and receives a bearer JWT good
    /// for `expires_in` seconds.
    ClientCredentials {
        client_id:     String,
        client_secret: String,
        token_url:     Option<String>,
        scopes:        Vec<String>,
    },
    /// Pre-issued static bearer. No refresh logic — caller is responsible.
    Bearer { token: String },
}

impl Credentials {
    pub fn client_credentials(client_id: impl Into<String>,
                              client_secret: impl Into<String>) -> Self
    {
        Credentials::ClientCredentials {
            client_id:     client_id.into(),
            client_secret: client_secret.into(),
            token_url:     None,
            scopes:        vec!["api".into()],
        }
    }
    pub fn bearer(token: impl Into<String>) -> Self {
        Credentials::Bearer { token: token.into() }
    }
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        if let Credentials::ClientCredentials { scopes: s, .. } = &mut self {
            *s = scopes;
        }
        self
    }
    pub fn with_token_url(mut self, url: impl Into<String>) -> Self {
        if let Credentials::ClientCredentials { token_url, .. } = &mut self {
            *token_url = Some(url.into());
        }
        self
    }
}

/// `/oauth2/token` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default = "default_token_type")]
    pub token_type:   String,
    pub expires_in:   u64,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope:        Option<String>,
}
fn default_token_type() -> String { "Bearer".into() }

#[derive(Debug, Clone)]
struct ActiveToken {
    bearer:   String,
    expires:  DateTime<Utc>,
}

/// Retry / backoff configuration. Default: up to 4 retries with
/// 500ms, 1s, 2s, 4s waits.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries:        u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms:     u64,
    pub timeout:            Duration,
}
impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries:        4,
            initial_backoff_ms: 500,
            max_backoff_ms:     30_000,
            timeout:            Duration::from_secs(30),
        }
    }
}

/// Main PTT Pro API client.
///
/// Cheap to clone — wraps an `Arc<>` to shared state under the hood.
#[derive(Debug, Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    http:        reqwest::Client,
    base_url:    url::Url,
    tenant:      String,
    creds:       RwLock<Option<Credentials>>,
    token:       RwLock<Option<ActiveToken>>,
    retry:       RetryPolicy,
}

impl Client {
    /// Build a client for `base_url` (typically `DEFAULT_BASE`) + `tenant`
    /// (your company slug, assigned by Zebra).
    pub fn new(base_url: impl AsRef<str>, tenant: impl Into<String>) -> Result<Self> {
        let base = url::Url::parse(base_url.as_ref())?;
        let http = reqwest::Client::builder()
            .user_agent(concat!("chimera-pttpro/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Network(format!("build http client: {}", e)))?;
        Ok(Self {
            inner: Arc::new(Inner {
                http,
                base_url: base,
                tenant:   tenant.into(),
                creds:    RwLock::new(None),
                token:    RwLock::new(None),
                retry:    RetryPolicy::default(),
            }),
        })
    }

    /// Attach credentials. Required before calling any authenticated
    /// endpoint. Builder-style — returns `Self` for chaining.
    pub fn with_credentials(self, creds: Credentials) -> Self {
        // Best-effort sync write — the lock is uncontested at this point.
        if let Ok(mut g) = self.inner.creds.try_write() { *g = Some(creds); }
        self
    }

    /// Override retry policy.
    pub fn with_retry_policy(self, policy: RetryPolicy) -> Self {
        // Replace `inner` with a fresh Arc that has the new retry policy.
        let old = self.inner;
        let new = Inner {
            http:     old.http.clone(),
            base_url: old.base_url.clone(),
            tenant:   old.tenant.clone(),
            creds:    RwLock::new(old.creds.try_read().ok().and_then(|g| g.clone())),
            token:    RwLock::new(old.token.try_read().ok().and_then(|g| g.clone())),
            retry:    policy,
        };
        Self { inner: Arc::new(new) }
    }

    /// Tenant slug this client targets.
    pub fn tenant(&self) -> &str { &self.inner.tenant }

    /// Base URL (parent of every tenant-scoped path).
    pub fn base_url(&self) -> &url::Url { &self.inner.base_url }

    // ─── Endpoint scope accessors ─────────────────────────────────────

    /// `/users` endpoint surface.
    pub fn users(&self) -> crate::users::UsersApi<'_> {
        crate::users::UsersApi { client: self }
    }
    /// `/groups` endpoint surface.
    pub fn groups(&self) -> crate::groups::GroupsApi<'_> {
        crate::groups::GroupsApi { client: self }
    }
    /// `/contacts` endpoint surface.
    pub fn contacts(&self) -> crate::contacts::ContactsApi<'_> {
        crate::contacts::ContactsApi { client: self }
    }
    /// `/departments` endpoint surface.
    pub fn departments(&self) -> crate::departments::DepartmentsApi<'_> {
        crate::departments::DepartmentsApi { client: self }
    }
    /// `/devices` endpoint surface.
    pub fn devices(&self) -> crate::devices::DevicesApi<'_> {
        crate::devices::DevicesApi { client: self }
    }
    /// `/activation` endpoint surface.
    pub fn activation(&self) -> crate::activation::ActivationApi<'_> {
        crate::activation::ActivationApi { client: self }
    }

    // ─── Auth flow ────────────────────────────────────────────────────

    /// Force a token exchange. Normally not needed — the request path
    /// authenticates lazily.
    pub async fn authenticate(&self) -> Result<()> {
        let creds = {
            let g = self.inner.creds.read().await;
            g.clone().ok_or(Error::NoCredentials)?
        };
        match creds {
            Credentials::Bearer { token } => {
                let mut g = self.inner.token.write().await;
                *g = Some(ActiveToken {
                    bearer:  token,
                    expires: Utc::now() + chrono::Duration::days(30),
                });
                Ok(())
            }
            Credentials::ClientCredentials {
                client_id, client_secret, token_url, scopes
            } => {
                let url = token_url.unwrap_or_else(|| {
                    let mut u = self.inner.base_url.clone();
                    u.set_path("/oauth2/token");
                    u.to_string()
                });
                let scope_str = scopes.join(" ");
                let form: Vec<(&str, &str)> = vec![
                    ("grant_type",    "client_credentials"),
                    ("client_id",     &client_id),
                    ("client_secret", &client_secret),
                    ("scope",         &scope_str),
                ];
                let resp = self.inner.http.post(&url).form(&form).send().await?;
                let status = resp.status();
                if !status.is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    return Err(Error::AuthFailed(format!("{}: {}", status, body)));
                }
                let tok: TokenResponse = resp.json().await?;
                let bearer  = tok.access_token.clone();
                let expires = Utc::now()
                    + chrono::Duration::seconds(tok.expires_in.saturating_sub(60) as i64);
                let mut g = self.inner.token.write().await;
                *g = Some(ActiveToken { bearer, expires });
                Ok(())
            }
        }
    }

    /// Get a valid bearer, refreshing if expired.
    async fn ensure_bearer(&self) -> Result<String> {
        // Fast path: read-locked check
        {
            let g = self.inner.token.read().await;
            if let Some(t) = g.as_ref() {
                if t.expires > Utc::now() + chrono::Duration::seconds(30) {
                    return Ok(t.bearer.clone());
                }
            }
        }
        // Slow path: refresh
        self.authenticate().await?;
        let g = self.inner.token.read().await;
        g.as_ref()
            .map(|t| t.bearer.clone())
            .ok_or_else(|| Error::TokenRefreshFailed("token missing after auth".into()))
    }

    // ─── Request execution ────────────────────────────────────────────

    /// Build a tenant-scoped URL: base + `/tenant/<tenant>/<path>`.
    pub(crate) fn url(&self, path: &str) -> Result<url::Url> {
        let p = path.trim_start_matches('/');
        let full = format!("/tenant/{}/{}", self.inner.tenant, p);
        let mut u = self.inner.base_url.clone();
        u.set_path(&full);
        Ok(u)
    }

    /// Execute an authenticated request with retry/backoff. The closure
    /// receives a freshly-built reqwest builder so it can attach query
    /// strings, JSON bodies, multipart, etc.
    pub(crate) async fn execute<F, T>(&self, mut build: F) -> Result<T>
    where
        F: FnMut(&reqwest::Client, &str) -> reqwest::RequestBuilder,
        T: serde::de::DeserializeOwned,
    {
        let mut attempt: u32 = 0;
        loop {
            let bearer = self.ensure_bearer().await?;
            let req = build(&self.inner.http, &bearer)
                .bearer_auth(&bearer);
            let resp = req.send().await;
            let result = match resp {
                Ok(r)  => classify(r).await,
                Err(e) => Err(Error::from(e)),
            };

            match result {
                Ok(value) => return Ok(value),
                Err(e) if e.is_retryable() && attempt < self.inner.retry.max_retries => {
                    let wait_ms = e.retry_after_secs()
                        .map(|s| s * 1000)
                        .unwrap_or_else(|| {
                            let base = self.inner.retry.initial_backoff_ms;
                            let pow  = 1u64 << attempt;
                            (base * pow).min(self.inner.retry.max_backoff_ms)
                        });
                    tracing::warn!(
                        attempt, wait_ms, error = %e,
                        "pttpro retry"
                    );
                    tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                    attempt += 1;
                    continue;
                }
                Err(Error::Unauthorized(_)) if attempt == 0 => {
                    // Token may have been revoked early — force a refresh
                    // and try one more time.
                    self.invalidate_token().await;
                    attempt += 1;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn invalidate_token(&self) {
        let mut g = self.inner.token.write().await;
        *g = None;
    }
}

/// Classify an HTTP response into typed success / error, deserialising
/// the success body into `T`.
async fn classify<T: serde::de::DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
    let status = resp.status();
    if status.is_success() {
        // 204 No Content → return unit if T is unit
        if status == reqwest::StatusCode::NO_CONTENT {
            let v = serde_json::from_str::<T>("null")
                .or_else(|_| serde_json::from_str::<T>("{}"));
            return v.map_err(Error::from);
        }
        let body = resp.text().await?;
        return serde_json::from_str::<T>(&body).map_err(Error::from);
    }
    let retry_after = resp.headers().get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    let body = resp.text().await.unwrap_or_default();
    Err(match status.as_u16() {
        401 => Error::Unauthorized(body),
        403 => Error::Forbidden(body),
        404 => Error::NotFound(body),
        409 => Error::Conflict(body),
        429 => Error::RateLimited { retry_after_secs: retry_after },
        s if (500..600).contains(&s) => Error::ServerError { status: s, body },
        s => Error::ClientError { status: s, body },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_constructor_handles_leading_slash() {
        let c = Client::new("https://api.example.com", "acme").unwrap();
        let u = c.url("users").unwrap();
        assert!(u.path().contains("/tenant/acme/users"));
        let u = c.url("/users").unwrap();
        assert!(u.path().contains("/tenant/acme/users"));
    }

    #[test]
    fn credentials_builder_chains() {
        let c = Credentials::client_credentials("cid", "secret")
            .with_scopes(vec!["read".into(), "write".into()])
            .with_token_url("https://auth.example.com/oauth2/token");
        match c {
            Credentials::ClientCredentials { scopes, token_url, .. } => {
                assert_eq!(scopes.len(), 2);
                assert!(token_url.unwrap().contains("auth.example.com"));
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn retry_policy_defaults_sensible() {
        let p = RetryPolicy::default();
        assert!(p.max_retries >= 3);
        assert!(p.initial_backoff_ms > 0);
        assert!(p.max_backoff_ms >= p.initial_backoff_ms);
    }

    #[test]
    fn client_clones_share_state() {
        let c1 = Client::new("https://x.test", "t").unwrap();
        let c2 = c1.clone();
        assert_eq!(c1.tenant(), c2.tenant());
        assert!(Arc::ptr_eq(&c1.inner, &c2.inner));
    }

    #[test]
    fn token_response_default_token_type() {
        let json = r#"{"access_token":"abc","expires_in":3600}"#;
        let t: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(t.token_type, "Bearer");
    }
}
