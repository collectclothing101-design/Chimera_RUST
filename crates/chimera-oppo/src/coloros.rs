// chimera-oppo/src/coloros.rs
// ColorOS / OxygenOS / RealmeUI specific helpers

use chimera_adb::shell::AdbShell;
use chimera_core::error::Result;

/// Detect ColorOS version
pub fn get_coloros_version(sh: &AdbShell) -> Option<String> {
    sh.get_prop("ro.build.version.oplusosv").ok()
        .filter(|s| !s.is_empty())
        .or_else(|| sh.get_prop("ro.build.version.ota").ok())
}

/// Detect OxygenOS version (OnePlus)
pub fn get_oxygenos_version(sh: &AdbShell) -> Option<String> {
    sh.get_prop("ro.build.version.ota").ok()
        .filter(|s| !s.is_empty())
}

/// Check if device is in Engineering Mode (allows FRP bypass)
pub fn is_engineering_mode(sh: &AdbShell) -> bool {
    sh.get_prop("persist.sys.oplus.engineering_mode")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
}

/// Get OPPO/Realme-specific security state
pub fn get_security_state(sh: &AdbShell) -> Vec<(String, String)> {
    let mut state = Vec::new();
    let props = [
        ("ro.oplus.image.fbo_enable",    "FBO Enabled"),
        ("persist.vendor.oplus.frp",     "FRP State"),
        ("ro.build.brand",               "Brand"),
        ("ro.product.oplusmodel",        "OPPO Model"),
        ("persist.radio.imei",           "Radio IMEI"),
    ];
    for (prop, label) in &props {
        if let Ok(val) = sh.get_prop(prop) {
            if !val.is_empty() && val != "unknown" {
                state.push((label.to_string(), val));
            }
        }
    }
    state
}

/// Disable OPPO anti-rollback protection (requires root, dangerous)
pub fn disable_anti_rollback(sh: &AdbShell) -> Result<()> {
    let _ = sh.run_root("resetprop ro.boot.avb_version 0.0");
    let _ = sh.run_root("resetprop ro.boot.vbmeta.avb_version 0.0");
    Ok(())
}
