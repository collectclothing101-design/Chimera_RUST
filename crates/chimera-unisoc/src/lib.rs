// chimera-unisoc/src/lib.rs
// Unisoc/Spreadtrum (SPD) device support
// Supports PAC firmware format, BROM mode

pub mod brom;
pub mod pac;
pub mod operations;
pub use operations::UnisocOperations;
