// chimera-edl/src/firehose.rs
// Qualcomm Firehose protocol (XML-based commands over USB after Sahara)


/// Build a firehose XML command
pub fn build_command(tag: &str, attrs: &[(&str, &str)]) -> String {
    let attrs_str = attrs.iter()
        .map(|(k, v)| format!(" {}=\"{}\"", k, v))
        .collect::<String>();
    format!("<?xml version=\"1.0\" ?><data><{}{}/></data>", tag, attrs_str)
}

/// Parse firehose XML response
pub fn parse_response(xml: &str) -> FirehoseResponse {
    let xml_lower = xml.to_lowercase();
    
    if xml_lower.contains("value=\"ack\"") || xml_lower.contains("rawmode=\"false\"") {
        FirehoseResponse::Ack
    } else if xml_lower.contains("value=\"nak\"") {
        let msg = extract_attr(xml, "rawmode").unwrap_or_else(|| "NAK".to_string());
        FirehoseResponse::Nak(msg)
    } else if xml_lower.contains("<log ") || xml_lower.contains("<log>") {
        let msg = extract_attr(xml, "value").unwrap_or_else(|| xml.to_string());
        FirehoseResponse::Log(msg)
    } else {
        FirehoseResponse::Unknown(xml.to_string())
    }
}

fn extract_attr(xml: &str, attr: &str) -> Option<String> {
    let search = format!("{}=\"", attr);
    if let Some(start) = xml.find(&search) {
        let start = start + search.len();
        if let Some(end) = xml[start..].find('"') {
            return Some(xml[start..start + end].to_string());
        }
    }
    None
}

#[derive(Debug, Clone)]
pub enum FirehoseResponse {
    Ack,
    Nak(String),
    Log(String),
    Unknown(String),
}

impl FirehoseResponse {
    pub fn is_ack(&self) -> bool {
        matches!(self, FirehoseResponse::Ack)
    }
}

/// Firehose protocol handler
pub struct FirehoseProtocol;

impl FirehoseProtocol {
    /// Configure firehose (initial setup)
    pub fn configure(max_payload: u32, verbose: bool) -> String {
        build_command("configure", &[
            ("MemoryName", "emmc"),
            ("ZlpAwareHost", "1"),
            ("SkipStorageInit", "0"),
            ("MaxPayloadSizeToTargetInBytes", &max_payload.to_string()),
            ("verbose", if verbose { "1" } else { "0" }),
        ])
    }

    /// Erase a partition
    pub fn erase(start_sector: u64, num_sectors: u64, lun: u8) -> String {
        build_command("erase", &[
            ("SECTOR_SIZE_IN_BYTES", "512"),
            ("num_partition_sectors", &num_sectors.to_string()),
            ("physical_partition_number", &lun.to_string()),
            ("start_sector", &start_sector.to_string()),
        ])
    }

    /// Read data from storage
    pub fn read(start_sector: u64, num_sectors: u64, lun: u8) -> String {
        build_command("read", &[
            ("SECTOR_SIZE_IN_BYTES", "512"),
            ("num_partition_sectors", &num_sectors.to_string()),
            ("physical_partition_number", &lun.to_string()),
            ("start_sector", &start_sector.to_string()),
        ])
    }

    /// Program (write) data to storage
    pub fn program(start_sector: u64, num_sectors: u64, lun: u8, file_sector_offset: u64) -> String {
        build_command("program", &[
            ("SECTOR_SIZE_IN_BYTES", "512"),
            ("file_sector_offset", &file_sector_offset.to_string()),
            ("num_partition_sectors", &num_sectors.to_string()),
            ("physical_partition_number", &lun.to_string()),
            ("start_sector", &start_sector.to_string()),
        ])
    }

    /// Set bootable partition
    pub fn set_bootable(lun: u8) -> String {
        build_command("setbootablestoragedrive", &[
            ("value", &lun.to_string()),
        ])
    }

    /// Power reset the device
    pub fn power(action: &str) -> String {
        build_command("power", &[
            ("value", action),
            ("DelayInSeconds", "2"),
        ])
    }

    /// Get storage info
    pub fn get_storage_info() -> String {
        build_command("getstorageinfo", &[
            ("physical_partition_number", "0"),
        ])
    }

    /// Read partition table (GPT)
    pub fn read_gpt(lun: u8) -> String {
        build_command("getpartitiontable", &[
            ("physical_partition_number", &lun.to_string()),
            ("xml_path", "/gpt.xml"),
        ])
    }

    /// Peek at memory (for EFS reading)
    pub fn peek(address: u64, size: u64) -> String {
        build_command("peek", &[
            ("address64", &format!("{:#010x}", address)),
            ("SizeInBytes", &size.to_string()),
        ])
    }

    /// Poke memory (for EFS writing)
    pub fn poke(address: u64, value: u64) -> String {
        build_command("poke", &[
            ("address64", &format!("{:#010x}", address)),
            ("SizeInBytes", "8"),
            ("Value", &value.to_string()),
        ])
    }
}
