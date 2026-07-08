// chimera-mtk/src/da_protocol.rs
// MediaTek Download Agent (DA) protocol implementation
// Communicates with MTK devices in BootROM mode

use chimera_core::error::{ChimeraError, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use rusb::{GlobalContext, DeviceHandle};
use std::time::Duration;
use std::io::Cursor;
use log::info;

// MTK USB IDs
pub const MTK_VID: u16 = 0x0E8D;
pub const MTK_PID_PRELOADER: u16 = 0x0003;
pub const MTK_PID_BROM: u16 = 0x2000;

// MTK DA Commands
pub const CMD_START_DA: u16 = 0x0001;
pub const CMD_DOWNLOAD_BLOADER: u16 = 0x0002;
pub const CMD_JUMP_DA: u16 = 0x0004;
pub const CMD_GET_TARGET_INFO: u16 = 0x0005;
pub const CMD_SEND_CERT: u16 = 0x0006;
pub const CMD_SEND_AUTH: u16 = 0x0007;
pub const CMD_FORMAT_FLASH: u16 = 0x0085;
pub const CMD_DOWNLOAD_NORMAL: u16 = 0x0082;
pub const CMD_WRITE_NVRAM: u16 = 0x0047;
pub const CMD_READ_NVRAM: u16 = 0x0048;

// Status codes
pub const STATUS_OK: u8 = 0x00;
pub const STATUS_READY: u8 = 0x5A;
pub const STATUS_NAK: u8 = 0xA5;

/// MTK device information
#[derive(Debug, Clone)]
pub struct MtkDeviceInfo {
    pub chip_id: u16,
    pub hw_version: u16,
    pub hw_sub_code: u16,
    pub hw_version_code: u16,
    pub hw_type: u16,
    pub preloader_version: u16,
    pub brom_version: u8,
}

/// MTK DA Client
pub struct MtkDaClient {
    handle: DeviceHandle<GlobalContext>,
    ep_in: u8,
    ep_out: u8,
}

impl MtkDaClient {
    /// Find and open MTK device
    pub fn open() -> Result<Self> {
        let devices = rusb::devices()
            .map_err(|e| ChimeraError::Usb(e.to_string()))?;
        
        for device in devices.iter() {
            let desc = match device.device_descriptor() {
                Ok(d) => d,
                Err(_) => continue,
            };
            
            if desc.vendor_id() != MTK_VID {
                continue;
            }
            
            if !matches!(desc.product_id(), 0x0003 | 0x2000 | 0x2001) {
                continue;
            }
            
            info!("Found MTK device: {:04x}:{:04x}", desc.vendor_id(), desc.product_id());
            
            let config = match device.active_config_descriptor() {
                Ok(c) => c,
                Err(_) => continue,
            };
            
            for iface in config.interfaces() {
                for iface_desc in iface.descriptors() {
                    let mut ep_in = 0u8;
                    let mut ep_out = 0u8;
                    let mut found = (false, false);
                    
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
        
        Err(ChimeraError::DeviceNotFound("MTK BootROM device not found. Short test points and connect USB.".into()))
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.handle.write_bulk(self.ep_out, data, Duration::from_secs(10))
            .map_err(|e| ChimeraError::Mtk(e.to_string()))?;
        Ok(())
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        let read = self.handle.read_bulk(self.ep_in, &mut buf, Duration::from_secs(10))
            .map_err(|e| ChimeraError::Mtk(e.to_string()))?;
        buf.truncate(read);
        Ok(buf)
    }

    fn read_exact(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut result = Vec::with_capacity(len);
        while result.len() < len {
            let chunk = self.read(len - result.len())?;
            if chunk.is_empty() {
                return Err(ChimeraError::Mtk("Read timeout".into()));
            }
            result.extend_from_slice(&chunk);
        }
        Ok(result)
    }

    /// Handshake with MTK BootROM
    pub fn handshake(&mut self) -> Result<()> {
        // Send start byte
        self.write(&[0xA0])?;
        let resp = self.read_exact(1)?;
        if resp[0] != 0x5F {
            return Err(ChimeraError::Mtk(format!("Handshake failed step 1: 0x{:02x}", resp[0])));
        }
        
        self.write(&[0x0A])?;
        let resp = self.read_exact(1)?;
        if resp[0] != 0xF5 {
            return Err(ChimeraError::Mtk(format!("Handshake failed step 2: 0x{:02x}", resp[0])));
        }
        
        self.write(&[0x50])?;
        let resp = self.read_exact(1)?;
        if resp[0] != 0xAF {
            return Err(ChimeraError::Mtk(format!("Handshake failed step 3: 0x{:02x}", resp[0])));
        }
        
        self.write(&[0x05])?;
        let resp = self.read_exact(1)?;
        if resp[0] != 0xFA {
            return Err(ChimeraError::Mtk(format!("Handshake failed step 4: 0x{:02x}", resp[0])));
        }
        
        info!("MTK BootROM handshake successful");
        Ok(())
    }

    /// Get device hardware info
    pub fn get_hw_info(&mut self) -> Result<MtkDeviceInfo> {
        // CMD_GET_TARGET_INFO
        let mut cmd = Vec::new();
        cmd.write_u16::<BigEndian>(CMD_GET_TARGET_INFO).unwrap();
        self.write(&cmd)?;
        
        let resp = self.read_exact(28)?;
        let mut cursor = Cursor::new(&resp);
        
        Ok(MtkDeviceInfo {
            chip_id: cursor.read_u16::<BigEndian>().unwrap_or(0),
            hw_version: cursor.read_u16::<BigEndian>().unwrap_or(0),
            hw_sub_code: cursor.read_u16::<BigEndian>().unwrap_or(0),
            hw_version_code: cursor.read_u16::<BigEndian>().unwrap_or(0),
            hw_type: cursor.read_u16::<BigEndian>().unwrap_or(0),
            preloader_version: cursor.read_u16::<BigEndian>().unwrap_or(0),
            brom_version: cursor.read_u8().unwrap_or(0),
        })
    }

    /// Send Download Agent binary
    pub fn send_da(&mut self, da_binary: &[u8], target_address: u32) -> Result<()> {
        let da_len = da_binary.len() as u32;
        
        let mut cmd = Vec::new();
        cmd.write_u16::<BigEndian>(CMD_START_DA).unwrap();
        cmd.write_u32::<BigEndian>(target_address).unwrap(); // DA load address
        cmd.write_u32::<BigEndian>(da_len).unwrap();
        cmd.write_u32::<BigEndian>(0).unwrap(); // signature length
        self.write(&cmd)?;
        
        // Read status
        let status = self.read_exact(2)?;
        if status[0] != STATUS_OK {
            return Err(ChimeraError::Mtk(format!("DA send failed: 0x{:02x}", status[0])));
        }
        
        // Send DA binary
        self.write(da_binary)?;
        
        // Read checksum
        let _checksum = self.read_exact(4)?;
        
        info!("DA sent successfully ({} bytes)", da_len);
        Ok(())
    }

    /// Jump to DA
    pub fn jump_da(&mut self, target_address: u32) -> Result<()> {
        let mut cmd = Vec::new();
        cmd.write_u16::<BigEndian>(CMD_JUMP_DA).unwrap();
        cmd.write_u32::<BigEndian>(target_address).unwrap();
        self.write(&cmd)?;
        
        let status = self.read_exact(2)?;
        if status[0] != STATUS_OK {
            return Err(ChimeraError::Mtk(format!("Jump DA failed: 0x{:02x}", status[0])));
        }
        
        info!("Jumped to DA at 0x{:08x}", target_address);
        Ok(())
    }

    /// Write NVRAM data (for IMEI repair)
    pub fn write_nvram(&mut self, lid: u16, data: &[u8]) -> Result<()> {
        let mut cmd = Vec::new();
        cmd.write_u16::<BigEndian>(CMD_WRITE_NVRAM).unwrap();
        cmd.write_u16::<BigEndian>(lid).unwrap();
        cmd.write_u16::<BigEndian>(data.len() as u16).unwrap();
        self.write(&cmd)?;
        
        let status = self.read_exact(2)?;
        if status[0] != STATUS_OK {
            return Err(ChimeraError::Mtk("NVRAM write init failed".into()));
        }
        
        self.write(data)?;
        
        // Checksum
        let csum: u8 = data.iter().fold(0u8, |a, b| a.wrapping_add(*b));
        self.write(&[csum])?;
        
        let final_status = self.read_exact(2)?;
        if final_status[0] != STATUS_OK {
            return Err(ChimeraError::Mtk("NVRAM write failed".into()));
        }
        
        Ok(())
    }

    /// Read NVRAM data
    pub fn read_nvram(&mut self, lid: u16) -> Result<Vec<u8>> {
        let mut cmd = Vec::new();
        cmd.write_u16::<BigEndian>(CMD_READ_NVRAM).unwrap();
        cmd.write_u16::<BigEndian>(lid).unwrap();
        self.write(&cmd)?;
        
        let status = self.read_exact(2)?;
        if status[0] != STATUS_OK {
            return Err(ChimeraError::Mtk("NVRAM read failed".into()));
        }
        
        let len_bytes = self.read_exact(2)?;
        let len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
        
        let data = self.read_exact(len)?;
        let _checksum = self.read_exact(1)?;
        
        Ok(data)
    }
}
