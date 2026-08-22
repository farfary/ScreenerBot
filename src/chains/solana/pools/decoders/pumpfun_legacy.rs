//! PumpFun Legacy decoder for PumpFun bonding curves
//!
//! Handles bonding curve accounts (NOT AMM pools). Bonding curves:
//! - Program: 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P
//! - Account sizes: 256 bytes (original) or 150 bytes (migrated/partial)
//! - Discriminator: 17 b7 f8 37 60 d8 ac 60
//! - Layout: reserves stored directly (no vault accounts)
//! - Token mint: provided by caller (from discovery phase)
//!
//! Note: Migrated bonding curves (150 bytes) still contain all reserve data
//! at the same offsets, just with truncated trailing data.

use super::{AccountData, PoolDecoder};
use crate::chains::solana::constants::PUMP_FUN_LEGACY_PROGRAM_ID;
use crate::chains::solana::constants::{SOL_DECIMALS, SOL_MINT};
use crate::chains::solana::pools::types::ProgramKind;
use crate::logger::{self, LogTag};
use crate::pools::types::PriceResult;
use crate::tokens::get_cached_decimals;
use std::collections::HashMap;
use std::time::Instant;

/// PumpFun bonding curve discriminator (first 8 bytes)
const BONDING_CURVE_DISCRIMINATOR: [u8; 8] = [0x17, 0xb7, 0xf8, 0x37, 0x60, 0xd8, 0xac, 0x60];

/// Expected account sizes for bonding curves
/// Full size: 256 bytes (original)
/// Migrated/partial: 150 bytes (after migration)
const BONDING_CURVE_SIZE_FULL: usize = 256;
const BONDING_CURVE_SIZE_MIGRATED: usize = 150;
const BONDING_CURVE_MIN_SIZE: usize = 48; // Minimum to read reserves

/// Offset of the curve's `complete` flag, immediately after the five u64
/// reserve/supply fields. Set by the program when the curve graduates and its
/// liquidity migrates to the PumpFun AMM (see `pumpfun_amm`), at which point
/// the curve is drained and every reserve reads zero.
const BONDING_CURVE_COMPLETE_OFFSET: usize = 48;

/// PumpFun Legacy pool decoder and calculator
pub struct PumpFunLegacyDecoder;

impl PoolDecoder for PumpFunLegacyDecoder {
    /// Get the program kinds this decoder supports
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::PumpFunLegacy]
    }

    /// Decode pool data and calculate price for PumpFun bonding curves
    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "PumpFun Legacy (Bonding Curve): Processing for base={} quote={}",
                base_mint, quote_mint
            ),
        );

        // Validate this is a SOL pair
        if base_mint != SOL_MINT && quote_mint != SOL_MINT {
            logger::debug(
                LogTag::PoolDecoder,
                &format!(
                    "PumpFun Legacy: Not a SOL pair (base={}, quote={}), skipping",
                    base_mint, quote_mint
                ),
            );
            return None;
        }

        // Determine which is the token mint (the non-SOL one)
        let token_mint = if base_mint == SOL_MINT {
            quote_mint
        } else {
            base_mint
        };

        // Find the bonding curve account
        for (pool_account, pool_data) in accounts.iter() {
            // Check owner
            if pool_data.owner.to_string() != PUMP_FUN_LEGACY_PROGRAM_ID {
                continue;
            }

            // Check size (accept both full 256-byte and migrated 150-byte bonding curves)
            let is_valid_size = pool_data.data.len() == BONDING_CURVE_SIZE_FULL
                || pool_data.data.len() == BONDING_CURVE_SIZE_MIGRATED
                || pool_data.data.len() >= BONDING_CURVE_MIN_SIZE;

            if !is_valid_size {
                logger::debug(
                    LogTag::PoolDecoder,
                    &format!(
                        "PumpFun Legacy: Account {} has size {} (expected {} or {}), skipping",
                        pool_account,
                        pool_data.data.len(),
                        BONDING_CURVE_SIZE_FULL,
                        BONDING_CURVE_SIZE_MIGRATED
                    ),
                );
                continue;
            }

            // Check discriminator
            if pool_data.data.len() < 8 || &pool_data.data[0..8] != BONDING_CURVE_DISCRIMINATOR {
                logger::debug(
                    LogTag::PoolDecoder,
                    &format!(
                        "PumpFun Legacy: Account {} has wrong discriminator, skipping",
                        pool_account
                    ),
                );
                continue;
            }

            logger::debug(
                LogTag::PoolDecoder,
                &format!(
                    "Found PumpFun bonding curve: pool={}, token={}",
                    pool_account, token_mint
                ),
            );

            // Read reserves from bonding curve account
            if let Some(price_result) =
                Self::calculate_bonding_curve_price(&pool_data.data, token_mint, pool_account)
            {
                return Some(price_result);
            }
        }

        None
    }
}

impl PumpFunLegacyDecoder {
    /// Calculate price from bonding curve reserves
    fn calculate_bonding_curve_price(
        data: &[u8],
        token_mint: &str,
        pool_account: &str,
    ) -> Option<PriceResult> {
        // Bonding curve layout (discovered offsets):
        // 0..8:   discriminator
        // 8..16:  virtual_token_reserves (u64)
        // 16..24: virtual_sol_reserves (u64)
        // 24..32: real_token_reserves (u64)
        // 32..40: real_sol_reserves (u64)
        // 40..48: token_total_supply (u64)
        // ...rest of fields

        if data.len() < 48 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "PumpFun bonding curve {} data too short: {} bytes",
                    pool_account,
                    data.len()
                ),
            );
            return None;
        }

        // Read reserve fields (all little-endian u64)
        let virtual_token_reserves = u64::from_le_bytes(data[8..16].try_into().ok()?);
        let virtual_sol_reserves = u64::from_le_bytes(data[16..24].try_into().ok()?);
        let real_token_reserves = u64::from_le_bytes(data[24..32].try_into().ok()?);
        let real_sol_reserves = u64::from_le_bytes(data[32..40].try_into().ok()?);
        let token_total_supply = u64::from_le_bytes(data[40..48].try_into().ok()?);

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Bonding curve {} reserves: virt_token={}, virt_sol={}, real_token={}, real_sol={}, supply={}",
                pool_account,
                virtual_token_reserves,
                virtual_sol_reserves,
                real_token_reserves,
                real_sol_reserves,
                token_total_supply
            ),
        );

        // A graduated curve is drained by design: the program moves the
        // liquidity to the PumpFun AMM and every reserve reads zero. That is a
        // routine lifecycle transition, not corrupt account data, so it must
        // not be reported as a decoder error -- doing so filed two ERROR lines
        // per graduating token into the same stream real decode failures use.
        // The curve simply no longer prices this token; the AMM pool does.
        let graduated = data
            .get(BONDING_CURVE_COMPLETE_OFFSET)
            .is_some_and(|flag| *flag != 0);

        // Validate reserves
        if virtual_token_reserves == 0 || virtual_sol_reserves == 0 {
            if graduated {
                logger::debug(
                    LogTag::PoolDecoder,
                    &format!(
                        "PumpFun bonding curve {pool_account} has graduated to the AMM; \
                         it no longer prices {token_mint}"
                    ),
                );
            } else {
                // Zero reserves on a curve that has NOT graduated is genuinely
                // anomalous and stays an error.
                logger::error(
                    LogTag::PoolDecoder,
                    &format!(
                        "PumpFun bonding curve {} has zero virtual reserves but is not complete: token={}, sol={}",
                        pool_account, virtual_token_reserves, virtual_sol_reserves
                    ),
                );
            }
            return None;
        }

        // Get token decimals
        let token_decimals = get_cached_decimals(crate::chains::ChainId::Solana, token_mint)?;

        logger::debug(
            LogTag::PoolDecoder,
            &format!("Token {token_mint} decimals: {token_decimals}"),
        );

        if token_decimals > 18 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "PumpFun bonding curve {} token decimals too large: {}",
                    pool_account, token_decimals
                ),
            );
            return None;
        }

        // Calculate price: SOL per token
        // Formula: price = virtual_sol_reserves / virtual_token_reserves (in human-readable units)
        let token_amount = virtual_token_reserves as f64 / 10_f64.powi(token_decimals as i32);
        let sol_amount = virtual_sol_reserves as f64 / 10_f64.powi(SOL_DECIMALS as i32);

        if token_amount <= 0.0 || !token_amount.is_finite() {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "PumpFun bonding curve {} token amount invalid: {}",
                    pool_account, token_amount
                ),
            );
            return None;
        }

        if token_amount == 0.0 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "PumpFun bonding curve {} token amount is zero after decimal adjustment",
                    pool_account
                ),
            );
            return None;
        }

        let price_sol = sol_amount / token_amount;

        if !price_sol.is_finite() || price_sol <= 0.0 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "PumpFun bonding curve {} price invalid: {}",
                    pool_account, price_sol
                ),
            );
            return None;
        }

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "PumpFun bonding curve price calculated: pool={}, token={}, price={:.15} SOL/token (virt_sol={} virt_token={})",
                pool_account,
                token_mint,
                price_sol,
                virtual_sol_reserves,
                virtual_token_reserves
            ),
        );

        Some(PriceResult {
            mint: token_mint.to_string(),
            price_usd: 0.0, // USD price calculated later
            price_sol,
            confidence: 1.0,
            source_pool: Some("PumpFun Bonding Curve".to_owned()),
            pool_address: pool_account.to_string(),
            slot: 0,
            timestamp: Instant::now(),
            sol_reserves: sol_amount,
            token_reserves: token_amount,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_curve_bytes(virtual_token_reserves: u64, virtual_sol_reserves: u64) -> Vec<u8> {
        let mut data = vec![0u8; 48];
        data[0..8].copy_from_slice(&BONDING_CURVE_DISCRIMINATOR);
        data[8..16].copy_from_slice(&virtual_token_reserves.to_le_bytes());
        data[16..24].copy_from_slice(&virtual_sol_reserves.to_le_bytes());
        // real_token_reserves, real_sol_reserves, token_total_supply left at 0.
        data
    }

    #[test]
    fn short_data_is_rejected_without_panicking() {
        assert!(PumpFunLegacyDecoder::calculate_bonding_curve_price(&[], "mint", "pool").is_none());
        assert!(PumpFunLegacyDecoder::calculate_bonding_curve_price(
            &vec![0u8; 47],
            "mint",
            "pool"
        )
        .is_none());
    }

    #[test]
    fn zero_virtual_reserves_are_rejected() {
        let data = valid_curve_bytes(0, 5_000_000_000);
        assert!(
            PumpFunLegacyDecoder::calculate_bonding_curve_price(&data, "mint", "pool").is_none()
        );
    }

    #[test]
    fn valid_curve_computes_sol_per_token_price() {
        let mint = crate::chains::solana::solana_sdk::pubkey::Pubkey::new_unique().to_string();
        crate::tokens::cache_decimals(crate::chains::ChainId::Solana, &mint, 6);

        // 1_000_000 tokens (6 decimals) virtual reserve against 1 SOL (9 decimals).
        let data = valid_curve_bytes(1_000_000_000_000, 1_000_000_000);
        let result = PumpFunLegacyDecoder::calculate_bonding_curve_price(&data, &mint, "pool")
            .expect("well-formed bonding curve data must decode");

        assert_eq!(result.mint, mint);
        assert_eq!(result.pool_address, "pool");
        assert!(
            (result.price_sol - 0.000_001).abs() < 1e-12,
            "price_sol = {}",
            result.price_sol
        );
    }
}
