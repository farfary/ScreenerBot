//! Wallet address resolution — the one chain-neutral entry point shared
//! code may use for "what is the configured trading wallet".
//!
//! Actual keypair/pubkey resolution (multi-wallet database main wallet, or
//! the legacy single-wallet fallback from `config.toml`) lives in
//! `crate::chains::solana::accounts` (`configured_keypair`,
//! `configured_pubkey`) — this module never returns a Solana-typed value.

/// Get the configured trading wallet's address as a base58 string.
///
/// # Returns
/// - `Ok(String)` - Base58 encoded public key
/// - `Err(String)` - No wallet configured, or decryption failed
pub fn get_wallet_pubkey_string() -> super::Result<String> {
    crate::chains::solana::accounts::configured_address()
        .map_err(|source| super::Error::WalletAddress { source })
}
