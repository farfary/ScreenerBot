//! Pure task-level gates that run before sizing or paper execution.

use crate::trader::constants::MAX_MANUAL_SLIPPAGE_PCT;

use super::types::{CopySkip, CopyTask, PipelinePolicy, RiskContext, SpendState};

pub fn precheck(
    task: &CopyTask,
    target_size_sol: f64,
    spend: SpendState,
    context: RiskContext,
    policy: PipelinePolicy,
) -> Result<(), CopySkip> {
    if !task.enabled {
        return Err(CopySkip::TaskDisabled);
    }
    if context.is_self_wallet {
        return Err(CopySkip::SelfCopy);
    }
    if !target_size_sol.is_finite() || target_size_sol <= 0.0 {
        return Err(CopySkip::InvalidSizing);
    }
    if let Some(minimum_sol) = task.min_target_trade_sol {
        if target_size_sol < minimum_sol {
            return Err(CopySkip::TargetBelowMinimum { minimum_sol });
        }
    }
    if let Some(maximum_sol) = task.max_target_trade_sol {
        if target_size_sol > maximum_sol {
            return Err(CopySkip::TargetAboveMaximum { maximum_sol });
        }
    }
    if task.buy_once_per_token && spend.token_buy_count > 0 {
        return Err(CopySkip::AlreadyBought);
    }
    if context.mint_blacklisted {
        return Err(CopySkip::Blacklisted);
    }
    if policy.require_filter_pass && !context.filter_passed {
        return Err(CopySkip::FilterRequired);
    }
    if !task.slippage_pct.is_finite()
        || task.slippage_pct <= 0.0
        || task.slippage_pct > MAX_MANUAL_SLIPPAGE_PCT
    {
        return Err(CopySkip::InvalidSlippage {
            maximum_pct: MAX_MANUAL_SLIPPAGE_PCT,
        });
    }
    Ok(())
}
