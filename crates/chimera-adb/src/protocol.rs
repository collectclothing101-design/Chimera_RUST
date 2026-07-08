// chimera-adb/src/protocol.rs
// ADB wire protocol implementation
// Based on the official ADB protocol specification

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chimera_core::error::{ChimeraError, Result};
use std::io::Cursor;

// ADB protocol constants
pub const ADB_SYNC: u32 = 0x434E5953; // "SYNC"
pub const ADB_CNXN: u32 = 0x4E584E43; // "CNXN"
pub const ADB_OPEN: u32 = 0x4E45504F; // "OPEN"
pub const ADB_OKAY: u32 = 0x59414B4F; // "OKAY"
pub const ADB_CLSE: u32 = 0x45534C43; // "CLSE"
pub const ADB_WRTE: u32 = 0x45545257; // "WRTE"
pub const ADB_AUTH: u32 = 0x48545541; // "AUTH"
pub const ADB_STLS: u32 = 0x534C5453; // "STLS"

pub const ADB_VERSION: u32 = 0x01000001;
pub const ADB_VERSION_SKIP_CHECKSUM: u32 = 0x01000001;
pub const MAX_PAYLOAD: u32 = 1024 * 1024;

pub const ADB_AUTH_TOKEN: u32 = 1;
pub const ADB_AUTH_SIGNATURE: u32 = 2;
pub const ADB_AUTH_RSAPUBLICKEY: u32 = 3;

/// ADB wire message
#[derive(Debug, Clone)]
pub struct AdbMessage {
    pub command: u32,
    pub arg0: u32,
    pub arg1: u32,
    pub data: Vec<u8>,
}

impl AdbMessage {
    pub fn new(command: u32, arg0: u32, arg1: u32, data: Vec<u8>) -> Self {
        Self { command, arg0, arg1, data }
    }

    pub fn connect(system_type: &str, serial: &str, banner: &str) -> Self {
        let data = format!("{}:{}:{}", system_type, serial, banner);
        Self::new(ADB_CNXN, ADB_VERSION, MAX_PAYLOAD, data.into_bytes())
    }

    pub fn auth_token(token: Vec<u8>) -> Self {
        Self::new(ADB_AUTH, ADB_AUTH_TOKEN, 0, token)
    }

    pub fn auth_signature(sig: Vec<u8>) -> Self {
        Self::new(ADB_AUTH, ADB_AUTH_SIGNATURE, 0, sig)
    }

    pub fn auth_public_key(key: Vec<u8>) -> Self {
        Self::new(ADB_AUTH, ADB_AUTH_RSAPUBLICKEY, 0, key)
    }

    pub fn open(local_id: u32, service: &str) -> Self {
        let mut data = service.as_bytes().to_vec();
        data.push(0); // null terminator
        Self::new(ADB_OPEN, local_id, 0, data)
    }

    pub fn okay(local_id: u32, remote_id: u32) -> Self {
        Self::new(ADB_OKAY, local_id, remote_id, vec![])
    }

    pub fn write(local_id: u32, remote_id: u32, data: Vec<u8>) -> Self {
        Self::new(ADB_WRTE, local_id, remote_id, data)
    }

    pub fn close(local_id: u32, remote_id: u32) -> Self {
        Self::new(ADB_CLSE, local_id, remote_id, vec![])
    }

    /// Serialize to wire format (24 byte header + data)
    pub fn serialize(&self) -> Vec<u8> {
        let data_len = self.data.len() as u32;
        let checksum = self.data_checksum();
        let magic = self.command ^ 0xFFFFFFFF;

        let mut buf = Vec::with_capacity(24 + self.data.len());
        buf.write_u32::<LittleEndian>(self.command).unwrap();
        buf.write_u32::<LittleEndian>(self.arg0).unwrap();
        buf.write_u32::<LittleEndian>(self.arg1).unwrap();
        buf.write_u32::<LittleEndian>(data_len).unwrap();
        buf.write_u32::<LittleEndian>(checksum).unwrap();
        buf.write_u32::<LittleEndian>(magic).unwrap();
        buf.extend_from_slice(&self.data);
        buf
    }

    /// Deserialize from wire format
    pub fn deserialize(data: &[u8]) -> Result<(Self, usize)> {
        if data.len() < 24 {
            return Err(ChimeraError::Adb("Message too short".into()));
        }

        let mut cursor = Cursor::new(data);
        let command = cursor.read_u32::<LittleEndian>().unwrap();
        let arg0 = cursor.read_u32::<LittleEndian>().unwrap();
        let arg1 = cursor.read_u32::<LittleEndian>().unwrap();
        let data_len = cursor.read_u32::<LittleEndian>().unwrap() as usize;
        let _checksum = cursor.read_u32::<LittleEndian>().unwrap();
        let _magic = cursor.read_u32::<LittleEndian>().unwrap();

        if data.len() < 24 + data_len {
            return Err(ChimeraError::Adb("Incomplete message payload".into()));
        }

        let payload = data[24..24 + data_len].to_vec();
        let msg = Self::new(command, arg0, arg1, payload);
        Ok((msg, 24 + data_len))
    }

    fn data_checksum(&self) -> u32 {
        self.data.iter().map(|&b| b as u32).sum()
    }
}

/// High-level ADB commands
#[derive(Debug, Clone)]
pub enum AdbCommand {
    Shell(String),
    ShellRaw(String),
    GetProp(String),
    SetProp(String, String),
    Reboot(Option<String>),
    Push(String, String),   // local -> remote
    Pull(String, String),   // remote -> local
    Install(String),
    Uninstall(String),
    Remount,
    Root,
    Unroot,
    Connect(String),        // host:port
    Disconnect(String),
    WaitForDevice,
    GetState,
    GetSerial,
    ListDevices,
    Sync,
    Forward(String, String),
    Reverse(String, String),
    Logcat(Option<Vec<String>>),
    Screenshot(String),
    Service(String),        // custom service
}

/// ADB transport (TCP or USB)
pub enum AdbTransport {
    Tcp { host: String, port: u16 },
    Usb { serial: String },
}
