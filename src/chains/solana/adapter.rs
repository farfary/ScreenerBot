//! The Solana implementation of [`ChainAdapter`].

use crate::chains::{
    adapter::ChainAdapter,
    solana::constants::{
        LAMPORTS_PER_SOL, SOL_DECIMALS, SOL_MINT, SYSTEM_PROGRAM_ID, USDC_MINT, USDT_MINT,
    },
    ChainError, ChainId, ChainMetadata,
};

/// The base58 alphabet (Bitcoin/Solana variant): excludes `0`, `O`, `I`, `l`.
const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// The Solana chain adapter. Stateless; use the [`ADAPTER`] singleton.
pub struct SolanaAdapter;

/// The singleton Solana adapter instance.
pub static ADAPTER: &SolanaAdapter = &SolanaAdapter;

impl ChainAdapter for SolanaAdapter {
    fn id(&self) -> ChainId {
        ChainId::Solana
    }

    fn metadata(&self) -> ChainMetadata {
        super::chain_metadata()
    }

    fn raw_units_per_native(&self) -> u64 {
        LAMPORTS_PER_SOL
    }

    fn validate_address(&self, address: &str) -> Result<(), ChainError> {
        super::accounts::validate_address(address).map_err(|_| ChainError::InvalidAccount {
            chain: ChainId::Solana,
            value: address.to_owned(),
        })
    }

    fn looks_like_address(&self, candidate: &str) -> bool {
        let trimmed = candidate.trim();
        (32..=44).contains(&trimmed.len()) && trimmed.chars().all(|c| BASE58_ALPHABET.contains(c))
    }

    fn looks_like_transaction_hash(&self, candidate: &str) -> bool {
        let trimmed = candidate.trim();
        (86..=88).contains(&trimmed.len()) && trimmed.chars().all(|c| BASE58_ALPHABET.contains(c))
    }

    fn native_asset_address(&self) -> &'static str {
        SOL_MINT
    }

    fn native_asset_decimals(&self) -> u8 {
        SOL_DECIMALS
    }

    fn is_native_asset(&self, address: &str) -> bool {
        address == SOL_MINT || address == SYSTEM_PROGRAM_ID
    }

    fn normalize_native_asset(&self, address: &str) -> String {
        if self.is_native_asset(address) {
            SOL_MINT.to_string()
        } else {
            address.to_string()
        }
    }

    fn stable_assets(&self) -> &'static [&'static str] {
        &[USDC_MINT, USDT_MINT]
    }

    fn market_data_network(&self) -> &'static str {
        "solana"
    }

    fn explorer_token_url(&self, address: &str) -> String {
        format!("https://solscan.io/token/{address}")
    }

    fn explorer_account_url(&self, address: &str) -> String {
        format!("https://solscan.io/account/{address}")
    }

    fn explorer_transaction_url(&self, hash: &str) -> String {
        format!("https://solscan.io/tx/{hash}")
    }

    fn dex_chart_url(&self, address: &str) -> String {
        format!("https://dexscreener.com/solana/{address}")
    }

    fn analytics_token_url(&self, address: &str) -> String {
        format!("https://birdeye.so/token/{address}?chain=solana")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real, well-formed ed25519 transaction signature (88 base58 chars).
    const REAL_SIGNATURE: &str =
        "5VERv8NMvzbJMEkV8xnrLkEaWRtSz9CosKDYjCJjBRnbJLgp8uirBgmQpjKhoR4tjF3ZpRzrFmBV6UjKdiSZkQUW";

    #[test]
    fn raw_native_round_trip() {
        let adapter = SolanaAdapter;
        assert_eq!(adapter.native_to_raw(1.0), LAMPORTS_PER_SOL);
        assert_eq!(adapter.raw_to_native(LAMPORTS_PER_SOL), 1.0);

        assert_eq!(adapter.native_to_raw(0.5), LAMPORTS_PER_SOL / 2);
        assert_eq!(adapter.raw_to_native(LAMPORTS_PER_SOL / 2), 0.5);
    }

    #[test]
    fn looks_like_address_shape_check() {
        let adapter = SolanaAdapter;
        assert!(adapter.looks_like_address(SOL_MINT));
        assert!(!adapter.looks_like_address("short1234"));
        assert!(!adapter.looks_like_address("So1111111111111111111111111111111111111110")); // contains '0'
        assert!(!adapter.looks_like_address("OO1111111111111111111111111111111111111112")); // contains 'O'
        assert!(!adapter.looks_like_address("II1111111111111111111111111111111111111112")); // contains 'I'
        assert!(!adapter.looks_like_address("ll1111111111111111111111111111111111111112"));
        // contains 'l'
    }

    #[test]
    fn looks_like_transaction_hash_shape_check() {
        let adapter = SolanaAdapter;
        assert!(adapter.looks_like_transaction_hash(REAL_SIGNATURE));
        assert!(!adapter.looks_like_transaction_hash(SOL_MINT)); // 44 chars, too short
    }

    #[test]
    fn native_asset_recognition() {
        let adapter = SolanaAdapter;
        assert!(adapter.is_native_asset(SOL_MINT));
        assert!(adapter.is_native_asset(SYSTEM_PROGRAM_ID));
        assert!(!adapter.is_native_asset(USDC_MINT));
    }

    #[test]
    fn stable_asset_recognition() {
        let adapter = SolanaAdapter;
        assert!(adapter.is_stable_asset(USDC_MINT));
        assert!(adapter.is_stable_asset(USDT_MINT));
    }

    #[test]
    fn normalize_native_asset_canonicalizes() {
        let adapter = SolanaAdapter;
        assert_eq!(adapter.normalize_native_asset(SYSTEM_PROGRAM_ID), SOL_MINT);
        assert_eq!(adapter.normalize_native_asset(USDC_MINT), USDC_MINT);
    }
}
