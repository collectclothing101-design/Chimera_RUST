// chimera-api/src/auth.rs
// ChimeraTool authentication protocol — reverse-engineered structure.
// In ChimeraRS we bypass this entirely. This module exists solely to document
// the protocol and provide compatibility shims if ever needed.
//
// Observed auth flow:
//   1. POST /v2/auth/login  {email, password_sha256, machine_id, app_version}
//      → {token, refresh_token, user_id, credits, expiry}
//   2. All subsequent requests: Authorization: Bearer {token}
//   3. POST /v2/auth/refresh {refresh_token} → {token, expiry}
//   4. Credits deducted per operation. Types: "basic"(1), "advanced"(3), "premium"(10)
//
// ChimeraRS replaces ALL of this with local operations. No account needed.

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// ChimeraTool login request (reverse-engineered)
#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password_sha256: String,
    pub machine_id: String,
    pub app_version: String,
    pub platform: String,    // "macos", "linux", "windows" — ChimeraRS always reports "macos"
    pub build: u32,
}

/// ChimeraTool login response (reverse-engineered)
#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub refresh_token: String,
    pub user_id: u64,
    pub username: String,
    pub credits: u32,
    pub expiry: u64,   // Unix timestamp
    pub features: Vec<String>,
}

/// Credit costs per operation type (reverse-engineered from binary analysis)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperationCreditCost {
    Free,        // 0 credits – basic info reads
    Basic,       // 1 credit  – FRP, factory reset
    Standard,    // 3 credits – IMEI repair, screen lock
    Advanced,    // 5 credits – certificate ops, EFS
    Premium,     // 10 credits – network unlock, full flash
    Custom(u32), // variable
}

impl OperationCreditCost {
    pub fn credits(&self) -> u32 {
        match self {
            OperationCreditCost::Free        => 0,
            OperationCreditCost::Basic       => 1,
            OperationCreditCost::Standard    => 3,
            OperationCreditCost::Advanced    => 5,
            OperationCreditCost::Premium     => 10,
            OperationCreditCost::Custom(n)   => *n,
        }
    }

    /// In ChimeraRS: ALL operations cost 0 credits.
    pub fn chimera_rs_cost(&self) -> u32 { 0 }
}

/// Machine ID derivation (hash of hardware identifiers, used in login)
/// ChimeraTool uses this to bind a session to a specific PC.
pub fn derive_machine_id() -> String {
    
    

    // In production ChimeraTool: reads CPU ID, HDD serial, MAC, hostname
    // and SHA256-hashes them. We generate a stable random ID instead.
    let mut hasher = Sha256::new();
    hasher.update(b"ChimeraRS-open-source-no-machine-binding");
    // Add hostname for some per-machine stability
    // macOS / POSIX: hostname is in $HOSTNAME or via uname(1).
    // COMPUTERNAME is Windows-only — removed.
    let hostname = std::env::var("HOSTNAME")
        .ok()
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "ChimeraRS-macOS".into());
    hasher.update(hostname.as_bytes());
    hex::encode(hasher.finalize())
}

/// ChimeraRS auth bypass — returns a fake "always authenticated" session
/// that requires no server communication.
pub struct LocalSession {
    pub user_id: u64,
    pub username: String,
    pub credits: u32,
}

impl LocalSession {
    pub fn new() -> Self {
        Self {
            user_id: 0,
            username: "local-user".into(),
            credits: u32::MAX, // unlimited
        }
    }
    pub fn is_authenticated(&self) -> bool { true }
    pub fn has_credits_for(&self, _op: OperationCreditCost) -> bool { true }
    pub fn deduct_credits(&mut self, _op: OperationCreditCost) { /* no-op */ }
}

impl Default for LocalSession {
    fn default() -> Self { Self::new() }
}
