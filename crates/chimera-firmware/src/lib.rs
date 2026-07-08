// chimera-firmware/src/lib.rs
// Firmware management: download, verify, extract

pub mod extractor;
pub mod downloader;
pub mod samsung_fw;
pub mod checker;

pub use extractor::FirmwareExtractor;
pub use downloader::FirmwareDownloader;
pub use checker::FirmwareChecker;
