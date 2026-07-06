//! `/tenant/{t}/departments` — org-hierarchy management.

use uuid::Uuid;
use crate::{Client, Result, Department, Page};

pub struct DepartmentsApi<'a> {
    pub(crate) client: &'a Client,
}

impl<'a> DepartmentsApi<'a> {
    pub async fn list(&self) -> Result<Vec<Department>> {
        let page: Page<Department> = self.client.execute(
            |http, _| http.get(self.client.url("departments").expect("url"))
        ).await?;
        Ok(page.items)
    }

    pub async fn get(&self, id: Uuid) -> Result<Department> {
        let p = format!("departments/{}", id);
        self.client.execute(|http, _| http.get(self.client.url(&p).expect("url"))).await
    }

    pub async fn create(&self, name: &str, description: Option<&str>,
                        parent_id: Option<Uuid>) -> Result<Department>
    {
        if name.trim().is_empty() {
            return Err(crate::Error::InvalidInput("department name is empty".into()));
        }
        let url  = self.client.url("departments")?;
        let body = serde_json::json!({
            "name":        name,
            "description": description,
            "parentId":    parent_id,
        });
        self.client.execute(move |http, _| http.post(url.clone()).json(&body)).await
    }

    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let p = format!("departments/{}", id);
        let url = self.client.url(&p)?;
        let _: serde_json::Value = self.client.execute(
            move |http, _| http.delete(url.clone())
        ).await?;
        Ok(())
    }
}
