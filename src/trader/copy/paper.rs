//! Deterministic paper fills using the decision-time pool price.

use super::types::{CopySkip, PaperFill, PaperSellFill};

/// Jupiter's mandatory referral fee. Kept equal to the hardcoded router constant;
/// it is not configurable because paper results must model the live cost path.
pub const PAPER_REFERRAL_FEE_BPS: u16 = 50;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaperCosts {
    pub network_fee_sol: f64,
    pub priority_fee_sol: f64,
}

/// The decision-time price a paper fill trades at, and where it came from. A
/// token the pool service does not track is priced at the observed trade's own
/// price instead; such a fill is exactly the configured slippage away from the
/// target by construction, so it measures nothing about execution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaperMarket {
    pub price_sol: f64,
    pub from_pool: bool,
}

impl PaperMarket {
    pub const fn pool(price_sol: f64) -> Self {
        Self {
            price_sol,
            from_pool: true,
        }
    }

    pub const fn observed(price_sol: f64) -> Self {
        Self {
            price_sol,
            from_pool: false,
        }
    }
}

pub fn simulate_fill(
    input_sol: f64,
    market: PaperMarket,
    slippage_pct: f64,
    costs: PaperCosts,
) -> Result<PaperFill, CopySkip> {
    if !input_sol.is_finite() || input_sol <= 0.0 {
        return Err(CopySkip::InvalidSizing);
    }
    let market_price_sol = market.price_sol;
    if !market_price_sol.is_finite() || market_price_sol <= 0.0 {
        return Err(CopySkip::InvalidPrice);
    }
    let fill_price_sol = market_price_sol * (1.0 + slippage_pct / 100.0);
    let referral_fee_sol = input_sol * f64::from(PAPER_REFERRAL_FEE_BPS) / 10_000.0;
    let token_amount = (input_sol - referral_fee_sol) / fill_price_sol;
    Ok(PaperFill {
        input_sol,
        market_price_sol,
        priced_from_pool: market.from_pool,
        fill_price_sol,
        token_amount,
        referral_fee_sol,
        network_fee_sol: costs.network_fee_sol,
        priority_fee_sol: costs.priority_fee_sol,
        total_cost_sol: input_sol + costs.network_fee_sol + costs.priority_fee_sol,
    })
}

/// Sell `token_amount` from the paper book at the decision-time price less
/// slippage, paying the same referral fee and network costs a live sell would.
pub fn simulate_sell(
    token_amount: f64,
    market: PaperMarket,
    slippage_pct: f64,
    costs: PaperCosts,
) -> Result<PaperSellFill, CopySkip> {
    if !token_amount.is_finite() || token_amount <= 0.0 {
        return Err(CopySkip::CopyPositionNotFound);
    }
    let market_price_sol = market.price_sol;
    if !market_price_sol.is_finite() || market_price_sol <= 0.0 {
        return Err(CopySkip::InvalidPrice);
    }
    let fill_price_sol = market_price_sol * (1.0 - slippage_pct / 100.0).max(0.0);
    let gross_sol = token_amount * fill_price_sol;
    let referral_fee_sol = gross_sol * f64::from(PAPER_REFERRAL_FEE_BPS) / 10_000.0;
    Ok(PaperSellFill {
        token_amount,
        market_price_sol,
        priced_from_pool: market.from_pool,
        fill_price_sol,
        gross_sol,
        referral_fee_sol,
        network_fee_sol: costs.network_fee_sol,
        priority_fee_sol: costs.priority_fee_sol,
        net_proceeds_sol: gross_sol
            - referral_fee_sol
            - costs.network_fee_sol
            - costs.priority_fee_sol,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const COSTS: PaperCosts = PaperCosts {
        network_fee_sol: 0.000005,
        priority_fee_sol: 0.0,
    };

    #[test]
    fn a_round_trip_at_an_unchanged_price_loses_exactly_slippage_and_fees() {
        let buy = simulate_fill(1.0, PaperMarket::pool(0.01), 1.0, COSTS).unwrap();
        let sell = simulate_sell(buy.token_amount, PaperMarket::pool(0.01), 1.0, COSTS).unwrap();
        let expected = buy.token_amount * 0.01 * 0.99 * (1.0 - 0.005) - COSTS.network_fee_sol;
        assert!((sell.net_proceeds_sol - expected).abs() < 1e-12);
        assert!(sell.net_proceeds_sol < buy.total_cost_sol);
    }

    #[test]
    fn a_sell_without_tokens_or_price_is_refused() {
        assert_eq!(
            simulate_sell(0.0, PaperMarket::pool(0.01), 1.0, COSTS),
            Err(CopySkip::CopyPositionNotFound)
        );
        assert_eq!(
            simulate_sell(10.0, PaperMarket::observed(f64::NAN), 1.0, COSTS),
            Err(CopySkip::InvalidPrice)
        );
    }
}
