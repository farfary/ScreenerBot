// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trade execution system.

mod buy;
mod sell;

pub(in crate::trader) use buy::{entry_size_native, entry_size_of};
pub(in crate::trader) use sell::{resolve_exit_size, ExitSize};

use crate::actions::ActionFailure;
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::positions::{Book, PositionManagement, PositionOrigin};
use crate::trader::actions::AutoCloseAction;
use crate::trader::types::{TradeAction, TradeDecision, TradeReason, TradeResult};

/// Who a trade's position belongs to and who may manage it, persisted with the round it
/// opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionContext {
    pub origin: PositionOrigin,
    pub management: PositionManagement,
}

impl ExecutionContext {
    /// The context a decision implies on its own: a manual or forced buy belongs to the
    /// user and is protected from the auto-trader; every other trade is the strategy's.
    pub fn for_decision(decision: &TradeDecision) -> Self {
        if matches!(
            decision.reason,
            TradeReason::ManualEntry | TradeReason::ForceBuy
        ) {
            Self {
                origin: PositionOrigin::Manual,
                management: PositionManagement::UserOnly,
            }
        } else {
            Self {
                origin: PositionOrigin::Auto {
                    strategy_id: decision.strategy_id.clone(),
                },
                management: PositionManagement::AutoTrader,
            }
        }
    }
}

/// Refuses a decision that is not for the live book before a live executor touches a
/// wallet, a quote or a transaction.
fn require_live(decision: &TradeDecision) -> crate::trader::Result<()> {
    if decision.book == Book::Live {
        Ok(())
    } else {
        Err(crate::trader::Error::PaperDecisionOnLivePath {
            mint: decision.mint.clone(),
        })
    }
}

/// Initialize the execution system
pub async fn init_execution_system() -> crate::trader::Result<()> {
    logger::info(LogTag::Trader, "Initializing execution system...");
    logger::info(LogTag::Trader, "Execution system initialized");
    Ok(())
}

/// Executes a trade decision on the book it names: a paper decision is filled by the
/// paper executor and never reaches a live executor; a live decision runs the swap.
/// `context` is the provenance a buy opens its round with.
pub async fn execute_trade(
    decision: &TradeDecision,
    context: ExecutionContext,
) -> crate::trader::Result<TradeResult> {
    match decision.book {
        Book::Paper => crate::trader::paper::executor::execute(decision, &context).await,
        Book::Live => match decision.action {
            TradeAction::Buy => buy::execute_buy(decision, context).await,
            TradeAction::Sell => sell::execute_sell(decision).await,
            TradeAction::DCA => buy::execute_dca(decision).await,
        },
    }
}

/// Execute an automatic sell and record it in the Actions panel. The position
/// monitor and copy trading both close through here, so every automatic live exit is
/// visible whichever service decided it; a paper exit sends nothing and records no
/// action. `origin` is the kind of the position sold (`PositionOrigin::kind`).
pub async fn execute_tracked_exit(
    decision: &TradeDecision,
    symbol: Option<&str>,
    origin: &str,
) -> crate::trader::Result<TradeResult> {
    let action = match decision.book {
        Book::Live => AutoCloseAction::new(
            &decision.mint,
            symbol,
            decision.position_id.as_ref().and_then(|s| s.parse().ok()),
            &format!("{:?}", decision.reason),
            decision.exit_percentage,
            origin,
        )
        .await
        .ok(),
        Book::Paper => None,
    };
    if let Some(ref a) = action {
        a.complete_evaluation().await;
        a.start_quote().await;
    }
    let execution = execute_trade(decision, ExecutionContext::for_decision(decision)).await;
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

#[cfg(test)]
mod tests;
