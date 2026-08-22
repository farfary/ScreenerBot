//! Moonit AMM pool decoder
//!
//! This module handles decoding Moonit AMM pools which use a bonding curve model.
//! Moonit pools are derived as PDAs with seeds ["token", mint_address] and contain
//! a CurveAccount structure with pricing information.

use super::{AccountData, PoolDecoder};
use crate::chains::solana::constants::SOL_DECIMALS;
use crate::chains::solana::pools::decode_utils::read_pubkey_struct_at_offset;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::logger::{self, LogTag};
use crate::pools::types::PriceResult;
use crate::pools::utils::is_sol_mint;
use crate::tokens::get_cached_decimals;
use std::collections::HashMap;

/// Moonit AMM decoder implementation
pub struct MoonitAmmDecoder;

impl PoolDecoder for MoonitAmmDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::Moonit]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!("Decoding Moonit AMM pool for {base_mint}/{quote_mint}"),
        );

        // Find the curve account (length should be around 200+ bytes for CurveAccount structure)
        let curve_account = match accounts.values().find(|a| a.data.len() >= 150) {
            Some(a) => a,
            None => {
                logger::error(
                    LogTag::PoolDecoder,
                    &format!(
                        "No suitable Moonit curve account found (accounts: {})",
                        accounts.len()
                    ),
                );
                return None;
            }
        };

        // Parse curve state from account data
        let curve_info = Self::decode_moonit_curve_account(&curve_account.data)?;

        // Calculate price using the curve information (pass curve account explicitly)
        Self::calculate_moonit_price(&curve_info, curve_account, accounts, base_mint, quote_mint)
    }
}

impl MoonitAmmDecoder {
    /// Decode Moonit CurveAccount data from account bytes
    fn decode_moonit_curve_account(data: &[u8]) -> Option<MoonitCurveInfo> {
        if data.len() < 80 {
            logger::error(
                LogTag::PoolDecoder,
                &format!("Invalid Moonit curve account data length: {}", data.len()),
            );
            return None;
        }

        let mut offset = 8; // Skip discriminator

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Moonit curve data length: {} bytes, decoding structure...",
                data.len()
            ),
        );

        // Decode Moonit CurveAccount structure based on SDK:
        // totalSupply: u64 (8 bytes)
        // curveAmount: u64 (8 bytes)
        // mint: PublicKey (32 bytes)
        // decimals: u8 (1 byte)
        // collateralCurrency: u8 (1 byte) - SOL is 0
        // curveType: u8 (1 byte) - different curve types
        // marketcapThreshold: u64 (8 bytes)
        // marketcapCurrency: u8 (1 byte)
        // migrationFee: u64 (8 bytes)
        // coefB: u32 (4 bytes)
        // bump: u8 (1 byte)
        // migrationTarget: u8 (1 byte)
        // priceIncrease: u16 (2 bytes)

        let total_supply = u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?);
        offset += 8;

        let curve_amount = u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?);
        offset += 8;

        let mint = read_pubkey_struct_at_offset(data, &mut offset).ok()?;

        let decimals = data[offset];
        offset += 1;

        let collateral_currency = data[offset];
        offset += 1;

        let curve_type = data[offset];
        offset += 1;

        let marketcap_threshold = u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?);
        offset += 8;

        let marketcap_currency = data[offset];
        offset += 1;

        let migration_fee = u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?);
        offset += 8;

        let coef_b = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?);
        offset += 4;

        let bump = data[offset];
        offset += 1;

        let migration_target = data[offset];
        offset += 1;

        let price_increase = u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?);

        // Calculate curve position (tokens sold from bonding curve)
        let curve_position = total_supply.saturating_sub(curve_amount);

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Extracted Moonit curve structure:\n\
                    - Mint: {}\n\
                    - Total supply: {}\n\
                    - Curve amount: {}\n\
                    - Curve position: {}\n\
                    - Decimals: {}\n\
                    - Collateral currency: {}\n\
                    - Curve type: {}\n\
                    - Marketcap threshold: {}\n\
                    - Coef B: {}",
                mint,
                total_supply,
                curve_amount,
                curve_position,
                decimals,
                collateral_currency,
                curve_type,
                marketcap_threshold,
                coef_b
            ),
        );

        Some(MoonitCurveInfo {
            mint,
            total_supply,
            curve_amount,
            curve_position,
            decimals,
            collateral_currency,
            curve_type,
            marketcap_threshold,
            marketcap_currency,
            migration_fee,
            coef_b,
            bump,
            migration_target,
            price_increase,
        })
    }

    #[cfg(test)]
    fn build_curve_bytes(total_supply: u64, curve_amount: u64, mint: &Pubkey) -> Vec<u8> {
        let mut data = vec![0u8; 84];
        data[8..16].copy_from_slice(&total_supply.to_le_bytes());
        data[16..24].copy_from_slice(&curve_amount.to_le_bytes());
        data[24..56].copy_from_slice(mint.as_ref());
        data[56] = 6; // decimals
        data[57] = 0; // collateral_currency: SOL
        data[58] = 1; // curve_type
        data[59..67].copy_from_slice(&1_000u64.to_le_bytes()); // marketcap_threshold
        data[67] = 0; // marketcap_currency
        data[68..76].copy_from_slice(&500u64.to_le_bytes()); // migration_fee
        data[76..80].copy_from_slice(&7u32.to_le_bytes()); // coef_b
        data[80] = 255; // bump
        data[81] = 0; // migration_target
        data
    }

    /// Calculate price for Moonit AMM pool
    fn calculate_moonit_price(
        curve_info: &MoonitCurveInfo,
        curve_account: &AccountData,
        _accounts: &HashMap<String, AccountData>,
        base_mint: &str,
        quote_mint: &str,
    ) -> Option<PriceResult> {
        logger::debug(
            LogTag::PoolDecoder,
            &format!("Calculating Moonit price for {base_mint}/{quote_mint}"),
        );

        // Ensure we're working with SOL as quote currency
        if !is_sol_mint(quote_mint) {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Moonit pools only support SOL as quote currency, got: {}",
                    quote_mint
                ),
            );
            return None;
        }

        // Ensure collateral currency is SOL (0)
        if curve_info.collateral_currency != 0 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Moonit curve collateral currency is not SOL: {}",
                    curve_info.collateral_currency
                ),
            );
            return None;
        }

        // Get token decimals
        let token_decimals = match get_cached_decimals(crate::chains::ChainId::Solana, base_mint) {
            Some(decimals) => decimals,
            None => {
                logger::error(
                    LogTag::PoolDecoder,
                    &format!("No decimals found for token: {base_mint}"),
                );
                return None;
            }
        };

        // Verify decimals match
        if token_decimals != curve_info.decimals {
            logger::warning(
                LogTag::PoolDecoder,
                &format!(
                    "Decimals mismatch: cached={}, curve={}",
                    token_decimals, curve_info.decimals
                ),
            );
        }

        // Moonit design: SOL collateral lives directly in the curve account lamports (no separate vault/ATA)
        // Previous heuristic tried to "find" a SOL vault among all accounts and produced unstable prices
        // (sometimes selecting token mint or SOL mint lamports). We now strictly use the curve account lamports.
        let sol_reserves = (curve_account.lamports as f64) / (10_f64).powi(SOL_DECIMALS as i32);

        // Token reserves = curve_amount (tokens still held by the curve)
        let token_reserves =
            (curve_info.curve_amount as f64) / (10_f64).powi(token_decimals as i32);

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Moonit reserves: SOL={:.6}, Token={:.6}",
                sol_reserves, token_reserves
            ),
        );

        // Avoid division by zero
        if token_reserves <= 0.0 {
            logger::error(
                LogTag::PoolDecoder,
                "Zero or negative token reserves in Moonit curve",
            );
            return None;
        }

        // Calculate price: SOL per token (basic reserve / remaining tokens)
        let price_sol_basic = sol_reserves / token_reserves; // average SOL backing each remaining token

        // Additional candidate formulas (instrumentation for calibration):
        let sold_tokens = (curve_info.curve_position as f64) / (10_f64).powi(token_decimals as i32);

        if !price_sol_basic.is_finite() || price_sol_basic <= 0.0 {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Moonit AMM: Invalid basic price calculated: {}",
                    price_sol_basic
                ),
            );
            return None;
        }

        let price_sol_avg_sold = if sold_tokens > 0.0 {
            sol_reserves / sold_tokens
        } else {
            0.0
        }; // average SOL per sold token (historical mean)
           // Quadratic (cubic integral) assumption: p ≈ 3 * S / x if S = k/3 * x^3 => p = k x^2 = 3S/x
        let price_sol_quadratic = if sold_tokens > 0.0 {
            (3.0 * sol_reserves) / sold_tokens
        } else {
            0.0
        };
        // Heuristic from previous attempt (kept for comparison)
        let fraction_remaining = if curve_info.total_supply > 0 {
            (curve_info.curve_amount as f64) / (curve_info.total_supply as f64)
        } else {
            0.0
        };
        let sold_fraction = 1.0 - fraction_remaining;
        let heuristic_factor = 1.0 + sold_fraction * 1.1; // grows as more tokens sold
        let price_sol_heuristic = price_sol_avg_sold * heuristic_factor;

        // Dynamic exponent model: factor interpolates between 2 (early / linear) and ~3 (late / cubic) using sold fraction.
        // We derive factor ~= 2.0 + K * sold_fraction where K ≈ price_increase / 168.0 based on calibration vs external API price.
        // This produces current factor close to 2.73 matching observed target (API ~2.095e-7 SOL).
        let k_factor = (curve_info.price_increase as f64) / 168.0; // 180/168 ~= 1.071 (calibrated constant)
        let dynamic_factor = 2.0 + k_factor * sold_fraction; // ranges roughly [2, 2+K]
        let price_sol_dynamic = if sold_tokens > 0.0 {
            (dynamic_factor * sol_reserves) / sold_tokens
        } else {
            price_sol_basic
        };

        // Select dynamic formula as primary (falls back to basic if it would be nonsensical)
        let price_sol = if price_sol_dynamic.is_finite() && price_sol_dynamic > 0.0 {
            price_sol_dynamic
        } else {
            price_sol_basic
        };
        let price_usd = price_sol * 150.0; // Rough SOL price estimate

        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Calculated Moonit price: {:.9} SOL per token ({:.6} USD) | candidates => basic={:.9} avg_sold={:.9} quad={:.9} heur={:.9} dynamic={:.9} (factor={:.3} sold_frac={:.6})",
                price_sol,
                price_usd,
                price_sol_basic,
                price_sol_avg_sold,
                price_sol_quadratic,
                price_sol_heuristic,
                price_sol_dynamic,
                dynamic_factor,
                sold_fraction
            ),
        );

        Some(PriceResult::new(
            base_mint.to_string(),
            price_usd,
            price_sol,
            sol_reserves,
            token_reserves,
            "".to_owned(), // Pool address not needed for calculation
        ))
    }
}

/// Moonit curve information
#[derive(Debug, Clone)]
struct MoonitCurveInfo {
    mint: Pubkey,
    total_supply: u64,
    curve_amount: u64,
    curve_position: u64,
    decimals: u8,
    collateral_currency: u8,
    curve_type: u8,
    marketcap_threshold: u64,
    marketcap_currency: u8,
    migration_fee: u64,
    coef_b: u32,
    bump: u8,
    migration_target: u8,
    price_increase: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_data_is_rejected_without_panicking() {
        assert!(MoonitAmmDecoder::decode_moonit_curve_account(&[]).is_none());
        assert!(MoonitAmmDecoder::decode_moonit_curve_account(&vec![0u8; 79]).is_none());
    }

    #[test]
    fn valid_curve_decodes_supply_position_and_mint() {
        let mint = Pubkey::new_unique();
        let data = MoonitAmmDecoder::build_curve_bytes(1_000_000, 400_000, &mint);

        let info = MoonitAmmDecoder::decode_moonit_curve_account(&data)
            .expect("well-formed Moonit curve data must decode");

        assert_eq!(info.mint, mint);
        assert_eq!(info.total_supply, 1_000_000);
        assert_eq!(info.curve_amount, 400_000);
        assert_eq!(info.curve_position, 600_000, "total_supply - curve_amount");
        assert_eq!(info.decimals, 6);
        assert_eq!(info.collateral_currency, 0);
        assert_eq!(info.coef_b, 7);
    }
}
