//! Wallet cryptographic operations
//!
//! Secure wallet generation, encryption, import/export functionality.

use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

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
pub fn generate_and_encrypt_keypair() -> Result<(Keypair, EncryptedData), String> {
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
pub fn parse_private_key(private_key: &str) -> Result<Keypair, String> {
    let trimmed = private_key.trim();

    // Check for JSON array format
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        parse_array_format(trimmed)
    } else {
        parse_base58_format(trimmed)
    }
}

/// Parse private key from array format [1,2,3,...]
fn parse_array_format(private_key: &str) -> Result<Keypair, String> {
    let inner = private_key.trim_start_matches('[').trim_end_matches(']');

    let bytes: Result<Vec<u8>, _> = inner.split(',').map(|s| s.trim().parse::<u8>()).collect();

    let bytes = bytes.map_err(|e| format!("Invalid array format: {}", e))?;

    if bytes.len() != 64 {
        return Err(format!(
            "Invalid key length: expected 64 bytes, got {}",
            bytes.len()
        ));
    }

    Keypair::from_bytes(&bytes).map_err(|e| format!("Invalid keypair bytes: {}", e))
}

/// Parse private key from base58 format
fn parse_base58_format(private_key: &str) -> Result<Keypair, String> {
    let decoded = bs58::decode(private_key)
        .into_vec()
        .map_err(|e| format!("Invalid base58 encoding: {}", e))?;

    if decoded.len() != 64 {
        return Err(format!(
            "Invalid key length: expected 64 bytes, got {}",
            decoded.len()
        ));
    }

    Keypair::from_bytes(&decoded).map_err(|e| format!("Invalid keypair bytes: {}", e))
}

/// Import a private key and return encrypted data
pub fn import_and_encrypt(private_key: &str) -> Result<(Keypair, EncryptedData), String> {
    let keypair = parse_private_key(private_key)?;

    // Re-encode to base58 for storage (normalized format)
    let private_key_b58 = bs58::encode(keypair.to_bytes()).into_string();
    let encrypted = encrypt_private_key(&private_key_b58)?;

    Ok((keypair, encrypted))
}

/// Export a wallet's private key in base58 format
pub fn export_private_key(encrypted_key: &str, nonce: &str) -> Result<String, String> {
    let encrypted = EncryptedData {
        ciphertext: encrypted_key.to_string(),
        nonce: nonce.to_string(),
    };

    decrypt_private_key(&encrypted)
}

/// Decrypt encrypted key and return keypair
pub fn decrypt_to_keypair(encrypted_key: &str, nonce: &str) -> Result<Keypair, String> {
    let private_key = export_private_key(encrypted_key, nonce)?;
    parse_private_key(&private_key)
}

// =============================================================================
// VALIDATION
// =============================================================================

/// Validate that a string is a valid Solana public key (base58)
pub fn validate_address(address: &str) -> Result<(), String> {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    Pubkey::from_str(address).map_err(|e| format!("Invalid Solana address: {}", e))?;
    Ok(())
}

/// Get the public key address from a keypair
pub fn keypair_to_address(keypair: &Keypair) -> String {
    keypair.pubkey().to_string()
}

/// Verify that a keypair matches an expected address
pub fn verify_keypair_address(keypair: &Keypair, expected_address: &str) -> Result<(), String> {
    let actual = keypair_to_address(keypair);
    if actual != expected_address {
        return Err(format!(
            "Keypair address mismatch: expected {}, got {}",
            expected_address, actual
        ));
    }
    Ok(())
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
