//! Typed errors for the PTT Pro client.
//!
//! Designed so callers can pattern-match on auth failures, rate limits,
//! and 5xx server errors separately from network or deserialisation
//! problems.

use thiserror::Error;

/// Crate-wide `Result` alias.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// No credentials configured before a request that needs auth.
    #[error("no credentials configured — call .with_credentials() first")]
    NoCredentials,

    /// OAuth2 token-exchange call failed.
    #[error("authentication failed: {0}")]
    AuthFailed(String),

    /// Token expired and refresh also failed.
    #[error("token refresh failed: {0}")]
    TokenRefreshFailed(String),

    /// HTTP request returned 4xx (other than 401/429).
    #[error("client error {status}: {body}")]
    ClientError { status: u16, body: String },

    /// HTTP 401 — token rejected even after refresh.
    #[error("unauthorized (401): {0}")]
    Unauthorized(String),

    /// HTTP 403 — token valid but lacks permission.
    #[error("forbidden (403): {0}")]
    Forbidden(String),

    /// HTTP 404 — resource missing.
    #[error("not found (404): {0}")]
    NotFound(String),

    /// HTTP 409 — usually duplicate-name on a create.
    #[error("conflict (409): {0}")]
    Conflict(String),

    /// HTTP 429 — rate limit. `retry_after_secs` if `Retry-After` header
    /// was parseable.
    #[error("rate limited (429), retry after {retry_after_secs:?} s")]
    RateLimited { retry_after_secs: Option<u64> },

    /// HTTP 5xx — server-side error. Caller usually retries with backoff.
    #[error("server error {status}: {body}")]
    ServerError { status: u16, body: String },

    /// Network-layer problem (DNS, TCP, TLS, timeout).
    #[error("network: {0}")]
    Network(String),

    /// Couldn't deserialise the response into the expected model.
    #[error("deserialization: {0}")]
    Deserialization(String),

    /// Couldn't serialise a request body.
    #[error("serialization: {0}")]
    Serialization(String),

    /// Bad URL given to the constructor.
    #[error("url: {0}")]
    Url(String),

    /// Bad input — e.g. validation on an email or empty group name.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Catch-all.
    #[error("other: {0}")]
    Other(String),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout()    { Error::Network(format!("timeout: {}", e)) }
        else if e.is_connect()    { Error::Network(format!("connect: {}", e)) }
        else if e.is_decode()     { Error::Deserialization(e.to_string()) }
        else if e.is_body()       { Error::Network(format!("body: {}", e)) }
        else                      { Error::Network(e.to_string()) }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        if e.is_io() || e.is_eof() { Error::Network(e.to_string()) }
        else { Error::Deserialization(e.to_string()) }
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self { Error::Url(e.to_string()) }
}

impl Error {
    /// True when the request should be retried after backoff.
    pub fn is_retryable(&self) -> bool {
        matches!(self,
            Error::RateLimited { .. } |
            Error::ServerError { .. } |
            Error::Network(_)
        )
    }

    /// Suggested seconds to wait before retrying, if available.
    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            Error::RateLimited { retry_after_secs } => *retry_after_secs,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retryable_classification() {
        assert!(Error::RateLimited { retry_after_secs: Some(5) }.is_retryable());
        assert!(Error::ServerError { status: 503, body: "x".into() }.is_retryable());
        assert!(Error::Network("dns failed".into()).is_retryable());
        assert!(!Error::Unauthorized("bad token".into()).is_retryable());
        assert!(!Error::NotFound("user 7".into()).is_retryable());
    }

    #[test]
    fn rate_limit_carries_retry_after() {
        let e = Error::RateLimited { retry_after_secs: Some(30) };
        assert_eq!(e.retry_after_secs(), Some(30));
    }

    #[test]
    fn display_strings_nonempty() {
        let e = Error::Unauthorized("x".into());
        assert!(!format!("{}", e).is_empty());
    }
}
