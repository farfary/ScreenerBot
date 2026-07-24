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
    // Validate current price
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(format!(
            "Invalid current_price for trailing stop: {}",
            current_price
        ));
    }

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
    let distance_pct = config::get_trailing_stop_distance_pct();

    // Runtime validation: distance must be less than activation to prevent impossible conditions
    if distance_pct >= activation_pct {
        return Err(format!(
            "Invalid trailing stop config: distance_pct ({:.1}%) must be less than activation_pct ({:.1}%)",
            distance_pct, activation_pct
        ));
    }

    // Calculate unrealized profit percentage using average entry price
    let entry_price = position.average_entry_price;
    if entry_price <= 0.0 || !entry_price.is_finite() {
        return Ok(None);
    }

    // ARM THE TRAIL OFF THE PEAK, NEVER OFF THE CURRENT PRICE.
    //
    // A trailing stop exists to protect a profit the position has ALREADY made, so what
    // arms it is how far the price has run at its best (`price_highest`, a persisted
    // running maximum seeded from the entry). Measuring activation against the CURRENT
    // price instead makes the trail un-arm itself as the price falls — which is exactly
    // when it is supposed to act. Concretely, with activation 20% and distance 10%: a
    // position that peaked at +25% and is now retracing through +12% is below the stop
    // (peak * 0.9 = +12.5%) but its CURRENT profit is under the 20% activation, so the
    // trail stayed silent and the position rode all the way down to the stop loss. The
    // gain it was meant to lock in was given back in full.
    let peak_profit_pct = (position.price_highest / entry_price - 1.0) * 100.0;

    // Check if the peak profit reached the activation threshold
    if peak_profit_pct >= activation_pct {
        // Calculate stop price based on highest recorded price
        let stop_price = position.price_highest * (1.0 - distance_pct / 100.0);

        // Only trigger if still profitable - prevents exits at loss after price retracement
        let current_profit_pct = (current_price / entry_price - 1.0) * 100.0;

        // Check if current price fell below stop price AND position is still profitable
        if current_price <= stop_price && current_profit_pct > 0.0 {
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
                exit_percentage: None,
                // Auto-trader slippage always follows config.
                slippage_pct: None,
            }));
        }
    }

    Ok(None)
}
