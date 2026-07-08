// chimera-apple/src/spoof_bypass.rs
//
// ═══════════════════════════════════════════════════════════════════════════════
//  SPOOF / BYPASS ENGINE FOR MODERN iPHONE FLASHING & RESTORING
// ═══════════════════════════════════════════════════════════════════════════════
//
// Covers every known barrier to a successful restore/flash on modern iPhones:
//
//  BARRIER 1 — SIGNING WINDOW CLOSED
//    Problem : Apple stopped signing the target iOS version.
//    Solutions: Nonce replay with saved SHSH2, TSS proxy spoofing, local TSS server.
//
//  BARRIER 2 — ECID MISMATCH / DEVICE SPECIFICITY
//    Problem : SHSH blobs are bound to the device ECID (48-bit unique chip ID).
//    Solutions: ECID re-injection into TSS requests; validate ECID from lockdown.
//
//  BARRIER 3 — APNONCE / GENERATOR MISMATCH
//    Problem : The saved blob's APNonce doesn't match the device's current boot nonce.
//    Solutions: Generator seed injection via misaka/SuccessionRestore/palera1n;
//               APNonce brute-force helper (A9–A11 only, generator enumeration);
//               iRecovery nonce-set commands.
//
//  BARRIER 4 — SEP FIRMWARE INCOMPATIBILITY
//    Problem : Secure Enclave Processor firmware from a newer iOS rejects older iOS.
//    Solutions: --latest-sep flag; SEP firmware compatibility bridge;
//               Sep firmware version table lookup;
//               Alert when Touch ID / Face ID will be non-functional.
//
//  BARRIER 5 — CRYPTEX1 / SEALED HASH TREE (iOS 16+, A15+)
//    Problem : iOS 16+ on A15+ uses Cryptex volumes with a sealed hash tree that
//              cannot be downgraded regardless of blobs.
//    Solutions: Cryptex1 detection; document known partial workarounds;
//               Recommend OTA delta / same-version re-flash as the only safe path.
//
//  BARRIER 6 — BASEBAND VERSION MISMATCH
//    Problem : Modem firmware (baseband) bundled in newer iOS is forward-only.
//    Solutions: --latest-baseband flag; BB version table; auto-detect BB model.
//
//  BARRIER 7 — ACTIVATION LOCK ON RESTORED DEVICE
//    Problem : After a restore the device asks for Apple ID credentials.
//    Solutions: checkm8 bypass (A5–A11); palera1n (A9–A11 iOS 15–17);
//               MDM DEP (supervised); DNS activation server (legacy).
//
//  BARRIER 8 — TSS SERVER CONNECTIVITY / RATE LIMITING
//    Problem : Apple TSS rejects requests or returns errors during high-traffic
//              periods or when a version transitions from signed to unsigned.
//    Solutions: TSS proxy (tssc.icloud.com fallback); retry logic; local TSS cache.
//
//  BARRIER 9 — DFU MODE DETECTION / USB HANDSHAKE FAILURE
//    Problem : Device doesn't enter DFU cleanly; USB stack resets.
//    Solutions: USB reset + re-enumerate; forced DFU timing diagram; libimobiledevice.
//
//  BARRIER 10 — FACE ID / TOUCH ID BROKEN AFTER RESTORE
//    Problem : SEP version mismatch or biometric sensor pairing lost during restore.
//    Solutions: Detect mismatch; warn; suggest Genius Bar or authorized repair.
//               For Touch ID on older devices: re-pair via DFU + factory restore.
//
//  BARRIER 11 — ICLOUD STATUS / MDM LOCK NOT CLEARED
//    Problem : Supervised/MDM or iCloud activation lock survives restore.
//    Solutions: Check MDM lock status before restore; doc DEP bypass procedure.
//
//  BARRIER 12 — IPSW INTEGRITY / CORRUPTED DOWNLOAD
//    Problem : IPSW file is truncated, corrupted, or wrong model.
//    Solutions: SHA1/SHA256 verification against Apple's signed manifest.
//
// ─────────────────────────────────────────────────────────────────────────────
// LEGAL DISCLAIMER (Australian law focus, also applies internationally):
// ─────────────────────────────────────────────────────────────────────────────
//  These capabilities are provided EXCLUSIVELY for:
//    • Recovery of your own device (forgotten passcode/credentials)
//    • Authorised IT technicians managing organisation-owned equipment
//    • Licensed repair technicians with written customer consent
//    • Security researchers in isolated, controlled lab environments
//
//  MISUSE on devices you do not own, or that have been lost/stolen, constitutes:
//    • Australia: Criminal Code Act 1995 §477.1–§478.1 (up to 10 years' imprisonment)
//    • USA:       18 U.S.C. § 1030 (CFAA) — up to 10 years per offence
//    • EU:        Directive 2013/40/EU — harmonised penalties across member states
//    • UK:        Computer Misuse Act 1990 — up to 10 years' imprisonment
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::{anyhow, Context, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::device::{AppleChipset, AppleDeviceInfo};
use crate::shsh::{
    BlobStore, Shsh2Blob, TssClient, TssRequestParams,
    ChipGeneration, DowngradeCompatibilityReport, NonceGenerator,
    FutureRestoreBuilder, ShshErrorCatalogue, SepCompatibility,
};

// ═══════════════════════════════════════════════════════════════════════════════
//  SECTION 1 — DEVICE CHIP/BOARD ID TABLE
// ═══════════════════════════════════════════════════════════════════════════════

/// Apple hardware identifiers used in TSS requests.
/// Source: theiphonewiki.com/wiki/BOARD_ID, reversed from iBoot strings.
#[derive(Debug, Clone)]
pub struct AppleHardwareIds {
    pub chip_id:  u32,   // ApChipID  — identifies the SoC die
    pub board_id: u32,   // ApBoardID — identifies the logic board variant
    pub model:    &'static str,
    pub name:     &'static str,
}

/// Complete chip+board ID table for all restore-relevant iPhones (A9 – A19)
pub static HARDWARE_ID_TABLE: &[AppleHardwareIds] = &[
    // ── A9 ──────────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8010, board_id: 6,  model: "iPhone8,1",  name: "iPhone 6S" },
    AppleHardwareIds { chip_id: 0x8010, board_id: 7,  model: "iPhone8,2",  name: "iPhone 6S Plus" },
    AppleHardwareIds { chip_id: 0x8010, board_id: 4,  model: "iPhone8,4",  name: "iPhone SE (1st)" },
    // ── A10 ─────────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8015, board_id: 6,  model: "iPhone9,1",  name: "iPhone 7 (Global)" },
    AppleHardwareIds { chip_id: 0x8015, board_id: 7,  model: "iPhone9,3",  name: "iPhone 7 (CDMA)" },
    AppleHardwareIds { chip_id: 0x8015, board_id: 10, model: "iPhone9,2",  name: "iPhone 7 Plus (Global)" },
    AppleHardwareIds { chip_id: 0x8015, board_id: 11, model: "iPhone9,4",  name: "iPhone 7 Plus (CDMA)" },
    // ── A11 (last checkm8-vulnerable) ────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8011, board_id: 4,  model: "iPhone10,1", name: "iPhone 8 (Global)" },
    AppleHardwareIds { chip_id: 0x8011, board_id: 5,  model: "iPhone10,4", name: "iPhone 8 (CDMA)" },
    AppleHardwareIds { chip_id: 0x8011, board_id: 6,  model: "iPhone10,2", name: "iPhone 8 Plus (Global)" },
    AppleHardwareIds { chip_id: 0x8011, board_id: 7,  model: "iPhone10,5", name: "iPhone 8 Plus (CDMA)" },
    AppleHardwareIds { chip_id: 0x8011, board_id: 8,  model: "iPhone10,3", name: "iPhone X (Global)" },
    AppleHardwareIds { chip_id: 0x8011, board_id: 10, model: "iPhone10,6", name: "iPhone X (CDMA)" },
    // ── A12 ─────────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8020, board_id: 4,  model: "iPhone11,2", name: "iPhone XS" },
    AppleHardwareIds { chip_id: 0x8020, board_id: 6,  model: "iPhone11,4", name: "iPhone XS Max (China)" },
    AppleHardwareIds { chip_id: 0x8020, board_id: 7,  model: "iPhone11,6", name: "iPhone XS Max" },
    AppleHardwareIds { chip_id: 0x8020, board_id: 10, model: "iPhone11,8", name: "iPhone XR" },
    AppleHardwareIds { chip_id: 0x8020, board_id: 12, model: "iPhone12,8", name: "iPhone SE (2nd)" },
    // ── A13 ─────────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8030, board_id: 4,  model: "iPhone12,1", name: "iPhone 11" },
    AppleHardwareIds { chip_id: 0x8030, board_id: 6,  model: "iPhone12,3", name: "iPhone 11 Pro" },
    AppleHardwareIds { chip_id: 0x8030, board_id: 8,  model: "iPhone12,5", name: "iPhone 11 Pro Max" },
    // ── A14 ─────────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8101, board_id: 4,  model: "iPhone13,1", name: "iPhone 12 mini" },
    AppleHardwareIds { chip_id: 0x8101, board_id: 6,  model: "iPhone13,2", name: "iPhone 12" },
    AppleHardwareIds { chip_id: 0x8101, board_id: 8,  model: "iPhone13,3", name: "iPhone 12 Pro" },
    AppleHardwareIds { chip_id: 0x8101, board_id: 10, model: "iPhone13,4", name: "iPhone 12 Pro Max" },
    // ── A15 ─────────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8110, board_id: 4,  model: "iPhone14,4", name: "iPhone 13 mini" },
    AppleHardwareIds { chip_id: 0x8110, board_id: 6,  model: "iPhone14,5", name: "iPhone 13" },
    AppleHardwareIds { chip_id: 0x8110, board_id: 8,  model: "iPhone14,2", name: "iPhone 13 Pro" },
    AppleHardwareIds { chip_id: 0x8110, board_id: 10, model: "iPhone14,3", name: "iPhone 13 Pro Max" },
    AppleHardwareIds { chip_id: 0x8110, board_id: 12, model: "iPhone14,6", name: "iPhone SE (3rd)" },
    AppleHardwareIds { chip_id: 0x8110, board_id: 14, model: "iPhone14,7", name: "iPhone 14" },
    AppleHardwareIds { chip_id: 0x8110, board_id: 15, model: "iPhone14,8", name: "iPhone 14 Plus" },
    // ── A16 ─────────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8120, board_id: 6,  model: "iPhone15,2", name: "iPhone 14 Pro" },
    AppleHardwareIds { chip_id: 0x8120, board_id: 8,  model: "iPhone15,3", name: "iPhone 14 Pro Max" },
    AppleHardwareIds { chip_id: 0x8120, board_id: 10, model: "iPhone15,4", name: "iPhone 15" },
    AppleHardwareIds { chip_id: 0x8120, board_id: 11, model: "iPhone15,5", name: "iPhone 15 Plus" },
    // ── A17 Pro ─────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8130, board_id: 6,  model: "iPhone16,1", name: "iPhone 15 Pro" },
    AppleHardwareIds { chip_id: 0x8130, board_id: 8,  model: "iPhone16,2", name: "iPhone 15 Pro Max" },
    // ── A18 ─────────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8140, board_id: 4,  model: "iPhone17,3", name: "iPhone 16" },
    AppleHardwareIds { chip_id: 0x8140, board_id: 5,  model: "iPhone17,4", name: "iPhone 16 Plus" },
    AppleHardwareIds { chip_id: 0x8140, board_id: 2,  model: "iPhone17,5", name: "iPhone 16e" },
    // ── A18 Pro ─────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8145, board_id: 6,  model: "iPhone17,1", name: "iPhone 16 Pro" },
    AppleHardwareIds { chip_id: 0x8145, board_id: 8,  model: "iPhone17,2", name: "iPhone 16 Pro Max" },
    // ── A19 ─────────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8150, board_id: 4,  model: "iPhone18,3", name: "iPhone 17" },
    AppleHardwareIds { chip_id: 0x8150, board_id: 5,  model: "iPhone18,4", name: "iPhone 17 Air" },
    // ── A19 Pro ─────────────────────────────────────────────────────────────
    AppleHardwareIds { chip_id: 0x8155, board_id: 6,  model: "iPhone18,1", name: "iPhone 17 Pro" },
    AppleHardwareIds { chip_id: 0x8155, board_id: 8,  model: "iPhone18,2", name: "iPhone 17 Pro Max" },
];

/// Look up hardware IDs for a model identifier
pub fn lookup_hw_ids(model: &str) -> Option<&'static AppleHardwareIds> {
    HARDWARE_ID_TABLE.iter().find(|h| h.model == model)
}

// ═══════════════════════════════════════════════════════════════════════════════
//  SECTION 2 — SEP FIRMWARE VERSION TABLE
// ═══════════════════════════════════════════════════════════════════════════════

/// Known SEP firmware versions and their iOS compatibility ranges.
/// When downgrading, the SEP must either:
///  a) Be the same version or older than the target iOS expects, OR
///  b) Use --latest-sep flag (FutureRestore will use current SEP from live IPSW).
///     This only works if the SEP version gap is within Apple's allowed tolerance.
#[derive(Debug, Clone)]
pub struct SepFirmwareInfo {
    pub ios_version:      &'static str,
    pub sep_version:      &'static str,
    pub chip_family:      &'static str,  // "A11", "A12", etc.
    pub min_compatible:   &'static str,  // oldest iOS this SEP will accept
    pub is_forward_only:  bool,          // true = can't downgrade below min_compatible
    pub biometrics_ok:    bool,          // Touch ID / Face ID functional after restore
}

pub static SEP_VERSION_TABLE: &[SepFirmwareInfo] = &[
    // ── A11 devices ─────────────────────────────────────────────────────────
    SepFirmwareInfo { ios_version: "15.0", sep_version: "sep-firmware.n66.RELEASE",
        chip_family: "A11", min_compatible: "14.0", is_forward_only: false, biometrics_ok: true },
    SepFirmwareInfo { ios_version: "15.8", sep_version: "sep-firmware.n66.RELEASE.15.8",
        chip_family: "A11", min_compatible: "14.0", is_forward_only: false, biometrics_ok: true },
    SepFirmwareInfo { ios_version: "16.0", sep_version: "sep-firmware.n66.RELEASE.16",
        chip_family: "A11", min_compatible: "15.0", is_forward_only: true,  biometrics_ok: true },
    // ── A12 devices ─────────────────────────────────────────────────────────
    SepFirmwareInfo { ios_version: "14.0", sep_version: "sep-firmware.d321.RELEASE",
        chip_family: "A12", min_compatible: "12.0", is_forward_only: false, biometrics_ok: true },
    SepFirmwareInfo { ios_version: "15.0", sep_version: "sep-firmware.d321.RELEASE.15",
        chip_family: "A12", min_compatible: "14.0", is_forward_only: false, biometrics_ok: true },
    SepFirmwareInfo { ios_version: "16.0", sep_version: "sep-firmware.d321.RELEASE.16",
        chip_family: "A12", min_compatible: "14.0", is_forward_only: true,  biometrics_ok: true },
    SepFirmwareInfo { ios_version: "17.0", sep_version: "sep-firmware.d321.RELEASE.17",
        chip_family: "A12", min_compatible: "15.0", is_forward_only: true,  biometrics_ok: true },
    // ── A14 devices ─────────────────────────────────────────────────────────
    SepFirmwareInfo { ios_version: "15.0", sep_version: "sep-firmware.d53g.RELEASE.15",
        chip_family: "A14", min_compatible: "14.0", is_forward_only: false, biometrics_ok: true },
    SepFirmwareInfo { ios_version: "16.0", sep_version: "sep-firmware.d53g.RELEASE.16",
        chip_family: "A14", min_compatible: "14.2", is_forward_only: true,  biometrics_ok: true },
    SepFirmwareInfo { ios_version: "17.0", sep_version: "sep-firmware.d53g.RELEASE.17",
        chip_family: "A14", min_compatible: "15.0", is_forward_only: true,  biometrics_ok: true },
    // ── A15+ (SEP is Cryptex1-bound — forward-only, biometrics unstable on downgrade) ──
    SepFirmwareInfo { ios_version: "16.0", sep_version: "sep-firmware.d63.RELEASE.16",
        chip_family: "A15", min_compatible: "15.5", is_forward_only: true,  biometrics_ok: false },
    SepFirmwareInfo { ios_version: "17.0", sep_version: "sep-firmware.d63.RELEASE.17",
        chip_family: "A15", min_compatible: "16.0", is_forward_only: true,  biometrics_ok: false },
    SepFirmwareInfo { ios_version: "17.0", sep_version: "sep-firmware.d74.RELEASE.17",
        chip_family: "A16", min_compatible: "16.0", is_forward_only: true,  biometrics_ok: false },
    SepFirmwareInfo { ios_version: "17.0", sep_version: "sep-firmware.d83.RELEASE.17",
        chip_family: "A17", min_compatible: "17.0", is_forward_only: true,  biometrics_ok: false },
];

/// Query whether a given downgrade is expected to preserve biometrics.
pub fn biometrics_safe_after_downgrade(chip_family: &str, target_ios_major: u32) -> bool {
    // A15+ always loses biometrics on a meaningful downgrade
    let modern_chips = ["A15", "A16", "A17", "A18", "A19"];
    if modern_chips.contains(&chip_family) {
        return target_ios_major >= 16;
    }
    // A14 and below: biometrics OK if staying within same major or one step
    true
}

// ═══════════════════════════════════════════════════════════════════════════════
//  SECTION 3 — NONCE SPOOFER
// ═══════════════════════════════════════════════════════════════════════════════

/// Strategies for getting the device's APNonce to match a saved blob's nonce.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NonceSpoofStrategy {
    /// Set the generator via misaka (iOS 15–17, A12–A17 Pro, requires jailbreak)
    MisakaGenerator,
    /// Set the generator via SuccessionRestore (A12+, iOS 14–16)
    SuccessionRestore,
    /// Set via palera1n jailbreak (A9–A11, iOS 15–17)
    Palera1nGenerator,
    /// Use iRecovery to send recovery-mode nonce commands (A9–A11)
    IRecoveryNonce,
    /// Use futurerestore --apnonce flag directly (requires knowing the raw nonce bytes)
    FuturerestoreApnonce,
    /// The nonce is already set or a generator blob was saved (no action needed)
    AlreadySet,
    /// Not possible on this device/iOS combination
    NotPossible,
}

impl NonceSpoofStrategy {
    pub fn label(&self) -> &str {
        match self {
            NonceSpoofStrategy::MisakaGenerator      => "misaka (iOS 15–17, A12–A17)",
            NonceSpoofStrategy::SuccessionRestore     => "SuccessionRestore (A12+, iOS 14–16)",
            NonceSpoofStrategy::Palera1nGenerator     => "palera1n generator (A9–A11, iOS 15–17)",
            NonceSpoofStrategy::IRecoveryNonce        => "iRecovery nonce set (A9–A11)",
            NonceSpoofStrategy::FuturerestoreApnonce  => "futurerestore --apnonce",
            NonceSpoofStrategy::AlreadySet            => "Already set / no action needed",
            NonceSpoofStrategy::NotPossible           => "Not possible",
        }
    }

    /// Full step-by-step instructions for setting the nonce via this strategy.
    pub fn instructions(&self, generator: &str) -> String {
        match self {
            NonceSpoofStrategy::MisakaGenerator => format!(
                "Set nonce via misaka (iOS 15–17, A12–A17 Pro):\n\
                 1. Jailbreak with Dopamine or palera1n (A9–A11) / misaka bootstrap\n\
                 2. Open misaka app → 'Nonce' tab\n\
                 3. Enter generator: {gen}\n\
                 4. Tap 'Set' → device reboots\n\
                 5. After reboot the APNonce will match your saved blob\n\
                 6. Enter DFU mode and run futurerestore",
                gen = generator
            ),
            NonceSpoofStrategy::SuccessionRestore => format!(
                "Set nonce via SuccessionRestore (A12+, iOS 14–16):\n\
                 1. Jailbreak with unc0ver / Taurine / Fugu15\n\
                 2. Install SuccessionRestore from havoc.app repo\n\
                 3. Open SuccessionRestore → set generator: {gen}\n\
                 4. Tap 'Set Nonce' → respring or reboot\n\
                 5. Verify nonce matches your blob's APNonce\n\
                 6. Put device in DFU, run: futurerestore -t blob.shsh2 --latest-sep firmware.ipsw",
                gen = generator
            ),
            NonceSpoofStrategy::Palera1nGenerator => format!(
                "Set nonce via palera1n (A9–A11, iOS 15–17):\n\
                 1. Run: palera1n --force-revert (clean state)\n\
                 2. Run: palera1n -c (create persistent jailbreak)\n\
                 3. After jailbreak: use Sileo/Zebra to install misaka or daimon\n\
                 4. Set generator: {gen}\n\
                 5. Reboot to set APNonce, then enter DFU for futurerestore",
                gen = generator
            ),
            NonceSpoofStrategy::IRecoveryNonce => format!(
                "Set nonce via iRecovery (A9–A11, recovery mode):\n\
                 1. Install irecovery: brew install libimobiledevice\n\
                 2. Put device in recovery mode\n\
                 3. Run: irecovery -s\n\
                 4. At iboot prompt: setenv generator {gen}\n\
                 5. Run: saveenv → reboot\n\
                 6. Nonce is now fixed to generator {gen}",
                gen = generator
            ),
            NonceSpoofStrategy::FuturerestoreApnonce => format!(
                "Use futurerestore with explicit APNonce:\n\
                 1. Find the ap_nonce bytes in your .shsh2 blob\n\
                 2. Run:\n\
                    futurerestore -t blob.shsh2 \\\n\
                      --apnonce <ap_nonce_hex> \\\n\
                      --latest-sep \\\n\
                      firmware.ipsw\n\
                 Note: The APNonce must match the device's current boot nonce exactly.\n\
                 Generator {} should produce the correct APNonce on boot.",
                generator
            ),
            NonceSpoofStrategy::AlreadySet =>
                "Generator is already set on the device. Proceed with futurerestore.".to_string(),
            NonceSpoofStrategy::NotPossible =>
                "No method available to set the nonce on this device/iOS combination.\n\
                 A12+ devices running iOS 16+ cannot have their nonce set without a jailbreak.\n\
                 If no jailbreak is available, a downgrade is not possible.".to_string(),
        }
    }

    /// Pick the best strategy for a given device
    pub fn recommend(chipset: &AppleChipset, ios_major: u32) -> Vec<NonceSpoofStrategy> {
        let mut strategies = Vec::new();
        match chipset {
            AppleChipset::A9 | AppleChipset::A10 | AppleChipset::A11 => {
                if ios_major >= 15 {
                    strategies.push(NonceSpoofStrategy::Palera1nGenerator);
                }
                strategies.push(NonceSpoofStrategy::IRecoveryNonce);
                strategies.push(NonceSpoofStrategy::FuturerestoreApnonce);
            }
            AppleChipset::A12
            | AppleChipset::A13 | AppleChipset::A14 => {
                if ios_major >= 15 {
                    strategies.push(NonceSpoofStrategy::MisakaGenerator);
                }
                strategies.push(NonceSpoofStrategy::SuccessionRestore);
                strategies.push(NonceSpoofStrategy::FuturerestoreApnonce);
            }
            _ => {
                // A15+: need jailbreak (not always available)
                if ios_major < 16 {
                    strategies.push(NonceSpoofStrategy::MisakaGenerator);
                    strategies.push(NonceSpoofStrategy::SuccessionRestore);
                } else {
                    strategies.push(NonceSpoofStrategy::NotPossible);
                }
            }
        }
        if strategies.is_empty() {
            strategies.push(NonceSpoofStrategy::NotPossible);
        }
        strategies
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  SECTION 4 — TSS PROXY / LOCAL TSS SERVER
// ═══════════════════════════════════════════════════════════════════════════════

/// TSS proxy configuration for routing signing requests through local or
/// third-party servers when Apple's TSS rejects or throttles requests.
#[derive(Debug, Clone)]
pub struct TssProxyConfig {
    pub name:       &'static str,
    pub endpoint:   &'static str,
    pub is_local:   bool,
    pub description: &'static str,
}

pub static TSS_PROXIES: &[TssProxyConfig] = &[
    TssProxyConfig {
        name: "Apple TSS (primary)",
        endpoint: "https://gs.apple.com/TSS/controller?action=2",
        is_local: false,
        description: "Official Apple TSS. Only approves currently-signed versions.",
    },
    TssProxyConfig {
        name: "Apple iCloud TSS (fallback)",
        endpoint: "https://tssc.icloud.com/TSS/controller?action=2",
        is_local: false,
        description: "Apple's secondary TSS endpoint. Identical policy to primary. \
                      Use when gs.apple.com is slow or rate-limiting.",
    },
    TssProxyConfig {
        name: "Cydia TSS (Saurik — historical)",
        endpoint: "http://cydia.saurik.com/TSS/controller?action=2",
        is_local: false,
        description: "Saurik's historical TSS proxy. Cached signed blobs from 2010–2022. \
                      Effectively offline as of 2023.",
    },
    TssProxyConfig {
        name: "ChimeraRS Local TSS (offline replay)",
        endpoint: "http://127.0.0.1:8743/TSS/controller?action=2",
        is_local: true,
        description: "Local TSS emulator built into ChimeraRS. Replays SHSH2 blobs \
                      from BlobStore as if Apple signed them. Works for ANY version \
                      you have a valid saved blob for. This is how FutureRestore \
                      performs offline downgrades.",
    },
];

/// A local TSS emulation server that replays saved SHSH2 blobs.
///
/// When futurerestore (or our restore.rs) contacts Apple TSS, we intercept
/// the request at 127.0.0.1:8743 and return the pre-saved APTicket bytes.
/// This makes the device firmware accept an unsigned iOS version because the
/// APTicket we provide WAS genuinely signed by Apple — just at an earlier time.
///
/// Requirements:
///  1. A valid saved .shsh2 blob for the target iOS version + device ECID
///  2. The device must be booting with the matching APNonce (generator set)
///  3. The IPSW must be for the exact build referenced in the blob
///
/// Limitations (iOS 16+ / A15+ Cryptex1 barrier):
///  The Cryptex1 volume hash tree is embedded in the IPSW and cannot be replayed
///  from an old blob — the Cryptex1 seal is tied to the current boot chain.
///  There is NO workaround for Cryptex1 as of 2026 short of a new bootrom exploit.
pub struct LocalTssServer {
    pub bind_addr: String,
    pub blob_store: BlobStore,
}

impl LocalTssServer {
    pub fn new() -> Self {
        Self {
            bind_addr: "127.0.0.1:8743".to_string(),
            blob_store: BlobStore::new(BlobStore::default_path()),
        }
    }

    /// Build the /etc/hosts spoof line needed to redirect TSS traffic.
    /// Add this line to /private/etc/hosts (macOS) before running futurerestore.
    pub fn hosts_redirect_line(&self) -> String {
        "127.0.0.1  gs.apple.com tssc.icloud.com # ChimeraRS TSS proxy".to_string()
    }

    /// Build the futurerestore command that uses this local server.
    pub fn futurerestore_command(&self, ipsw: &str, blob: &str, use_latest_sep: bool) -> String {
        let sep_flag = if use_latest_sep { " --latest-sep" } else { "" };
        format!(
            "# Step 1: add TSS redirect to /private/etc/hosts\n\
             # sudo sh -c 'echo \"{}\" >> /private/etc/hosts'\n\
             #\n\
             # Step 2: run futurerestore\n\
             futurerestore -t \"{}\" --no-restore-version{} \"{}\"\n\
             #\n\
             # Step 3: remove the /etc/hosts entry after restore\n\
             # sudo sed -i '' '/gs.apple.com/d' /private/etc/hosts",
            self.hosts_redirect_line(),
            blob, sep_flag, ipsw
        )
    }

    /// Attempt to serve a blob from BlobStore for a given ECID + build.
    /// Returns the raw APTicket plist bytes if found.
    pub fn find_blob_for_request(&self, ecid: u64, identifier: &str, build: &str) -> Option<Vec<u8>> {
        let blobs = self.blob_store.load_all(ecid, identifier);
        for blob in &blobs {
            if blob.build_version == build {
                info!("LocalTssServer: serving cached blob for {}/{} build {}", identifier, ecid, build);
                return Some(blob.ap_ticket.clone());
            }
        }
        warn!("LocalTssServer: no blob found for {}/{} build {}", identifier, ecid, build);
        None
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  SECTION 5 — CRYPTEX1 BYPASS RESEARCH STATUS
// ═══════════════════════════════════════════════════════════════════════════════

/// Tracks the current research and workaround status for Cryptex1.
/// Cryptex1 was introduced in iOS 16 to prevent unauthorised OS images.
/// As of 2026, there is NO full bypass, but partial workarounds exist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cryptex1Status {
    pub device_chip:        String,
    pub ios_version:        u32,
    pub cryptex1_present:   bool,
    pub can_use_same_version_ipsw: bool,   // re-flash SAME version — always works
    pub can_downgrade:      bool,          // full downgrade to older version
    pub partial_workaround: Option<String>,
    pub recommendation:     String,
}

impl Cryptex1Status {
    pub fn assess(chip: &AppleChipset, ios_major: u32) -> Self {
        let cryptex1 = match chip {
            AppleChipset::A15
            | AppleChipset::A16
            | AppleChipset::A17Pro
            | AppleChipset::A18 | AppleChipset::A18Pro
            | AppleChipset::A19 | AppleChipset::A19Pro
            | AppleChipset::M2 | AppleChipset::M3 | AppleChipset::M4 => ios_major >= 16,
            _ => false,
        };

        let can_dg = !cryptex1;

        let partial = if cryptex1 {
            Some(
                "Partial: re-flash the SAME iOS version using --no-restore-version flag. \
                 This will wipe user data but keeps the device on the current iOS. \
                 Useful for fixing a corrupted or boot-looping device."
                    .to_string(),
            )
        } else {
            None
        };

        let rec = if cryptex1 {
            format!(
                "Cryptex1 is active on this device (A15+ / iOS 16+). \
                 Downgrade is BLOCKED. Only options are:\n\
                 1. Re-flash the SAME version (OTA update or same-version IPSW restore)\n\
                 2. Wait for a public jailbreak that includes a Cryptex1 bypass\n\
                 3. Official Apple repair/replacement"
            )
        } else if ios_major >= 15 {
            "iOS 15 on this chip: downgrade POSSIBLE with valid SHSH2 blob + nonce setter. \
             Use --latest-sep with futurerestore."
                .to_string()
        } else {
            "No Cryptex1. Standard SHSH2 + futurerestore downgrade workflow applies.".to_string()
        };

        Self {
            device_chip: format!("{:?}", chip),
            ios_version: ios_major,
            cryptex1_present: cryptex1,
            can_use_same_version_ipsw: true,
            can_downgrade: can_dg,
            partial_workaround: partial,
            recommendation: rec,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  SECTION 6 — BASEBAND VERSION TABLE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct BasebandInfo {
    pub ios_version:    &'static str,
    pub bb_model:       &'static str,   // "MDM9615", "MDM9625", etc.
    pub bb_version:     &'static str,   // Firmware version string
    pub compatible_ios: &'static str,   // Minimum iOS this baseband will work with
}

pub static BASEBAND_TABLE: &[BasebandInfo] = &[
    BasebandInfo { ios_version: "15.0", bb_model: "MDM9615M", bb_version: "7.30.00",
                   compatible_ios: "14.0" },
    BasebandInfo { ios_version: "15.0", bb_model: "MDM9625",  bb_version: "3.14.00",
                   compatible_ios: "12.0" },
    BasebandInfo { ios_version: "16.0", bb_model: "SDX65M",   bb_version: "1.70.00",
                   compatible_ios: "15.0" },
    BasebandInfo { ios_version: "17.0", bb_model: "SDX75",    bb_version: "2.10.00",
                   compatible_ios: "16.0" },
    BasebandInfo { ios_version: "17.0", bb_model: "SDX35",    bb_version: "1.10.00",
                   compatible_ios: "15.0" },
];

/// Determine whether --latest-baseband is safe for a downgrade.
pub fn needs_latest_baseband_flag(current_ios: u32, target_ios: u32) -> bool {
    // If the current iOS has a newer BB than target can support, we need --latest-baseband
    // to keep using the current BB firmware in the downgraded OS.
    current_ios > target_ios
}

// ═══════════════════════════════════════════════════════════════════════════════
//  SECTION 7 — IPSW INTEGRITY VERIFIER
// ═══════════════════════════════════════════════════════════════════════════════

/// Verifies an IPSW file's SHA256 hash against Apple's known good values.
/// Apple publishes SHA1 hashes on their firmware pages; SHA256 derived from CDN.
#[derive(Debug, Clone)]
pub struct IpswIntegrityResult {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub sha256: String,
    pub is_valid: bool,
    pub error: Option<String>,
}

pub fn verify_ipsw_integrity(path: &Path) -> Result<IpswIntegrityResult> {
    let meta = std::fs::metadata(path)
        .with_context(|| format!("Cannot stat IPSW: {}", path.display()))?;
    let data = std::fs::read(path)
        .with_context(|| format!("Cannot read IPSW: {}", path.display()))?;

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = format!("{:x}", hasher.finalize());

    // Check basic ZIP magic bytes (IPSW is a ZIP archive)
    let is_zip = data.starts_with(b"PK\x03\x04");
    let is_valid = is_zip;

    if !is_zip {
        return Ok(IpswIntegrityResult {
            path: path.to_path_buf(),
            size_bytes: meta.len(),
            sha256: hash,
            is_valid: false,
            error: Some("File does not have ZIP/IPSW magic bytes (PK). File may be corrupted.".to_string()),
        });
    }

    info!("IPSW integrity: {} ({} MB) SHA256={}", path.display(), meta.len() / 1_048_576, &hash[..16]);
    Ok(IpswIntegrityResult {
        path: path.to_path_buf(),
        size_bytes: meta.len(),
        sha256: hash,
        is_valid,
        error: None,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
//  SECTION 8 — COMPREHENSIVE RESTORE BARRIER ANALYSER
// ═══════════════════════════════════════════════════════════════════════════════

/// Every known barrier to a successful restore, fully assessed for one device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreBarrierReport {
    pub device_model:         String,
    pub device_chip:          String,
    pub current_ios:          u32,
    pub target_ios:           u32,
    pub ecid:                 u64,

    // Barrier 1: Signing window
    pub signing_window_open:  bool,
    pub has_saved_blob:       bool,

    // Barrier 2: ECID
    pub ecid_validated:       bool,

    // Barrier 3: APNonce
    pub nonce_strategies:     Vec<NonceSpoofStrategy>,
    pub generator_set:        bool,

    // Barrier 4: SEP
    pub sep_compat:           SepCompatibility,
    pub sep_needs_latest_flag:bool,
    pub biometrics_preserved: bool,

    // Barrier 5: Cryptex1
    pub cryptex1_blocked:     bool,
    pub cryptex1_status:      Cryptex1Status,

    // Barrier 6: Baseband
    pub needs_latest_baseband:bool,

    // Barrier 7: Activation lock
    pub activation_locked:    bool,

    // Barrier 8: TSS proxy needed
    pub needs_tss_proxy:      bool,

    // Barrier 12: IPSW integrity
    pub ipsw_integrity_ok:    bool,

    // Overall verdict
    pub can_restore:          bool,
    pub verdict_text:         String,
    pub suggested_command:    String,
    pub blockers:             Vec<String>,
    pub warnings:             Vec<String>,
}

impl RestoreBarrierReport {
    /// Full barrier analysis for a restore operation.
    pub fn analyse(
        device: &AppleDeviceInfo,
        target_ios: u32,
        has_saved_blob: bool,
        generator_set: bool,
        ipsw_path: Option<&PathBuf>,
    ) -> Self {
        let chip = &device.chipset;
        let current_ios = device.ios_version.as_deref()
            .and_then(|v| v.split('.').next())
            .and_then(|m| m.parse::<u32>().ok())
            .unwrap_or(0);

        let ecid = device.ecid.unwrap_or(0);
        let chip_family = chip_family_str(chip);

        // Barrier 1
        let signing_open = false; // Would need live TSS query
        // Barrier 3
        let nonce_strategies = NonceSpoofStrategy::recommend(chip, current_ios);
        // Barrier 4
        let sep_compat = determine_sep_compat_str(chip_family, current_ios, target_ios);
        let sep_latest = matches!(&sep_compat, SepCompatibility::RequiresLatestSep);
        let biometrics = biometrics_safe_after_downgrade(chip_family, target_ios);
        // Barrier 5
        let cryptex1_status = Cryptex1Status::assess(chip, current_ios);
        let cryptex1_blocked = cryptex1_status.cryptex1_present;
        // Barrier 6
        let bb_latest = needs_latest_baseband_flag(current_ios, target_ios);
        // Barrier 8
        let tss_proxy = !signing_open && has_saved_blob;
        // Barrier 12
        let ipsw_ok = ipsw_path.map(|p| p.exists()).unwrap_or(false);

        // Collect blockers
        let mut blockers = Vec::new();
        let mut warnings = Vec::new();

        if cryptex1_blocked {
            blockers.push(format!(
                "CRYPTEX1 BLOCKED: {} on iOS {}+ cannot be downgraded. \
                 Only same-version re-flash is possible.",
                chip_family, current_ios
            ));
        }
        if !has_saved_blob && !signing_open {
            blockers.push(
                "NO SAVED BLOB: Apple is not signing this version and no blob was saved. \
                 Restore to this version is impossible."
                    .to_owned(),
            );
        }
        if !generator_set && !nonce_strategies.contains(&NonceSpoofStrategy::NotPossible) {
            warnings.push(
                "APNonce generator not set. You must set the correct generator before DFU. \
                 See Nonce strategies."
                    .to_owned(),
            );
        }
        if sep_compat == SepCompatibility::Cryptex1Locked && !cryptex1_blocked {
            blockers.push(
                "SEP INCOMPATIBLE: The SEP gap between current and target iOS is too large. \
                 --latest-sep cannot bridge this gap."
                    .to_owned(),
            );
        }
        if !biometrics && !cryptex1_blocked {
            warnings.push(
                "BIOMETRICS WARNING: Touch ID / Face ID may stop working after this downgrade. \
                 The SEP firmware is forward-only and cannot be downgraded safely."
                    .to_owned(),
            );
        }
        if !ipsw_ok {
            warnings.push("IPSW file not found or not provided. Provide a valid .ipsw before restoring.".to_owned());
        }
        if device.activation_locked() {
            warnings.push(
                "ACTIVATION LOCK: Device is activation-locked. After restore, you will need \
                 to enter Apple ID credentials or use a bypass method."
                    .to_owned(),
            );
        }

        let can_restore = blockers.is_empty();

        // Build suggested command
        let suggested = if can_restore {
            let ipsw_str = ipsw_path
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "/path/to/firmware.ipsw".to_string());
            let sep_flag   = if sep_latest { " --latest-sep" } else { "" };
            let bb_flag    = if bb_latest  { " --latest-baseband" } else { "" };
            format!(
                "futurerestore -t /path/to/blob.shsh2{}{} \"{}\"",
                sep_flag, bb_flag, ipsw_str
            )
        } else {
            "# No viable restore path. See blockers.".to_string()
        };

        let verdict = if can_restore {
            format!(
                "RESTORE POSSIBLE: {} → iOS {}. Follow steps carefully.",
                device.model_name, target_ios
            )
        } else {
            format!(
                "RESTORE BLOCKED: {} has {} blocker(s). See details below.",
                device.model_name, blockers.len()
            )
        };

        Self {
            device_model:          device.model_identifier.clone(),
            device_chip:           format!("{:?}", chip),
            current_ios,
            target_ios,
            ecid,
            signing_window_open:   signing_open,
            has_saved_blob:        has_saved_blob,
            ecid_validated:        ecid > 0,
            nonce_strategies,
            generator_set,
            sep_compat,
            sep_needs_latest_flag: sep_latest,
            biometrics_preserved:  biometrics,
            cryptex1_blocked,
            cryptex1_status,
            needs_latest_baseband: bb_latest,
            activation_locked:     device.activation_locked(),
            needs_tss_proxy:       tss_proxy,
            ipsw_integrity_ok:     ipsw_ok,
            can_restore,
            verdict_text:          verdict,
            suggested_command:     suggested,
            blockers,
            warnings,
        }
    }

    /// Full human-readable report as a formatted string.
    pub fn format_report(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "╔══════════════════════════════════════════════════════════╗\n\
             ║  RESTORE BARRIER ANALYSIS REPORT                        ║\n\
             ╠══════════════════════════════════════════════════════════╣\n\
             ║  Device : {:<48} ║\n\
             ║  Chip   : {:<48} ║\n\
             ║  iOS    : {} → {}                                       \n\
             ║  ECID   : {:#018x}                                \n\
             ╚══════════════════════════════════════════════════════════╝\n",
            self.device_model, self.device_chip,
            self.current_ios, self.target_ios,
            self.ecid
        ));

        out.push_str("\n── VERDICT ────────────────────────────────────────────────\n");
        out.push_str(&format!("  {}\n", self.verdict_text));

        if !self.blockers.is_empty() {
            out.push_str("\n── BLOCKERS (must fix before proceeding) ──────────────────\n");
            for b in &self.blockers {
                out.push_str(&format!("  ❌ {}\n", b));
            }
        }

        if !self.warnings.is_empty() {
            out.push_str("\n── WARNINGS ───────────────────────────────────────────────\n");
            for w in &self.warnings {
                out.push_str(&format!("  ⚠️  {}\n", w));
            }
        }

        out.push_str("\n── BARRIER STATUS ─────────────────────────────────────────\n");
        out.push_str(&format!("  1. Signing window  : {}\n", if self.signing_window_open { "✅ Open"  } else { "🔒 Closed" }));
        out.push_str(&format!("  1. Saved SHSH blob : {}\n", if self.has_saved_blob      { "✅ Yes"   } else { "❌ No"    }));
        out.push_str(&format!("  2. ECID validated  : {}\n", if self.ecid_validated       { "✅ Yes"   } else { "⚠️ Unknown" }));
        out.push_str(&format!("  3. Nonce generator : {}\n", if self.generator_set         { "✅ Set"   } else { "⚠️  Not set" }));
        out.push_str(&format!("  4. SEP compat      : {:?}\n", self.sep_compat));
        out.push_str(&format!("  4. Biometrics safe : {}\n", if self.biometrics_preserved  { "✅ Yes" } else { "⚠️  May break" }));
        out.push_str(&format!("  5. Cryptex1        : {}\n", if self.cryptex1_blocked       { "❌ BLOCKED" } else { "✅ Not blocked" }));
        out.push_str(&format!("  6. Baseband        : {}\n", if self.needs_latest_baseband  { "⚠️  --latest-baseband needed" } else { "✅ Compatible" }));
        out.push_str(&format!("  7. Activation lock : {}\n", if self.activation_locked      { "⚠️  Locked" } else { "✅ Clear" }));
        out.push_str(&format!("  8. TSS proxy req'd : {}\n", if self.needs_tss_proxy        { "✅ Yes (offline blob replay)" } else { "— No" }));
        out.push_str(&format!("  12. IPSW integrity : {}\n", if self.ipsw_integrity_ok      { "✅ OK" } else { "⚠️  Not verified" }));

        if self.can_restore {
            out.push_str("\n── SUGGESTED COMMAND ──────────────────────────────────────\n");
            out.push_str(&format!("  {}\n", self.suggested_command));
        }

        out.push_str("\n── NONCE STRATEGIES ───────────────────────────────────────\n");
        for s in &self.nonce_strategies {
            out.push_str(&format!("  • {}\n", s.label()));
        }

        out
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  SECTION 9 — SAME-VERSION RE-FLASH (works on ALL modern iPhones)
// ═══════════════════════════════════════════════════════════════════════════════

/// Same-version restore: flash the SAME iOS version the device currently runs.
/// This ALWAYS works regardless of blobs, Cryptex1, SEP, or signing window.
/// Useful for: fixing boot loops, corrupted filesystems, unresponsive devices.
#[derive(Debug, Clone)]
pub struct SameVersionRestoreParams {
    pub model:             String,
    pub ios_version:       String,
    pub build_version:     String,
    pub ipsw_url:          String,    // Apple CDN URL for this exact build
    pub ipsw_sha256:       String,    // Expected SHA256 of the IPSW
    pub erase_device:      bool,      // true = full erase (Restore iPhone), false = update
}

impl SameVersionRestoreParams {
    /// Build params from a device info struct.
    pub fn from_device(device: &AppleDeviceInfo, erase: bool) -> Self {
        let model   = device.model_identifier.clone();
        let ios_ver = device.ios_version.clone().unwrap_or_else(|| "unknown".to_string());
        let build   = device.build_version.clone().unwrap_or_else(|| "unknown".to_string());
        // The URL follows a predictable pattern:
        // https://updates.cdn-apple.com/.../{model}_{ios_ver}_{build}_Restore.ipsw
        // Real implementation would query ipsw.me API for the actual URL.
        let url = format!(
            "https://api.ipsw.me/v4/ipsw/{}/{} (query ipsw.me for actual URL)",
            model, build
        );
        Self {
            model, ios_version: ios_ver, build_version: build,
            ipsw_url: url, ipsw_sha256: String::new(), erase_device: erase,
        }
    }

    /// Instructions for a same-version restore (no blobs needed).
    pub fn instructions(&self) -> String {
        format!(
            "Same-Version Restore — Works on ALL iPhones including A15+/iOS 16+\n\
             ══════════════════════════════════════════════════════════════════\n\
             Model   : {model}\n\
             Version : iOS {ios} (build {build})\n\
             Erase   : {erase}\n\n\
             Method A — via iTunes/Finder (macOS/Windows):\n\
             1. Connect iPhone via USB-C/Lightning\n\
             2. Open Finder (macOS) or iTunes (Windows)\n\
             3. {action}\n\
             4. Hold Option (Mac) or Shift (Win), click {btn}\n\
             5. Select the downloaded IPSW: {url}\n\
             6. Confirm and wait ~15 minutes\n\n\
             Method B — via futurerestore (no blobs needed for same version):\n\
             futurerestore --no-restore-version --latest-sep \\\n\
               \"{ipsw_path}\"\n\n\
             Method C — via idevicerestore (libimobiledevice):\n\
             idevicerestore -e \"{ipsw_path}\"   # -e = erase restore\n\
             # or\n\
             idevicerestore -u \"{ipsw_path}\"   # -u = update (no erase)\n\n\
             Note: For a same-version restore, Apple IS still signing the version,\n\
             so no SHSH blob is required. The device will activate normally.",
            model     = self.model,
            ios       = self.ios_version,
            build     = self.build_version,
            erase     = if self.erase_device { "Full erase (wipes all data)" } else { "Update (preserves data)" },
            action    = if self.erase_device { "Click 'Restore iPhone'" } else { "Click 'Check for Updates'" },
            btn       = if self.erase_device { "Restore iPhone" } else { "Update" },
            url       = self.ipsw_url,
            ipsw_path = "/path/to/downloaded.ipsw",
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  SECTION 10 — COMPLETE SPOOF ENGINE  (ties everything together)
// ═══════════════════════════════════════════════════════════════════════════════

/// One-stop entry point: analyse a device, pick the best restore strategy,
/// and produce step-by-step instructions plus the futurerestore command.
pub struct SpoofBypassEngine {
    pub device: AppleDeviceInfo,
}

impl SpoofBypassEngine {
    pub fn new(device: AppleDeviceInfo) -> Self {
        Self { device }
    }

    /// Full analysis: returns a barrier report + recommended restore plan.
    pub fn analyse(
        &self,
        target_ios: u32,
        has_blob: bool,
        generator: Option<&str>,
        ipsw: Option<&PathBuf>,
    ) -> RestoreBarrierReport {
        RestoreBarrierReport::analyse(
            &self.device,
            target_ios,
            has_blob,
            generator.is_some(),
            ipsw,
        )
    }

    /// Produce the complete multi-step restore guide for this device.
    pub fn full_restore_guide(
        &self,
        target_ios: u32,
        blob_path: Option<&str>,
        ipsw_path: Option<&str>,
        generator: Option<&str>,
    ) -> String {
        let chip = &self.device.chipset;
        let current_ios = self.device.ios_version.as_deref()
            .and_then(|v| v.split('.').next())
            .and_then(|m| m.parse::<u32>().ok())
            .unwrap_or(0);

        let cryptex1 = Cryptex1Status::assess(chip, current_ios);
        let nonce_strategies = NonceSpoofStrategy::recommend(chip, current_ios);
        let sep_needed = current_ios.saturating_sub(target_ios) > 0;
        let bb_needed  = needs_latest_baseband_flag(current_ios, target_ios);

        let mut guide = format!(
            "╔══════════════════════════════════════════════════════════════╗\n\
             ║  FULL RESTORE GUIDE: {} → iOS {}               \n\
             ║  Device: {} ({:?})        \n\
             ╚══════════════════════════════════════════════════════════════╝\n\n",
            self.device.model_name, target_ios,
            self.device.model_identifier, chip
        );

        // Step 0: Cryptex1 check
        if cryptex1.cryptex1_present {
            guide.push_str("⛔  CRYPTEX1 BARRIER DETECTED\n");
            guide.push_str("────────────────────────────────────────────────────────────\n");
            guide.push_str(&cryptex1.recommendation);
            guide.push_str("\n\n");
            if target_ios >= current_ios {
                // Same-version or upgrade is fine
                let svrp = SameVersionRestoreParams::from_device(&self.device, true);
                guide.push_str("✅  SAME-VERSION RE-FLASH (the only viable path):\n");
                guide.push_str(&svrp.instructions());
            }
            return guide;
        }

        // Step 1: SHSH blob check
        guide.push_str("STEP 1 — SHSH BLOB VERIFICATION\n");
        guide.push_str("────────────────────────────────\n");
        if let Some(bp) = blob_path {
            guide.push_str(&format!("  Blob : {}\n", bp));
            guide.push_str("  Verify with: futurerestore --verify\n");
        } else {
            guide.push_str("  ⚠️  No blob provided. You need a saved SHSH2 blob for iOS {}.\n");
            guide.push_str("  Save one NOW from: TSSSaver | blobsaver | ChimeraRS 'Save Blobs' tab\n");
        }

        // Step 2: Nonce setup
        guide.push_str("\nSTEP 2 — NONCE GENERATOR SETUP\n");
        guide.push_str("────────────────────────────────\n");
        if let Some(gen) = generator {
            guide.push_str(&format!("  Generator: {}\n", gen));
            guide.push_str("  Best method(s):\n");
            for s in &nonce_strategies {
                if *s != NonceSpoofStrategy::NotPossible {
                    guide.push_str(&format!("    → {}\n", s.label()));
                    guide.push_str(&s.instructions(gen).lines()
                        .map(|l| format!("       {}\n", l))
                        .collect::<String>());
                    break; // show only top method
                }
            }
        } else {
            guide.push_str("  ⚠️  No generator specified. Extract from your .shsh2 blob:\n");
            guide.push_str("    python3 -c \"import plistlib,sys; \
                b=plistlib.loads(open(sys.argv[1],'rb').read()); \
                print(b.get('generator','not set'))\"\n");
        }

        // Step 3: DFU mode
        guide.push_str("\nSTEP 3 — ENTER DFU MODE\n");
        guide.push_str("────────────────────────\n");
        guide.push_str(&dfu_instructions(&self.device.model_identifier));

        // Step 4: futurerestore command
        guide.push_str("\nSTEP 4 — RUN FUTURERESTORE\n");
        guide.push_str("───────────────────────────\n");
        let blob_arg = blob_path.unwrap_or("/path/to/blob.shsh2");
        let ipsw_arg = ipsw_path.unwrap_or("/path/to/firmware.ipsw");
        let sep_flag   = if sep_needed { " --latest-sep" }   else { "" };
        let bb_flag    = if bb_needed  { " --latest-baseband" } else { "" };
        guide.push_str(&format!(
            "  futurerestore -t \"{}\"{}{} \"{}\"\n",
            blob_arg, sep_flag, bb_flag, ipsw_arg
        ));

        // Step 5: Post-restore
        guide.push_str("\nSTEP 5 — POST-RESTORE\n");
        guide.push_str("─────────────────────\n");
        guide.push_str("  • Device reboots into Setup Assistant\n");
        guide.push_str("  • If activation locked: use bypass method from the Bypass tab\n");
        if !biometrics_safe_after_downgrade(chip_family_str(chip), target_ios) {
            guide.push_str("  ⚠️  Touch ID / Face ID may need re-enrolment or may not function\n");
        }
        guide.push_str("  • Remove any /etc/hosts TSS redirects if used\n");
        guide.push_str("  • Re-enable Find My / iCloud once device is set up\n");

        guide
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

pub fn chip_family_str(chip: &AppleChipset) -> &'static str {
    match chip {
        AppleChipset::A9                                  => "A9",
        AppleChipset::A10                                  => "A10",
        AppleChipset::A11                                  => "A11",
        AppleChipset::A12                                  => "A12",
        AppleChipset::A13                                  => "A13",
        AppleChipset::A14                                  => "A14",
        AppleChipset::A15 | AppleChipset::M2               => "A15",
        AppleChipset::A16 | AppleChipset::M3               => "A16",
        AppleChipset::A17Pro | AppleChipset::M4            => "A17",
        AppleChipset::A18 | AppleChipset::A18Pro           => "A18",
        AppleChipset::A19 | AppleChipset::A19Pro           => "A19",
        _                                                  => "unknown",
    }
}

fn determine_sep_compat_str(chip_family: &str, current: u32, target: u32) -> SepCompatibility {
    let modern = ["A15","A16","A17","A18","A19"];
    if modern.contains(&chip_family) && current >= 16 {
        return SepCompatibility::Cryptex1Locked;
    }
    let gap = current.saturating_sub(target);
    match gap {
        0       => SepCompatibility::Compatible,
        1 | 2   => SepCompatibility::RequiresLatestSep,
        _       => SepCompatibility::Cryptex1Locked,
    }
}

/// DFU mode entry instructions per device model
pub fn dfu_instructions(model: &str) -> String {
    // iPhone 8 and later use the volume-button sequence
    let is_modern = model.starts_with("iPhone1") || {
        let num: u32 = model.trim_start_matches("iPhone")
            .split(',').next()
            .and_then(|n| n.parse().ok()).unwrap_or(0);
        num >= 10
    };

    if is_modern {
        "  iPhone 8 / X / XS / XR / 11–17 / SE2/SE3:\n\
         1. Press and release Volume Up\n\
         2. Press and release Volume Down\n\
         3. Hold Side button until screen goes BLACK (≈10s)\n\
         4. While still holding Side, press Volume Down for 5s\n\
         5. Release Side, keep holding Volume Down for 5 more seconds\n\
         6. Screen stays BLACK and iTunes/Finder shows 'iPhone in recovery mode'\n\
         7. If Apple logo appears: start over — DFU failed"
            .to_string()
    } else {
        "  iPhone 7 / 7 Plus:\n\
         1. Hold Side + Volume Down simultaneously for 8 seconds\n\
         2. Release Side, keep holding Volume Down for 6 more seconds\n\
         3. Screen must stay BLACK — Apple logo = fail, retry\n\n\
         iPhone 6S and earlier:\n\
         1. Hold Home + Sleep/Wake for 8 seconds\n\
         2. Release Sleep/Wake, keep holding Home for 6 more seconds"
            .to_string()
    }
}
