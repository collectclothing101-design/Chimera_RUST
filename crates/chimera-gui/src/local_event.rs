// crates/chimera-gui/src/local_event.rs
// GUI-local event types that supplement chimera_core::ChimeraEvent
// These are sent worker → AppState via a dedicated crossbeam channel.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

/// IPSW firmware entry from ipsw.me API
#[derive(Debug, Clone)]
pub struct IpswEntry {
    pub identifier:  String,   // iPhone14,2
    pub version:     String,   // 17.4.1
    pub build_id:    String,   // 21E236
    pub url:         String,
    pub filesize:    u64,
    pub sha1sum:     String,
    pub signed:      bool,
}

/// Active download task
#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub id:         String,
    pub name:       String,
    pub url:        String,
    pub dest_path:  String,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub status:     DownloadStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadStatus {
    Queued,
    Running,
    Verifying,
    Done,
    Failed(String),
    Cancelled,
}

impl DownloadTask {
    pub fn progress(&self) -> f32 {
        if self.bytes_total == 0 { return 0.0; }
        self.bytes_done as f32 / self.bytes_total as f32
    }
}

/// GUI-local events — worker sends these via `gui_event_tx`
#[derive(Debug)]
pub enum LocalEvent {
    // SSH
    SshConnected {
        output_rx: crossbeam_channel::Receiver<String>,
        input_tx:  crossbeam_channel::Sender<String>,
    },
    SshDisconnected,
    SshLine(String),

    // ADB QR
    QrReady { serial: String, svg: String },

    // Sony TA
    TaInfoResult { device_id: String, entries: Vec<(String, String)> },

    // AU Unlock
    AuDeviceRead { device_id: String, imei: String, carrier: String },

    // IPSW search
    IpswSearchResults { entries: Vec<IpswEntry> },

    // Download manager
    DownloadProgress { id: String, bytes_done: u64, bytes_total: u64 },
    DownloadComplete  { id: String, dest_path: String },
    DownloadFailed    { id: String, error: String },

    // API console
    ApiResponse { status: String, body: String, latency_ms: u64 },

    // Network tools
    TcpTestResult { host: String, port: u16, open: bool, latency_ms: u64 },
    DnsResult     { host: String, ips: Vec<String> },

    // Futurerestore
    FuturerestoreLine(String),
    FuturerestoreDone { success: bool, message: String },
}
