/// Orca Whirlpool decoder
///
/// This decoder handles Orca Whirlpool concentrated liquidity pools.
/// Based on the official Orca Whirlpool program structure from
/// https://github.com/orca-so/whirlpools/blob/main/programs/whirlpool/src/state/whirlpool.rs
use super::{AccountData, PoolDecoder};
use crate::constants::{SOL_DECIMALS, SOL_MINT, ORCA_WHIRLPOOL_PROGRAM_ID};
use crate::logger::{self, LogTag};
use crate::pools::types::{PriceResult, ProgramKind};
use crate::tokens::get_cached_decimals;
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
        quote_mint: &str,
    ) -> Option<PriceResult> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Orca Whirlpool decoder: base={} quote={}",
                base_mint, quote_mint
            ),
        );

        // Find the pool account
        let pool_account = accounts
            .values()
            .find(|acc| acc.owner.to_string() == ORCA_WHIRLPOOL_PROGRAM_ID)?;

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Found Orca Whirlpool pool account {} with {} bytes",
                pool_account.pubkey,
                pool_account.data.len()
            ),
        );

        // Parse Orca Whirlpool structure
        let pool_info = Self::parse_whirlpool_data(&pool_account.data)?;

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Parsed Orca Whirlpool:\n  token_mint_a: {}\n  token_mint_b: {}\n  token_vault_a: {}\n  token_vault_b: {}\n  sqrt_price: {}\n  liquidity: {}",
                pool_info.token_mint_a,
                pool_info.token_mint_b,
                pool_info.token_vault_a,
                pool_info.token_vault_b,
                pool_info.sqrt_price,
                pool_info.liquidity
            ),
        );

        // Determine which token is SOL and which is the target
        let (sol_vault, token_vault, sol_reserve, token_reserve, is_token_a_sol) =
            if pool_info.token_mint_a == SOL_MINT {
                // A is SOL, B is token
                logger::debug(
                    LogTag::PoolDecoder,
                    &format!(
                        "Token A is SOL, Token B is target. Looking for vaults: A={}, B={}",
                        pool_info.token_vault_a, pool_info.token_vault_b
                    ),
                );
                logger::debug(
                    LogTag::PoolDecoder,
                    &format!(
                        "Available accounts: {:?}",
                        accounts.keys().collect::<Vec<_>>()
                    ),
                );

                let sol_vault_account = match accounts.get(&pool_info.token_vault_a) {
                    Some(account) => account,
                    None => {
                        logger::error(
                            LogTag::PoolDecoder,
                            &format!(
                                "SOL vault account {} not found in fetched accounts",
                                pool_info.token_vault_a
                            ),
                        );
                        return None;
                    }
                };

                let token_vault_account = match accounts.get(&pool_info.token_vault_b) {
                    Some(account) => account,
                    None => {
                        logger::error(
                            LogTag::PoolDecoder,
                            &format!(
                                "Token vault account {} not found in fetched accounts",
                                pool_info.token_vault_b
                            ),
                        );
                        return None;
                    }
                };

                let sol_reserve = Self::extract_token_account_balance(&sol_vault_account.data)?;
                let token_reserve = Self::extract_token_account_balance(&token_vault_account.data)?;

                (
                    pool_info.token_vault_a,
                    pool_info.token_vault_b,
                    sol_reserve,
                    token_reserve,
                    true,
                )
            } else if pool_info.token_mint_b == SOL_MINT {
                // B is SOL, A is token
                logger::debug(
                    LogTag::PoolDecoder,
                    &format!(
                        "Token B is SOL, Token A is target. Looking for vaults: A={}, B={}",
                        pool_info.token_vault_a, pool_info.token_vault_b
                    ),
                );
                logger::debug(
                    LogTag::PoolDecoder,
                    &format!(
                        "Available accounts: {:?}",
                        accounts.keys().collect::<Vec<_>>()
                    ),
                );

                let sol_vault_account = match accounts.get(&pool_info.token_vault_b) {
                    Some(account) => account,
                    None => {
                        logger::error(
                            LogTag::PoolDecoder,
                            &format!(
                                "SOL vault account {} not found in fetched accounts",
                                pool_info.token_vault_b
                            ),
                        );
                        return None;
                    }
                };

                let token_vault_account = match accounts.get(&pool_info.token_vault_a) {
                    Some(account) => account,
                    None => {
                        logger::error(
                            LogTag::PoolDecoder,
                            &format!(
                                "Token vault account {} not found in fetched accounts",
                                pool_info.token_vault_a
                            ),
                        );
                        return None;
                    }
                };

                let sol_reserve = Self::extract_token_account_balance(&sol_vault_account.data)?;
                let token_reserve = Self::extract_token_account_balance(&token_vault_account.data)?;

                (
                    pool_info.token_vault_b,
                    pool_info.token_vault_a,
                    sol_reserve,
                    token_reserve,
                    false,
                )
            } else {
                logger::warning(
                    LogTag::PoolDecoder,
                    &format!(
                        "Orca Whirlpool pool does not contain SOL. Mints: {} and {}",
                        pool_info.token_mint_a, pool_info.token_mint_b
                    ),
                );
                return None;
            };

        if sol_reserve == 0 || token_reserve == 0 {
            logger::warning(LogTag::PoolDecoder, "Orca Whirlpool pool has zero reserves");
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
                logger::error(
                    LogTag::PoolDecoder,
                    &format!(
                        "No decimals found for Orca token: {}, skipping pool calculation",
                        token_mint
                    ),
                );
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;

        // Calculate price using the CORRECT Orca sqrt_price formula from official repository
        // https://github.com/orca-so/whirlpools/blob/main/rust-sdk/core/src/math/price.rs#L27-L45
        // sqrt_price encodes sqrt(price), where price is token_b per token_a, adjusted by decimals.
        // We must return SOL per target token (price_sol).
        let price_sol = if is_token_a_sol {
            // Token A is SOL, Token B is target token.
            // First compute price_b_per_a (token B per token A):
            // price_b_per_a = (sqrt_price / Q64)^2 * 10^(decimals_a - decimals_b)
            let q64_resolution = 18446744073709551616.0; // 2^64
            let sqrt_price_normalized = (pool_info.sqrt_price as f64) / q64_resolution;
            let price_b_per_a = sqrt_price_normalized.powi(2)
                * (10_f64).powi((sol_decimals as i32) - (token_decimals as i32));
            // We want SOL per token (A per B), so invert:
            1.0 / price_b_per_a
        } else {
            // Token B is SOL, Token A is target token.
            // price_b_per_a = (sqrt_price / Q64)^2 * 10^(decimals_a - decimals_b)
            // Here, b is SOL, a is target token; price_b_per_a is SOL per token already.
            let q64_resolution = 18446744073709551616.0; // 2^64
            let sqrt_price_normalized = (pool_info.sqrt_price as f64) / q64_resolution;
            sqrt_price_normalized.powi(2)
                * (10_f64).powi((token_decimals as i32) - (sol_decimals as i32))
        };

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Orca Whirlpool price calculated: {:.12} SOL per token\n  SOL vault: {}\n  Token vault: {}\n  sqrt_price: {}\n  Token A is SOL: {}",
                price_sol,
                sol_vault,
                token_vault,
                pool_info.sqrt_price,
                is_token_a_sol
            ),
        );

        Some(PriceResult::new(
            token_mint.to_string(),
            0.0, // No USD calculation
            price_sol,
            sol_reserve as f64,
            token_reserve as f64,
            String::new(), // Pool address will be set by calculator
        ))
    }
}

impl OrcaWhirlpoolDecoder {
    /// Extract reserve account addresses from Whirlpool pool data for analyzer use
    /// Returns the account addresses that need to be fetched: [token_vault_a, token_vault_b]
    pub fn extract_reserve_accounts(pool_data: &[u8]) -> Option<Vec<String>> {
        if pool_data.len() < 653 {
            return None;
        }

        // Use the exact Orca Whirlpool structure offsets based on official source
        // Discriminator: 8 bytes (offset 0)
        // whirlpools_config: 32 bytes (offset 8)
        // whirlpool_bump: 1 byte (offset 40)
        // tick_spacing: 2 bytes (offset 41)
        // fee_tier_index_seed: 2 bytes (offset 43)
        // fee_rate: 2 bytes (offset 45)
        // protocol_fee_rate: 2 bytes (offset 47)
        // liquidity: 16 bytes (offset 49)
        // sqrt_price: 16 bytes (offset 65)
        // tick_current_index: 4 bytes (offset 81)
        // protocol_fee_owed_a: 8 bytes (offset 85)
        // protocol_fee_owed_b: 8 bytes (offset 93)
        // token_mint_a: 32 bytes (offset 101)
        // token_vault_a: 32 bytes (offset 133)
        // fee_growth_global_a: 16 bytes (offset 165)
        // token_mint_b: 32 bytes (offset 181)
        // token_vault_b: 32 bytes (offset 213)

        let token_vault_a = Self::extract_pubkey_at_offset(pool_data, 133)?;
        let token_vault_b = Self::extract_pubkey_at_offset(pool_data, 213)?;

        Some(vec![token_vault_a, token_vault_b])
    }

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

    /// Extract pubkey at fixed offset for analyzer use
    fn extract_pubkey_at_offset(data: &[u8], offset: usize) -> Option<String> {
        if offset + 32 > data.len() {
            return None;
        }

        let pubkey_bytes = &data[offset..offset + 32];
        let pubkey = Pubkey::new_from_array(pubkey_bytes.try_into().ok()?);
        Some(pubkey.to_string())
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
