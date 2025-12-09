//! Secure storage module for encrypting sensitive data (private keys)
//!
//! Uses AES-256-GCM encryption with machine-derived keys.
//! The encryption key is derived from the machine's unique ID + app salt,
//! so encrypted data can only be decrypted on the same machine.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// Salt used for key derivation - app-specific to prevent rainbow attacks
const APP_SALT: &[u8] = b"screenerbot-wallet-encryption-v1";

/// Encrypted data with nonce for AES-256-GCM
#[derive(Debug, Clone)]
pub struct EncryptedData {
    /// Base64-encoded ciphertext (includes auth tag)
    pub ciphertext: String,
    /// Base64-encoded 12-byte nonce
    pub nonce: String,
}

/// Derive a 256-bit encryption key from machine ID
///
/// Uses BLAKE3 to hash: machine_id + app_salt → 32-byte key
fn derive_encryption_key() -> Result<[u8; 32], String> {
    // Get machine unique ID
    let machine_id = machine_uid::get()
        .map_err(|e| format!("Failed to get machine ID: {}", e))?;

    // Derive key using BLAKE3: hash(machine_id || salt)
    let mut hasher = blake3::Hasher::new();
    hasher.update(machine_id.as_bytes());
    hasher.update(APP_SALT);
    
    let hash = hasher.finalize();
    let key: [u8; 32] = *hash.as_bytes();
    
    Ok(key)
}

/// Encrypt a private key string using AES-256-GCM
///
/// # Arguments
/// * `plaintext` - The private key to encrypt (base58 string)
///
/// # Returns
/// * `EncryptedData` containing base64-encoded ciphertext and nonce
pub fn encrypt_private_key(plaintext: &str) -> Result<EncryptedData, String> {
    let key = derive_encryption_key()?;
    
    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;
    
    // Generate random 12-byte nonce
    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;
    
    Ok(EncryptedData {
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(nonce_bytes),
    })
}

/// Decrypt a private key using AES-256-GCM
///
/// # Arguments
/// * `encrypted` - The encrypted data (ciphertext + nonce)
///
/// # Returns
/// * The decrypted private key string
pub fn decrypt_private_key(encrypted: &EncryptedData) -> Result<String, String> {
    let key = derive_encryption_key()?;
    
    // Decode base64
    let ciphertext = BASE64
        .decode(&encrypted.ciphertext)
        .map_err(|e| format!("Failed to decode ciphertext: {}", e))?;
    
    let nonce_bytes = BASE64
        .decode(&encrypted.nonce)
        .map_err(|e| format!("Failed to decode nonce: {}", e))?;
    
    if nonce_bytes.len() != 12 {
        return Err(format!(
            "Invalid nonce length: expected 12 bytes, got {}",
            nonce_bytes.len()
        ));
    }
    
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Failed to create cipher: {}", e))?;
    
    // Decrypt
    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Decryption failed - wrong machine or corrupted data".to_string())?;
    
    String::from_utf8(plaintext_bytes)
        .map_err(|e| format!("Decrypted data is not valid UTF-8: {}", e))
}

/// Check if encrypted wallet data is present and valid
pub fn has_encrypted_wallet(ciphertext: &str, nonce: &str) -> bool {
    !ciphertext.is_empty() && !nonce.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let original = "test_private_key_base58_encoded_string";
        
        let encrypted = encrypt_private_key(original).expect("Encryption should succeed");
        
        assert!(!encrypted.ciphertext.is_empty());
        assert!(!encrypted.nonce.is_empty());
        assert_ne!(encrypted.ciphertext, original); // Should be different
        
        let decrypted = decrypt_private_key(&encrypted).expect("Decryption should succeed");
        
        assert_eq!(decrypted, original);
    }
}
