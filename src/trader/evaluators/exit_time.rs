//! Time-based exit override

use crate::positions::Position;
use crate::trader::config;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Check if a position should be exited based on time override rules
///
/// Time override exits positions that:
/// 1. Have been held longer than configured duration
/// 2. Are at a loss exceeding the configured threshold (negative %)
///
/// Purpose: Force exit positions stuck in losses for extended periods
/// Supports flexible time units: seconds, minutes, hours, days
pub async fn check_time_override(
    position: &Position,
    current_price: f64,
) -> Result<Option<TradeDecision>, String> {
    // Validate current price
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(format!(
            "Invalid current_price for time override: {}",
            current_price
        ));
    }

    // Check if time override is enabled
    let time_override_enabled = config::is_time_override_enabled();
    if !time_override_enabled {
        return Ok(None);
    }

    // Get time override configuration
    let loss_threshold_pct = config::get_time_override_loss_threshold_pct();
    let duration_seconds = config::get_time_override_duration_seconds();

    // Defensive runtime validation of config values
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err(format!(
            "Invalid time_override_duration: {} seconds",
            duration_seconds
        ));
    }
    if !loss_threshold_pct.is_finite() {
        return Err(format!(
            "Invalid time_override_loss_threshold_pct: {}",
            loss_threshold_pct
        ));
    }
    // Warn if threshold is positive (would exit on profit, likely misconfigured)
    if loss_threshold_pct > 0.0 {
        return Err(format!(
            "Invalid time_override_loss_threshold_pct: {} (must be <= 0 to represent loss)",
            loss_threshold_pct
        ));
    }

    // Calculate position age in seconds
    let position_age_seconds = (Utc::now() - position.entry_time).num_seconds() as f64;

    // Check if position has exceeded duration threshold
    if position_age_seconds >= duration_seconds {
        // Calculate current P&L percentage using average entry price
        let entry_price = position.average_entry_price;
        if entry_price <= 0.0 || !entry_price.is_finite() {
            return Ok(None);
        }

        let pnl_pct = ((current_price - entry_price) / entry_price) * 100.0;

        // Configuration value is negative (e.g. -40) meaning "exit if loss is 40% or worse"
        if pnl_pct <= loss_threshold_pct {
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
