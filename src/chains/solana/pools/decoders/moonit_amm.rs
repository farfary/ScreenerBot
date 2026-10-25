// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Moonit bonding-curve decoder
//!
//! Program ID: MOONIT_AMM_PROGRAM_ID (MoonCVVN...)
//!
//! Decodes the curve with the shared `CurveAccount` layout
//! (`crate::chains::solana::pools::layouts::moonit`) and prices the token at the
//! curve's marginal rate: the virtual collateral over the virtual token reserves
//! the programme recomputes from the tokens sold, the same reserves the direct
//! swap venue quotes against. The reported SOL reserve is the collateral the
//! curve has raised (virtual collateral less its initial virtual reserve) and the
//! token reserve is what the curve still holds. Only `ConstantProductV1` curves
//! over SOL collateral are priced; any other curve, a token whose cached
//! decimals disagree with the curve's, or a position past the virtual supply
//! yields no price.

use super::{AccountData, PoolDecoder};
use crate::chains::solana::constants::{MOONIT_AMM_PROGRAM_ID, SOL_DECIMALS};
use crate::chains::solana::pools::layouts::moonit::{
    current_reserves, CurveAccountState, COLLATERAL_CURRENCY_SOL, CURVE_TYPE_CONSTANT_PRODUCT_V1,
    INITIAL_VIRTUAL_COLLATERAL_RESERVES,
};
use crate::chains::solana::pools::types::ProgramKind;
use crate::logger::{self, LogTag};
use crate::pools::types::PriceResult;
use crate::tokens::get_cached_decimals;
use std::collections::HashMap;

/// Moonit bonding-curve decoder implementation
pub struct MoonitAmmDecoder;

impl PoolDecoder for MoonitAmmDecoder {
    fn supported_programs() -> Vec<ProgramKind> {
        vec![ProgramKind::Moonit]
    }

    fn decode_and_calculate(
        accounts: &HashMap<String, AccountData>,
        _base_mint: &str,
        _quote_mint: &str,
    ) -> Option<PriceResult> {
        let curve_account = accounts
            .values()
            .find(|a| a.owner.to_string() == MOONIT_AMM_PROGRAM_ID)?;
        let Some(curve) = CurveAccountState::decode(curve_account.pubkey, &curve_account.data)
        else {
            logger::error(
                LogTag::PoolDecoder,
                &format!(
                    "Moonit curve {} does not match the CurveAccount layout ({} bytes)",
                    curve_account.pubkey,
                    curve_account.data.len()
                ),
            );
            return None;
        };
        let mut price = curve_price(&curve)?;
        price.slot = curve_account.slot;
        Some(price)
    }
}

/// The marginal price of `curve` in SOL per token, with its raised collateral and
/// remaining tokens as the reserves.
fn curve_price(curve: &CurveAccountState) -> Option<PriceResult> {
    if curve.curve_type != CURVE_TYPE_CONSTANT_PRODUCT_V1
        || curve.collateral_currency != COLLATERAL_CURRENCY_SOL
    {
        logger::debug(
            LogTag::PoolDecoder,
            &format!(
                "Moonit curve {} is curve_type {} over collateral {}; only ConstantProductV1 over SOL is priced",
                curve.pool, curve.curve_type, curve.collateral_currency
            ),
        );
        return None;
    }

    let token_mint = curve.mint.to_string();
    let Some(token_decimals) = get_cached_decimals(crate::chains::ChainId::Solana, &token_mint)
    else {
        logger::error(
            LogTag::PoolDecoder,
            &format!("Moonit: Token decimals not found for {token_mint}, skipping price"),
        );
        return None;
    };
    if token_decimals != curve.decimals || token_decimals > 18 {
        logger::warning(
            LogTag::PoolDecoder,
            &format!(
                "Moonit curve {}: cached decimals {} for {} disagree with the curve's {}",
                curve.pool, token_decimals, token_mint, curve.decimals
            ),
        );
        return None;
    }

    let (virtual_tokens, virtual_collateral) = current_reserves(curve.curve_position())?;
    let token_scale = 10f64.powi(token_decimals as i32);
    let sol_scale = 10f64.powi(SOL_DECIMALS as i32);
    let price_sol = (virtual_collateral as f64 / sol_scale) / (virtual_tokens as f64 / token_scale);
    if !price_sol.is_finite() || price_sol <= 0.0 {
        logger::error(
            LogTag::PoolDecoder,
            &format!("Moonit: Invalid price calculated: {price_sol}"),
        );
        return None;
    }

    let raised = virtual_collateral.saturating_sub(INITIAL_VIRTUAL_COLLATERAL_RESERVES);
    let sol_reserves = raised as f64 / sol_scale;
    let token_reserves = curve.curve_amount as f64 / token_scale;

    logger::debug(
        LogTag::PoolDecoder,
        &format!(
            "Moonit price: {:.12} SOL (raised={:.9} SOL, remaining={:.6} tokens, position={})",
            price_sol,
            sol_reserves,
            token_reserves,
            curve.curve_position()
        ),
    );

    let mut price = PriceResult::new(
        token_mint,
        0.0,
        price_sol,
        sol_reserves,
        token_reserves,
        curve.pool.to_string(),
    );
    price.source_pool = Some(ProgramKind::Moonit.display_name().to_string());
    Some(price)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::solana_sdk::pubkey::Pubkey;

    /// A `ConstantProductV1` SOL curve with `sold` raw tokens (9 dp) sold.
    fn curve(sold: u64) -> CurveAccountState {
        let mint = Pubkey::new_unique();
        crate::tokens::cache_decimals(crate::chains::ChainId::Solana, &mint.to_string(), 9);
        let total_supply = 1_000_000_000_000_000_000;
        CurveAccountState {
            pool: Pubkey::new_unique(),
            total_supply,
            curve_amount: total_supply - sold,
            mint,
            decimals: 9,
            collateral_currency: COLLATERAL_CURRENCY_SOL,
            curve_type: CURVE_TYPE_CONSTANT_PRODUCT_V1,
        }
    }

    #[test]
    fn a_fresh_curve_prices_at_its_initial_virtual_reserves() {
        let price = curve_price(&curve(0)).expect("a fresh curve prices");
        // 30 SOL of virtual collateral over 1,073,000,000 virtual tokens
        let expected = 30.0 / 1_073_000_000.0;
        assert!((price.price_native - expected).abs() / expected < 1e-12);
        assert_eq!(price.native_reserves, 0.0, "nothing raised yet");
        assert!((price.token_reserves - 1_000_000_000.0).abs() < 1e-3);
    }

    #[test]
    fn the_price_is_the_marginal_rate_of_the_constant_product() {
        let sold = 500_000_000_000_000_000; // half the virtual supply's tokens sold
        let price = curve_price(&curve(sold)).expect("prices");
        let (vtr, vcr) = current_reserves(sold as u128).unwrap();
        let expected = (vcr as f64 / 1e9) / (vtr as f64 / 1e9);
        assert!((price.price_native - expected).abs() / expected < 1e-12);
        // The price rises with the position: (1073/573)^2 of the start.
        let start = 30.0 / 1_073_000_000.0;
        assert!(price.price_native > 3.0 * start);
        let raised = (vcr - INITIAL_VIRTUAL_COLLATERAL_RESERVES) as f64 / 1e9;
        assert!((price.native_reserves - raised).abs() < 1e-9);
    }

    #[test]
    fn a_curve_other_than_constant_product_over_sol_yields_no_price() {
        let mut other_curve = curve(1_000);
        other_curve.curve_type = 2;
        assert!(curve_price(&other_curve).is_none());

        let mut other_collateral = curve(1_000);
        other_collateral.collateral_currency = 1;
        assert!(curve_price(&other_collateral).is_none());
    }

    #[test]
    fn cached_decimals_that_disagree_with_the_curve_yield_no_price() {
        let mut c = curve(1_000);
        c.decimals = 6;
        assert!(curve_price(&c).is_none());
    }
}
