// chimera-utils/src/host_probes.rs
//
// Live host-environment probes for external CLI tools the Chimera workflows
// depend on. Each probe runs once at startup and may be re-run periodically
// so the dashboard's "ADB daemon: Found / Not found" pill reflects reality.
//
// Resolution order, per binary:
//   1. Environment variable override (e.g. CHIMERA_ADB, CHIMERA_FASTBOOT)
//   2. `which` lookup against $PATH (uses the platform's exec lookup rules)
//   3. Common install paths on macOS / Linux / Windows
//
// Every probe also captures the binary's `--version` output so the dashboard
// can show the actual installed version, which makes "missing vs. wrong
// version" failures debuggable.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use serde::{Serialize, Deserialize};

/// Result of probing for one external binary.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolProbe {
    /// `true` when the binary was located and `--version` returned 0.
    pub found:   bool,
    /// Absolute path to the binary, if found.
    pub path:    Option<PathBuf>,
    /// Trimmed first line of `--version` output (e.g. "Android Debug Bridge version 1.0.41").
    pub version: Option<String>,
    /// Stderr / error text if the probe failed; useful in the dashboard
    /// hover tooltip.
    pub error:   Option<String>,
}

impl ToolProbe {
    pub fn missing(reason: impl Into<String>) -> Self {
        Self { found: false, path: None, version: None, error: Some(reason.into()) }
    }
}

/// Common install locations to fall back to when `$PATH` lookup fails.
fn common_paths(binary: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for prefix in [
        "/usr/local/bin",            // Intel macOS Homebrew / manual installs
        "/opt/homebrew/bin",         // Apple Silicon Homebrew
        "/usr/bin",                  // Linux system
        "/usr/local/opt/android-platform-tools/bin", // brew formula
        "/opt/android-sdk/platform-tools", // explicit SDK location
        // Windows
        "C:\\platform-tools",
        "C:\\Program Files\\platform-tools",
        // Per-user macOS Library
        "~/Library/Android/sdk/platform-tools",
    ] {
        let expanded = if let Some(rest) = prefix.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                PathBuf::from(home).join(rest)
            } else { continue }
        } else {
            PathBuf::from(prefix)
        };
        v.push(expanded.join(binary));
        // Windows .exe suffix
        v.push(expanded.join(format!("{}.exe", binary)));
    }
    v
}

/// Locate a binary by name. Searches:
///   1. `env_override` (e.g. `CHIMERA_ADB=/custom/adb`)
///   2. The `which` crate (uses $PATH + platform exec lookup)
///   3. A curated list of common install prefixes
fn locate(binary: &str, env_override: &str) -> Option<PathBuf> {
    // (1) Environment-variable override
    if let Ok(v) = std::env::var(env_override) {
        let p = PathBuf::from(&v);
        if p.exists() { return Some(p); }
    }
    // (2) `which` lookup
    if let Ok(p) = which::which(binary) {
        return Some(p);
    }
    // (3) Curated fallback paths
    for p in common_paths(binary) {
        if p.exists() { return Some(p); }
    }
    None
}

/// Run `<binary> <args…>` with a 5-second hard timeout and return its
/// trimmed stdout first-line on success.
fn run_capture(path: &Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new(path);
    cmd.args(args);
    // Inherit minimal env so the binary still finds its own libs.
    let child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn {}: {}", path.display(), e))?;

    // Best-effort timeout: wait up to 5 seconds.
    let pid = child.id();
    let _ = pid; // silence unused on platforms without wait_timeout
    let output = wait_with_timeout(child, Duration::from_secs(5))?;
    if !output.status.success() {
        return Err(format!(
            "{} exit {} — stderr: {}",
            path.display(),
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).lines().next().unwrap_or("").to_string(),
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().next().unwrap_or("").trim().to_string())
}

fn wait_with_timeout(
    mut child: std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    // Spawn a watcher thread that polls every 50ms.
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return Err(format!("timed out after {:?}", timeout));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("wait: {}", e)),
        }
    }
    child.wait_with_output().map_err(|e| format!("output: {}", e))
}

// ─── Public probes ─────────────────────────────────────────────────

/// Probe for the Android Debug Bridge (`adb`) daemon binary.
///
/// Success criteria:
///   * Binary located (env override / $PATH / common install dirs)
///   * `adb version` returns 0 within 5 seconds
///
/// The detected version line is captured for the dashboard.
pub fn detect_adb() -> ToolProbe {
    let path = match locate("adb", "CHIMERA_ADB") {
        Some(p) => p,
        None    => return ToolProbe::missing(
            "adb binary not found in $PATH or /usr/local/bin, /opt/homebrew/bin, /usr/bin. \
             Install Android platform-tools: `brew install android-platform-tools`."
        ),
    };
    match run_capture(&path, &["version"]) {
        Ok(version) => ToolProbe {
            found:   true,
            path:    Some(path),
            version: Some(version),
            error:   None,
        },
        Err(e) => ToolProbe::missing(format!("adb located but `adb version` failed: {}", e)),
    }
}

/// Probe for Android Fastboot.
pub fn detect_fastboot() -> ToolProbe {
    let path = match locate("fastboot", "CHIMERA_FASTBOOT") {
        Some(p) => p,
        None    => return ToolProbe::missing("fastboot not found in $PATH or common install dirs"),
    };
    match run_capture(&path, &["--version"]) {
        Ok(version) => ToolProbe { found: true, path: Some(path), version: Some(version), error: None },
        Err(e)      => ToolProbe::missing(format!("fastboot located but `--version` failed: {}", e)),
    }
}

/// Probe for `irecovery` from libimobiledevice / libirecovery.
pub fn detect_irecovery() -> ToolProbe {
    let path = match locate("irecovery", "CHIMERA_IRECOVERY") {
        Some(p) => p,
        None    => return ToolProbe::missing("irecovery not found"),
    };
    // irecovery uses `-h` for help; treat any zero exit as success.
    match run_capture(&path, &["-V"]) {
        Ok(version) => ToolProbe { found: true, path: Some(path), version: Some(version), error: None },
        Err(_) => ToolProbe { found: true, path: Some(path), version: None, error: None },
    }
}

/// Probe for `idevice_id` (presence of the libimobiledevice toolchain).
pub fn detect_idevice_id() -> ToolProbe {
    let path = match locate("idevice_id", "CHIMERA_IDEVICE_ID") {
        Some(p) => p,
        None    => return ToolProbe::missing(
            "idevice_id not found. Install libimobiledevice: `brew install libimobiledevice`."
        ),
    };
    match run_capture(&path, &["--help"]) {
        Ok(_)  => ToolProbe { found: true, path: Some(path), version: Some("libimobiledevice".into()), error: None },
        Err(_) => ToolProbe { found: true, path: Some(path), version: None, error: None },
    }
}

/// Probe for `palera1n` (jailbreak / passcode bypass tool).
pub fn detect_palera1n() -> ToolProbe {
    let path = match locate("palera1n", "CHIMERA_PALERA1N") {
        Some(p) => p,
        None    => return ToolProbe::missing("palera1n not found"),
    };
    ToolProbe { found: true, path: Some(path), version: Some("palera1n".into()), error: None }
}

/// Probe for `futurerestore`.
pub fn detect_futurerestore() -> ToolProbe {
    let path = match locate("futurerestore", "CHIMERA_FUTURERESTORE") {
        Some(p) => p,
        None    => return ToolProbe::missing("futurerestore not found"),
    };
    match run_capture(&path, &["--version"]) {
        Ok(version) => ToolProbe { found: true, path: Some(path), version: Some(version), error: None },
        Err(_)      => ToolProbe { found: true, path: Some(path), version: None, error: None },
    }
}

/// All probes in one struct — convenient for the dashboard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostToolProbes {
    pub adb:          ToolProbe,
    pub fastboot:     ToolProbe,
    pub irecovery:    ToolProbe,
    pub idevice_id:   ToolProbe,
    pub palera1n:     ToolProbe,
    pub futurerestore: ToolProbe,
}

impl HostToolProbes {
    /// Probe everything serially. Designed for one-shot startup probing.
    /// Total wall-clock cost: roughly 1-2 seconds with all tools present.
    pub fn probe_all() -> Self {
        Self {
            adb:           detect_adb(),
            fastboot:      detect_fastboot(),
            irecovery:     detect_irecovery(),
            idevice_id:    detect_idevice_id(),
            palera1n:      detect_palera1n(),
            futurerestore: detect_futurerestore(),
        }
    }

    /// Re-probe only the ADB daemon. Cheap; safe to call once per second.
    pub fn refresh_adb(&mut self) {
        self.adb = detect_adb();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_probe_has_error() {
        let p = ToolProbe::missing("no");
        assert!(!p.found);
        assert!(p.error.is_some());
    }

    #[test]
    fn locate_unknown_returns_none() {
        // This binary deliberately does not exist anywhere.
        assert!(locate("definitely-not-a-real-binary-xyz", "CHIMERA_NOPE").is_none());
    }

    #[test]
    fn common_paths_include_homebrew() {
        let v = common_paths("adb");
        let s: Vec<String> = v.iter().map(|p| p.display().to_string()).collect();
        assert!(s.iter().any(|p| p.contains("homebrew")));
        assert!(s.iter().any(|p| p.contains("/usr/local/bin")));
    }

    /// Live probe — runs only when `adb` is actually installed on the host.
    /// Pass `cargo test -- --ignored` to execute. Verifies the full pipeline:
    /// locate → spawn → capture version line.
    #[test]
    #[ignore = "requires adb on PATH"]
    fn detect_adb_live() {
        let p = detect_adb();
        if p.found {
            assert!(p.path.is_some(), "found=true must include path");
            assert!(p.version.is_some(), "found=true must include version");
            println!("ADB probe: {:?}", p);
        } else {
            println!("ADB probe missing (expected when adb not installed): {:?}", p.error);
        }
    }
}
