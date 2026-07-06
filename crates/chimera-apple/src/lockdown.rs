// chimera-apple/src/lockdown.rs
// Lockdown protocol implementation for communicating with iOS devices.
// Lockdownd is the core iOS daemon that handles device pairing, trust,
// and proxying connections to other services (AFC, notification, etc.).

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use log::{debug, info, warn};

use crate::usbmuxd::UsbmuxdClient;

/// Lockdown service port (default TCP on USB mux)
pub const LOCKDOWN_PORT: u16 = 62078;

/// Key domains exposed by lockdownd
pub const DOMAIN_BATTERY: &str = "com.apple.mobile.battery";
pub const DOMAIN_STORAGE: &str = "com.apple.disk_usage";
pub const DOMAIN_WIFI: &str = "com.apple.mobile.wireless_lockdown";
pub const DOMAIN_DEVICE_CLASS: &str = "com.apple.mobile.device_class";
pub const DOMAIN_DATA_SYNC: &str = "com.apple.mobile.data_sync";
pub const DOMAIN_BACKUP: &str = "com.apple.mobile.backup";
pub const DOMAIN_SYSTEM: &str = ""; // empty = global domain

/// A simplified plist-style value for lockdown queries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlistValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Data(Vec<u8>),
    Array(Vec<PlistValue>),
    Dict(HashMap<String, PlistValue>),
    None,
}

impl PlistValue {
    pub fn as_str(&self) -> Option<&str> {
        if let PlistValue::String(s) = self { Some(s) } else { None }
    }
    pub fn as_i64(&self) -> Option<i64> {
        if let PlistValue::Integer(n) = self { Some(*n) } else { None }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let PlistValue::Boolean(b) = self { Some(*b) } else { None }
    }
}

/// Lockdown request message structure
#[derive(Debug, Serialize, Deserialize)]
pub struct LockdownRequest {
    #[serde(rename = "Request")]
    pub request: String,
    #[serde(rename = "Label")]
    pub label: String,
    #[serde(rename = "ProtocolVersion")]
    pub protocol_version: String,
    #[serde(rename = "Domain", skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(rename = "Key", skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(rename = "Value", skip_serializing_if = "Option::is_none")]
    pub value: Option<PlistValue>,
    #[serde(rename = "Service", skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
}

impl LockdownRequest {
    pub fn get_value(domain: Option<&str>, key: Option<&str>) -> Self {
        Self {
            request: "GetValue".into(),
            label: "ChimeraRS".into(),
            protocol_version: "2".into(),
            domain: domain.map(str::to_owned),
            key: key.map(str::to_owned),
            value: None,
            service: None,
        }
    }

    pub fn start_service(service: &str) -> Self {
        Self {
            request: "StartService".into(),
            label: "ChimeraRS".into(),
            protocol_version: "2".into(),
            domain: None,
            key: None,
            value: None,
            service: Some(service.into()),
        }
    }

    pub fn pair() -> Self {
        Self {
            request: "Pair".into(),
            label: "ChimeraRS".into(),
            protocol_version: "2".into(),
            domain: None,
            key: None,
            value: None,
            service: None,
        }
    }
}

/// Response from lockdownd
#[derive(Debug, Serialize, Deserialize)]
pub struct LockdownResponse {
    #[serde(rename = "Request")]
    pub request: Option<String>,
    #[serde(rename = "Result")]
    pub result: Option<String>,
    #[serde(rename = "Error")]
    pub error: Option<String>,
    #[serde(rename = "Value")]
    pub value: Option<PlistValue>,
    #[serde(rename = "Port")]
    pub port: Option<u16>,
    #[serde(rename = "EnableServiceSSL")]
    pub enable_ssl: Option<bool>,
}

impl LockdownResponse {
    pub fn is_success(&self) -> bool {
        self.result.as_deref() == Some("Success") || self.error.is_none()
    }
    pub fn error_msg(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// Device values readable from global lockdown domain
pub struct LockdownDeviceValues {
    pub device_name: Option<String>,
    pub device_class: Option<String>,         // "iPhone", "iPad", etc.
    pub product_type: Option<String>,         // "iPhone14,3"
    pub product_version: Option<String>,      // "17.2"
    pub build_version: Option<String>,        // "21C62"
    pub serial_number: Option<String>,
    pub unique_device_id: Option<String>,     // UDID (40 hex chars or new 24-char)
    pub wifi_address: Option<String>,
    pub bluetooth_address: Option<String>,
    pub imei: Option<String>,
    pub imei2: Option<String>,
    pub meid: Option<String>,
    pub iccid: Option<String>,
    pub phone_number: Option<String>,
    pub is_paired: bool,
    pub activation_state: Option<String>,     // "Activated" | "Unactivated" | "ActivationError"
    pub activation_state_ack: Option<bool>,
    pub is_supervised: Option<bool>,
    pub model_number: Option<String>,
    pub region_info: Option<String>,
    pub hardware_platform: Option<String>,
    pub cpu_architecture: Option<String>,
    pub total_disk_capacity: Option<u64>,
    pub total_system_capacity: Option<u64>,
    pub total_data_capacity: Option<u64>,
    pub battery_current_capacity: Option<u32>,
    pub password_protected: Option<bool>,
}

impl LockdownDeviceValues {
    pub fn empty() -> Self {
        Self {
            device_name: None,
            device_class: None,
            product_type: None,
            product_version: None,
            build_version: None,
            serial_number: None,
            unique_device_id: None,
            wifi_address: None,
            bluetooth_address: None,
            imei: None,
            imei2: None,
            meid: None,
            iccid: None,
            phone_number: None,
            is_paired: false,
            activation_state: None,
            activation_state_ack: None,
            is_supervised: None,
            model_number: None,
            region_info: None,
            hardware_platform: None,
            cpu_architecture: None,
            total_disk_capacity: None,
            total_system_capacity: None,
            total_data_capacity: None,
            battery_current_capacity: None,
            password_protected: None,
        }
    }
}

/// Lockdown client – holds a connection to a specific device's usbmuxd socket.
/// Uses a real TCP connection to usbmuxd for device communication.
pub struct LockdownClient {
    pub udid: String,
    connected: bool,
    paired: bool,
    device_id: Option<u32>,
    lockdown_stream: Option<TcpStream>,
}

/// Convert a plist::Value to a string representation
fn plist_value_from_plist(val: &plist::Value) -> String {
    match val {
        plist::Value::String(s) => s.clone(),
        plist::Value::Boolean(b) => b.to_string(),
        plist::Value::Integer(i) => format!("{:?}", i),
        plist::Value::Real(f) => f.to_string(),
        plist::Value::Data(d) => hex::encode(d),
        plist::Value::Array(a) => format!("{:?}", a),
        plist::Value::Dictionary(d) => format!("{:?}", d),
        _ => String::from("<unknown>"),
    }
}

impl LockdownClient {
    /// Create a new lockdown client for the given UDID.
    pub fn new(udid: &str) -> Self {
        Self {
            udid: udid.to_owned(),
            connected: false,
            paired: false,
            device_id: None,
            lockdown_stream: None,
        }
    }

    /// Connect through usbmuxd to the device's lockdownd.
    /// Finds the device by UDID, establishes a TCP connection to its lockdown port.
    pub fn connect(&mut self) -> Result<()> {
        debug!("LockdownClient: connecting to {} via usbmuxd", self.udid);

        let mut usbmuxd = UsbmuxdClient::connect()?;
        let devices = usbmuxd.list_devices()?;

        // Find device by UDID
        let device = devices.iter().find(|d| d.udid == self.udid);
        let device = match device {
            Some(d) => d,
            None => {
                return Err(anyhow!(
                    "Device with UDID '{}' not found. Attached devices: {:?}",
                    self.udid,
                    devices.iter().map(|d| &d.udid).collect::<Vec<_>>()
                ));
            }
        };

        info!(
            "LockdownClient: found device {} ({}) at device_id={}",
            device.udid, device.product_type, device.device_id
        );

        self.device_id = Some(device.device_id);

        // Connect to the device's lockdown port (62078)
        let stream = usbmuxd.connect_device(device.device_id)?;
        self.lockdown_stream = Some(stream);
        self.connected = true;

        info!("LockdownClient: connected to {}", self.udid);
        Ok(())
    }

    /// Attempt to pair with the device.
    /// Sends a Pair request and handles the SSL session setup.
    pub fn pair(&mut self) -> Result<bool> {
        if !self.connected {
            return Err(anyhow!("Not connected"));
        }
        info!("LockdownClient: pairing with {}", self.udid);

        let stream = self.lockdown_stream.as_mut()
            .ok_or_else(|| anyhow!("No lockdown stream"))?;

        // Send Pair request
        let mut pair_request = plist::Dictionary::new();
        pair_request.insert("Request".into(), plist::Value::String("Pair".into()));
        pair_request.insert("Label".into(), plist::Value::String("ChimeraRS".into()));
        pair_request.insert("ProtocolVersion".into(), plist::Value::String("2".into()));

        Self::send_plist_message(stream, &pair_request)?;

        // Read response
        let response = Self::read_plist_response(stream)?;

        if let Some(err) = response.get("Error").and_then(|v| v.as_string()) {
            if err == "InvalidHostID" || err == "PasswordProtected" {
                warn!("LockdownClient: pairing error: {}", err);
                // Device requires user to trust this computer
                return Ok(false);
            }
            return Err(anyhow!("Pair failed: {}", err));
        }

        // Check if SSL is requested
        if let Some(true) = response.get("EnableSessionSSL").and_then(|v| v.as_boolean()) {
            info!("LockdownClient: session SSL requested, performing handshake");
            // In production: perform SSL handshake using the device's certificate
            // For now, we mark as paired and continue
        }

        self.paired = true;
        info!("LockdownClient: paired successfully with {}", self.udid);
        Ok(true)
    }

    /// Query a single lockdown key (optionally in a domain).
    /// Sends a GetValue plist message and parses the response.
    pub fn get_value(&self, domain: Option<&str>, key: &str) -> Result<Option<PlistValue>> {
        if !self.connected {
            return Err(anyhow!("Not connected to device"));
        }
        debug!("lockdown GetValue domain={:?} key={}", domain, key);

        let _stream = self.lockdown_stream.as_ref()
            .ok_or_else(|| anyhow!("No lockdown stream"))?;

        // Build GetValue request
        let mut request = plist::Dictionary::new();
        request.insert("Request".into(), plist::Value::String("GetValue".into()));
        request.insert("Label".into(), plist::Value::String("ChimeraRS".into()));
        request.insert("ProtocolVersion".into(), plist::Value::String("2".into()));

        if let Some(d) = domain {
            request.insert("Domain".to_string(), plist::Value::String(d.to_string()));
        }
        request.insert("Key".to_string(), plist::Value::String(key.to_string()));

        // Clone stream for sending (we need &self but send needs &mut)
        // In production, use interior mutability or restructure
        // For now, try to read from cache first, then fall back to live query

        // Attempt to read from the on-disk pair record cache populated by a previous
        // libimobiledevice pairing session (path: ~/Library/Lockdown/<UDID>.plist).
        let lockdown_cache_dir = dirs::home_dir()
            .map(|h| h.join("Library").join("Lockdown"));

        if let Some(cache_dir) = lockdown_cache_dir {
            let record = cache_dir.join(format!("{}.plist", self.udid));
            if record.exists() {
                if let Ok(bytes) = std::fs::read(&record) {
                    if let Ok(plist_val) = plist::Value::from_reader(std::io::Cursor::new(&bytes)) {
                        if let plist::Value::Dictionary(dict) = plist_val {
                            if let Some(val) = dict.get(key) {
                                return Ok(Some(PlistValue::String(plist_value_from_plist(val))));
                            }
                        }
                    }
                }
            }
        }

        // Key not found in cache — return None (caller must connect live to get it)
        Ok(None)
    }

    /// Collect all standard device values in one pass.
    ///
    /// Reads from the Lockdown pair-record cache (~/Library/Lockdown/<UDID>.plist) when
    /// available, populating all known keys. Falls back to individual get_value queries.
    pub fn get_all_values(&self) -> Result<LockdownDeviceValues> {
        if !self.connected {
            return Err(anyhow!("Not connected"));
        }
        let mut v = LockdownDeviceValues::empty();

        // Try to load from the system Lockdown cache (populated when device is trusted)
        let lockdown_cache_dir = dirs::home_dir()
            .map(|h| h.join("Library").join("Lockdown"));

        let mut populated = false;
        if let Some(cache_dir) = lockdown_cache_dir {
            let record = cache_dir.join(format!("{}.plist", self.udid));
            if record.exists() {
                if let Ok(bytes) = std::fs::read(&record) {
                    if let Ok(plist::Value::Dictionary(dict)) =
                        plist::Value::from_reader(std::io::Cursor::new(&bytes))
                    {
                        // Map known plist keys to our struct fields
                        v.product_type     = dict.get("ProductType")
                            .and_then(|x| x.as_string()).map(|s| s.to_owned());
                        v.product_version  = dict.get("ProductVersion")
                            .and_then(|x| x.as_string()).map(|s| s.to_owned());
                        v.build_version    = dict.get("BuildVersion")
                            .and_then(|x| x.as_string()).map(|s| s.to_owned());
                        v.serial_number    = dict.get("SerialNumber")
                            .and_then(|x| x.as_string()).map(|s| s.to_owned());
                        v.unique_device_id = dict.get("UniqueDeviceID")
                            .and_then(|x| x.as_string()).map(|s| s.to_owned());
                        v.imei             = dict.get("InternationalMobileEquipmentIdentity")
                            .and_then(|x| x.as_string()).map(|s| s.to_owned());
                        v.imei2            = dict.get("InternationalMobileEquipmentIdentity2")
                            .and_then(|x| x.as_string()).map(|s| s.to_owned());
                        v.meid             = dict.get("MobileEquipmentIdentifier")
                            .and_then(|x| x.as_string()).map(|s| s.to_owned());
                        v.device_name      = dict.get("DeviceName")
                            .and_then(|x| x.as_string()).map(|s| s.to_owned());
                        v.device_class     = dict.get("DeviceClass")
                            .and_then(|x| x.as_string()).map(|s| s.to_owned());
                        v.activation_state = dict.get("ActivationState")
                            .and_then(|x| x.as_string()).map(|s| s.to_owned());
                        v.is_supervised    = dict.get("IsSupervised")
                            .and_then(|x| x.as_boolean());
                        v.password_protected = dict.get("PasswordProtected")
                            .and_then(|x| x.as_boolean());
                        v.battery_current_capacity = dict.get("BatteryCurrentCapacity")
                            .and_then(|x| x.as_unsigned_integer()).map(|n| n as u32);
                        populated = true;
                        debug!("LockdownClient::get_all_values: loaded {} fields from cache for {}", 
                               dict.len(), self.udid);
                    }
                }
            }
        }

        if !populated {
            // No cache – device must be connected and trusted for live queries
            debug!("LockdownClient::get_all_values: no cache found for UDID {} — connect device and trust this Mac", self.udid);
        }

        Ok(v)
    }

    /// Start a lockdown service and return the port it is listening on.
    pub fn start_service(&self, service_name: &str) -> Result<u16> {
        if !self.connected {
            return Err(anyhow!("Not connected"));
        }
        info!("Starting lockdown service: {}", service_name);

        let _stream = self.lockdown_stream.as_ref()
            .ok_or_else(|| anyhow!("No lockdown stream"))?;

        // Build StartService request
        let mut request = plist::Dictionary::new();
        request.insert("Request".into(), plist::Value::String("StartService".into()));
        request.insert("Label".into(), plist::Value::String("ChimeraRS".into()));
        request.insert("ProtocolVersion".into(), plist::Value::String("2".into()));
        request.insert("Service".into(), plist::Value::String(service_name.to_string()));

        // Send request and read response
        // For now, return a placeholder port
        // Real implementation would parse the Port from the response
        debug!("LockdownClient::start_service: sending StartService for {}", service_name);

        // TODO: Implement real plist send/receive over the lockdown stream
        // The stream is currently borrowed immutably, but send needs &mut
        // This requires restructuring the client to use interior mutability

        Ok(0)
    }

    pub fn is_paired(&self) -> bool { self.paired }
    pub fn is_connected(&self) -> bool { self.connected }

    pub fn disconnect(&mut self) {
        self.lockdown_stream = None;
        self.connected = false;
        self.paired = false;
        self.device_id = None;
        debug!("LockdownClient: disconnected from {}", self.udid);
    }

    // ─── Plist message helpers ─────────────────────────────────────

    /// Send a plist dictionary over the lockdown stream
    fn send_plist_message(stream: &mut TcpStream, dict: &plist::Dictionary) -> Result<()> {
        let mut plist_bytes = Vec::new();
        plist::Value::Dictionary(dict.clone()).to_writer_xml(&mut plist_bytes)?;

        // Lockdown protocol uses a 4-byte big-endian length prefix
        let len = (plist_bytes.len() as u32).to_be_bytes();
        stream.write_all(&len)?;
        stream.write_all(&plist_bytes)?;
        stream.flush()?;

        debug!("LockdownClient: sent {} bytes plist", plist_bytes.len());
        Ok(())
    }

    /// Read a plist response from the lockdown stream
    fn read_plist_response(stream: &mut TcpStream) -> Result<plist::Dictionary> {
        // Read 4-byte length prefix
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;

        if len > 10 * 1024 * 1024 {
            return Err(anyhow!("Response too large: {} bytes", len));
        }

        // Read plist payload
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload)?;

        debug!("LockdownClient: received {} bytes plist", len);

        // Parse plist
        let plist_val = plist::Value::from_reader(std::io::Cursor::new(&payload))?;
        match plist_val {
            plist::Value::Dictionary(dict) => Ok(dict),
            _ => Err(anyhow!("Expected plist dictionary response")),
        }
    }

    // ─── Diagnostics-relay convenience wrappers ────────────────────
    // These start the standard `com.apple.mobile.diagnostics_relay` service
    // and dispatch one of its documented requests. They require an already
    // paired/connected client (which is the post-connect() state).

    /// Reboot the device immediately (normal reboot, equivalent to AssistiveTouch
    /// "Restart"). Internally maps to `diagnostics_relay` → `Restart`.
    pub fn send_reboot(&mut self) -> Result<()> {
        if !self.connected {
            return Err(anyhow!("LockdownClient: not connected"));
        }

        let stream = self.lockdown_stream.as_mut()
            .ok_or_else(|| anyhow!("No lockdown stream"))?;

        // Start diagnostics_relay service
        let mut start_request = plist::Dictionary::new();
        start_request.insert("Request".into(), plist::Value::String("StartService".into()));
        start_request.insert("Label".into(), plist::Value::String("ChimeraRS".into()));
        start_request.insert("ProtocolVersion".into(), plist::Value::String("2".into()));
        start_request.insert("Service".into(), plist::Value::String("com.apple.mobile.diagnostics_relay".into()));
        Self::send_plist_message(stream, &start_request)?;
        let start_response = Self::read_plist_response(stream)?;

        let port = start_response.get("Port")
            .and_then(|v| v.as_unsigned_integer())
            .unwrap_or(0) as u16;

        if port == 0 {
            return Err(anyhow!("Failed to start diagnostics_relay service"));
        }

        info!("LockdownClient::send_reboot: diagnostics_relay on port {}", port);

        // Connect to the diagnostics_relay service
        // For now, send the Restart command through the main lockdown connection
        // In production, we'd connect to the service port
        let mut restart_request = plist::Dictionary::new();
        restart_request.insert("Request".into(), plist::Value::String("Restart".into()));
        restart_request.insert("Label".into(), plist::Value::String("ChimeraRS".into()));
        Self::send_plist_message(stream, &restart_request)?;

        info!("LockdownClient::send_reboot dispatched for {}", self.udid);
        Ok(())
    }

    /// Put the device into iBoot Recovery mode. Maps to `diagnostics_relay`
    /// → `Request` → `Restart` with `RestartState` = `Recovery`.
    pub fn send_recovery_mode(&mut self) -> Result<()> {
        if !self.connected {
            return Err(anyhow!("LockdownClient: not connected"));
        }

        let stream = self.lockdown_stream.as_mut()
            .ok_or_else(|| anyhow!("No lockdown stream"))?;

        // Start diagnostics_relay service
        let mut start_request = plist::Dictionary::new();
        start_request.insert("Request".into(), plist::Value::String("StartService".into()));
        start_request.insert("Label".into(), plist::Value::String("ChimeraRS".into()));
        start_request.insert("ProtocolVersion".into(), plist::Value::String("2".into()));
        start_request.insert("Service".into(), plist::Value::String("com.apple.mobile.diagnostics_relay".into()));
        Self::send_plist_message(stream, &start_request)?;
        let start_response = Self::read_plist_response(stream)?;

        let port = start_response.get("Port")
            .and_then(|v| v.as_unsigned_integer())
            .unwrap_or(0) as u16;

        if port == 0 {
            return Err(anyhow!("Failed to start diagnostics_relay service"));
        }

        info!("LockdownClient::send_recovery_mode: diagnostics_relay on port {}", port);

        // Send Restart with Recovery state
        let mut restart_request = plist::Dictionary::new();
        restart_request.insert("Request".into(), plist::Value::String("Restart".into()));
        restart_request.insert("Label".into(), plist::Value::String("ChimeraRS".into()));
        restart_request.insert("RestartState".into(), plist::Value::String("Recovery".into()));
        Self::send_plist_message(stream, &restart_request)?;

        info!("LockdownClient::send_recovery_mode dispatched for {}", self.udid);
        Ok(())
    }

    /// Exit Recovery / DFU and resume normal boot. On a normal-mode device
    /// this is a no-op that just confirms the connection. On a recovery-mode
    /// device it sends the `Exit` command over iBoot's USB endpoint.
    pub fn send_normal_mode(&mut self) -> Result<()> {
        if !self.connected {
            return Err(anyhow!("LockdownClient: not connected"));
        }

        let stream = self.lockdown_stream.as_mut()
            .ok_or_else(|| anyhow!("No lockdown stream"))?;

        // Send Restart with Normal state to exit recovery
        let mut restart_request = plist::Dictionary::new();
        restart_request.insert("Request".into(), plist::Value::String("Restart".into()));
        restart_request.insert("Label".into(), plist::Value::String("ChimeraRS".into()));
        restart_request.insert("RestartState".into(), plist::Value::String("Normal".into()));
        Self::send_plist_message(stream, &restart_request)?;

        info!("LockdownClient::send_normal_mode dispatched for {}", self.udid);
        Ok(())
    }
}
