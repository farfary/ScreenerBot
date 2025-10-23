//! Risk management utilities

use crate::positions::Position;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Check if a position should be exited based on risk limits
pub async fn check_risk_limits(
    position: &Position,
    current_price: f64,
) -> Result<Option<TradeDecision>, String> {
    // For now, this is a placeholder for future risk management features
    // Examples of what could be added:
    // - Maximum loss per position
    // - Maximum daily loss limits
    // - Portfolio-level risk checks
    // - Volatility-based position sizing adjustments

    // Check for extreme losses (as a basic safety measure)
    // Use average_entry_price which accounts for DCA entries
    let entry_price = position.average_entry_price;
    if entry_price <= 0.0 || !entry_price.is_finite() {
        return Ok(None); // Invalid entry price, skip risk check
    }
    
    let loss_pct = (1.0 - current_price / entry_price) * 100.0;

    // If loss exceeds 90%, trigger emergency exit
    if loss_pct >= 90.0 {
        return Ok(Some(TradeDecision {
            position_id: position.id.map(|id| id.to_string()),
            mint: position.mint.clone(),
            action: TradeAction::Sell,
            reason: TradeReason::RiskManagement,
            strategy_id: None,
            timestamp: Utc::now(),
            priority: TradePriority::Emergency,
            price_sol: Some(current_price),
            size_sol: None,
        }));
    }

    Ok(None)
}
