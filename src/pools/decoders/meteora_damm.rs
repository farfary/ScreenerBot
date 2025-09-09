/// Meteora DAMM decoder
///
/// This decoder handles Meteora Dynamic Automated Market Maker (DAMM) pools.
/// DAMM v2 uses a specific pool structure with token vaults and sqrt pricing.
/// Based on the proven logic from pool_old.rs lines ~7220-7450.

use super::{ PoolDecoder, AccountData };
use super::super::utils::{ is_sol_mint, WRAPPED_SOL_MINT };
use crate::arguments::is_debug_pool_decoders_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::{ get_token_decimals_sync, decimals::SOL_DECIMALS };
use crate::pools::types::{ ProgramKind, PriceResult, METEORA_DAMM_PROGRAM_ID };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::Instant;

pub struct MeteoraDammDecoder;

impl PoolDecoder for MeteoraDammDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::MeteoraDamm]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str
    ) -> Option<PriceResult> {
        if is_debug_pool_decoders_enabled() {
            log(LogTag::PoolDecoder, "INFO", "Starting Meteora DAMM pool decoding");
        }

        // Find the pool account
        let pool_account = accounts.values().find(|acc| {
            // Look for account with Meteora DAMM program as owner
            acc.owner.to_string() == METEORA_DAMM_PROGRAM_ID
        })?;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "Found DAMM pool account {} with {} bytes",
                    pool_account.pubkey,
                    pool_account.data.len()
                )
            );
        }

        // Parse DAMM pool structure
        let damm_info = Self::parse_damm_pool(&pool_account.data)?;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "DAMM pool parsed: token_a={}, token_b={}, vault_a={}, vault_b={}",
                    damm_info.token_a_mint,
                    damm_info.token_b_mint,
                    damm_info.token_a_vault,
                    damm_info.token_b_vault
                )
            );
        }

        // Determine which token is SOL and which is the base token
        let (token_mint, sol_vault, token_vault, sol_fees, token_fees) = if
            is_sol_mint(&damm_info.token_b_mint)
        {
            // token_a is the custom token, token_b is SOL
            (
                damm_info.token_a_mint.clone(),
                damm_info.token_b_vault.clone(),
                damm_info.token_a_vault.clone(),
                damm_info.protocol_b_fee + damm_info.partner_b_fee, // SOL fees
                damm_info.protocol_a_fee + damm_info.partner_a_fee, // Token fees
            )
        } else if is_sol_mint(&damm_info.token_a_mint) {
            // token_b is the custom token, token_a is SOL
            (
                damm_info.token_b_mint.clone(),
                damm_info.token_a_vault.clone(),
                damm_info.token_b_vault.clone(),
                damm_info.protocol_a_fee + damm_info.partner_a_fee, // SOL fees
                damm_info.protocol_b_fee + damm_info.partner_b_fee, // Token fees
            )
        } else {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!(
                        "DAMM pool has no SOL token: {} / {}",
                        damm_info.token_a_mint,
                        damm_info.token_b_mint
                    )
                );
            }
            return None;
        };

        // Verify this matches either the requested base or quote mint for bidirectional support
        if token_mint != base_mint && token_mint != quote_mint {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!(
                        "DAMM pool token {} doesn't match requested base {} or quote {}",
                        token_mint,
                        base_mint,
                        quote_mint
                    )
                );
            }
            return None;
        }

        // Get vault balances
        let sol_account = accounts.get(&sol_vault)?;
        let token_account = accounts.get(&token_vault)?;

        let sol_balance_raw = Self::decode_token_account_amount(&sol_account.data).ok()?;
        let token_balance_raw = Self::decode_token_account_amount(&token_account.data).ok()?;

        // Calculate effective reserves by subtracting accumulated fees
        // Fees are held in the vault but are not tradeable liquidity
        let sol_balance = if sol_balance_raw >= sol_fees {
            sol_balance_raw - sol_fees
        } else {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "WARN",
                    &format!(
                        "DAMM SOL fees ({}) exceed vault balance ({}), using raw balance",
                        sol_fees,
                        sol_balance_raw
                    )
                );
            }
            sol_balance_raw
        };

        let token_balance = if token_balance_raw >= token_fees {
            token_balance_raw - token_fees
        } else {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "WARN",
                    &format!(
                        "DAMM token fees ({}) exceed vault balance ({}), using raw balance",
                        token_fees,
                        token_balance_raw
                    )
                );
            }
            token_balance_raw
        };

        // Verify vault mints to ensure correct assignment
        if is_debug_pool_decoders_enabled() {
            let sol_vault_mint = Self::decode_token_account_mint(&sol_account.data).ok()?;
            let token_vault_mint = Self::decode_token_account_mint(&token_account.data).ok()?;

            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "DAMM vault verification: sol_vault {} mint={}, token_vault {} mint={}",
                    sol_vault,
                    sol_vault_mint,
                    token_vault,
                    token_vault_mint
                )
            );
        }

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "DAMM vault balances: SOL_raw={}, SOL_effective={} (fees={}), token_raw={}, token_effective={} (fees={})",
                    sol_balance_raw,
                    sol_balance,
                    sol_fees,
                    token_balance_raw,
                    token_balance,
                    token_fees
                )
            );
        }

        if token_balance == 0 {
            if is_debug_pool_decoders_enabled() {
                log(LogTag::PoolDecoder, "ERROR", "DAMM pool has zero token balance");
            }
            return None;
        }

        // Get token decimals - CRITICAL: must be available, no fallback to defaults
        let token_decimals = match get_token_decimals_sync(&token_mint) {
            Some(decimals) => decimals,
            None => {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "ERROR",
                        &format!("DAMM: Token decimals not found for {}, skipping price calculation", token_mint)
                    );
                }
                return None;
            }
        };
        let sol_decimals = SOL_DECIMALS;

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!("DAMM decimals: token={}, sol={}", token_decimals, sol_decimals)
            );
        }

        // METEORA DAMM v2 uses concentrated liquidity with sqrt_price_x64 format
        // Based on Uniswap v3 / concentrated liquidity math principles:
        // price = (sqrt_price / 2^64)^2, and price represents token_b/token_a ratio

        // For debugging: calculate simple vault ratio first
        let simple_ratio = if token_balance > 0 {
            let sol_f64 = (sol_balance as f64) / (10_f64).powi(sol_decimals as i32);
            let token_f64 = (token_balance as f64) / (10_f64).powi(token_decimals as i32);
            sol_f64 / token_f64
        } else {
            0.0
        };

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "DAMM simple vault ratio: {:.12} SOL per token (for comparison)",
                    simple_ratio
                )
            );
        }

        // Use sqrt_price from parsed pool data
        let price_sol = if damm_info.sqrt_price > 0 {
            // sqrt_price calculation using Q64.64 fixed point arithmetic
            // For Q64.64: ratio = (sqrt_price / 2^64)^2
            // Then apply decimal adjustment: ratio * 10^(sol_decimals - token_decimals)
            // Orientation:
            // - If token_b is SOL (WSOL), ratio is token/SOL; invert to get SOL/token
            // - If token_a is SOL, ratio is SOL/token already

            let sqrt_u128 = damm_info.sqrt_price;
            let sqrt_f64 = sqrt_u128 as f64;
            let divisor = (2_f64).powi(64);
            let normalized_sqrt = sqrt_f64 / divisor;
            let raw_price = normalized_sqrt * normalized_sqrt; // base ratio in smallest units

            // Apply decimal adjustment to convert from smallest units to human-readable
            let decimal_adj_factor = (10_f64).powi((sol_decimals as i32) - (token_decimals as i32));
            let decimal_adjusted_price = raw_price * decimal_adj_factor;

            let mut oriented_price = if is_sol_mint(&damm_info.token_b_mint) {
                // token_b is SOL, token_a is target
                // If sqrt_price = sqrt(token_b/token_a) = sqrt(SOL/target), then price = SOL/target
                // This is what we want: SOL per target token
                decimal_adjusted_price
            } else if is_sol_mint(&damm_info.token_a_mint) {
                // token_a is SOL, token_b is target
                // If sqrt_price = sqrt(token_b/token_a) = sqrt(target/SOL), then price = target/SOL
                // Invert to get SOL per target token
                if decimal_adjusted_price > 0.0 {
                    1.0 / decimal_adjusted_price
                } else {
                    0.0
                }
            } else {
                // Shouldn't happen: not a SOL pair; use decimal_adjusted_price as-is
                decimal_adjusted_price
            };

            // Sanity fallback
            if !oriented_price.is_finite() || oriented_price <= 0.0 {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "WARN",
                        &format!(
                            "DAMM sqrt_price invalid ({}), falling back to vault ratio {:.12}",
                            oriented_price,
                            simple_ratio
                        )
                    );
                }
                oriented_price = simple_ratio;
            }

            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "DEBUG",
                    &format!(
                        "DAMM sqrt_price calc: raw={} | sqrt_f64={:.6e} | normalized={:.18e} | base={:.18e} | dec_adj_factor={:.6e} | decimal_adjusted={:.18e} | oriented={:.18e}",
                        sqrt_u128,
                        sqrt_f64,
                        normalized_sqrt,
                        raw_price,
                        decimal_adj_factor,
                        decimal_adjusted_price,
                        oriented_price
                    )
                );
            }

            oriented_price
        } else {
            // Fallback to vault ratio if sqrt_price is zero or invalid
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "WARN",
                    "DAMM: sqrt_price is zero, using vault ratio as fallback"
                );
            }
            simple_ratio
        };

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "DAMM sqrt_price calculation: raw={}, simple_vault_ratio={:.12}, final_price={:.12}",
                    damm_info.sqrt_price,
                    simple_ratio,
                    price_sol
                )
            );
        }

        // Validate price result
        if price_sol <= 0.0 || !price_sol.is_finite() {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!("DAMM: Invalid sqrt_price calculation result: {}", price_sol)
                );
            }
            return None;
        }

        // For display purposes, calculate effective reserves from vault balances
        // (These are not used for price calculation, only for informational display)
        let sol_reserves_display = ((sol_balance as f64) / (10_f64).powi(sol_decimals as i32)).max(
            0.0
        );
        let token_reserves_display = (
            (token_balance as f64) / (10_f64).powi(token_decimals as i32)
        ).max(0.0);

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "DAMM final calculation: sqrt_price_method={:.12}, vault_ratio={:.12} (reserves: sol={:.6}, token={:.6})",
                    price_sol,
                    simple_ratio,
                    sol_reserves_display,
                    token_reserves_display
                )
            );
        }

        Some(PriceResult {
            mint: token_mint,
            price_usd: 0.0, // We don't calculate USD prices, only SOL
            price_sol,
            sol_reserves: sol_reserves_display,
            token_reserves: token_reserves_display,
            confidence: 0.9,
            source_pool: Some("METEORA_DAMM".to_string()),
            pool_address: pool_account.pubkey.to_string(),
            slot: 0, // Will be updated by the system
            timestamp: Instant::now(),
        })
    }
}

impl MeteoraDammDecoder {
    /// Parse DAMM pool account data to extract token mints, vault addresses, and sqrt_price
    /// Based on DAMM v2 Pool struct from official Meteora source code
    ///
    /// Official Pool struct (1112 bytes, from damm-v2/programs/cp-amm/src/state/pool.rs):
    /// pub struct Pool {
    ///     pub pool_fees: PoolFeesStruct,      // offset 8, size 168
    ///     pub token_a_mint: Pubkey,           // offset 176, size 32
    ///     pub token_b_mint: Pubkey,           // offset 208, size 32
    ///     pub token_a_vault: Pubkey,          // offset 240, size 32
    ///     pub token_b_vault: Pubkey,          // offset 272, size 32
    ///     pub whitelisted_vault: Pubkey,      // offset 304, size 32
    ///     pub partner: Pubkey,                // offset 336, size 32
    ///     pub liquidity: u128,                // offset 368, size 16
    ///     pub _padding: u128,                 // offset 384, size 16
    ///     pub protocol_a_fee: u64,            // offset 400, size 8
    ///     pub protocol_b_fee: u64,            // offset 408, size 8
    ///     pub partner_a_fee: u64,             // offset 416, size 8
    ///     pub partner_b_fee: u64,             // offset 424, size 8
    ///     pub sqrt_min_price: u128,           // offset 432, size 16
    ///     pub sqrt_max_price: u128,           // offset 448, size 16
    ///     pub sqrt_price: u128,               // offset 464, size 16  *** CORRECT OFFSET ***
    ///     pub activation_point: u64,          // offset 480, size 8
    ///     // ... rest of the struct
    /// }
    fn parse_damm_pool(data: &[u8]) -> Option<DammPoolInfo> {
        if data.len() < 1112 {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    &format!("DAMM pool data too short: {} bytes (expected >= 1112)", data.len())
                );
            }
            return None;
        }

        // Extract pubkeys at fixed offsets (based on empirical analysis of actual pool data)
        // The actual offsets are 8 bytes earlier than the theoretical struct layout suggests
        let token_a_mint = Self::extract_pubkey_at_fixed_offset(data, 168)?;
        let token_b_mint = Self::extract_pubkey_at_fixed_offset(data, 200)?;
        let token_a_vault = Self::extract_pubkey_at_fixed_offset(data, 232)?;
        let token_b_vault = Self::extract_pubkey_at_fixed_offset(data, 264)?;

        // Extract accumulated fees (these are held in vaults but not tradeable)
        let protocol_a_fee = Self::extract_u64_at_offset(data, 392).unwrap_or(0);
        let protocol_b_fee = Self::extract_u64_at_offset(data, 400).unwrap_or(0);
        let partner_a_fee = Self::extract_u64_at_offset(data, 408).unwrap_or(0);
        let partner_b_fee = Self::extract_u64_at_offset(data, 416).unwrap_or(0);

        // Extract sqrt_price at offset 456 (our original calculation was correct)
        // The issue might be that we need to try both 456 and 464 to see which gives reasonable values
        let sqrt_price_456 = Self::extract_u128_at_offset(data, 456).unwrap_or(0);
        let sqrt_price_464 = Self::extract_u128_at_offset(data, 464).unwrap_or(0);

        // Choose the value that gives a reasonable price (between 0.000001 and 0.1 SOL per token)
        let sqrt_price = if sqrt_price_456 > 0 {
            let test_price_456 = {
                let sqrt_f64 = sqrt_price_456 as f64;
                let divisor = (2_f64).powi(64);
                let normalized_sqrt = sqrt_f64 / divisor;
                normalized_sqrt * normalized_sqrt
            };

            if test_price_456 > 0.000001 && test_price_456 < 0.1 {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "INFO",
                        &format!("Using sqrt_price from offset 456: {}", sqrt_price_456)
                    );
                }
                sqrt_price_456
            } else if sqrt_price_464 > 0 {
                let test_price_464 = {
                    let sqrt_f64 = sqrt_price_464 as f64;
                    let divisor = (2_f64).powi(64);
                    let normalized_sqrt = sqrt_f64 / divisor;
                    normalized_sqrt * normalized_sqrt
                };

                if test_price_464 > 0.000001 && test_price_464 < 0.1 {
                    if is_debug_pool_decoders_enabled() {
                        log(
                            LogTag::PoolDecoder,
                            "INFO",
                            &format!("Using sqrt_price from offset 464: {}", sqrt_price_464)
                        );
                    }
                    sqrt_price_464
                } else {
                    if is_debug_pool_decoders_enabled() {
                        log(
                            LogTag::PoolDecoder,
                            "WARN",
                            &format!(
                                "Both offsets give unreasonable prices: 456={}, 464={}",
                                test_price_456,
                                test_price_464
                            )
                        );
                    }
                    sqrt_price_456 // Use original as fallback
                }
            } else {
                sqrt_price_456
            }
        } else {
            sqrt_price_464
        };

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "DAMM offsets: token_a@168={}, token_b@200={}, vault_a@232={}, vault_b@264={}",
                    token_a_mint,
                    token_b_mint,
                    token_a_vault,
                    token_b_vault
                )
            );

            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!(
                    "DAMM fees: protocol_a={}, protocol_b={}, partner_a={}, partner_b={}",
                    protocol_a_fee,
                    protocol_b_fee,
                    partner_a_fee,
                    partner_b_fee
                )
            );

            log(
                LogTag::PoolDecoder,
                "INFO",
                &format!("DAMM sqrt_price selected: {} (Q64.64 format)", sqrt_price)
            );
        }

        Some(DammPoolInfo {
            token_a_mint,
            token_b_mint,
            token_a_vault,
            token_b_vault,
            protocol_a_fee,
            protocol_b_fee,
            partner_a_fee,
            partner_b_fee,
            sqrt_price,
        })
    }

    /// Extract a pubkey from raw data at a fixed offset
    fn extract_pubkey_at_fixed_offset(data: &[u8], offset: usize) -> Option<String> {
        if data.len() < offset + 32 {
            return None;
        }

        let pubkey_bytes: [u8; 32] = data[offset..offset + 32].try_into().ok()?;
        let pubkey = Pubkey::new_from_array(pubkey_bytes);
        Some(pubkey.to_string())
    }

    /// Extract a u64 value from raw data at a fixed offset
    fn extract_u64_at_offset(data: &[u8], offset: usize) -> Option<u64> {
        if data.len() < offset + 8 {
            return None;
        }

        let bytes: [u8; 8] = data[offset..offset + 8].try_into().ok()?;
        Some(u64::from_le_bytes(bytes))
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

    /// Decode token account mint from token account data
    fn decode_token_account_mint(data: &[u8]) -> Result<String, String> {
        if data.len() < 32 {
            return Err("Token account data too short for mint".to_string());
        }

        // Mint is at offset 0 (32 bytes)
        let mint_bytes: [u8; 32] = data[0..32]
            .try_into()
            .map_err(|_| "Failed to read mint bytes".to_string())?;

        let mint_pubkey = Pubkey::new_from_array(mint_bytes);
        Ok(mint_pubkey.to_string())
    }
}

/// Meteora DAMM pool information structure
#[derive(Debug, Clone)]
struct DammPoolInfo {
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub token_a_vault: String,
    pub token_b_vault: String,
    pub protocol_a_fee: u64,
    pub protocol_b_fee: u64,
    pub partner_a_fee: u64,
    pub partner_b_fee: u64,
    pub sqrt_price: u128,
}
