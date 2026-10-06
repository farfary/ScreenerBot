// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Shared helpers for the Solana pool account decoders and the analyzer.
//!
//! - Pubkey parsing: reading a 32-byte pubkey out of raw account data and
//!   formatting/round-tripping it through `Pubkey`. Fixed-width integer reads
//!   live in `crate::chains::solana::layout`.
//! - SOL pairing: SOL and stablecoin mint detection, and token-pair orientation
//!   (TOKEN/SOL vs SOL/TOKEN) with the matching vault order, so the analyzer
//!   and the decoders pair vaults identically.

use super::types::{PoolMintVaultInfo, TokenPairInfo};
use crate::chains::adapter::ChainAdapter;
use crate::chains::solana::adapter::ADAPTER;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::{Error, Result};
use crate::logger::{self, LogTag};

/// Read a pubkey from data at given offset, advancing the offset
pub fn read_pubkey_at_offset(data: &[u8], offset: &mut usize) -> Result<String> {
    if *offset + 32 > data.len() {
        return Err(Error::Decode {
            payload: "pool account pubkey",
            detail: format!("offset {} + 32 exceeds data length {}", *offset, data.len()),
        });
    }

    let pubkey_bytes = &data[*offset..*offset + 32];
    *offset += 32;

    let pubkey = Pubkey::new_from_array(pubkey_bytes.try_into().map_err(|_| Error::Decode {
        payload: "pool account pubkey",
        detail: "invalid pubkey bytes".to_owned(),
    })?);

    Ok(pubkey.to_string())
}

/// Read a pubkey from data at fixed offset without advancing
pub fn read_pubkey_at(data: &[u8], offset: usize) -> Option<String> {
    if offset + 32 > data.len() {
        return None;
    }
    let pk = Pubkey::new_from_array(data[offset..offset + 32].try_into().ok()?);
    Some(pk.to_string())
}

/// Read a pubkey as Pubkey struct from data at given offset, advancing the offset
pub fn read_pubkey_struct_at_offset(
    data: &[u8],
    offset: &mut usize,
) -> std::result::Result<Pubkey, &'static str> {
    if data.len() < *offset + 32 {
        return Err("Insufficient data for pubkey");
    }
    let pubkey_bytes = &data[*offset..*offset + 32];
    *offset += 32;
    Pubkey::try_from(pubkey_bytes).map_err(|_| "Invalid pubkey")
}

impl TokenPairInfo {
    /// Create a new TokenPairInfo for invalid pairs (non-SOL)
    pub fn invalid(reason: String) -> Self {
        logger::debug(
            LogTag::PoolService,
            &format!("Invalid token pair: {reason}"),
        );

        Self {
            token_mint: String::new(),
            sol_mint: ADAPTER.native_asset_address().to_string(),
            token_vault: String::new(),
            sol_vault: String::new(),
            sol_is_first: false,
            is_native_pair: false,
        }
    }
}

/// Check if a mint address represents SOL (wrapped SOL or system program)
pub fn is_sol_mint(mint: &str) -> bool {
    ADAPTER.is_native_asset(mint)
}

/// Check if a mint address is a stablecoin that we should skip
pub fn is_stablecoin_mint(mint: &str) -> bool {
    ADAPTER.is_stable_asset(mint)
}

/// Normalize SOL mint to wrapped SOL format
pub fn normalize_sol_mint(mint: &str) -> String {
    ADAPTER.normalize_native_asset(mint)
}

/// Determine if a token pair is SOL-based and extract the correct token/vault pairing
///
/// This function handles all possible configurations:
/// - TOKEN/SOL (token as base, SOL as quote)
/// - SOL/TOKEN (SOL as base, token as quote)
/// - Rejects stablecoin pairs (USDC, USDT, etc.)
/// - Rejects non-SOL pairs
///
/// Returns TokenPairInfo with correct pairing for price calculation
pub fn analyze_token_pair(pool_info: PoolMintVaultInfo) -> TokenPairInfo {
    let mint1 = &pool_info.mint1;
    let mint2 = &pool_info.mint2;
    let vault1 = &pool_info.vault1;
    let vault2 = &pool_info.vault2;

    logger::debug(
        LogTag::PoolService,
        &format!(
            "Analyzing token pair: mint1={}, mint2={}, vault1={}, vault2={}",
            &mint1[..8],
            &mint2[..8],
            &vault1[..8],
            &vault2[..8]
        ),
    );

    // Check for stablecoin pairs - reject these
    if is_stablecoin_mint(mint1) {
        return TokenPairInfo::invalid(format!("Mint1 is stablecoin: {}", &mint1[..8]));
    }
    if is_stablecoin_mint(mint2) {
        return TokenPairInfo::invalid(format!("Mint2 is stablecoin: {}", &mint2[..8]));
    }

    // Determine SOL pairing
    let (token_mint, sol_mint, token_vault, sol_vault, sol_is_first) = if is_sol_mint(mint1) {
        // mint1 is SOL, mint2 is token: SOL/TOKEN configuration
        if is_sol_mint(mint2) {
            // Both are SOL variants - invalid
            return TokenPairInfo::invalid("Both mints are SOL variants".to_owned());
        }
        (
            mint2.clone(),
            normalize_sol_mint(mint1),
            vault2.clone(),
            vault1.clone(),
            true, // SOL is first
        )
    } else if is_sol_mint(mint2) {
        // mint2 is SOL, mint1 is token: TOKEN/SOL configuration
        (
            mint1.clone(),
            normalize_sol_mint(mint2),
            vault1.clone(),
            vault2.clone(),
            false, // SOL is second
        )
    } else {
        // Neither mint is SOL - not a SOL-based pair
        return TokenPairInfo::invalid(format!(
            "No SOL mint found: mint1={}, mint2={}",
            &mint1[..8],
            &mint2[..8]
        ));
    };

    logger::debug(
        LogTag::PoolService,
        &format!(
            "Valid SOL pair: token={}, sol_is_first={}, token_vault={}, sol_vault={}",
            &token_mint[..8],
            sol_is_first,
            &token_vault[..8],
            &sol_vault[..8]
        ),
    );

    TokenPairInfo {
        token_mint,
        sol_mint,
        token_vault,
        sol_vault,
        sol_is_first,
        is_native_pair: true,
    }
}

/// Get the correct vault addresses for analyzer extraction
///
/// This function ensures the analyzer extracts vaults in the same order
/// that the decoder expects them to be in
pub fn get_analyzer_vault_order(pool_info: PoolMintVaultInfo) -> Vec<String> {
    let pair_info = analyze_token_pair(pool_info);

    if !pair_info.is_native_pair {
        // Return empty if not a valid SOL pair
        return vec![];
    }

    // Return vaults in the order: [token_vault, sol_vault]
    // This matches what the decoder expects to find
    vec![pair_info.token_vault, pair_info.sol_vault]
}

/// Validate that a pool contains SOL and return normalized token pair
///
/// This is the main validation function that both analyzer and decoder should use
pub fn validate_sol_pool(pool_info: PoolMintVaultInfo) -> Result<TokenPairInfo> {
    let pair_info = analyze_token_pair(pool_info);

    if !pair_info.is_native_pair {
        Err(Error::InvalidPool {
            reason: "pool does not contain SOL as base or quote".to_owned(),
        })
    } else {
        Ok(pair_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_pubkey_at_offset_advances_and_round_trips() {
        let pk = Pubkey::new_unique();
        let mut data = vec![0u8; 4];
        data.extend_from_slice(&pk.to_bytes());
        let mut offset = 4;
        let read = read_pubkey_at_offset(&data, &mut offset).unwrap();
        assert_eq!(read, pk.to_string());
        assert_eq!(offset, 36);
    }

    #[test]
    fn read_pubkey_at_offset_rejects_short_data() {
        let data = vec![0u8; 10];
        let mut offset = 0;
        assert!(read_pubkey_at_offset(&data, &mut offset).is_err());
    }

    #[test]
    fn read_pubkey_at_does_not_advance() {
        let pk = Pubkey::new_unique();
        let data = pk.to_bytes().to_vec();
        assert_eq!(read_pubkey_at(&data, 0), Some(pk.to_string()));
    }

    #[test]
    fn read_pubkey_struct_at_offset_returns_pubkey_type() {
        let pk = Pubkey::new_unique();
        let data = pk.to_bytes().to_vec();
        let mut offset = 0;
        let read = read_pubkey_struct_at_offset(&data, &mut offset).unwrap();
        assert_eq!(read, pk);
        assert_eq!(offset, 32);
    }
}
