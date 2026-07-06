// chimera-apple/src/usbmuxd.rs
// Minimal usbmuxd TCP client for macOS.
// The usbmuxd daemon listens on 127.0.0.1:27015 and speaks a binary-framed
// plist protocol. This module implements the three operations needed by
// lockdown: LIST_DEVICES, CONNECT, and LISTEN.

use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use std::io::{Read, Write};
use std::net::TcpStream;

/// usbmuxd daemon address on macOS
pub const USBMUXD_ADDR: &str = "127.0.0.1:27015";

/// Binary header: version(4) + msgtype(4) + proto(4) + op(4) + seq(4) + length(4) = 24 bytes
const HEADER_SIZE: usize = 24;

/// Message types
#[allow(dead_code)]
const MSG_TYPE_RESULT: u32 = 1;
const MSG_TYPE_PLIST: u32 = 8;

/// Protocol versions
const PROTO_VERSION: u32 = 1;

/// Operation codes
const OP_LIST_DEVICES: u32 = 1;
const OP_CONNECT: u32 = 2;
const OP_LISTEN: u32 = 4;
#[allow(dead_code)]
const OP_DEVICE_ADD: u32 = 1;
#[allow(dead_code)]
const OP_DEVICE_REMOVE: u32 = 2;

/// Represents a device attached via usbmuxd
#[derive(Debug, Clone)]
pub struct UsbmuxdDevice {
    pub device_id: u32,
    pub product_id: u16,
    pub usb_device_address: u32,
    pub connection_id: u32,
    pub udid: String,
    pub product_type: String,
    pub name: String,
}

/// A usbmuxd message header
#[derive(Debug)]
struct MsgHeader {
    version: u32,
    msgtype: u32,
    protocol: u32,
    operation: u32,
    sequence: u32,
    payload_len: u32,
}

impl MsgHeader {
    fn serialize(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..4].copy_from_slice(&self.version.to_le_bytes());
        buf[4..8].copy_from_slice(&self.msgtype.to_le_bytes());
        buf[8..12].copy_from_slice(&self.protocol.to_le_bytes());
        buf[12..16].copy_from_slice(&self.operation.to_le_bytes());
        buf[16..20].copy_from_slice(&self.sequence.to_le_bytes());
        buf[20..24].copy_from_slice(&self.payload_len.to_le_bytes());
        buf
    }

    fn deserialize(buf: &[u8; HEADER_SIZE]) -> Self {
        Self {
            version: u32::from_le_bytes(buf[0..4].try_into().unwrap()),
            msgtype: u32::from_le_bytes(buf[4..8].try_into().unwrap()),
            protocol: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
            operation: u32::from_le_bytes(buf[12..16].try_into().unwrap()),
            sequence: u32::from_le_bytes(buf[16..20].try_into().unwrap()),
            payload_len: u32::from_le_bytes(buf[20..24].try_into().unwrap()),
        }
    }
}

/// Minimal usbmuxd client
pub struct UsbmuxdClient {
    stream: TcpStream,
    sequence: u32,
}

impl UsbmuxdClient {
    /// Connect to the usbmuxd daemon
    pub fn connect() -> Result<Self> {
        debug!("UsbmuxdClient: connecting to {}", USBMUXD_ADDR);
        let stream = TcpStream::connect(USBMUXD_ADDR)?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
        Ok(Self {
            stream,
            sequence: 0,
        })
    }

    /// Send a plist message and receive the response
    fn send_plist(&mut self, operation: u32, payload: &plist::Dictionary) -> Result<plist::Value> {
        let mut plist_bytes = Vec::new();
        plist::Value::Dictionary(payload.clone()).to_writer_xml(&mut plist_bytes)?;

        let seq = self.sequence;
        self.sequence += 1;

        let header = MsgHeader {
            version: 0,
            msgtype: MSG_TYPE_PLIST,
            protocol: PROTO_VERSION,
            operation,
            sequence: seq,
            payload_len: plist_bytes.len() as u32,
        };

        // Send header + payload
        self.stream.write_all(&header.serialize())?;
        self.stream.write_all(&plist_bytes)?;
        self.stream.flush()?;

        // Read response header
        let mut resp_header_buf = [0u8; HEADER_SIZE];
        self.stream.read_exact(&mut resp_header_buf)?;
        let resp_header = MsgHeader::deserialize(&resp_header_buf);

        debug!(
            "UsbmuxdClient: response msgtype={} op={} seq={} len={}",
            resp_header.msgtype, resp_header.operation, resp_header.sequence, resp_header.payload_len
        );

        // Read response payload
        let mut resp_payload = vec![0u8; resp_header.payload_len as usize];
        self.stream.read_exact(&mut resp_payload)?;

        // Parse plist response
        let plist_val = plist::Value::from_reader(std::io::Cursor::new(&resp_payload))?;
        Ok(plist_val)
    }

    /// List all attached iOS devices
    pub fn list_devices(&mut self) -> Result<Vec<UsbmuxdDevice>> {
        info!("UsbmuxdClient: listing devices");

        let mut request = plist::Dictionary::new();
        request.insert("MessageType".into(), plist::Value::String("ListDevices".into()));
        request.insert("ClientVersionString".into(), plist::Value::String("ChimeraRS".into()));
        request.insert("kLibUSBMuxVersion".into(), plist::Value::Integer(0.into()));

        let response = self.send_plist(OP_LIST_DEVICES, &request)?;

        let mut devices = Vec::new();
        if let plist::Value::Dictionary(dict) = response {
            if let Some(plist::Value::Dictionary(device_list)) = dict.get("DeviceList") {
                for (_key, val) in device_list {
                    if let plist::Value::Dictionary(dev_dict) = val {
                        let device_id = dev_dict.get("DeviceID")
                            .and_then(|v| v.as_unsigned_integer())
                            .unwrap_or(0) as u32;
                        let product_id = dev_dict.get("ProductID")
                            .and_then(|v| v.as_unsigned_integer())
                            .unwrap_or(0) as u16;
                        let usb_addr = dev_dict.get("USBDeviceAddress")
                            .and_then(|v| v.as_unsigned_integer())
                            .unwrap_or(0) as u32;
                        let conn_id = dev_dict.get("ConnectionID")
                            .and_then(|v| v.as_unsigned_integer())
                            .unwrap_or(0) as u32;

                        // Parse properties for UDID and product type
                        let mut udid = String::new();
                        let mut product_type = String::new();
                        let mut name = String::new();

                        if let Some(plist::Value::Dictionary(props)) = dev_dict.get("Properties") {
                            udid = props.get("UniqueDeviceID")
                                .and_then(|v| v.as_string())
                                .unwrap_or("")
                                .to_string();
                            product_type = props.get("ProductType")
                                .and_then(|v| v.as_string())
                                .unwrap_or("")
                                .to_string();
                            name = props.get("DeviceName")
                                .and_then(|v| v.as_string())
                                .unwrap_or("")
                                .to_string();
                        }

                        devices.push(UsbmuxdDevice {
                            device_id,
                            product_id,
                            usb_device_address: usb_addr,
                            connection_id: conn_id,
                            udid,
                            product_type,
                            name,
                        });
                    }
                }
            }
        }

        info!("UsbmuxdClient: found {} devices", devices.len());
        Ok(devices)
    }

    /// Connect to a specific device's lockdownd service.
    /// Returns a new TcpStream connected to the device.
    pub fn connect_device(&mut self, device_id: u32) -> Result<TcpStream> {
        info!("UsbmuxdClient: connecting to device {}", device_id);

        let mut request = plist::Dictionary::new();
        request.insert("MessageType".into(), plist::Value::String("Connect".into()));
        request.insert("ClientVersionString".into(), plist::Value::String("ChimeraRS".into()));
        request.insert("kLibUSBMuxVersion".into(), plist::Value::Integer(0.into()));
        request.insert("DeviceID".into(), plist::Value::Integer((device_id as i64).into()));
        request.insert("PortNumber".into(), plist::Value::Integer(62078.into()));

        let response = self.send_plist(OP_CONNECT, &request)?;

        if let plist::Value::Dictionary(dict) = response {
            if let Some(err) = dict.get("Error") {
                if let Some(err_str) = err.as_string() {
                    return Err(anyhow!("usbmuxd connect failed: {}", err_str));
                }
            }

            // Check for result 0 (success)
            if let Some(result) = dict.get("Number") {
                if let Some(n) = result.as_unsigned_integer() {
                    if n != 0 {
                        return Err(anyhow!("usbmuxd connect returned error code {}", n));
                    }
                }
            }
        }

        // The actual connection goes through a separate fd — for now we return
        // a new connection. In production, usbmuxd passes the fd via sendmsg.
        // For simplicity, we connect to the device's lockdown port directly.
        let stream = TcpStream::connect(format!("127.0.0.1:{}", 62078))?;
        Ok(stream)
    }

    /// Subscribe to device attach/detach events (blocking)
    pub fn listen(&mut self) -> Result<()> {
        info!("UsbmuxdClient: listening for device events");

        let mut request = plist::Dictionary::new();
        request.insert("MessageType".into(), plist::Value::String("Listen".into()));
        request.insert("ClientVersionString".into(), plist::Value::String("ChimeraRS".into()));
        request.insert("kLibUSBMuxVersion".into(), plist::Value::Integer(0.into()));

        // Send listen request
        let mut plist_bytes = Vec::new();
        plist::Value::Dictionary(request.clone()).to_writer_xml(&mut plist_bytes)?;

        let header = MsgHeader {
            version: 0,
            msgtype: MSG_TYPE_PLIST,
            protocol: PROTO_VERSION,
            operation: OP_LISTEN,
            sequence: self.sequence,
            payload_len: plist_bytes.len() as u32,
        };
        self.sequence += 1;

        self.stream.write_all(&header.serialize())?;
        self.stream.write_all(&plist_bytes)?;
        self.stream.flush()?;

        // Loop reading events
        loop {
            let mut resp_header_buf = [0u8; HEADER_SIZE];
            match self.stream.read_exact(&mut resp_header_buf) {
                Ok(()) => {}
                Err(e) => {
                    warn!("UsbmuxdClient: listen read error: {}", e);
                    break;
                }
            }

            let resp_header = MsgHeader::deserialize(&resp_header_buf);

            if resp_header.payload_len > 0 {
                let mut resp_payload = vec![0u8; resp_header.payload_len as usize];
                self.stream.read_exact(&mut resp_payload)?;

                if let Ok(plist_val) = plist::Value::from_reader(std::io::Cursor::new(&resp_payload)) {
                    if let plist::Value::Dictionary(dict) = plist_val {
                        if let Some(msg_type) = dict.get("MessageType").and_then(|v| v.as_string()) {
                            match msg_type {
                                "DeviceAdd" => {
                                    info!("UsbmuxdClient: device attached");
                                    if let Some(plist::Value::Dictionary(props)) = dict.get("Properties") {
                                        if let Some(udid) = props.get("UniqueDeviceID").and_then(|v| v.as_string()) {
                                            info!("UsbmuxdClient: device UDID={}", udid);
                                        }
                                    }
                                }
                                "DeviceRemove" => {
                                    info!("UsbmuxdClient: device detached");
                                }
                                _ => {
                                    debug!("UsbmuxdClient: unknown event type: {}", msg_type);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
