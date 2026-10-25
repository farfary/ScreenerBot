// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trade execution system.

mod buy;
mod sell;

pub use buy::{execute_buy, execute_buy_managed, execute_dca};
pub use sell::execute_sell;

use crate::actions::ActionFailure;
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::trader::actions::AutoCloseAction;
use crate::trader::types::{TradeAction, TradeDecision, TradeResult};

/// Initialize the execution system
pub async fn init_execution_system() -> crate::trader::Result<()> {
    logger::info(LogTag::Trader, "Initializing execution system...");
    logger::info(LogTag::Trader, "Execution system initialized");
    Ok(())
}

/// Execute a trade decision
pub async fn execute_trade(decision: &TradeDecision) -> crate::trader::Result<TradeResult> {
    match decision.action {
        TradeAction::Buy => buy::execute_buy(decision).await,
        TradeAction::Sell => sell::execute_sell(decision).await,
        TradeAction::DCA => buy::execute_dca(decision).await,
    }
}

/// Execute an automatic sell and record it in the Actions panel. The position
/// monitor and copy trading both close through here, so every automatic exit is
/// visible whichever service decided it.
pub async fn execute_tracked_exit(
    decision: &TradeDecision,
    symbol: Option<&str>,
) -> crate::trader::Result<TradeResult> {
    let action = AutoCloseAction::new(
        &decision.mint,
        symbol,
        decision.position_id.as_ref().and_then(|s| s.parse().ok()),
        &format!("{:?}", decision.reason),
    )
    .await
    .ok();
    if let Some(ref a) = action {
        a.complete_evaluation().await;
        a.start_quote().await;
    }
    let execution = execute_trade(decision).await;
    if let Some(ref a) = action {
        match &execution {
            Ok(result) if result.success => {
                a.complete_quote().await;
                a.start_swap().await;
                let signature = result.tx_signature.as_deref().unwrap_or("unknown");
                a.complete_swap(signature, result.executed_size_native)
                    .await;
                a.complete(result.tx_signature.as_deref()).await;
            }
            Ok(result) => {
                let error = result.error.clone().unwrap_or_default();
                a.fail(ActionFailure::with_details(
                    ids::ACTIONS_FAILURE_EXIT,
                    error.as_str(),
                ))
                .await;
            }
            Err(error) => {
                a.fail(ActionFailure::with_details(
                    ids::ACTIONS_FAILURE_EXIT,
                    error.to_string().as_str(),
                ))
                .await;
            }
        }
    }
    execution
}
