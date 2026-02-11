//! Buy operation execution

use crate::logger::{self, LogTag};
use crate::positions;
use crate::trader::config;
use crate::trader::types::{TradeDecision, TradeResult};

/// Execute a buy trade
pub async fn execute_buy(decision: &TradeDecision) -> Result<TradeResult, String> {
    // Check connectivity before executing trade - critical operation
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc"]).await {
        let error = format!("Cannot execute buy - Unhealthy endpoints: {}", unhealthy);
        logger::error(LogTag::Trader, &error);
        return Ok(TradeResult::failure(decision.clone(), error, 0));
    }

    logger::info(
        LogTag::Trader,
        &format!(
            "Executing buy for token {} (reason: {:?}, strategy: {:?})",
            decision.mint, decision.reason, decision.strategy_id
        ),
    );

    // Note: Trade size is read from config by open_position_direct
    // decision.size_sol is informational only - actual size comes from cfg.trader.trade_size_sol
    let trade_size_sol = decision
        .size_sol
        .unwrap_or_else(|| config::get_trade_size_sol());

    // Enforce maximum trade size limit
    let max_allowed = config::get_trade_size_sol() * crate::trader::constants::MAX_TRADE_SIZE_MULTIPLIER;
    let trade_size_sol = trade_size_sol.min(max_allowed);

    // Call positions open with size so manual size is honored
    match positions::open_position_with_size(&decision.mint, trade_size_sol).await {
        Ok(transaction_signature) => {
            logger::info(
                LogTag::Trader,
                &format!(
                    "Buy executed: {} | ~{} SOL | TX: {}",
                    decision.mint, trade_size_sol, transaction_signature
                ),
            );

            Ok(TradeResult::success(
                decision.clone(),
                transaction_signature,
                decision.price_sol.unwrap_or(0.0), // Will be updated by verification
                trade_size_sol,
                None, // Position ID will be set by verification
            ))
        }
        Err(e) => {
            if let Some(remaining) = crate::positions::parse_position_slot_error(&e) {
                let guard_msg = format!(
                    "{}: global position limit reached (remaining permits: {})",
                    crate::positions::POSITION_SLOT_UNAVAILABLE_ERR,
                    remaining
                );

                logger::debug(
                    LogTag::Trader,
                    &format!(
            "Entry guard engaged for {} – max open positions reached (permits left: {})",
            decision.mint, remaining
          ),
                );

                return Ok(TradeResult::failure(decision.clone(), guard_msg, 0));
            }

            let error = format!("Buy execution failed: {}", e);
            logger::error(LogTag::Trader, &error);
            Ok(TradeResult::failure(decision.clone(), error, 0))
        }
    }
}

/// Execute a DCA (dollar cost averaging) buy
pub async fn execute_dca(decision: &TradeDecision) -> Result<TradeResult, String> {
    // Check connectivity before executing DCA - critical operation
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc"]).await {
        let error = format!("Cannot execute DCA - Unhealthy endpoints: {}", unhealthy);
        logger::error(LogTag::Trader, &error);
        return Ok(TradeResult::failure(decision.clone(), error, 0));
    }

    logger::info(
        LogTag::Trader,
        &format!(
            "Executing DCA for position {} token {}",
            decision.position_id.as_deref().unwrap_or("unknown"),
            decision.mint
        ),
    );

    // Determine DCA amount from decision or config
    let dca_amount_sol = decision
        .size_sol
        .unwrap_or_else(|| config::get_trade_size_sol() * 0.5); // Default to 50% of initial size

    // Call positions::add_to_position to handle DCA entry
    match positions::add_to_position(&decision.mint, dca_amount_sol).await {
        Ok(transaction_signature) => {
            logger::info(
                LogTag::Trader,
                &format!(
                    "DCA executed: {} | {} SOL | TX: {}",
                    decision.mint, dca_amount_sol, transaction_signature
                ),
            );

            Ok(TradeResult::success(
                decision.clone(),
                transaction_signature,
                decision.price_sol.unwrap_or(0.0),
                dca_amount_sol,
                decision.position_id.clone(),
            ))
        }
        Err(e) => {
            let error = format!("DCA execution failed: {}", e);
            logger::error(LogTag::Trader, &error);
            Ok(TradeResult::failure(decision.clone(), error, 0))
        }
    }
}
