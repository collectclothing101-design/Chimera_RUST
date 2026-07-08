// chimera-adb/src/client.rs
// ADB client - connects via TCP to ADB server (localhost:5037) or directly

use std::net::TcpStream;
use std::io::{Read, Write};
use std::time::Duration;
use chimera_core::error::{ChimeraError, Result};
use chimera_core::device::{DeviceInfo, DeviceBrand, DeviceChipset, ConnectionMode, DeviceState};
use base64::{Engine as _, engine::general_purpose};

const ADB_SERVER_PORT: u16 = 5037;
const ADB_SERVER_HOST: &str = "127.0.0.1";

/// ADB device entry
#[derive(Debug, Clone)]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
    pub model: Option<String>,
    pub product: Option<String>,
    pub transport_id: Option<u32>,
}

impl AdbDevice {
    pub fn to_device_info(&self) -> DeviceInfo {
        let mut info = DeviceInfo::new_unknown(self.serial.clone());
        info.connection_mode = ConnectionMode::Adb;
        info.state = match self.state.as_str() {
            "device" => DeviceState::Authorized,
            "unauthorized" => DeviceState::Unauthorized,
            "offline" => DeviceState::Offline,
            "recovery" => DeviceState::Recovery,
            "sideload" => DeviceState::Sideload,
            "bootloader" => DeviceState::Bootloader,
            _ => DeviceState::Connected,
        };
        if let Some(model) = &self.model {
            info.model = model.clone();
        }
        info
    }
}

/// Main ADB client
pub struct AdbClient {
    server_host: String,
    server_port: u16,
    timeout: Duration,
}

impl AdbClient {
    pub fn new() -> Self {
        Self {
            server_host: ADB_SERVER_HOST.to_string(),
            server_port: ADB_SERVER_PORT,
            timeout: Duration::from_secs(10),
        }
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout = Duration::from_secs(secs);
        self
    }

    /// Connect to ADB server, return a raw TcpStream
    fn connect_server(&self) -> Result<TcpStream> {
        let stream = TcpStream::connect(format!("{}:{}", self.server_host, self.server_port))
            .map_err(|e| ChimeraError::Adb(format!("Cannot connect to ADB server: {}", e)))?;
        stream.set_read_timeout(Some(self.timeout)).ok();
        stream.set_write_timeout(Some(self.timeout)).ok();
        Ok(stream)
    }

    /// Send a raw ADB protocol request and read response
    fn send_request(&self, request: &str) -> Result<Vec<u8>> {
        let mut stream = self.connect_server()?;
        let msg = format!("{:04X}{}", request.len(), request);
        stream.write_all(msg.as_bytes()).map_err(|e| ChimeraError::Adb(e.to_string()))?;
        
        // Read OKAY/FAIL status
        let mut status = [0u8; 4];
        stream.read_exact(&mut status).map_err(|e| ChimeraError::Adb(e.to_string()))?;
        
        match &status {
            b"OKAY" => {
                let mut response = Vec::new();
                stream.read_to_end(&mut response).ok();
                Ok(response)
            }
            b"FAIL" => {
                // Read error length + message
                let mut len_bytes = [0u8; 4];
                stream.read_exact(&mut len_bytes).map_err(|e| ChimeraError::Adb(e.to_string()))?;
                let len = u32::from_str_radix(std::str::from_utf8(&len_bytes).unwrap_or("0000"), 16).unwrap_or(0) as usize;
                let mut err_msg = vec![0u8; len];
                stream.read_exact(&mut err_msg).ok();
                Err(ChimeraError::Adb(String::from_utf8_lossy(&err_msg).to_string()))
            }
            other => {
                Err(ChimeraError::Adb(format!("Unexpected ADB status: {:?}", other)))
            }
        }
    }

    /// List connected devices
    pub fn list_devices(&self) -> Result<Vec<AdbDevice>> {
        let response = self.send_request("host:devices-l")?;
        let text = String::from_utf8_lossy(&response);
        
        // Skip 4-byte length prefix if present
        let lines_text = if response.len() >= 4 {
            let prefix = std::str::from_utf8(&response[..4]).unwrap_or("");
            if prefix.chars().all(|c| c.is_ascii_hexdigit()) {
                std::str::from_utf8(&response[4..]).unwrap_or(&text)
            } else {
                &text
            }
        } else {
            &text
        };
        
        let mut devices = Vec::new();
        for line in lines_text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            let mut parts = line.split_whitespace();
            let serial = parts.next().unwrap_or("").to_string();
            let state = parts.next().unwrap_or("unknown").to_string();
            
            let mut model = None;
            let mut product = None;
            let mut transport_id = None;
            
            for part in parts {
                if let Some(m) = part.strip_prefix("model:") {
                    model = Some(m.to_string());
                } else if let Some(p) = part.strip_prefix("product:") {
                    product = Some(p.to_string());
                } else if let Some(t) = part.strip_prefix("transport_id:") {
                    transport_id = t.parse().ok();
                }
            }
            
            if !serial.is_empty() {
                devices.push(AdbDevice { serial, state, model, product, transport_id });
            }
        }
        
        Ok(devices)
    }

    /// Execute shell command on device, return output
    pub fn shell(&self, serial: &str, cmd: &str) -> Result<String> {
        let request = format!("host:transport:{}", serial);
        let mut stream = self.connect_server()?;
        
        // Set transport
        let msg = format!("{:04X}{}", request.len(), request);
        stream.write_all(msg.as_bytes())?;
        
        let mut status = [0u8; 4];
        stream.read_exact(&mut status)?;
        if &status != b"OKAY" {
            return Err(ChimeraError::Adb(format!("Transport failed for {}", serial)));
        }
        
        // Send shell command
        let shell_req = format!("shell:{}", cmd);
        let shell_msg = format!("{:04X}{}", shell_req.len(), shell_req);
        stream.write_all(shell_msg.as_bytes())?;
        
        let mut status2 = [0u8; 4];
        stream.read_exact(&mut status2)?;
        if &status2 != b"OKAY" {
            return Err(ChimeraError::Adb(format!("Shell command failed: {}", cmd)));
        }
        
        let mut output = Vec::new();
        stream.read_to_end(&mut output)?;
        
        Ok(String::from_utf8_lossy(&output).to_string())
    }

    /// Get a property from the device
    pub fn get_prop(&self, serial: &str, prop: &str) -> Result<String> {
        let output = self.shell(serial, &format!("getprop {}", prop))?;
        Ok(output.trim().to_string())
    }

    /// Set a property on the device (requires root)
    pub fn set_prop(&self, serial: &str, prop: &str, value: &str) -> Result<()> {
        self.shell(serial, &format!("setprop {} {}", prop, value))?;
        Ok(())
    }

    /// Reboot the device
    pub fn reboot(&self, serial: &str, mode: Option<&str>) -> Result<()> {
        let cmd = match mode {
            Some(m) => format!("reboot:{}", m),
            None => "reboot:".to_string(),
        };
        
        let request = format!("host:transport:{}", serial);
        let mut stream = self.connect_server()?;
        let msg = format!("{:04X}{}", request.len(), request);
        stream.write_all(msg.as_bytes())?;
        let mut status = [0u8; 4];
        stream.read_exact(&mut status)?;
        
        let reboot_msg = format!("{:04X}{}", cmd.len(), cmd);
        stream.write_all(reboot_msg.as_bytes())?;
        Ok(())
    }

    /// Get full device info from ADB
    pub fn get_device_info(&self, serial: &str) -> Result<DeviceInfo> {
        let mut info = DeviceInfo::new_unknown(serial);
        info.connection_mode = ConnectionMode::Adb;
        info.state = DeviceState::Authorized;
        
        // Read key properties
        let props = [
            ("ro.product.brand", "brand"),
            ("ro.product.model", "model"),
            ("ro.product.name", "product_name"),
            ("ro.build.version.release", "android_version"),
            ("ro.build.display.id", "build_id"),
            ("ro.baseband.version", "baseband"),
            ("ro.build.version.security_patch", "security_patch"),
            ("ro.hardware", "hardware"),
            ("ro.boot.hardware", "boot_hardware"),
            ("persist.sys.csc", "csc"),
            ("ro.csc.country_code", "country"),
            ("ril.iccid.sim1", "iccid"),
        ];
        
        for (prop, _key) in &props {
            if let Ok(val) = self.get_prop(serial, prop) {
                let val = val.trim();
                if !val.is_empty() && val != "unknown" {
                    match *_key {
                        "brand" => {
                            info.brand = parse_brand(val);
                        }
                        "model" => {
                            info.model = val.to_string();
                        }
                        "android_version" => {
                            info.android_version = Some(val.to_string());
                        }
                        "build_id" => {
                            info.build_number = Some(val.to_string());
                        }
                        "baseband" => {
                            info.baseband_version = Some(val.to_string());
                        }
                        "security_patch" => {
                            info.security_patch = Some(val.to_string());
                        }
                        "csc" => {
                            info.csc = Some(val.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Try to get IMEI via service call
        if let Ok(imei) = self.shell(serial, "service call iphonesubinfo 1 | grep -oE '[0-9]{15}'") {
            let imei = imei.trim().to_string();
            if !imei.is_empty() {
                info.imei = Some(imei);
            }
        }
        
        // Get chipset info
        if let Ok(soc) = self.get_prop(serial, "ro.hardware.chipname") {
            if !soc.is_empty() {
                info.chipset = parse_chipset(&soc);
            }
        }
        
        info.serial = Some(serial.to_string());
        Ok(info)
    }

    /// Enable ADB over TCP (WiFi ADB)
    pub fn enable_tcpip(&self, serial: &str, port: u16) -> Result<()> {
        let request = format!("host:transport:{}", serial);
        let mut stream = self.connect_server()?;
        let msg = format!("{:04X}{}", request.len(), request);
        stream.write_all(msg.as_bytes())?;
        let mut status = [0u8; 4];
        stream.read_exact(&mut status)?;
        
        let tcpip_cmd = format!("tcpip:{}", port);
        let tcpip_msg = format!("{:04X}{}", tcpip_cmd.len(), tcpip_cmd);
        stream.write_all(tcpip_msg.as_bytes())?;
        Ok(())
    }

    /// Push file to device
    pub fn push(&self, serial: &str, local_path: &str, remote_path: &str) -> Result<()> {
        // Use shell cp + base64 for small files
        let data = std::fs::read(local_path).map_err(|e| ChimeraError::Io(e.to_string()))?;
        let b64 = general_purpose::STANDARD.encode(&data);
        
        // Split into chunks and push via shell
        let chunk_size = 1024;
        let first_chunk = &b64[..chunk_size.min(b64.len())];
        self.shell(serial, &format!("echo '{}' | base64 -d > {}", first_chunk, remote_path))?;
        
        let mut offset = chunk_size.min(b64.len());
        while offset < b64.len() {
            let end = (offset + chunk_size).min(b64.len());
            let chunk = &b64[offset..end];
            self.shell(serial, &format!("echo '{}' | base64 -d >> {}", chunk, remote_path))?;
            offset = end;
        }
        
        Ok(())
    }

    /// Pull file from device  
    pub fn pull(&self, serial: &str, remote_path: &str, local_path: &str) -> Result<Vec<u8>> {
        let b64 = self.shell(serial, &format!("base64 {}", remote_path))?;
        let data = general_purpose::STANDARD.decode(b64.trim())
            .map_err(|e| ChimeraError::Adb(format!("Base64 decode failed: {}", e)))?;
        
        if !local_path.is_empty() {
            std::fs::write(local_path, &data)?;
        }
        
        Ok(data)
    }

    /// Root the ADB daemon
    pub fn root(&self, serial: &str) -> Result<()> {
        let request = format!("host:transport:{}", serial);
        let mut stream = self.connect_server()?;
        let msg = format!("{:04X}{}", request.len(), request);
        stream.write_all(msg.as_bytes())?;
        let mut status = [0u8; 4];
        stream.read_exact(&mut status)?;
        
        let root_msg = format!("{:04X}root:", 5);
        stream.write_all(root_msg.as_bytes())?;
        Ok(())
    }

    /// Get ADB server version
    pub fn server_version(&self) -> Result<u32> {
        let response = self.send_request("host:version")?;
        let ver_str = String::from_utf8_lossy(&response);
        let ver_str = ver_str.trim();
        u32::from_str_radix(ver_str, 16)
            .map_err(|e| ChimeraError::Adb(format!("Invalid version: {}", e)))
    }

    /// Kill ADB server
    pub fn kill_server(&self) -> Result<()> {
        let _ = self.send_request("host:kill");
        Ok(())
    }

    /// Start ADB server (spawn adb server process)
    pub fn start_server(&self) -> Result<()> {
        std::process::Command::new("adb")
            .arg("start-server")
            .output()
            .map_err(|e| ChimeraError::Adb(format!("Cannot start ADB server: {}", e)))?;
        Ok(())
    }

    // ─── TCP/Wi-Fi connection helpers ─────────────────────────────

    /// Connect to a device over TCP/IP. `target` is `host:port` (port
    /// defaults to 5555 if omitted). Equivalent to `adb connect host:port`.
    pub fn connect_tcp(&self, target: &str) -> Result<String> {
        let addr = if target.contains(':') { target.to_string() }
                   else { format!("{}:5555", target) };
        let response = self.send_request(&format!("host:connect:{}", addr))?;
        Ok(String::from_utf8_lossy(&response).trim().to_string())
    }

    /// Disconnect a TCP-connected device. Pass `host:port` or "" to
    /// disconnect every TCP target.
    pub fn disconnect_tcp(&self, target: &str) -> Result<String> {
        let path = if target.is_empty() {
            "host:disconnect:".to_string()
        } else {
            format!("host:disconnect:{}", target)
        };
        let response = self.send_request(&path)?;
        Ok(String::from_utf8_lossy(&response).trim().to_string())
    }

    /// Alias matching the `adb connect` / `adb disconnect` UX.
    pub fn connect(&self, target: &str) -> Result<String> { self.connect_tcp(target) }
    pub fn disconnect(&self, target: &str) -> Result<String> { self.disconnect_tcp(target) }
}

impl Default for AdbClient {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_brand(s: &str) -> DeviceBrand {
    match s.to_lowercase().as_str() {
        "samsung" => DeviceBrand::Samsung,
        "xiaomi" | "redmi" | "poco" => DeviceBrand::Xiaomi,
        "huawei" => DeviceBrand::Huawei,
        "honor" => DeviceBrand::Honor,
        "oppo" => DeviceBrand::Oppo,
        "realme" => DeviceBrand::Realme,
        "vivo" => DeviceBrand::Vivo,
        "oneplus" => DeviceBrand::OnePlus,
        "motorola" | "moto" | "lenovo" => DeviceBrand::Motorola,
        "lg" | "lge" => DeviceBrand::LG,
        "htc" => DeviceBrand::HTC,
        "sony" | "sonyericsson" => DeviceBrand::Sony,
        "nokia" | "hmd" => DeviceBrand::Nokia,
        "zte" => DeviceBrand::ZTE,
        "asus" => DeviceBrand::Asus,
        _ => DeviceBrand::Unknown,
    }
}

fn parse_chipset(s: &str) -> DeviceChipset {
    let lower = s.to_lowercase();
    if lower.contains("snapdragon") || lower.contains("msm") || lower.contains("sm8") || lower.contains("qcom") {
        DeviceChipset::Qualcomm
    } else if lower.contains("exynos") {
        DeviceChipset::Exynos
    } else if lower.contains("kirin") || lower.contains("hi") {
        DeviceChipset::Kirin
    } else if lower.contains("helio") || lower.contains("mtk") || lower.contains("mediatek") || lower.contains("mt6") {
        DeviceChipset::MediaTek
    } else if lower.contains("unisoc") || lower.contains("spreadtrum") || lower.contains("sc") {
        DeviceChipset::Unisoc
    } else {
        DeviceChipset::Unknown
    }
}
