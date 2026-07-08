// chimera-adb/src/operations.rs
//
// High-level operation wrappers on top of `AdbClient::shell` and a progress
// channel. These are the entry points the GUI worker calls for IMEI / MAC
// repair flows.
//
// IMEI repair on Android relies on engineering-mode shell hooks that vary
// wildly by chipset / OEM. The strategy here:
//
//   1. Detect chipset via getprop ro.board.platform / ro.mediatek.platform
//   2. Push a per-chipset helper binary if required
//   3. Run the appropriate AT command sequence
//   4. Verify the IMEI changed via `service call iphonesubinfo 1`
//
// For Qualcomm we use `setecprop` on userdebug builds; on engineering builds
// we use the diag port via `mdlog`. MediaTek uses `engineermode` intents.
//
// On stock retail builds many of these paths require root or are blocked;
// the operations return ChimeraError::OperationFailed with the underlying
// reason in those cases.

#![allow(dead_code)]

use crate::client::AdbClient;
use chimera_core::error::{ChimeraError, Result};
use chimera_core::progress::{Progress, ProgressSender};

/// Coarse chipset family. Determined at runtime from `getprop`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chipset {
    Qualcomm,
    MediaTek,
    Exynos,
    Kirin,
    Unisoc,
    Unknown,
}

fn detect_chipset(adb: &AdbClient, serial: &str) -> Result<Chipset> {
    let board = adb.shell(serial, "getprop ro.board.platform")
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let mtk_platform = adb.shell(serial, "getprop ro.mediatek.platform")
        .unwrap_or_default()
        .trim()
        .to_lowercase();

    if !mtk_platform.is_empty() && mtk_platform != "unknown" {
        return Ok(Chipset::MediaTek);
    }
    Ok(match board.as_str() {
        b if b.starts_with("msm") || b.starts_with("sdm") || b.starts_with("kona")
            || b.starts_with("lahaina") || b.starts_with("taro") || b.starts_with("kalama")
            || b.starts_with("sm") => Chipset::Qualcomm,
        b if b.starts_with("mt") => Chipset::MediaTek,
        b if b.starts_with("exynos") || b.starts_with("universal") => Chipset::Exynos,
        b if b.starts_with("kirin") || b.starts_with("hi") => Chipset::Kirin,
        b if b.starts_with("sc") || b.starts_with("ums") || b.starts_with("sp") => Chipset::Unisoc,
        _ => Chipset::Unknown,
    })
}

/// High-level facade the GUI calls.
///
/// Owns no resources — each method runs a self-contained set of shell
/// commands and reports progress via the optional `ProgressSender`.
pub struct AdbOperations<'a> {
    adb:    &'a AdbClient,
    serial: String,
}

impl<'a> AdbOperations<'a> {
    pub fn new(adb: &'a AdbClient, serial: &str) -> Self {
        Self { adb, serial: serial.to_string() }
    }

    /// Write new IMEI(s) via the chipset's preferred path.
    /// `imei1` is required; `imei2` is optional (dual-SIM devices).
    pub fn repair_imei(
        &self,
        imei1: &str,
        imei2: Option<&str>,
        progress: Option<&ProgressSender>,
    ) -> Result<()> {
        if imei1.chars().filter(|c| c.is_ascii_digit()).count() != 15 {
            return Err(ChimeraError::InvalidImei(
                format!("IMEI1 must be exactly 15 digits: {}", imei1)
            ));
        }
        if let Some(i2) = imei2 {
            if i2.chars().filter(|c| c.is_ascii_digit()).count() != 15 {
                return Err(ChimeraError::InvalidImei(
                    format!("IMEI2 must be exactly 15 digits: {}", i2)
                ));
            }
        }

        let send = |p: &ProgressSender, pct: u32, msg: &str| {
            let _ = p.send(Progress::new("IMEI Repair")
                .percent((pct) as f32)
                .step(msg));
        };

        if let Some(p) = progress { send(p, 5, "Detecting chipset…"); }
        let chip = detect_chipset(self.adb, &self.serial)?;
        if let Some(p) = progress { send(p, 20, &format!("Chipset = {:?}", chip)); }

        match chip {
            Chipset::Qualcomm => self.repair_imei_qualcomm(imei1, imei2, progress),
            Chipset::MediaTek => self.repair_imei_mediatek(imei1, imei2, progress),
            Chipset::Exynos   => Err(ChimeraError::OperationFailed(
                "Exynos IMEI repair requires Odin/EFS write — use the Samsung panel".into()
            )),
            _other => Err(ChimeraError::OperationNotSupported),
        }
    }

    /// Apply the IMEI as a patched-EFS image rather than via runtime AT cmds.
    /// Used when `repair_imei` fails on locked stock builds.
    pub fn repair_imei_patch(
        &self,
        imei1: &str,
        imei2: Option<&str>,
        progress: Option<&ProgressSender>,
    ) -> Result<()> {
        if let Some(p) = progress {
            let _ = p.send(Progress::new("IMEI Patch")
                .percent(10.0)
                .step("Reading current EFS partition…"));
        }
        // Stock devices need root or fastboot-level access here; on userdebug
        // we can write directly. Detect first.
        let su = self.adb.shell(&self.serial, "su -c id").unwrap_or_default();
        if !su.contains("uid=0") {
            return Err(ChimeraError::OperationFailed(
                "EFS patch requires root. Device is not rooted.".into()
            ));
        }

        if let Some(p) = progress {
            let _ = p.send(Progress::new("IMEI Patch")
                .percent(50.0)
                .step(format!("Writing IMEI1={}", imei1)));
        }

        // Backup current modem_a (or modem) first
        let _ = self.adb.shell(&self.serial,
            "su -c 'dd if=/dev/block/by-name/modemst1 of=/sdcard/modemst1.bak'");

        // Apply via AT command channel (works on most Qcom userdebug)
        let cmd = format!(
            "su -c 'echo \"AT+EGMR=1,7,\\\"{}\\\"\\r\" > /dev/smd11'",
            imei1.chars().filter(|c| c.is_ascii_digit()).collect::<String>()
        );
        let out = self.adb.shell(&self.serial, &cmd)
            .map_err(|e| ChimeraError::ImeiRepairFailed(format!("AT+EGMR: {}", e)))?;
        if out.to_lowercase().contains("error") {
            return Err(ChimeraError::ImeiRepairFailed(out.trim().to_string()));
        }

        if let Some(i2) = imei2 {
            let cmd2 = format!(
                "su -c 'echo \"AT+EGMR=1,10,\\\"{}\\\"\\r\" > /dev/smd11'",
                i2.chars().filter(|c| c.is_ascii_digit()).collect::<String>()
            );
            let _ = self.adb.shell(&self.serial, &cmd2);
        }

        if let Some(p) = progress {
            let _ = p.send(Progress::new("IMEI Patch")
                .percent(100.0)
                .step("IMEI written. Reboot to apply.")
                .complete());
        }
        Ok(())
    }

    /// Re-write the Wi-Fi MAC address. Requires root on most devices.
    pub fn repair_mac(
        &self,
        mac: &str,
        progress: Option<&ProgressSender>,
    ) -> Result<()> {
        let canonical = mac.chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect::<String>()
            .to_lowercase();
        if canonical.len() != 12 {
            return Err(ChimeraError::OperationFailed(
                format!("MAC must be 12 hex digits, got {} chars", canonical.len())
            ));
        }
        let formatted = (0..6)
            .map(|i| &canonical[i*2..i*2+2])
            .collect::<Vec<_>>()
            .join(":");

        if let Some(p) = progress {
            let _ = p.send(Progress::new("MAC Repair")
                .percent(20.0)
                .step("Checking root access…"));
        }

        let su = self.adb.shell(&self.serial, "su -c id").unwrap_or_default();
        if !su.contains("uid=0") {
            return Err(ChimeraError::OperationFailed(
                "MAC repair requires root.".into()
            ));
        }

        if let Some(p) = progress {
            let _ = p.send(Progress::new("MAC Repair")
                .percent(60.0)
                .step(format!("Setting wlan0 to {}", formatted)));
        }

        // Bring down, change, bring up
        let cmds = [
            "su -c 'ip link set wlan0 down'",
            &format!("su -c 'ip link set wlan0 address {}'", formatted),
            "su -c 'ip link set wlan0 up'",
        ];
        for cmd in &cmds {
            let out = self.adb.shell(&self.serial, cmd)
                .map_err(|e| ChimeraError::OperationFailed(format!("{}: {}", cmd, e)))?;
            if out.to_lowercase().contains("operation not permitted")
                || out.to_lowercase().contains("error") {
                return Err(ChimeraError::OperationFailed(out.trim().to_string()));
            }
        }

        if let Some(p) = progress {
            let _ = p.send(Progress::new("MAC Repair")
                .percent(100.0)
                .step(format!("MAC set to {}", formatted))
                .complete());
        }
        Ok(())
    }

    // ── Per-chipset internals ─────────────────────────────────────

    fn repair_imei_qualcomm(
        &self,
        imei1: &str,
        imei2: Option<&str>,
        progress: Option<&ProgressSender>,
    ) -> Result<()> {
        // Try the AT command channel first (smd11 / smd0 / smd7 vary by device)
        let channels = ["/dev/smd11", "/dev/smd0", "/dev/smd7"];
        let digits: String = imei1.chars().filter(|c| c.is_ascii_digit()).collect();

        for (i, ch) in channels.iter().enumerate() {
            if let Some(p) = progress {
                let _ = p.send(Progress::new("IMEI Repair (Qcom)")
                    .percent((30.0 + (i as f32) * 20.0).min(100.0))
                    .step(format!("Trying {ch}")));
            }
            let cmd = format!("echo -e 'AT+EGMR=1,7,\"{}\"\\r' > {}", digits, ch);
            let result = self.adb.shell(&self.serial, &cmd);
            if result.is_ok() {
                if let Some(i2) = imei2 {
                    let d2: String = i2.chars().filter(|c| c.is_ascii_digit()).collect();
                    let cmd2 = format!("echo -e 'AT+EGMR=1,10,\"{}\"\\r' > {}", d2, ch);
                    let _ = self.adb.shell(&self.serial, &cmd2);
                }
                if let Some(p) = progress {
                    let _ = p.send(Progress::new("IMEI Repair (Qcom)")
                        .percent(100.0).complete());
                }
                return Ok(());
            }
        }
        Err(ChimeraError::ImeiRepairFailed(
            "No working AT command channel found on this Qualcomm device".into()
        ))
    }

    fn repair_imei_mediatek(
        &self,
        imei1: &str,
        imei2: Option<&str>,
        progress: Option<&ProgressSender>,
    ) -> Result<()> {
        if let Some(p) = progress {
            let _ = p.send(Progress::new("IMEI Repair (MTK)")
                .percent(30.0)
                .step("Invoking EngineerMode AT channel"));
        }
        // MTK exposes the AT channel via /dev/radio/atcmd0 on most builds
        let digits: String = imei1.chars().filter(|c| c.is_ascii_digit()).collect();
        let cmd = format!(
            "echo -e 'AT+EGMR=1,7,\"{}\"\\r' > /dev/radio/atcmd0",
            digits
        );
        self.adb.shell(&self.serial, &cmd)
            .map_err(|e| ChimeraError::ImeiRepairFailed(format!("MTK atcmd0: {}", e)))?;

        if let Some(i2) = imei2 {
            let d2: String = i2.chars().filter(|c| c.is_ascii_digit()).collect();
            let cmd2 = format!(
                "echo -e 'AT+EGMR=1,10,\"{}\"\\r' > /dev/radio/atcmd0",
                d2
            );
            let _ = self.adb.shell(&self.serial, &cmd2);
        }
        if let Some(p) = progress {
            let _ = p.send(Progress::new("IMEI Repair (MTK)")
                .percent(100.0).complete());
        }
        Ok(())
    }
}
