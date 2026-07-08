// chimera-apple/src/recovery.rs
// Recovery mode and DFU mode management for Apple devices.
// Handles entering/exiting recovery mode, sending iBEC/iBSS, and
// communicating over the USB bulk interface used by irecovery.

use anyhow::{anyhow, Result};
use log::{info, debug, warn};
use serde::{Deserialize, Serialize};

/// Recovery-mode USB interface constants
pub const APPLE_RECOVERY_VID: u16 = 0x05AC;
pub const APPLE_RECOVERY_PID: u16 = 0x1281;
pub const APPLE_DFU_PID: u16 = 0x1227;

/// iRecovery USB interface/endpoint indices (same across all Apple devices)
pub const RECOVERY_INTERFACE: u8 = 0;
pub const RECOVERY_ENDPOINT_IN: u8 = 0x81;
pub const RECOVERY_ENDPOINT_OUT: u8 = 0x01;

/// Control request type for sending commands to iBoot
pub const REQUEST_TYPE: u8 = 0x40; // vendor | device
pub const REQUEST_FILE: u8 = 0x00;
pub const REQUEST_STATUS: u8 = 0x03;
pub const REQUEST_COMMAND: u8 = 0x40;

/// Recovery mode commands recognised by iBoot
pub mod iboot_cmd {
    pub const REBOOT: &str = "reboot";
    pub const POWEROFF: &str = "poweroff";
    pub const SET_AUTO_BOOT: &str = "setenv auto-boot true";
    pub const UNSET_AUTO_BOOT: &str = "setenv auto-boot false";
    pub const SAVE_ENV: &str = "saveenv";
    pub const FSBOOT: &str = "fsboot";
    pub const RESET: &str = "reset";
    pub const BOOT: &str = "go";
    pub const RAMDISK: &str = "ramdisk";
}

/// Current recovery sub-mode
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecoverySubMode {
    /// iBSS – first-stage bootloader (no iBEC loaded yet)
    Ibss,
    /// iBEC – second-stage, accepts most iBoot commands
    Ibec,
    /// Waiting for an IPSW or ramdisk image
    Restore,
    /// Generic recovery shell
    Shell,
    Unknown,
}

/// Handle for interacting with a device in Recovery or DFU mode via libusb/rusb.
pub struct RecoveryClient {
    pub udid: String,
    pub pid: u16,
    pub is_dfu: bool,
    connected: bool,
}

impl RecoveryClient {
    pub fn new(udid: &str, pid: u16) -> Self {
        Self {
            udid: udid.to_owned(),
            pid,
            is_dfu: pid == APPLE_DFU_PID,
            connected: false,
        }
    }

    /// Open USB connection to the device in recovery / DFU mode.
    pub fn open(&mut self) -> Result<()> {
        debug!("RecoveryClient::open pid=0x{:04X}", self.pid);
        // Real: rusb::open_device_with_vid_pid(APPLE_RECOVERY_VID, self.pid)
        //       claim_interface(RECOVERY_INTERFACE)
        self.connected = true;
        Ok(())
    }

    /// Send an iBoot text command (recovery mode only; ignored in DFU).
    pub fn send_command(&self, cmd: &str) -> Result<()> {
        if !self.connected {
            return Err(anyhow!("Not connected to recovery device"));
        }
        if self.is_dfu {
            return Err(anyhow!("DFU mode does not accept text commands"));
        }
        info!("RecoveryClient: sending command '{}'", cmd);
        // Real: write_bulk(RECOVERY_ENDPOINT_OUT, cmd.as_bytes())
        //       followed by read_bulk to get prompt
        Ok(())
    }

    /// Upload a binary image (iBSS/iBEC/ramdisk/kernel) via USB bulk OUT.
    pub fn upload_file(&self, data: &[u8], progress_cb: impl Fn(f32)) -> Result<()> {
        if !self.connected {
            return Err(anyhow!("Not connected"));
        }
        info!("RecoveryClient: uploading {} bytes", data.len());
        // Real implementation splits data into 0x8000-byte chunks and writes them
        // via write_bulk(RECOVERY_ENDPOINT_OUT, chunk) in a loop, calling progress_cb.
        let total = data.len() as f32;
        let mut uploaded = 0usize;
        // Simulate chunk-based upload
        let chunk_size = 0x8000;
        while uploaded < data.len() {
            let end = (uploaded + chunk_size).min(data.len());
            uploaded = end;
            progress_cb(uploaded as f32 / total);
        }
        Ok(())
    }

    /// Put device into DFU mode by sending a specific USB control request sequence.
    /// Works when device is already in recovery mode (iBEC/iBSS shell).
    pub fn enter_dfu(&self) -> Result<()> {
        if !self.connected {
            return Err(anyhow!("Not connected"));
        }
        info!("RecoveryClient: entering DFU mode");
        // Real: send control request (REQUEST_TYPE, 0xC0, 0, 0) then reboot
        Ok(())
    }

    /// Exit recovery mode back to normal iOS boot.
    pub fn exit_recovery(&self) -> Result<()> {
        self.send_command(iboot_cmd::SET_AUTO_BOOT)?;
        self.send_command(iboot_cmd::SAVE_ENV)?;
        self.send_command(iboot_cmd::REBOOT)?;
        info!("RecoveryClient: exiting recovery mode");
        Ok(())
    }

    /// Reboot the device (works in both recovery and DFU – DFU ignores, triggers USB reset).
    pub fn reboot(&self) -> Result<()> {
        if self.is_dfu {
            // DFU: send a USB reset to force hardware reboot
            warn!("DFU reboot: issuing USB reset");
            return Ok(());
        }
        self.send_command(iboot_cmd::REBOOT)
    }

    pub fn close(&mut self) {
        self.connected = false;
    }
}

/// Utility: detect whether a rusb device is in DFU or Recovery mode by VID/PID.
pub fn detect_recovery_mode(vid: u16, pid: u16) -> Option<RecoverySubMode> {
    if vid != APPLE_RECOVERY_VID {
        return None;
    }
    match pid {
        APPLE_DFU_PID => Some(RecoverySubMode::Ibss),
        APPLE_RECOVERY_PID => Some(RecoverySubMode::Shell),
        _ => None,
    }
}
