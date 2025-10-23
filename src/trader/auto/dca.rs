//! Dollar Cost Averaging implementation

use crate::logger::{log, LogTag};
use crate::positions;
use crate::trader::config;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::{Duration, Utc};

/// Process DCA opportunities for eligible positions
pub async fn process_dca_opportunities() -> Result<Vec<TradeDecision>, String> {
    // Check if DCA is enabled
    let dca_enabled = config::is_dca_enabled();
    if !dca_enabled {
        return Ok(Vec::new());
    }

    let dca_threshold_pct = config::get_dca_threshold_pct();
    let dca_max_count = config::get_dca_max_count();
    let dca_size_pct = config::get_dca_size_percentage();
    let dca_cooldown_minutes = config::get_dca_cooldown_minutes();

    // Get all open positions
    let open_positions = positions::get_open_positions().await;
    if open_positions.is_empty() {
        return Ok(Vec::new());
    }

    let mut dca_decisions = Vec::new();

    for position in open_positions {
        // Skip if position doesn't have required data
        let position_id = match position.id {
            Some(id) => id,
            None => continue,
        };

        let current_price = match position.current_price {
            Some(price) if price > 0.0 && price.is_finite() => price,
            _ => continue,
        };

        // Check if already at DCA limit
        if position.dca_count >= dca_max_count as u32 {
            continue;
        }

        // Check DCA cooldown
        if let Some(last_dca_time) = position.last_dca_time {
            let elapsed_minutes = (Utc::now() - last_dca_time).num_minutes();
            if elapsed_minutes < dca_cooldown_minutes {
                continue;
            }
        }

        // Calculate P&L percentage
        let entry_price = position.average_entry_price;
        if entry_price <= 0.0 || !entry_price.is_finite() {
            continue;
        }

        let pnl_pct = ((current_price - entry_price) / entry_price) * 100.0;

        // Check if below DCA threshold
        if pnl_pct >= dca_threshold_pct {
            continue; // Not losing enough to DCA
        }

        // Calculate DCA amount
        let initial_size_sol = position.entry_size_sol;
        let dca_amount_sol = initial_size_sol * (dca_size_pct / 100.0);

        log(
            LogTag::Trader,
            "DCA_OPPORTUNITY",
            &format!(
                "📉 DCA opportunity: {} | P&L: {:.2}% | Price: {:.9} SOL → {:.9} SOL | DCA #{} | Amount: {:.4} SOL",
                position.symbol,
                pnl_pct,
                entry_price,
                current_price,
                position.dca_count + 1,
                dca_amount_sol
            ),
        );

        dca_decisions.push(TradeDecision {
            position_id: Some(position_id.to_string()),
            mint: position.mint.clone(),
            action: TradeAction::DCA,
            reason: TradeReason::DCAScheduled,
            strategy_id: None,
            timestamp: Utc::now(),
            priority: TradePriority::Normal,
            price_sol: Some(current_price),
            size_sol: Some(dca_amount_sol),
        });
    }

    if !dca_decisions.is_empty() {
        log(
            LogTag::Trader,
            "INFO",
            &format!("Found {} DCA opportunities", dca_decisions.len()),
        );
    }

    Ok(dca_decisions)
}
