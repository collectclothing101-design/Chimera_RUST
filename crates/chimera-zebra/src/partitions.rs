//! UFS partition map enumeration (A/B + dynamic super-partition).
//!
//! Zebra TC5x devices use Qualcomm's standard A/B partitioning with a
//! `super` logical-block container that holds `system`, `vendor`,
//! `product`, `system_ext`, `odm`, `oem`, and `zebra` sub-partitions.
//! Reading this layout requires only ADB shell access.

use std::process::Command;
use serde::{Serialize, Deserialize};
use crate::{Result, ZebraError};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Partition {
    pub name:       String,
    pub by_name_dev:Option<String>,
    pub size_bytes: Option<u64>,
    pub is_logical: bool,  // sub-partition under super-block
    pub slot:       Option<String>, // "_a" / "_b" / None
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartitionMap {
    pub current_slot: Option<String>,
    pub partitions:   Vec<Partition>,
}

/// Read the partition map from a connected ADB device.
pub fn read_partition_map(target: Option<&str>) -> Result<PartitionMap> {
    let probe = chimera_utils::host_probes::detect_adb();
    if !probe.found { return Err(ZebraError::Adb("adb missing".into())); }
    let adb = probe.path.as_ref().unwrap();

    let mut map = PartitionMap::default();

    // active slot
    let slot = sh(adb, target, "getprop ro.boot.slot_suffix")?;
    let s = slot.trim();
    if !s.is_empty() { map.current_slot = Some(s.to_string()); }

    // by-name listing
    let ls = sh(adb, target,
        "ls -la /dev/block/by-name/ 2>/dev/null || ls -la /dev/block/platform/*/by-name/")?;
    for line in ls.lines() {
        let line = line.trim();
        if !line.starts_with('l') { continue; }   // symlinks only
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 11 { continue; }
        let name = parts[8].to_string();
        let dev  = parts[10].to_string();
        let (base, slot_suffix) = split_slot(&name);
        map.partitions.push(Partition {
            name:        base.to_string(),
            by_name_dev: Some(dev),
            size_bytes:  None,
            is_logical:  is_logical_zebra_part(base),
            slot:        slot_suffix.map(|s| s.to_string()),
        });
    }

    // Pull sizes via `blockdev --getsize64` (one round-trip per partition is
    // expensive, so we batch into a single shell command).
    let cmd = "for p in /dev/block/by-name/*; do \
                 echo \"$p|$(blockdev --getsize64 $p 2>/dev/null)\"; \
               done";
    if let Ok(sizes) = sh(adb, target, cmd) {
        for line in sizes.lines() {
            if let Some((dev, size)) = line.split_once('|') {
                let dev = dev.trim();
                let size_n: u64 = size.trim().parse().unwrap_or(0);
                for p in map.partitions.iter_mut() {
                    if p.by_name_dev.as_deref() == Some(dev) {
                        p.size_bytes = Some(size_n);
                    }
                }
            }
        }
    }

    Ok(map)
}

fn split_slot(name: &str) -> (&str, Option<&str>) {
    if let Some(rest) = name.strip_suffix("_a") { (rest, Some("_a")) }
    else if let Some(rest) = name.strip_suffix("_b") { (rest, Some("_b")) }
    else { (name, None) }
}

fn is_logical_zebra_part(base: &str) -> bool {
    matches!(base, "system" | "vendor" | "product" | "system_ext" | "odm" | "oem" | "zebra")
}

fn sh(adb: &std::path::Path, target: Option<&str>, cmd: &str) -> Result<String> {
    let mut c = Command::new(adb);
    if let Some(s) = target { c.args(["-s", s]); }
    c.arg("shell").arg(cmd);
    let out = c.output().map_err(|e| ZebraError::Adb(e.to_string()))?;
    if !out.status.success() {
        return Err(ZebraError::Adb(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_split_handles_ab() {
        assert_eq!(split_slot("boot_a"),    ("boot", Some("_a")));
        assert_eq!(split_slot("boot_b"),    ("boot", Some("_b")));
        assert_eq!(split_slot("userdata"),  ("userdata", None));
        assert_eq!(split_slot("vbmeta_a"),  ("vbmeta", Some("_a")));
    }

    #[test]
    fn detects_logical_subparts() {
        assert!(is_logical_zebra_part("system"));
        assert!(is_logical_zebra_part("vendor"));
        assert!(is_logical_zebra_part("zebra"));
        assert!(!is_logical_zebra_part("boot"));
        assert!(!is_logical_zebra_part("userdata"));
    }

    #[test]
    fn empty_map_default() {
        let m = PartitionMap::default();
        assert!(m.current_slot.is_none());
        assert!(m.partitions.is_empty());
    }
}
