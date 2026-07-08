// chimera-core/src/protocol.rs
// Abstract protocol traits that all device backends implement

use crate::device::DeviceInfo;
use crate::error::Result;
use async_trait::async_trait;
use bytes::Bytes;

/// Any protocol handler must implement this
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    /// Send raw data
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    /// Receive raw data with timeout
    async fn recv(&mut self, timeout_ms: u64) -> Result<Vec<u8>>;
    /// Send and receive
    async fn exchange(&mut self, data: &[u8], timeout_ms: u64) -> Result<Vec<u8>> {
        self.send(data).await?;
        self.recv(timeout_ms).await
    }
    /// Check if still connected
    fn is_connected(&self) -> bool;
    /// Close the connection
    async fn close(&mut self) -> Result<()>;
}

/// All device operations implement this
#[async_trait]
pub trait DeviceOperations: Send + Sync {
    /// Get device information
    async fn get_info(&mut self) -> Result<DeviceInfo>;
    
    /// Factory reset the device
    async fn factory_reset(&mut self) -> Result<()>;
    
    /// Remove FRP (Factory Reset Protection)
    async fn remove_frp(&mut self) -> Result<()>;
    
    /// Repair IMEI number
    async fn repair_imei(&mut self, imei: &str, slot: u8) -> Result<()>;
    
    /// Repair IMEI using patch method
    async fn repair_imei_patch(&mut self, imei: &str, slot: u8) -> Result<()>;
    
    /// Store backup of security partition
    async fn store_backup(&mut self, path: &str) -> Result<()>;
    
    /// Restore backup from file
    async fn restore_backup(&mut self, path: &str) -> Result<()>;
    
    /// Update firmware
    async fn update_firmware(&mut self, firmware_path: &str) -> Result<()>;
}

/// USB packet structure
#[derive(Debug, Clone)]
pub struct UsbPacket {
    pub endpoint: u8,
    pub data: Bytes,
}

/// Generic command-response pair
#[derive(Debug, Clone)]
pub struct CommandResponse {
    pub command: String,
    pub response: String,
    pub success: bool,
}
