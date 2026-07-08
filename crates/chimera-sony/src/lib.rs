// chimera-sony/src/lib.rs
// Sony Xperia device support: TA partition, trim area, DRM keys, bootloader unlock

pub mod operations;
pub mod ta_partition;
pub mod bootloader;
pub mod flash;

pub use operations::SonyOperations;
