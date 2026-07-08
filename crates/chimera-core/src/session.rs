// chimera-core/src/session.rs
// Session manager: persists device history, operation logs, and user preferences

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::fs;
use crate::device::DeviceInfo;
use crate::error::{ChimeraError, Result};
use chrono::{DateTime, Local};

/// A single recorded operation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    pub id: u64,
    pub timestamp: DateTime<Local>,
    pub device_model: String,
    pub device_serial: Option<String>,
    pub operation: String,
    pub status: RecordStatus,
    pub details: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecordStatus {
    Success,
    Failed,
    Cancelled,
    InProgress,
}

impl std::fmt::Display for RecordStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success   => write!(f, "✅ Success"),
            Self::Failed    => write!(f, "❌ Failed"),
            Self::Cancelled => write!(f, "⚠️ Cancelled"),
            Self::InProgress=> write!(f, "⏳ In Progress"),
        }
    }
}

/// A device that has been seen before (history entry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHistoryEntry {
    pub serial: String,
    pub model: String,
    pub brand: String,
    pub last_seen: DateTime<Local>,
    pub operation_count: u32,
    pub notes: String,
}

/// Full session data (persisted to disk)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub version: u32,
    pub operation_history: VecDeque<OperationRecord>,
    pub device_history: Vec<DeviceHistoryEntry>,
    pub next_op_id: u64,
}

impl Default for SessionData {
    fn default() -> Self {
        Self {
            version: 1,
            operation_history: VecDeque::with_capacity(500),
            device_history: Vec::new(),
            next_op_id: 1,
        }
    }
}

pub struct SessionManager {
    data: SessionData,
    save_path: PathBuf,
    max_history: usize,
}

impl SessionManager {
    pub fn new(save_dir: &Path) -> Self {
        let save_path = save_dir.join("chimera_session.json");
        let data = Self::load_from_file(&save_path).unwrap_or_default();
        Self { data, save_path, max_history: 500 }
    }

    fn load_from_file(path: &Path) -> Option<SessionData> {
        let bytes = fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn save(&self) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&self.data)
            .map_err(|e| ChimeraError::Io(e.to_string()))?;
        if let Some(parent) = self.save_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.save_path, bytes)?;
        Ok(())
    }

    /// Record the start of an operation; returns the assigned ID
    pub fn begin_operation(&mut self, device: &DeviceInfo, operation: &str) -> u64 {
        let id = self.data.next_op_id;
        self.data.next_op_id += 1;
        let record = OperationRecord {
            id,
            timestamp: Local::now(),
            device_model: device.model.clone(),
            device_serial: device.serial.clone(),
            operation: operation.to_string(),
            status: RecordStatus::InProgress,
            details: None,
            duration_ms: None,
        };
        self.data.operation_history.push_back(record);
        if self.data.operation_history.len() > self.max_history {
            self.data.operation_history.pop_front();
        }
        id
    }

    /// Complete an operation record
    pub fn finish_operation(&mut self, id: u64, status: RecordStatus, details: Option<String>, duration_ms: u64) {
        if let Some(r) = self.data.operation_history.iter_mut().rfind(|r| r.id == id) {
            r.status = status;
            r.details = details;
            r.duration_ms = Some(duration_ms);
        }
    }

    /// Record that a device was seen
    pub fn record_device_seen(&mut self, device: &DeviceInfo) {
        let serial = match &device.serial { Some(s) => s.clone(), None => return };
        if let Some(entry) = self.data.device_history.iter_mut().find(|e| e.serial == serial) {
            entry.last_seen = Local::now();
            entry.operation_count += 1;
        } else {
            self.data.device_history.push(DeviceHistoryEntry {
                serial,
                model: device.model.clone(),
                brand: format!("{}", device.brand),
                last_seen: Local::now(),
                operation_count: 1,
                notes: String::new(),
            });
        }
    }

    pub fn history(&self) -> &VecDeque<OperationRecord> {
        &self.data.operation_history
    }

    pub fn device_history(&self) -> &[DeviceHistoryEntry] {
        &self.data.device_history
    }

    pub fn clear_history(&mut self) {
        self.data.operation_history.clear();
    }

    /// Export the full log as CSV text
    pub fn export_csv(&self) -> String {
        let mut out = String::from("ID,Timestamp,Device,Serial,Operation,Status,Details,Duration(ms)\n");
        for r in &self.data.operation_history {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                r.id,
                r.timestamp.format("%Y-%m-%d %H:%M:%S"),
                r.device_model,
                r.device_serial.as_deref().unwrap_or(""),
                r.operation,
                format!("{}", r.status).replace("✅ ", "").replace("❌ ", "").replace("⚠️ ", "").replace("⏳ ", ""),
                r.details.as_deref().unwrap_or("").replace(',', ";"),
                r.duration_ms.map(|d| d.to_string()).unwrap_or_default(),
            ));
        }
        out
    }
}
