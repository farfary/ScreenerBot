//! Dollar Cost Averaging implementation

use crate::logger::{self, LogTag};
use crate::positions;
use crate::trader::auto::dca_evaluation::{DcaConfigSnapshot, DcaEvaluation};
use crate::trader::config;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Process DCA opportunities for eligible positions
pub async fn process_dca_opportunities() -> Result<Vec<TradeDecision>, String> {
    // Build config snapshot (batch read)
    let dca_config = DcaConfigSnapshot {
        enabled: config::is_dca_enabled(),
        max_count: config::get_dca_max_count() as u32,
        cooldown_minutes: config::get_dca_cooldown_minutes(),
        threshold_pct: config::get_dca_threshold_pct(),
        size_percentage: config::get_dca_size_percentage(),
    };

    // Early exit if DCA is disabled
    if !dca_config.enabled {
        return Ok(Vec::new());
    }

    // Get all open positions
    let open_positions = positions::get_open_positions().await;
    if open_positions.is_empty() {
        return Ok(Vec::new());
    }

    let mut dca_decisions = Vec::new();

    for position in open_positions {
        // Skip if position doesn't have ID
        let position_id = match position.id {
            Some(id) => id,
            None => continue,
        };

        // Evaluate DCA opportunity using structured evaluation
        let evaluation = match DcaEvaluation::evaluate(&position, dca_config.clone()) {
            Ok(eval) => eval,
            Err(e) => {
                logger::error(
                    LogTag::Trader,
                    &format!("DCA evaluation failed for {}: {}", position.symbol, e),
                );
                continue;
            }
        };

        if evaluation.should_trigger {
            logger::info(
                LogTag::Trader,
                &format!(
                    "📉 DCA opportunity: {} | {}",
                    position.symbol,
                    evaluation.summary()
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
                price_sol: Some(evaluation.calculations.current_price),
                size_sol: Some(evaluation.calculations.dca_amount_sol),
            });
        } else {
            // Always log debug details via the centralized logger; the logger will filter by level
            logger::debug(
                LogTag::Trader,
                &format!(
                    "DCA not triggered for {}: {}",
                    position.symbol,
                    evaluation.summary()
                ),
            );
        }
    }

    if !dca_decisions.is_empty() {
        logger::info(
            LogTag::Trader,
            &format!("Found {} DCA opportunities", dca_decisions.len()),
        );
    }

    Ok(dca_decisions)
}
