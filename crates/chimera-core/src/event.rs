// chimera-core/src/event.rs
// Events flowing from background threads to the GUI

use crossbeam_channel::{Receiver, Sender, unbounded};
use crate::device::DeviceInfo;
use crate::progress::Progress;
use crate::diagnostics::FullDiagnostics;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum ChimeraEvent {
    // Device lifecycle
    DeviceConnected(DeviceInfo),
    DeviceDisconnected(String),          // device_id
    DeviceInfoUpdated(DeviceInfo),

    // Operation feedback
    OperationProgress(String, Progress),  // (device_id, progress)
    OperationSuccess(String, String),     // (device_id, message)
    OperationFailed(String, String),      // (device_id, error_message)

    // Firmware downloads
    FirmwareDownloadProgress { url: String, percent: f32 },
    FirmwareDownloadComplete { path: String },

    // Apple device info payload
    DeviceInfoPayload { device_id: String, payload_json: String },

    // iCloud activation lock status
    ActivationStatus {
        device_id: String,
        status: String,
        account_hint: Option<String>,
        is_supervised: Option<bool>,
        mdm_org: Option<String>,
    },

    // Network lock (carrier lock) status
    NetworkLockStatus { device_id: String, locked: bool },

    // Carrier unlock guide/instructions
    UnlockGuideResult { device_id: String, instructions: String },

    // NCK unlock code result
    NckResult { imei: String, nck: String },

    // AU carrier unlock guide result
    AuGuideResult { imei: String, instructions: String },

    // Diagnostics result
    DiagnosticsReady(String, Box<FullDiagnostics>),  // (device_id, diag)

    // Generic log message
    Log(LogLevel, String),

    // Generic error
    Error { source: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Success,
}

pub struct EventBus {
    sender: Sender<ChimeraEvent>,
    receiver: Receiver<ChimeraEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self { sender, receiver }
    }

    pub fn sender(&self) -> Sender<ChimeraEvent> {
        self.sender.clone()
    }

    pub fn receiver(&self) -> &Receiver<ChimeraEvent> {
        &self.receiver
    }

    pub fn send(&self, event: ChimeraEvent) {
        let _ = self.sender.send(event);
    }

    pub fn try_recv(&self) -> Option<ChimeraEvent> {
        self.receiver.try_recv().ok()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
