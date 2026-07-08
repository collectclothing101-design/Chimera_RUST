// chimera-apple/src/ipsw.rs
// IPSW firmware archive handling: parsing, validation, extraction of restore manifests,
// and per-partition image lookup for flashing via Recovery/DFU mode.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use log::{info, debug};

/// An entry from the BuildManifest.plist inside an IPSW
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildIdentity {
    /// e.g. "iPhone14,3"
    pub product_type: String,
    /// e.g. "17.2"
    pub product_version: String,
    /// e.g. "21C62"
    pub build_version: String,
    /// Maps image role → relative path inside ZIP
    pub images: HashMap<String, IpswImage>,
    /// Restore behaviour
    pub restore_behavior: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpswImage {
    /// Path within the IPSW zip archive
    pub path: String,
    /// Expected SHA1 digest (hex)
    pub digest: Option<String>,
    /// Expected file size in bytes
    pub size: Option<u64>,
    pub personalized: bool,
}

/// Metadata extracted from a BuildManifest.plist (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpswManifest {
    pub product_build_version: String,
    pub product_version: String,
    pub supported_product_types: Vec<String>,
    pub identities: Vec<BuildIdentity>,
}

impl IpswManifest {
    /// Parse BuildManifest.plist bytes (XML or binary plist) into a manifest.
    pub fn parse(plist_bytes: &[u8]) -> Result<Self> {
        // Use the `plist` crate to deserialise the BuildManifest
        let value = plist::Value::from_reader(std::io::Cursor::new(plist_bytes))
            .map_err(|e| anyhow!("Failed to parse BuildManifest plist: {}", e))?;

        let dict = match value {
            plist::Value::Dictionary(d) => d,
            _ => return Err(anyhow!("BuildManifest root is not a dictionary")),
        };

        let product_build_version = dict.get("ProductBuildVersion")
            .and_then(|v| v.as_string())
            .unwrap_or("unknown")
            .to_owned();

        let product_version = dict.get("ProductVersion")
            .and_then(|v| v.as_string())
            .unwrap_or("unknown")
            .to_owned();

        let supported_product_types = dict.get("SupportedProductTypes")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|x| x.as_string().map(|s| s.to_owned()))
                .collect())
            .unwrap_or_default();

        // Parse BuildIdentities array
        let mut identities = Vec::new();
        if let Some(plist::Value::Array(build_ids)) = dict.get("BuildIdentities") {
            for id_val in build_ids {
                if let plist::Value::Dictionary(id_dict) = id_val {
                    let product_type = id_dict.get("ApChipID")
                        .and_then(|v| v.as_string())
                        .or_else(|| id_dict.get("Manifest").and_then(|_| Some("unknown")))
                        .unwrap_or("unknown")
                        .to_owned();

                    // Get Info sub-dict for version info
                    let info_dict = id_dict.get("Info")
                        .and_then(|v| if let plist::Value::Dictionary(d) = v { Some(d) } else { None });

                    let pt = info_dict.and_then(|d| d.get("DeviceClass"))
                        .and_then(|v| v.as_string())
                        .unwrap_or(&product_type)
                        .to_owned();

                    let pv = info_dict.and_then(|d| d.get("OSVersion"))
                        .and_then(|v| v.as_string())
                        .unwrap_or(&product_version)
                        .to_owned();

                    let bv = info_dict.and_then(|d| d.get("BuildNumber"))
                        .and_then(|v| v.as_string())
                        .unwrap_or(&product_build_version)
                        .to_owned();

                    // Parse Manifest dict → images
                    let mut images = HashMap::new();
                    if let Some(plist::Value::Dictionary(manifest)) = id_dict.get("Manifest") {
                        for (role, img_val) in manifest {
                            if let plist::Value::Dictionary(img_dict) = img_val {
                                let path = img_dict.get("Path")
                                    .and_then(|v| v.as_string())
                                    .unwrap_or("")
                                    .to_owned();
                                let digest = img_dict.get("Digest")
                                    .and_then(|v| if let plist::Value::Data(d) = v { Some(hex::encode(d)) } else { None });
                                let size = img_dict.get("__size")
                                    .and_then(|v| v.as_unsigned_integer());
                                let personalized = img_dict.get("Personalized")
                                    .and_then(|v| v.as_boolean())
                                    .unwrap_or(false);
                                images.insert(role.clone(), IpswImage { path, digest, size, personalized });
                            }
                        }
                    }

                    let restore_behavior = id_dict.get("Info")
                        .and_then(|v| if let plist::Value::Dictionary(d) = v { d.get("RestoreBehavior") } else { None })
                        .and_then(|v| v.as_string())
                        .map(|s| s.to_owned());

                    identities.push(BuildIdentity {
                        product_type: pt,
                        product_version: pv,
                        build_version: bv,
                        images,
                        restore_behavior,
                    });
                }
            }
        }

        Ok(Self { product_build_version, product_version, supported_product_types, identities })
    }

    /// Find the best BuildIdentity for a given device model.
    pub fn best_identity(&self, model: &str) -> Option<&BuildIdentity> {
        self.identities.iter().find(|id| id.product_type == model)
    }
}

/// Represents an opened IPSW archive ready for restore operations.
pub struct IpswArchive {
    pub path: PathBuf,
    pub manifest: IpswManifest,
    /// Total uncompressed size in bytes
    pub total_size: u64,
}

impl IpswArchive {
    /// Open and validate an IPSW file.
    ///
    /// Opens the ZIP archive, locates BuildManifest.plist, parses it, and
    /// records the total uncompressed size across all entries.
    pub fn open(path: &Path) -> Result<Self> {
        info!("IpswArchive: opening {}", path.display());
        if !path.exists() {
            return Err(anyhow!("IPSW file not found: {}", path.display()));
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "ipsw" && ext != "zip" {
            return Err(anyhow!("File does not appear to be an IPSW archive (expected .ipsw or .zip)"));
        }

        let file = std::fs::File::open(path)
            .map_err(|e| anyhow!("Cannot open IPSW file: {}", e))?;
        let mut zip = zip::ZipArchive::new(file)
            .map_err(|e| anyhow!("Not a valid ZIP/IPSW archive: {}", e))?;

        // Locate BuildManifest.plist (must be at the root of the ZIP)
        let manifest_bytes = {
            let mut entry = zip.by_name("BuildManifest.plist")
                .map_err(|_| anyhow!("BuildManifest.plist not found in IPSW — is this a valid IPSW?"))?;
            use std::io::Read;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            buf
        };

        let manifest = IpswManifest::parse(&manifest_bytes)?;

        // Calculate total uncompressed size
        let total_size: u64 = (0..zip.len())
            .map(|i| zip.by_index(i).map(|e| e.size()).unwrap_or(0))
            .sum();

        info!("IPSW opened: {} {} ({} bytes uncompressed, {} identities)",
              manifest.product_version, manifest.product_build_version,
              total_size, manifest.identities.len());

        Ok(Self {
            path: path.to_path_buf(),
            manifest,
            total_size,
        })
    }

    /// Extract a named image from the IPSW zip to a destination directory.
    pub fn extract_image(&self, image_path: &str, dest: &Path) -> Result<PathBuf> {
        debug!("IpswArchive: extracting {} to {}", image_path, dest.display());
        std::fs::create_dir_all(dest)?;

        let file = std::fs::File::open(&self.path)
            .map_err(|e| anyhow!("Cannot re-open IPSW: {}", e))?;
        let mut zip = zip::ZipArchive::new(file)
            .map_err(|e| anyhow!("ZIP open error: {}", e))?;

        let mut entry = zip.by_name(image_path)
            .map_err(|_| anyhow!("Image '{}' not found in IPSW archive", image_path))?;

        let out_name = Path::new(image_path).file_name()
            .ok_or_else(|| anyhow!("Invalid image path: {}", image_path))?;
        let out_path = dest.join(out_name);

        let mut out_file = std::fs::File::create(&out_path)
            .map_err(|e| anyhow!("Cannot create output file {}: {}", out_path.display(), e))?;

        use std::io::{Read, Write};
        let mut buf = vec![0u8; 65536];
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 { break; }
            out_file.write_all(&buf[..n])?;
        }

        debug!("IpswArchive: extracted {} ({} bytes) → {}", image_path, entry.size(), out_path.display());
        Ok(out_path)
    }

    /// Verify the SHA1 digest of an extracted image against the manifest.
    pub fn verify_image(&self, local_path: &Path, expected_sha1: &str) -> Result<bool> {
        use sha1::{Sha1, Digest};
        use std::io::Read;

        let mut file = std::fs::File::open(local_path)
            .map_err(|e| anyhow!("Cannot open file for verification: {}", e))?;
        let mut hasher = Sha1::new();
        let mut buf = vec![0u8; 65536];
        loop {
            let n = file.read(&mut buf)?;
            if n == 0 { break; }
            hasher.update(&buf[..n]);
        }
        let actual = hex::encode(hasher.finalize());
        let matches = actual.eq_ignore_ascii_case(expected_sha1);
        if !matches {
            debug!("SHA1 mismatch for {}: expected={} got={}", local_path.display(), expected_sha1, actual);
        }
        Ok(matches)
    }
}

/// Well-known image roles used during an Apple restore
pub mod image_role {
    pub const IBSS: &str = "iBSS";
    pub const IBEC: &str = "iBEC";
    pub const DEVICETREE: &str = "DeviceTree";
    pub const KERNEL_CACHE: &str = "KernelCache";
    pub const OS_IMAGE: &str = "OS";
    pub const RESTORE_RAMDISK: &str = "RestoreRamDisk";
    pub const UPDATE_RAMDISK: &str = "UpdateRamDisk";
    pub const SEP_FIRMWARE: &str = "SEP";
    pub const BASEBAND_FIRMWARE: &str = "BasebandFirmware";
    pub const SYSTEM_VOLUME: &str = "SystemVolume";
}

/// Top-level validator for an IPSW file on disk.
///
/// Returns `Ok(true)` when the archive opens, contains a parseable
/// `BuildManifest.plist`, and that manifest has at least one identity.
/// Returns `Err` for I/O, zip, or plist errors.
///
/// This is the entry point the GUI's `OperationRequest::AppleValidateIpsw`
/// handler calls. Internally it leverages `IpswArchive::open`, which
/// already extracts and parses the BuildManifest as part of its open path.
pub fn validate_ipsw(path: impl AsRef<std::path::Path>) -> chimera_core::Result<bool> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(chimera_core::ChimeraError::FileNotFound(
            path.display().to_string()
        ));
    }
    match IpswArchive::open(path) {
        Ok(archive) => Ok(!archive.manifest.identities.is_empty()),
        Err(e) => Err(chimera_core::ChimeraError::Firmware(
            format!("Validate IPSW {}: {}", path.display(), e)
        )),
    }
}
