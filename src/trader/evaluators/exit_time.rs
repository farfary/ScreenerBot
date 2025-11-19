//! Time-based exit override

use crate::positions::Position;
use crate::trader::config;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Check if a position should be exited based on time override rules
pub async fn check_time_override(
    position: &Position,
    current_price: f64,
) -> Result<Option<TradeDecision>, String> {
    // Get time override configuration
    let loss_threshold_pct = config::get_time_override_loss_threshold_pct();
    let duration_hours = config::get_time_override_duration_hours();

    // Calculate position age in hours
    let position_age_hours = (Utc::now() - position.entry_time).num_seconds() as f64 / 3600.0;

    // Check if position has exceeded duration threshold
    if position_age_hours >= duration_hours {
        // Calculate current loss percentage using average entry price
        let entry_price = position.average_entry_price;
        if entry_price <= 0.0 || !entry_price.is_finite() {
            return Ok(None);
        }

        let loss_pct = (1.0 - current_price / entry_price) * 100.0;

        // Check if loss exceeds threshold
        if loss_pct >= loss_threshold_pct {
            return Ok(Some(TradeDecision {
                position_id: position.id.map(|id| id.to_string()),
                mint: position.mint.clone(),
                action: TradeAction::Sell,
                reason: TradeReason::TimeOverride,
                strategy_id: None,
                timestamp: Utc::now(),
                priority: TradePriority::High,
                price_sol: Some(current_price),
                size_sol: None, // Sell entire position
            }));
        }
    }

    Ok(None)
}
