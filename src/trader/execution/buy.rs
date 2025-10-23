//! Buy operation execution

use crate::logger::{log, LogTag};
use crate::positions;
use crate::trader::config;
use crate::trader::types::{TradeDecision, TradeResult};

/// Execute a buy trade
pub async fn execute_buy(decision: &TradeDecision) -> Result<TradeResult, String> {
    log(
        LogTag::Trader,
        "INFO",
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

    // Call positions open with size so manual size is honored
    match positions::open_position_with_size(&decision.mint, trade_size_sol).await {
        Ok(transaction_signature) => {
            log(
                LogTag::Trader,
                "SUCCESS",
                &format!(
                    "✅ Buy executed: {} | ~{} SOL | TX: {}",
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
            let error = format!("Buy execution failed: {}", e);
            log(LogTag::Trader, "ERROR", &error);
            Ok(TradeResult::failure(decision.clone(), error, 0))
        }
    }
}

/// Execute a DCA (dollar cost averaging) buy
pub async fn execute_dca(decision: &TradeDecision) -> Result<TradeResult, String> {
    log(
        LogTag::Trader,
        "INFO",
        &format!(
            "Executing DCA for position {} token {}",
            decision
                .position_id
                .as_ref()
                .unwrap_or(&"unknown".to_string()),
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
            log(
                LogTag::Trader,
                "SUCCESS",
                &format!(
                    "✅ DCA executed: {} | {} SOL | TX: {}",
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
            log(LogTag::Trader, "ERROR", &error);
            Ok(TradeResult::failure(decision.clone(), error, 0))
        }
    }
}
