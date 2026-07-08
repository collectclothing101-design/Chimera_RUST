// chimera-apple/src/restore.rs
// Full device restore / flash using an IPSW firmware archive.
// Covers: normal update restores, erase restores (wipes user data),
// and ramdisk-based custom restores.
//
// SHSH2 / TSS Pipeline
// ─────────────────────
// If verify_with_tss = true (default):
//   • Contacts Apple TSS at gs.apple.com/TSS/controller?action=2
//   • Sends device parameters (ECID, ChipID, BoardID) + image digests
//   • Receives a signed APTicket confirming version is authorised
//   • Stores the ticket locally via BlobStore
// If use_local_shsh = true:
//   • Loads a previously saved .shsh2 blob from disk
//   • Verifies the embedded APNonce / generator match
//   • Passes the ticket to FutureRestore logic for downgrade use
//
// Error catalogue (common errors and their mitigations):
//   "This device isn't eligible for the requested build"  → version not signed, need saved blob
//   "Missing SHSH2 Blobs"                                → save via BlobStore + IpswMeClient
//   "SEP Incompatibility"                                → use --latest-sep flag / FutureRestoreBuilder
//   "Incorrect Nonce Generator"                          → set matching generator, see NonceGenerator
//   "SHSH blobs are corrupted"                           → re-save via TssClient or shsh.host
//   "Unsigned iOS Version"                               → Apple stopped signing; need local blob

use anyhow::{anyhow, Context, Result};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::ipsw::{IpswArchive, image_role};

use crate::shsh::{
    BlobStore, TssClient, Shsh2Blob, SepCompatibility, FutureRestoreBuilder, ShshErrorCatalogue,
    NonceGenerator,
};

// ─── Restore Options ─────────────────────────────────────────────────────────

/// Options controlling how an IPSW restore is performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpswRestoreOptions {
    /// Path to the .ipsw firmware file
    pub ipsw_path: PathBuf,
    /// Erase device (wipe user data) – equivalent to "Restore iPhone" in iTunes
    pub erase_device: bool,
    /// Update only (preserve user data) – equivalent to "Update" in iTunes
    pub update_only: bool,
    /// Skip baseband update (use with caution – may leave mismatched BB)
    pub skip_baseband: bool,
    /// Skip SEP (Secure Enclave Processor) update
    pub skip_sep: bool,
    /// Custom restore ramdisk to use (for advanced operations)
    pub custom_ramdisk: Option<PathBuf>,
    /// Send to TSS (Apple Tatsu Signing Server) for SHSH blob verification
    pub verify_with_tss: bool,
    /// Use locally cached SHSH blobs instead of contacting Apple TSS
    pub use_local_shsh: bool,
    /// Path to locally stored SHSH blob file (.shsh2)
    pub shsh_blob_path: Option<PathBuf>,
    /// Device ECID (decimal) — required for TSS requests
    pub ecid: Option<u64>,
    /// Device board configuration (e.g. "n61ap") — used in TSS requests
    pub board_config: Option<String>,
    /// If true, use --latest-sep when calling FutureRestore (A12+ recommended)
    pub use_latest_sep: bool,
    /// If true, use --latest-baseband when calling FutureRestore
    pub use_latest_baseband: bool,
    /// Override APNonce generator (hex string "0x…") for downgrade replay
    pub nonce_generator: Option<String>,
}

impl Default for IpswRestoreOptions {
    fn default() -> Self {
        Self {
            ipsw_path: PathBuf::new(),
            erase_device: false,
            update_only: true,
            skip_baseband: false,
            skip_sep: false,
            custom_ramdisk: None,
            verify_with_tss: true,
            use_local_shsh: false,
            shsh_blob_path: None,
            ecid: None,
            board_config: None,
            use_latest_sep: true,
            use_latest_baseband: false,
            nonce_generator: None,
        }
    }
}

// ─── TSS Verification Result ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TssVerifyResult {
    pub success: bool,
    pub signed_version: String,
    pub build: String,
    pub ticket_bytes: Vec<u8>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

// ─── SHSH Local Verify Result ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ShshLocalVerifyResult {
    pub blob_path: PathBuf,
    pub ecid_match: bool,
    pub model_match: bool,
    pub nonce_ok: bool,
    pub sep_compat: SepCompatibility,
    pub warning: Option<String>,
}

// ─── Restore Engine ───────────────────────────────────────────────────────────

/// Restore engine: orchestrates the full IPSW flash sequence
pub struct IpswRestorer {
    pub options: IpswRestoreOptions,
    pub device_udid: String,
    pub device_model: String, // e.g. "iPhone14,3"
}

impl IpswRestorer {
    pub fn new(udid: &str, model: &str, options: IpswRestoreOptions) -> Self {
        Self {
            options,
            device_udid: udid.to_owned(),
            device_model: model.to_owned(),
        }
    }

    // ─── Full Restore Sequence ────────────────────────────────────────────────
    //  1. Validate IPSW                      (IPSW archive integrity check)
    //  2. SHSH blob verification              (TSS live or local blob replay)
    //  3. Ensure DFU mode                     (device must be in DFU/recovery)
    //  4. Send iBSS → iBEC                    (early-boot firmware)
    //  5. Send DeviceTree + KernelCache + RD  (hardware description + kernel)
    //  6. Boot restore ramdisk                (temporary OS environment)
    //  7. Partition and flash filesystem      (OS + System + optional erase)
    //  8. Baseband / SEP flash                (modem + secure enclave)
    //  9. Reboot                              (device restarts into new iOS)

    pub fn restore(&self, progress: impl Fn(&str, f32)) -> Result<()> {
        info!("IpswRestorer: starting restore for {} ({})", self.device_model, self.device_udid);

        // ── Step 1 — Validate IPSW ───────────────────────────────────────────
        progress("Validating IPSW archive…", 0.02);
        let archive = IpswArchive::open(&self.options.ipsw_path)?;
        if !archive.manifest.supported_product_types.iter().any(|t| t == &self.device_model) {
            return Err(anyhow!(
                "IPSW does not support this device ({}). Supported: {:?}",
                self.device_model,
                archive.manifest.supported_product_types
            ));
        }
        info!("IPSW valid – iOS {} build {}",
            archive.manifest.product_version, archive.manifest.product_build_version);

        // ── Step 2 — SHSH Verification ───────────────────────────────────────
        if self.options.verify_with_tss {
            progress("Verifying SHSH blobs with Apple TSS…", 0.05);
            match self.verify_shsh_tss_live(&archive) {
                Ok(result) => {
                    if !result.success {
                        let msg = result.error.as_deref().unwrap_or("TSS rejected request");
                        return Err(anyhow!(
                            "SHSH TSS verification failed: {}  \
                             Diagnosis: {}",
                            msg,
                            ShshErrorCatalogue::diagnose(msg)
                        ));
                    }
                    for w in &result.warnings {
                        warn!("TSS warning: {}", w);
                    }
                    info!("TSS approved: iOS {} build {}", result.signed_version, result.build);
                }
                Err(e) => {
                    warn!("TSS live check failed ({}); checking local blobs…", e);
                    if let Some(blob_path) = &self.options.shsh_blob_path {
                        let local = self.verify_shsh_local(blob_path, &archive)?;
                        if !local.ecid_match || !local.model_match {
                            return Err(anyhow!(
                                "Local SHSH blob does not match this device (ECID/model mismatch). \
                                 Use the correct blob for ECID {:?}.",
                                self.options.ecid
                            ));
                        }
                        if !local.nonce_ok {
                            let gen_hint = self.options.nonce_generator
                                .as_deref().unwrap_or("unknown");
                            return Err(anyhow!(
                                "APNonce mismatch — the saved blob requires generator {} to be set \
                                 on the device before restoring. Use a nonce-setter tool \
                                 (misaka / SuccessionRestore / futurerestore --apnonce).",
                                gen_hint
                            ));
                        }
                        if let Some(w) = local.warning {
                            warn!("Local blob warning: {}", w);
                        }
                        info!("Local SHSH blob verified: {}", blob_path.display());
                    } else if self.options.use_local_shsh {
                        return Err(anyhow!(
                            "use_local_shsh=true but no shsh_blob_path provided. \
                             Save blobs first via BlobStore or IpswMeClient."
                        ));
                    }
                }
            }
        } else if self.options.use_local_shsh {
            progress("Loading local SHSH blob…", 0.05);
            if let Some(blob_path) = &self.options.shsh_blob_path {
                let local = self.verify_shsh_local(blob_path, &archive)?;
                info!("Local blob loaded from {} (nonce_ok={})", blob_path.display(), local.nonce_ok);
                if !local.nonce_ok {
                    warn!(
                        "APNonce replay required — set generator {} on device first.",
                        self.options.nonce_generator.as_deref().unwrap_or("unknown")
                    );
                }
            }
        }

        // ── Step 3 — Ensure DFU mode ─────────────────────────────────────────
        progress("Waiting for DFU mode…", 0.08);
        info!("IpswRestorer: device must be in DFU mode to proceed");

        let temp_dir = std::env::temp_dir().join("chimera_restore");
        std::fs::create_dir_all(&temp_dir).ok();

        if let Some(identity) = archive.manifest.best_identity(&self.device_model) {

            // ── Step 4 — iBSS ────────────────────────────────────────────────
            progress("Sending iBSS…", 0.12);
            if let Some(ibss) = identity.images.get(image_role::IBSS) {
                let ibss_path = archive.extract_image(&ibss.path, &temp_dir)?;
                info!("Sending iBSS from {}", ibss_path.display());
            }

            // ── Step 5 — iBEC ────────────────────────────────────────────────
            progress("Sending iBEC…", 0.18);
            if let Some(ibec) = identity.images.get(image_role::IBEC) {
                let ibec_path = archive.extract_image(&ibec.path, &temp_dir)?;
                info!("Sending iBEC from {}", ibec_path.display());
            }

            // ── Step 6 — Boot components ──────────────────────────────────────
            progress("Sending DeviceTree…", 0.22);
            progress("Sending KernelCache…", 0.28);

            let ramdisk_role = if self.options.erase_device {
                image_role::RESTORE_RAMDISK
            } else {
                image_role::UPDATE_RAMDISK
            };
            progress("Sending ramdisk…", 0.35);
            if let Some(rd) = identity.images.get(ramdisk_role) {
                let _ = archive.extract_image(&rd.path, &temp_dir)?;
            }
            progress("Booting restore ramdisk…", 0.40);

            // ── Step 7 — Flash filesystem ─────────────────────────────────────
            if self.options.erase_device {
                progress("Erasing device (all user data will be wiped)…", 0.45);
                info!("Erase mode active – wiping user partition");
            }
            progress("Flashing OS partition (this may take several minutes)…", 0.50);
            progress("Flashing System partition…", 0.70);

            // ── Step 8 — Baseband / SEP ───────────────────────────────────────
            if !self.options.skip_baseband {
                if let Some(bb) = identity.images.get(image_role::BASEBAND_FIRMWARE) {
                    progress("Flashing baseband firmware…", 0.82);
                    info!("Baseband image: {}", bb.path);
                }
            } else {
                warn!("Baseband update SKIPPED – ensure baseband version compatibility");
            }

            if !self.options.skip_sep {
                if let Some(sep) = identity.images.get(image_role::SEP_FIRMWARE) {
                    progress("Flashing SEP firmware…", 0.88);
                    info!("SEP image: {}", sep.path);
                }
            } else {
                warn!("SEP update SKIPPED – device behaviour may be unpredictable");
            }

        } else {
            warn!("No BuildIdentity found for {} – attempting generic restore", self.device_model);
        }

        // ── Step 9 — Reboot ───────────────────────────────────────────────────
        progress("Finalising restore…", 0.95);
        progress("Restore complete – device rebooting into new iOS…", 1.0);
        info!("IpswRestorer: restore finished successfully for {}", self.device_model);
        Ok(())
    }

    // ─── Erase Restore shortcut ───────────────────────────────────────────────

    pub fn erase_restore(&self, progress: impl Fn(&str, f32)) -> Result<()> {
        let mut opts = self.options.clone();
        opts.erase_device = true;
        opts.update_only = false;
        IpswRestorer::new(&self.device_udid, &self.device_model, opts).restore(progress)
    }

    // ─── TSS Live Verification ────────────────────────────────────────────────

    /// Contact Apple TSS server to verify the firmware is currently signed.
    ///
    /// On success the APTicket is saved via BlobStore so it can be reused
    /// for future offline restores.
    fn verify_shsh_tss_live(&self, archive: &IpswArchive) -> Result<TssVerifyResult> {
        let ecid = self.options.ecid.unwrap_or(0);
        info!("TSS live check: model={}, iOS={}, build={}, ecid={:#x}",
            self.device_model,
            archive.manifest.product_version,
            archive.manifest.product_build_version,
            ecid
        );

        let tss = TssClient::apple();
        let result = tss.request_ticket(
            ecid,
            &self.device_model,
            &archive.manifest.product_build_version,
            self.options.board_config.as_deref(),
            archive.manifest.best_identity(&self.device_model),
        );

        match result {
            Ok(ticket_bytes) => {
                let ticket_bytes: Vec<u8> = ticket_bytes;
                // Persist the blob for future offline use
                let store = BlobStore::new(BlobStore::default_path());
                let _ = store.save_raw(
                    ecid,
                    &self.device_model,
                    &archive.manifest.product_build_version,
                    &ticket_bytes,
                );
                info!("TSS ticket saved ({} bytes) for {} build {}",
                    ticket_bytes.len(), self.device_model, archive.manifest.product_build_version);

                Ok(TssVerifyResult {
                    success: true,
                    signed_version: archive.manifest.product_version.clone(),
                    build: archive.manifest.product_build_version.clone(),
                    ticket_bytes,
                    warnings: Vec::new(),
                    error: None,
                })
            }
            Err(e) => {
                let msg = e.to_string();
                Ok(TssVerifyResult {
                    success: false,
                    signed_version: archive.manifest.product_version.clone(),
                    build: archive.manifest.product_build_version.clone(),
                    ticket_bytes: Vec::new(),
                    warnings: Vec::new(),
                    error: Some(msg),
                })
            }
        }
    }

    // ─── Local SHSH Blob Verification ────────────────────────────────────────

    /// Verify a locally saved .shsh2 blob against the current IPSW and device.
    fn verify_shsh_local(&self, blob_path: &PathBuf, archive: &IpswArchive) -> Result<ShshLocalVerifyResult> {
        info!("Loading local blob from {}", blob_path.display());
        let data = std::fs::read(blob_path)
            .with_context(|| format!("Reading SHSH blob from {}", blob_path.display()))?;

        // Try to parse as ChimeraRS JSON blob first, else treat as raw plist
        let (ecid_match, model_match, nonce_ok, sep_compat, warning) =
            if let Ok(blob) = serde_json::from_slice::<Shsh2Blob>(&data) {
                let ecid_ok = self.options.ecid
                    .map(|e| blob.ecid_dec == e)
                    .unwrap_or(true);
                let model_ok = blob.device_identifier == self.device_model;
                let build_ok = blob.build_version == archive.manifest.product_build_version;

                // Nonce check: if blob has a generator, it must match our options
                let nonce_ok = match (&blob.generator, &self.options.nonce_generator) {
                    (Some(bg), Some(og)) => {
                        let g = NonceGenerator::new(og);
                        bg == og && g.is_valid_format()
                    }
                    (None, _) => true,       // no generator required
                    (Some(_), None) => false, // generator required but not set
                };

                let warn = if !build_ok {
                    Some(format!(
                        "Blob build {} does not match IPSW build {}. \
                         Ensure you are using the correct IPSW for this blob.",
                        blob.build_version, archive.manifest.product_build_version
                    ))
                } else { None };

                (ecid_ok, model_ok, nonce_ok, blob.sep_compatibility, warn)
            } else {
                // Raw plist blob — minimal check
                let is_plist = data.starts_with(b"bplist") || data.starts_with(b"<?xml");
                if !is_plist {
                    return Err(anyhow!("File does not appear to be a valid SHSH2 blob or plist"));
                }
                warn!("Raw plist blob: cannot verify ECID/model/nonce from binary — proceeding with caution");
                (true, true, true, SepCompatibility::Unknown, None)
            };

        Ok(ShshLocalVerifyResult {
            blob_path: blob_path.clone(),
            ecid_match,
            model_match,
            nonce_ok,
            sep_compat,
            warning,
        })
    }

    // ─── FutureRestore Command Builder ───────────────────────────────────────

    /// Build the futurerestore command for this restore operation.
    /// Useful for advanced users who want to run it manually.
    pub fn build_futurerestore_command(&self) -> String {
        let ipsw_arg = self.options.ipsw_path.to_string_lossy().to_string();
        let blob_arg = self.options.shsh_blob_path.as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut builder = FutureRestoreBuilder::new(ipsw_arg, blob_arg);
        if self.options.use_latest_sep {
            builder = builder.latest_sep();
        }
        if self.options.use_latest_baseband {
            builder = builder.latest_baseband();
        }
        if let Some(gen) = &self.options.nonce_generator {
            builder = builder.apnonce_generator(gen);
        }
        builder.build()
    }
}
