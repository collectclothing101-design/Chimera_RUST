// Unisoc BROM protocol
use chimera_core::error::{ChimeraError, Result};
use rusb::{GlobalContext, DeviceHandle};
use std::time::Duration;
use log::info;

pub const UNISOC_VID: u16 = 0x1782;
pub const UNISOC_PID: u16 = 0x4D00;

pub struct UnisocBrom {
    handle: DeviceHandle<GlobalContext>,
    ep_in: u8,
    ep_out: u8,
}

impl UnisocBrom {
    pub fn open() -> Result<Self> {
        let devices = rusb::devices().map_err(|e| ChimeraError::Usb(e.to_string()))?;
        for device in devices.iter() {
            let desc = match device.device_descriptor() {
                Ok(d) => d,
                Err(_) => continue,
            };
            if desc.vendor_id() != UNISOC_VID || desc.product_id() != UNISOC_PID { continue; }
            info!("Found Unisoc BROM device");
            
            let config = match device.active_config_descriptor() { Ok(c) => c, Err(_) => continue };
            for iface in config.interfaces() {
                for iface_desc in iface.descriptors() {
                    let mut ep_in = 0u8; let mut ep_out = 0u8; let mut found = (false, false);
                    for ep in iface_desc.endpoint_descriptors() {
                        if ep.transfer_type() == rusb::TransferType::Bulk {
                            match ep.direction() {
                                rusb::Direction::In => { ep_in = ep.address(); found.0 = true; }
                                rusb::Direction::Out => { ep_out = ep.address(); found.1 = true; }
                            }
                        }
                    }
                    if found.0 && found.1 {
                        if let Ok(handle) = device.open() {
                            if handle.kernel_driver_active(iface.number()).unwrap_or(false) {
                                let _ = handle.detach_kernel_driver(iface.number());
                            }
                            if handle.claim_interface(iface.number()).is_ok() {
                                return Ok(Self { handle, ep_in, ep_out });
                            }
                        }
                    }
                }
            }
        }
        Err(ChimeraError::DeviceNotFound("Unisoc BROM not found. Connect device with Vol Down.".into()))
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.handle.write_bulk(self.ep_out, data, Duration::from_secs(10))
            .map_err(|e| ChimeraError::Unisoc(e.to_string()))?;
        Ok(())
    }

    pub fn read(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        let read = self.handle.read_bulk(self.ep_in, &mut buf, Duration::from_secs(10))
            .map_err(|e| ChimeraError::Unisoc(e.to_string()))?;
        buf.truncate(read);
        Ok(buf)
    }
}
