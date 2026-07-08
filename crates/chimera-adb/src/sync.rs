// chimera-adb/src/sync.rs
// ADB SYNC protocol for file push/pull

use byteorder::{LittleEndian, WriteBytesExt};

#[allow(dead_code)]
const SYNC_DATA_MAX: usize = 64 * 1024;

pub const ID_STAT: u32 = 0x54415453; // "STAT"
pub const ID_LIST: u32 = 0x5453494C; // "LIST"  
pub const ID_SEND: u32 = 0x444E4553; // "SEND"
pub const ID_RECV: u32 = 0x56434552; // "RECV"
pub const ID_DENT: u32 = 0x544E4544; // "DENT"
pub const ID_DONE: u32 = 0x454E4F44; // "DONE"
pub const ID_DATA: u32 = 0x41544144; // "DATA"
pub const ID_OKAY: u32 = 0x59414B4F; // "OKAY"
pub const ID_FAIL: u32 = 0x4C494146; // "FAIL"
pub const ID_QUIT: u32 = 0x54495551; // "QUIT"

/// ADB sync message
#[derive(Debug)]
pub struct SyncMessage {
    pub id: u32,
    pub namelen: u32,
}

impl SyncMessage {
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8);
        buf.write_u32::<LittleEndian>(self.id).unwrap();
        buf.write_u32::<LittleEndian>(self.namelen).unwrap();
        buf
    }
}

/// Directory entry from DENT
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub mode: u32,
    pub size: u32,
    pub time: u32,
    pub name: String,
}

/// File stat from STAT
#[derive(Debug, Clone)]
pub struct FileStat {
    pub mode: u32,
    pub size: u32,
    pub time: u32,
}

/// Build SEND request data
pub fn build_send_request(remote_path: &str, mode: u32) -> Vec<u8> {
    let dest = format!("{},{}", remote_path, mode);
    let mut buf = Vec::new();
    buf.write_u32::<LittleEndian>(ID_SEND).unwrap();
    buf.write_u32::<LittleEndian>(dest.len() as u32).unwrap();
    buf.extend_from_slice(dest.as_bytes());
    buf
}

/// Build DATA chunk
pub fn build_data_chunk(data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.write_u32::<LittleEndian>(ID_DATA).unwrap();
    buf.write_u32::<LittleEndian>(data.len() as u32).unwrap();
    buf.extend_from_slice(data);
    buf
}

/// Build DONE message
pub fn build_done(timestamp: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.write_u32::<LittleEndian>(ID_DONE).unwrap();
    buf.write_u32::<LittleEndian>(timestamp).unwrap();
    buf
}

/// Build RECV request
pub fn build_recv_request(remote_path: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.write_u32::<LittleEndian>(ID_RECV).unwrap();
    buf.write_u32::<LittleEndian>(remote_path.len() as u32).unwrap();
    buf.extend_from_slice(remote_path.as_bytes());
    buf
}
