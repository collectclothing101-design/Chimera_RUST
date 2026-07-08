// crates/chimera-gui/src/persistence.rs
// Disk persistence: settings · history · SHSH catalogue · log file
#![allow(dead_code)]

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_mut)]
use std::path::PathBuf;
use crate::state::AppSettings;
use crate::history::HistoryEntry;

// ── Data directory ─────────────────────────────────────────────────────────

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("Library").join("Application Support")
        })
        .join("ChimeraRS")
}

pub fn settings_path() -> PathBuf { data_dir().join("settings.json") }
pub fn history_path()  -> PathBuf { data_dir().join("history.json") }
pub fn log_file_path() -> PathBuf { data_dir().join("chimera.log") }
pub fn shsh_dir()      -> PathBuf { data_dir().join("shsh") }

fn ensure_dirs() {
    let _ = std::fs::create_dir_all(data_dir());
    let _ = std::fs::create_dir_all(shsh_dir());
}

// ── Settings ───────────────────────────────────────────────────────────────

pub fn load_settings() -> AppSettings {
    let path = settings_path();
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(s) = serde_json::from_str::<AppSettings>(&data) {
                return s;
            }
        }
    }
    AppSettings::default()
}

pub fn save_settings(s: &AppSettings) -> std::io::Result<()> {
    ensure_dirs();
    let json = serde_json::to_string_pretty(s)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(settings_path(), json)
}

// ── History ────────────────────────────────────────────────────────────────

pub fn load_history() -> Vec<HistoryEntry> {
    let path = history_path();
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<Vec<HistoryEntry>>(&data) {
                return v;
            }
        }
    }
    Vec::new()
}

pub fn save_history(entries: &[HistoryEntry]) -> std::io::Result<()> {
    ensure_dirs();
    let json = serde_json::to_string_pretty(entries)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(history_path(), json)
}

pub fn append_history_entry(entry: &HistoryEntry) -> std::io::Result<()> {
    let mut entries = load_history();
    entries.push(entry.clone());
    if entries.len() > 10_000 {
        entries.drain(..entries.len() - 10_000);
    }
    save_history(&entries)
}

// ── Log file ───────────────────────────────────────────────────────────────

pub fn append_log_line(line: &str) -> std::io::Result<()> {
    use std::io::Write;
    ensure_dirs();
    let mut f = std::fs::OpenOptions::new()
        .create(true).append(true)
        .open(log_file_path())?;
    writeln!(f, "{}", line)
}

// ── SHSH blob catalogue ───────────────────────────────────────────────────

pub fn list_shsh_blobs() -> Vec<String> {
    let dir = shsh_dir();
    if !dir.exists() { return Vec::new(); }
    std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
              .filter(|e| {
                  let ext = e.path().extension()
                      .map(|x| x.to_string_lossy().to_string())
                      .unwrap_or_default();
                  ext == "shsh2" || ext == "shsh" || ext == "bord"
              })
              .map(|e| e.file_name().to_string_lossy().to_string())
              .collect()
        })
        .unwrap_or_default()
}
