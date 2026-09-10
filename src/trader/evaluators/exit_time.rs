//! Time-based exit override

use crate::positions::Position;
use crate::trader::policy::TimePolicy;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// The time-override rule on plain numbers, shared by the live evaluator and the
/// paper copy book: exit a position held at least `duration_seconds` whose P&L is
/// at or below `loss_threshold_pct` (negative, e.g. -40 = a 40% loss or worse).
/// `Err` carries the detail of an invalid configuration.
pub fn time_override_triggered(
    entry_price: f64,
    current_price: f64,
    held_seconds: f64,
    policy: &TimePolicy,
) -> Result<bool, String> {
    if !policy.enabled {
        return Ok(false);
    }
    let loss_threshold_pct = policy.loss_threshold_pct;
    let duration_seconds = policy.duration_seconds;
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err(format!(
            "invalid time_override_duration: {duration_seconds} seconds"
        ));
    }
    if !loss_threshold_pct.is_finite() {
        return Err(format!(
            "invalid time_override_loss_threshold_pct: {loss_threshold_pct}"
        ));
    }
    // A positive threshold would exit on profit, which is a misconfiguration.
    if loss_threshold_pct > 0.0 {
        return Err(format!(
            "invalid time_override_loss_threshold_pct: {loss_threshold_pct} (must be <= 0 to represent loss)"
        ));
    }
    if held_seconds < duration_seconds || entry_price <= 0.0 || !entry_price.is_finite() {
        return Ok(false);
    }
    let pnl_pct = ((current_price - entry_price) / entry_price) * 100.0;
    Ok(pnl_pct <= loss_threshold_pct)
}

/// Check if a position should be exited based on time override rules
///
/// Supports flexible time units: seconds, minutes, hours, days (resolved into
/// `duration_seconds` by the policy snapshot).
pub async fn check_time_override(
    position: &Position,
    current_price: f64,
    policy: &TimePolicy,
) -> crate::trader::Result<Option<TradeDecision>> {
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(crate::positions::Error::InvalidPrice {
            mint: position.mint.clone(),
            price: current_price,
        }
        .into());
    }
    let held_seconds = (Utc::now() - position.entry_time).num_seconds() as f64;
    let triggered = time_override_triggered(
        position.average_entry_price,
        current_price,
        held_seconds,
        policy,
    )
    .map_err(|detail| crate::trader::Error::StrategyEvaluation {
        mint: position.mint.clone(),
        detail,
    })?;
    if !triggered {
        return Ok(None);
    }
    Ok(Some(TradeDecision {
        position_id: position.id.map(|id| id.to_string()),
        mint: position.mint.clone(),
        action: TradeAction::Sell,
        reason: TradeReason::TimeOverride,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::High,
        price_sol: Some(current_price),
        size_sol: None, // Sell entire position
        exit_percentage: None,
        // Auto-trader slippage always follows config.
        slippage_pct: None,
    }))
}
