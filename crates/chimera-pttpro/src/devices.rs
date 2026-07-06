//! `/tenant/{t}/devices` — fleet enrollment + lifecycle.

use crate::{Client, Result, Device, NewDevice, Page};
use crate::models::{ListFilter, DeviceState};

pub struct DevicesApi<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> DevicesApi<'a> {
    /// List enrolled devices.
    pub async fn list(&self, filter: Option<ListFilter>) -> Result<Vec<Device>> {
        let f = filter.unwrap_or_default();
        let page: Page<Device> = self.client.execute(|http, _| {
            let mut req = http.get(self.client.url("devices").expect("url"));
            if let Some(p) = f.page      { req = req.query(&[("page", p)]); }
            if let Some(s) = f.page_size { req = req.query(&[("pageSize", s)]); }
            if let Some(s) = &f.search   { req = req.query(&[("search", s)]); }
            req
        }).await?;
        Ok(page.items)
    }

    /// Filter devices by current lifecycle state. Server-side filter; the
    /// client just adds a query parameter.
    pub async fn list_by_state(&self, state: DeviceState) -> Result<Vec<Device>> {
        let page: Page<Device> = self.client.execute(|http, _| {
            // Serde will render DeviceState to its snake_case wire form.
            let s = match state {
                DeviceState::Pending         => "pending",
                DeviceState::Active          => "active",
                DeviceState::Stale           => "stale",
                DeviceState::Suspended       => "suspended",
                DeviceState::Decommissioning => "decommissioning",
            };
            http.get(self.client.url("devices").expect("url"))
                .query(&[("state", s)])
        }).await?;
        Ok(page.items)
    }

    /// Look up by serial number.
    pub async fn get(&self, serial: &str) -> Result<Device> {
        let p = format!("devices/{}", urlencoding::encode(serial));
        self.client.execute(|http, _| http.get(self.client.url(&p).expect("url"))).await
    }

    /// Enroll a new device (create a tenant record + reserve the serial).
    pub async fn enroll(&self, body: &NewDevice) -> Result<Device> {
        if body.serial.trim().is_empty() {
            return Err(crate::Error::InvalidInput("device serial is empty".into()));
        }
        if body.model.trim().is_empty() {
            return Err(crate::Error::InvalidInput("device model is empty".into()));
        }
        let url = self.client.url("devices")?;
        self.client.execute(move |http, _| http.post(url.clone()).json(body)).await
    }

    /// Bind an already-enrolled device to a user.
    pub async fn assign(&self, serial: &str, user_id: uuid::Uuid) -> Result<Device> {
        let p = format!("devices/{}/assign", urlencoding::encode(serial));
        let url = self.client.url(&p)?;
        let body = serde_json::json!({ "userId": user_id });
        self.client.execute(move |http, _| http.post(url.clone()).json(&body)).await
    }

    /// Unbind a device (does NOT delete it — see `decommission()`).
    pub async fn unassign(&self, serial: &str) -> Result<Device> {
        let p = format!("devices/{}/unassign", urlencoding::encode(serial));
        let url = self.client.url(&p)?;
        self.client.execute(move |http, _| http.post(url.clone())).await
    }

    /// Mark a device for decommissioning. Server clears it at next sweep
    /// (typically within 24 h). Reversible until the sweep completes.
    pub async fn decommission(&self, serial: &str) -> Result<Device> {
        let p = format!("devices/{}/decommission", urlencoding::encode(serial));
        let url = self.client.url(&p)?;
        self.client.execute(move |http, _| http.post(url.clone())).await
    }

    /// Suspend a device (refuses to authenticate until reactivated).
    pub async fn suspend(&self, serial: &str) -> Result<Device> {
        let p = format!("devices/{}/suspend", urlencoding::encode(serial));
        let url = self.client.url(&p)?;
        self.client.execute(move |http, _| http.post(url.clone())).await
    }

    /// Reactivate a suspended device.
    pub async fn reactivate(&self, serial: &str) -> Result<Device> {
        let p = format!("devices/{}/reactivate", urlencoding::encode(serial));
        let url = self.client.url(&p)?;
        self.client.execute(move |http, _| http.post(url.clone())).await
    }
}

// ─── Note on dep ────────────────────────────────────────────────────
// `urlencoding` is a workspace dep — we use it as `urlencoding::encode(...)`
// inline above without a top-level `use`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enroll_rejects_empty_serial() {
        let c = Client::new("https://api.example.com", "acme").unwrap();
        let body = NewDevice { serial: "".into(), model: "TC53".into(), ..Default::default() };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(c.devices().enroll(&body)).err().unwrap();
        assert!(matches!(err, crate::Error::InvalidInput(_)));
    }

    #[test]
    fn enroll_rejects_empty_model() {
        let c = Client::new("https://api.example.com", "acme").unwrap();
        let body = NewDevice { serial: "ABC123".into(), model: "".into(), ..Default::default() };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(c.devices().enroll(&body)).err().unwrap();
        assert!(matches!(err, crate::Error::InvalidInput(_)));
    }
}
