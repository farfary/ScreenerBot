//! Pure copy sizing with every hard cap applied in one place.

use crate::trader::constants::{MAX_TRADE_SIZE_MULTIPLIER, MIN_TRADE_SIZE_SOL};

use super::types::{CopySkip, CopyTask, SizingMode, SpendState};

pub fn size_for(
    task: &CopyTask,
    target_size_sol: f64,
    spend: SpendState,
    engine_trade_size_sol: f64,
) -> Result<f64, CopySkip> {
    let requested = match task.sizing {
        SizingMode::Fixed { sol } => sol,
        SizingMode::RatioOfTarget { pct } if pct.is_finite() && pct > 0.0 => {
            target_size_sol * pct / 100.0
        }
        SizingMode::RatioOfTarget { .. } => return Err(CopySkip::InvalidSizing),
        SizingMode::PercentOfTargetPortfolio { .. } => return Err(CopySkip::UnsupportedSizingMode),
    };

    if !requested.is_finite()
        || requested <= 0.0
        || !engine_trade_size_sol.is_finite()
        || engine_trade_size_sol <= 0.0
    {
        return Err(CopySkip::InvalidSizing);
    }

    let budget_remaining = (task.total_budget_sol - spend.total_spent_sol).max(0.0);
    if budget_remaining < MIN_TRADE_SIZE_SOL {
        return Err(CopySkip::BudgetExhausted);
    }
    let token_remaining = (task.max_sol_per_token - spend.token_spent_sol).max(0.0);
    if token_remaining < MIN_TRADE_SIZE_SOL {
        return Err(CopySkip::TokenCapReached);
    }

    let engine_ceiling = engine_trade_size_sol * MAX_TRADE_SIZE_MULTIPLIER;
    let sized = requested
        .min(task.max_sol_per_trade)
        .min(token_remaining)
        .min(budget_remaining)
        .min(engine_ceiling);

    if !sized.is_finite() || sized < MIN_TRADE_SIZE_SOL {
        return Err(CopySkip::BelowMinimumSize {
            minimum_sol: MIN_TRADE_SIZE_SOL,
        });
    }
    Ok(sized)
}
