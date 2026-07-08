//! Wrap `idevicerestore` — flash an IPSW to a device.

use std::path::Path;
use std::time::Duration;
use std::process::{Command, Stdio};
use crate::tool::{resolve, ImobileTool, ImobileError};

/// Options that map onto `idevicerestore` flags.
#[derive(Debug, Clone, Default)]
pub struct RestoreOptions {
    pub erase: bool,
    pub update: bool,
    /// `--no-action` — exit after parsing IPSW; useful for dry-run validation.
    pub dry_run: bool,
    /// `-d/--debug` — verbose libirecovery output.
    pub debug: bool,
}

/// Flash `ipsw_path` to `udid` (or first device if None). Streams stdout
/// through `progress_cb` line-by-line so the GUI can render real-time
/// progress instead of waiting for full completion.
pub fn restore_ipsw<F>(
    udid: Option<&str>,
    ipsw_path: &Path,
    opts: RestoreOptions,
    mut progress_cb: F,
) -> Result<(), ImobileError>
where
    F: FnMut(&str) + Send + 'static,
{
    let binary = resolve(ImobileTool::Idevicerestore)?;

    let mut cmd = Command::new(&binary);
    if opts.erase   { cmd.arg("--erase"); }
    if opts.update  { cmd.arg("--update"); }
    if opts.dry_run { cmd.arg("--no-action"); }
    if opts.debug   { cmd.arg("--debug"); }
    if let Some(u) = udid { cmd.args(["-u", u]); }
    cmd.arg(ipsw_path);

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ImobileError::Io { tool: "idevicerestore".into(), source: e })?;

    let stdout = child.stdout.take().expect("stdout piped");
    let reader_thread = std::thread::spawn(move || {
        use std::io::{BufRead, BufReader};
        for line in BufReader::new(stdout).lines().map_while(|l| l.ok()) {
            progress_cb(&line);
        }
    });

    let output = child.wait_with_output()
        .map_err(|e| ImobileError::Io { tool: "idevicerestore".into(), source: e })?;
    let _ = reader_thread.join();

    if !output.status.success() {
        return Err(ImobileError::NonZeroExit {
            tool:   "idevicerestore".into(),
            code:   output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(())
}

/// Dry-run / validation invocation — parses the IPSW without touching the
/// device. Used by the "Validate IPSW" flow as a stronger check than the
/// pure-Rust zip-only validator.
pub fn validate_ipsw(ipsw_path: &Path) -> Result<(), ImobileError> {
    let _ = std::env::var("CHIMERA_IDEVICERESTORE"); // honour env override
    let _ = Duration::from_secs(30);
    restore_ipsw(None, ipsw_path,
        RestoreOptions { dry_run: true, ..Default::default() },
        |_| {})
}
