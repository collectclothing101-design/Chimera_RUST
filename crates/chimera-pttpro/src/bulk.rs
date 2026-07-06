//! CSV-driven bulk fleet provisioning.
//!
//! Takes a CSV like
//!
//! ```csv
//! username,display_name,email,department,device_serial,device_model,group
//! alice.kim,Alice Kim,akim@acme.com,Receiving,12345TC53A001,TC53,Floor-A
//! bob.lee,Bob Lee,blee@acme.com,Picking,12345TC53A002,TC53,Floor-A
//! ```
//!
//! and drives the full create-user → assign-department → enroll-device →
//! generate-activation-code → assign-to-group sequence for every row.
//!
//! Outputs a second CSV with the generated activation codes that you can
//! hand to your EMM team to push as managed-config values.

use std::path::Path;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::{Client, Result, NewUser, NewDevice, NewGroup, models::GroupKind};

/// One row in the input CSV.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionRow {
    pub username:      String,
    pub display_name:  String,
    #[serde(default)]
    pub email:         Option<String>,
    #[serde(default)]
    pub department:    Option<String>,
    pub device_serial: String,
    pub device_model:  String,
    #[serde(default)]
    pub group:         Option<String>,
}

/// One row in the output CSV — everything we emitted on the server +
/// the activation code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionResult {
    pub username:        String,
    pub user_id:         Option<Uuid>,
    pub device_serial:   String,
    pub activation_code: Option<String>,
    pub status:          String,      // "ok" | "skipped" | "error"
    pub error:           Option<String>,
}

/// Aggregate report of one bulk run.
#[derive(Debug, Default, Serialize)]
pub struct BulkReport {
    pub total:        usize,
    pub succeeded:    usize,
    pub skipped:      usize,
    pub failed:       usize,
    pub results:      Vec<ProvisionResult>,
}

/// Read a CSV path into `Vec<ProvisionRow>`.
pub fn read_csv(path: &Path) -> Result<Vec<ProvisionRow>> {
    let mut rdr = csv::Reader::from_path(path)
        .map_err(|e| crate::Error::InvalidInput(format!("open csv {}: {}", path.display(), e)))?;
    let mut rows = Vec::new();
    for r in rdr.deserialize::<ProvisionRow>() {
        let row = r.map_err(|e| crate::Error::InvalidInput(format!("csv row: {}", e)))?;
        rows.push(row);
    }
    Ok(rows)
}

/// Write the results back to a CSV path.
pub fn write_csv(path: &Path, results: &[ProvisionResult]) -> Result<()> {
    let mut w = csv::Writer::from_path(path)
        .map_err(|e| crate::Error::InvalidInput(format!("create csv {}: {}", path.display(), e)))?;
    for r in results {
        w.serialize(r)
            .map_err(|e| crate::Error::InvalidInput(format!("write row: {}", e)))?;
    }
    w.flush().map_err(|e| crate::Error::InvalidInput(format!("flush: {}", e)))?;
    Ok(())
}

/// Drive the full provisioning workflow for one CSV against `client`.
///
/// **Behaviour**:
///   - Existing users (matched by username, case-insensitive) are reused;
///     no duplicate-create errors.
///   - Existing devices are skipped with status `skipped` unless `--reissue`
///     is set (then a fresh activation code is issued).
///   - Departments + groups are created on demand and cached per-run.
///   - Failures on one row do NOT abort the run; they're recorded and
///     processing continues to the next row.
pub async fn provision_csv(
    client:      &Client,
    rows:        &[ProvisionRow],
    reissue:     bool,
) -> BulkReport {
    let mut report = BulkReport { total: rows.len(), ..Default::default() };

    // Caches so we don't re-fetch departments / groups per row.
    let mut dept_cache:  HashMap<String, Uuid> = HashMap::new();
    let mut group_cache: HashMap<String, Uuid> = HashMap::new();

    for row in rows {
        let r = provision_single(client, row, reissue, &mut dept_cache, &mut group_cache).await;
        match &r.status[..] {
            "ok"      => report.succeeded += 1,
            "skipped" => report.skipped += 1,
            _         => report.failed += 1,
        }
        report.results.push(r);
    }
    report
}

async fn provision_single(
    client:      &Client,
    row:         &ProvisionRow,
    reissue:     bool,
    dept_cache:  &mut HashMap<String, Uuid>,
    group_cache: &mut HashMap<String, Uuid>,
) -> ProvisionResult {
    let mut result = ProvisionResult {
        username:        row.username.clone(),
        user_id:         None,
        device_serial:   row.device_serial.clone(),
        activation_code: None,
        status:          "error".into(),
        error:           None,
    };

    // (1) Resolve / create department
    let department_id = if let Some(dn) = row.department.as_ref().filter(|s| !s.trim().is_empty()) {
        match resolve_department(client, dn, dept_cache).await {
            Ok(id)  => Some(id),
            Err(e)  => { result.error = Some(format!("department: {}", e)); return result; }
        }
    } else { None };

    // (2) Resolve / create user
    let user = match resolve_user(client, row, department_id).await {
        Ok(u)   => u,
        Err(e)  => { result.error = Some(format!("user: {}", e)); return result; }
    };
    result.user_id = Some(user.id);

    // (3) Enroll device (skip if already exists & not reissue)
    let already_exists = client.devices().get(&row.device_serial).await.is_ok();
    if already_exists && !reissue {
        result.status = "skipped".into();
        result.error  = Some("device already enrolled (use --reissue to refresh)".into());
        return result;
    }
    if !already_exists {
        let nd = NewDevice {
            serial:           row.device_serial.clone(),
            model:            row.device_model.clone(),
            assigned_user_id: Some(user.id),
            department_id,
        };
        if let Err(e) = client.devices().enroll(&nd).await {
            result.error = Some(format!("enroll: {}", e));
            return result;
        }
    } else if let Err(e) = client.devices().assign(&row.device_serial, user.id).await {
        result.error = Some(format!("assign: {}", e));
        return result;
    }

    // (4) Generate activation code
    match client.activation().generate(&user.id, &row.device_serial).await {
        Ok(code) => result.activation_code = Some(code.code),
        Err(e)   => { result.error = Some(format!("activation: {}", e)); return result; }
    }

    // (5) Optionally add user to a talkgroup
    if let Some(gn) = row.group.as_ref().filter(|s| !s.trim().is_empty()) {
        match resolve_group(client, gn, group_cache).await {
            Ok(gid) => {
                if let Err(e) = client.groups().add_member(gid, user.id).await {
                    // group failure shouldn't void the rest of the row
                    result.error = Some(format!("group add (non-fatal): {}", e));
                }
            }
            Err(e) => {
                result.error = Some(format!("group resolve (non-fatal): {}", e));
            }
        }
    }

    result.status = "ok".into();
    result
}

async fn resolve_department(client: &Client, name: &str,
                            cache: &mut HashMap<String, Uuid>) -> Result<Uuid>
{
    if let Some(id) = cache.get(name) { return Ok(*id); }
    let depts = client.departments().list().await?;
    if let Some(d) = depts.iter().find(|d| d.name.eq_ignore_ascii_case(name)) {
        cache.insert(name.to_string(), d.id);
        return Ok(d.id);
    }
    let created = client.departments().create(name, None, None).await?;
    cache.insert(name.to_string(), created.id);
    Ok(created.id)
}

async fn resolve_user(client: &Client, row: &ProvisionRow,
                      department_id: Option<Uuid>) -> Result<crate::User>
{
    if let Some(u) = client.users().find_by_username(&row.username).await? {
        return Ok(u);
    }
    let body = NewUser {
        username:      row.username.clone(),
        display_name:  row.display_name.clone(),
        email:         row.email.clone(),
        department_id,
        ..Default::default()
    };
    client.users().create(&body).await
}

async fn resolve_group(client: &Client, name: &str,
                       cache: &mut HashMap<String, Uuid>) -> Result<Uuid>
{
    if let Some(id) = cache.get(name) { return Ok(*id); }
    let groups = client.groups().list(None).await?;
    if let Some(g) = groups.iter().find(|g| g.name.eq_ignore_ascii_case(name)) {
        cache.insert(name.to_string(), g.id);
        return Ok(g.id);
    }
    let created = client.groups().create(&NewGroup {
        name:        name.to_string(),
        kind:        Some(GroupKind::Persistent),
        ..Default::default()
    }).await?;
    cache.insert(name.to_string(), created.id);
    Ok(created.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_csv_parses_basic_row() {
        let tmp = std::env::temp_dir().join("test-pttpro-bulk.csv");
        std::fs::write(&tmp, "username,display_name,email,department,device_serial,device_model,group\n\
                              alice,Alice Kim,alice@x.com,Recv,SN1,TC53,Floor-A\n").unwrap();
        let rows = read_csv(&tmp).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].username, "alice");
        assert_eq!(rows[0].device_model, "TC53");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn write_csv_roundtrip() {
        let tmp = std::env::temp_dir().join("test-pttpro-bulk-out.csv");
        let results = vec![
            ProvisionResult {
                username:        "alice".into(),
                user_id:         Some(Uuid::nil()),
                device_serial:   "SN1".into(),
                activation_code: Some("ABCD-EFGH-IJKL".into()),
                status:          "ok".into(),
                error:           None,
            }
        ];
        write_csv(&tmp, &results).unwrap();
        let body = std::fs::read_to_string(&tmp).unwrap();
        assert!(body.contains("alice"));
        assert!(body.contains("ABCD-EFGH-IJKL"));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn bulk_report_default_zero_counts() {
        let r = BulkReport::default();
        assert_eq!(r.total, 0);
        assert_eq!(r.succeeded, 0);
        assert!(r.results.is_empty());
    }
}
