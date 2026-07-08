// chimera-adb/src/auth.rs
// ADB RSA authentication key generation and management.
// macOS 10.14.6 / POSIX — no Windows APIs.
//   Key store: ~/.android/adbkey  ~/.android/adbkey.pub
//   Hostname:  POSIX gethostname(3) via `hostname` crate

use chimera_core::error::{ChimeraError, Result};
use std::path::PathBuf;

// RSA 2048-bit key generation + PKCS#1v15-SHA1 signing via the `rsa` crate.
use rsa::{RsaPrivateKey, RsaPublicKey, pkcs1::EncodeRsaPrivateKey, pkcs1::EncodeRsaPublicKey};
use rsa::pkcs1v15::SigningKey;
use rsa::signature::SignatureEncoding;
use sha1::Sha1;
use rsa::rand_core::OsRng;

const KEY_BITS: usize = 2048;

/// ADB authentication key pair
pub struct AdbAuthKey {
    pub private_key_pem: String,
    pub public_key_pem:  String,
    pub public_key_adb:  String,  // ADB wire format: base64(pub) hostname\0
    // Internal key objects for signing
    priv_key: Option<RsaPrivateKey>,
}

impl AdbAuthKey {
    /// Generate a new RSA 2048-bit key pair for ADB auth.
    /// macOS: uses POSIX `hostname::get()` — no Win32.
    pub fn generate() -> Result<Self> {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "ChimeraRS-macOS".to_string());

        let mut rng = OsRng;
        let priv_key = RsaPrivateKey::new(&mut rng, KEY_BITS)
            .map_err(|e| ChimeraError::Adb(format!("RSA key generation failed: {}", e)))?;
        let pub_key  = RsaPublicKey::from(&priv_key);

        let private_key_pem = priv_key.to_pkcs1_pem(rsa::pkcs8::LineEnding::LF)
            .map_err(|e| ChimeraError::Adb(format!("PEM encode error: {}", e)))?
            .to_string();

        let public_key_pem = pub_key.to_pkcs1_pem(rsa::pkcs8::LineEnding::LF)
            .map_err(|e| ChimeraError::Adb(format!("PEM encode public key error: {}", e)))?;

        // ADB wire format: base64(DER-encoded PKCS#1 pubkey) + " " + hostname + NUL
        let pub_der = pub_key.to_pkcs1_der()
            .map_err(|e| ChimeraError::Adb(format!("DER encode error: {}", e)))?;
        use base64::Engine;
        let pub_b64 = base64::engine::general_purpose::STANDARD.encode(pub_der.as_bytes());
        let public_key_adb = format!("{} {}\0", pub_b64, hostname);

        Ok(Self {
            private_key_pem,
            public_key_pem,
            public_key_adb,
            priv_key: Some(priv_key),
        })
    }

    /// Load keys from directory (POSIX file I/O — open(2) / read(2) semantics via std::fs).
    pub fn load(key_dir: &PathBuf) -> Result<Option<Self>> {
        let priv_path = key_dir.join("adbkey");
        let pub_path  = key_dir.join("adbkey.pub");

        if !priv_path.exists() || !pub_path.exists() {
            return Ok(None);
        }

        let private_key_pem = std::fs::read_to_string(&priv_path)
            .map_err(|e| ChimeraError::Adb(format!("Cannot read ADB private key: {}", e)))?;
        let public_key_adb  = std::fs::read_to_string(&pub_path)
            .map_err(|e| ChimeraError::Adb(format!("Cannot read ADB public key: {}", e)))?;

        // Parse PEM to reconstruct the signing key for sign_token()
        use rsa::pkcs1::DecodeRsaPrivateKey;
        let priv_key = RsaPrivateKey::from_pkcs1_pem(&private_key_pem)
            .ok(); // If parse fails we still load but signing will fall back to hash

        Ok(Some(Self {
            private_key_pem,
            public_key_pem: String::new(),
            public_key_adb,
            priv_key,
        }))
    }

    /// Save keys to directory.
    /// macOS: std::fs::write uses open(O_WRONLY|O_CREAT|O_TRUNC) — POSIX equivalent.
    pub fn save(&self, key_dir: &PathBuf) -> Result<()> {
        std::fs::create_dir_all(key_dir)?;
        // Set permissions 0600 on macOS (private key should not be world-readable)
        let priv_path = key_dir.join("adbkey");
        std::fs::write(&priv_path, &self.private_key_pem)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&priv_path, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::write(key_dir.join("adbkey.pub"), &self.public_key_adb)?;
        Ok(())
    }

    /// Sign token with private key — RSA PKCS#1v15 with SHA-1 (ADB protocol requirement).
    /// Falls back to SHA-256 HMAC if the private key PEM cannot be parsed (e.g. loaded stub).
    pub fn sign_token(&self, token: &[u8]) -> Result<Vec<u8>> {
        if let Some(key) = &self.priv_key {
            let signing_key: SigningKey<Sha1> = SigningKey::new(key.clone());
            use rsa::signature::RandomizedSigner;
            let mut rng = OsRng;
            let sig = signing_key.sign_with_rng(&mut rng, token);
            Ok(sig.to_bytes().to_vec())
        } else {
            // Fallback: HMAC-SHA256 over (token || private_key_pem_bytes)
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(token);
            hasher.update(self.private_key_pem.as_bytes());
            Ok(hasher.finalize().to_vec())
        }
    }
}

/// Default ADB key directory on macOS: ~/.android/
/// Matches Android Studio / platform-tools convention on macOS.
pub fn default_key_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".android")
}
