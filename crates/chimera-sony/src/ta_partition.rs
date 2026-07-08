// chimera-sony/src/ta_partition.rs
// Sony TA (Trim Area) partition operations
// The TA partition holds DRM keys, bootloader lock state, and calibration data.

use chimera_core::error::Result;

use log::{info, warn};

/// Sony TA unit IDs
pub mod ta_units {
    pub const BOOTLOADER_UNLOCK:   u32 = 0x001A;  // Bootloader unlock flag
    pub const DRM_ATTESTATION:     u32 = 0x0029;  // DRM attestation cert
    pub const PRODUCT_ID:          u32 = 0x0008;  // Product/model ID
    pub const IMEI_PRIMARY:        u32 = 0x0001;  // Primary IMEI
    pub const IMEI_SECONDARY:      u32 = 0x0002;  // Secondary IMEI
    pub const WIFI_MAC:            u32 = 0x0010;  // Wi-Fi MAC address
    pub const BT_MAC:              u32 = 0x0011;  // Bluetooth MAC address
    pub const LOCK_COUNTER:        u32 = 0x001B;  // Boot unlock counter
    pub const TRIM_AREA_FLAGS:     u32 = 0x0016;  // General TA flags
    pub const SOC_ID:              u32 = 0x0032;  // SoC hardware ID
}

/// A single entry in the TA partition
#[derive(Debug, Clone)]
pub struct TaEntry {
    pub unit_id: u32,
    pub data: Vec<u8>,
}

impl TaEntry {
    pub fn new(unit_id: u32, data: Vec<u8>) -> Self {
        Self { unit_id, data }
    }

    /// Get data as ASCII string (if applicable)
    pub fn as_string(&self) -> Option<String> {
        String::from_utf8(self.data.clone()).ok()
            .map(|s| s.trim_end_matches('\0').to_string())
    }

    /// Get data as hex string
    pub fn as_hex(&self) -> String {
        hex::encode(&self.data)
    }
}

/// Parsed Sony TA partition
pub struct TaPartition {
    pub entries: Vec<TaEntry>,
    pub raw: Vec<u8>,
}

impl TaPartition {
    /// Parse TA partition from raw bytes
    /// Sony TA format: multiple fixed-size records, each 512 bytes
    /// Layout per record: [unit_id: 4 bytes LE][data_len: 4 bytes LE][data: variable][padding]
    pub fn parse(raw: &[u8]) -> Result<Self> {
        let mut entries = Vec::new();
        let mut offset = 0usize;
        let record_size = 512;

        while offset + record_size <= raw.len() {
            let chunk = &raw[offset..offset + record_size];
            
            let unit_id = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let data_len = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]) as usize;
            
            if unit_id == 0 || unit_id == 0xFFFFFFFF {
                offset += record_size;
                continue;
            }
            
            let actual_len = data_len.min(record_size - 8);
            let data = chunk[8..8 + actual_len].to_vec();
            
            entries.push(TaEntry::new(unit_id, data));
            offset += record_size;
        }

        Ok(TaPartition {
            entries,
            raw: raw.to_vec(),
        })
    }

    /// Find a TA entry by unit ID
    pub fn find(&self, unit_id: u32) -> Option<&TaEntry> {
        self.entries.iter().find(|e| e.unit_id == unit_id)
    }

    /// Update or insert a TA entry (returns new raw bytes)
    pub fn set_entry(&mut self, unit_id: u32, data: Vec<u8>) -> Result<Vec<u8>> {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.unit_id == unit_id) {
            entry.data = data;
        } else {
            self.entries.push(TaEntry::new(unit_id, data));
        }
        self.serialize()
    }

    /// Serialize back to raw bytes (512-byte records)
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let record_size = 512usize;
        let mut out = self.raw.clone();
        
        for entry in &self.entries {
            // Find existing record offset by scanning raw for matching unit_id
            let unit_bytes = entry.unit_id.to_le_bytes();
            let mut found_offset = None;
            
            let mut off = 0usize;
            while off + record_size <= out.len() {
                if out[off..off+4] == unit_bytes {
                    found_offset = Some(off);
                    break;
                }
                off += record_size;
            }
            
            let data_len = entry.data.len().min(record_size - 8);
            let write_at = found_offset.unwrap_or_else(|| {
                // Append new record
                out.extend(vec![0u8; record_size]);
                out.len() - record_size
            });
            
            out[write_at..write_at+4].copy_from_slice(&unit_bytes);
            out[write_at+4..write_at+8].copy_from_slice(&(data_len as u32).to_le_bytes());
            out[write_at+8..write_at+8+data_len].copy_from_slice(&entry.data[..data_len]);
        }
        
        Ok(out)
    }

    /// Check if bootloader is unlocked
    pub fn is_bootloader_unlocked(&self) -> bool {
        self.find(ta_units::BOOTLOADER_UNLOCK)
            .map(|e| e.data.first().copied().unwrap_or(0) != 0)
            .unwrap_or(false)
    }

    /// Get unlock counter value
    pub fn get_unlock_counter(&self) -> u32 {
        self.find(ta_units::LOCK_COUNTER)
            .and_then(|e| {
                if e.data.len() >= 4 {
                    Some(u32::from_le_bytes([e.data[0], e.data[1], e.data[2], e.data[3]]))
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }
}

/// Read TA partition via ADB (requires root)
pub fn read_ta_via_adb(shell: &chimera_adb::shell::AdbShell) -> Result<TaPartition> {
    // Find TA block device
    let ta_path = shell.run("ls /dev/block/by-name/TA 2>/dev/null || ls /dev/block/bootdevice/by-name/TA 2>/dev/null || echo /dev/block/TA").ok()
        .unwrap_or_else(|| "/dev/block/by-name/TA".to_string());
    let ta_path = ta_path.trim().to_string();

    info!("Reading TA partition from: {}", ta_path);
    
    // Read TA partition to temp file
    let tmp = "/data/local/tmp/ta_backup.bin";
    shell.run_root(&format!("dd if={} of={} bs=4096", ta_path, tmp))?;
    
    // Pull the file
    let data = shell.read_file(tmp)?;
    TaPartition::parse(&data)
}

/// Write TA partition via ADB (requires root; DANGEROUS)
pub fn write_ta_via_adb(shell: &chimera_adb::shell::AdbShell, ta: &TaPartition) -> Result<()> {
    let ta_path = "/dev/block/by-name/TA";
    let tmp = "/data/local/tmp/ta_write.bin";
    
    let raw = ta.serialize()?;
    warn!("Writing TA partition — this is irreversible!");
    
    shell.write_file(tmp, &raw)?;
    shell.run_root(&format!("dd if={} of={} bs=4096", tmp, ta_path))?;
    shell.run_root(&format!("rm -f {}", tmp))?;
    
    info!("TA partition written successfully");
    Ok(())
}
