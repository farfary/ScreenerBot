/// Pool utilities for consistent SOL detection and vault pairing across analyzer and decoders
///
/// This module provides centralized logic for:
/// - Detecting SOL mints (wrapped and native forms)
/// - Determining token pair orientation (TOKEN/SOL vs SOL/TOKEN)
/// - Pairing vaults correctly based on mint types
/// - Handling all possible base/quote token combinations

use crate::global::is_debug_pool_service_enabled;
use crate::logger::{ log, LogTag };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// SOL mint constants - all possible representations of SOL
pub const WRAPPED_SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const NATIVE_SOL_MINT: &str = "11111111111111111111111111111111"; // System Program ID
pub const SOL_DECIMALS: u8 = 9;

/// Common stablecoin mints that we should skip (not SOL-based pricing)
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

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
        if is_debug_pool_service_enabled() {
            log(LogTag::PoolService, "DEBUG", &format!("Invalid token pair: {}", reason));
        }

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
    if is_sol_mint(mint) { WRAPPED_SOL_MINT.to_string() } else { mint.to_string() }
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

    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "DEBUG",
            &format!(
                "Analyzing token pair: mint1={}, mint2={}, vault1={}, vault2={}",
                &mint1[..8],
                &mint2[..8],
                &vault1[..8],
                &vault2[..8]
            )
        );
    }

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
        return TokenPairInfo::invalid(
            format!("No SOL mint found: mint1={}, mint2={}", &mint1[..8], &mint2[..8])
        );
    };

    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "SUCCESS",
            &format!(
                "Valid SOL pair: token={}, sol_is_first={}, token_vault={}, sol_vault={}",
                &token_mint[..8],
                sol_is_first,
                &token_vault[..8],
                &sol_vault[..8]
            )
        );
    }

    TokenPairInfo {
        token_mint,
        sol_mint,
        token_vault,
        sol_vault,
        sol_is_first,
        is_sol_pair: true,
    }
}

/// Extract mints and vaults from PumpFun pool account data
///
/// PumpFun pool structure:
/// - discriminator (8 bytes)
/// - pool_bump (u8)
/// - index (u16)
/// - creator (32 bytes)
/// - creator (32 bytes) - duplicate in some pool versions
/// - base_mint (32 bytes)
/// - quote_mint (32 bytes)
/// - lp_mint (32 bytes)
/// - pool_base_token_account (32 bytes)
/// - pool_quote_token_account (32 bytes)
/// - ... additional fields
pub fn extract_pumpfun_mints_and_vaults(data: &[u8]) -> Option<PoolMintVaultInfo> {
    if data.len() < 200 {
        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "ERROR",
                &format!("PumpFun pool data too short: {} bytes", data.len())
            );
        }
        return None;
    }

    let mut offset = 8; // Skip discriminator

    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "DEBUG",
            &format!("Extracting PumpFun pool data ({} bytes)", data.len())
        );
    }

    // Skip pool_bump (u8) and index (u16)
    offset += 1 + 2;

    // Skip first creator pubkey
    offset += 32;

    // Handle potential duplicate creator field (exists in some pool versions)
    // We'll read the next 32 bytes and check if it looks like a creator or a mint
    let potential_creator_or_mint = read_pubkey_at_offset(data, &mut offset).ok()?;

    // Check if this looks like a mint by trying to parse as pubkey and checking length
    // If it's a valid mint, we'll treat it as mint1, otherwise skip as duplicate creator
    let (mint1, mint2) = if is_likely_mint(&potential_creator_or_mint) {
        // This is mint1, read mint2 next
        let mint2 = read_pubkey_at_offset(data, &mut offset).ok()?;
        (potential_creator_or_mint, mint2)
    } else {
        // This was duplicate creator, read both mints
        let mint1 = read_pubkey_at_offset(data, &mut offset).ok()?;
        let mint2 = read_pubkey_at_offset(data, &mut offset).ok()?;
        (mint1, mint2)
    };

    // Skip lp_mint
    offset += 32;

    // Read vault addresses
    let vault1 = read_pubkey_at_offset(data, &mut offset).ok()?;
    let vault2 = read_pubkey_at_offset(data, &mut offset).ok()?;

    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "DEBUG",
            &format!(
                "Extracted PumpFun: mint1={}, mint2={}, vault1={}, vault2={}",
                &mint1[..8],
                &mint2[..8],
                &vault1[..8],
                &vault2[..8]
            )
        );
    }

    Some(PoolMintVaultInfo {
        mint1,
        mint2,
        vault1,
        vault2,
    })
}

/// Check if a pubkey string is likely a mint address (heuristic)
fn is_likely_mint(pubkey_str: &str) -> bool {
    // Check if it matches known mint patterns or is a valid pubkey format
    // SOL mints, USDC, USDT, or other token mints typically have specific characteristics
    is_sol_mint(pubkey_str) ||
        is_stablecoin_mint(pubkey_str) ||
        (pubkey_str.len() == 44 && pubkey_str.chars().all(|c| c.is_alphanumeric()))
}

/// Helper function to read pubkey at offset
fn read_pubkey_at_offset(data: &[u8], offset: &mut usize) -> Result<String, String> {
    if *offset + 32 > data.len() {
        return Err(format!("Offset {} + 32 exceeds data length {}", *offset, data.len()));
    }

    let pubkey_bytes = &data[*offset..*offset + 32];
    *offset += 32;

    let pubkey = Pubkey::new_from_array(
        pubkey_bytes.try_into().map_err(|_| "Invalid pubkey bytes".to_string())?
    );

    Ok(pubkey.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sol_detection() {
        assert!(is_sol_mint(WRAPPED_SOL_MINT));
        assert!(is_sol_mint(NATIVE_SOL_MINT));
        assert!(!is_sol_mint("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")); // USDC
    }

    #[test]
    fn test_stablecoin_detection() {
        assert!(is_stablecoin_mint(USDC_MINT));
        assert!(is_stablecoin_mint(USDT_MINT));
        assert!(!is_stablecoin_mint(WRAPPED_SOL_MINT));
    }

    #[test]
    fn test_token_pair_analysis() {
        // Test TOKEN/SOL pair
        let pool_info = PoolMintVaultInfo {
            mint1: "SomeTokenMint12345678901234567890123456".to_string(),
            mint2: WRAPPED_SOL_MINT.to_string(),
            vault1: "TokenVault12345678901234567890123456789".to_string(),
            vault2: "SolVault123456789012345678901234567890".to_string(),
        };

        let result = analyze_token_pair(pool_info);
        assert!(result.is_sol_pair);
        assert!(!result.sol_is_first);
        assert_eq!(result.token_mint, "SomeTokenMint12345678901234567890123456");
        assert_eq!(result.sol_mint, WRAPPED_SOL_MINT);

        // Test SOL/TOKEN pair
        let pool_info = PoolMintVaultInfo {
            mint1: WRAPPED_SOL_MINT.to_string(),
            mint2: "SomeTokenMint12345678901234567890123456".to_string(),
            vault1: "SolVault123456789012345678901234567890".to_string(),
            vault2: "TokenVault12345678901234567890123456789".to_string(),
        };

        let result = analyze_token_pair(pool_info);
        assert!(result.is_sol_pair);
        assert!(result.sol_is_first);
    }

    #[test]
    fn test_stablecoin_rejection() {
        // Test TOKEN/USDC pair (should be rejected)
        let pool_info = PoolMintVaultInfo {
            mint1: "SomeTokenMint12345678901234567890123456".to_string(),
            mint2: USDC_MINT.to_string(),
            vault1: "TokenVault12345678901234567890123456789".to_string(),
            vault2: "UsdcVault12345678901234567890123456789".to_string(),
        };

        let result = analyze_token_pair(pool_info);
        assert!(!result.is_sol_pair);
    }
}
