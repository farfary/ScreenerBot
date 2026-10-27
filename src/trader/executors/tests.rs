// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The live executors refuse every decision that is not for the live book before they
//! touch a wallet, a quote or a transaction, and the execution context a decision
//! implies.

use chrono::Utc;

use super::*;
use crate::trader::types::TradePriority;
use crate::trader::Error;

fn decision(action: TradeAction, reason: TradeReason, book: Book) -> TradeDecision {
    TradeDecision {
        position_id: Some("7".to_owned()),
        mint: "PaperMint1111111111111111111111111111111111".to_owned(),
        action,
        reason,
        strategy_id: Some("strategy".to_owned()),
        book,
        timestamp: Utc::now(),
        priority: TradePriority::Normal,
        price_native: Some(0.001),
        size_native: Some(0.1),
        exit_percentage: None,
        slippage_pct: None,
        take_profit_level: None,
    }
}

fn refused_on_live_path(result: crate::trader::Result<TradeResult>, mint: &str) {
    match result {
        Err(Error::PaperDecisionOnLivePath { mint: refused }) => assert_eq!(refused, mint),
        other => panic!("a paper decision reached a live executor: {other:?}"),
    }
}

/// Each live executor refuses a paper decision as its first step, so even a caller that
/// bypassed the dispatcher could never sign, send or swap for a paper round.
#[tokio::test]
async fn paper_decision_on_live_path_is_refused() {
    let buy = decision(TradeAction::Buy, TradeReason::StrategySignal, Book::Paper);
    refused_on_live_path(
        buy::execute_buy(&buy, ExecutionContext::for_decision(&buy)).await,
        &buy.mint,
    );

    let add = decision(TradeAction::DCA, TradeReason::DCAScheduled, Book::Paper);
    refused_on_live_path(buy::execute_dca(&add).await, &add.mint);

    let sell = decision(TradeAction::Sell, TradeReason::TakeProfit, Book::Paper);
    refused_on_live_path(sell::execute_sell(&sell).await, &sell.mint);
}

#[test]
fn only_live_decisions_pass_the_live_guard() {
    for book in Book::ALL {
        let decision = decision(TradeAction::Buy, TradeReason::StrategySignal, book);
        assert_eq!(
            require_live(&decision).is_ok(),
            book == Book::Live,
            "{book:?}"
        );
    }
}

#[test]
fn a_manual_or_forced_buy_belongs_to_the_user() {
    for reason in [TradeReason::ManualEntry, TradeReason::ForceBuy] {
        let context =
            ExecutionContext::for_decision(&decision(TradeAction::Buy, reason, Book::Live));
        assert_eq!(context.origin, PositionOrigin::Manual);
        assert_eq!(context.management, PositionManagement::UserOnly);
    }
}

#[test]
fn a_strategy_buy_belongs_to_its_strategy() {
    let context = ExecutionContext::for_decision(&decision(
        TradeAction::Buy,
        TradeReason::StrategySignal,
        Book::Paper,
    ));
    assert_eq!(
        context.origin,
        PositionOrigin::Auto {
            strategy_id: Some("strategy".to_owned())
        }
    );
    assert_eq!(context.management, PositionManagement::AutoTrader);
}
