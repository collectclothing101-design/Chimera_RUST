// chimera-devices/src/lib.rs
// Device database, auto-detection, and supported models registry

pub mod detector;
pub mod database;
pub mod scanner;

pub use detector::DeviceDetector;
pub use database::DeviceDatabase;
pub use scanner::UsbScanner;
