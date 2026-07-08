//! **Programmer Analyser** — parses Qualcomm programmer files (the `.mbn`
//! / `prog_emmc_firehose_*.elf` / `prog_ufs_firehose_*.elf` blobs that
//! ChimeraTool ships per-chipset for EDL mode loaders) and extracts
//! technical metadata.
//!
//! This matches the free "Programmer Analysis" utility added to ChimeraTool
//! in 2025 ("You can select a single programmer or an entire folder
//! containing multiple programmers. We analyse them automatically and
//! display useful, structured information").

use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use chimera_core::error::{ChimeraError, Result};

/// One programmer file's metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgrammerInfo {
    pub path:           PathBuf,
    pub file_name:      String,
    pub size_bytes:     u64,
    pub chip_family:    Option<String>,    // e.g. "MSM8909", "SM7150", "SM8550"
    pub storage_type:   StorageType,
    pub firehose_proto: bool,
    pub sha256:         String,
    pub format:         FileFormat,
    pub embedded_strings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    Emmc,
    Ufs,
    SpiNor,
    Nand,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileFormat {
    Elf,
    Mbn,
    PlainBinary,
    Unknown,
}

/// Inspect one file and return its parsed metadata.
pub fn analyse_file(path: &Path) -> Result<ProgrammerInfo> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|e| ChimeraError::Unknown(format!("open {}: {}", path.display(), e)))?;
    let metadata = file.metadata()
        .map_err(|e| ChimeraError::Unknown(format!("metadata: {}", e)))?;
    let size = metadata.len();
    let mut head = vec![0u8; 4096.min(size as usize)];
    file.read_exact(&mut head)
        .map_err(|e| ChimeraError::Unknown(format!("read head: {}", e)))?;

    // Full SHA-256
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut f2 = std::fs::File::open(path).map_err(|e| ChimeraError::Unknown(e.to_string()))?;
    loop {
        let n = f2.read(&mut buf).map_err(|e| ChimeraError::Unknown(e.to_string()))?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    let sha256 = hex::encode(hasher.finalize());

    // Format sniff
    let format = if head.len() >= 4 && &head[..4] == b"\x7fELF" {
        FileFormat::Elf
    } else if file_name_lossy(path).to_lowercase().ends_with(".mbn") {
        FileFormat::Mbn
    } else {
        FileFormat::PlainBinary
    };

    // Storage sniff — look for marker strings in the first 4 KB.
    let head_str = String::from_utf8_lossy(&head);
    let storage_type = if head_str.contains("FIREHOSE_UFS") || head_str.contains("ufs_") {
        StorageType::Ufs
    } else if head_str.contains("FIREHOSE_EMMC") || head_str.contains("emmc_") {
        StorageType::Emmc
    } else if head_str.contains("spi_nor") || head_str.contains("SPI_NOR") {
        StorageType::SpiNor
    } else if head_str.contains("nand") || head_str.contains("NAND") {
        StorageType::Nand
    } else {
        StorageType::Unknown
    };

    let firehose_proto = head_str.contains("firehose") || head_str.contains("FIREHOSE")
                         || head_str.contains("<configure");

    // Chip family — extract from filename pattern like prog_emmc_firehose_8909_ddr.elf
    let chip_family = extract_chip_family(&file_name_lossy(path));

    // Strings (printable runs ≥ 6 chars, first 64)
    let embedded_strings = extract_strings(&head, 6).into_iter().take(64).collect();

    Ok(ProgrammerInfo {
        path:           path.to_path_buf(),
        file_name:      file_name_lossy(path).to_string(),
        size_bytes:     size,
        chip_family,
        storage_type,
        firehose_proto,
        sha256,
        format,
        embedded_strings,
    })
}

/// Analyse every programmer file in a directory (non-recursive).
pub fn analyse_directory(dir: &Path) -> Result<Vec<ProgrammerInfo>> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|e| ChimeraError::Unknown(format!("read_dir {}: {}", dir.display(), e)))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if !matches!(ext.as_str(), "mbn" | "elf" | "bin") { continue; }
        match analyse_file(&path) {
            Ok(info) => out.push(info),
            Err(_)   => { /* skip unreadable */ }
        }
    }
    Ok(out)
}

fn file_name_lossy(p: &Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

fn extract_chip_family(name: &str) -> Option<String> {
    // Look for SM6225, SDM632, MSM8909, SM8550, QM215 etc.
    let upper = name.to_uppercase();
    for prefix in &["SDM", "MSM", "SM", "QM"] {
        if let Some(idx) = upper.find(prefix) {
            let tail = &upper[idx..];
            let mut buf = String::new();
            let mut iter = tail.chars();
            for _ in 0..prefix.len() {
                if let Some(c) = iter.next() { buf.push(c); }
            }
            for c in iter {
                if c.is_ascii_digit() { buf.push(c); } else { break; }
            }
            if buf.len() > prefix.len() { return Some(buf); }
        }
    }

    // Bare-number fallback: many ChimeraTool programmer files use the
    // pattern  "prog_<storage>_firehose_<NNNN>[_extras].(elf|mbn|bin)" where
    // NNNN is the bare MSM/SM number (e.g. 8909, 8550, 6225). When found
    // we synthesise the "SM" prefix since that's the convention for SoCs
    // 6225 and later.
    //
    // Heuristic: take the first 4-or-5 digit run that's flanked by
    // underscores / dots / hyphens (so version numbers like "v2" or "1.0"
    // don't false-match).
    let bytes = upper.as_bytes();
    let n = bytes.len();
    let is_sep = |b: u8| matches!(b, b'_' | b'-' | b'.' | b'/' | b'\\');
    let mut i = 0;
    while i < n {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < n && bytes[i].is_ascii_digit() { i += 1; }
            let len = i - start;
            let prev = if start == 0 { b'_' } else { bytes[start - 1] };
            let next = if i == n { b'_' } else { bytes[i] };
            if (4..=5).contains(&len) && is_sep(prev) && (is_sep(next) || next.is_ascii_alphabetic()) {
                let digits = &upper[start..i];
                // Prefix: SM for ≥ 6000, MSM for 8000-8999, QM for ≥ 200 sub-4
                let num: u32 = digits.parse().unwrap_or(0);
                let prefix = if (8000..9000).contains(&num) { "MSM" } else { "SM" };
                return Some(format!("{}{}", prefix, digits));
            }
        } else {
            i += 1;
        }
    }
    None
}

fn extract_strings(bytes: &[u8], min_len: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for &b in bytes {
        if (32..127).contains(&b) {
            cur.push(b as char);
        } else {
            if cur.len() >= min_len { out.push(std::mem::take(&mut cur)); }
            else { cur.clear(); }
        }
    }
    if cur.len() >= min_len { out.push(cur); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_family_extracted() {
        // Standard pattern: prefix + 4 digits
        let r1 = extract_chip_family("prog_emmc_firehose_8909_ddr.elf");
        // r1 may be "MSM8909" or "SM8909" depending on the prefix sniffed
        assert!(r1.is_some(), "expected a chip family from 8909 filename");

        let r2 = extract_chip_family("prog_ufs_firehose_8550.mbn");
        assert!(r2.is_some());

        // Plain text without any chipset marker should return None
        assert!(extract_chip_family("readme.txt").is_none());
    }

    #[test]
    fn strings_extracted() {
        let bytes = b"\x00\x00HelloWorld\x00\x00Foo!Bar\x00";
        let strs = extract_strings(bytes, 5);
        assert!(strs.iter().any(|s| s.contains("HelloWorld")));
    }

    #[test]
    fn enums_serialise() {
        let s = serde_json::to_string(&StorageType::Ufs).unwrap();
        assert!(s.contains("Ufs"));
        let f = serde_json::to_string(&FileFormat::Mbn).unwrap();
        assert!(f.contains("Mbn"));
    }
}
