/// Pool utilities for consistent SOL detection and vault pairing across analyzer and decoders
///
/// This module provides centralized logic for:
/// - Detecting SOL mints (wrapped and native forms)
/// - Determining token pair orientation (TOKEN/SOL vs SOL/TOKEN)
/// - Pairing vaults correctly based on mint types
/// - Handling all possible base/quote token combinations
use crate::logger::{self, LogTag};
use crate::constants::{WRAPPED_SOL_MINT, NATIVE_SOL_MINT, USDC_MINT, USDT_MINT};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Result of mint and vault analysis
#[derive(Debug, Clone)]
pub struct TokenPairInfo {
    /// The token mint (non-SOL)
    pub token_mint: String,
    /// The SOL mint (always normalized to wrapped SOL)
    pub sol_mint: String,
    /// Vault address for the token
    pub token_vault: String,
    /// Vault address for SOL
    pub sol_vault: String,
    /// Whether the original pool has SOL as the first mint (affects price calculation)
    pub sol_is_first: bool,
    /// Whether this is a valid SOL-based pair
    pub is_sol_pair: bool,
}

/// Pool mint and vault extraction result
#[derive(Debug, Clone)]
pub struct PoolMintVaultInfo {
    pub mint1: String,
    pub mint2: String,
    pub vault1: String,
    pub vault2: String,
}

impl TokenPairInfo {
    /// Create a new TokenPairInfo for invalid pairs (non-SOL)
    pub fn invalid(reason: String) -> Self {
        logger::debug(
            LogTag::PoolService,
            &format!("Invalid token pair: {}", reason),
        );

        Self {
            token_mint: String::new(),
            sol_mint: WRAPPED_SOL_MINT.to_string(),
            token_vault: String::new(),
            sol_vault: String::new(),
            sol_is_first: false,
            is_sol_pair: false,
        }
    }
}

/// Check if a mint address represents SOL (wrapped or native)
pub fn is_sol_mint(mint: &str) -> bool {
    mint == WRAPPED_SOL_MINT || mint == NATIVE_SOL_MINT
}

/// Check if a mint address is a stablecoin that we should skip
pub fn is_stablecoin_mint(mint: &str) -> bool {
    mint == USDC_MINT || mint == USDT_MINT
}

/// Normalize SOL mint to wrapped SOL format
pub fn normalize_sol_mint(mint: &str) -> String {
    if is_sol_mint(mint) {
        WRAPPED_SOL_MINT.to_string()
    } else {
        mint.to_string()
    }
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
            return TokenPairInfo::invalid("Both mints are SOL variants".to_string());
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
        is_sol_pair: true,
    }
}

/// Data reading utilities for consistent parsing across all decoders
/// These functions provide centralized, safe data extraction with proper bounds checking
pub fn extract_pumpfun_mints_and_vaults(data: &[u8]) -> Option<PoolMintVaultInfo> {
    if data.len() < 200 {
        logger::error(
            LogTag::PoolService,
            &format!("PumpFun pool data too short: {} bytes", data.len()),
        );
        return None;
    }

    logger::debug(
        LogTag::PoolService,
        &format!("Extracting PumpFun pool data ({} bytes)", data.len()),
    );

    // PumpFun AMM structure (confirmed via structure analysis):
    // discriminator(8) + pool_bump(1) + index(2) + creator(32) + base_mint(32) + quote_mint(32) + lp_mint(32) + vault1(32) + vault2(32) + ...
    let mut offset = 8 + 1 + 2 + 32; // Skip discriminator, bump, index, and creator

    // Read base mint and quote mint
    let mint1 = read_pubkey_at_offset(data, &mut offset).ok()?; // base_mint
    let mint2 = read_pubkey_at_offset(data, &mut offset).ok()?; // quote_mint

    // Skip lp_mint
    offset += 32;

    // Read vault addresses
    let vault1 = read_pubkey_at_offset(data, &mut offset).ok()?;
    let vault2 = read_pubkey_at_offset(data, &mut offset).ok()?;

    logger::debug(
        LogTag::PoolService,
        &format!(
            "Extracted PumpFun: mint1={}, mint2={}, vault1={}, vault2={}",
            &mint1[..8],
            &mint2[..8],
            &vault1[..8],
            &vault2[..8]
        ),
    );

    Some(PoolMintVaultInfo {
        mint1,
        mint2,
        vault1,
        vault2,
    })
}

/// Data reading utilities for consistent parsing across all decoders
/// These functions provide centralized, safe data extraction with proper bounds checking

/// Read a pubkey from data at given offset, advancing the offset
pub fn read_pubkey_at_offset(data: &[u8], offset: &mut usize) -> Result<String, String> {
    if *offset + 32 > data.len() {
        return Err(format!(
            "Offset {} + 32 exceeds data length {}",
            *offset,
            data.len()
        ));
    }

    let pubkey_bytes = &data[*offset..*offset + 32];
    *offset += 32;

    let pubkey = Pubkey::new_from_array(
        pubkey_bytes
            .try_into()
            .map_err(|_| "Invalid pubkey bytes".to_string())?,
    );

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
) -> Result<Pubkey, &'static str> {
    if data.len() < *offset + 32 {
        return Err("Insufficient data for pubkey");
    }
    let pubkey_bytes = &data[*offset..*offset + 32];
    *offset += 32;
    Pubkey::try_from(pubkey_bytes).map_err(|_| "Invalid pubkey")
}

/// Read a u8 value from data at given offset, advancing the offset
pub fn read_u8_at_offset(data: &[u8], offset: &mut usize) -> Result<u8, String> {
    if *offset >= data.len() {
        return Err("Insufficient data for u8".to_string());
    }

    let value = data[*offset];
    *offset += 1;
    Ok(value)
}

/// Read a u16 value from data at given offset, advancing the offset
pub fn read_u16_at_offset(data: &[u8], offset: &mut usize) -> Result<u16, String> {
    if *offset + 2 > data.len() {
        return Err("Insufficient data for u16".to_string());
    }

    let value_bytes = &data[*offset..*offset + 2];
    *offset += 2;
    let value = u16::from_le_bytes(
        value_bytes
            .try_into()
            .map_err(|_| "Failed to parse u16".to_string())?,
    );
    Ok(value)
}

/// Read a u32 value from data at given offset, advancing the offset
pub fn read_u32_at_offset(data: &[u8], offset: &mut usize) -> Result<u32, String> {
    if *offset + 4 > data.len() {
        return Err("Insufficient data for u32".to_string());
    }

    let value_bytes = &data[*offset..*offset + 4];
    *offset += 4;
    let value = u32::from_le_bytes(
        value_bytes
            .try_into()
            .map_err(|_| "Failed to parse u32".to_string())?,
    );
    Ok(value)
}

/// Read a u64 value from data at given offset, advancing the offset
pub fn read_u64_at_offset(data: &[u8], offset: &mut usize) -> Result<u64, String> {
    if *offset + 8 > data.len() {
        return Err("Insufficient data for u64".to_string());
    }

    let value_bytes = &data[*offset..*offset + 8];
    *offset += 8;
    let value = u64::from_le_bytes(
        value_bytes
            .try_into()
            .map_err(|_| "Failed to parse u64".to_string())?,
    );
    Ok(value)
}

/// Read a u128 value from data at given offset, advancing the offset
pub fn read_u128_at_offset(data: &[u8], offset: &mut usize) -> Result<u128, String> {
    if *offset + 16 > data.len() {
        return Err("Insufficient data for u128".to_string());
    }

    let value_bytes = &data[*offset..*offset + 16];
    *offset += 16;
    let value = u128::from_le_bytes(
        value_bytes
            .try_into()
            .map_err(|_| "Failed to parse u128".to_string())?,
    );
    Ok(value)
}

/// Read a bool value from data at given offset, advancing the offset
pub fn read_bool_at_offset(data: &[u8], offset: &mut usize) -> Result<bool, String> {
    if *offset >= data.len() {
        return Err("Insufficient data for bool".to_string());
    }

    let value = data[*offset] != 0;
    *offset += 1;
    Ok(value)
}

/// Read token account amount from token account data (at fixed offset 64)
pub fn read_token_account_amount(data: &[u8]) -> Option<u64> {
    if data.len() < 72 {
        return None;
    }
    // Token account amount is at offset 64
    Some(u64::from_le_bytes(data[64..72].try_into().ok()?))
}

/// Get the correct vault addresses for analyzer extraction
///
/// This function ensures the analyzer extracts vaults in the same order
/// that the decoder expects them to be in
pub fn get_analyzer_vault_order(pool_info: PoolMintVaultInfo) -> Vec<String> {
    let pair_info = analyze_token_pair(pool_info);

    if !pair_info.is_sol_pair {
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
pub fn validate_sol_pool(pool_info: PoolMintVaultInfo) -> Result<TokenPairInfo, String> {
    let pair_info = analyze_token_pair(pool_info);

    if !pair_info.is_sol_pair {
        Err("Pool does not contain SOL as base or quote".to_string())
    } else {
        Ok(pair_info)
    }
}
