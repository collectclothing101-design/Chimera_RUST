// chimera-fastboot/src/variables.rs
// Known fastboot variable names

pub const VAR_PRODUCT: &str = "product";
pub const VAR_VERSION: &str = "version";
pub const VAR_VERSION_BOOTLOADER: &str = "version-bootloader";
pub const VAR_VERSION_BASEBAND: &str = "version-baseband";
pub const VAR_SERIALNO: &str = "serialno";
pub const VAR_SECURE: &str = "secure";
pub const VAR_UNLOCKED: &str = "unlocked";
pub const VAR_CURRENT_SLOT: &str = "current-slot";
pub const VAR_SLOT_SUFFIXES: &str = "slot-suffixes";
pub const VAR_SLOT_COUNT: &str = "slot-count";
pub const VAR_HAS_SLOT_BOOT: &str = "has-slot:boot";
pub const VAR_HAS_SLOT_SYSTEM: &str = "has-slot:system";
pub const VAR_PARTITION_SIZE: &str = "partition-size";
pub const VAR_PARTITION_TYPE: &str = "partition-type";
pub const VAR_MAX_DOWNLOAD_SIZE: &str = "max-download-size";
pub const VAR_IS_USERSPACE: &str = "is-userspace";
pub const VAR_BATTERY_VOLTAGE: &str = "battery-voltage";

pub fn get_partition_size_var(partition: &str) -> String {
    format!("partition-size:{}", partition)
}

pub fn get_partition_type_var(partition: &str) -> String {
    format!("partition-type:{}", partition)
}
