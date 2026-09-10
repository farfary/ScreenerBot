//! Return on Investment (ROI) based exit strategy

use crate::positions::Position;
use crate::trader::policy::RoiPolicy;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// The ROI-target rule on plain numbers, shared by the live evaluator and the
/// paper copy book.
pub fn roi_target_reached(entry_price: f64, current_price: f64, policy: &RoiPolicy) -> bool {
    policy.enabled
        && entry_price.is_finite()
        && entry_price > 0.0
        && (current_price / entry_price - 1.0) * 100.0 >= policy.target_profit_pct
}

/// Check if a position should be exited based on ROI target
pub async fn check_roi_exit(
    position: &Position,
    current_price: f64,
    policy: &RoiPolicy,
) -> crate::trader::Result<Option<TradeDecision>> {
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(crate::positions::Error::InvalidPrice {
            mint: position.mint.clone(),
            price: current_price,
        }
        .into());
    }
    if !roi_target_reached(position.average_entry_price, current_price, policy) {
        return Ok(None);
    }
    Ok(Some(TradeDecision {
        position_id: position.id.map(|id| id.to_string()),
        mint: position.mint.clone(),
        action: TradeAction::Sell,
        reason: TradeReason::TakeProfit,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::Normal,
        price_sol: Some(current_price),
        size_sol: None, // Will sell entire position
        exit_percentage: None,
        // Auto-trader slippage always follows config.
        slippage_pct: None,
    }))
}
