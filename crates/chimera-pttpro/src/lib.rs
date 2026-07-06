//! `chimera-pttpro` — Zebra Workforce Connect PTT Pro client.
//!
//! Targets two API surfaces:
//!
//! 1. **IT-Admin REST API** — tenant management. Create / update / delete
//!    users, groups, contacts, departments. Assign roles and permissions.
//!
//! 2. **Provisioning API** — bulk device enrollment. Generate activation
//!    codes, bind devices to users, push device-configuration bundles.
//!
//! Both surfaces hang off a single `https://api.ptt.zebra.com/<tenant>/...`
//! base URL with bearer-token auth.
//!
//! ## Quick start
//!
//! ```no_run
//! use chimera_pttpro::{Client, Credentials};
//!
//! # async fn demo() -> anyhow::Result<()> {
//! let client = Client::new("https://api.ptt.zebra.com", "acme-corp")?
//!     .with_credentials(Credentials::client_credentials(
//!         "your-client-id",
//!         "your-client-secret",
//!     ));
//! client.authenticate().await?;
//!
//! // List every user in the tenant
//! let users = client.users().list(None).await?;
//! println!("{} users in tenant", users.len());
//!
//! // Provision a brand-new device for a user
//! let code = client.activation()
//!     .generate(&users[0].id, "TC53-SERIAL-12345")
//!     .await?;
//! println!("Activation code: {}", code.code);
//! # Ok(()) }
//! ```
//!
//! ## Offline / mock-server development
//!
//! With the `mock` feature enabled, [`mock::MockServer`] spins up a local
//! HTTP server that mimics the documented endpoints. Tests in this crate
//! use it; you can use it from your own integration tests too.
//!
//! ## API spec source notes
//!
//! Zebra's full OpenAPI spec is gated behind the Zebra Partner Portal. The
//! endpoint shapes in this crate are modelled from:
//!   - Zebra TechDocs (techdocs.zebra.com/ptt-pro/)
//!   - The Android / iOS Workforce Connect SDK public method signatures
//!   - Generic Workforce Connect REST patterns used by adjacent products
//!
//! When you get the official spec for your tenant, compare against the
//! request/response models in [`models`] and adjust per-tenant.

#![allow(missing_docs)]
#![allow(rustdoc::bare_urls)]

pub mod error;
pub mod client;
pub mod models;
pub mod users;
pub mod groups;
pub mod contacts;
pub mod departments;
pub mod devices;
pub mod activation;

#[cfg(feature = "bulk")]
pub mod bulk;

#[cfg(feature = "mock")]
pub mod mock;

pub use client::{Client, Credentials, TokenResponse};
pub use error::{Error, Result};
pub use models::{
    User, NewUser, Group, NewGroup, Contact, Department, Device, NewDevice,
    ActivationCode, Role, Permission, Page,
};
