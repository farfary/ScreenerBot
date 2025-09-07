/// Meteora DAMM decoder
///
/// This decoder handles Meteora Dynamic Automated Market Maker (DAMM) pools.
/// DAMM v2 uses a specific pool structure with token vaults and sqrt pricing.
/// Based on the proven logic from pool_old.rs lines ~7220-7450.

use super::{ PoolDecoder, AccountData };
use super::super::utils::{ is_sol_mint, WRAPPED_SOL_MINT };
use crate::arguments::is_debug_pool_decoders_enabled;use crate::logger::{ log, LogTag };
use crate::tokens::decimals::{ get_cached_decimals, SOL_DECIMALS };
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

        // Get token decimals - CRITICAL: must be cached, no fallback to defaults
        let token_decimals = match get_cached_decimals(&token_mint) {
            Some(decimals) => decimals,
            None => {
                if is_debug_pool_decoders_enabled() {
                    log(
                        LogTag::PoolDecoder,
                        "ERROR",
                        &format!("DAMM: Token decimals not cached for {}, skipping price calculation", token_mint)
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

        // DAMM v2 uses concentrated liquidity with sqrt_price, NOT vault balance ratios!
        // Extract sqrt_price from the account data
        // Based on real pool account analysis - offset 456 confirmed with functioning XVM/SOL pool
        let sqrt_price_offset = 456; // Verified offset for sqrt_price field
        if pool_account.data.len() < sqrt_price_offset + 16 {
            if is_debug_pool_decoders_enabled() {
                log(
                    LogTag::PoolDecoder,
                    "ERROR",
                    "Account data too short for sqrt_price extraction"
                );
            }
            return None;
        }

        // Read sqrt_price as u128 little endian
        let sqrt_price_bytes = &pool_account.data[sqrt_price_offset..sqrt_price_offset + 16];
        let sqrt_price = match sqrt_price_bytes.try_into() {
            Ok(bytes) => u128::from_le_bytes(bytes),
            Err(_) => {
                if is_debug_pool_decoders_enabled() {
                    log(LogTag::PoolDecoder, "ERROR", "Failed to extract sqrt_price bytes");
                }
                return None;
            }
        };

        if is_debug_pool_decoders_enabled() {
            log(
                LogTag::PoolDecoder,
                "DEBUG",
                &format!("Raw sqrt_price at offset {}: {}", sqrt_price_offset, sqrt_price)
            );
        }
        if sqrt_price == 0 {
            if is_debug_pool_decoders_enabled() {
                log(LogTag::PoolDecoder, "ERROR", "DAMM pool has zero sqrt_price");
            }
            return None;
        }

        // Calculate price using official Meteora formula:
        // price = (sqrt_price^2 / 2^128) * 10^(tokenA_decimals - tokenB_decimals)
        //
        // First convert sqrt_price from Q64.64 to floating point
        let sqrt_price_f64 = (sqrt_price as f64) / (2_f64).powi(64);

        // Square it to get the actual price ratio
        let price_ratio = sqrt_price_f64 * sqrt_price_f64;

        // Apply decimal adjustment based on token order
        // DAMM v2 stores sqrt_price as sqrt(token_b/token_a)
        let price_sol = if is_sol_mint(&damm_info.token_b_mint) {
            // token_a is custom token, token_b is SOL
            // sqrt_price = sqrt(SOL/token) -> price = SOL per token (what we want)
            price_ratio * (10_f64).powi((sol_decimals as i32) - (token_decimals as i32))
        } else {
            // token_a is SOL, token_b is custom token
            // sqrt_price = sqrt(token/SOL) -> price = token per SOL
            // Invert to get SOL per token
            let token_per_sol =
                price_ratio * (10_f64).powi((token_decimals as i32) - (sol_decimals as i32));
            if token_per_sol > 0.0 {
                1.0 / token_per_sol
            } else {
                if is_debug_pool_decoders_enabled() {
                    log(LogTag::PoolDecoder, "ERROR", "DAMM: Invalid token_per_sol ratio");
                }
                return None;
            }
        };

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
                    "DAMM sqrt_price calculation: sqrt_price_raw={}, sqrt_price_f64={:.12}, price_ratio={:.12}, price_sol={:.12} (reserves: sol={:.6}, token={:.6})",
                    sqrt_price,
                    sqrt_price_f64,
                    price_ratio,
                    price_sol,
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
    /// Key offsets in 1112-byte pool account:
    /// - token_a_mint: 168 (32 bytes)
    /// - token_b_mint: 200 (32 bytes)
    /// - token_a_vault: 232 (32 bytes)
    /// - token_b_vault: 264 (32 bytes)
    /// - protocol_a_fee: 392 (8 bytes)
    /// - protocol_b_fee: 400 (8 bytes)
    /// - partner_a_fee: 408 (8 bytes)
    /// - partner_b_fee: 416 (8 bytes)
    /// - sqrt_price: ~712 (16 bytes) [CRITICAL: concentrated liquidity pricing]
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

        // Extract pubkeys at fixed offsets (based on official DAMM v2 Pool struct)
        let token_a_mint = Self::extract_pubkey_at_fixed_offset(data, 168)?;
        let token_b_mint = Self::extract_pubkey_at_fixed_offset(data, 200)?;
        let token_a_vault = Self::extract_pubkey_at_fixed_offset(data, 232)?;
        let token_b_vault = Self::extract_pubkey_at_fixed_offset(data, 264)?;

        // Extract accumulated fees (these are held in vaults but not tradeable)
        // Based on official DAMM v2 Pool struct layout from IDL:
        // offset 392: protocol_a_fee: u64, offset 400: protocol_b_fee: u64
        // offset 408: partner_a_fee: u64, offset 416: partner_b_fee: u64
        let protocol_a_fee = Self::extract_u64_at_offset(data, 392).unwrap_or(0);
        let protocol_b_fee = Self::extract_u64_at_offset(data, 400).unwrap_or(0);
        let partner_a_fee = Self::extract_u64_at_offset(data, 408).unwrap_or(0);
        let partner_b_fee = Self::extract_u64_at_offset(data, 416).unwrap_or(0);

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

            // Extract and log sqrt_price for debugging
            if let Some(sqrt_price_raw) = Self::extract_u128_at_offset(data, 448) {
                log(
                    LogTag::PoolDecoder,
                    "INFO",
                    &format!("DAMM sqrt_price@448: {}", sqrt_price_raw)
                );
            } else {
                log(
                    LogTag::PoolDecoder,
                    "WARN",
                    "DAMM: Could not extract sqrt_price at offset 448"
                );
            }
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
}
