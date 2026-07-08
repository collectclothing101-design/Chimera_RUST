// chimera-mtk/src/lib.rs
// MediaTek (MTK) device support
// Full DA (Download Agent) protocol implementation
// Supports: Helio, Dimensity, MT67xx, MT68xx chipsets

pub mod preloader;
pub mod da_protocol;
pub mod operations;
pub mod brom;

pub use operations::MtkOperations;
pub use da_protocol::MtkDaClient;
