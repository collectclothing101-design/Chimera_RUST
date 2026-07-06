//! `/tenant/{t}/activation` — activation code generation + redemption tracking.
//!
//! **Why this matters**: in a fleet workflow the activation code is the
//! single secret you push to the device through your EMM. The PTT Pro
//! client redeems it on first launch and exchanges it for a long-lived
//! device token.
//!
//! Typical flow:
//!
//! 1. IT admin enrolls device serial in PTT Pro tenant
//! 2. IT admin assigns the device to a user
//! 3. IT admin calls `generate(user_id, device_serial)` → activation code
//! 4. The code goes into the EMM as a per-device managed-config value
//! 5. EMM pushes the PTT Pro client + config to the device
//! 6. Client reads its config, posts the code to PTT Pro's redemption
//!    endpoint, receives the device token, stores it
//! 7. Subsequent PTT calls authenticate with the device token

use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::{Client, Result, ActivationCode};

pub struct ActivationApi<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> ActivationApi<'a> {
    /// Generate a new activation code for `user_id` + `device_serial`.
    ///
    /// The code is single-use; once redeemed the client exchanges it for
    /// a long-lived device token and the code is marked expired.
    pub async fn generate(&self, user_id: &Uuid, device_serial: &str) -> Result<ActivationCode> {
        if device_serial.trim().is_empty() {
            return Err(crate::Error::InvalidInput("device serial is empty".into()));
        }
        let url = self.client.url("activation/codes")?;
        let body = serde_json::json!({
            "userId":       user_id,
            "deviceSerial": device_serial,
        });
        self.client.execute(move |http, _| http.post(url.clone()).json(&body)).await
    }

    /// Generate an activation code with a custom expiry window (default
    /// is 72 h server-side).
    pub async fn generate_with_expiry(
        &self,
        user_id:       &Uuid,
        device_serial: &str,
        expires_at:    DateTime<Utc>,
    ) -> Result<ActivationCode> {
        if device_serial.trim().is_empty() {
            return Err(crate::Error::InvalidInput("device serial is empty".into()));
        }
        let url  = self.client.url("activation/codes")?;
        let body = serde_json::json!({
            "userId":       user_id,
            "deviceSerial": device_serial,
            "expiresAt":    expires_at.to_rfc3339(),
        });
        self.client.execute(move |http, _| http.post(url.clone()).json(&body)).await
    }

    /// Bulk-generate codes for many (user, device) pairs in one request.
    /// Server allocates them atomically.
    pub async fn generate_bulk(
        &self,
        pairs: &[(Uuid, String)],
    ) -> Result<Vec<ActivationCode>> {
        if pairs.is_empty() {
            return Err(crate::Error::InvalidInput("no pairs supplied".into()));
        }
        let url = self.client.url("activation/codes/bulk")?;
        let entries: Vec<serde_json::Value> = pairs.iter().map(|(uid, dev)| {
            serde_json::json!({ "userId": uid, "deviceSerial": dev })
        }).collect();
        let body = serde_json::json!({ "entries": entries });

        #[derive(serde::Deserialize)]
        struct Response { codes: Vec<ActivationCode> }
        let r: Response = self.client.execute(
            move |http, _| http.post(url.clone()).json(&body)
        ).await?;
        Ok(r.codes)
    }

    /// Look up an existing code by its string value.
    pub async fn get(&self, code: &str) -> Result<ActivationCode> {
        let p = format!("activation/codes/{}", urlencoding::encode(code));
        self.client.execute(|http, _| http.get(self.client.url(&p).expect("url"))).await
    }

    /// Revoke a code before it's redeemed (e.g. when a device is lost).
    pub async fn revoke(&self, code: &str) -> Result<()> {
        let p = format!("activation/codes/{}", urlencoding::encode(code));
        let url = self.client.url(&p)?;
        let _: serde_json::Value = self.client.execute(
            move |http, _| http.delete(url.clone())
        ).await?;
        Ok(())
    }

    /// List every code emitted for a user (typically 1 outstanding at a time).
    pub async fn list_for_user(&self, user_id: &Uuid) -> Result<Vec<ActivationCode>> {
        let p = format!("users/{}/activation/codes", user_id);
        #[derive(serde::Deserialize)]
        struct Wrap { items: Vec<ActivationCode> }
        let w: Wrap = self.client.execute(
            |http, _| http.get(self.client.url(&p).expect("url"))
        ).await?;
        Ok(w.items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_rejects_empty_serial() {
        let c = Client::new("https://api.example.com", "acme").unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(c.activation().generate(&Uuid::nil(), "")).err().unwrap();
        assert!(matches!(err, crate::Error::InvalidInput(_)));
    }

    #[test]
    fn bulk_rejects_empty_input() {
        let c = Client::new("https://api.example.com", "acme").unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(c.activation().generate_bulk(&[])).err().unwrap();
        assert!(matches!(err, crate::Error::InvalidInput(_)));
    }
}
