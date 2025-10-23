//! Trailing stop loss implementation

use crate::positions::Position;
use crate::trader::config;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Check if a position should be exited based on trailing stop
pub async fn check_trailing_stop(
    position: &Position,
    current_price: f64,
) -> Result<Option<TradeDecision>, String> {
    // Skip if position doesn't have highest price recorded
    if position.price_highest <= 0.0 {
        return Ok(None);
    }

    // Get trailing stop configuration
    let trailing_enabled = config::is_trailing_stop_enabled();
    if !trailing_enabled {
        return Ok(None);
    }

    // Get trailing percentages
    let activation_pct = config::get_trailing_stop_activation_pct();
    let trailing_pct = config::get_trailing_stop_pct();

    // Calculate unrealized profit percentage
    let profit_pct = (current_price / position.entry_price - 1.0) * 100.0;

    // Check if profit exceeds activation threshold
    if profit_pct >= activation_pct {
        // Calculate stop price based on highest recorded price
        let stop_price = position.price_highest * (1.0 - trailing_pct / 100.0);

        // Check if current price fell below stop price
        if current_price <= stop_price {
            return Ok(Some(TradeDecision {
                position_id: position.id.map(|id| id.to_string()),
                mint: position.mint.clone(),
                action: TradeAction::Sell,
                reason: TradeReason::TrailingStop,
                strategy_id: None,
                timestamp: Utc::now(),
                priority: TradePriority::High, // High priority for trailing stops
                price_sol: Some(current_price),
                size_sol: None, // Will sell entire position
            }));
        }
    }

    Ok(None)
}
