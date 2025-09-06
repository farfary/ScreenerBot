/// Raydium CLMM (Concentrated Liquidity Market Maker) decoder
///
/// This decoder handles Raydium Concentrated Liquidity pools.
/// CLMM uses a sqrt_price_x64 format (Q64.64) and token vaults for pricing.
/// Based on Uniswap v3 math principles but with Raydium-specific implementation.

use super::{ PoolDecoder, AccountData };
use crate::global::is_debug_pool_calculator_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::decimals::get_cached_decimals;
use crate::pools::types::{ ProgramKind, PriceResult, SOL_MINT, RAYDIUM_CLMM_PROGRAM_ID };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::Instant;

pub struct RaydiumClmmDecoder;

impl PoolDecoder for RaydiumClmmDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::RaydiumClmm]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str
    ) -> Option<PriceResult> {
        if is_debug_pool_calculator_enabled() {
            log(LogTag::PoolCalculator, "INFO", "Starting Raydium CLMM pool decoding");
        }

        // Find the pool account
        let pool_account = accounts.values().find(|acc| {
            // Look for account with Raydium CLMM program as owner
            acc.owner.to_string() == RAYDIUM_CLMM_PROGRAM_ID
        })?;

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "INFO",
                &format!(
                    "Found CLMM pool account {} with {} bytes",
                    pool_account.pubkey,
                    pool_account.data.len()
                )
            );
        }

        // Parse CLMM pool structure
        let clmm_info = Self::parse_clmm_pool(&pool_account.data)?;

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "INFO",
                &format!(
                    "CLMM pool parsed: token_mint_0={}, token_mint_1={}, vault_0={}, vault_1={}, sqrt_price_x64={}",
                    clmm_info.token_mint_0,
                    clmm_info.token_mint_1,
                    clmm_info.token_vault_0,
                    clmm_info.token_vault_1,
                    clmm_info.sqrt_price_x64
                )
            );
        }

        // Determine which token is SOL and which is the base token
        let (token_mint, sol_vault, token_vault, is_token_0) = if
            clmm_info.token_mint_1 == SOL_MINT
        {
            // token_mint_0 is the custom token, token_mint_1 is SOL
            (
                clmm_info.token_mint_0.clone(),
                clmm_info.token_vault_1.clone(),
                clmm_info.token_vault_0.clone(),
                true,
            )
        } else if clmm_info.token_mint_0 == SOL_MINT {
            // token_mint_1 is the custom token, token_mint_0 is SOL
            (
                clmm_info.token_mint_1.clone(),
                clmm_info.token_vault_0.clone(),
                clmm_info.token_vault_1.clone(),
                false,
            )
        } else {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "ERROR",
                    &format!(
                        "CLMM pool has no SOL token: {} / {}",
                        clmm_info.token_mint_0,
                        clmm_info.token_mint_1
                    )
                );
            }
            return None;
        };

        // Verify this matches the requested base mint
        if token_mint != base_mint {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "ERROR",
                    &format!("CLMM pool token {} doesn't match requested {}", token_mint, base_mint)
                );
            }
            return None;
        }

        // Get vault balances
        let sol_account = accounts.get(&sol_vault)?;
        let token_account = accounts.get(&token_vault)?;

        let sol_balance = Self::decode_token_account_amount(&sol_account.data).ok()?;
        let token_balance = Self::decode_token_account_amount(&token_account.data).ok()?;

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "INFO",
                &format!(
                    "CLMM vault balances: SOL={}, token={}, is_token_0={}",
                    sol_balance,
                    token_balance,
                    is_token_0
                )
            );
        }

        if token_balance == 0 {
            if is_debug_pool_calculator_enabled() {
                log(LogTag::PoolCalculator, "ERROR", "CLMM pool has zero token balance");
            }
            return None;
        }

        // Get token decimals - CRITICAL: must be cached, no fallback to defaults
        let token_decimals = match get_cached_decimals(&token_mint) {
            Some(decimals) => decimals,
            None => {
                if is_debug_pool_calculator_enabled() {
                    log(
                        LogTag::PoolCalculator,
                        "ERROR",
                        &format!("CLMM: Token decimals not cached for {}, skipping price calculation", token_mint)
                    );
                }
                return None;
            }
        };
        let sol_decimals = 9u8; // SOL always has 9 decimals

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "INFO",
                &format!("CLMM decimals: token={}, sol={}", token_decimals, sol_decimals)
            );
        }

        // Calculate price using sqrt_price_x64
        // sqrt_price_x64 is in Q64.64 format, so we divide by 2^64 to get the actual sqrt_price
        // price = sqrt_price^2, and it represents token_1/token_0 price

        let sqrt_price = (clmm_info.sqrt_price_x64 as f64) / (2_f64).powi(64);
        let raw_price = sqrt_price * sqrt_price; // price = sqrt_price^2

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "INFO",
                &format!(
                    "CLMM sqrt_price_x64={}, sqrt_price={}, raw_price={}",
                    clmm_info.sqrt_price_x64,
                    sqrt_price,
                    raw_price
                )
            );
        }

        // Apply decimal adjustments and determine final price
        // raw_price represents token_1/token_0 ratio
        let price_sol = if is_token_0 {
            // Custom token is token_0, SOL is token_1
            // raw_price = SOL/token, so this is what we want
            raw_price * (10_f64).powi((token_decimals as i32) - (sol_decimals as i32))
        } else {
            // Custom token is token_1, SOL is token_0
            // raw_price = token/SOL, so we need to invert it
            (1.0 / raw_price) * (10_f64).powi((token_decimals as i32) - (sol_decimals as i32))
        };

        // Convert reserves to human-readable format for display
        let sol_reserves = (sol_balance as f64) / (10_f64).powi(sol_decimals as i32);
        let token_reserves = (token_balance as f64) / (10_f64).powi(token_decimals as i32);

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "INFO",
                &format!(
                    "CLMM price calculation: {:.12} SOL per token (sol_reserves={:.6}, token_reserves={:.6})",
                    price_sol,
                    sol_reserves,
                    token_reserves
                )
            );
        }

        Some(PriceResult {
            mint: token_mint,
            price_usd: 0.0, // We don't calculate USD prices, only SOL
            price_sol,
            sol_reserves,
            token_reserves,
            confidence: 0.9,
            source_pool: Some("RAYDIUM_CLMM".to_string()),
            pool_address: pool_account.pubkey.to_string(),
            slot: 0, // Will be updated by the system
            timestamp: Instant::now(),
        })
    }
}

impl RaydiumClmmDecoder {
    /// Parse CLMM pool account data to extract token mints, vault addresses, and sqrt_price
    /// Based on Raydium CLMM PoolState struct from official source code
    fn parse_clmm_pool(data: &[u8]) -> Option<ClmmPoolInfo> {
        // Minimum size check - CLMM pools are quite large (1000+ bytes)
        if data.len() < 800 {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "ERROR",
                    &format!("CLMM pool data too short: {} bytes (expected >= 800)", data.len())
                );
            }
            return None;
        }

        // Skip discriminator (8 bytes) and bump (1 byte)
        let mut offset = 8 + 1;

        // Skip amm_config (32 bytes) and owner (32 bytes)
        offset += 32 + 32;

        // Extract token mints at offsets (based on PoolState struct)
        let token_mint_0 = Self::extract_pubkey_at_offset(data, offset)?;
        offset += 32;
        let token_mint_1 = Self::extract_pubkey_at_offset(data, offset)?;
        offset += 32;

        // Extract token vaults
        let token_vault_0 = Self::extract_pubkey_at_offset(data, offset)?;
        offset += 32;
        let token_vault_1 = Self::extract_pubkey_at_offset(data, offset)?;
        offset += 32;

        // Skip observation_key (32 bytes)
        offset += 32;

        // Skip mint_decimals_0 (1 byte), mint_decimals_1 (1 byte), tick_spacing (2 bytes)
        offset += 1 + 1 + 2;

        // Skip liquidity (16 bytes)
        offset += 16;

        // Extract sqrt_price_x64 (16 bytes, u128)
        let sqrt_price_x64 = Self::extract_u128_at_offset(data, offset)?;

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "INFO",
                &format!(
                    "CLMM offsets: token_mint_0@{}={}, token_mint_1@{}={}, vault_0@{}={}, vault_1@{}={}",
                    73,
                    token_mint_0,
                    105,
                    token_mint_1,
                    137,
                    token_vault_0,
                    169,
                    token_vault_1
                )
            );
        }

        Some(ClmmPoolInfo {
            token_mint_0,
            token_mint_1,
            token_vault_0,
            token_vault_1,
            sqrt_price_x64,
        })
    }

    /// Extract a pubkey from raw data at a fixed offset
    fn extract_pubkey_at_offset(data: &[u8], offset: usize) -> Option<String> {
        if data.len() < offset + 32 {
            return None;
        }

        let pubkey_bytes: [u8; 32] = data[offset..offset + 32].try_into().ok()?;
        let pubkey = Pubkey::new_from_array(pubkey_bytes);
        Some(pubkey.to_string())
    }

    /// Extract a u128 value from raw data at a fixed offset
    fn extract_u128_at_offset(data: &[u8], offset: usize) -> Option<u128> {
        if data.len() < offset + 16 {
            return None;
        }

        let bytes: [u8; 16] = data[offset..offset + 16].try_into().ok()?;
        Some(u128::from_le_bytes(bytes))
    }

    /// Decode token account amount from token account data
    fn decode_token_account_amount(data: &[u8]) -> Result<u64, String> {
        if data.len() < 72 {
            return Err("Token account data too short".to_string());
        }

        // Token account amount is at offset 64 (8 bytes, little-endian)
        let amount_bytes: [u8; 8] = data[64..72]
            .try_into()
            .map_err(|_| "Failed to read amount bytes".to_string())?;

        Ok(u64::from_le_bytes(amount_bytes))
    }
}

/// Raydium CLMM pool information structure
#[derive(Debug, Clone)]
struct ClmmPoolInfo {
    pub token_mint_0: String,
    pub token_mint_1: String,
    pub token_vault_0: String,
    pub token_vault_1: String,
    pub sqrt_price_x64: u128,
}
