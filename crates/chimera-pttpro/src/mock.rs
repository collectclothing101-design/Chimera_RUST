//! Local mock PTT Pro server, feature-gated behind `--features mock`.
//!
//! Spins up an Axum HTTP server that mimics every documented endpoint with
//! an in-memory store. Use it in tests, demos, and offline development.
//!
//! ```ignore
//! # #[cfg(feature = "mock")]
//! # async fn demo() -> anyhow::Result<()> {
//! use chimera_pttpro::{mock::MockServer, Client, Credentials};
//!
//! let mock = MockServer::start().await?;
//! let client = Client::new(mock.base_url(), "test-tenant")?
//!     .with_credentials(Credentials::bearer("any"));
//!
//! let user = client.users().create(&Default::default()).await?;
//! mock.shutdown().await;
//! # Ok(()) }
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{Router, Json, extract::{State, Path as AxPath, Query}, response::IntoResponse,
           http::StatusCode, routing::{get, post, delete, patch, put}};
use uuid::Uuid;
use chrono::{Utc, Duration};
use crate::models::*;

/// In-memory tenant store.
#[derive(Default, Clone)]
pub struct Store {
    pub users:       HashMap<Uuid, User>,
    pub groups:      HashMap<Uuid, Group>,
    pub departments: HashMap<Uuid, Department>,
    pub devices:     HashMap<String, Device>,
    pub codes:       HashMap<String, ActivationCode>,
    pub tenant_id:   Uuid,
}

type SharedStore = Arc<RwLock<Store>>;

/// Running mock server handle.
pub struct MockServer {
    base:      String,
    handle:    tokio::task::JoinHandle<()>,
    shutdown:  tokio::sync::oneshot::Sender<()>,
    pub store: SharedStore,
}

impl MockServer {
    /// Bind to an OS-assigned port on 127.0.0.1 and start serving.
    pub async fn start() -> Result<Self, std::io::Error> {
        let store = Arc::new(RwLock::new(Store {
            tenant_id: Uuid::new_v4(),
            ..Default::default()
        }));
        let app = router(store.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr: SocketAddr = listener.local_addr()?;
        let base = format!("http://{}", addr);

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async { let _ = rx.await; })
                .await;
        });

        Ok(MockServer { base, handle, shutdown: tx, store })
    }

    pub fn base_url(&self) -> &str { &self.base }

    pub async fn shutdown(self) {
        let _ = self.shutdown.send(());
        let _ = self.handle.await;
    }
}

fn router(store: SharedStore) -> Router {
    Router::new()
        // OAuth2
        .route("/oauth2/token",                                  post(token))
        // Users
        .route("/tenant/:tenant/users",                         get(list_users).post(create_user))
        .route("/tenant/:tenant/users/:id",                    get(get_user).patch(patch_user).delete(delete_user))
        // Groups
        .route("/tenant/:tenant/groups",                        get(list_groups).post(create_group))
        .route("/tenant/:tenant/groups/:id",                   get(get_group).delete(delete_group))
        .route("/tenant/:tenant/groups/:id/members",           post(add_member).put(set_members))
        .route("/tenant/:tenant/groups/:id/members/:user_id", delete(remove_member))
        // Departments
        .route("/tenant/:tenant/departments",                   get(list_departments).post(create_department))
        .route("/tenant/:tenant/departments/:id",              get(get_department).delete(delete_department))
        // Devices
        .route("/tenant/:tenant/devices",                       get(list_devices).post(enroll_device))
        .route("/tenant/:tenant/devices/:serial",              get(get_device))
        .route("/tenant/:tenant/devices/:serial/assign",       post(assign_device))
        .route("/tenant/:tenant/devices/:serial/unassign",     post(unassign_device))
        // Activation
        .route("/tenant/:tenant/activation/codes",              post(generate_code))
        .route("/tenant/:tenant/activation/codes/bulk",         post(generate_bulk))
        .route("/tenant/:tenant/activation/codes/:code",       get(get_code).delete(revoke_code))
        .route("/tenant/:tenant/users/:id/activation/codes",   get(list_codes_for_user))
        .with_state(store)
}

// ─── Auth ─────────────────────────────────────────────────────────────

async fn token() -> impl IntoResponse {
    let resp = crate::client::TokenResponse {
        access_token: "mock-bearer-token".into(),
        token_type:   "Bearer".into(),
        expires_in:   3600,
        refresh_token: None,
        scope:        Some("api".into()),
    };
    (StatusCode::OK, Json(resp))
}

// ─── Users ────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct ListQ { page: Option<u32>, #[serde(rename = "pageSize")] page_size: Option<u32>,
               search: Option<String> }

async fn list_users(State(s): State<SharedStore>,
                    Query(q): Query<ListQ>) -> impl IntoResponse {
    let g = s.read().await;
    let mut items: Vec<User> = g.users.values().cloned().collect();
    if let Some(needle) = q.search.as_deref() {
        let n = needle.to_lowercase();
        items.retain(|u| u.username.to_lowercase().contains(&n) ||
                         u.display_name.to_lowercase().contains(&n));
    }
    items.sort_by(|a, b| a.username.cmp(&b.username));
    paginate(items, q.page, q.page_size)
}

async fn create_user(State(s): State<SharedStore>,
                     Json(body): Json<NewUser>) -> impl IntoResponse {
    let mut g = s.write().await;
    if g.users.values().any(|u| u.username.eq_ignore_ascii_case(&body.username)) {
        return (StatusCode::CONFLICT, "username already exists").into_response();
    }
    let now = Utc::now();
    let user = User {
        id:                 Uuid::new_v4(),
        username:           body.username,
        email:              body.email,
        display_name:       body.display_name,
        department_id:      body.department_id,
        role:               body.role.unwrap_or(Role::User),
        permissions:        vec![Permission::InitiatePttCall, Permission::JoinTalkgroup],
        assigned_device_id: None,
        created_at:         now,
        updated_at:         now,
        active:             true,
    };
    g.users.insert(user.id, user.clone());
    (StatusCode::CREATED, Json(user)).into_response()
}

async fn get_user(State(s): State<SharedStore>,
                  AxPath((_t, id)): AxPath<(String, Uuid)>) -> impl IntoResponse {
    let g = s.read().await;
    match g.users.get(&id) {
        Some(u) => (StatusCode::OK, Json(u.clone())).into_response(),
        None    => (StatusCode::NOT_FOUND, "user not found").into_response(),
    }
}

async fn patch_user(State(s): State<SharedStore>,
                    AxPath((_t, id)): AxPath<(String, Uuid)>,
                    Json(patch): Json<serde_json::Value>) -> impl IntoResponse {
    let mut g = s.write().await;
    let Some(user) = g.users.get_mut(&id) else {
        return (StatusCode::NOT_FOUND, "user not found").into_response();
    };
    if let Some(v) = patch.get("active").and_then(|x| x.as_bool()) { user.active = v; }
    if let Some(v) = patch.get("displayName").and_then(|x| x.as_str()) {
        user.display_name = v.to_string();
    }
    if let Some(v) = patch.get("email").and_then(|x| x.as_str()) {
        user.email = Some(v.to_string());
    }
    user.updated_at = Utc::now();
    let u = user.clone();
    (StatusCode::OK, Json(u)).into_response()
}

async fn delete_user(State(s): State<SharedStore>,
                     AxPath((_t, id)): AxPath<(String, Uuid)>) -> impl IntoResponse {
    let mut g = s.write().await;
    if g.users.remove(&id).is_some() {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        (StatusCode::NOT_FOUND, "user not found").into_response()
    }
}

// ─── Groups ───────────────────────────────────────────────────────────

async fn list_groups(State(s): State<SharedStore>, Query(q): Query<ListQ>)
    -> impl IntoResponse
{
    let g = s.read().await;
    let items: Vec<Group> = g.groups.values().cloned().collect();
    paginate(items, q.page, q.page_size)
}

async fn create_group(State(s): State<SharedStore>, Json(body): Json<NewGroup>)
    -> impl IntoResponse
{
    let mut g = s.write().await;
    let now = Utc::now();
    let grp = Group {
        id:          Uuid::new_v4(),
        name:        body.name,
        description: body.description,
        member_ids:  body.member_ids,
        kind:        body.kind.unwrap_or(GroupKind::Persistent),
        created_at:  now,
        updated_at:  now,
    };
    g.groups.insert(grp.id, grp.clone());
    (StatusCode::CREATED, Json(grp)).into_response()
}

async fn get_group(State(s): State<SharedStore>,
                   AxPath((_t, id)): AxPath<(String, Uuid)>) -> impl IntoResponse {
    let g = s.read().await;
    match g.groups.get(&id) {
        Some(x) => (StatusCode::OK, Json(x.clone())).into_response(),
        None    => (StatusCode::NOT_FOUND, "group not found").into_response(),
    }
}

async fn delete_group(State(s): State<SharedStore>,
                      AxPath((_t, id)): AxPath<(String, Uuid)>) -> impl IntoResponse {
    let mut g = s.write().await;
    if g.groups.remove(&id).is_some() {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        (StatusCode::NOT_FOUND, "group not found").into_response()
    }
}

#[derive(serde::Deserialize)]
struct MemberBody { #[serde(rename = "userId")] user_id: Uuid }

async fn add_member(State(s): State<SharedStore>,
                    AxPath((_t, id)): AxPath<(String, Uuid)>,
                    Json(body): Json<MemberBody>) -> impl IntoResponse {
    let mut g = s.write().await;
    let Some(grp) = g.groups.get_mut(&id) else {
        return (StatusCode::NOT_FOUND, "group not found").into_response();
    };
    if !grp.member_ids.contains(&body.user_id) {
        grp.member_ids.push(body.user_id);
        grp.updated_at = Utc::now();
    }
    let cloned = grp.clone();
    (StatusCode::OK, Json(cloned)).into_response()
}

async fn remove_member(State(s): State<SharedStore>,
                       AxPath((_t, id, user_id)): AxPath<(String, Uuid, Uuid)>)
    -> impl IntoResponse
{
    let mut g = s.write().await;
    let Some(grp) = g.groups.get_mut(&id) else {
        return (StatusCode::NOT_FOUND, "group not found").into_response();
    };
    grp.member_ids.retain(|u| *u != user_id);
    grp.updated_at = Utc::now();
    let c = grp.clone();
    (StatusCode::OK, Json(c)).into_response()
}

#[derive(serde::Deserialize)]
struct SetMembersBody { #[serde(rename = "memberIds")] member_ids: Vec<Uuid> }

async fn set_members(State(s): State<SharedStore>,
                     AxPath((_t, id)): AxPath<(String, Uuid)>,
                     Json(body): Json<SetMembersBody>) -> impl IntoResponse {
    let mut g = s.write().await;
    let Some(grp) = g.groups.get_mut(&id) else {
        return (StatusCode::NOT_FOUND, "group not found").into_response();
    };
    grp.member_ids = body.member_ids;
    grp.updated_at = Utc::now();
    let c = grp.clone();
    (StatusCode::OK, Json(c)).into_response()
}

// ─── Departments ──────────────────────────────────────────────────────

async fn list_departments(State(s): State<SharedStore>, Query(q): Query<ListQ>)
    -> impl IntoResponse
{
    let g = s.read().await;
    let items: Vec<Department> = g.departments.values().cloned().collect();
    paginate(items, q.page, q.page_size)
}

#[derive(serde::Deserialize)]
struct NewDeptBody { name: String, #[serde(default)] description: Option<String>,
                     #[serde(rename = "parentId", default)] parent_id: Option<Uuid> }

async fn create_department(State(s): State<SharedStore>,
                           Json(body): Json<NewDeptBody>) -> impl IntoResponse {
    let mut g = s.write().await;
    let now = Utc::now();
    let d = Department {
        id:          Uuid::new_v4(),
        name:        body.name,
        description: body.description,
        parent_id:   body.parent_id,
        created_at:  now,
        updated_at:  now,
    };
    g.departments.insert(d.id, d.clone());
    (StatusCode::CREATED, Json(d)).into_response()
}

async fn get_department(State(s): State<SharedStore>,
                        AxPath((_t, id)): AxPath<(String, Uuid)>) -> impl IntoResponse {
    let g = s.read().await;
    match g.departments.get(&id) {
        Some(x) => (StatusCode::OK, Json(x.clone())).into_response(),
        None    => (StatusCode::NOT_FOUND, "department not found").into_response(),
    }
}

async fn delete_department(State(s): State<SharedStore>,
                           AxPath((_t, id)): AxPath<(String, Uuid)>)
    -> impl IntoResponse
{
    let mut g = s.write().await;
    if g.departments.remove(&id).is_some() {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        (StatusCode::NOT_FOUND, "department not found").into_response()
    }
}

// ─── Devices ──────────────────────────────────────────────────────────

async fn list_devices(State(s): State<SharedStore>, Query(q): Query<ListQ>)
    -> impl IntoResponse
{
    let g = s.read().await;
    let items: Vec<Device> = g.devices.values().cloned().collect();
    paginate(items, q.page, q.page_size)
}

async fn enroll_device(State(s): State<SharedStore>, Json(body): Json<NewDevice>)
    -> impl IntoResponse
{
    let mut g = s.write().await;
    if g.devices.contains_key(&body.serial) {
        return (StatusCode::CONFLICT, "device already enrolled").into_response();
    }
    let tenant_id = g.tenant_id;
    let d = Device {
        id:                body.serial.clone(),
        model:             body.model,
        assigned_user_id:  body.assigned_user_id,
        firmware_version:  None,
        last_seen_at:      None,
        state:             DeviceState::Pending,
        tenant_id,
    };
    g.devices.insert(d.id.clone(), d.clone());
    (StatusCode::CREATED, Json(d)).into_response()
}

async fn get_device(State(s): State<SharedStore>,
                    AxPath((_t, serial)): AxPath<(String, String)>) -> impl IntoResponse {
    let g = s.read().await;
    match g.devices.get(&serial) {
        Some(d) => (StatusCode::OK, Json(d.clone())).into_response(),
        None    => (StatusCode::NOT_FOUND, "device not found").into_response(),
    }
}

#[derive(serde::Deserialize)]
struct AssignBody { #[serde(rename = "userId")] user_id: Uuid }

async fn assign_device(State(s): State<SharedStore>,
                       AxPath((_t, serial)): AxPath<(String, String)>,
                       Json(body): Json<AssignBody>) -> impl IntoResponse {
    let mut g = s.write().await;
    let Some(d) = g.devices.get_mut(&serial) else {
        return (StatusCode::NOT_FOUND, "device not found").into_response();
    };
    d.assigned_user_id = Some(body.user_id);
    let c = d.clone();
    (StatusCode::OK, Json(c)).into_response()
}

async fn unassign_device(State(s): State<SharedStore>,
                         AxPath((_t, serial)): AxPath<(String, String)>)
    -> impl IntoResponse
{
    let mut g = s.write().await;
    let Some(d) = g.devices.get_mut(&serial) else {
        return (StatusCode::NOT_FOUND, "device not found").into_response();
    };
    d.assigned_user_id = None;
    let c = d.clone();
    (StatusCode::OK, Json(c)).into_response()
}

// ─── Activation codes ─────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct GenBody {
    #[serde(rename = "userId")]       user_id:       Uuid,
    #[serde(rename = "deviceSerial")] device_serial: String,
    #[serde(rename = "expiresAt", default)] expires_at: Option<String>,
}

async fn generate_code(State(s): State<SharedStore>, Json(body): Json<GenBody>)
    -> impl IntoResponse
{
    let mut g = s.write().await;
    let now = Utc::now();
    let expires = body.expires_at
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|| now + Duration::hours(72));
    let code = format!("{:X}-{:X}-{:X}",
        rand_u32(), rand_u32(), rand_u32());
    let entry = ActivationCode {
        code:           code.clone(),
        user_id:        body.user_id,
        device_serial:  body.device_serial,
        issued_at:      now,
        expires_at:     expires,
        redeemed:       false,
        redeemed_at:    None,
    };
    g.codes.insert(code, entry.clone());
    (StatusCode::CREATED, Json(entry)).into_response()
}

#[derive(serde::Deserialize)]
struct BulkBody { entries: Vec<GenBody> }

async fn generate_bulk(State(s): State<SharedStore>, Json(body): Json<BulkBody>)
    -> impl IntoResponse
{
    let mut g = s.write().await;
    let mut codes = Vec::with_capacity(body.entries.len());
    let now = Utc::now();
    for entry in body.entries {
        let code = format!("{:X}-{:X}-{:X}",
            rand_u32(), rand_u32(), rand_u32());
        let expires = entry.expires_at
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|| now + Duration::hours(72));
        let e = ActivationCode {
            code: code.clone(), user_id: entry.user_id,
            device_serial: entry.device_serial,
            issued_at: now, expires_at: expires,
            redeemed: false, redeemed_at: None,
        };
        g.codes.insert(code, e.clone());
        codes.push(e);
    }
    (StatusCode::CREATED, Json(serde_json::json!({ "codes": codes }))).into_response()
}

async fn get_code(State(s): State<SharedStore>,
                  AxPath((_t, code)): AxPath<(String, String)>) -> impl IntoResponse {
    let g = s.read().await;
    match g.codes.get(&code) {
        Some(c) => (StatusCode::OK, Json(c.clone())).into_response(),
        None    => (StatusCode::NOT_FOUND, "code not found").into_response(),
    }
}

async fn revoke_code(State(s): State<SharedStore>,
                     AxPath((_t, code)): AxPath<(String, String)>) -> impl IntoResponse {
    let mut g = s.write().await;
    if g.codes.remove(&code).is_some() {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        (StatusCode::NOT_FOUND, "code not found").into_response()
    }
}

async fn list_codes_for_user(State(s): State<SharedStore>,
                             AxPath((_t, user_id)): AxPath<(String, Uuid)>)
    -> impl IntoResponse
{
    let g = s.read().await;
    let items: Vec<ActivationCode> = g.codes.values()
        .filter(|c| c.user_id == user_id)
        .cloned()
        .collect();
    (StatusCode::OK, Json(serde_json::json!({ "items": items }))).into_response()
}

// ─── Helpers ──────────────────────────────────────────────────────────

fn paginate<T: Clone + serde::Serialize>(
    items:   Vec<T>,
    page:    Option<u32>,
    psize:   Option<u32>,
) -> axum::response::Response {
    let page  = page.unwrap_or(0);
    let size  = psize.unwrap_or(25).max(1);
    let total = items.len() as u64;
    let total_pages = ((total as u32) + size - 1) / size;
    let start = (page * size) as usize;
    let end   = ((page + 1) * size).min(total as u32) as usize;
    let slice: Vec<T> = if start < items.len() { items[start..end.min(items.len())].to_vec() } else { vec![] };
    let p = Page {
        items:       slice,
        page,
        page_size:   size,
        total_items: total,
        total_pages,
    };
    (StatusCode::OK, Json(p)).into_response()
}

/// Deterministic-ish pseudo-random u32 for mock activation codes.
/// Uses system clock — no security implications, mock only.
fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos()).unwrap_or(0);
    nanos.wrapping_mul(2_654_435_761)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_server_starts_and_serves_token() {
        let mock = MockServer::start().await.unwrap();
        let url = format!("{}/oauth2/token", mock.base_url());
        let client = reqwest::Client::new();
        let resp = client.post(&url)
            .form(&[("grant_type", "client_credentials"),
                    ("client_id", "x"), ("client_secret", "y")])
            .send().await.unwrap();
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body.get("access_token").is_some());
        mock.shutdown().await;
    }

    #[tokio::test]
    async fn mock_create_user_then_list() {
        let mock = MockServer::start().await.unwrap();
        let client = crate::Client::new(mock.base_url(), "acme").unwrap()
            .with_credentials(crate::Credentials::bearer("any"));
        let new_u = NewUser {
            username: "alice".into(),
            display_name: "Alice".into(),
            ..Default::default()
        };
        let created = client.users().create(&new_u).await.unwrap();
        assert_eq!(created.username, "alice");
        let list = client.users().list(None).await.unwrap();
        assert_eq!(list.len(), 1);
        mock.shutdown().await;
    }

    #[tokio::test]
    async fn mock_create_user_conflict_on_duplicate() {
        let mock = MockServer::start().await.unwrap();
        let client = crate::Client::new(mock.base_url(), "acme").unwrap()
            .with_credentials(crate::Credentials::bearer("any"));
        let body = NewUser {
            username: "bob".into(),
            display_name: "Bob".into(),
            ..Default::default()
        };
        client.users().create(&body).await.unwrap();
        let err = client.users().create(&body).await.err().unwrap();
        assert!(matches!(err, crate::Error::Conflict(_)));
        mock.shutdown().await;
    }

    #[tokio::test]
    async fn mock_full_provisioning_flow() {
        let mock   = MockServer::start().await.unwrap();
        let client = crate::Client::new(mock.base_url(), "acme").unwrap()
            .with_credentials(crate::Credentials::bearer("any"));

        let user = client.users().create(&NewUser {
            username: "alice.kim".into(),
            display_name: "Alice Kim".into(),
            ..Default::default()
        }).await.unwrap();

        let device = client.devices().enroll(&NewDevice {
            serial: "TC53-SN-0001".into(),
            model:  "TC53".into(),
            assigned_user_id: Some(user.id),
            ..Default::default()
        }).await.unwrap();
        assert_eq!(device.id, "TC53-SN-0001");
        assert_eq!(device.assigned_user_id, Some(user.id));

        let code = client.activation()
            .generate(&user.id, "TC53-SN-0001").await.unwrap();
        assert!(!code.code.is_empty());
        assert!(!code.redeemed);

        mock.shutdown().await;
    }

    #[tokio::test]
    async fn mock_bulk_activation_codes() {
        let mock   = MockServer::start().await.unwrap();
        let client = crate::Client::new(mock.base_url(), "acme").unwrap()
            .with_credentials(crate::Credentials::bearer("any"));
        let user = client.users().create(&NewUser {
            username: "test".into(), display_name: "Test".into(),
            ..Default::default()
        }).await.unwrap();
        let pairs = vec![
            (user.id, "SN-A".to_string()),
            (user.id, "SN-B".to_string()),
            (user.id, "SN-C".to_string()),
        ];
        let codes = client.activation().generate_bulk(&pairs).await.unwrap();
        assert_eq!(codes.len(), 3);
        mock.shutdown().await;
    }
}
