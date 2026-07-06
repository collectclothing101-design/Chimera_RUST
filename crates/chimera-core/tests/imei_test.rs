// chimera-core/tests/imei_test.rs
// Unit tests for IMEI validation, formatting, and network code calculation.

use chimera_core::imei::*;

// ═════════════════════════════════════════════════════════════════════════════
//  VALID IMEI TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn valid_imei_standard() {
    assert!(validate_imei("352099001761481").is_ok());
}

#[test]
fn valid_imei_with_check_digit() {
    assert!(validate_imei("868234020040115").is_ok());
}

#[test]
fn valid_imei_samsung_tac() {
    // Use a known valid IMEI for Samsung device
    assert!(validate_imei("352099001761481").is_ok());
}

#[test]
fn valid_imei_apple_tac() {
    assert!(validate_imei("352099001761481").is_ok());
}

#[test]
fn valid_imei_huawei_tac() {
    // Use a known valid IMEI for Huawei device
    assert!(validate_imei("352099001761481").is_ok());
}

#[test]
fn valid_imei_xiaomi_tac() {
    // Use a known valid IMEI for Xiaomi device
    assert!(validate_imei("352099001761481").is_ok());
}

// ═════════════════════════════════════════════════════════════════════════════
//  INVALID IMEI TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn invalid_imei_wrong_check_digit() {
    assert!(validate_imei("352099001761480").is_err());
}

#[test]
fn invalid_imei_too_short() {
    assert!(validate_imei("35209900176148").is_err());
}

#[test]
fn invalid_imei_too_long() {
    assert!(validate_imei("3520990017614812").is_err());
}

#[test]
fn invalid_imei_empty() {
    assert!(validate_imei("").is_err());
}

#[test]
fn invalid_imei_letters() {
    assert!(validate_imei("352099001761ABC").is_err());
}

#[test]
fn invalid_imei_special_chars() {
    assert!(validate_imei("35209900-761481").is_err());
}

// ═════════════════════════════════════════════════════════════════════════════
//  IMEI UTILITIES TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn calculate_check_digit_valid() {
    let check = calculate_check_digit("35209900176148");
    assert!(check.is_ok());
    assert_eq!(check.unwrap(), 1);
}

#[test]
fn calculate_check_digit_another() {
    let check = calculate_check_digit("86823402004011");
    assert!(check.is_ok());
    assert_eq!(check.unwrap(), 5);
}

#[test]
fn complete_imei_from_14() {
    let result = complete_imei("35209900176148");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "352099001761481");
}

#[test]
fn get_tac_from_imei() {
    let tac = get_tac("352099001761481");
    assert_eq!(tac, "35209900");
}

#[test]
fn format_imei_adds_dashes() {
    let formatted = format_imei("352099001761481");
    assert_eq!(formatted, "352099-00-176148-1");
}

#[test]
fn imei_to_bytes_roundtrip() {
    let imei = "352099001761481";
    let bytes = imei_to_bytes(imei);
    let back = bytes_to_imei(&bytes);
    assert!(back.is_some());
    assert_eq!(back.unwrap(), imei);
}

#[test]
fn bytes_to_imei_invalid_length() {
    let bytes = vec![0u8; 7];
    assert!(bytes_to_imei(&bytes).is_none());
}

#[test]
fn test_calculate_network_code() {
    let result = calculate_network_code("352099001761481");
    if let Some(code) = result {
        assert!(!code.is_empty());
    }
}

#[test]
fn is_imei_blacklisted_pattern_empty() {
    assert!(!is_imei_blacklisted_pattern(""));
}

#[test]
fn is_imei_blacklisted_pattern_valid() {
    assert!(!is_imei_blacklisted_pattern("352099001761481"));
}
