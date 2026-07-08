// chimera-edl/src/usb.rs
// USB communication for EDL mode (Qualcomm 9008 interface)

use chimera_core::error::{ChimeraError, Result};
use rusb::{GlobalContext, DeviceHandle};
use std::time::Duration;
use log::{debug, info};

pub const EDL_VID: u16 = 0x05C6;
pub const EDL_PID: u16 = 0x9008;
pub const EDL_PID_COMPOSITE: u16 = 0x900E;
pub const EDL_TIMEOUT: Duration = Duration::from_secs(30);
pub const EDL_FLASH_TIMEOUT: Duration = Duration::from_secs(300);

pub struct EdlUsb {
    pub handle: DeviceHandle<GlobalContext>,
    pub ep_in: u8,
    pub ep_out: u8,
    pub if_num: u8,
}

impl EdlUsb {
    /// Find and open EDL device
    pub fn open() -> Result<Self> {
        let devices = rusb::devices()
            .map_err(|e| ChimeraError::Usb(e.to_string()))?;
        
        for device in devices.iter() {
            let desc = match device.device_descriptor() {
                Ok(d) => d,
                Err(_) => continue,
            };
            
            let is_edl = (desc.vendor_id() == EDL_VID && desc.product_id() == EDL_PID)
                || (desc.vendor_id() == EDL_VID && desc.product_id() == EDL_PID_COMPOSITE);
            
            if !is_edl {
                continue;
            }
            
            info!("Found EDL device: {:04x}:{:04x}", desc.vendor_id(), desc.product_id());
            
            let config = match device.active_config_descriptor() {
                Ok(c) => c,
                Err(_) => continue,
            };
            
            for iface in config.interfaces() {
                for iface_desc in iface.descriptors() {
                    // EDL uses bulk endpoints on any interface
                    let mut ep_in = 0u8;
                    let mut ep_out = 0u8;
                    let mut has_bulk_in = false;
                    let mut has_bulk_out = false;
                    
                    for ep in iface_desc.endpoint_descriptors() {
                        if ep.transfer_type() == rusb::TransferType::Bulk {
                            match ep.direction() {
                                rusb::Direction::In => {
                                    ep_in = ep.address();
                                    has_bulk_in = true;
                                }
                                rusb::Direction::Out => {
                                    ep_out = ep.address();
                                    has_bulk_out = true;
                                }
                            }
                        }
                    }
                    
                    if has_bulk_in && has_bulk_out {
                        if let Ok(handle) = device.open() {
                            // Detach kernel driver if needed
                            if handle.kernel_driver_active(iface.number()).unwrap_or(false) {
                                let _ = handle.detach_kernel_driver(iface.number());
                            }
                            
                            if handle.claim_interface(iface.number()).is_ok() {
                                debug!("EDL USB: ep_in=0x{:02x} ep_out=0x{:02x}", ep_in, ep_out);
                                return Ok(Self {
                                    handle,
                                    ep_in,
                                    ep_out,
                                    if_num: iface.number(),
                                });
                            }
                        }
                    }
                }
            }
        }
        
        Err(ChimeraError::DeviceNotFound("EDL device (Qualcomm 9008) not found. Ensure device is in EDL mode.".into()))
    }

    /// Write data
    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        let written = self.handle
            .write_bulk(self.ep_out, data, EDL_TIMEOUT)
            .map_err(|e| ChimeraError::Usb(format!("EDL write error: {}", e)))?;
        Ok(written)
    }

    /// Write with larger timeout for flashing
    pub fn write_flash(&mut self, data: &[u8]) -> Result<usize> {
        let written = self.handle
            .write_bulk(self.ep_out, data, EDL_FLASH_TIMEOUT)
            .map_err(|e| ChimeraError::Usb(format!("EDL flash write error: {}", e)))?;
        Ok(written)
    }

    /// Read data  
    pub fn read(&mut self, max_len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; max_len];
        let read = self.handle
            .read_bulk(self.ep_in, &mut buf, EDL_TIMEOUT)
            .map_err(|e| ChimeraError::Usb(format!("EDL read error: {}", e)))?;
        buf.truncate(read);
        Ok(buf)
    }

    /// Read with larger timeout
    pub fn read_timeout(&mut self, max_len: usize, timeout: Duration) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; max_len];
        let read = self.handle
            .read_bulk(self.ep_in, &mut buf, timeout)
            .map_err(|e| ChimeraError::Usb(format!("EDL read error: {}", e)))?;
        buf.truncate(read);
        Ok(buf)
    }

    /// Write XML command and read all XML responses
    pub fn exchange_xml(&mut self, cmd: &str) -> Result<Vec<String>> {
        debug!("EDL TX: {}", cmd);
        self.write(cmd.as_bytes())?;
        
        let mut responses = Vec::new();
        let mut full_buf = String::new();
        
        for _ in 0..100 {
            match self.read(65536) {
                Ok(data) if !data.is_empty() => {
                    let text = String::from_utf8_lossy(&data).to_string();
                    full_buf.push_str(&text);
                    
                    // Split on </data> boundaries
                    while let Some(end) = full_buf.find("</data>") {
                        let response = full_buf[..end + 7].to_string();
                        full_buf = full_buf[end + 7..].to_string();
                        debug!("EDL RX: {}", response);
                        
                        let parsed = crate::firehose::parse_response(&response);
                        let done = matches!(parsed, crate::firehose::FirehoseResponse::Ack | crate::firehose::FirehoseResponse::Nak(_));
                        responses.push(response);
                        
                        if done {
                            return Ok(responses);
                        }
                    }
                }
                Ok(_) => {
                    // Empty read, try again
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
        
        Ok(responses)
    }
}
