//! Sell operation execution

use crate::config::with_config;
use crate::logger::{log, LogTag};
use crate::positions;
use crate::trader::types::{TradeDecision, TradeReason, TradeResult};

/// Execute a sell trade
pub async fn execute_sell(decision: &TradeDecision) -> Result<TradeResult, String> {
    log(
        LogTag::Trader,
        "INFO",
        &format!(
            "Executing sell for position {} token {} (reason: {:?})",
            decision
                .position_id
                .as_ref()
                .unwrap_or(&"unknown".to_string()),
            decision.mint,
            decision.reason
        ),
    );

    // Determine exit type based on configuration and reason
    let partial_exit_enabled = with_config(|cfg| cfg.positions.partial_exit_enabled);
    let is_emergency_exit = matches!(
        decision.reason,
        TradeReason::StopLoss | TradeReason::Blacklisted | TradeReason::ForceSell
    );

    // Emergency exits are always full exits, otherwise check config for partial exit support
    let exit_reason = format!("{:?}", decision.reason);

    if partial_exit_enabled && !is_emergency_exit {
        // Partial exit enabled - get percentage from config
        let exit_percentage = with_config(|cfg| cfg.positions.partial_exit_default_pct);

        match positions::partial_close_position(&decision.mint, exit_percentage, &exit_reason.clone())
            .await
        {
            Ok(transaction_signature) => {
                log(
                    LogTag::Trader,
                    "SUCCESS",
                    &format!(
                        "✅ Partial sell executed: {} | {}% | TX: {} | Reason: {}",
                        decision.mint, exit_percentage, transaction_signature, exit_reason
                    ),
                );

                Ok(TradeResult::success(
                    decision.clone(),
                    transaction_signature,
                    decision.price_sol.unwrap_or(0.0),
                    0.0, // Exit size will be calculated by verification
                    decision.position_id.clone(),
                ))
            }
            Err(e) => {
                let error = format!("Partial sell execution failed: {}", e);
                log(LogTag::Trader, "ERROR", &error);
                Ok(TradeResult::failure(decision.clone(), error, 0))
            }
        }
    } else {
        // Full exit (either disabled or emergency exit)
        match positions::close_position_direct(&decision.mint, exit_reason.clone()).await {
            Ok(transaction_signature) => {
                log(
                    LogTag::Trader,
                    "SUCCESS",
                    &format!(
                        "✅ Full sell executed: {} | TX: {} | Reason: {}",
                        decision.mint, transaction_signature, exit_reason
                    ),
                );

                Ok(TradeResult::success(
                    decision.clone(),
                    transaction_signature,
                    decision.price_sol.unwrap_or(0.0),
                    0.0, // Exit size will be calculated by verification
                    decision.position_id.clone(),
                ))
            }
            Err(e) => {
                let error = format!("Full sell execution failed: {}", e);
                log(LogTag::Trader, "ERROR", &error);
                Ok(TradeResult::failure(decision.clone(), error, 0))
            }
        }
    }
}
