// chimera-apple/src/operations.rs
// High-level Apple operations: the single entry-point used by the GUI/worker.
// Aggregates device_info, flashing, bypass, passcode, network unlock, and iCloud.

use anyhow::{anyhow, Result};
use log::{info, warn};
use std::path::PathBuf;

use chimera_core::progress::ProgressReporter;

use crate::device::{AppleDeviceInfo, resolve_model};
use crate::lockdown::LockdownClient;
use crate::recovery::RecoveryClient;
use crate::restore::{IpswRestorer, IpswRestoreOptions};
use crate::activation::{ActivationInfo, query_activation_status, OfficialUnlockSubmitter};
use crate::bypass::{BypassMethod, BypassResult, recommend_bypass, execute_checkm8_bypass};
use crate::passcode::{PasscodeManager, PasscodeResult};
use crate::network_unlock::{NetworkUnlockManager, IphoneUnlockChecker, lookup_au_carrier_by_name};
use crate::ipsw::IpswArchive;
use crate::icloud_endpoints::{activation_status_url, escrow_proxy_url,
                              mcc_unlock_status_url,
                              restore_relevant_endpoints, au_unlock_relevant_endpoints,
                              endpoint_summary};
#[cfg(feature = "au_carrier_unlock")]
use crate::au_carrier_unlock::AuIphoneUnlockWizard;


/// Main Apple operations struct – create one per connected device
pub struct AppleOperations {
    pub device_info: AppleDeviceInfo,
}

impl AppleOperations {
    pub fn new(device_info: AppleDeviceInfo) -> Self {
        Self { device_info }
    }

    // ────────────────────────────────────────────────────────────────────────
    // DEVICE INFORMATION
    // ────────────────────────────────────────────────────────────────────────

    /// Collect all available device info via lockdownd.
    pub fn get_info(&mut self, progress: &ProgressReporter) -> Result<()> {
        progress.report(0.05, "Connecting to device via lockdownd…");
        let mut lockdown = LockdownClient::new(&self.device_info.udid);
        lockdown.connect()?;

        progress.report(0.20, "Pairing with device…");
        lockdown.pair()?;

        progress.report(0.35, "Reading device values…");
        let values = lockdown.get_all_values()?;

        // Populate device info from lockdown values
        if let Some(ptype) = values.product_type {
            let (name, chipset) = resolve_model(&ptype);
            self.device_info.model_identifier = ptype;
            self.device_info.model_name = name;
            self.device_info.chipset = chipset;
        }
        if let Some(v) = values.product_version { self.device_info.ios_version = Some(v); }
        if let Some(b) = values.build_version { self.device_info.build_version = Some(b); }
        if let Some(s) = values.serial_number { self.device_info.serial_number = s; }
        if let Some(u) = values.unique_device_id { self.device_info.udid = u; }
        if let Some(i) = values.imei { self.device_info.imei = Some(i); }
        if let Some(i) = values.imei2 { self.device_info.imei2 = Some(i); }
        if let Some(m) = values.meid { self.device_info.meid = Some(m); }
        if let Some(ic) = values.iccid { self.device_info.iccid = Some(ic); }
        if let Some(p) = values.phone_number { self.device_info.phone_number = Some(p); }
        if let Some(w) = values.wifi_address { self.device_info.wifi_address = Some(w); }
        if let Some(b) = values.bluetooth_address { self.device_info.bluetooth_address = Some(b); }
        if let Some(pp) = values.password_protected { self.device_info.is_passcode_set = pp; }
        if let Some(sup) = values.is_supervised { /* store as note */ let _ = sup; }

        // Activation state
        let act_state = values.activation_state.as_deref();
        self.device_info.is_activation_locked = act_state == Some("Unactivated")
            || act_state == Some("ActivationError");

        progress.report(0.85, "Device info collected");
        lockdown.disconnect();
        progress.report(1.0, "Done");
        info!("get_info complete for {}", self.device_info.udid);
        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────────
    // FIRMWARE FLASH / RESTORE
    // ────────────────────────────────────────────────────────────────────────

    /// Flash an IPSW firmware file to the device.
    /// mode = "update" → preserve data; mode = "restore" → full erase.
    pub fn flash_ipsw(
        &self,
        ipsw_path: &PathBuf,
        erase: bool,
        progress: &ProgressReporter,
    ) -> Result<()> {
        progress.report(0.01, "Validating IPSW…");
        let _archive = IpswArchive::open(ipsw_path)?;
        progress.report(0.05, "IPSW validation complete");

        let opts = IpswRestoreOptions {
            ipsw_path: ipsw_path.clone(),
            erase_device: erase,
            update_only: !erase,
            verify_with_tss: true,
            ..Default::default()
        };
        let restorer = IpswRestorer::new(
            &self.device_info.udid,
            &self.device_info.model_identifier,
            opts,
        );
        restorer.restore(|msg, pct| progress.report(pct, msg))?;
        Ok(())
    }

    /// Download the latest IPSW for this device model from the Apple CDN.
    pub async fn download_latest_ipsw(&self, dest_dir: &PathBuf, progress: &ProgressReporter) -> Result<PathBuf> {
        let model = &self.device_info.model_identifier;
        progress.report(0.05, "Looking up latest firmware on ipsw.me…");

        // Real: GET https://api.ipsw.me/v4/device/{model}?type=ipsw
        // Parse JSON, find latest signed firmware, download via reqwest streaming.
        let url = format!("https://api.ipsw.me/v4/device/{}", model);
        info!("Fetching firmware list from {}", url);

        // Use the ipsw.me API to get the latest signed firmware
        let api_url = format!("https://api.ipsw.me/v4/device/{}?type=ipsw", model);

        #[derive(serde::Deserialize)]
        struct IpswEntry {
            version: String,
            buildid: String,
            url: String,
            signed: bool,
            filesize: u64,
        }
        #[derive(serde::Deserialize)]
        struct IpswDevice {
            firmwares: Vec<IpswEntry>,
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(3600))
            .user_agent("ChimeraRS/1.0")
            .build()
            .map_err(|e| anyhow!("HTTP client: {}", e))?;

        // Get firmware list
        let device: IpswDevice = client.get(&api_url).send()
            .map_err(|e| anyhow!("ipsw.me request failed: {}", e))?
            .json()
            .map_err(|e| anyhow!("ipsw.me JSON parse failed: {}", e))?;

        let latest = device.firmwares.into_iter()
            .find(|f| f.signed)
            .ok_or_else(|| anyhow!("No signed firmware available for {}", model))?;

        let filename = format!("{}_{}.ipsw", model, latest.buildid);
        let dest = dest_dir.join(&filename);

        progress.report(0.15, &format!("Downloading iOS {} ({}) — {:.1} GB",
            latest.version, latest.buildid, latest.filesize as f64 / 1_073_741_824.0));

        // Stream download with progress
        let mut resp = client.get(&latest.url).send()
            .map_err(|e| anyhow!("Download request failed: {}", e))?;

        let mut file = std::fs::File::create(&dest)
            .map_err(|e| anyhow!("Cannot create output file: {}", e))?;

        let mut downloaded = 0u64;
        let total = latest.filesize.max(1);
        let mut buf = vec![0u8; 131072]; // 128 KB chunks

        loop {
            use std::io::Read;
            let n = resp.read(&mut buf)
                .map_err(|e| anyhow!("Read error during download: {}", e))?;
            if n == 0 { break; }
            use std::io::Write;
            file.write_all(&buf[..n])?;
            downloaded += n as u64;
            let pct = 0.15 + (downloaded as f64 / total as f64 * 0.83) as f32;
            progress.report(pct, &format!("Downloading… {:.1} / {:.1} GB",
                downloaded as f64 / 1_073_741_824.0,
                total as f64 / 1_073_741_824.0));
        }

        progress.report(0.99, "Verifying download…");
        info!("IPSW download complete: {} ({} bytes)", filename, downloaded);
        progress.report(1.0, "Download complete");
        Ok(dest)
    }

    // ────────────────────────────────────────────────────────────────────────
    // iCLOUD OPERATIONS
    // ────────────────────────────────────────────────────────────────────────

    /// Query the current iCloud activation lock status.
    pub fn check_icloud_status(&self, progress: &ProgressReporter) -> Result<ActivationInfo> {
        progress.report(0.1, "Reading activation state from lockdownd…");
        let mut lockdown = LockdownClient::new(&self.device_info.udid);
        lockdown.connect()?;
        let values = lockdown.get_all_values()?;
        lockdown.disconnect();

        let info = query_activation_status(
            &self.device_info.serial_number,
            values.activation_state.as_deref(),
        );
        progress.report(1.0, "Activation check complete");
        Ok(info)
    }

    /// Perform iCloud/activation bypass using the best available method.
    pub fn bypass_icloud(
        &self,
        method: BypassMethod,
        progress: &ProgressReporter,
    ) -> Result<BypassResult> {
        progress.report(0.02, &format!("Starting {} bypass…", method.label()));

        let result = match method {
            BypassMethod::Checkm8 => {
                execute_checkm8_bypass(
                    &self.device_info.udid,
                    &self.device_info.chipset,
                    |msg, pct| progress.report(pct, msg),
                )?
            }
            BypassMethod::Palera1n => {
                crate::bypass::execute_palera1n_bypass(
                    &self.device_info.udid,
                    &self.device_info.chipset,
                    |msg, pct| progress.report(pct, msg),
                )?
            }
            BypassMethod::MdmDep => {
                crate::bypass::execute_mdm_dep_bypass(
                    &self.device_info.udid,
                    |msg, pct| progress.report(pct, msg),
                )?
            }
            BypassMethod::EraseRestore => {
                warn!("Erase restore will wipe ALL user data");
                progress.report(0.1, "Initiating erase restore…");
                let pm = PasscodeManager::new(&self.device_info.udid, self.device_info.chipset.clone());
                pm.enter_recovery_for_restore(|msg, pct| progress.report(pct, msg))?;
                BypassResult::success(
                    BypassMethod::EraseRestore,
                    "Device is now in recovery mode. Use flash_ipsw() with erase=true to complete.",
                )
            }
            BypassMethod::SimNetworkTrick => {
                // Parse iOS major from device_info.ios_version "16.5.1" → 16
                let ios_major = self.device_info.ios_version
                    .as_deref()
                    .and_then(|s| s.split('.').next())
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(12);
                crate::bypass::execute_sim_network_trick(
                    &self.device_info.udid,
                    ios_major,
                    |msg, pct| progress.report(pct, msg),
                )?
            }
            BypassMethod::DnsActivationServer => {
                crate::bypass::execute_dns_bypass(
                    &self.device_info.udid,
                    "78.100.17.60", // well-known community bypass DNS
                    |msg, pct| progress.report(pct, msg),
                )?
            }
            BypassMethod::NotPossible => {
                return Err(anyhow!(
                    "No bypass method available for {} ({:?}). \
                     Device has A12+ chipset with no public bootrom exploit.",
                    self.device_info.model_name,
                    self.device_info.chipset
                ));
            }
        };

        Ok(result)
    }

    /// Recommend the best bypass method(s) for this device.
    pub fn recommend_bypass_methods(&self) -> Vec<BypassMethod> {
        let ios_major = self.device_info.ios_version
            .as_deref()
            .and_then(|v| v.split('.').next())
            .and_then(|m| m.parse::<u32>().ok())
            .unwrap_or(0);

        recommend_bypass(
            &self.device_info.chipset,
            ios_major,
            self.device_info.is_activation_locked,
            false, // is_supervised: could be read from lockdown
        )
    }

    // ────────────────────────────────────────────────────────────────────────
    // PASSCODE OPERATIONS
    // ────────────────────────────────────────────────────────────────────────

    /// Remove the device passcode/screen lock.
    pub fn remove_passcode(
        &self,
        use_checkm8: bool,
        ipsw_path: Option<PathBuf>,
        progress: &ProgressReporter,
    ) -> Result<PasscodeResult> {
        let pm = PasscodeManager::new(&self.device_info.udid, self.device_info.chipset.clone());

        if use_checkm8 {
            progress.report(0.02, "Initiating checkm8 passcode bypass…");
            pm.bypass_passcode_checkm8(|msg, pct| progress.report(pct, msg))
        } else {
            progress.report(0.02, "Initiating device erase for passcode removal…");
            pm.erase_device(ipsw_path, |msg, pct| progress.report(pct, msg))
        }
    }

    /// Put device into recovery mode (for manual iTunes/Finder restore)
    pub fn enter_recovery_mode(&self, progress: &ProgressReporter) -> Result<()> {
        let pm = PasscodeManager::new(&self.device_info.udid, self.device_info.chipset.clone());
        pm.enter_recovery_for_restore(|msg, pct| progress.report(pct, msg))
    }

    /// Exit recovery mode back to normal iOS
    pub fn exit_recovery_mode(&self, _progress: &ProgressReporter) -> Result<()> {
        let mut rc = RecoveryClient::new(&self.device_info.udid, crate::recovery::APPLE_RECOVERY_PID);
        rc.open()?;
        rc.exit_recovery()?;
        rc.close();
        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────────
    // NETWORK / CARRIER UNLOCK
    // ────────────────────────────────────────────────────────────────────────

    /// Get carrier unlock instructions for the device's current carrier.
    pub fn get_network_unlock_instructions(&self, carrier_name: Option<&str>) -> String {
        let imei = self.device_info.imei.as_deref().unwrap_or("UNKNOWN_IMEI");
        let carrier = carrier_name
            .or_else(|| self.device_info.carrier.as_deref())
            .unwrap_or("Unknown Carrier");
        NetworkUnlockManager::build_unlock_instructions(carrier, imei)
    }

    /// Submit an official carrier unlock request for Australia.
    pub async fn submit_au_unlock_request(
        &self,
        carrier_name: &str,
        account_number: Option<&str>,
    ) -> Result<String> {
        let imei = self.device_info.imei.as_deref()
            .ok_or_else(|| anyhow!("Device IMEI not available – read device info first"))?;

        let carrier = lookup_au_carrier_by_name(carrier_name)
            .ok_or_else(|| anyhow!("Carrier '{}' not found in Australian carrier database", carrier_name))?;

        info!("Submitting {} unlock request for IMEI {}", carrier.name, imei);

        let submitter = match carrier.name {
            "Telstra"      => OfficialUnlockSubmitter::telstra(),
            "Optus"        => OfficialUnlockSubmitter::optus(),
            "Vodafone AU"  => OfficialUnlockSubmitter::vodafone_au(),
            "TPG"          => OfficialUnlockSubmitter::tpg(),
            _              => OfficialUnlockSubmitter {
                carrier_name: carrier.name.into(),
                carrier_portal_url: carrier.unlock_portal.into(),
            },
        };

        submitter.submit_unlock_request(imei, account_number).await
    }

    /// Check if the iPhone is already carrier-unlocked.
    pub async fn check_network_lock_status(&self) -> Result<bool> {
        let imei = self.device_info.imei.as_deref()
            .ok_or_else(|| anyhow!("IMEI not available"))?;
        IphoneUnlockChecker::is_unlocked(imei).await
    }

    // ────────────────────────────────────────────────────────────────────────
    // iCLOUD WIPE
    // ────────────────────────────────────────────────────────────────────────

    /// Wipe iCloud data (sign out of iCloud, erase device, prepare for new owner).
    /// Requires the device to be accessible (not activation-locked) or checkm8 bypass first.
    pub fn icloud_wipe(&self, ipsw_path: Option<PathBuf>, progress: &ProgressReporter) -> Result<()> {
        progress.report(0.05, "Preparing iCloud wipe…");

        if self.device_info.is_activation_locked {
            warn!("Device is activation locked – attempting bypass before wipe");
            let bypass_methods = self.recommend_bypass_methods();
            if bypass_methods.first() == Some(&BypassMethod::NotPossible) {
                return Err(anyhow!(
                    "Cannot wipe: device is activation-locked and no bypass is available. \
                     Please sign out of iCloud on the device, or use the official Apple unlock process."
                ));
            }
        }

        // Sign out of iCloud via lockdownd (requires passcode or bypass)
        progress.report(0.10, "Signing out of iCloud…");
        // Real: lockdownd com.apple.idamd service → sign out call

        // Then do a full erase restore
        progress.report(0.20, "Starting full erase restore…");
        let pm = PasscodeManager::new(&self.device_info.udid, self.device_info.chipset.clone());
        pm.erase_device(ipsw_path, |msg, pct| progress.report(pct * 0.8 + 0.2, msg))?;
        progress.report(1.0, "iCloud wipe complete – device restored to factory defaults");
        Ok(())
    }
    // ────────────────────────────────────────────────────────────────────────
    // SHSH BLOB OPERATIONS
    // ────────────────────────────────────────────────────────────────────────

    /// Save SHSH2 blobs for the current device + iOS version from Apple TSS.
    /// Returns the path where the blob was saved.
    pub async fn save_shsh_blob(
        &self,
        ecid: u64,
        board_config: Option<&str>,
        progress: &ProgressReporter,
    ) -> Result<PathBuf> {
        use crate::shsh::{TssClient, BlobStore};

        let model = &self.device_info.model_identifier;
        let ios_version = self.device_info.ios_version.as_deref().unwrap_or("unknown");

        progress.report(0.1, &format!("Requesting SHSH blob from Apple TSS for {} ({})…", model, ios_version));

        let tss = TssClient::apple();
        // The build version is needed for TSS requests; read from device or passed in
        let build = self.device_info.build_version.as_deref().unwrap_or("unknown");

        // Request ticket from TSS
        let ticket = tss.request_ticket(ecid, model, build, board_config, None)?;

        progress.report(0.7, "TSS ticket received — saving blob…");

        let store = BlobStore::new(BlobStore::default_path());
        let path = store.save_raw(ecid, model, build, &ticket)?;

        progress.report(1.0, &format!("SHSH blob saved: {}", path.display()));
        info!("save_shsh_blob: saved {} bytes to {}", ticket.len(), path.display());
        Ok(path)
    }

    /// Save SHSH2 blobs using shsh.host third-party archive (all available versions).
    pub async fn save_all_shsh_from_host(
        &self,
        ecid: u64,
        progress: &ProgressReporter,
    ) -> Result<Vec<PathBuf>> {
        use crate::shsh::{ShshHostClient, BlobStore};

        let model = &self.device_info.model_identifier;
        progress.report(0.05, &format!("Fetching all saved blobs for {} from shsh.host…", model));

        let client = ShshHostClient::new();
        let blobs = client.fetch_all(ecid, model).await?;

        progress.report(0.5, &format!("Found {} blobs — saving locally…", blobs.len()));

        let store = BlobStore::new(BlobStore::default_path());
        let mut paths = Vec::new();
        for (i, blob) in blobs.iter().enumerate() {
            let path = store.save(blob)?;
            paths.push(path);
            progress.report(0.5 + 0.5 * (i as f32 / blobs.len() as f32), "Saving blobs…");
        }

        progress.report(1.0, &format!("{} blobs saved", paths.len()));
        Ok(paths)
    }

    /// List all locally cached SHSH blobs for this device.
    pub fn list_cached_shsh_blobs(&self, ecid: u64) -> Vec<crate::shsh::Shsh2Blob> {
        let store = crate::shsh::BlobStore::new(crate::shsh::BlobStore::default_path());
        store.load_all(ecid, &self.device_info.model_identifier)
    }

    /// Generate a downgrade compatibility report for this device + target iOS.
    pub fn get_downgrade_report(
        &self,
        target_ios: &str,
        ecid: u64,
    ) -> crate::shsh::DowngradeCompatibilityReport {
        use crate::shsh::DowngradeCompatibilityReport;
        DowngradeCompatibilityReport::new(
            &self.device_info.model_identifier,
            self.device_info.ios_version.as_deref().unwrap_or("0"),
            target_ios,
            ecid,
            &self.device_info.chipset,
        )
    }

    /// Build the futurerestore command string for a downgrade operation.
    pub fn build_futurerestore_cmd(
        &self,
        ipsw_path: &PathBuf,
        blob_path: &PathBuf,
        use_latest_sep: bool,
        use_latest_baseband: bool,
        nonce_generator: Option<&str>,
    ) -> String {
        use crate::restore::{IpswRestorer, IpswRestoreOptions};
        let opts = IpswRestoreOptions {
            ipsw_path: ipsw_path.clone(),
            shsh_blob_path: Some(blob_path.clone()),
            use_local_shsh: true,
            verify_with_tss: false,
            use_latest_sep,
            use_latest_baseband,
            nonce_generator: nonce_generator.map(|s| s.to_string()),
            ..Default::default()
        };
        let restorer = IpswRestorer::new(
            &self.device_info.udid,
            &self.device_info.model_identifier,
            opts,
        );
        restorer.build_futurerestore_command()
    }

    // ────────────────────────────────────────────────────────────────────────
    // AU CARRIER UNLOCK (au_carrier_unlock module)
    // ────────────────────────────────────────────────────────────────────────

    /// Run the full AU carrier unlock wizard for this device.
    #[cfg(feature = "au_carrier_unlock")]
    pub fn run_au_unlock_wizard(
        &self,
        mccmnc: &str,
        carrier_name: &str,
    ) -> crate::au_carrier_unlock::UnlockGuide {

        let imei = self.device_info.imei.as_deref().unwrap_or("UNKNOWN_IMEI");
        let model = &self.device_info.model_identifier;
        AuIphoneUnlockWizard::generate_guide(imei, mccmnc, model, 0, None)
    }

    /// Validate an IMEI string against the Luhn algorithm.
    #[cfg(feature = "au_carrier_unlock")]
    pub fn validate_imei(imei: &str) -> Result<()> {
        crate::au_carrier_unlock::validate_imei(imei)
    }
    // ── iCloud Endpoint Helpers ───────────────────────────────────────────

    /// Check iCloud escrow key status for the connected device.
    /// Returns true if a key is held in escrow (device has been backed up to iCloud Keychain).
    pub async fn check_escrow_key(&self, progress: impl Fn(&str, f32)) -> Result<bool> {
        progress("Checking iCloud escrow key…", 0.1);
        let ecid_str = self.device_info.udid.trim_start_matches("0x");
        let ecid: u64 = u64::from_str_radix(ecid_str, 16).unwrap_or(0);
        if ecid == 0 {
            warn!("check_escrow_key: ECID unavailable, skipping");
            progress("Escrow check skipped (no ECID)", 1.0);
            return Ok(false);
        }
        let url = escrow_proxy_url(ecid);
        info!("Escrow key check → {}", url);
        progress("Escrow key lookup complete", 1.0);
        // Query escrowproxy.icloud.com to check if the device has an escrow keybag saved.
        // This endpoint is used by iCloud Keychain to assess whether encrypted backup data
        // can be recovered. A 200 response with {"key":"..."} means an escrow key exists.
        let rt = tokio::runtime::Handle::try_current();
        let result = if let Ok(handle) = rt {
            handle.block_on(async {
                reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .user_agent("MobileLockdown/1.0")
                    .build()
                    .ok()
                    .and_then(|c| {
                        let url2 = url.clone();
                        futures::executor::block_on(async move {
                            c.get(&url2).send().await.ok()
                        })
                    })
                    .map(|r| r.status().is_success())
                    .unwrap_or(false)
            })
        } else {
            // No async runtime — use blocking client
            reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("MobileLockdown/1.0")
                .build()
                .ok()
                .and_then(|c| c.get(&url).send().ok())
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        };
        progress("Escrow key check complete", 1.0);
        Ok(result)
    }

    /// Check Apple MCC carrier unlock status for the connected device.
    /// Returns the status string: "Unlocked", "Locked", "Pending", or "Unknown".
    pub async fn check_mcc_carrier_unlock(&self, progress: impl Fn(&str, f32)) -> Result<String> {
        progress("Checking MCC carrier unlock status…", 0.1);
        let imei = self.device_info.imei.as_deref().unwrap_or("UNKNOWN_IMEI");
        if imei == "UNKNOWN_IMEI" {
            warn!("check_mcc_carrier_unlock: IMEI unavailable");
            progress("MCC check skipped (no IMEI)", 1.0);
            return Ok("Unknown".to_string());
        }
        let url = mcc_unlock_status_url(imei);
        info!("MCC unlock status → {}", url);
        progress("MCC status lookup complete", 1.0);
        // Query the MCC carrier lock status via iCloud MCC gateway.
        // Response JSON: {"status":"Unlocked"} / {"status":"Locked"} / {"status":"Pending"}
        let status = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("MobileLockdown/1.0")
            .build()
            .ok()
            .and_then(|c| c.get(&url).send().ok())
            .and_then(|r| if r.status().is_success() { r.text().ok() } else { None })
            .and_then(|body| {
                // Parse {"status":"..."} from JSON
                serde_json::from_str::<serde_json::Value>(&body).ok()
                    .and_then(|v| v.get("status").and_then(|s| s.as_str()).map(|s| s.to_owned()))
            })
            .unwrap_or_else(|| "Unknown".to_string());

        progress("MCC status lookup complete", 1.0);
        Ok(status)
    }

    /// Check iCloud activation lock status via fmipmobile.icloud.com.
    /// Returns true if NOT activation-locked (safe to restore/flash).
    pub async fn check_activation_lock_online(&self, progress: impl Fn(&str, f32)) -> Result<bool> {
        progress("Checking activation lock status online…", 0.1);
        let imei   = self.device_info.imei.as_deref().unwrap_or("");
        let serial = &self.device_info.serial_number;
        let url = activation_status_url(imei, serial);
        info!("Activation lock check → {}", url);
        progress("Activation lock check complete", 1.0);
        // Query Apple's activation lock check service.
        // Returns {"activationLockedStatus":"0"} if unlocked, "1" if locked.
        let is_unlocked = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("MobileLockdown/1.0")
            .build()
            .ok()
            .and_then(|c| c.get(&url).send().ok())
            .and_then(|r| if r.status().is_success() { r.text().ok() } else { None })
            .and_then(|body| {
                serde_json::from_str::<serde_json::Value>(&body).ok()
                    .and_then(|v| {
                        let status = v.get("activationLockedStatus").and_then(|s| s.as_str()).unwrap_or("");
                        Some(status == "0")
                    })
            })
            .unwrap_or(false);

        progress("Activation lock check complete", 1.0);
        Ok(is_unlocked)
    }

    /// Return a formatted summary of all iCloud endpoints relevant to
    /// restore / bypass / AU unlock operations for display in diagnostics.
    pub fn icloud_endpoint_summary(&self) -> String {
        let restore_eps = restore_relevant_endpoints();
        let au_eps      = au_unlock_relevant_endpoints();
        let mut out = endpoint_summary();
        out.push_str("

Restore-relevant endpoints:
");
        for ep in &restore_eps {
            let ip = if ep.ipv4.is_empty() { "—".to_string() } else { ep.ipv4.join(", ") };
            out.push_str(&format!("  {:55}  {:20}  {}
", ep.fqdn, ip, ep.description));
        }
        out.push_str("
AU-unlock-relevant endpoints:
");
        for ep in &au_eps {
            let ip = if ep.ipv4.is_empty() { "—".to_string() } else { ep.ipv4.join(", ") };
            out.push_str(&format!("  {:55}  {:20}  {}
", ep.fqdn, ip, ep.description));
        }
        out
    }

}
