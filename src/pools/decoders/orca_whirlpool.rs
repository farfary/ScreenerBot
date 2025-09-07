/// Orca Whirlpool decoder
///
/// This decoder handles Orca Whirlpool concentrated liquidity pools.
/// Based on the official Orca Whirlpool program structure from
/// https://github.com/orca-so/whirlpools/blob/main/programs/whirlpool/src/state/whirlpool.rs

use super::{ PoolDecoder, AccountData };
use crate::global::is_debug_pool_calculator_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::decimals::{ get_cached_decimals, SOL_DECIMALS };
use crate::pools::types::{ ProgramKind, PriceResult, SOL_MINT, ORCA_WHIRLPOOL_PROGRAM_ID };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::Instant;

pub struct OrcaWhirlpoolDecoder;

impl PoolDecoder for OrcaWhirlpoolDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::OrcaWhirlpool]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str
    ) -> Option<PriceResult> {
        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "START",
                &format!("Orca Whirlpool decoder: base={} quote={}", base_mint, quote_mint)
            );
        }

        // Find the pool account
        let pool_account = accounts
            .values()
            .find(|acc| { acc.owner.to_string() == ORCA_WHIRLPOOL_PROGRAM_ID })?;

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "INFO",
                &format!(
                    "Found Orca Whirlpool pool account {} with {} bytes",
                    pool_account.pubkey,
                    pool_account.data.len()
                )
            );
        }

        // Parse Orca Whirlpool structure
        let pool_info = Self::parse_whirlpool_data(&pool_account.data)?;

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "INFO",
                &format!(
                    "Parsed Orca Whirlpool:\n  token_mint_a: {}\n  token_mint_b: {}\n  token_vault_a: {}\n  token_vault_b: {}\n  sqrt_price: {}\n  liquidity: {}",
                    pool_info.token_mint_a,
                    pool_info.token_mint_b,
                    pool_info.token_vault_a,
                    pool_info.token_vault_b,
                    pool_info.sqrt_price,
                    pool_info.liquidity
                )
            );
        }

        // Determine which token is SOL and which is the target
        let (sol_vault, token_vault, sol_reserve, token_reserve, is_token_a_sol) = if
            pool_info.token_mint_a == SOL_MINT
        {
            // A is SOL, B is token
            let sol_vault_account = accounts.get(&pool_info.token_vault_a)?;
            let token_vault_account = accounts.get(&pool_info.token_vault_b)?;
            let sol_reserve = Self::extract_token_account_balance(&sol_vault_account.data)?;
            let token_reserve = Self::extract_token_account_balance(&token_vault_account.data)?;

            (pool_info.token_vault_a, pool_info.token_vault_b, sol_reserve, token_reserve, true)
        } else if pool_info.token_mint_b == SOL_MINT {
            // B is SOL, A is token
            let sol_vault_account = accounts.get(&pool_info.token_vault_b)?;
            let token_vault_account = accounts.get(&pool_info.token_vault_a)?;
            let sol_reserve = Self::extract_token_account_balance(&sol_vault_account.data)?;
            let token_reserve = Self::extract_token_account_balance(&token_vault_account.data)?;

            (pool_info.token_vault_b, pool_info.token_vault_a, sol_reserve, token_reserve, false)
        } else {
            if is_debug_pool_calculator_enabled() {
                log(
                    LogTag::PoolCalculator,
                    "WARN",
                    &format!(
                        "Orca Whirlpool pool does not contain SOL. Mints: {} and {}",
                        pool_info.token_mint_a,
                        pool_info.token_mint_b
                    )
                );
            }
            return None;
        };

        if sol_reserve == 0 || token_reserve == 0 {
            if is_debug_pool_calculator_enabled() {
                log(LogTag::PoolCalculator, "WARN", "Orca Whirlpool pool has zero reserves");
            }
            return None;
        }

        // Get token decimals from cache - CRITICAL: must be cached, no fallback
        let token_mint = if is_token_a_sol {
            &pool_info.token_mint_b
        } else {
            &pool_info.token_mint_a
        };
        let token_decimals = match get_cached_decimals(token_mint) {
            Some(decimals) => decimals,
            None => {
                if is_debug_pool_calculator_enabled() {
                    log(
                        LogTag::PoolCalculator,
                        "ERROR",
                        &format!("No cached decimals for Orca token: {}, skipping pool calculation", token_mint)
                    );
                }
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;

        // Calculate price using sqrt_price (more accurate for concentrated liquidity)
        let price_sol = if pool_info.sqrt_price > 0 {
            // Orca Whirlpool sqrt_price calculation
            // The sqrt_price is stored as a Q64.64 fixed point number
            // Price = (sqrt_price / 2^64)^2
            let sqrt_price_scaled = (pool_info.sqrt_price as f64) / (2_f64).powi(64);
            let raw_price = sqrt_price_scaled * sqrt_price_scaled;

            // The price represents token_a/token_b ratio
            let final_price = if is_token_a_sol {
                // SOL is token_a, so raw_price = SOL/TOKEN, we want TOKEN/SOL
                1.0 / raw_price
            } else {
                // TOKEN is token_a, so raw_price = TOKEN/SOL, which is what we want
                raw_price
            };

            // Adjust for decimal differences
            let decimal_adjustment =
                (10_f64).powi(sol_decimals as i32) / (10_f64).powi(token_decimals as i32);
            final_price * decimal_adjustment
        } else {
            // Fallback to reserve ratio calculation
            let sol_adjusted = (sol_reserve as f64) / (10_f64).powi(sol_decimals as i32);
            let token_adjusted = (token_reserve as f64) / (10_f64).powi(token_decimals as i32);
            sol_adjusted / token_adjusted
        };

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::PoolCalculator,
                "SUCCESS",
                &format!(
                    "Orca Whirlpool price calculated: {:.12} SOL\n  SOL reserves: {} ({})\n  Token reserves: {} ({})\n  sqrt_price: {}",
                    price_sol,
                    sol_reserve,
                    sol_vault,
                    token_reserve,
                    token_vault,
                    pool_info.sqrt_price
                )
            );
        }

        Some(
            PriceResult::new(
                token_mint.to_string(),
                0.0, // No USD calculation
                price_sol,
                sol_reserve as f64,
                token_reserve as f64,
                String::new() // Pool address will be set by calculator
            )
        )
    }
}

impl OrcaWhirlpoolDecoder {
    /// Parse Orca Whirlpool pool data according to the official structure
    fn parse_whirlpool_data(data: &[u8]) -> Option<WhirlpoolInfo> {
        if data.len() < 653 {
            return None;
        }

        let mut offset = 8; // Skip discriminator

        // Skip whirlpools_config (32 bytes)
        offset += 32;

        // Skip whirlpool_bump (1 byte)
        offset += 1;

        // Skip tick_spacing (2 bytes)
        offset += 2;

        // Skip fee_tier_index_seed (2 bytes)
        offset += 2;

        // Skip fee_rate (2 bytes)
        offset += 2;

        // Skip protocol_fee_rate (2 bytes)
        offset += 2;

        // Read liquidity (16 bytes)
        let liquidity = u128::from_le_bytes(data[offset..offset + 16].try_into().ok()?);
        offset += 16;

        // Read sqrt_price (16 bytes)
        let sqrt_price = u128::from_le_bytes(data[offset..offset + 16].try_into().ok()?);
        offset += 16;

        // Skip tick_current_index (4 bytes)
        offset += 4;

        // Skip protocol_fee_owed_a (8 bytes)
        offset += 8;

        // Skip protocol_fee_owed_b (8 bytes)
        offset += 8;

        // Read token_mint_a (32 bytes)
        let token_mint_a = Pubkey::try_from(&data[offset..offset + 32])
            .ok()?
            .to_string();
        offset += 32;

        // Read token_vault_a (32 bytes)
        let token_vault_a = Pubkey::try_from(&data[offset..offset + 32])
            .ok()?
            .to_string();
        offset += 32;

        // Skip fee_growth_global_a (16 bytes)
        offset += 16;

        // Read token_mint_b (32 bytes)
        let token_mint_b = Pubkey::try_from(&data[offset..offset + 32])
            .ok()?
            .to_string();
        offset += 32;

        // Read token_vault_b (32 bytes)
        let token_vault_b = Pubkey::try_from(&data[offset..offset + 32])
            .ok()?
            .to_string();

        Some(WhirlpoolInfo {
            token_mint_a,
            token_vault_a,
            token_mint_b,
            token_vault_b,
            liquidity,
            sqrt_price,
        })
    }

    /// Extract token account balance from token account data
    fn extract_token_account_balance(data: &[u8]) -> Option<u64> {
        if data.len() < 72 {
            return None;
        }

        // Token account balance is at offset 64 (8 bytes)
        Some(u64::from_le_bytes(data[64..72].try_into().ok()?))
    }
}

/// Orca Whirlpool pool information
#[derive(Debug, Clone)]
struct WhirlpoolInfo {
    pub token_mint_a: String,
    pub token_vault_a: String,
    pub token_mint_b: String,
    pub token_vault_b: String,
    pub liquidity: u128,
    pub sqrt_price: u128,
}
