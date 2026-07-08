//! Wrap `idevicepair` — pair / unpair / validate trust between host and device.

use std::time::Duration;
use serde::{Serialize, Deserialize};
use crate::tool::{run, ImobileTool, ImobileError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PairResult {
    Success,
    AlreadyPaired,
    UserRefusedTrust,
    PasswordProtected,
    Unknown,
}

fn classify(stdout: &str, stderr: &str) -> PairResult {
    let combined = format!("{}\n{}", stdout, stderr).to_lowercase();
    if combined.contains("success") || combined.contains("paired") {
        if combined.contains("already") { PairResult::AlreadyPaired }
        else { PairResult::Success }
    }
    else if combined.contains("refused") || combined.contains("declined") || combined.contains("trust") {
        PairResult::UserRefusedTrust
    }
    else if combined.contains("passcode") || combined.contains("password") {
        PairResult::PasswordProtected
    }
    else { PairResult::Unknown }
}

pub fn pair(udid: Option<&str>) -> Result<PairResult, ImobileError> {
    let mut args = vec!["pair"];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    match run(ImobileTool::Idevicepair, &args, Duration::from_secs(30)) {
        Ok(o) => Ok(classify(&String::from_utf8_lossy(&o.stdout),
                             &String::from_utf8_lossy(&o.stderr))),
        Err(ImobileError::NonZeroExit { stderr, .. }) => Ok(classify("", &stderr)),
        Err(e) => Err(e),
    }
}

pub fn unpair(udid: Option<&str>) -> Result<(), ImobileError> {
    let mut args = vec!["unpair"];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    run(ImobileTool::Idevicepair, &args, Duration::from_secs(10)).map(|_| ())
}

pub fn validate(udid: Option<&str>) -> Result<bool, ImobileError> {
    let mut args = vec!["validate"];
    if let Some(u) = udid { args.push("-u"); args.push(u); }
    match run(ImobileTool::Idevicepair, &args, Duration::from_secs(10)) {
        Ok(_)  => Ok(true),
        Err(ImobileError::NonZeroExit { .. }) => Ok(false),
        Err(e) => Err(e),
    }
}
