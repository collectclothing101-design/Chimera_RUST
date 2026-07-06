//! Typed request/response models.
//!
//! Field shapes follow the public Workforce Connect REST conventions:
//! camelCase wire format, ISO-8601 timestamps, lowercase enum variants
//! in JSON. When your tenant's spec disagrees, adjust the `#[serde(rename)]`
//! attributes — every wire-format field is centralised in this module.

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

// ─── Pagination wrapper ───────────────────────────────────────────────

/// A paginated response envelope. The wire format wraps every collection
/// response with `{ items: [...], page, pageSize, totalItems, totalPages }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items:        Vec<T>,
    pub page:         u32,
    pub page_size:    u32,
    pub total_items:  u64,
    pub total_pages:  u32,
}

impl<T> Page<T> {
    /// True if there is at least one page beyond this one.
    pub fn has_next(&self) -> bool { self.page + 1 < self.total_pages }
}

// ─── User ─────────────────────────────────────────────────────────────

/// A PTT Pro user (= one person who can place / receive calls).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id:                Uuid,
    pub username:          String,
    pub email:             Option<String>,
    pub display_name:      String,
    pub department_id:     Option<Uuid>,
    pub role:              Role,
    pub permissions:       Vec<Permission>,
    pub assigned_device_id:Option<String>,
    pub created_at:        DateTime<Utc>,
    pub updated_at:        DateTime<Utc>,
    pub active:            bool,
}

/// Body for `POST /users` — fields needed at user-create time.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NewUser {
    pub username:      String,
    pub display_name:  String,
    pub email:         Option<String>,
    pub department_id: Option<Uuid>,
    pub role:          Option<Role>,
    pub initial_password: Option<String>,
}

/// Role assigned to a user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Supervisor,
    User,
    Guest,
}
impl Default for Role { fn default() -> Self { Role::User } }

/// Fine-grained permissions. `Role` implies a baseline set; explicit
/// `permissions` overlay or restrict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    InitiatePttCall,
    JoinTalkgroup,
    CreateTalkgroup,
    ManageUsers,
    ManageDevices,
    ViewAuditLog,
    ExportData,
    EmergencyCall,
}

// ─── Group ────────────────────────────────────────────────────────────

/// A talkgroup — one PTT channel. Calls go to every member at once.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id:           Uuid,
    pub name:         String,
    pub description:  Option<String>,
    pub member_ids:   Vec<Uuid>,
    pub kind:         GroupKind,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NewGroup {
    pub name:        String,
    pub description: Option<String>,
    pub member_ids:  Vec<Uuid>,
    pub kind:        Option<GroupKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupKind {
    /// Always-on talkgroup — members hear every call automatically.
    Persistent,
    /// Ad-hoc — call has to be initiated explicitly each time.
    AdHoc,
    /// Broadcast — only supervisors can talk, members listen.
    Broadcast,
    /// Emergency channel — high-priority preemption.
    Emergency,
}
impl Default for GroupKind { fn default() -> Self { GroupKind::Persistent } }

// ─── Contact ──────────────────────────────────────────────────────────

/// A peer the user can call directly (1-to-1, not a talkgroup).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id:               Uuid,
    pub owner_user_id:    Uuid,
    pub target_user_id:   Uuid,
    pub label:            Option<String>,
    pub created_at:       DateTime<Utc>,
}

// ─── Department ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Department {
    pub id:           Uuid,
    pub name:         String,
    pub description:  Option<String>,
    pub parent_id:    Option<Uuid>,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}

// ─── Device ───────────────────────────────────────────────────────────

/// A device enrolled in the PTT Pro tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id:                String,    // Zebra device serial number
    pub model:             String,    // TC52 / TC52x / TC53 / TC53e
    pub assigned_user_id:  Option<Uuid>,
    pub firmware_version:  Option<String>,
    pub last_seen_at:      Option<DateTime<Utc>>,
    pub state:             DeviceState,
    pub tenant_id:         Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceState {
    /// Created in tenant; activation code issued but not yet redeemed.
    Pending,
    /// Activation code redeemed; device is online and reachable.
    Active,
    /// Device hasn't checked in for > 30 days.
    Stale,
    /// Disabled by admin; client refuses to authenticate.
    Suspended,
    /// Pending decommission; will be removed at next sweep.
    Decommissioning,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NewDevice {
    /// Zebra device serial. Required.
    pub serial:           String,
    pub model:            String,
    pub assigned_user_id: Option<Uuid>,
    pub department_id:    Option<Uuid>,
}

// ─── Activation code ──────────────────────────────────────────────────

/// One-time activation code emitted by the Provisioning API.
///
/// Delivered to the device by EMM (typically as a managed configuration
/// value on the PTT Pro client APK). The client redeems it on first
/// launch, exchanging it for a long-lived device token.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationCode {
    pub code:             String,
    pub user_id:          Uuid,
    pub device_serial:    String,
    pub issued_at:        DateTime<Utc>,
    pub expires_at:       DateTime<Utc>,
    pub redeemed:         bool,
    pub redeemed_at:      Option<DateTime<Utc>>,
}

// ─── Filter helpers ───────────────────────────────────────────────────

/// Common query parameters for list endpoints.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFilter {
    pub page:      Option<u32>,
    pub page_size: Option<u32>,
    pub search:    Option<String>,
    pub sort_by:   Option<String>,
    pub sort_dir:  Option<SortDir>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDir { Asc, Desc }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_serialises_lowercase() {
        let r = serde_json::to_string(&Role::Supervisor).unwrap();
        assert_eq!(r, "\"supervisor\"");
    }

    #[test]
    fn group_kind_uses_snake_case() {
        let k = serde_json::to_string(&GroupKind::AdHoc).unwrap();
        assert_eq!(k, "\"ad_hoc\"");
    }

    #[test]
    fn page_has_next_logic() {
        let p = Page::<u8> { items: vec![], page: 0, page_size: 10,
                             total_items: 25, total_pages: 3 };
        assert!(p.has_next());
        let last = Page::<u8> { items: vec![], page: 2, page_size: 10,
                                total_items: 25, total_pages: 3 };
        assert!(!last.has_next());
    }

    #[test]
    fn new_user_camel_case_wire() {
        let u = NewUser {
            username: "alice".into(),
            display_name: "Alice Cooper".into(),
            ..Default::default()
        };
        let s = serde_json::to_string(&u).unwrap();
        assert!(s.contains("\"displayName\":\"Alice Cooper\""));
        assert!(s.contains("\"username\":\"alice\""));
    }

    #[test]
    fn device_state_serialises_snake_case() {
        let s = serde_json::to_string(&DeviceState::Decommissioning).unwrap();
        assert_eq!(s, "\"decommissioning\"");
    }
}
