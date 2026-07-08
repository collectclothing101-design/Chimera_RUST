// chimera-api/src/upload_api.rs
// Replacement for upload.chimeratool.com — local file handling.
// Original: multipart POST to upload EFS backups, certificates, logs to user's cloud account.
// ChimeraRS: all files stay local. No cloud upload. User chooses destination.

use anyhow::Result;
use std::path::{Path, PathBuf};
use log::info;
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Types of files that ChimeraTool uploaded remotely
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UploadFileType {
    EfsBackup,
    CertificateBackup,
    TaPartitionBackup,   // Sony
    DiagnosticLog,
    OperationLog,
    FirmwareFile,
    CrashReport,
}

/// Local save result (replaces upload response)
#[derive(Debug, Serialize, Deserialize)]
pub struct LocalSaveResult {
    pub file_type: UploadFileType,
    pub saved_path: String,
    pub file_size: u64,
    pub checksum_sha256: String,
    pub saved_at: String,
}

/// Save a backup file locally (replaces cloud upload)
pub fn save_backup_locally(
    data: &[u8],
    file_type: UploadFileType,
    device_model: &str,
    base_dir: &Path,
) -> Result<LocalSaveResult> {
    use sha2::{Sha256, Digest};
    use std::io::Write;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let subdir = match &file_type {
        UploadFileType::EfsBackup           => "efs",
        UploadFileType::CertificateBackup   => "cert",
        UploadFileType::TaPartitionBackup   => "ta",
        UploadFileType::DiagnosticLog       => "logs",
        UploadFileType::OperationLog        => "logs",
        UploadFileType::FirmwareFile        => "firmware",
        UploadFileType::CrashReport         => "crashes",
    };

    let dir = base_dir.join(subdir).join(device_model);
    std::fs::create_dir_all(&dir)?;

    let ext = match &file_type {
        UploadFileType::EfsBackup | UploadFileType::CertificateBackup
        | UploadFileType::TaPartitionBackup => "bin",
        UploadFileType::DiagnosticLog
        | UploadFileType::OperationLog      => "log",
        UploadFileType::FirmwareFile        => "bin",
        UploadFileType::CrashReport         => "txt",
    };

    let filename = format!("{}_{}.{}", device_model, timestamp, ext);
    let path = dir.join(&filename);
    std::fs::File::create(&path)?.write_all(data)?;

    let hash = hex::encode(Sha256::digest(data));
    let size = data.len() as u64;

    info!("Saved backup locally: {} ({} bytes, sha256: {})", path.display(), size, &hash[..16]);

    Ok(LocalSaveResult {
        file_type,
        saved_path: path.to_string_lossy().to_string(),
        file_size: size,
        checksum_sha256: hash,
        saved_at: timestamp,
    })
}

/// Export operation log to local file (replaces upload.chimeratool.com /v1/diagnostic)
pub fn export_log_locally(
    log_entries: &[(String, String, String)], // (timestamp, level, message)
    device_id: &str,
    base_dir: &Path,
) -> Result<PathBuf> {
    use std::io::Write;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let dir = base_dir.join("logs");
    std::fs::create_dir_all(&dir)?;

    let filename = format!("chimera_log_{}_{}.txt", device_id, timestamp);
    let path = dir.join(&filename);
    let mut f = std::fs::File::create(&path)?;

    writeln!(f, "ChimeraRS Operation Log")?;
    writeln!(f, "Generated: {}", chrono::Utc::now().to_rfc3339())?;
    writeln!(f, "Device: {}", device_id)?;
    writeln!(f, "─────────────────────────────────────────")?;
    for (ts, level, msg) in log_entries {
        writeln!(f, "[{}] [{}] {}", ts, level, msg)?;
    }

    info!("Log exported to: {}", path.display());
    Ok(path)
}
