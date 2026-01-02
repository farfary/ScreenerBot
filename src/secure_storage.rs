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

/// Get the machine unique ID for key derivation
///
/// On desktop platforms (macOS, Windows, Linux), uses the machine-uid crate.
/// On Android, uses a combination of app data directory hash as a fallback.
fn get_machine_id() -> Result<String, String> {
    #[cfg(not(target_os = "android"))]
    {
        machine_uid::get().map_err(|e| format!("Failed to get machine ID: {}", e))
    }

    #[cfg(target_os = "android")]
    {
        // On Android, we use a hash of the app's unique installation ID
        // This is stored in the app's private data directory and is unique per installation
        use crate::paths::get_data_directory;

        let data_dir = get_data_directory();

        // Create a unique ID based on the data directory path and a stored UUID
        let id_file = data_dir.join(".device_id");

        if id_file.exists() {
            std::fs::read_to_string(&id_file)
                .map_err(|e| format!("Failed to read device ID: {}", e))
        } else {
            // Generate a new UUID for this installation
            let new_id = uuid::Uuid::new_v4().to_string();

            // Ensure directory exists
            if let Some(parent) = id_file.parent() {
                std::fs::create_dir_all(parent).ok();
            }

            std::fs::write(&id_file, &new_id)
                .map_err(|e| format!("Failed to write device ID: {}", e))?;

            Ok(new_id)
        }
    }
}

/// Derive a 256-bit encryption key from machine ID
///
/// Uses BLAKE3 to hash: machine_id + app_salt → 32-byte key
fn derive_encryption_key() -> Result<[u8; 32], String> {
    // Get machine unique ID
    let machine_id = get_machine_id()?;

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
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| format!("Failed to create cipher: {}", e))?;

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
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| format!("Failed to create cipher: {}", e))?;

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

// =============================================================================
// PASSWORD HASHING FOR LOCKSCREEN
// =============================================================================

/// Generate a random 16-byte salt for password hashing
///
/// # Returns
/// Base64-encoded salt string
pub fn generate_password_salt() -> String {
    let salt: [u8; 16] = rand::random();
    BASE64.encode(salt)
}

/// Hash a password using BLAKE3 with salt
///
/// Uses BLAKE3 keyed hash for password hashing which provides:
/// - Fast hashing (important for PIN verification UX)
/// - Cryptographic security
/// - Resistance to rainbow table attacks (via salt)
///
/// # Arguments
/// * `password` - The plaintext password to hash
/// * `salt` - Base64-encoded salt
///
/// # Returns
/// Base64-encoded hash string
pub fn hash_password(password: &str, salt: &str) -> Result<String, String> {
    let salt_bytes = BASE64
        .decode(salt)
        .map_err(|e| format!("Invalid salt encoding: {}", e))?;

    // Derive a key from salt for keyed hashing
    let mut key = [0u8; 32];
    let mut key_hasher = blake3::Hasher::new();
    key_hasher.update(&salt_bytes);
    key_hasher.update(b"screenerbot-lockscreen-v1");
    let key_hash = key_hasher.finalize();
    key.copy_from_slice(key_hash.as_bytes());

    // Hash password with the derived key
    let mut hasher = blake3::Hasher::new_keyed(&key);
    hasher.update(password.as_bytes());
    let hash = hasher.finalize();

    Ok(BASE64.encode(hash.as_bytes()))
}

/// Verify a password against a stored hash using constant-time comparison
///
/// # Arguments
/// * `password` - The password attempt to verify
/// * `salt` - Base64-encoded salt used when hashing
/// * `stored_hash` - Base64-encoded stored hash to compare against
///
/// # Returns
/// `true` if password matches, `false` otherwise
pub fn verify_password(password: &str, salt: &str, stored_hash: &str) -> bool {
    // Hash the attempt
    let attempt_hash = match hash_password(password, salt) {
        Ok(h) => h,
        Err(_) => return false,
    };

    // Constant-time comparison to prevent timing attacks
    constant_time_compare(attempt_hash.as_bytes(), stored_hash.as_bytes())
}

/// Constant-time byte comparison to prevent timing attacks
///
/// Returns true only if both slices have equal length and content.
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    // XOR all bytes and accumulate - result is 0 only if all bytes match
    let result = a
        .iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y));

    result == 0
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
