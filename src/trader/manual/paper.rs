// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Manual close of a paper round: a full manual exit decided for the paper book, filled
//! by the paper executor and never by a live one.

use chrono::Utc;

use crate::positions::{Book, Position};
use crate::trader::error::Error;
use crate::trader::executors;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult};

use super::detach::detached;

/// Closes the open paper round `position` in full at a paper fill.
///
/// Runs detached from the caller like every manual trade, so a dropped request never
/// leaves a paper fill half booked.
pub async fn manual_paper_close(position: Position) -> Result<TradeResult, Error> {
    detached(async move {
        let decision = paper_close_decision(&position);
        executors::execute_trade(
            &decision,
            executors::ExecutionContext::for_decision(&decision),
        )
        .await
    })
    .await
}

/// The decision that closes `position` in full as a manual exit on the paper book.
fn paper_close_decision(position: &Position) -> TradeDecision {
    TradeDecision {
        position_id: position.id.map(|id| id.to_string()),
        mint: position.mint.clone(),
        action: TradeAction::Sell,
        reason: TradeReason::ManualExit,
        strategy_id: None,
        book: Book::Paper,
        timestamp: Utc::now(),
        priority: TradePriority::High,
        price_native: position.current_price,
        size_native: None,
        exit_percentage: Some(100.0),
        slippage_pct: None,
        take_profit_level: None,
        paper_leg_key: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::positions::{Opening, PositionManagement, PositionOrigin};

    #[test]
    fn a_paper_close_is_a_full_manual_exit_on_the_paper_book() {
        let mut position = Position::opening(Opening {
            mint: "PaperCloseMint11111111111111111111111111111".to_owned(),
            symbol: "PAPER".to_owned(),
            name: "Paper".to_owned(),
            entry_price: 0.001,
            entry_time: Utc::now(),
            size_native: 0.1,
            entry_signature: None,
            origin: PositionOrigin::Auto {
                strategy_id: Some("entry".to_owned()),
            },
            management: PositionManagement::AutoTrader,
            entry_liquidity: None,
            book: Book::Paper,
        });
        position.id = Some(7);
        position.current_price = Some(0.002);

        let decision = paper_close_decision(&position);
        assert_eq!(decision.book, Book::Paper);
        assert_eq!(decision.action, TradeAction::Sell);
        assert_eq!(decision.reason, TradeReason::ManualExit);
        assert_eq!(decision.position_id.as_deref(), Some("7"));
        assert_eq!(decision.exit_percentage, Some(100.0));
        assert_eq!(decision.price_native, Some(0.002));
    }
}
