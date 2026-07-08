//! Samsung **CSC Change** — Country Specific Code (Consumer Software
//! Customisation). Documented in detail at
//! <https://chimeratool.com/features/csc-change>.
//!
//! ## What CSC controls
//!
//! - Active locales / language list
//! - Default keyboards installed
//! - Default APN configuration
//! - Whether camera shutter can be silenced (Japanese/Korean firmwares lock it)
//! - Carrier-specific app preinstalls
//! - FM radio enable/disable
//!
//! ## How the change is applied
//!
//! On modern Samsung firmware the CSC code lives in
//! `/efs/imei/mps_code.dat` and `omc.ini` inside the `/data/` partition.
//! The procedure rewrites both then triggers a CSC re-application via
//! a hidden service menu intent.

use serde::{Serialize, Deserialize};
use chimera_core::error::{ChimeraError, Result};

/// Catalogue of every Samsung CSC code we know about. Sourced from the
/// public CSC list at <https://www.sammobile.com/firmwares/csc-codes/>.
///
/// We hold owned strings so the struct round-trips through serde without
/// lifetime gymnastics when embedded in result types or sent over the FFI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CscCode {
    pub code:        String,
    pub country:     String,
    pub operator:    String,
    pub mcc_mnc:     Option<String>,
}

/// Internal compile-time entries (slim refs); converted to owned `CscCode`
/// at lookup time.
#[derive(Debug, Clone, Copy)]
struct CscEntry {
    code:     &'static str,
    country:  &'static str,
    operator: &'static str,
    mcc_mnc:  Option<&'static str>,
}

impl From<&'static CscEntry> for CscCode {
    fn from(e: &'static CscEntry) -> Self {
        Self {
            code:     e.code.to_string(),
            country:  e.country.to_string(),
            operator: e.operator.to_string(),
            mcc_mnc:  e.mcc_mnc.map(String::from),
        }
    }
}

/// A representative selection (~80 entries) of the most common Samsung CSCs.
/// The full Samsung CSC space is ~600 codes; this list covers every major
/// carrier region pertinent to repair shops.
const CSC_DATABASE: &[CscEntry] = &[
    // United States
    CscEntry { code: "ATT",  country: "USA", operator: "AT&T",            mcc_mnc: Some("310/410") },
    CscEntry { code: "TMB",  country: "USA", operator: "T-Mobile",        mcc_mnc: Some("310/260") },
    CscEntry { code: "VZW",  country: "USA", operator: "Verizon",         mcc_mnc: Some("311/480") },
    CscEntry { code: "SPR",  country: "USA", operator: "Sprint",          mcc_mnc: Some("310/120") },
    CscEntry { code: "USC",  country: "USA", operator: "U.S. Cellular",   mcc_mnc: Some("311/220") },
    CscEntry { code: "XAA",  country: "USA", operator: "Unlocked USA",    mcc_mnc: None },
    CscEntry { code: "AIO",  country: "USA", operator: "Cricket",         mcc_mnc: Some("310/150") },
    CscEntry { code: "TFN",  country: "USA", operator: "TracFone",        mcc_mnc: Some("310/410") },
    // Canada
    CscEntry { code: "RWC",  country: "CAN", operator: "Rogers",          mcc_mnc: Some("302/720") },
    CscEntry { code: "BMC",  country: "CAN", operator: "Bell",            mcc_mnc: Some("302/610") },
    CscEntry { code: "TLS",  country: "CAN", operator: "Telus",           mcc_mnc: Some("302/220") },
    // UK / Ireland
    CscEntry { code: "BTU",  country: "GBR", operator: "Unlocked UK",     mcc_mnc: None },
    CscEntry { code: "EVR",  country: "GBR", operator: "EE",              mcc_mnc: Some("234/30") },
    CscEntry { code: "VOD",  country: "GBR", operator: "Vodafone UK",     mcc_mnc: Some("234/15") },
    CscEntry { code: "O2U",  country: "GBR", operator: "O2 UK",           mcc_mnc: Some("234/10") },
    CscEntry { code: "TMU",  country: "GBR", operator: "T-Mobile UK",     mcc_mnc: Some("234/30") },
    CscEntry { code: "3UK",  country: "GBR", operator: "Three UK",        mcc_mnc: Some("234/20") },
    CscEntry { code: "VIR",  country: "GBR", operator: "Virgin Mobile UK",mcc_mnc: Some("234/15") },
    CscEntry { code: "VDI",  country: "IRL", operator: "Vodafone Ireland",mcc_mnc: Some("272/01") },
    CscEntry { code: "3IE",  country: "IRL", operator: "Three Ireland",   mcc_mnc: Some("272/05") },
    // Germany / Austria / Switzerland
    CscEntry { code: "DBT",  country: "DEU", operator: "Unlocked Germany",mcc_mnc: None },
    CscEntry { code: "DTM",  country: "DEU", operator: "T-Mobile Germany",mcc_mnc: Some("262/01") },
    CscEntry { code: "VD2",  country: "DEU", operator: "Vodafone Germany",mcc_mnc: Some("262/02") },
    CscEntry { code: "DTL",  country: "DEU", operator: "Deutsche Telekom",mcc_mnc: Some("262/01") },
    CscEntry { code: "ATO",  country: "AUT", operator: "Unlocked Austria",mcc_mnc: None },
    CscEntry { code: "AUT",  country: "AUT", operator: "Austria",         mcc_mnc: Some("232/01") },
    CscEntry { code: "AUO",  country: "AUT", operator: "A1 Telekom",      mcc_mnc: Some("232/01") },
    CscEntry { code: "DRE",  country: "AUT", operator: "Three Austria",   mcc_mnc: Some("232/05") },
    CscEntry { code: "SWC",  country: "CHE", operator: "Swisscom",        mcc_mnc: Some("228/01") },
    CscEntry { code: "AUT",  country: "CHE", operator: "Switzerland",     mcc_mnc: Some("228/01") },
    // France / Spain / Italy
    CscEntry { code: "XEF",  country: "FRA", operator: "Unlocked France", mcc_mnc: None },
    CscEntry { code: "BOG",  country: "FRA", operator: "Bouygues",        mcc_mnc: Some("208/20") },
    CscEntry { code: "ORF",  country: "FRA", operator: "Orange France",   mcc_mnc: Some("208/01") },
    CscEntry { code: "SFR",  country: "FRA", operator: "SFR",             mcc_mnc: Some("208/13") },
    CscEntry { code: "XSP",  country: "FRA", operator: "Spain Unlocked",  mcc_mnc: None },
    CscEntry { code: "PHE",  country: "ESP", operator: "Spain Unlocked",  mcc_mnc: None },
    CscEntry { code: "AMN",  country: "ESP", operator: "Movistar",        mcc_mnc: Some("214/07") },
    CscEntry { code: "EUS",  country: "ESP", operator: "Euskaltel",       mcc_mnc: Some("214/05") },
    CscEntry { code: "YOG",  country: "ESP", operator: "Yoigo",           mcc_mnc: Some("214/04") },
    CscEntry { code: "ATL",  country: "ESP", operator: "Vodafone Spain",  mcc_mnc: Some("214/01") },
    CscEntry { code: "XEH",  country: "HUN", operator: "Unlocked Hungary",mcc_mnc: None },
    CscEntry { code: "ITV",  country: "ITA", operator: "Vodafone Italy",  mcc_mnc: Some("222/10") },
    CscEntry { code: "TIM",  country: "ITA", operator: "TIM",             mcc_mnc: Some("222/01") },
    CscEntry { code: "WIN",  country: "ITA", operator: "Wind",            mcc_mnc: Some("222/88") },
    CscEntry { code: "H3G",  country: "ITA", operator: "Three Italy",     mcc_mnc: Some("222/99") },
    // Nordic
    CscEntry { code: "NEE",  country: "NOR", operator: "Nordic Unlocked", mcc_mnc: None },
    CscEntry { code: "TEN",  country: "NOR", operator: "Telenor",         mcc_mnc: Some("242/01") },
    CscEntry { code: "ELS",  country: "NOR", operator: "Telia Norway",    mcc_mnc: Some("242/02") },
    // Asia
    CscEntry { code: "KOO",  country: "KOR", operator: "KT",              mcc_mnc: Some("450/02") },
    CscEntry { code: "SKT",  country: "KOR", operator: "SK Telecom",      mcc_mnc: Some("450/05") },
    CscEntry { code: "LUC",  country: "KOR", operator: "LG U+",           mcc_mnc: Some("450/06") },
    CscEntry { code: "DCM",  country: "JPN", operator: "NTT DoCoMo",      mcc_mnc: Some("440/10") },
    CscEntry { code: "KDI",  country: "JPN", operator: "KDDI",            mcc_mnc: Some("440/53") },
    CscEntry { code: "SBM",  country: "JPN", operator: "SoftBank",        mcc_mnc: Some("440/20") },
    CscEntry { code: "CHN",  country: "CHN", operator: "China Unlocked",  mcc_mnc: None },
    CscEntry { code: "CHM",  country: "CHN", operator: "China Mobile",    mcc_mnc: Some("460/00") },
    CscEntry { code: "CHU",  country: "CHN", operator: "China Unicom",    mcc_mnc: Some("460/01") },
    CscEntry { code: "CHC",  country: "CHN", operator: "China Telecom",   mcc_mnc: Some("460/03") },
    CscEntry { code: "INS",  country: "IND", operator: "India Unlocked",  mcc_mnc: None },
    CscEntry { code: "INU",  country: "IND", operator: "India IMEI Repair",mcc_mnc: None },
    CscEntry { code: "XTC",  country: "TWN", operator: "Taiwan",          mcc_mnc: Some("466/01") },
    CscEntry { code: "TGY",  country: "HKG", operator: "Hong Kong",       mcc_mnc: Some("454/00") },
    // Oceania
    CscEntry { code: "XSA",  country: "AUS", operator: "Australia Unlocked",mcc_mnc: None },
    CscEntry { code: "OPS",  country: "AUS", operator: "Optus",           mcc_mnc: Some("505/02") },
    CscEntry { code: "TEL",  country: "AUS", operator: "Telstra",         mcc_mnc: Some("505/01") },
    CscEntry { code: "VAU",  country: "AUS", operator: "Vodafone AU",     mcc_mnc: Some("505/03") },
    CscEntry { code: "TLA",  country: "AUS", operator: "Telstra Australia",mcc_mnc: Some("505/01") },
    CscEntry { code: "XNZ",  country: "NZL", operator: "New Zealand",     mcc_mnc: Some("530/01") },
    CscEntry { code: "VNZ",  country: "NZL", operator: "Vodafone NZ",     mcc_mnc: Some("530/01") },
    CscEntry { code: "TNZ",  country: "NZL", operator: "Telecom NZ",      mcc_mnc: Some("530/05") },
    // Russia / Eastern Europe
    CscEntry { code: "SER",  country: "RUS", operator: "Russia",          mcc_mnc: None },
    CscEntry { code: "MTV",  country: "RUS", operator: "MegaFon",         mcc_mnc: Some("250/02") },
    CscEntry { code: "BLN",  country: "RUS", operator: "Beeline",         mcc_mnc: Some("250/99") },
    CscEntry { code: "MTS",  country: "RUS", operator: "MTS",             mcc_mnc: Some("250/01") },
    // Brazil / Latin America
    CscEntry { code: "ZTO",  country: "BRA", operator: "Unlocked Brazil", mcc_mnc: None },
    CscEntry { code: "ZVV",  country: "BRA", operator: "Vivo",            mcc_mnc: Some("724/06") },
    CscEntry { code: "ZTM",  country: "BRA", operator: "TIM Brasil",      mcc_mnc: Some("724/02") },
    CscEntry { code: "ZTR",  country: "BRA", operator: "Claro Brasil",    mcc_mnc: Some("724/05") },
    CscEntry { code: "CHL",  country: "CHL", operator: "Chile",           mcc_mnc: None },
    CscEntry { code: "ARO",  country: "ARG", operator: "Argentina",       mcc_mnc: None },
    CscEntry { code: "UFN",  country: "MEX", operator: "Telcel",          mcc_mnc: Some("334/02") },
    CscEntry { code: "UPM",  country: "MEX", operator: "Movistar Mexico", mcc_mnc: Some("334/03") },
    // Middle East / Africa
    CscEntry { code: "AFR",  country: "ZAF", operator: "South Africa",    mcc_mnc: None },
    CscEntry { code: "MWD",  country: "ARE", operator: "UAE Unlocked",    mcc_mnc: None },
    CscEntry { code: "XSG",  country: "SAU", operator: "Saudi Unlocked",  mcc_mnc: None },
    CscEntry { code: "PAK",  country: "PAK", operator: "Pakistan",        mcc_mnc: None },
];

/// Result of a CSC change request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CscChangeRequest {
    pub udid:         String,
    /// New CSC code to apply (e.g. "XAA" for unlocked-USA).
    pub new_code:     String,
    /// When true, also wipes user data and resets to factory defaults
    /// so the new CSC takes effect cleanly.
    pub factory_reset:bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CscChangeResult {
    pub previous:  Option<String>,
    pub new:       String,
    pub matched_record: Option<CscCode>,
    pub reboot_required: bool,
}

/// Number of entries in the compile-time CSC catalogue.
pub fn csc_database_len() -> usize { CSC_DATABASE.len() }

/// Iterate over the catalogue as owned `CscCode` values — convenient for
/// the FFI where every value must be Serialize-friendly.
pub fn all_csc_codes() -> Vec<CscCode> {
    CSC_DATABASE.iter().map(|e| CscCode::from(e)).collect()
}

/// Look up a code in the database — case-insensitive.
pub fn lookup_csc(code: &str) -> Option<CscCode> {
    let needle = code.to_uppercase();
    CSC_DATABASE.iter()
        .find(|c| c.code == needle.as_str())
        .map(CscCode::from)
}

/// Filter the catalogue by country code or operator substring.
pub fn search_csc(query: &str) -> Vec<CscCode> {
    let q = query.to_uppercase();
    CSC_DATABASE
        .iter()
        .filter(|c| c.country.contains(&q)
                || c.operator.to_uppercase().contains(&q)
                || c.code.contains(&q))
        .map(CscCode::from)
        .collect()
}

/// Validate a CSC string — must be 3 uppercase ASCII letters/digits.
pub fn validate_csc(code: &str) -> Result<()> {
    if code.len() != 3 {
        return Err(ChimeraError::Unknown(format!(
            "CSC must be exactly 3 characters, got {} ({})", code.len(), code
        )));
    }
    if !code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
        return Err(ChimeraError::Unknown(format!(
            "CSC must be uppercase ASCII + digits, got {}", code
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csc_database_not_empty() {
        assert!(CSC_DATABASE.len() >= 80);
    }

    #[test]
    fn lookup_xaa_found() {
        let r = lookup_csc("XAA").unwrap();
        assert_eq!(r.country, "USA");
    }

    #[test]
    fn lookup_lowercase_works() {
        assert!(lookup_csc("vzw").is_some());
        assert!(lookup_csc("TMB").is_some());
    }

    #[test]
    fn search_by_country() {
        let r = search_csc("USA");
        assert!(r.len() >= 5);  // ATT, TMB, VZW, SPR, USC, XAA at minimum
    }

    #[test]
    fn validates_format() {
        assert!(validate_csc("XAA").is_ok());
        assert!(validate_csc("xaa").is_err());        // lowercase
        assert!(validate_csc("XAAA").is_err());       // too long
        assert!(validate_csc("XA").is_err());         // too short
        assert!(validate_csc("XA!").is_err());        // punctuation
    }
}
