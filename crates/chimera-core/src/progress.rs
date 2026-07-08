// chimera-core/src/progress.rs
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};

/// Progress update from an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub operation: String,
    pub step: String,
    pub percent: f32,   // 0.0 .. 100.0
    pub bytes_done: Option<u64>,
    pub bytes_total: Option<u64>,
    pub is_complete: bool,
    pub is_error: bool,
    pub error_message: Option<String>,
    pub log_line: Option<String>,
}

impl Progress {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            step: String::new(),
            percent: 0.0,
            bytes_done: None,
            bytes_total: None,
            is_complete: false,
            is_error: false,
            error_message: None,
            log_line: None,
        }
    }

    pub fn step(mut self, step: impl Into<String>) -> Self {
        self.step = step.into();
        self
    }

    pub fn percent(mut self, pct: f32) -> Self {
        self.percent = pct.clamp(0.0, 100.0);
        self
    }

    pub fn bytes(mut self, done: u64, total: u64) -> Self {
        self.bytes_done = Some(done);
        self.bytes_total = Some(total);
        if total > 0 {
            self.percent = (done as f32 / total as f32 * 100.0).clamp(0.0, 100.0);
        }
        self
    }

    pub fn complete(mut self) -> Self {
        self.is_complete = true;
        self.percent = 100.0;
        self
    }

    pub fn error(mut self, msg: impl Into<String>) -> Self {
        self.is_error = true;
        self.error_message = Some(msg.into());
        self
    }

    pub fn log(mut self, line: impl Into<String>) -> Self {
        self.log_line = Some(line.into());
        self
    }
}

pub type ProgressSender = Sender<Progress>;
pub type ProgressReceiver = Receiver<Progress>;

pub fn progress_channel() -> (ProgressSender, ProgressReceiver) {
    crossbeam_channel::unbounded()
}

/// Convenience wrapper for sending progress updates with a simple (percent, message) API.
/// Wraps a ProgressSender and exposes a .report(f32, &str) method.
pub struct ProgressReporter {
    tx: Option<ProgressSender>,
    name: String,
}

impl ProgressReporter {
    pub fn new(name: impl Into<String>) -> Self {
        Self { tx: None, name: name.into() }
    }

    pub fn with_sender(name: impl Into<String>, tx: ProgressSender) -> Self {
        Self { tx: Some(tx), name: name.into() }
    }

    /// Send a progress update: percent is 0.0–1.0, message is the step description.
    pub fn report(&self, percent: f32, message: &str) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(
                Progress::new(self.name.clone())
                    .percent(percent * 100.0)
                    .step(message)
            );
        }
    }

    /// Send a completion signal.
    pub fn complete(&self, message: &str) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(
                Progress::new(self.name.clone())
                    .percent(100.0)
                    .step(message)
                    .complete()
            );
        }
    }

    /// Send an error signal.
    pub fn error(&self, message: &str) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(
                Progress::new(self.name.clone())
                    .error(message)
            );
        }
    }
}