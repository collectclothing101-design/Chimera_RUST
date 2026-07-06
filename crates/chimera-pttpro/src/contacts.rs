//! `/tenant/{t}/contacts` — 1-to-1 contact-list management.

use uuid::Uuid;
use crate::{Client, Result, Contact, Page};

pub struct ContactsApi<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> ContactsApi<'a> {
    /// Every contact entry owned by `user_id`.
    pub async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<Contact>> {
        let p = format!("users/{}/contacts", user_id);
        let page: Page<Contact> = self.client.execute(
            |http, _| http.get(self.client.url(&p).expect("url"))
        ).await?;
        Ok(page.items)
    }

    /// Add `target_user_id` to `owner_user_id`'s contact list.
    pub async fn add(&self, owner_user_id: Uuid, target_user_id: Uuid,
                     label: Option<&str>) -> Result<Contact>
    {
        let p = format!("users/{}/contacts", owner_user_id);
        let url = self.client.url(&p)?;
        let body = serde_json::json!({
            "targetUserId": target_user_id,
            "label":        label,
        });
        self.client.execute(move |http, _| http.post(url.clone()).json(&body)).await
    }

    /// Remove a contact by id.
    pub async fn remove(&self, owner_user_id: Uuid, contact_id: Uuid) -> Result<()> {
        let p = format!("users/{}/contacts/{}", owner_user_id, contact_id);
        let url = self.client.url(&p)?;
        let _: serde_json::Value = self.client.execute(
            move |http, _| http.delete(url.clone())
        ).await?;
        Ok(())
    }
}
