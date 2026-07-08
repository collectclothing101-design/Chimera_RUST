// chimera-edl/src/lib.rs
// Qualcomm EDL (Emergency Download Mode) full implementation
// Covers Sahara protocol (init) + Firehose protocol (operations)

pub mod sahara;
pub mod firehose;
pub mod client;
pub mod usb;
pub mod operations;

pub use client::EdlClient;
pub use sahara::{SaharaProtocol, SaharaState};
pub use firehose::{FirehoseProtocol};
pub use operations::EdlOperations;
