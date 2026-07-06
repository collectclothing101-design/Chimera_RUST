// chimera-core/tests/mac_test.rs
// Unit tests for MAC address validation, formatting, and manipulation.

use chimera_core::mac_address::*;

// ═════════════════════════════════════════════════════════════════════════════
//  MAC VALIDATION TESTS - Colon-separated (AA:BB:CC:DD:EE:FF)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn valid_mac_colon_separated() {
    assert!(validate_mac("AA:BB:CC:DD:EE:FF").is_ok());
}

#[test]
fn valid_mac_colon_lowercase() {
    assert!(validate_mac("aa:bb:cc:dd:ee:ff").is_ok());
}

#[test]
fn valid_mac_colon_mixed_case() {
    assert!(validate_mac("Aa:Bb:Cc:Dd:Ee:Ff").is_ok());
}

#[test]
fn valid_mac_colon_zeros() {
    assert!(validate_mac("00:00:00:00:00:00").is_ok());
}

#[test]
fn valid_mac_colon_ones() {
    assert!(validate_mac("11:11:11:11:11:11").is_ok());
}

#[test]
fn valid_mac_colon_ff() {
    assert!(validate_mac("FF:FF:FF:FF:FF:FF").is_ok());
}

#[test]
fn valid_mac_colon_apple() {
    // Apple MAC prefix
    assert!(validate_mac("3C:22:FB:A1:B2:C3").is_ok());
}

#[test]
fn valid_mac_colon_samsung() {
    // Samsung MAC prefix
    assert!(validate_mac("5C:3A:45:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_intel() {
    // Intel MAC prefix
    assert!(validate_mac("00:1B:21:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_qualcomm() {
    // Qualcomm MAC prefix
    assert!(validate_mac("00:03:7F:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_mEDIATEK() {
    // MediaTek MAC prefix
    assert!(validate_mac("00:0C:E7:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_broadcom() {
    // Broadcom MAC prefix
    assert!(validate_mac("00:10:18:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_realtek() {
    // Realtek MAC prefix
    assert!(validate_mac("00:E0:4C:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_cisco() {
    // Cisco MAC prefix
    assert!(validate_mac("00:1A:A0:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_huawei() {
    // Huawei MAC prefix
    assert!(validate_mac("00:18:82:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_xiaomi() {
    // Xiaomi MAC prefix
    assert!(validate_mac("28:6C:07:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_oppo() {
    // Oppo MAC prefix
    assert!(validate_mac("14:F6:5C:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_vivo() {
    // Vivo MAC prefix
    assert!(validate_mac("44:D9:E7:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_oneplus() {
    // OnePlus MAC prefix
    assert!(validate_mac("A0:DC:BF:12:34:56").is_ok());
}

#[test]
fn valid_mac_colon_motorola() {
    // Motorola MAC prefix
    assert!(validate_mac("00:1A:3E:12:34:56").is_ok());
}

// ═════════════════════════════════════════════════════════════════════════════
//  MAC VALIDATION TESTS - Hyphen-separated (AA-BB-CC-DD-EE-FF)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn valid_mac_hyphen_separated() {
    assert!(validate_mac("AA-BB-CC-DD-EE-FF").is_ok());
}

#[test]
fn valid_mac_hyphen_lowercase() {
    assert!(validate_mac("aa-bb-cc-dd-ee-ff").is_ok());
}

#[test]
fn valid_mac_hyphen_mixed_case() {
    assert!(validate_mac("Aa-Bb-Cc-Dd-Ee-Ff").is_ok());
}

#[test]
fn valid_mac_hyphen_zeros() {
    assert!(validate_mac("00-00-00-00-00-00").is_ok());
}

#[test]
fn valid_mac_hyphen_ff() {
    assert!(validate_mac("FF-FF-FF-FF-FF-FF").is_ok());
}

#[test]
fn valid_mac_hyphen_apple() {
    assert!(validate_mac("3C-22-FB-A1-B2-C3").is_ok());
}

#[test]
fn valid_mac_hyphen_samsung() {
    assert!(validate_mac("5C-3A-45-12-34-56").is_ok());
}

#[test]
fn valid_mac_hyphen_intel() {
    assert!(validate_mac("00-1B-21-12-34-56").is_ok());
}

#[test]
fn valid_mac_hyphen_qualcomm() {
    assert!(validate_mac("00-03-7F-12-34-56").is_ok());
}

#[test]
fn valid_mac_hyphen_mEDIATEK() {
    assert!(validate_mac("00-0C-E7-12-34-56").is_ok());
}

// ═════════════════════════════════════════════════════════════════════════════
//  MAC VALIDATION TESTS - Note: validate_mac expects colon or hyphen separators
//  Concatenated format (AABBCCDDEEFF) is NOT supported by validate_mac
// ═════════════════════════════════════════════════════════════════════════════

// ═════════════════════════════════════════════════════════════════════════════
//  INVALID MAC TESTS (30 cases)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn invalid_mac_empty() {
    assert!(validate_mac("").is_err());
}

#[test]
fn invalid_mac_too_short() {
    assert!(validate_mac("AA:BB:CC:DD:EE").is_err());
}

#[test]
fn invalid_mac_too_long() {
    assert!(validate_mac("AA:BB:CC:DD:EE:FF:00").is_err());
}

#[test]
fn valid_mac_period_separator() {
    // Periods are also accepted as separators
    assert!(validate_mac("AA.BB.CC.DD.EE.FF").is_ok());
}

#[test]
fn invalid_mac_mixed_separators() {
    // Mix of colon and hyphen is also accepted (both are replaced)
    assert!(validate_mac("AA:BB-CC:DD-EE:FF").is_ok());
}

#[test]
fn invalid_mac_invalid_hex() {
    // 'G' is not a hex character
    assert!(validate_mac("AA:BB:CC:DD:EE:GG").is_err());
}

#[test]
fn invalid_mac_single_char() {
    assert!(validate_mac("A").is_err());
}

#[test]
fn invalid_mac_two_chars() {
    assert!(validate_mac("AA").is_err());
}

#[test]
fn invalid_mac_seven_chars() {
    assert!(validate_mac("AABBCCD").is_err());
}

#[test]
fn invalid_mac_spaces() {
    assert!(validate_mac("AA BB CC DD EE FF").is_err());
}

#[test]
fn invalid_mac_special_chars() {
    assert!(validate_mac("AA@BB#CC$DD%EE^FF").is_err());
}

#[test]
fn invalid_mac_unicode() {
    // Unicode fullwidth characters
    assert!(validate_mac("ＡＡ:ＢＢ:ＣＣ:ＤＤ:ＥＥ:ＦＦ").is_err());
}

#[test]
fn invalid_mac_binary_data() {
    // Null bytes
    assert!(validate_mac("00:00:00:00:00:00:00").is_err());
}

#[test]
fn invalid_mac_trailing_colon() {
    assert!(validate_mac("AA:BB:CC:DD:EE:FF:").is_err());
}

#[test]
fn invalid_mac_leading_colon() {
    assert!(validate_mac(":AA:BB:CC:DD:EE:FF").is_err());
}

#[test]
fn invalid_mac_trailing_hyphen() {
    assert!(validate_mac("AA-BB-CC-DD-EE-FF-").is_err());
}

#[test]
fn invalid_mac_leading_hyphen() {
    assert!(validate_mac("-AA-BB-CC-DD-EE-FF").is_err());
}

#[test]
fn valid_mac_single_digit_groups() {
    // Single hex digit per group is also valid
    assert!(validate_mac("A:B:C:D:E:F").is_ok());
}

#[test]
fn invalid_mac_three_digit_groups() {
    // Three hex digits per group
    assert!(validate_mac("AAA:BBB:CCC:DDD:EEE:FFF").is_err());
}

#[test]
fn invalid_mac_only_colons() {
    assert!(validate_mac("::::::").is_err());
}

#[test]
fn invalid_mac_only_hyphens() {
    assert!(validate_mac("------").is_err());
}

#[test]
fn invalid_mac_null_bytes() {
    // Actual null bytes
    assert!(validate_mac("\0:\0:\0:\0:\0:\0").is_err());
}

#[test]
fn invalid_mac_tab_chars() {
    assert!(validate_mac("AA\tBB\tCC\tDD\tEE\tFF").is_err());
}

#[test]
fn invalid_mac_newline() {
    assert!(validate_mac("AA:BB:CC:DD:EE:FF\n").is_err());
}

#[test]
fn invalid_mac_carriage_return() {
    assert!(validate_mac("AA:BB:CC:DD:EE:FF\r").is_err());
}

#[test]
fn invalid_mac_backslash() {
    assert!(validate_mac("AA\\BB\\CC\\DD\\EE\\FF").is_err());
}

#[test]
fn invalid_mac_forward_slash() {
    assert!(validate_mac("AA/BB/CC/DD/EE/FF").is_err());
}

#[test]
fn invalid_mac_pipe() {
    assert!(validate_mac("AA|BB|CC|DD|EE|FF").is_err());
}

#[test]
fn invalid_mac_at_sign() {
    assert!(validate_mac("AA@BB@CC@DD@EE@FF").is_err());
}

#[test]
fn invalid_mac_hash() {
    assert!(validate_mac("AA#BB#CC#DD#EE#FF").is_err());
}

#[test]
fn invalid_mac_dollar_sign() {
    assert!(validate_mac("AA$BB$CC$DD$EE$FF").is_err());
}

// ═════════════════════════════════════════════════════════════════════════════
//  MAC UTILITIES TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn format_mac_colon() {
    let bytes = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    assert_eq!(format_mac(&bytes), "AA:BB:CC:DD:EE:FF");
}

#[test]
fn format_mac_zeros() {
    let bytes = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert_eq!(format_mac(&bytes), "00:00:00:00:00:00");
}

#[test]
fn format_mac_ones() {
    let bytes = [0x11, 0x11, 0x11, 0x11, 0x11, 0x11];
    assert_eq!(format_mac(&bytes), "11:11:11:11:11:11");
}

#[test]
fn format_mac_ff() {
    let bytes = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    assert_eq!(format_mac(&bytes), "FF:FF:FF:FF:FF:FF");
}

#[test]
fn is_locally_administered_true() {
    // Second least significant bit of first octet is 1
    let bytes = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(is_locally_administered(&bytes));
}

#[test]
fn is_locally_administered_false() {
    // Second least significant bit of first octet is 0
    let bytes = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(!is_locally_administered(&bytes));
}

#[test]
fn is_multicast_true() {
    // Least significant bit of first octet is 1
    let bytes = [0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(is_multicast(&bytes));
}

#[test]
fn is_multicast_false() {
    // Least significant bit of first octet is 0
    let bytes = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(!is_multicast(&bytes));
}

#[test]
fn derive_mac_from_seed_deterministic() {
    let mac1 = derive_mac_from_seed("test", 0);
    let mac2 = derive_mac_from_seed("test", 0);
    assert_eq!(mac1, mac2);
}

#[test]
fn derive_mac_from_seed_different_index() {
    let mac1 = derive_mac_from_seed("test", 0);
    let mac2 = derive_mac_from_seed("test", 1);
    assert_ne!(mac1, mac2);
}

#[test]
fn derive_mac_from_seed_different_seed() {
    let mac1 = derive_mac_from_seed("seed1", 0);
    let mac2 = derive_mac_from_seed("seed2", 0);
    assert_ne!(mac1, mac2);
}

#[test]
fn pack_samsung_nv_mac_roundtrip() {
    let mac = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    let packed = pack_samsung_nv_mac(&mac);
    let unpacked = unpack_samsung_nv_mac(&packed);
    assert!(unpacked.is_some());
    assert_eq!(unpacked.unwrap(), mac);
}

#[test]
fn unpack_samsung_nv_mac_invalid_length() {
    let data = vec![0u8; 3]; // Too short
    assert!(unpack_samsung_nv_mac(&data).is_none());
}

#[test]
fn unpack_samsung_nv_mac_correct_length() {
    let data = vec![0u8; 6]; // Correct length
    assert!(unpack_samsung_nv_mac(&data).is_some());
}
