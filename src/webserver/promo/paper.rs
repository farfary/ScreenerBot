// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Promo generators for the paper book: the rounds the promo paper strategy took part in,
//! the paper run they belong to and its cash, read by the Stats tab's Paper book, the
//! strategies roster and the strategy readiness alike.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};

use crate::positions::db::StrategyRound;
use crate::positions::paper::{paper_cash, PaperAccount, PaperStrategyEvidence};
use crate::positions::{Book, Opening, Position, PositionManagement, PositionOrigin};

use super::data::{
    PROMO_CLOSED_TOKENS, PROMO_PAPER_ROUNDS, PROMO_PAPER_RUN_HOURS,
    PROMO_PAPER_STARTING_BALANCE_SOL, PROMO_PAPER_STRATEGY_ID, PROMO_TRADE_SIZE_SOL,
};

/// Network fee of each promo paper buy and sale, in raw units, as the live promo rounds
/// carry it.
const PROMO_PAPER_FEE_RAW: u64 = 5000;

/// Ids of the promo paper rounds start here, clear of the live promo positions.
const PROMO_PAPER_FIRST_ID: i64 = 1001;

/// When the promo paper run started, relative to `now`.
pub(super) fn run_started_at(now: DateTime<Utc>) -> DateTime<Utc> {
    now - Duration::hours(PROMO_PAPER_RUN_HOURS)
}

/// The rounds of the promo paper run at `now`, open then closed, in fixture order.
pub(super) fn promo_paper_rounds(now: DateTime<Utc>) -> (Vec<Position>, Vec<Position>) {
    let (open, closed): (Vec<Position>, Vec<Position>) = PROMO_PAPER_ROUNDS
        .iter()
        .zip(PROMO_PAPER_FIRST_ID..)
        .map(|(round, id)| paper_round(round, id, now))
        .partition(|round| round.exit_time.is_none());
    (open, closed)
}

/// The paper evidence of the promo run: every round the promo paper strategy opened,
/// with no failed exits.
pub(super) fn promo_paper_evidence(now: DateTime<Utc>) -> PaperStrategyEvidence {
    let (open, closed) = promo_paper_rounds(now);
    let mut rounds: Vec<StrategyRound> = open
        .into_iter()
        .chain(closed)
        .map(|position| StrategyRound {
            position,
            exit_strategy_id: None,
        })
        .collect();
    rounds.sort_by_key(|round| round.position.entry_time);
    PaperStrategyEvidence::new(Some(run_started_at(now)), rounds, HashMap::new())
}

/// The promo paper account: the run's starting balance with every buy, sale and fee of
/// its rounds, computed the way the live account is.
pub fn get_promo_paper_account() -> PaperAccount {
    let now = Utc::now();
    let (open, closed) = promo_paper_rounds(now);
    let rounds: Vec<Position> = open.into_iter().chain(closed).collect();
    PaperAccount {
        epoch: 1,
        starting_balance_native: PROMO_PAPER_STARTING_BALANCE_SOL,
        started_at: run_started_at(now),
        cash_native: paper_cash(PROMO_PAPER_STARTING_BALANCE_SOL, &rounds),
    }
}

/// One promo paper round: `(closed token index, entry price, exit or current price,
/// minutes since entry, (hold minutes, exit reason) when closed)`.
fn paper_round(
    &(token, entry, price, entered_minutes_ago, exit): &(usize, f64, f64, i64, Option<(i64, &str)>),
    id: i64,
    now: DateTime<Utc>,
) -> Position {
    let (symbol, name, mint, ..) = PROMO_CLOSED_TOKENS[token];
    let entry_time = now - Duration::minutes(entered_minutes_ago);
    let size = PROMO_TRADE_SIZE_SOL;
    let mut position = Position::opening(Opening {
        mint: mint.to_owned(),
        symbol: symbol.to_owned(),
        name: name.to_owned(),
        entry_price: entry,
        entry_time,
        size_native: size,
        entry_signature: None,
        origin: PositionOrigin::Auto {
            strategy_id: Some(PROMO_PAPER_STRATEGY_ID.to_owned()),
        },
        management: PositionManagement::AutoTrader,
        entry_liquidity: None,
        book: Book::Paper,
    });
    position.id = Some(id);
    position.total_size_native = size;
    position.transaction_entry_verified = true;
    position.entry_fee_raw = Some(PROMO_PAPER_FEE_RAW);
    let pnl = (price - entry) / entry * size;
    let pnl_percent = (price - entry) / entry * 100.0;
    match exit {
        Some((hold_minutes, reason)) => {
            position.transaction_exit_verified = true;
            position.exit_fee_raw = Some(PROMO_PAPER_FEE_RAW);
            position.exit_price = Some(price);
            position.exit_time = Some(entry_time + Duration::minutes(hold_minutes));
            position.native_received = Some(size + pnl);
            position.pnl = Some(pnl);
            position.pnl_percent = Some(pnl_percent);
            position.closed_reason = Some(reason.to_owned());
        }
        None => {
            position.current_price = Some(price);
            position.current_price_updated = Some(now);
            position.unrealized_pnl = Some(pnl);
            position.unrealized_pnl_percent = Some(pnl_percent);
        }
    }
    position
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_rounds_sit_inside_the_run_and_belong_to_the_paper_strategy() {
        let now = Utc::now();
        let (open, closed) = promo_paper_rounds(now);
        assert!(!open.is_empty());
        assert!(closed.len() >= 2);
        let started = run_started_at(now);
        for round in open.iter().chain(&closed) {
            assert_eq!(round.book, Book::Paper);
            assert!(round.entry_time > started, "{}", round.symbol);
            assert_eq!(
                round.origin.entry_strategy_id(),
                Some(PROMO_PAPER_STRATEGY_ID)
            );
            if let Some(exit_time) = round.exit_time {
                assert!(exit_time <= now, "{}", round.symbol);
            }
        }
    }

    #[test]
    fn paper_cash_is_the_starting_balance_after_every_round() {
        let account = get_promo_paper_account();
        let (open, closed) = promo_paper_rounds(Utc::now());
        let realized: f64 = closed.iter().filter_map(|round| round.pnl).sum();
        let fees = crate::chains::adapter().raw_to_native(PROMO_PAPER_FEE_RAW);
        let fee_count = (open.len() + 2 * closed.len()) as f64;
        let expected = PROMO_PAPER_STARTING_BALANCE_SOL + realized
            - open.len() as f64 * PROMO_TRADE_SIZE_SOL
            - fee_count * fees;
        assert!((account.cash_native - expected).abs() < 1e-9);
        assert!(account.cash_native > 0.0);
    }
}
