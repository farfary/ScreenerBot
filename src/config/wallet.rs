//! Wallet management functions — loading keypairs and public keys.

use super::utils::with_config;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};

/// Load the main wallet keypair from the wallets database
///
/// This function uses the new multi-wallet system. For async code, prefer
/// using `wallets::get_main_keypair()` directly.
///
/// The private key is stored encrypted in the wallets database. This function:
/// 1. Checks if wallets module is initialized
/// 2. Falls back to legacy config.toml if not initialized
/// 3. Returns the main wallet keypair
///
/// # Returns
/// - `Ok(Keypair)` - Successfully retrieved main wallet keypair
/// - `Err(String)` - No main wallet configured or decryption failed
pub fn get_wallet_keypair() -> Result<Keypair, String> {
    // Try the new wallets module first (async -> sync bridge)
    // Use try_read to avoid blocking if we're in an async context
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // We're in an async context - use block_in_place
        return tokio::task::block_in_place(|| {
            handle.block_on(async {
                // Check if wallets module is initialized
                if crate::wallets::is_initialized().await {
                    return crate::wallets::get_main_keypair().await;
                }
                // Fall back to legacy config
                get_wallet_keypair_from_config()
            })
        });
    }

    // Not in async context - create a temporary runtime
    // This should only happen during early startup
    get_wallet_keypair_from_config()
}

/// Legacy function to get wallet keypair from config.toml
///
/// Used as fallback before wallets module is initialized
fn get_wallet_keypair_from_config() -> Result<Keypair, String> {
    with_config(|cfg| {
        // Check if encrypted wallet data exists
        if cfg.wallet_encrypted.is_empty() || cfg.wallet_nonce.is_empty() {
            return Err("Wallet not configured - encrypted private key is missing".to_owned());
        }

        // Decrypt the private key
        let encrypted = crate::secure_storage::EncryptedData {
            ciphertext: cfg.wallet_encrypted.clone(),
            nonce: cfg.wallet_nonce.clone(),
        };

        let private_key = crate::secure_storage::decrypt_private_key(&encrypted)
            .map_err(|e| format!("Failed to decrypt wallet: {e}"))?;

        // Parse the decrypted private key (base58 or array format)
        let keypair = if private_key.starts_with('[') && private_key.ends_with(']') {
            load_keypair_from_array_format(&private_key)?
        } else {
            load_keypair_from_base58_format(&private_key)?
        };

        Ok(keypair)
    })
}

/// Helper function to load keypair from array format
///
/// Parses private key strings in the format "[1,2,3,4,...]"where each
/// number represents a byte value from 0-255.
fn load_keypair_from_array_format(private_key_str: &str) -> Result<Keypair, String> {
    let private_key_str = private_key_str
        .trim_start_matches('[')
        .trim_end_matches(']');

    let private_key_bytes: Result<Vec<u8>, _> = private_key_str
        .split(',')
        .map(|s| s.trim().parse::<u8>())
        .collect();

    match private_key_bytes {
        Ok(bytes) => {
            if bytes.len() != 64 {
                return Err(format!(
                    "Invalid private key length: expected 64 bytes, got {}",
                    bytes.len()
                ));
            }
            Keypair::from_bytes(&bytes)
                .map_err(|e| format!("Failed to create keypair from array: {e}"))
        }
        Err(e) => Err(format!("Failed to parse private key array: {e}")),
    }
}

/// Helper function to load keypair from base58 format
///
/// Parses private key strings in base58 format, which is the standard
/// Solana wallet format used by most tools and libraries.
fn load_keypair_from_base58_format(private_key_str: &str) -> Result<Keypair, String> {
    let decoded = bs58::decode(private_key_str)
        .into_vec()
        .map_err(|e| format!("Failed to decode base58 private key: {e}"))?;

    if decoded.len() != 64 {
        return Err(format!(
            "Invalid private key length: expected 64 bytes, got {}",
            decoded.len()
        ));
    }

    Keypair::from_bytes(&decoded).map_err(|e| format!("Failed to create keypair from base58: {e}"))
}

/// Get the wallet public key from the configuration
///
/// This loads the keypair and extracts just the public key.
///
/// # Returns
/// - `Ok(Pubkey)` - The wallet's public key
/// - `Err(String)` - Failed to load or parse keypair
///
/// # Example
/// ```
/// use screenerbot::config::get_wallet_pubkey;
///
/// let pubkey = get_wallet_pubkey()?;
/// println!("Wallet address: {pubkey}");
/// ```
pub fn get_wallet_pubkey() -> Result<Pubkey, String> {
    get_wallet_keypair().map(|kp| kp.pubkey())
}

/// Get the wallet public key as a base58 string
///
/// This is useful for logging or display purposes where you need to show
/// the wallet address but don't need the Pubkey type.
///
/// # Returns
/// - `Ok(String)` - Base58 encoded public key
/// - `Err(String)` - Failed to load or parse keypair
///
/// # Example
/// ```
/// use screenerbot::config::get_wallet_pubkey_string;
///
/// let address = get_wallet_pubkey_string()?;
/// println!("Wallet address: {address}");
/// ```
pub fn get_wallet_pubkey_string() -> Result<String, String> {
    get_wallet_pubkey().map(|pk| pk.to_string())
}
