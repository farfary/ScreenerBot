//! Return on Investment (ROI) based exit strategy

use crate::positions::Position;
use crate::trader::config;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Check if a position should be exited based on ROI target
pub async fn check_roi_exit(
    position: &Position,
    current_price: f64,
) -> Result<Option<TradeDecision>, String> {
    // Validate current price
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(format!("Invalid current_price for ROI exit: {}", current_price));
    }

    // Check if ROI-based exit is enabled
    let roi_enabled = config::is_roi_exit_enabled();
    if !roi_enabled {
        return Ok(None);
    }

    // Get target ROI percentage
    let target_profit_pct = config::get_target_profit_pct();

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
        }));
    }

    Ok(None)
}
