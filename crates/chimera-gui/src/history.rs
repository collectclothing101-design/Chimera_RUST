// crates/chimera-gui/src/history.rs
// Operation history — persisted to ~/Library/Application Support/ChimeraRS/history.json
#![allow(dead_code)]

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_mut)]
use serde::{Deserialize, Serialize};
use chimera_core::event::LogLevel;
use eframe::egui::Color32;
use crate::theme::C;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HistoryResult {
    Success,
    Failed,
    Partial,
}

impl HistoryResult {
    pub fn color(&self) -> Color32 {
        match self {
            HistoryResult::Success => C::G,
            HistoryResult::Failed  => C::R,
            HistoryResult::Partial => C::A,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            HistoryResult::Success => "SUCCESS",
            HistoryResult::Failed  => "FAILED",
            HistoryResult::Partial => "PARTIAL",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp:    String,
    pub operation:    String,
    pub device_model: String,
    pub device_brand: String,
    pub serial:       String,
    pub result:       HistoryResult,
    pub notes:        String,
}

impl HistoryEntry {
    pub fn new(
        operation: &str,
        device_model: &str,
        device_brand: &str,
        serial: &str,
        result: HistoryResult,
        notes: &str,
    ) -> Self {
        Self {
            timestamp:    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            operation:    operation.to_string(),
            device_model: device_model.to_string(),
            device_brand: device_brand.to_string(),
            serial:       serial.to_string(),
            result,
            notes:        notes.to_string(),
        }
    }

    pub fn matches_filter(&self, q: &str) -> bool {
        if q.is_empty() { return true; }
        let q = q.to_lowercase();
        self.operation.to_lowercase().contains(&q)
            || self.device_model.to_lowercase().contains(&q)
            || self.device_brand.to_lowercase().contains(&q)
            || self.serial.to_lowercase().contains(&q)
            || self.notes.to_lowercase().contains(&q)
    }

    pub fn from_event(
        operation: &str,
        device_model: &str,
        device_brand: &str,
        serial: &str,
        level: &LogLevel,
        msg: &str,
    ) -> Self {
        let result = match level {
            LogLevel::Success => HistoryResult::Success,
            LogLevel::Error   => HistoryResult::Failed,
            _                 => HistoryResult::Partial,
        };
        Self::new(operation, device_model, device_brand, serial, result, msg)
    }
}
