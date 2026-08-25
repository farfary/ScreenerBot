//! Solana keypair generation, parsing, validation and signing primitives.
//!
//! Encryption of the resulting secret is chain-neutral and lives in
//! `crate::secure_storage`; this module owns only the Solana-typed half.

use crate::chains::solana::solana_sdk::signature::Keypair;
use crate::chains::solana::solana_sdk::signer::Signer;
use crate::chains::solana::{Error, Result};

use crate::secure_storage::{decrypt_private_key, encrypt_private_key, EncryptedData};

// =============================================================================
// WALLET GENERATION
// =============================================================================

/// Generate a new Solana keypair using secure random generation
///
/// Uses Solana SDK's Keypair::new() which internally uses a CSPRNG
pub fn generate_keypair() -> Keypair {
    Keypair::new()
}

/// Generate a new keypair and return with its encrypted private key
pub fn generate_and_encrypt_keypair() -> Result<(Keypair, EncryptedData)> {
    let keypair = generate_keypair();
    let private_key_b58 = bs58::encode(keypair.to_bytes()).into_string();
    let encrypted = encrypt_private_key(&private_key_b58)?;
    Ok((keypair, encrypted))
}

// =============================================================================
// IMPORT / EXPORT
// =============================================================================

/// Parse a private key from various formats and return keypair
///
/// Supports:
/// - Base58 encoded (standard Solana format)
/// - JSON array format: [1,2,3,...]
pub fn parse_private_key(private_key: &str) -> Result<Keypair> {
    let trimmed = private_key.trim();

    // Check for JSON array format
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        parse_array_format(trimmed)
    } else {
        parse_base58_format(trimmed)
    }
}

/// Parse private key from array format [1,2,3,...]
fn parse_array_format(private_key: &str) -> Result<Keypair> {
    let inner = private_key.trim_start_matches('[').trim_end_matches(']');

    let bytes: std::result::Result<Vec<u8>, _> =
        inner.split(',').map(|s| s.trim().parse::<u8>()).collect();

    let bytes = bytes.map_err(|e| Error::InvalidKeypair {
        detail: format!("invalid array format: {e}"),
    })?;

    if bytes.len() != 64 {
        return Err(Error::InvalidKeypair {
            detail: format!("invalid key length: expected 64 bytes, got {}", bytes.len()),
        });
    }

    Keypair::from_bytes(&bytes).map_err(|e| Error::InvalidKeypair {
        detail: e.to_string(),
    })
}

/// Parse private key from base58 format
fn parse_base58_format(private_key: &str) -> Result<Keypair> {
    let decoded = bs58::decode(private_key)
        .into_vec()
        .map_err(|e| Error::InvalidKeypair {
            detail: format!("invalid base58 encoding: {e}"),
        })?;

    if decoded.len() != 64 {
        return Err(Error::InvalidKeypair {
            detail: format!(
                "invalid key length: expected 64 bytes, got {}",
                decoded.len()
            ),
        });
    }

    Keypair::from_bytes(&decoded).map_err(|e| Error::InvalidKeypair {
        detail: e.to_string(),
    })
}

/// Import a private key and return encrypted data
pub fn import_and_encrypt(private_key: &str) -> Result<(Keypair, EncryptedData)> {
    let keypair = parse_private_key(private_key)?;

    // Re-encode to base58 for storage (normalized format)
    let private_key_b58 = bs58::encode(keypair.to_bytes()).into_string();
    let encrypted = encrypt_private_key(&private_key_b58)?;

    Ok((keypair, encrypted))
}

/// Export a wallet's private key in base58 format
pub fn export_private_key(encrypted_key: &str, nonce: &str) -> Result<String> {
    let encrypted = EncryptedData {
        ciphertext: encrypted_key.to_string(),
        nonce: nonce.to_string(),
    };

    decrypt_private_key(&encrypted).map_err(Error::from)
}

/// Decrypt encrypted key and return keypair
pub fn decrypt_to_keypair(encrypted_key: &str, nonce: &str) -> Result<Keypair> {
    let private_key = export_private_key(encrypted_key, nonce)?;
    parse_private_key(&private_key)
}

// =============================================================================
// VALIDATION
// =============================================================================

/// Validate that a string is a valid Solana public key (base58)
pub fn validate_address(address: &str) -> Result<()> {
    parse_pubkey_safe(address)?;
    Ok(())
}

/// Parse a pubkey string with consistent error formatting
pub fn parse_pubkey_safe(
    address: &str,
) -> Result<crate::chains::solana::solana_sdk::pubkey::Pubkey> {
    use crate::chains::solana::solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    Pubkey::from_str(address).map_err(|_| Error::InvalidAddress {
        kind: "pubkey",
        value: address.to_owned(),
    })
}

/// Get the public key address from a keypair
pub fn keypair_to_address(keypair: &Keypair) -> String {
    keypair.pubkey().to_string()
}

// =============================================================================
// ADDRESS-ONLY / MATERIAL HELPERS
// =============================================================================
//
// These exist so callers outside `crate::chains::solana` (wallet CRUD, bulk
// import, the legacy config migration) never need to hold a `Keypair` value
// even transiently — they get back the address string and/or the
// chain-neutral `EncryptedData` they actually need to persist or compare.

/// Generate a new wallet's address plus its encrypted key material.
pub fn generate_wallet_material() -> Result<(String, EncryptedData)> {
    let (keypair, encrypted) = generate_and_encrypt_keypair()?;
    Ok((keypair_to_address(&keypair), encrypted))
}

/// Parse and encrypt a user-submitted private key, returning its address
/// plus encrypted key material.
pub fn import_wallet_material(private_key: &str) -> Result<(String, EncryptedData)> {
    let (keypair, encrypted) = import_and_encrypt(private_key)?;
    Ok((keypair_to_address(&keypair), encrypted))
}

/// Derive just the address from a private key string (duplicate checks that
/// don't need to store the key).
pub fn address_from_private_key(private_key: &str) -> Result<String> {
    parse_private_key(private_key).map(|kp| keypair_to_address(&kp))
}

/// Derive just the address from an already-encrypted key (one-time
/// migration paths that don't need the decrypted key afterward).
pub fn address_from_encrypted_key(ciphertext: &str, nonce: &str) -> Result<String> {
    decrypt_to_keypair(ciphertext, nonce).map(|kp| keypair_to_address(&kp))
}

/// Generate a fresh ephemeral keypair and return its address plus base58
/// secret, for tools that hand the user a brand-new burner wallet rather
/// than storing it (the dashboard's wallet generator).
pub fn generate_keypair_strings() -> (String, String) {
    let keypair = generate_keypair();
    let secret = bs58::encode(keypair.to_bytes()).into_string();
    (keypair_to_address(&keypair), secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let kp1 = generate_keypair();
        let kp2 = generate_keypair();

        // Each generated keypair should be unique
        assert_ne!(kp1.pubkey(), kp2.pubkey());
    }

    #[test]
    fn test_generate_and_encrypt() {
        let result = generate_and_encrypt_keypair();
        assert!(result.is_ok());

        let (keypair, encrypted) = result.unwrap();
        assert!(!encrypted.ciphertext.is_empty());
        assert!(!encrypted.nonce.is_empty());

        // Verify we can decrypt back
        let decrypted = decrypt_to_keypair(&encrypted.ciphertext, &encrypted.nonce);
        assert!(decrypted.is_ok());
        assert_eq!(decrypted.unwrap().pubkey(), keypair.pubkey());
    }

    #[test]
    fn test_parse_base58() {
        // This is a test keypair - do not use in production
        let keypair = generate_keypair();
        let b58 = bs58::encode(keypair.to_bytes()).into_string();

        let parsed = parse_private_key(&b58);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().pubkey(), keypair.pubkey());
    }

    #[test]
    fn parsed_keypair_produces_a_verifiable_signature() {
        let keypair = generate_keypair();
        let b58 = bs58::encode(keypair.to_bytes()).into_string();
        let parsed = parse_private_key(&b58).unwrap();

        let message = b"screenerbot-signing-test";
        let signature = parsed.sign_message(message);
        assert!(signature.verify(&parsed.pubkey().to_bytes(), message));
    }

    #[test]
    fn invalid_base58_error_does_not_echo_the_input() {
        let secret_like_but_invalid = "not-a-valid-base58-key-!!!";
        let err = parse_private_key(secret_like_but_invalid)
            .unwrap_err()
            .to_string();
        assert!(
            !err.contains(secret_like_but_invalid),
            "error message must not echo the rejected input back: {err}"
        );
    }

    #[test]
    fn wrong_length_array_error_does_not_echo_byte_values() {
        let bytes = vec![7u8; 32]; // half the required length
        let array_str = format!(
            "[{}]",
            bytes
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        let err = parse_private_key(&array_str).unwrap_err().to_string();
        assert!(err.contains("expected 64 bytes"));
        assert!(
            !err.contains(&array_str),
            "error message must not echo the rejected key bytes back: {err}"
        );
    }

    #[test]
    fn test_parse_array_format() {
        let keypair = generate_keypair();
        let bytes = keypair.to_bytes();
        let array_str = format!(
            "[{}]",
            bytes
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let parsed = parse_private_key(&array_str);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().pubkey(), keypair.pubkey());
    }
}
