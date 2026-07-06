//! `/tenant/{t}/users` — user management.

use uuid::Uuid;
use crate::{Client, Result, User, NewUser, Page};
use crate::models::ListFilter;

/// API surface for the `/users` endpoint.
///
/// Borrowed reference to the parent client — cheap, no allocation.
pub struct UsersApi<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> UsersApi<'a> {
    /// List users with optional filter. `None` = first page, default size.
    pub async fn list(&self, filter: Option<ListFilter>) -> Result<Vec<User>> {
        let f = filter.unwrap_or_default();
        let page: Page<User> = self.client.execute(|http, _| {
            let url = self.client.url("users").expect("url builds");
            let mut req = http.get(url);
            if let Some(p)  = f.page      { req = req.query(&[("page", p)]); }
            if let Some(s)  = f.page_size { req = req.query(&[("pageSize", s)]); }
            if let Some(s)  = &f.search   { req = req.query(&[("search", s)]); }
            if let Some(s)  = &f.sort_by  { req = req.query(&[("sortBy", s)]); }
            req
        }).await?;
        Ok(page.items)
    }

    /// Fetch every user across all pages (concatenates pages serially).
    /// For large tenants prefer `list()` with explicit paging.
    pub async fn list_all(&self) -> Result<Vec<User>> {
        let mut out = Vec::new();
        let mut page = 0;
        let page_size = 100u32;
        loop {
            let chunk: Page<User> = self.client.execute(|http, _| {
                http.get(self.client.url("users").expect("url"))
                    .query(&[("page", page), ("pageSize", page_size)])
            }).await?;
            let total_pages = chunk.total_pages;
            out.extend(chunk.items);
            page += 1;
            if page >= total_pages { break; }
        }
        Ok(out)
    }

    /// Fetch one user by id.
    pub async fn get(&self, user_id: Uuid) -> Result<User> {
        let path = format!("users/{}", user_id);
        self.client.execute(|http, _| http.get(self.client.url(&path).expect("url"))).await
    }

    /// Look up by username (case-insensitive).
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let f = ListFilter {
            search: Some(username.to_string()),
            page_size: Some(50),
            ..Default::default()
        };
        let list = self.list(Some(f)).await?;
        Ok(list.into_iter().find(|u| u.username.eq_ignore_ascii_case(username)))
    }

    /// Create a new user. Returns the fully-populated server-issued user.
    pub async fn create(&self, body: &NewUser) -> Result<User> {
        if body.username.trim().is_empty() {
            return Err(crate::Error::InvalidInput("username is empty".into()));
        }
        if body.display_name.trim().is_empty() {
            return Err(crate::Error::InvalidInput("displayName is empty".into()));
        }
        let url = self.client.url("users")?;
        self.client.execute(move |http, _| http.post(url.clone()).json(body)).await
    }

    /// Patch one or more fields on an existing user.
    pub async fn update(&self, user_id: Uuid, patch: &serde_json::Value) -> Result<User> {
        let path = format!("users/{}", user_id);
        let url = self.client.url(&path)?;
        self.client.execute(move |http, _| http.patch(url.clone()).json(patch)).await
    }

    /// Delete a user.
    pub async fn delete(&self, user_id: Uuid) -> Result<()> {
        let path = format!("users/{}", user_id);
        let url = self.client.url(&path)?;
        let _: serde_json::Value = self.client.execute(
            move |http, _| http.delete(url.clone())
        ).await?;
        Ok(())
    }

    /// Suspend (soft-delete) a user — they cannot log in until reactivated.
    pub async fn suspend(&self, user_id: Uuid) -> Result<User> {
        self.update(user_id, &serde_json::json!({ "active": false })).await
    }

    /// Reactivate a suspended user.
    pub async fn reactivate(&self, user_id: Uuid) -> Result<User> {
        self.update(user_id, &serde_json::json!({ "active": true })).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_validates_empty_username() {
        // We can build the call without hitting the network because
        // validation runs first.
        let c = Client::new("https://api.example.com", "acme").unwrap();
        let api = c.users();
        let body = NewUser { username: "".into(), display_name: "x".into(), ..Default::default() };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(api.create(&body)).err().unwrap();
        assert!(matches!(err, crate::Error::InvalidInput(_)));
    }

    #[test]
    fn create_validates_empty_display_name() {
        let c = Client::new("https://api.example.com", "acme").unwrap();
        let api = c.users();
        let body = NewUser { username: "alice".into(), display_name: "".into(), ..Default::default() };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(api.create(&body)).err().unwrap();
        assert!(matches!(err, crate::Error::InvalidInput(_)));
    }
}
