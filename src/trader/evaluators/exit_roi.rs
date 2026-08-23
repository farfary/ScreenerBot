//! Return on Investment (ROI) based exit strategy

use crate::positions::Position;
use crate::trader::policy::RoiPolicy;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Check if a position should be exited based on ROI target
pub async fn check_roi_exit(
    position: &Position,
    current_price: f64,
    policy: &RoiPolicy,
) -> crate::trader::Result<Option<TradeDecision>> {
    // Validate current price
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(crate::positions::Error::InvalidPrice {
            mint: position.mint.clone(),
            price: current_price,
        }
        .into());
    }

    // Check if ROI-based exit is enabled
    let roi_enabled = policy.enabled;
    if !roi_enabled {
        return Ok(None);
    }

    // Get target ROI percentage
    let target_profit_pct = policy.target_profit_pct;

    // Calculate unrealized profit percentage using average entry price
    let entry_price = position.average_entry_price;
    if entry_price <= 0.0 || !entry_price.is_finite() {
        return Ok(None);
    }

    let profit_pct = (current_price / entry_price - 1.0) * 100.0;

    // Check if profit exceeds target
    if profit_pct >= target_profit_pct {
        return Ok(Some(TradeDecision {
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
        }));
    }

    Ok(None)
}
