// chimera-utils/src/lib.rs
// Utility functions for ChimeraRS

pub mod imei_check;
pub mod network_codes;
pub mod au_network_unlock;
pub mod qr_code;
pub mod tap_diag;
pub mod magisk;
pub mod host_probes;

pub use imei_check::ImeiChecker;
pub use network_codes::NetworkCodeCalculator;
pub use au_network_unlock::{AuCarrierRecord, AU_CARRIER_DB, detect_carrier_from_mccmnc,
                             lookup_by_name, lookup_by_mnc, calculate_samsung_nck_au,
                             calculate_lg_nck_au, calculate_motorola_nck_au,
                             AuUnlockInstructions, apply_nck_via_adb};
pub use qr_code::QrCodeGenerator;
pub use host_probes::{ToolProbe, HostToolProbes,
                       detect_adb, detect_fastboot, detect_irecovery,
                       detect_idevice_id, detect_palera1n, detect_futurerestore};
