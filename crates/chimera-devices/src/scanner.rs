// chimera-devices/src/scanner.rs
// USB device scanner - continuously monitors for device connects/disconnects

use chimera_core::device::{DeviceInfo, DeviceState};
use chimera_core::usb::lookup_device;
use chimera_core::error::{ChimeraError, Result};
use chimera_core::event::ChimeraEvent;
use log::{info, warn};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crossbeam_channel::Sender;

/// Actively scan for USB devices
pub struct UsbScanner {
    event_sender: Sender<ChimeraEvent>,
    known_devices: Arc<Mutex<HashMap<String, DeviceInfo>>>,
}

impl UsbScanner {
    pub fn new(event_sender: Sender<ChimeraEvent>) -> Self {
        Self {
            event_sender,
            known_devices: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Scan for currently connected USB devices
    pub fn scan_once(&self) -> Result<Vec<DeviceInfo>> {
        let devices = rusb::devices()
            .map_err(|e| ChimeraError::Usb(e.to_string()))?;
        
        let mut found = Vec::new();
        
        for device in devices.iter() {
            let desc = match device.device_descriptor() {
                Ok(d) => d,
                Err(_) => continue,
            };
            
            let vid = desc.vendor_id();
            let pid = desc.product_id();
            
            if let Some(db_entry) = lookup_device(vid, pid) {
                let mut info = DeviceInfo::new_unknown(format!("{:04x}:{:04x}", vid, pid));
                info.brand = db_entry.brand.clone();
                info.connection_mode = db_entry.mode.clone();
                info.usb_vid = Some(vid);
                info.usb_pid = Some(pid);
                info.state = DeviceState::Connected;
                
                // Try to get serial number
                if let Ok(handle) = device.open() {
                    if let Ok(serial) = handle.read_serial_number_string_ascii(&desc) {
                        info.serial = Some(serial.clone());
                        info.id = serial;
                    }
                }
                
                found.push(info);
            }
        }
        
        Ok(found)
    }

    /// Start polling for device changes
    pub fn start_polling(self, poll_interval_ms: u64) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut prev_devices: HashMap<String, DeviceInfo> = HashMap::new();
            
            loop {
                match self.scan_once() {
                    Ok(current) => {
                        let current_map: HashMap<String, DeviceInfo> = current
                            .into_iter()
                            .map(|d| (d.id.clone(), d))
                            .collect();
                        
                        // Find new devices
                        for (id, info) in &current_map {
                            if !prev_devices.contains_key(id) {
                                info!("Device connected: {} ({:?})", id, info.brand);
                                let _ = self.event_sender.send(ChimeraEvent::DeviceConnected(info.clone()));
                                
                                if let Ok(mut known) = self.known_devices.lock() {
                                    known.insert(id.clone(), info.clone());
                                }
                            }
                        }
                        
                        // Find disconnected devices
                        for (id, info) in &prev_devices {
                            if !current_map.contains_key(id) {
                                info!("Device disconnected: {}", id);
                                let serial = info.serial.clone().unwrap_or_else(|| id.clone());
                                let _ = self.event_sender.send(ChimeraEvent::DeviceDisconnected(serial));
                                
                                if let Ok(mut known) = self.known_devices.lock() {
                                    known.remove(id);
                                }
                            }
                        }
                        
                        prev_devices = current_map;
                    }
                    Err(e) => {
                        warn!("USB scan error: {}", e);
                    }
                }
                
                std::thread::sleep(Duration::from_millis(poll_interval_ms));
            }
        })
    }
}
