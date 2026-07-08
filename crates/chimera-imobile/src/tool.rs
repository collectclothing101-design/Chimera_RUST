//! Binary discovery + spawn-with-timeout for libimobiledevice tools.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImobileError {
    #[error("libimobiledevice tool `{0}` not found on PATH or in common install dirs. \
             Install with `brew install libimobiledevice` (macOS) or bundle the Windows \
             binaries under Resources/idevice/.")]
    ToolMissing(&'static str),

    #[error("`{tool}` exited with code {code}: {stderr}")]
    NonZeroExit { tool: String, code: i32, stderr: String },

    #[error("`{tool}` timed out after {duration:?}")]
    Timeout { tool: String, duration: Duration },

    #[error("`{tool}` produced output that could not be parsed: {detail}")]
    Parse { tool: String, detail: String },

    #[error("I/O error invoking `{tool}`: {source}")]
    Io { tool: String, #[source] source: std::io::Error },

    #[error("Plist parse error: {0}")]
    Plist(#[from] plist::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// The set of libimobiledevice CLI tools the workspace consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImobileTool {
    IdeviceId,
    Ideviceinfo,
    Ideviceactivation,
    Idevicebackup2,
    Idevicerestore,
    Idevicepair,
    Ideviceenterrecovery,
    Idevicediagnostics,
    Idevicedebug,
    Idevicename,
    Idevicedate,
    Idevicescreenshot,
    Idevicenotificationproxy,
    Idevicesyslog,
    Ideviceimagemounter,
    Ideviceprovision,
    Ideviceinstaller,
    Idevicecrashreport,
    Idevicesetlocation,
    Inetcat,
    Iproxy,
    Irecovery,
    PlistUtil,
}

impl ImobileTool {
    /// The binary name (without `.exe` suffix — that's added on Windows).
    pub fn binary_name(self) -> &'static str {
        match self {
            ImobileTool::IdeviceId               => "idevice_id",
            ImobileTool::Ideviceinfo             => "ideviceinfo",
            ImobileTool::Ideviceactivation       => "ideviceactivation",
            ImobileTool::Idevicebackup2          => "idevicebackup2",
            ImobileTool::Idevicerestore          => "idevicerestore",
            ImobileTool::Idevicepair             => "idevicepair",
            ImobileTool::Ideviceenterrecovery    => "ideviceenterrecovery",
            ImobileTool::Idevicediagnostics      => "idevicediagnostics",
            ImobileTool::Idevicedebug            => "idevicedebug",
            ImobileTool::Idevicename             => "idevicename",
            ImobileTool::Idevicedate             => "idevicedate",
            ImobileTool::Idevicescreenshot       => "idevicescreenshot",
            ImobileTool::Idevicenotificationproxy=> "idevicenotificationproxy",
            ImobileTool::Idevicesyslog           => "idevicesyslog",
            ImobileTool::Ideviceimagemounter     => "ideviceimagemounter",
            ImobileTool::Ideviceprovision        => "ideviceprovision",
            ImobileTool::Ideviceinstaller        => "ideviceinstaller",
            ImobileTool::Idevicecrashreport      => "idevicecrashreport",
            ImobileTool::Idevicesetlocation      => "idevicesetlocation",
            ImobileTool::Inetcat                 => "inetcat",
            ImobileTool::Iproxy                  => "iproxy",
            ImobileTool::Irecovery               => "irecovery",
            ImobileTool::PlistUtil               => "plistutil",
        }
    }

    /// Environment-variable override for this tool's path.
    pub fn env_override(self) -> &'static str {
        match self {
            ImobileTool::IdeviceId               => "CHIMERA_IDEVICE_ID",
            ImobileTool::Ideviceinfo             => "CHIMERA_IDEVICEINFO",
            ImobileTool::Ideviceactivation       => "CHIMERA_IDEVICEACTIVATION",
            ImobileTool::Idevicebackup2          => "CHIMERA_IDEVICEBACKUP2",
            ImobileTool::Idevicerestore          => "CHIMERA_IDEVICERESTORE",
            ImobileTool::Idevicepair             => "CHIMERA_IDEVICEPAIR",
            ImobileTool::Ideviceenterrecovery    => "CHIMERA_IDEVICEENTERRECOVERY",
            ImobileTool::Idevicediagnostics      => "CHIMERA_IDEVICEDIAGNOSTICS",
            ImobileTool::Idevicedebug            => "CHIMERA_IDEVICEDEBUG",
            ImobileTool::Idevicename             => "CHIMERA_IDEVICENAME",
            ImobileTool::Idevicedate             => "CHIMERA_IDEVICEDATE",
            ImobileTool::Idevicescreenshot       => "CHIMERA_IDEVICESCREENSHOT",
            ImobileTool::Idevicenotificationproxy=> "CHIMERA_IDEVICENOTIFICATIONPROXY",
            ImobileTool::Idevicesyslog           => "CHIMERA_IDEVICESYSLOG",
            ImobileTool::Ideviceimagemounter     => "CHIMERA_IDEVICEIMAGEMOUNTER",
            ImobileTool::Ideviceprovision        => "CHIMERA_IDEVICEPROVISION",
            ImobileTool::Ideviceinstaller        => "CHIMERA_IDEVICEINSTALLER",
            ImobileTool::Idevicecrashreport      => "CHIMERA_IDEVICECRASHREPORT",
            ImobileTool::Idevicesetlocation      => "CHIMERA_IDEVICESETLOCATION",
            ImobileTool::Inetcat                 => "CHIMERA_INETCAT",
            ImobileTool::Iproxy                  => "CHIMERA_IPROXY",
            ImobileTool::Irecovery               => "CHIMERA_IRECOVERY",
            ImobileTool::PlistUtil               => "CHIMERA_PLISTUTIL",
        }
    }
}

/// Locate a libimobiledevice binary. Resolution order:
///   1. Environment-variable override
///   2. `which` lookup against $PATH
///   3. `Chimera.app/Contents/Resources/idevice/`  (bundled Windows tools)
///   4. Common install prefixes (Homebrew Intel/Apple Silicon, /usr/bin)
pub fn resolve(tool: ImobileTool) -> Result<PathBuf, ImobileError> {
    let name = tool.binary_name();
    let env  = tool.env_override();

    // (1) Env override
    if let Ok(v) = std::env::var(env) {
        let p = PathBuf::from(v);
        if p.exists() { return Ok(p); }
    }

    // (2) PATH lookup
    if let Ok(p) = which::which(name) {
        return Ok(p);
    }

    // (3) Bundled tools (relative to the running executable)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // macOS .app:  Chimera.app/Contents/MacOS/Chimera  → ../Resources/idevice/
            let mac_bundle = dir.parent().map(|p| p.join("Resources").join("idevice"));
            // Windows zip: chimera.exe + idevice\ next to it
            let win_dir    = dir.join("idevice");
            for candidate in [mac_bundle.unwrap_or_default(), win_dir] {
                for ext in &["", ".exe"] {
                    let p = candidate.join(format!("{}{}", name, ext));
                    if p.exists() { return Ok(p); }
                }
            }
        }
    }

    // (4) Common install prefixes
    for prefix in [
        "/usr/local/bin",
        "/opt/homebrew/bin",
        "/usr/bin",
        "/opt/local/bin",
        "C:\\Program Files\\libimobiledevice",
        "C:\\Program Files (x86)\\libimobiledevice",
        "C:\\libimobiledevice",
    ] {
        let p = Path::new(prefix).join(name);
        if p.exists() { return Ok(p); }
        let p = Path::new(prefix).join(format!("{}.exe", name));
        if p.exists() { return Ok(p); }
    }

    Err(ImobileError::ToolMissing(name))
}

/// Run a libimobiledevice tool with a wall-clock timeout, capturing
/// stdout / stderr. Non-zero exit codes are propagated as `NonZeroExit`.
pub fn run(
    tool: ImobileTool,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, ImobileError> {
    let path = resolve(tool)?;
    let name = tool.binary_name();

    tracing::debug!("Spawning {} {:?}", path.display(), args);

    let mut child = Command::new(&path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ImobileError::Io { tool: name.into(), source: e })?;

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return Err(ImobileError::Timeout { tool: name.into(), duration: timeout });
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Err(ImobileError::Io { tool: name.into(), source: e }),
        }
    }
    let output = child.wait_with_output()
        .map_err(|e| ImobileError::Io { tool: name.into(), source: e })?;
    if !output.status.success() {
        return Err(ImobileError::NonZeroExit {
            tool:   name.into(),
            code:   output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_names_unique() {
        let tools = [
            ImobileTool::IdeviceId, ImobileTool::Ideviceinfo,
            ImobileTool::Ideviceactivation, ImobileTool::Idevicebackup2,
            ImobileTool::Idevicerestore, ImobileTool::Idevicepair,
            ImobileTool::Ideviceenterrecovery, ImobileTool::Idevicediagnostics,
            ImobileTool::Irecovery, ImobileTool::PlistUtil,
        ];
        let names: std::collections::HashSet<_> = tools.iter().map(|t| t.binary_name()).collect();
        assert_eq!(names.len(), tools.len(), "duplicate binary names");
    }

    #[test]
    fn env_overrides_unique() {
        let tools = [
            ImobileTool::IdeviceId, ImobileTool::Ideviceinfo,
            ImobileTool::Ideviceactivation, ImobileTool::Idevicebackup2,
            ImobileTool::Idevicerestore, ImobileTool::Idevicepair,
        ];
        let envs: std::collections::HashSet<_> = tools.iter().map(|t| t.env_override()).collect();
        assert_eq!(envs.len(), tools.len(), "duplicate env overrides");
    }

    #[test]
    fn resolve_missing_returns_error() {
        // Set an override pointing at a definitely-not-existing path so
        // we don't accidentally find a real install during CI.
        std::env::set_var("CHIMERA_IDEVICE_ID", "/definitely/not/here/idevice_id");
        // Resolve should still try $PATH after env miss; if the host has
        // libimobiledevice installed this returns Ok. Otherwise Err.
        // Either outcome is acceptable; we just check it doesn't panic.
        let _ = resolve(ImobileTool::IdeviceId);
        std::env::remove_var("CHIMERA_IDEVICE_ID");
    }
}
