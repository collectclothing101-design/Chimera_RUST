// chimera-fastboot/src/lib.rs
// Full Fastboot protocol implementation
// Handles USB communication with devices in fastboot/bootloader mode

pub mod protocol;
pub mod client;
pub mod flash;
pub mod variables;

pub use client::FastbootClient;
pub use protocol::{FastbootResponse, FastbootCommand};
