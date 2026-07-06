// chimera-apple/src/lib.rs
// Apple device support: iPhone/iPad/iPod flashing, iCloud bypass,
// passcode operations, activation lock, and recovery mode management.

pub mod device;
pub mod lockdown;
pub mod recovery;
pub mod usbmuxd;
pub mod restore;
pub mod activation;
pub mod bypass;
pub mod passcode;
pub mod network_unlock;
pub mod operations;
pub mod ipsw;
pub mod shsh;
/// AU carrier-unlock module is disabled by default.
/// Enable with: cargo build --features au_carrier_unlock
#[cfg(feature = "au_carrier_unlock")]
pub mod au_carrier_unlock;
pub mod icloud_endpoints;

pub use operations::AppleOperations;
pub use device::{AppleDevice, AppleDeviceInfo, IosConnectionMode, AppleChipset};
pub use activation::{ActivationLockStatus, ActivationInfo};
pub use restore::IpswRestoreOptions;
pub use shsh::{Shsh2Blob, BlobStore, TssClient, IpswMeClient,
               DowngradeCompatibilityReport, FutureRestoreBuilder,
               NonceGenerator, nonce_generator_instructions, ShshErrorCatalogue};
#[cfg(feature = "au_carrier_unlock")]
pub use au_carrier_unlock::{AuCarrier, AU_CARRIERS, AuUnlockRequest, UnlockRequestStatus,
                             AuIphoneUnlockWizard, validate_imei, is_apple_imei,
                             lookup_by_mccmnc, lookup_by_name, detect_carrier_from_device,
                             UnlockGuide};
pub use bypass::{BypassMethod, BypassResult};
pub use icloud_endpoints::{ICloudEndpoint, ICloudEndpointRole, ICLOUD_ENDPOINTS,
               endpoints_by_role, endpoints_with_ips, probe_confirmed_endpoints,
               find_by_fqdn, restore_relevant_endpoints, au_unlock_relevant_endpoints,
               activation_status_url, escrow_proxy_url, mcc_unlock_status_url,
               gateway_australia_url, endpoint_summary, APPLE_ICLOUD_IPV4_BLOCKS};

/// Apple USB Vendor ID
pub const APPLE_VID: u16 = 0x05AC;

/// USB Product IDs for Apple devices in different modes
pub mod apple_pid {
    /// DFU mode (all devices)
    pub const DFU: u16 = 0x1227;
    /// Recovery mode
    pub const RECOVERY: u16 = 0x1281;
    /// iPhone in normal mode (range 0x1290–0x12AF)
    pub const IPHONE_MIN: u16 = 0x1290;
    pub const IPHONE_MAX: u16 = 0x12AF;
    /// iPad in normal mode
    pub const IPAD_MIN: u16 = 0x12AB;
    pub const IPAD_MAX: u16 = 0x12FF;
}
