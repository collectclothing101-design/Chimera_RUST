// Fastboot client implementation using rusb for direct USB communication

use rusb::{DeviceHandle, GlobalContext};
use chimera_core::error::{ChimeraError, Result};
use chimera_core::device::DeviceInfo;
use chimera_core::progress::{ProgressSender, Progress};
use crate::protocol::{FastbootCommand, FastbootResponse, FASTBOOT_MAX_DOWNLOAD_SIZE, FLASH_TIMEOUT_MS, USB_TIMEOUT_MS};
use log::{debug, info};
use std::time::Duration;

const FASTBOOT_USB_CLASS: u8 = 0xFF;
const FASTBOOT_USB_SUBCLASS: u8 = 0x42;
const FASTBOOT_USB_PROTOCOL: u8 = 0x03;
/// USB Full-Speed packet boundary for Fastboot bulk-out transfers.
#[allow(dead_code)]
pub const MAX_PACKET: usize = 64;

/// Fastboot client - communicates directly via USB
pub struct FastbootClient {
    handle: DeviceHandle<GlobalContext>,
    ep_in: u8,
    ep_out: u8,
    serial: String,
    timeout: Duration,
}

impl FastbootClient {
    /// Find and open the first available fastboot device
    pub fn open_first() -> Result<Self> {
        let _context = GlobalContext::default();
        
        for device in rusb::devices()
            .map_err(|e| ChimeraError::Usb(e.to_string()))?
            .iter()
        {
            let config = match device.active_config_descriptor() {
                Ok(c) => c,
                Err(_) => continue,
            };
            
            for interface in config.interfaces() {
                for iface_desc in interface.descriptors() {
                    if iface_desc.class_code() == FASTBOOT_USB_CLASS
                        && iface_desc.sub_class_code() == FASTBOOT_USB_SUBCLASS
                        && iface_desc.protocol_code() == FASTBOOT_USB_PROTOCOL
                    {
                        let mut ep_in = 0u8;
                        let mut ep_out = 0u8;
                        
                        for ep in iface_desc.endpoint_descriptors() {
                            match ep.direction() {
                                rusb::Direction::In => ep_in = ep.address(),
                                rusb::Direction::Out => ep_out = ep.address(),
                            }
                        }
                        
                        if let Ok(handle) = device.open() {
                            let _ = handle.claim_interface(interface.number());
                            let serial = handle.read_serial_number_string_ascii(&device.device_descriptor().unwrap())
                                .unwrap_or_else(|_| "unknown".to_string());
                            
                            return Ok(Self {
                                handle,
                                ep_in,
                                ep_out,
                                serial,
                                timeout: Duration::from_millis(USB_TIMEOUT_MS),
                            });
                        }
                    }
                }
            }
        }
        
        Err(ChimeraError::DeviceNotFound("No fastboot device found".into()))
    }

    /// Open device by serial number
    pub fn open_by_serial(serial: &str) -> Result<Self> {
        // Try to use `fastboot -s serial` approach via subprocess
        // For direct USB, enumerate all devices
        Err(ChimeraError::DeviceNotFound(format!("Fastboot device {} not found", serial)))
    }

    /// Send a command and receive response
    pub fn command(&mut self, cmd: &FastbootCommand) -> Result<Vec<FastbootResponse>> {
        let wire_cmd = cmd.to_wire();
        debug!("Fastboot CMD: {}", wire_cmd);
        
        // Write command
        self.write(wire_cmd.as_bytes())?;
        
        // Read all responses until OKAY or FAIL
        let mut responses = Vec::new();
        loop {
            let response = self.read_response()?;
            let done = matches!(response, FastbootResponse::Okay(_) | FastbootResponse::Fail(_));
            responses.push(response);
            if done {
                break;
            }
        }
        
        Ok(responses)
    }

    /// Get a variable
    pub fn get_var(&mut self, var: &str) -> Result<String> {
        let responses = self.command(&FastbootCommand::GetVar(var.to_string()))?;
        for resp in &responses {
            if let FastbootResponse::Okay(val) = resp {
                return Ok(val.clone());
            }
        }
        Err(ChimeraError::Fastboot(format!("getvar:{} failed", var)))
    }

    /// Flash a partition with data
    pub fn flash_partition(&mut self, partition: &str, data: &[u8], progress: Option<&ProgressSender>) -> Result<()> {
        info!("Flashing partition: {} ({} bytes)", partition, data.len());
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Flash").step(format!("Uploading {} ({} bytes)...", partition, data.len())).percent(0.0));
        }
        
        // Send download command
        let download_cmd = format!("download:{:08x}", data.len());
        self.write(download_cmd.as_bytes())?;
        let resp = self.read_response()?;
        
        if let FastbootResponse::Data(_) = resp {
            // Send data in chunks
            let chunk_size = 65536;
            let mut sent = 0usize;
            
            for chunk in data.chunks(chunk_size) {
                self.write(chunk)?;
                sent += chunk.len();
                
                if let Some(tx) = progress {
                    let pct = sent as f32 / data.len() as f32 * 80.0;
                    let _ = tx.send(Progress::new("Flash").step(format!("Uploading {}...", partition)).bytes(sent as u64, data.len() as u64).percent(pct));
                }
            }
            
            // Read final OKAY after download
            let final_resp = self.read_response()?;
            if !final_resp.is_okay() {
                return Err(ChimeraError::Fastboot(format!("Download failed: {}", final_resp.message())));
            }
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Flash").step(format!("Flashing {}...", partition)).percent(85.0));
        }
        
        // Send flash command
        let flash_cmd = format!("flash:{}", partition);
        self.write(flash_cmd.as_bytes())?;
        
        // Read flash responses (may have multiple INFO lines)
        loop {
            let resp = self.read_response_with_timeout(Duration::from_millis(FLASH_TIMEOUT_MS))?;
            match &resp {
                FastbootResponse::Okay(_) => break,
                FastbootResponse::Fail(msg) => {
                    return Err(ChimeraError::FastbootFailed {
                        cmd: flash_cmd,
                        response: msg.clone(),
                    });
                }
                FastbootResponse::Info(msg) => {
                    if let Some(tx) = progress {
                        let _ = tx.send(Progress::new("Flash").step(msg.clone()).percent(90.0));
                    }
                }
                FastbootResponse::Data(_) => {}
            }
        }
        
        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Flash").step(format!("{} flashed successfully", partition)).percent(100.0).complete());
        }
        
        Ok(())
    }

    /// Erase a partition
    pub fn erase_partition(&mut self, partition: &str) -> Result<()> {
        let cmd = format!("erase:{}", partition);
        self.write(cmd.as_bytes())?;
        let resp = self.read_response()?;
        if !resp.is_okay() {
            return Err(ChimeraError::Fastboot(format!("Erase {} failed: {}", partition, resp.message())));
        }
        Ok(())
    }

    /// Reboot device
    pub fn reboot(&mut self, mode: Option<&str>) -> Result<()> {
        let cmd = match mode {
            Some("bootloader") => "reboot-bootloader",
            Some("recovery") => "reboot-recovery",
            Some("fastboot") => "reboot-fastboot",
            _ => "reboot",
        };
        self.write(cmd.as_bytes())?;
        Ok(())
    }

    pub fn boot_image(&mut self, data: &[u8], progress: Option<&ProgressSender>) -> Result<()> {
        info!("Booting temporary image ({} bytes)", data.len());

        if data.len() as u64 > FASTBOOT_MAX_DOWNLOAD_SIZE {
            return Err(ChimeraError::Fastboot(format!(
                "Image is too large for fastboot download: {} bytes",
                data.len()
            )));
        }

        if let Some(tx) = progress {
            let _ = tx.send(
                Progress::new("Boot TWRP")
                    .step(format!("Uploading temporary recovery image ({} bytes)...", data.len()))
                    .percent(0.0),
            );
        }

        let download_cmd = format!("download:{:08x}", data.len());
        self.write(download_cmd.as_bytes())?;
        let resp = self.read_response()?;

        if !matches!(resp, FastbootResponse::Data(_)) {
            return Err(ChimeraError::Fastboot(format!(
                "Boot image download was rejected: {}",
                resp.message()
            )));
        }

        let chunk_size = 65536;
        let mut sent = 0usize;
        for chunk in data.chunks(chunk_size) {
            self.write(chunk)?;
            sent += chunk.len();
            if let Some(tx) = progress {
                let pct = sent as f32 / data.len() as f32 * 80.0;
                let _ = tx.send(
                    Progress::new("Boot TWRP")
                        .step("Uploading temporary recovery image...")
                        .bytes(sent as u64, data.len() as u64)
                        .percent(pct),
                );
            }
        }

        let final_resp = self.read_response()?;
        if !final_resp.is_okay() {
            return Err(ChimeraError::Fastboot(format!(
                "Boot image upload failed: {}",
                final_resp.message()
            )));
        }

        if let Some(tx) = progress {
            let _ = tx.send(Progress::new("Boot TWRP").step("Requesting fastboot boot...").percent(90.0));
        }

        self.write(FastbootCommand::Boot.to_wire().as_bytes())?;
        loop {
            let resp = self.read_response_with_timeout(Duration::from_millis(FLASH_TIMEOUT_MS))?;
            match &resp {
                FastbootResponse::Okay(_) => break,
                FastbootResponse::Fail(msg) => {
                    return Err(ChimeraError::FastbootFailed {
                        cmd: "boot".into(),
                        response: msg.clone(),
                    });
                }
                FastbootResponse::Info(msg) => {
                    if let Some(tx) = progress {
                        let _ = tx.send(Progress::new("Boot TWRP").step(msg.clone()).percent(95.0));
                    }
                }
                FastbootResponse::Data(_) => {}
            }
        }

        if let Some(tx) = progress {
            let _ = tx.send(
                Progress::new("Boot TWRP")
                    .step("Temporary recovery image booted successfully")
                    .percent(100.0)
                    .complete(),
            );
        }

        Ok(())
    }

    /// Unlock bootloader
    pub fn unlock_bootloader(&mut self) -> Result<()> {
        // First try flashing:unlock (newer)
        self.write(b"flashing unlock")?;
        let resp = self.read_response()?;
        if resp.is_okay() {
            return Ok(());
        }
        
        // Try OEM unlock (older)
        self.write(b"oem unlock")?;
        let resp = self.read_response()?;
        if resp.is_okay() {
            return Ok(());
        }
        
        Err(ChimeraError::Fastboot("Bootloader unlock failed".into()))
    }

    /// Lock bootloader
    pub fn lock_bootloader(&mut self) -> Result<()> {
        self.write(b"flashing lock")?;
        let resp = self.read_response()?;
        if !resp.is_okay() {
            return Err(ChimeraError::Fastboot("Bootloader lock failed".into()));
        }
        Ok(())
    }

    /// Get all device variables
    pub fn get_all_vars(&mut self) -> Result<std::collections::HashMap<String, String>> {
        self.write(b"getvar:all")?;
        
        let mut vars = std::collections::HashMap::new();
        loop {
            let resp = self.read_response()?;
            match &resp {
                FastbootResponse::Info(msg) => {
                    if let Some((k, v)) = msg.split_once(':') {
                        vars.insert(k.trim().to_string(), v.trim().to_string());
                    }
                }
                FastbootResponse::Okay(_) => break,
                FastbootResponse::Fail(_) => break,
                _ => {}
            }
        }
        
        Ok(vars)
    }

    /// Get device info from fastboot variables
    pub fn get_device_info(&mut self) -> Result<DeviceInfo> {
        let vars = self.get_all_vars()?;
        let mut info = DeviceInfo::new_unknown(self.serial.clone());
        
        if let Some(model) = vars.get("ro.product.model").or_else(|| vars.get("product")) {
            info.model = model.clone();
        }
        if let Some(serial) = vars.get("serialno") {
            info.serial = Some(serial.clone());
        }
        
        info.connection_mode = chimera_core::device::ConnectionMode::Fastboot;
        info.state = chimera_core::device::DeviceState::Bootloader;
        
        // Check bootloader lock status
        if let Some(lock) = vars.get("unlocked") {
            info.bootloader_status = Some(match lock.as_str() {
                "yes" | "true" => chimera_core::device::BootloaderStatus::Unlocked,
                _ => chimera_core::device::BootloaderStatus::Locked,
            });
        }
        
        Ok(info)
    }

    /// Write bytes to USB OUT endpoint
    fn write(&mut self, data: &[u8]) -> Result<()> {
        let written = self.handle
            .write_bulk(self.ep_out, data, self.timeout)
            .map_err(|e| ChimeraError::Usb(e.to_string()))?;
        
        if written != data.len() {
            return Err(ChimeraError::Fastboot(format!(
                "Short write: {} vs {}",
                written,
                data.len()
            )));
        }
        
        Ok(())
    }

    /// Read a single response from USB IN endpoint
    fn read_response(&mut self) -> Result<FastbootResponse> {
        self.read_response_with_timeout(self.timeout)
    }

    fn read_response_with_timeout(&mut self, timeout: Duration) -> Result<FastbootResponse> {
        let mut buf = vec![0u8; 65536];
        let read = self.handle
            .read_bulk(self.ep_in, &mut buf, timeout)
            .map_err(|e| ChimeraError::Usb(e.to_string()))?;
        
        FastbootResponse::parse(&buf[..read])
    }
}