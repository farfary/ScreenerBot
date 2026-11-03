// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! AES-256-GCM encryption for private keys with machine-derived keys.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use zeroize::Zeroizing;

use super::{Error, Result};

/// Salt used for key derivation — app-specific to prevent rainbow attacks.
const APP_SALT: &[u8] = b"screenerbot-wallet-encryption-v1";

/// Encrypted data with nonce for AES-256-GCM.
#[derive(Debug, Clone)]
pub struct EncryptedData {
    /// Base64-encoded ciphertext (includes auth tag).
    pub ciphertext: String,
    /// Base64-encoded 12-byte nonce.
    pub nonce: String,
}

/// Get the machine unique ID for key derivation.
fn get_machine_id() -> Result<String> {
    #[cfg(not(target_os = "android"))]
    {
        machine_uid::get().map_err(|error| Error::MachineIdentity {
            detail: error.to_string(),
        })
    }

    #[cfg(target_os = "android")]
    {
        let id_file = crate::paths::get_device_id_path();

        if id_file.exists() {
            std::fs::read_to_string(&id_file).map_err(crate::errors::IoError::from)
        } else {
            let new_id = uuid::Uuid::new_v4().to_string();
            if let Some(parent) = id_file.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&id_file, &new_id).map_err(crate::errors::IoError::from)?;
            Ok(new_id)
        }
    }
}

/// Derive a 256-bit encryption key from machine ID.
///
/// The key is wiped from memory when the returned value is dropped.
fn derive_encryption_key() -> Result<Zeroizing<[u8; 32]>> {
    let machine_id = get_machine_id()?;

    let mut hasher = blake3::Hasher::new();
    hasher.update(machine_id.as_bytes());
    hasher.update(APP_SALT);

    let hash = hasher.finalize();
    Ok(Zeroizing::new(*hash.as_bytes()))
}

/// Encrypt a private key string using AES-256-GCM.
pub fn encrypt_private_key(plaintext: &str) -> Result<EncryptedData> {
    let key = derive_encryption_key()?;

    let cipher = Aes256Gcm::new_from_slice(key.as_slice()).map_err(|error| {
        Error::CryptographicOperation {
            operation: "cipher initialization",
            detail: error.to_string(),
        }
    })?;

    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|error| Error::CryptographicOperation {
            operation: "encryption",
            detail: error.to_string(),
        })?;

    Ok(EncryptedData {
        ciphertext: BASE64.encode(&ciphertext),
        nonce: BASE64.encode(nonce_bytes),
    })
}

/// Decrypt a private key using AES-256-GCM.
///
/// The plaintext is wiped from memory when the returned value is dropped; a
/// caller that needs an owned `String` converts at its own boundary.
pub fn decrypt_private_key(encrypted: &EncryptedData) -> Result<Zeroizing<String>> {
    let key = derive_encryption_key()?;

    let ciphertext =
        BASE64
            .decode(&encrypted.ciphertext)
            .map_err(|error| Error::InvalidEncoding {
                field: "ciphertext",
                detail: error.to_string(),
            })?;

    let nonce_bytes = BASE64
        .decode(&encrypted.nonce)
        .map_err(|error| Error::InvalidEncoding {
            field: "nonce",
            detail: error.to_string(),
        })?;

    if nonce_bytes.len() != 12 {
        return Err(Error::InvalidNonceLength {
            actual: nonce_bytes.len(),
        });
    }

    let nonce = Nonce::from_slice(&nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(key.as_slice()).map_err(|error| {
        Error::CryptographicOperation {
            operation: "cipher initialization",
            detail: error.to_string(),
        }
    })?;

    let plaintext_bytes = Zeroizing::new(
        cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| Error::DecryptionFailed)?,
    );

    let plaintext = std::str::from_utf8(&plaintext_bytes).map_err(|error| Error::InvalidUtf8 {
        detail: error.to_string(),
    })?;
    Ok(Zeroizing::new(plaintext.to_owned()))
}

/// Check if encrypted wallet data is present and valid.
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
        assert_ne!(encrypted.ciphertext, original);

        let decrypted = decrypt_private_key(&encrypted).expect("Decryption should succeed");

        assert_eq!(decrypted.as_str(), original);
    }

    /// A record written by any earlier build must still open: the key is
    /// BLAKE3(machine id, APP_SALT), the cipher AES-256-GCM, and the stored form
    /// standard base64 of the ciphertext with tag and of the 12-byte nonce.
    #[test]
    fn decrypts_a_record_in_the_stored_format() {
        #[cfg(not(target_os = "android"))]
        let machine_id = machine_uid::get().expect("machine id");
        #[cfg(target_os = "android")]
        let machine_id = get_machine_id().expect("machine id");
        let mut hasher = blake3::Hasher::new();
        hasher.update(machine_id.as_bytes());
        hasher.update(b"screenerbot-wallet-encryption-v1");
        let expected_key = *hasher.finalize().as_bytes();

        let key = derive_encryption_key().expect("derive key");
        assert_eq!(*key, expected_key);

        let original = "fixed_vector_private_key_base58";
        let nonce_bytes = [7u8; 12];
        let cipher = Aes256Gcm::new_from_slice(&expected_key).expect("cipher");
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), original.as_bytes())
            .expect("encrypt");
        let stored = EncryptedData {
            ciphertext: BASE64.encode(&ciphertext),
            nonce: BASE64.encode(nonce_bytes),
        };

        let decrypted = decrypt_private_key(&stored).expect("stored record decrypts");
        assert_eq!(decrypted.as_str(), original);
    }
}
