// chimera-edl/src/client.rs
// Main EDL client combining Sahara + Firehose

use chimera_core::error::{ChimeraError, Result};
use chimera_core::progress::{Progress, ProgressSender};
use crate::sahara::{SaharaProtocol, SaharaState, SaharaPacket};
use crate::firehose::FirehoseProtocol;
use crate::usb::EdlUsb;
use log::{info, warn};
use std::time::Duration;

/// Main EDL client
pub struct EdlClient {
    usb: EdlUsb,
    sahara: SaharaProtocol,
    is_firehose_ready: bool,
}

impl EdlClient {
    /// Connect to EDL device and complete Sahara handshake
    pub fn connect(prog_loader: Option<&[u8]>) -> Result<Self> {
        let usb = EdlUsb::open()?;
        let mut client = Self {
            usb,
            sahara: SaharaProtocol::new(),
            is_firehose_ready: false,
        };
        
        client.run_sahara(prog_loader)?;
        Ok(client)
    }

    /// Run Sahara protocol to initialize device
    fn run_sahara(&mut self, prog_loader: Option<&[u8]>) -> Result<()> {
        info!("Starting Sahara protocol...");
        
        // Give device time to enumerate
        std::thread::sleep(Duration::from_millis(500));
        
        let mut retries = 0;
        loop {
            match self.usb.read(65536) {
                Ok(data) if !data.is_empty() => {
                    let packet = SaharaPacket::parse(&data)?;
                    
                    match self.sahara.process_packet(&packet)? {
                        Some(response) => {
                            self.usb.write(&response)?;
                        }
                        None => {}
                    }
                    
                    match &self.sahara.state {
                        SaharaState::CmdReady => {
                            info!("Sahara complete - device in CMD mode (firehose ready)");
                            self.is_firehose_ready = true;
                            return Ok(());
                        }
                        SaharaState::FileRequested { id, offset, length } => {
                            let _id = *id;
                            let offset = *offset as usize;
                            let length = *length as usize;
                            
                            if let Some(loader) = prog_loader {
                                let end = (offset + length).min(loader.len());
                                let chunk = &loader[offset..end];
                                self.usb.write(chunk)?;
                            } else {
                                return Err(ChimeraError::Sahara(
                                    "Device needs programmer loader but none provided".into()
                                ));
                            }
                        }
                        SaharaState::Complete => {
                            info!("Sahara transfer complete");
                            self.is_firehose_ready = true;
                            return Ok(());
                        }
                        SaharaState::Error(code) => {
                            return Err(ChimeraError::Sahara(format!("Sahara error code: {}", code)));
                        }
                        _ => {}
                    }
                }
                _ => {
                    retries += 1;
                    if retries > 50 {
                        return Err(ChimeraError::ConnectionTimeout { timeout_ms: 5000 });
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }

    /// Send firehose command and get response
    pub fn firehose_cmd(&mut self, cmd: &str) -> Result<bool> {
        if !self.is_firehose_ready {
            return Err(ChimeraError::Edl("Firehose not ready".into()));
        }
        
        let responses = self.usb.exchange_xml(cmd)?;
        for resp_xml in &responses {
            let resp = crate::firehose::parse_response(resp_xml);
            match resp {
                crate::firehose::FirehoseResponse::Ack => return Ok(true),
                crate::firehose::FirehoseResponse::Nak(msg) => {
                    warn!("Firehose NAK: {}", msg);
                    return Ok(false);
                }
                _ => {}
            }
        }
        Ok(false)
    }

    /// Configure firehose
    pub fn configure(&mut self) -> Result<()> {
        let cmd = FirehoseProtocol::configure(1048576, false);
        self.firehose_cmd(&cmd)?;
        Ok(())
    }

    /// Read a range of sectors
    pub fn read_sectors(&mut self, start: u64, count: u64, lun: u8) -> Result<Vec<u8>> {
        let cmd = FirehoseProtocol::read(start, count, lun);
        let _responses = self.usb.exchange_xml(&cmd)?;
        
        // After ACK, read raw sector data
        let sector_size = 512usize;
        let total_bytes = count as usize * sector_size;
        let mut data = Vec::with_capacity(total_bytes);
        
        while data.len() < total_bytes {
            let remaining = total_bytes - data.len();
            let chunk = self.usb.read(remaining.min(65536))?;
            if chunk.is_empty() {
                break;
            }
            data.extend_from_slice(&chunk);
        }
        
        Ok(data)
    }

    /// Write sectors
    pub fn write_sectors(&mut self, start: u64, lun: u8, data: &[u8], progress: Option<&ProgressSender>) -> Result<()> {
        let sector_size = 512u64;
        let num_sectors = (data.len() as u64 + sector_size - 1) / sector_size;
        
        let cmd = FirehoseProtocol::program(start, num_sectors, lun, 0);
        let _responses = self.usb.exchange_xml(&cmd)?;
        
        // Send raw data
        let chunk_size = 65536usize;
        let mut sent = 0usize;
        
        for chunk in data.chunks(chunk_size) {
            self.usb.write_flash(chunk)?;
            sent += chunk.len();
            
            if let Some(tx) = progress {
                let pct = sent as f32 / data.len() as f32 * 100.0;
                let _ = tx.send(Progress::new("EDL Write").step("Writing sectors...").bytes(sent as u64, data.len() as u64).percent(pct));
            }
        }
        
        // Read final ACK
        let _final_responses = self.usb.exchange_xml("")?;
        
        Ok(())
    }

    /// Erase partition/sectors
    pub fn erase_sectors(&mut self, start: u64, count: u64, lun: u8) -> Result<()> {
        let cmd = FirehoseProtocol::erase(start, count, lun);
        let ok = self.firehose_cmd(&cmd)?;
        if !ok {
            return Err(ChimeraError::Edl(format!("Erase failed at sector {}", start)));
        }
        Ok(())
    }

    /// Reset/reboot device
    pub fn reboot(&mut self, action: &str) -> Result<()> {
        let cmd = FirehoseProtocol::power(action);
        self.firehose_cmd(&cmd)?;
        Ok(())
    }

    /// Get storage information
    pub fn get_storage_info(&mut self) -> Result<String> {
        let cmd = FirehoseProtocol::get_storage_info();
        let responses = self.usb.exchange_xml(&cmd)?;
        Ok(responses.join("\n"))
    }
}
