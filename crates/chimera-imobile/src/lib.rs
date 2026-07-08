//! `chimera-imobile` — Rust wrapper for the libimobiledevice CLI toolchain.
//!
//! The libimobiledevice project ships a suite of command-line tools that
//! talk to iOS devices over the lockdownd / usbmuxd protocol. This crate
//! wraps each one as a Rust function so the rest of the workspace can call
//! `idevice_id::list()` and get back a `Vec<UdidEntry>` instead of having
//! to shell out + parse stdout in every call site.
//!
//! ## Resolution
//!
//! Each tool is located via [`chimera_utils::host_probes`]: env-var override
//! first (e.g. `CHIMERA_IDEVICE_ID`), then `$PATH`, then common install
//! prefixes (Homebrew Intel/Apple Silicon, Linux system, Windows
//! `Program Files`).
//!
//! ## Bundling on Windows
//!
//! The user's RAMDISK toolchain (Windows .exe set compiled by iFred09 in May
//! 2020) is bundled at `Chimera.app/Contents/Resources/idevice/` for
//! Windows builds. macOS builds fall back to `brew install libimobiledevice`.
//!
//! ## Modules
//!
//! - [`tool`]       — discover + resolve binaries
//! - [`idevice_id`] — list connected iOS UDIDs
//! - [`info`]       — read lockdownd values via `ideviceinfo`
//! - [`activation`] — fetch / push activation records
//! - [`backup`]     — backup + restore via `idevicebackup2`
//! - [`restore`]    — flash IPSW via `idevicerestore`
//! - [`recovery`]   — enter/exit recovery via `ideviceenterrecovery` + `irecovery`
//! - [`diagnostics`]— battery/storage/thermals via `idevicediagnostics`
//! - [`pair`]       — pairing operations via `idevicepair`

pub mod tool;
pub mod idevice_id;
pub mod info;
pub mod activation;
pub mod backup;
pub mod restore;
pub mod recovery;
pub mod diagnostics;
pub mod pair;

pub use tool::{ImobileTool, ImobileError, resolve, run};

// Convenience re-exports
pub use idevice_id::{list as list_devices, UdidEntry};
pub use info::{ideviceinfo, DeviceProperties};
pub use pair::{pair, unpair, validate, PairResult};
pub use recovery::{enter_recovery, exit_recovery, RecoveryAction};
pub use activation::{fetch_activation, ActivationState};

/// Crate-wide result alias.
pub type Result<T> = std::result::Result<T, ImobileError>;
