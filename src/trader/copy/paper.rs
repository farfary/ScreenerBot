//! Deterministic paper fills using the decision-time pool price.

use super::types::{CopySkip, PaperFill};

/// Jupiter's mandatory referral fee. Kept equal to the hardcoded router constant;
/// it is not configurable because paper results must model the live cost path.
pub const PAPER_REFERRAL_FEE_BPS: u16 = 50;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaperCosts {
    pub network_fee_sol: f64,
    pub priority_fee_sol: f64,
}

pub fn simulate_fill(
    input_sol: f64,
    market_price_sol: f64,
    slippage_pct: f64,
    costs: PaperCosts,
) -> Result<PaperFill, CopySkip> {
    if !input_sol.is_finite() || input_sol <= 0.0 {
        return Err(CopySkip::InvalidSizing);
    }
    if !market_price_sol.is_finite() || market_price_sol <= 0.0 {
        return Err(CopySkip::InvalidPrice);
    }
    let fill_price_sol = market_price_sol * (1.0 + slippage_pct / 100.0);
    let referral_fee_sol = input_sol * f64::from(PAPER_REFERRAL_FEE_BPS) / 10_000.0;
    let token_amount = (input_sol - referral_fee_sol) / fill_price_sol;
    Ok(PaperFill {
        input_sol,
        market_price_sol,
        fill_price_sol,
        token_amount,
        referral_fee_sol,
        network_fee_sol: costs.network_fee_sol,
        priority_fee_sol: costs.priority_fee_sol,
        total_cost_sol: input_sol + costs.network_fee_sol + costs.priority_fee_sol,
    })
}
