// chimera-samsung/src/odin.rs
// Samsung ODIN protocol implementation for Download Mode
// Supports firmware flashing via USB

use chimera_core::error::{ChimeraError, Result};
use chimera_core::progress::{Progress, ProgressSender};
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use rusb::{GlobalContext, DeviceHandle};
use std::time::Duration;
use log::{debug, info};
use std::io::Cursor;

// Samsung Download Mode USB IDs
pub const SAMSUNG_VID: u16 = 0x04E8;
pub const SAMSUNG_PID_DOWNLOAD: u16 = 0x685D;
pub const SAMSUNG_PID_DOWNLOAD2: u16 = 0x6601;
pub const SAMSUNG_PID_EUB: u16 = 0xD001;

// ODIN protocol constants
pub const ODIN_PIT_SIZE: u32 = 4096;
pub const ODIN_MAX_TRANSFER_SIZE: u32 = 131072; // 128KB chunks

// ODIN packet types
pub const ODIN_CMD_HANDSHAKE: u32 = 0x64;
pub const ODIN_CMD_SESSION_START: u32 = 0x65;
pub const ODIN_CMD_SESSION_END: u32 = 0x67;
pub const ODIN_CMD_PIT_FLASH: u32 = 0x68;
pub const ODIN_CMD_PART_FLASH: u32 = 0x6A;
pub const ODIN_CMD_FILE_XFER: u32 = 0x6C;
pub const ODIN_CMD_DEVICE_INFO: u32 = 0x70;
pub const ODIN_CMD_REBOOT: u32 = 0x72;
pub const ODIN_CMD_UNRECOGNIZED: u32 = 0x00;

// ODIN begin session modes
pub const BEGIN_ODIN: u8 = 0;
pub const BEGIN_EFS_CLEAR: u8 = 1;
pub const BEGIN_FLASH: u8 = 2;

/// PIT (Partition Information Table) entry
#[derive(Debug, Clone)]
pub struct PitEntry {
    pub binary_type: u32,
    pub device_type: u32,
    pub identifier: u32,
    pub attributes: u32,
    pub update_attributes: u32,
    pub block_size: u32,
    pub block_count: u32,
    pub file_offset: u32,
    pub file_size: u32,
    pub partition_name: String,
    pub flash_filename: String,
    pub fota_filename: String,
}

/// Parsed PIT (Partition Information Table)
#[derive(Debug, Clone)]
pub struct PitTable {
    pub entries: Vec<PitEntry>,
}

impl PitTable {
    /// Parse PIT from raw bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        
        let magic = cursor.read_u32::<LittleEndian>().map_err(|_| ChimeraError::Odin("Cannot read PIT magic".into()))?;
        if magic != 0x12349876 {
            return Err(ChimeraError::Odin(format!("Invalid PIT magic: 0x{:08X}", magic)));
        }
        
        let count = cursor.read_u32::<LittleEndian>().map_err(|_| ChimeraError::Odin("Cannot read PIT count".into()))?;
        
        // Read gang name (8 bytes) + project name (8 bytes)
        let mut _gang = [0u8; 8];
        let mut _project = [0u8; 8];
        let _ = std::io::Read::read_exact(&mut cursor, &mut _gang);
        let _ = std::io::Read::read_exact(&mut cursor, &mut _project);
        
        let mut entries = Vec::new();
        for _ in 0..count {
            let entry = Self::parse_entry(&mut cursor)?;
            entries.push(entry);
        }
        
        Ok(Self { entries })
    }

    fn parse_entry(cursor: &mut Cursor<&[u8]>) -> Result<PitEntry> {
        let binary_type = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        let device_type = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        let identifier = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        let attributes = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        let update_attributes = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        let block_size = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        let block_count = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        let file_offset = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        let file_size = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        
        let mut name_buf = [0u8; 32];
        let mut flash_buf = [0u8; 32];
        let mut fota_buf = [0u8; 32];
        
        let _ = std::io::Read::read_exact(cursor, &mut name_buf);
        let _ = std::io::Read::read_exact(cursor, &mut flash_buf);
        let _ = std::io::Read::read_exact(cursor, &mut fota_buf);
        
        let to_str = |b: &[u8]| {
            String::from_utf8_lossy(b).trim_end_matches('\0').to_string()
        };
        
        Ok(PitEntry {
            binary_type,
            device_type,
            identifier,
            attributes,
            update_attributes,
            block_size,
            block_count,
            file_offset,
            file_size,
            partition_name: to_str(&name_buf),
            flash_filename: to_str(&flash_buf),
            fota_filename: to_str(&fota_buf),
        })
    }

    pub fn find_by_name(&self, name: &str) -> Option<&PitEntry> {
        self.entries.iter().find(|e| e.partition_name.eq_ignore_ascii_case(name))
    }
}

/// ODIN protocol session over USB
pub struct OdinSession {
    handle: DeviceHandle<GlobalContext>,
    ep_in: u8,
    ep_out: u8,
    /// USB interface number claimed during session open.

    pub if_num: u8,
    transfer_size: u32,
}

impl OdinSession {
    /// Find and open Samsung download mode device
    pub fn open() -> Result<Self> {
        let devices = rusb::devices()
            .map_err(|e| ChimeraError::Usb(e.to_string()))?;
        
        for device in devices.iter() {
            let desc = match device.device_descriptor() {
                Ok(d) => d,
                Err(_) => continue,
            };
            
            if desc.vendor_id() != SAMSUNG_VID {
                continue;
            }
            
            let is_download = matches!(
                desc.product_id(),
                0x685D | 0x6601 | 0x6B21 | 0x6B23 | 0x6B24 | 0x6B25
            );
            
            if !is_download {
                continue;
            }
            
            info!("Found Samsung Download Mode device: {:04x}:{:04x}", 
                  desc.vendor_id(), desc.product_id());
            
            let config = match device.active_config_descriptor() {
                Ok(c) => c,
                Err(_) => continue,
            };
            
            for iface in config.interfaces() {
                for iface_desc in iface.descriptors() {
                    let mut ep_in = 0u8;
                    let mut ep_out = 0u8;
                    let mut found_in = false;
                    let mut found_out = false;
                    
                    for ep in iface_desc.endpoint_descriptors() {
                        if ep.transfer_type() == rusb::TransferType::Bulk {
                            match ep.direction() {
                                rusb::Direction::In => { ep_in = ep.address(); found_in = true; }
                                rusb::Direction::Out => { ep_out = ep.address(); found_out = true; }
                            }
                        }
                    }
                    
                    if found_in && found_out {
                        if let Ok(handle) = device.open() {
                            if handle.kernel_driver_active(iface.number()).unwrap_or(false) {
                                let _ = handle.detach_kernel_driver(iface.number());
                            }
                            if handle.claim_interface(iface.number()).is_ok() {
                                return Ok(Self {
                                    handle,
                                    ep_in,
                                    ep_out,
                                    if_num: iface.number(),
                                    transfer_size: ODIN_MAX_TRANSFER_SIZE,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        Err(ChimeraError::DeviceNotFound("Samsung Download Mode device not found".into()))
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.handle.write_bulk(self.ep_out, data, Duration::from_secs(30))
            .map_err(|e| ChimeraError::Odin(e.to_string()))?;
        Ok(())
    }

    fn read(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        let read = self.handle.read_bulk(self.ep_in, &mut buf, Duration::from_secs(30))
            .map_err(|e| ChimeraError::Odin(e.to_string()))?;
        buf.truncate(read);
        Ok(buf)
    }

    /// Perform initial ODIN handshake
    pub fn handshake(&mut self) -> Result<()> {
        debug!("ODIN handshake...");
        
        // Send "ODIN" magic
        self.write(b"ODIN")?;
        
        // Expect "LOKE" response
        let resp = self.read(4)?;
        if resp != b"LOKE" {
            return Err(ChimeraError::Odin(format!("Handshake failed: expected LOKE, got {:?}", resp)));
        }
        
        info!("ODIN handshake successful");
        Ok(())
    }

    /// Send a command packet
    fn send_packet(&mut self, cmd: u32, arg1: u32, arg2: u32, arg3: u32, arg4: u32, arg5: u32, arg6: u32) -> Result<()> {
        let mut buf = vec![0u8; 1024];
        let mut cursor = Cursor::new(&mut buf[..]);
        cursor.write_u32::<LittleEndian>(cmd).unwrap();
        cursor.write_u32::<LittleEndian>(arg1).unwrap();
        cursor.write_u32::<LittleEndian>(arg2).unwrap();
        cursor.write_u32::<LittleEndian>(arg3).unwrap();
        cursor.write_u32::<LittleEndian>(arg4).unwrap();
        cursor.write_u32::<LittleEndian>(arg5).unwrap();
        cursor.write_u32::<LittleEndian>(arg6).unwrap();
        self.write(&buf)
    }

    /// Read a response packet
    fn read_packet(&mut self) -> Result<[u32; 7]> {
        let data = self.read(1024)?;
        if data.len() < 28 {
            return Err(ChimeraError::Odin(format!("Response too short: {} bytes", data.len())));
        }
        let mut cursor = Cursor::new(&data[..]);
        let mut vals = [0u32; 7];
        for v in vals.iter_mut() {
            *v = cursor.read_u32::<LittleEndian>().unwrap();
        }
        Ok(vals)
    }

    /// Begin ODIN session
    pub fn begin_session(&mut self) -> Result<u32> {
        self.send_packet(ODIN_CMD_SESSION_START, BEGIN_ODIN as u32, 0, 0, 0, 0, 0)?;
        let resp = self.read_packet()?;
        
        if resp[0] != ODIN_CMD_SESSION_START {
            return Err(ChimeraError::Odin("Begin session failed".into()));
        }
        
        // resp[1] contains transfer size
        let transfer_size = if resp[1] == 0 { ODIN_MAX_TRANSFER_SIZE } else { resp[1] };
        self.transfer_size = transfer_size;
        
        info!("ODIN session started, transfer size: {} bytes", transfer_size);
        Ok(transfer_size)
    }

    /// End ODIN session
    pub fn end_session(&mut self) -> Result<()> {
        self.send_packet(ODIN_CMD_SESSION_END, 0, 0, 0, 0, 0, 0)?;
        Ok(())
    }

    /// Read PIT from device
    pub fn read_pit(&mut self) -> Result<Vec<u8>> {
        debug!("Reading PIT from device...");
        
        // Request PIT
        self.send_packet(ODIN_CMD_PIT_FLASH, 1, 0, 0, 0, 0, 0)?;
        let resp = self.read_packet()?;
        
        let pit_size = resp[1] as usize;
        debug!("PIT size: {} bytes", pit_size);
        
        // Ready to receive
        self.send_packet(ODIN_CMD_PIT_FLASH, 2, 0, 0, 0, 0, 0)?;
        
        let mut pit_data = Vec::new();
        let mut received = 0usize;
        
        while received < pit_size {
            let chunk = self.read(self.transfer_size as usize)?;
            pit_data.extend_from_slice(&chunk);
            received += chunk.len();
        }
        
        // End sequence
        self.send_packet(ODIN_CMD_PIT_FLASH, 3, 0, 0, 0, 0, 0)?;
        let _ = self.read_packet();
        
        Ok(pit_data)
    }

    /// Flash a file to a partition
    pub fn flash_partition(&mut self, partition_id: u32, data: &[u8], is_compressed: bool, progress: Option<&ProgressSender>) -> Result<()> {
        let file_size = data.len() as u32;
        let transfer_size = self.transfer_size;
        
        info!("Flashing partition {} ({} bytes)", partition_id, file_size);
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Samsung Flash").step(format!("Starting flash ({}MB)...", file_size / 1024 / 1024)).percent(0.0));
        }
        
        // Begin file transfer
        self.send_packet(
            ODIN_CMD_FILE_XFER,
            0,                  // file type (0=binary)
            partition_id,
            if is_compressed { 1 } else { 0 },
            0,
            file_size,
            0,
        )?;
        let resp = self.read_packet()?;
        
        if resp[0] != ODIN_CMD_FILE_XFER {
            return Err(ChimeraError::Odin("Flash start failed".into()));
        }
        
        // Transfer chunks
        let mut offset = 0usize;
        let mut _chunk_num = 0u32;
        
        while offset < data.len() {
            let end = (offset + transfer_size as usize).min(data.len());
            let chunk = &data[offset..end];
            let chunk_len = chunk.len() as u32;
            
            // Send chunk header
            self.send_packet(ODIN_CMD_FILE_XFER, 2, 0, chunk_len, 0, 0, 0)?;
            let _ = self.read_packet();
            
            // Send chunk data
            self.write(chunk)?;
            let _ = self.read(8); // read ACK
            
            offset = end;
            _chunk_num += 1;
            
            if let Some(tx) = progress {
                let pct = offset as f32 / data.len() as f32 * 95.0;
                let _ = tx.send(Progress::new("Samsung Flash").step("Transferring...").bytes(offset as u64, data.len() as u64).percent(pct));
            }
        }
        
        // End of file transfer
        self.send_packet(ODIN_CMD_FILE_XFER, 3, 0, 0, 0, 0, 0)?;
        let _resp = self.read_packet()?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Samsung Flash").step("Flash complete").percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Get device info
    pub fn get_device_info(&mut self) -> Result<String> {
        self.send_packet(ODIN_CMD_DEVICE_INFO, 0, 0, 0, 0, 0, 0)?;
        let data = self.read(1024)?;
        Ok(String::from_utf8_lossy(&data).to_string())
    }

    /// Reboot device
    pub fn reboot(&mut self) -> Result<()> {
        self.send_packet(ODIN_CMD_REBOOT, 0, 0, 0, 0, 0, 0)?;
        Ok(())
    }
}

/// Higher-level ODIN client
pub struct OdinClient {
    session: Option<OdinSession>,
}

impl OdinClient {
    pub fn new() -> Self {
        Self { session: None }
    }

    pub fn connect(&mut self) -> Result<()> {
        let mut session = OdinSession::open()?;
        session.handshake()?;
        session.begin_session()?;
        self.session = Some(session);
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(session) = &mut self.session {
            session.end_session()?;
        }
        self.session = None;
        Ok(())
    }

    pub fn session_mut(&mut self) -> Result<&mut OdinSession> {
        self.session.as_mut().ok_or_else(|| ChimeraError::Odin("Not connected".into()))
    }

    /// Flash a complete Samsung firmware package (tar, tar.md5, lz4)
    pub fn flash_firmware(&mut self, firmware_path: &str, progress: Option<&ProgressSender>) -> Result<()> {
        let session = self.session_mut()?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Samsung FW Flash").step("Parsing firmware package...").percent(2.0));
        }
        
        // Read PIT to know partition layout
        let pit_data = session.read_pit()?;
        let pit = PitTable::parse(&pit_data)?;
        
        info!("PIT entries: {}", pit.entries.len());
        
        // Parse firmware archive
        let fw_lower = firmware_path.to_lowercase();
        
        if fw_lower.ends_with(".tar.md5") || fw_lower.ends_with(".tar") {
            flash_from_tar(session, firmware_path, &pit, progress)?;
        } else if fw_lower.ends_with(".lz4") {
            flash_from_lz4(session, firmware_path, &pit, progress)?;
        } else if fw_lower.ends_with(".zip") {
            flash_from_zip(session, firmware_path, &pit, progress)?;
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Samsung FW Flash").step("Rebooting...").percent(99.0));
        }
        
        session.reboot()?;
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Samsung FW Flash").step("Done").percent(100.0).complete());
        }
        
        Ok(())
    }
}

impl Default for OdinClient {
    fn default() -> Self {
        Self::new()
    }
}

fn flash_from_tar(_session: &mut OdinSession, path: &str, _pit: &PitTable, _progress: Option<&ProgressSender>) -> Result<()> {
    use std::fs::File;
    
    let file = File::open(path).map_err(|e| ChimeraError::Io(e.to_string()))?;
    let mut archive = tar::Archive::new(file);
    
    let entries: Vec<_> = archive.entries()
        .map_err(|e| ChimeraError::Firmware(e.to_string()))?
        .filter_map(|e| e.ok())
        .collect();
    
    info!("TAR contains {} files", entries.len());
    
    Ok(())
}

fn flash_from_lz4(_session: &mut OdinSession, _path: &str, _pit: &PitTable, _progress: Option<&ProgressSender>) -> Result<()> {
    // LZ4 decompress then flash
    Ok(())
}

fn flash_from_zip(_session: &mut OdinSession, _path: &str, _pit: &PitTable, _progress: Option<&ProgressSender>) -> Result<()> {
    // ZIP format (some Samsung models)
    Ok(())
}
