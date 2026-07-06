//! `/tenant/{t}/groups` — talkgroup management.

use uuid::Uuid;
use crate::{Client, Result, Group, NewGroup, Page};
use crate::models::ListFilter;

pub struct GroupsApi<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> GroupsApi<'a> {
    pub async fn list(&self, filter: Option<ListFilter>) -> Result<Vec<Group>> {
        let f = filter.unwrap_or_default();
        let page: Page<Group> = self.client.execute(|http, _| {
            let mut req = http.get(self.client.url("groups").expect("url"));
            if let Some(p) = f.page      { req = req.query(&[("page", p)]); }
            if let Some(s) = f.page_size { req = req.query(&[("pageSize", s)]); }
            if let Some(s) = &f.search   { req = req.query(&[("search", s)]); }
            req
        }).await?;
        Ok(page.items)
    }

    pub async fn get(&self, group_id: Uuid) -> Result<Group> {
        let p = format!("groups/{}", group_id);
        self.client.execute(|http, _| http.get(self.client.url(&p).expect("url"))).await
    }

    pub async fn create(&self, body: &NewGroup) -> Result<Group> {
        if body.name.trim().is_empty() {
            return Err(crate::Error::InvalidInput("group name is empty".into()));
        }
        let url = self.client.url("groups")?;
        self.client.execute(move |http, _| http.post(url.clone()).json(body)).await
    }

    pub async fn delete(&self, group_id: Uuid) -> Result<()> {
        let p = format!("groups/{}", group_id);
        let url = self.client.url(&p)?;
        let _: serde_json::Value = self.client.execute(
            move |http, _| http.delete(url.clone())
        ).await?;
        Ok(())
    }

    /// Add a single user to the group.
    pub async fn add_member(&self, group_id: Uuid, user_id: Uuid) -> Result<Group> {
        let p = format!("groups/{}/members", group_id);
        let url = self.client.url(&p)?;
        let body = serde_json::json!({ "userId": user_id });
        self.client.execute(move |http, _| http.post(url.clone()).json(&body)).await
    }

    /// Remove a single user from the group.
    pub async fn remove_member(&self, group_id: Uuid, user_id: Uuid) -> Result<Group> {
        let p = format!("groups/{}/members/{}", group_id, user_id);
        let url = self.client.url(&p)?;
        self.client.execute(move |http, _| http.delete(url.clone())).await
    }

    /// Replace the entire member list in one call (atomic on the server).
    pub async fn set_members(&self, group_id: Uuid, members: &[Uuid]) -> Result<Group> {
        let p = format!("groups/{}/members", group_id);
        let url = self.client.url(&p)?;
        let body = serde_json::json!({ "memberIds": members });
        self.client.execute(move |http, _| http.put(url.clone()).json(&body)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_rejects_empty_name() {
        let c = Client::new("https://api.example.com", "acme").unwrap();
        let body = NewGroup { name: " ".into(), ..Default::default() };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(c.groups().create(&body)).err().unwrap();
        assert!(matches!(err, crate::Error::InvalidInput(_)));
    }
}
