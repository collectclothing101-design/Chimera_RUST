//! Samsung **Read Codes** procedure — returns all 6 lock codes in a single
//! operation: Master (MCK/Freeze/PUK), Network (NCK), Subset (SPCK), Service
//! Provider (SP), Corporate (CP), and SIM-lock. This matches ChimeraTool's
//! "Read Codes Online" feature documented at
//! <https://chimeratool.com/features/read-codes-online>.
//!
//! ## Method
//!
//! Two transport paths are available, in order of preference:
//!
//! 1. **AT-command path** — when the device is in ADB+Diag mode the codes
//!    live in NV items reachable via `AT+NVREAD` / vendor `AT+SECNCK?`
//!    queries through a Samsung modem AT channel.
//!
//! 2. **EUB mode** — Exynos USB Boot. The codes live in the NV partition
//!    we can read directly through the EUB protocol on the SoC chip ID
//!    families documented at <https://chimeratool.com/docs/all-supported-exynos-models>.
//!
//! This module exposes a typed result struct; the actual AT / EUB I/O is
//! orchestrated by `chimera-samsung::operations::SamsungOperations`.

use serde::{Serialize, Deserialize};
use chimera_core::error::{ChimeraError, Result};

/// All 6 lock codes that ChimeraTool's Read Codes procedure returns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LockCodes {
    /// Master Control Key — the universal unlock code. Also unlocks PUK
    /// and Freeze sub-locks.
    pub mck:         Option<String>,
    /// Network code: removes carrier SIM-network restriction.
    pub nck:         Option<String>,
    /// Subset code: more granular network family lock (e.g. T-Mobile US
    /// vs T-Mobile Czech).
    pub subset:      Option<String>,
    /// Service Provider code.
    pub sp:          Option<String>,
    /// Corporate / enterprise code.
    pub cp:          Option<String>,
    /// SIM-lock code (specific to one ICCID).
    pub sim_lock:    Option<String>,
    /// True when the device is currently locked to a network — useful for
    /// the GUI to colour "Locked / Unlocked".
    pub is_locked:   bool,
    /// Free-form remaining-tries counter (e.g. "5/5 tries remaining").
    pub tries_left:  Option<String>,
}

/// Status helper — convenience for showing "X/6 codes recovered".
impl LockCodes {
    pub fn recovered_count(&self) -> u32 {
        let s = |o: &Option<String>| if o.as_deref().filter(|s| !s.is_empty()).is_some() { 1 } else { 0 };
        s(&self.mck) + s(&self.nck) + s(&self.subset) + s(&self.sp)
            + s(&self.cp) + s(&self.sim_lock)
    }
    pub fn all_six(&self) -> bool { self.recovered_count() == 6 }
}

/// Parse raw AT response text into the typed LockCodes struct.
///
/// Samsung firmware emits codes in different formats across firmware
/// generations; this parser handles every documented variation:
///
///   MCK: 12345678                              # one per line, colon-separated
///   MCK=12345678                               # one per line, equals-separated
///   +SECNCK: MCK=...,NCK=...,SPCK=...,CPCK=... # AT response, comma-separated pairs
///
pub fn parse_at_response(at_response: &str) -> Result<LockCodes> {
    let mut codes = LockCodes::default();

    for raw_line in at_response.lines() {
        let line = raw_line.trim();
        if line.is_empty() { continue; }
        if line.contains("LOCKED") { codes.is_locked = true; }
        if let Some(idx) = line.find("tries") {
            let head = &line[..idx];
            if let Some(num) = head.split_whitespace().last() {
                codes.tries_left = Some(num.to_string());
            }
        }

        // ── Case A: "+SECNCK: KEY=VAL,KEY=VAL,…" — strip the AT-tag prefix
        // ONLY when the head before ": " starts with '+'. Otherwise the head
        // itself is the key (e.g. "MCK: 12345678").
        let body = if let Some(idx) = line.find(": ") {
            let head = &line[..idx];
            if head.starts_with('+') {
                &line[idx + 2..]
            } else {
                line
            }
        } else {
            line
        };

        // ── Case B: comma-separated KEY=VAL pairs OR a single KEY[: =]VAL.
        // We split on commas first; if there are no commas the whole body
        // is one entry.
        for entry in body.split(',') {
            let entry = entry.trim().trim_end_matches(';');
            if entry.is_empty() { continue; }
            // Find the first '=' or ':' or whitespace as separator
            let sep_idx = entry.find('=')
                .or_else(|| entry.find(':'))
                .or_else(|| entry.find(char::is_whitespace));
            let (key_raw, val_raw) = match sep_idx {
                Some(i) => (&entry[..i], entry[i+1..].trim_start()),
                None    => continue,
            };
            let key = key_raw.trim().to_uppercase();
            let val = val_raw.trim().trim_matches('"').to_string();
            if val.is_empty() { continue; }
            match key.as_str() {
                "MCK" | "MASTER" | "FREEZE"  => codes.mck      = Some(val),
                "NCK" | "NETWORK"            => codes.nck      = Some(val),
                "SUBSET" | "NSCK" | "SUBCK"  => codes.subset   = Some(val),
                "SPCK" | "SP"                => codes.sp       = Some(val),
                "CPCK" | "CP" | "CORP"       => codes.cp       = Some(val),
                "SIMCK" | "SIM"              => codes.sim_lock = Some(val),
                _ => {}
            }
        }
    }

    if codes.recovered_count() == 0 {
        return Err(ChimeraError::Unknown(
            "no lock codes found in AT response — device may not be locked, \
             or this firmware doesn't expose codes via AT".into()));
    }
    Ok(codes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_secnck_response() {
        let at = "+SECNCK: MCK=12345678,NCK=87654321,SPCK=11223344,CPCK=55667788\r\nOK\r\n";
        let c = parse_at_response(at).unwrap();
        assert_eq!(c.mck.as_deref(), Some("12345678"));
        assert_eq!(c.nck.as_deref(), Some("87654321"));
        assert_eq!(c.sp.as_deref(),  Some("11223344"));
        assert_eq!(c.cp.as_deref(),  Some("55667788"));
        assert_eq!(c.recovered_count(), 4);
        assert!(!c.all_six());
    }

    #[test]
    fn parses_multi_line() {
        let at = "MCK: 11111111\nNCK: 22222222\nSUBSET: 33333333\nSPCK: 44444444\nCPCK: 55555555\nSIMCK: 66666666";
        let c = parse_at_response(at).unwrap();
        assert_eq!(c.recovered_count(), 6);
        assert!(c.all_six());
    }

    #[test]
    fn empty_response_errs() {
        assert!(parse_at_response("").is_err());
    }

    #[test]
    fn locked_flag_set() {
        let at = "MCK=88888888\nDEVICE LOCKED";
        let c = parse_at_response(at).unwrap();
        assert!(c.is_locked);
    }
}
