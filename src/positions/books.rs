// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The one dispatch point over the live and the paper book, so no caller branches on
//! the book itself, and the one reader of every mint a book holds.

use std::collections::{HashMap, HashSet};

use super::types::{Book, Position};

/// The open rounds of `book`. The paper book is empty until it is loaded.
pub async fn open_positions(book: Book) -> Vec<Position> {
    match book {
        Book::Live => super::state::get_open_positions().await,
        Book::Paper => match super::paper::book() {
            Some(paper) => paper.open_positions().await,
            None => Vec::new(),
        },
    }
}

/// The closed rounds of the live book each strategy took part in, by strategy id: verified
/// and not archived, under the entry strategy that opened the round (origin `auto` with a
/// strategy, the selection the live performance queries count) and under the exit strategy
/// whose decision closed it, as [`super::db::StrategyRound::involves`] selects a paper
/// round. One pass over the positions in memory; no store read. The paper book keeps its
/// closed rounds in its store only (`PaperBook::strategy_evidence`).
pub async fn live_auto_rounds() -> HashMap<String, Vec<Position>> {
    let positions = super::state::POSITIONS.read().await;
    strategy_rounds(positions.iter())
}

/// The closed, unarchived rounds of `positions` grouped by every strategy that took part
/// in each: its entry strategy and its closing exit strategy, once each.
pub(crate) fn strategy_rounds<'a>(
    positions: impl Iterator<Item = &'a Position>,
) -> HashMap<String, Vec<Position>> {
    let mut rounds: HashMap<String, Vec<Position>> = HashMap::new();
    for position in
        positions.filter(|position| !position.archived && position.transaction_exit_verified)
    {
        let entry = position.origin.entry_strategy_id();
        let exit = position
            .exit_strategy_id
            .as_deref()
            .filter(|exit| Some(*exit) != entry);
        for strategy_id in entry.into_iter().chain(exit) {
            rounds
                .entry(strategy_id.to_owned())
                .or_default()
                .push(position.clone());
        }
    }
    rounds
}

/// The mints of the open rounds of `book`.
pub async fn open_mints(book: Book) -> Vec<String> {
    open_positions(book)
        .await
        .into_iter()
        .map(|position| position.mint)
        .collect()
}

/// True while `mint` has an open round in `book`; on the live book an open still
/// pending counts as well.
pub async fn holds_open_round(book: Book, mint: &str) -> bool {
    match book {
        Book::Live => super::state::holds_open_round(mint).await,
        Book::Paper => match super::paper::book() {
            Some(paper) => paper.holds_open_round(mint).await,
            None => false,
        },
    }
}

/// The current state of the open round `position` of its own book, with the marks booked
/// since it was read; `None` once the round is no longer open.
pub async fn fresh(position: &Position) -> Option<Position> {
    match position.book {
        Book::Live => super::state::get_position_by_mint(&position.mint).await,
        Book::Paper => match (super::paper::book(), position.id) {
            (Some(paper), Some(position_id)) => paper.position(position_id).await,
            _ => None,
        },
    }
}

/// The open round of `book` in `mint`, if it holds one.
pub async fn open_round(book: Book, mint: &str) -> Option<Position> {
    match book {
        Book::Live => super::state::get_position_by_mint(mint).await,
        Book::Paper => match super::paper::book() {
            Some(paper) => paper.open_round(mint).await,
            None => None,
        },
    }
}

/// True while the store of `book` holds restored contents not yet reconciled with the
/// wallet history. A paper book holds no wallet state to reconcile.
pub fn awaiting_reconciliation(book: Book) -> bool {
    match book {
        Book::Live => super::awaiting_reconciliation(),
        Book::Paper => false,
    }
}

/// True while a partial exit of `mint` in `book` is sent and not yet settled. A paper
/// partial books at once and is never pending.
pub async fn partial_exit_pending(book: Book, mint: &str) -> bool {
    match book {
        Book::Live => super::state::is_partial_exit_pending(mint).await,
        Book::Paper => false,
    }
}

/// True while an add of `mint` in `book` is sent and not yet settled. A paper add books
/// at once and is never pending.
pub async fn dca_pending(book: Book, mint: &str) -> bool {
    match book {
        Book::Live => super::state::is_dca_pending(mint).await,
        Book::Paper => false,
    }
}

/// The latest round of `mint` in `book`, open or closed.
pub async fn latest_by_mint(book: Book, mint: &str) -> super::Result<Option<Position>> {
    match book {
        Book::Live => super::db::get_latest_position_by_mint(mint).await,
        Book::Paper => match super::paper::book() {
            Some(paper) => paper.latest_by_mint(mint).await,
            None => Ok(None),
        },
    }
}

/// Every mint a book holds: the live and the paper open rounds, copy-trading paper
/// rounds among them, once each. These are the tokens whose prices, pools and charts
/// the open positions are marked and exited on.
pub async fn held_mints() -> Vec<String> {
    merge_mints([open_mints(Book::Live).await, open_mints(Book::Paper).await])
}

/// The mints of `sources` in order, each once.
fn merge_mints<const N: usize>(sources: [Vec<String>; N]) -> Vec<String> {
    let mut seen = HashSet::new();
    sources
        .into_iter()
        .flatten()
        .filter(|mint| seen.insert(mint.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::positions::types::{Opening, PositionManagement, PositionOrigin};

    fn round(entry: Option<&str>, exit: Option<&str>, closed: bool) -> Position {
        let mut position = Position::opening(Opening {
            mint: "Mint".to_owned(),
            symbol: "SYM".to_owned(),
            name: "Token".to_owned(),
            entry_price: 1.0,
            entry_time: chrono::Utc::now(),
            size_native: 1.0,
            entry_signature: None,
            origin: PositionOrigin::Auto {
                strategy_id: entry.map(str::to_owned),
            },
            management: PositionManagement::AutoTrader,
            entry_liquidity: None,
            book: Book::Live,
        });
        position.transaction_exit_verified = closed;
        position.exit_strategy_id = exit.map(str::to_owned);
        position
    }

    /// A closed live round counts for the entry strategy that opened it and for the exit
    /// strategy that closed it; a round closed by a rule counts for its entry strategy
    /// only, and a round still open counts for neither.
    #[test]
    fn live_rounds_group_under_their_entry_and_closing_exit_strategies() {
        let positions = [
            round(Some("entry"), Some("exit"), true),
            round(Some("entry"), None, true),
            round(None, Some("exit"), true),
            round(Some("entry"), Some("exit"), false),
            round(Some("same"), Some("same"), true),
        ];
        let rounds = strategy_rounds(positions.iter());
        let count = |id: &str| rounds.get(id).map_or(0, Vec::len);
        assert_eq!(count("entry"), 2);
        assert_eq!(count("exit"), 2);
        assert_eq!(
            count("same"),
            1,
            "a strategy that opened and closed a round counts it once"
        );
        assert_eq!(rounds.len(), 3);
    }

    #[test]
    fn held_mints_cover_live_and_paper_books_once_each() {
        let live = vec!["A".to_owned(), "B".to_owned()];
        // A copy task's paper round is a round of the paper book: its mint arrives
        // once, through the paper book, whatever else holds it.
        let paper = vec!["B".to_owned(), "C".to_owned(), "D".to_owned()];
        assert_eq!(merge_mints([live, paper]), vec!["A", "B", "C", "D"]);
        assert!(merge_mints([Vec::new(), Vec::new()]).is_empty());
    }
}
