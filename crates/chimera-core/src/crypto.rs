// chimera-core/src/crypto.rs
// Cryptographic helpers: AES encryption for backups, certificate operations, hash utilities

use crate::error::{ChimeraError, Result};
use aes::Aes256;
use aes::cipher::{BlockCipherEncrypt, BlockCipherDecrypt, KeyInit};
use sha2::{Sha256, Digest as Sha2Digest};

/// Derive a 256-bit AES key from a password using SHA-256
pub fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(password.as_bytes());
    h.update(salt);
    h.update(b"chimera_key_derivation_v1");
    let result = h.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Encrypt a single 16-byte block with AES-256-ECB
pub fn aes256_encrypt_block(data: &[u8; 16], key: &[u8; 32]) -> [u8; 16] {
    let cipher = Aes256::new_from_slice(key).expect("32-byte key");
    let mut block = aes::Block::try_from(&data[..]).expect("16-byte block");
    cipher.encrypt_block(&mut block);
    block.into()
}

/// Decrypt a single 16-byte block with AES-256-ECB
pub fn aes256_decrypt_block(data: &[u8; 16], key: &[u8; 32]) -> [u8; 16] {
    let cipher = Aes256::new_from_slice(key).expect("32-byte key");
    let mut block = aes::Block::try_from(&data[..]).expect("16-byte block");
    cipher.decrypt_block(&mut block);
    block.into()
}

/// Encrypt arbitrary data with AES-256-ECB (PKCS7 padding)
pub fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let cipher = Aes256::new_from_slice(key).expect("32-byte key");
    let mut padded = data.to_vec();
    let pad_len = 16 - (padded.len() % 16);
    padded.extend(std::iter::repeat(pad_len as u8).take(pad_len));
    let mut out = Vec::with_capacity(padded.len());
    for chunk in padded.chunks(16) {
        let mut block = aes::Block::try_from(chunk).expect("16-byte chunk");
        cipher.encrypt_block(&mut block);
        let arr: [u8; 16] = block.into();
        out.extend_from_slice(&arr);
    }
    out
}

/// Decrypt arbitrary data with AES-256-ECB (PKCS7 unpadding)
pub fn decrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if data.len() % 16 != 0 || data.is_empty() {
        return Err(ChimeraError::Parse("Invalid ciphertext length".into()));
    }
    let cipher = Aes256::new_from_slice(key).expect("32-byte key");
    let mut out = Vec::with_capacity(data.len());
    for chunk in data.chunks(16) {
        let mut block = aes::Block::try_from(chunk).expect("16-byte chunk");
        cipher.decrypt_block(&mut block);
        let arr: [u8; 16] = block.into();
        out.extend_from_slice(&arr);
    }
    let pad_len = *out.last().unwrap_or(&0) as usize;
    if pad_len == 0 || pad_len > 16 {
        return Err(ChimeraError::Parse("Invalid PKCS7 padding".into()));
    }
    out.truncate(out.len() - pad_len);
    Ok(out)
}

/// Compute SHA-256 hash as hex string
pub fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

/// Compute MD5 hash as hex string
pub fn md5_hex(data: &[u8]) -> String {
    use md5::Md5;
    use md5::Digest;
    let mut h = Md5::new();
    h.update(data);
    hex::encode(h.finalize())
}

/// CRC32 checksum
pub fn crc32(data: &[u8]) -> u32 {
    crc32fast::hash(data)
}

/// Verify a SHA-256 checksum
pub fn verify_sha256(data: &[u8], expected_hex: &str) -> Result<()> {
    let actual = sha256_hex(data);
    if actual.to_lowercase() == expected_hex.to_lowercase() {
        Ok(())
    } else {
        Err(ChimeraError::ChecksumMismatch {
            expected: expected_hex.to_string(),
            actual,
        })
    }
}

/// Verify an MD5 checksum
pub fn verify_md5(data: &[u8], expected_hex: &str) -> Result<()> {
    let actual = md5_hex(data);
    if actual.to_lowercase() == expected_hex.to_lowercase() {
        Ok(())
    } else {
        Err(ChimeraError::ChecksumMismatch {
            expected: expected_hex.to_string(),
            actual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_encrypt_decrypt() {
        let key = derive_key("test_password", b"salt1234");
        let plaintext = b"Hello, ChimeraRS!";
        let encrypted = encrypt_data(plaintext, &key);
        let decrypted = decrypt_data(&encrypted, &key).unwrap();
        assert_eq!(&decrypted, plaintext);
    }
    #[test]
    fn test_sha256() {
        let hash = sha256_hex(b"");
        assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }
}
