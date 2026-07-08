// chimera-edl/src/sahara.rs
// Qualcomm Sahara protocol - initial handshake and loader upload

use chimera_core::error::{ChimeraError, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;
use log::debug;

// Sahara packet IDs
pub const SAHARA_HELLO_ID: u32 = 0x01;
pub const SAHARA_HELLO_RESP_ID: u32 = 0x02;
pub const SAHARA_READ_DATA_ID: u32 = 0x03;
pub const SAHARA_END_XFER_ID: u32 = 0x04;
pub const SAHARA_DONE_ID: u32 = 0x05;
pub const SAHARA_DONE_RESP_ID: u32 = 0x06;
pub const SAHARA_RESET_ID: u32 = 0x07;
pub const SAHARA_RESET_RESP_ID: u32 = 0x08;
pub const SAHARA_MEMORY_DEBUG_ID: u32 = 0x09;
pub const SAHARA_MEMORY_READ_ID: u32 = 0x0A;
pub const SAHARA_CMD_READY_ID: u32 = 0x0B;
pub const SAHARA_SWITCH_MODE_ID: u32 = 0x0C;
pub const SAHARA_EXECUTE_ID: u32 = 0x0D;
pub const SAHARA_EXECUTE_RESP_ID: u32 = 0x0E;
pub const SAHARA_EXECUTE_DATA_ID: u32 = 0x0F;
pub const SAHARA_MEMORY_DEBUG64_ID: u32 = 0x10;
pub const SAHARA_MEMORY_READ64_ID: u32 = 0x11;
pub const SAHARA_READ_DATA64_ID: u32 = 0x12;
pub const SAHARA_RESET_STATE_ID: u32 = 0x13;
pub const SAHARA_WRITE_DATA_ID: u32 = 0x14;
pub const SAHARA_WRITE_DATA64_ID: u32 = 0x15;

// Sahara modes
pub const SAHARA_MODE_IMAGE_TX_PENDING: u32 = 0x00;
pub const SAHARA_MODE_IMAGE_TX_COMPLETE: u32 = 0x01;
pub const SAHARA_MODE_MEMORY_DEBUG: u32 = 0x02;
pub const SAHARA_MODE_CMD: u32 = 0x03;

// Sahara version
pub const SAHARA_VERSION: u32 = 2;
pub const SAHARA_VERSION_SUPPORTED: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub enum SaharaState {
    Waiting,
    HelloReceived,
    FileRequested { id: u32, offset: u32, length: u32 },
    Complete,
    Error(u32),
    CmdReady,
}

/// Sahara packet
#[derive(Debug, Clone)]
pub struct SaharaPacket {
    pub id: u32,
    pub length: u32,
    pub data: Vec<u8>,
}

impl SaharaPacket {
    pub fn parse(buf: &[u8]) -> Result<Self> {
        if buf.len() < 8 {
            return Err(ChimeraError::Sahara("Packet too short".into()));
        }
        let mut cursor = Cursor::new(buf);
        let id = cursor.read_u32::<LittleEndian>().unwrap();
        let length = cursor.read_u32::<LittleEndian>().unwrap();
        let data = buf[8..].to_vec();
        Ok(Self { id, length, data })
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_u32::<LittleEndian>(self.id).unwrap();
        buf.write_u32::<LittleEndian>(self.length).unwrap();
        buf.extend_from_slice(&self.data);
        buf
    }
}

/// Build Hello Response packet
pub fn build_hello_response(mode: u32) -> Vec<u8> {
    let mut data = Vec::new();
    data.write_u32::<LittleEndian>(SAHARA_VERSION).unwrap();
    data.write_u32::<LittleEndian>(SAHARA_VERSION_SUPPORTED).unwrap();
    data.write_u32::<LittleEndian>(0x00400000).unwrap(); // max packet size
    data.write_u32::<LittleEndian>(mode).unwrap();
    data.extend_from_slice(&[0u8; 16]); // reserved

    let pkt = SaharaPacket {
        id: SAHARA_HELLO_RESP_ID,
        length: (8 + data.len()) as u32,
        data,
    };
    pkt.serialize()
}

/// Build Done packet
pub fn build_done() -> Vec<u8> {
    let pkt = SaharaPacket {
        id: SAHARA_DONE_ID,
        length: 8,
        data: vec![],
    };
    pkt.serialize()
}

/// Build Reset packet
pub fn build_reset() -> Vec<u8> {
    let pkt = SaharaPacket {
        id: SAHARA_RESET_ID,
        length: 8,
        data: vec![],
    };
    pkt.serialize()
}

/// Sahara protocol handler
pub struct SaharaProtocol {
    pub state: SaharaState,
    pub mode: u32,
}

impl SaharaProtocol {
    pub fn new() -> Self {
        Self {
            state: SaharaState::Waiting,
            mode: SAHARA_MODE_CMD,
        }
    }

    /// Process incoming packet, return response to send
    pub fn process_packet(&mut self, packet: &SaharaPacket) -> Result<Option<Vec<u8>>> {
        match packet.id {
            SAHARA_HELLO_ID => {
                debug!("Sahara HELLO received");
                self.state = SaharaState::HelloReceived;
                // Parse hello
                let mut cursor = Cursor::new(&packet.data);
                let _version = cursor.read_u32::<LittleEndian>().ok();
                let _min_version = cursor.read_u32::<LittleEndian>().ok();
                let _max_pkt_size = cursor.read_u32::<LittleEndian>().ok();
                let _mode = cursor.read_u32::<LittleEndian>().ok();
                
                // Respond with CMD mode
                Ok(Some(build_hello_response(self.mode)))
            }

            SAHARA_READ_DATA_ID => {
                let mut cursor = Cursor::new(&packet.data);
                let image_id = cursor.read_u32::<LittleEndian>().unwrap_or(0);
                let offset = cursor.read_u32::<LittleEndian>().unwrap_or(0);
                let length = cursor.read_u32::<LittleEndian>().unwrap_or(0);
                debug!("Sahara READ_DATA: image={} offset={} len={}", image_id, offset, length);
                self.state = SaharaState::FileRequested { id: image_id, offset, length };
                Ok(None) // Caller should send the data
            }

            SAHARA_END_XFER_ID => {
                let mut cursor = Cursor::new(&packet.data);
                let _image_id = cursor.read_u32::<LittleEndian>().ok();
                let status = cursor.read_u32::<LittleEndian>().unwrap_or(1);
                if status == 0 {
                    self.state = SaharaState::Complete;
                    Ok(Some(build_done()))
                } else {
                    self.state = SaharaState::Error(status);
                    Err(ChimeraError::Sahara(format!("Transfer error: status={}", status)))
                }
            }

            SAHARA_CMD_READY_ID => {
                debug!("Sahara CMD_READY - device ready for firehose");
                self.state = SaharaState::CmdReady;
                Ok(None)
            }

            SAHARA_DONE_RESP_ID => {
                self.state = SaharaState::Complete;
                Ok(None)
            }

            _ => {
                debug!("Unknown Sahara packet: id=0x{:04X}", packet.id);
                Ok(None)
            }
        }
    }
}

impl Default for SaharaProtocol {
    fn default() -> Self {
        Self::new()
    }
}
