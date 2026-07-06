//! EMM (Enterprise Mobility Management) agent detection.
//!
//! Zebra TC52/TC53 fleets are typically managed by one of: SOTI MobiControl,
//! VMware Workspace ONE (AirWatch), Ivanti Avalanche, Microsoft Intune,
//! 42Gears SureMDM, IBM MaaS360, MobileIron / Ivanti UEM, Hexnode,
//! Scalefusion, Esper, or Zebra Workforce Connect.
//!
//! This module enumerates installed packages via ADB and surfaces the
//! detected agent. The result is purely informational — refusal to
//! proceed with bypass operations when an EMM is present is the caller's
//! decision (typically the GUI's destructive-confirm modal).

use serde::{Serialize, Deserialize};
use crate::{Result, ZebraError};

/// Known EMM agents that ship on Zebra fleets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmmAgent {
    SotiMobiControl,
    WorkspaceOne,         // VMware (now Omnissa)
    IvantiAvalanche,
    MicrosoftIntune,
    SureMdm,              // 42Gears
    MaaS360,              // IBM
    MobileIron,           // also: Ivanti EPMM
    Hexnode,
    Scalefusion,
    Esper,
    AndroidEnterprise,    // Google DPC — vendor-neutral fallback
    ZebraWorkforceConnect,
    Unknown(String),
}

impl EmmAgent {
    pub fn display(&self) -> String {
        match self {
            EmmAgent::SotiMobiControl       => "SOTI MobiControl".into(),
            EmmAgent::WorkspaceOne          => "VMware Workspace ONE / Omnissa".into(),
            EmmAgent::IvantiAvalanche       => "Ivanti Avalanche".into(),
            EmmAgent::MicrosoftIntune       => "Microsoft Intune".into(),
            EmmAgent::SureMdm               => "42Gears SureMDM".into(),
            EmmAgent::MaaS360               => "IBM MaaS360".into(),
            EmmAgent::MobileIron            => "MobileIron / Ivanti EPMM".into(),
            EmmAgent::Hexnode               => "Hexnode UEM".into(),
            EmmAgent::Scalefusion           => "Scalefusion".into(),
            EmmAgent::Esper                 => "Esper".into(),
            EmmAgent::AndroidEnterprise     => "Android Enterprise (vendor-neutral)".into(),
            EmmAgent::ZebraWorkforceConnect => "Zebra Workforce Connect".into(),
            EmmAgent::Unknown(s)            => format!("Unknown ({})", s),
        }
    }
}

/// Result of an EMM probe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmmDetection {
    /// Every agent we matched on the device — usually one, occasionally
    /// two (e.g. Workforce Connect alongside an EMM).
    pub detected:          Vec<EmmAgent>,
    /// Device-owner package (from `dpm dump-device-owner`).
    pub device_owner_pkg:  Option<String>,
    /// Profile-owner package — set when a work profile is provisioned.
    pub profile_owner_pkg: Option<String>,
    /// True when ANY device-owner OR profile-owner is set.
    pub is_managed:        bool,
    /// True when the detected agent matches the device-owner package
    /// (i.e. the device is hard-enrolled in that EMM, not just running
    /// the agent app).
    pub enrolled:          bool,
}

/// Run all detection probes against an ADB device.
///
/// `target` = None ⇒ first connected device.
pub fn detect_emm(target: Option<&str>) -> Result<EmmDetection> {
    let probe = chimera_utils::host_probes::detect_adb();
    if !probe.found {
        return Err(ZebraError::Adb("adb not found on host".into()));
    }
    let adb = probe.path.as_ref().unwrap();

    let pkgs = adb_shell(adb, target, "pm list packages")?;
    let detected = classify_packages(&pkgs);

    let dpm = adb_shell(adb, target, "dpm dump-device-owner").unwrap_or_default();
    let owner = extract_owner_package(&dpm);

    let dpsys = adb_shell(adb, target, "dumpsys device_policy").unwrap_or_default();
    let profile_owner = parse_profile_owner(&dpsys);

    let is_managed = owner.is_some() || profile_owner.is_some();
    let enrolled = owner.as_ref()
        .map(|o| detected.iter().any(|a| package_matches_agent(o, a)))
        .unwrap_or(false);

    Ok(EmmDetection {
        detected,
        device_owner_pkg:  owner,
        profile_owner_pkg: profile_owner,
        is_managed,
        enrolled,
    })
}

/// Classify a `pm list packages` dump into a list of detected agents.
pub fn classify_packages(pm_list: &str) -> Vec<EmmAgent> {
    let mut out = Vec::new();
    for line in pm_list.lines() {
        let pkg = line.trim().trim_start_matches("package:");
        match pkg {
            "net.soti.mobicontrol.androidwork"
                | "net.soti.mobicontrol"             => out.push(EmmAgent::SotiMobiControl),
            "com.airwatch.androidagent"
                | "com.workspaceone.intelligenthub"
                | "com.airwatch.androidagent.intelligenthub" => out.push(EmmAgent::WorkspaceOne),
            "com.wavelink.tn"
                | "com.wavelink.terminalemulation"
                | "com.ivanti.avalanche.client"     => out.push(EmmAgent::IvantiAvalanche),
            "com.microsoft.windowsintune.companyportal"
                | "com.microsoft.intune"
                | "com.microsoft.intune.companyportal" => out.push(EmmAgent::MicrosoftIntune),
            "com.nix"
                | "com.gears42.suremdm"             => out.push(EmmAgent::SureMdm),
            "com.fiberlink.maas360.android.control"
                | "com.fiberlink.maas360"           => out.push(EmmAgent::MaaS360),
            "com.mobileiron"
                | "com.mobileiron.client.android.nondm"
                | "com.mobileiron.go"
                | "com.ivanti.epmm.client"          => out.push(EmmAgent::MobileIron),
            "com.hexnode.mdm"                       => out.push(EmmAgent::Hexnode),
            "com.scalefusion.androidsdk"
                | "com.promobitech.mobilock.pro"    => out.push(EmmAgent::Scalefusion),
            "io.esper.agent"
                | "io.shoonya.shoonyadpc"           => out.push(EmmAgent::Esper),
            "com.google.android.apps.work.clouddpc"
                | "com.google.android.apps.enterprise.dmagent" => out.push(EmmAgent::AndroidEnterprise),
            "com.symbol.workforceconnect"
                | "com.zebra.workforceconnect"      => out.push(EmmAgent::ZebraWorkforceConnect),
            _ => {}
        }
    }
    out.sort_by_key(|a| format!("{:?}", a));
    out.dedup_by_key(|a| format!("{:?}", a));
    out
}

/// Extract the device-owner package from `dpm dump-device-owner` output.
fn extract_owner_package(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(start) = line.find("ComponentInfo{") {
            let rest = &line[start + 14..];
            if let Some(slash) = rest.find('/') {
                return Some(rest[..slash].to_string());
            }
        }
        if let Some(pkg) = line.strip_prefix("Device Owner package: ") {
            return Some(pkg.trim().to_string());
        }
    }
    None
}

/// `dumpsys device_policy` "Profile Owner" section parser.
fn parse_profile_owner(dumpsys: &str) -> Option<String> {
    let mut in_section = false;
    for line in dumpsys.lines() {
        let line = line.trim();
        if line.starts_with("Profile Owner") { in_section = true; continue; }
        if in_section {
            if let Some(start) = line.find("ComponentInfo{") {
                let rest = &line[start + 14..];
                if let Some(slash) = rest.find('/') {
                    return Some(rest[..slash].to_string());
                }
            }
            if line.is_empty() && in_section { break; }
        }
    }
    None
}

fn package_matches_agent(pkg: &str, agent: &EmmAgent) -> bool {
    matches!(
        (agent, pkg),
        (EmmAgent::SotiMobiControl,        "net.soti.mobicontrol.androidwork")  |
        (EmmAgent::SotiMobiControl,        "net.soti.mobicontrol")              |
        (EmmAgent::WorkspaceOne,           "com.airwatch.androidagent")         |
        (EmmAgent::WorkspaceOne,           "com.workspaceone.intelligenthub")   |
        (EmmAgent::IvantiAvalanche,        "com.wavelink.tn")                   |
        (EmmAgent::IvantiAvalanche,        "com.ivanti.avalanche.client")       |
        (EmmAgent::MicrosoftIntune,        "com.microsoft.windowsintune.companyportal") |
        (EmmAgent::SureMdm,                "com.nix")                           |
        (EmmAgent::MaaS360,                "com.fiberlink.maas360.android.control") |
        (EmmAgent::MobileIron,             "com.mobileiron")                    |
        (EmmAgent::Hexnode,                "com.hexnode.mdm")                   |
        (EmmAgent::Scalefusion,            "com.promobitech.mobilock.pro")      |
        (EmmAgent::Scalefusion,            "com.scalefusion.androidsdk")        |
        (EmmAgent::Esper,                  "io.shoonya.shoonyadpc")             |
        (EmmAgent::AndroidEnterprise,      "com.google.android.apps.work.clouddpc") |
        (EmmAgent::ZebraWorkforceConnect,  "com.symbol.workforceconnect")
    )
}

fn adb_shell(adb: &std::path::Path, target: Option<&str>, cmd: &str) -> Result<String> {
    use std::process::Command;
    let mut c = Command::new(adb);
    if let Some(s) = target { c.args(["-s", s]); }
    c.arg("shell").arg(cmd);
    let out = c.output().map_err(|e| ZebraError::Adb(format!("spawn: {}", e)))?;
    if !out.status.success() {
        return Err(ZebraError::Adb(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_soti() {
        let pm = "package:net.soti.mobicontrol.androidwork\npackage:com.android.chrome\n";
        let r = classify_packages(pm);
        assert!(r.contains(&EmmAgent::SotiMobiControl));
    }

    #[test]
    fn detects_workspace_one() {
        let pm = "package:com.airwatch.androidagent\n";
        let r = classify_packages(pm);
        assert!(r.contains(&EmmAgent::WorkspaceOne));
    }

    #[test]
    fn detects_avalanche() {
        let pm = "package:com.wavelink.tn\n";
        let r = classify_packages(pm);
        assert!(r.contains(&EmmAgent::IvantiAvalanche));
    }

    #[test]
    fn detects_intune() {
        let pm = "package:com.microsoft.windowsintune.companyportal\n";
        let r = classify_packages(pm);
        assert!(r.contains(&EmmAgent::MicrosoftIntune));
    }

    #[test]
    fn detects_suremdm() {
        let pm = "package:com.nix\n";
        assert!(classify_packages(pm).contains(&EmmAgent::SureMdm));
    }

    #[test]
    fn detects_maas360() {
        let pm = "package:com.fiberlink.maas360.android.control\n";
        assert!(classify_packages(pm).contains(&EmmAgent::MaaS360));
    }

    #[test]
    fn detects_workforce_connect() {
        let pm = "package:com.symbol.workforceconnect\n";
        assert!(classify_packages(pm).contains(&EmmAgent::ZebraWorkforceConnect));
    }

    #[test]
    fn no_emm_yields_empty_list() {
        let pm = "package:com.android.chrome\npackage:com.google.android.gms\n";
        assert!(classify_packages(pm).is_empty());
    }

    #[test]
    fn dedups_repeated_agents() {
        let pm = "package:net.soti.mobicontrol.androidwork\npackage:net.soti.mobicontrol\n";
        let r = classify_packages(pm);
        // Both lines map to SotiMobiControl — should appear exactly once
        assert_eq!(r.iter().filter(|a| matches!(a, EmmAgent::SotiMobiControl)).count(), 1);
    }

    #[test]
    fn extracts_device_owner_componentinfo() {
        let s = "Device Owner:\n  ComponentInfo{net.soti.mobicontrol.androidwork/.MobiControlAdmin}\n";
        assert_eq!(extract_owner_package(s),
                   Some("net.soti.mobicontrol.androidwork".into()));
    }

    #[test]
    fn extracts_device_owner_plain_pkg() {
        let s = "Device Owner package: com.airwatch.androidagent\n";
        assert_eq!(extract_owner_package(s), Some("com.airwatch.androidagent".into()));
    }

    #[test]
    fn agent_display_names_nonempty() {
        for a in [EmmAgent::SotiMobiControl, EmmAgent::WorkspaceOne,
                  EmmAgent::IvantiAvalanche, EmmAgent::MicrosoftIntune,
                  EmmAgent::SureMdm, EmmAgent::MaaS360, EmmAgent::MobileIron,
                  EmmAgent::Hexnode, EmmAgent::Scalefusion, EmmAgent::Esper,
                  EmmAgent::AndroidEnterprise, EmmAgent::ZebraWorkforceConnect] {
            assert!(!a.display().is_empty());
        }
    }
}
