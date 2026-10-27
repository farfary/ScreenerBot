// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Solana keypair parsing, validation and signing primitives.
//!
//! Encryption of the resulting secret is chain-neutral and lives in
//! `crate::secure_storage`; this module owns only the Solana-typed half.

use crate::chains::solana::solana_sdk::signature::Keypair;
use crate::chains::solana::solana_sdk::signer::Signer;
use crate::chains::solana::{Error, Result};

use crate::secure_storage::{decrypt_private_key, EncryptedData};

use super::signing::refuse_in_paper_scope;

// =============================================================================
// IMPORT / EXPORT
// =============================================================================

/// Parse a private key from various formats and return keypair
///
/// Supports:
/// - Base58 encoded (standard Solana format)
/// - JSON array format: [1,2,3,...]
pub fn parse_private_key(private_key: &str) -> Result<Keypair> {
    refuse_in_paper_scope("parse_private_key")?;
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

    Keypair::try_from(&bytes[..]).map_err(|e| Error::InvalidKeypair {
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

    Keypair::try_from(&decoded[..]).map_err(|e| Error::InvalidKeypair {
        detail: e.to_string(),
    })
}

/// Export a wallet's private key in base58 format
pub fn export_private_key(encrypted_key: &str, nonce: &str) -> Result<String> {
    refuse_in_paper_scope("export_private_key")?;
    let encrypted = EncryptedData {
        ciphertext: encrypted_key.to_string(),
        nonce: nonce.to_string(),
    };

    decrypt_private_key(&encrypted).map_err(Error::from)
}

/// Decrypt encrypted key and return keypair
pub fn decrypt_to_keypair(encrypted_key: &str, nonce: &str) -> Result<Keypair> {
    refuse_in_paper_scope("decrypt_to_keypair")?;
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
// ADDRESS-ONLY HELPERS
// =============================================================================
//
// These exist so callers outside `crate::chains::solana` (the legacy config
// migration) never need to hold a `Keypair` value even transiently — they get
// back the address string they actually need to persist or compare.

/// Derive just the address from an already-encrypted key (one-time
/// migration paths that don't need the decrypted key afterward).
pub fn address_from_encrypted_key(ciphertext: &str, nonce: &str) -> Result<String> {
    decrypt_to_keypair(ciphertext, nonce).map(|kp| keypair_to_address(&kp))
}

/// A fresh key as its address and its base58 secret, the form the setup flow accepts.
/// Tests outside this module take a trading-wallet key through it and never hold a
/// `Keypair`.
#[cfg(test)]
pub(crate) fn new_test_key() -> (String, String) {
    let keypair = Keypair::new();
    (
        keypair_to_address(&keypair),
        bs58::encode(keypair.to_bytes()).into_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_base58() {
        // This is a test keypair - do not use in production
        let keypair = Keypair::new();
        let b58 = bs58::encode(keypair.to_bytes()).into_string();

        let parsed = parse_private_key(&b58);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().pubkey(), keypair.pubkey());
    }

    #[test]
    fn parsed_keypair_produces_a_verifiable_signature() {
        let keypair = Keypair::new();
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
        let keypair = Keypair::new();
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
