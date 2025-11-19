//! Sell operation execution

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions;
use crate::trader::types::{TradeDecision, TradeReason, TradeResult};

/// Execute a sell trade
pub async fn execute_sell(decision: &TradeDecision) -> Result<TradeResult, String> {
    // Check connectivity before executing trade - critical operation
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc"]).await {
        let error = format!("Cannot execute sell - Unhealthy endpoints: {}", unhealthy);
        logger::error(LogTag::Trader, &error);
        return Ok(TradeResult::failure(decision.clone(), error, 0));
    }

    logger::info(
        LogTag::Trader,
        &format!(
            "Executing sell for position {} token {} (reason: {:?})",
            decision.position_id.as_deref().unwrap_or("unknown"),
            decision.mint,
            decision.reason
        ),
    );

    // Extract exit percentage from decision.size_sol (default to 100% if not specified)
    let exit_percentage = decision.size_sol.unwrap_or(100.0);

    // Determine exit type based on configuration and reason
    let partial_exit_enabled = with_config(|cfg| cfg.positions.partial_exit_enabled);
    let is_emergency_exit = matches!(
        decision.reason,
        TradeReason::StopLoss | TradeReason::Blacklisted | TradeReason::ForceSell
    );

    // Emergency exits are always full exits, otherwise check config and percentage
    let exit_reason = format!("{:?}", decision.reason);

    if partial_exit_enabled && !is_emergency_exit && exit_percentage < 100.0 {
        // Partial exit
        match positions::partial_close_position(
            &decision.mint,
            exit_percentage,
            &exit_reason.clone(),
        )
        .await
        {
            Ok(transaction_signature) => {
                logger::info(
                    LogTag::Trader,
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
                logger::error(LogTag::Trader, &error);
                Ok(TradeResult::failure(decision.clone(), error, 0))
            }
        }
    } else {
        // Full exit (either disabled, emergency exit, or 100%)
        match positions::close_position_direct(&decision.mint, exit_reason.clone()).await {
            Ok(transaction_signature) => {
                logger::info(
                    LogTag::Trader,
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
                logger::error(LogTag::Trader, &error);
                Ok(TradeResult::failure(decision.clone(), error, 0))
            }
        }
    }
}
