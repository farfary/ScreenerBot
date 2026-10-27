// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of the paper book on an isolated paper store: its bookings, their idempotency,
//! the capacity, cash and open cooldown bounds under concurrent buys, a run reset racing
//! a buy, the cash of the paper account and the failed-exit streak kept in the store.

use chrono::{Duration, Utc};
use futures::future::join_all;
use rusqlite::params;

use super::*;
use crate::chains::{ChainId, RawAmount};
use crate::positions::booking::{CloseFill, DcaFill, EntryFill, PartialExitFill};
use crate::positions::types::{Opening, Position, PositionManagement, PositionOrigin};
use crate::positions::Error;
use crate::trader::paper::{PaperFill, PaperModel, PaperSellFill};

const WALLET: &str = "PaperWa11et1111111111111111111111111111111";
const DECIMALS: u8 = 6;

async fn paper_book() -> (PaperBook, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let db = PositionsDatabase::open_at(
        Book::Paper,
        directory.path().join("paper_positions.db"),
        ChainId::Solana,
    )
    .await
    .unwrap();
    (PaperBook::new(db, Vec::new()), directory)
}

fn opening(mint: &str) -> Opening {
    Opening {
        mint: mint.to_owned(),
        symbol: "PAPER".to_owned(),
        name: "Paper".to_owned(),
        entry_price: 0.001,
        entry_time: Utc::now(),
        size_native: 0.1,
        entry_signature: Some("must-not-be-kept".to_owned()),
        origin: PositionOrigin::Auto {
            strategy_id: Some("entry-strategy".to_owned()),
        },
        management: PositionManagement::AutoTrader,
        entry_liquidity: None,
        book: Book::Live,
    }
}

fn entry_fill() -> EntryFill {
    EntryFill {
        effective_entry_price: 0.001,
        token_amount: RawAmount::new(100_000_000),
        fee_raw: 5_000,
        native_size: 0.1,
    }
}

fn buy_leg() -> PaperLeg {
    let fill = PaperFill {
        input_sol: 0.1,
        market_price_sol: 0.001,
        priced_from_pool: true,
        fill_price_sol: 0.001,
        token_amount: 100.0,
        referral_fee_sol: 0.0,
        network_fee_sol: 0.000005,
        priority_fee_sol: 0.0,
        total_cost_sol: 0.100005,
        model: PaperModel::Quote,
        quote: None,
    };
    PaperLeg::buy(&fill, Some("entry-strategy".to_owned()), Utc::now()).unwrap()
}

fn sell_leg(tokens: f64) -> PaperLeg {
    let fill = PaperSellFill {
        token_amount: tokens,
        market_price_sol: 0.002,
        priced_from_pool: true,
        fill_price_sol: 0.002,
        gross_sol: tokens * 0.002,
        referral_fee_sol: 0.0,
        network_fee_sol: 0.000005,
        priority_fee_sol: 0.0,
        net_proceeds_sol: tokens * 0.002,
        model: PaperModel::Quote,
        quote: None,
    };
    PaperLeg::sell(&fill, Some("exit-strategy".to_owned()), Utc::now()).unwrap()
}

/// Open terms within the current run of `WALLET`, starting it with ten SOL when none
/// has started, with no open cooldown.
async fn terms(book: &PaperBook, max_open: usize) -> PaperOpenTerms {
    let epoch = book.account(WALLET, 10.0).await.unwrap().epoch;
    PaperOpenTerms {
        max_open,
        open_cooldown_secs: 0,
        epoch,
    }
}

fn count(book: &PaperBook, sql: &str) -> i64 {
    book.db
        .get_connection()
        .unwrap()
        .query_row(sql, [], |row| row.get(0))
        .unwrap()
}

#[tokio::test]
async fn a_paper_round_is_booked_leg_by_leg_with_its_evidence() {
    let (book, _directory) = paper_book().await;
    let opened = book
        .open(
            WALLET,
            opening("MintA"),
            &entry_fill(),
            &buy_leg(),
            &terms(&book, 5).await,
        )
        .await
        .unwrap();
    let id = opened.id.unwrap();
    assert_eq!(opened.book, Book::Paper);
    assert_eq!(opened.entry_transaction_signature, None);
    assert!(opened.transaction_entry_verified);
    assert_eq!(
        opened.remaining_token_amount,
        Some(RawAmount::new(100_000_000))
    );

    let dca = DcaFill {
        tokens_bought: RawAmount::new(50_000_000),
        native_spent: 0.025,
        fee_raw: 5_000,
        dca_time: Utc::now(),
    };
    let PaperBooked::Booked(after_dca) = book
        .dca(WALLET, id, &dca, 0.0005, DECIMALS, &buy_leg())
        .await
        .unwrap()
    else {
        panic!("the DCA was not booked");
    };
    assert_eq!(after_dca.dca_count, 1);
    assert!(after_dca.average_entry_price < 0.001);

    let partial = PartialExitFill {
        exit_amount: RawAmount::new(75_000_000),
        native_received: 0.15,
        effective_exit_price: 0.002,
        unrealized_pnl: None,
        fee_raw: 5_000,
    };
    let PaperBooked::Booked(after_partial) = book
        .partial(WALLET, id, &partial, 50.0, Some(1), &sell_leg(75.0))
        .await
        .unwrap()
    else {
        panic!("the partial exit was not booked");
    };
    assert_eq!(after_partial.partial_exit_count, 1);
    assert_eq!(after_partial.take_profit_level, 1);
    assert!(after_partial.exit_time.is_none());

    let close = CloseFill {
        effective_exit_price: 0.002,
        native_received: 0.15,
        fee_raw: 5_000,
        exit_time: Utc::now(),
    };
    let PaperBooked::Booked(closed) = book
        .close(WALLET, id, &close, 0.002, "take_profit", &sell_leg(75.0))
        .await
        .unwrap()
    else {
        panic!("the close was not booked");
    };
    assert!(closed.exit_time.is_some());
    assert!(closed.transaction_exit_verified);
    assert_eq!(closed.exit_transaction_signature, None);
    assert_eq!(closed.closed_reason.as_deref(), Some("take_profit"));
    assert_eq!(closed.book, Book::Paper);
    assert!(book.open_positions().await.is_empty());

    let connection = book.db.get_connection().unwrap();
    let mut legs = connection
        .prepare(
            "SELECT leg_kind, leg_ref, strategy_id FROM paper_legs WHERE position_id = ?1 ORDER BY id",
        )
        .unwrap();
    let legs: Vec<(String, String, Option<String>)> = legs
        .query_map(params![id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .map(std::result::Result::unwrap)
        .collect();
    assert_eq!(
        legs.iter().map(|leg| leg.0.as_str()).collect::<Vec<_>>(),
        ["entry", "dca", "partial", "exit"]
    );
    assert!(legs.iter().all(|leg| leg.1.starts_with(PAPER_LEG_PREFIX)));
    assert_eq!(legs[3].2.as_deref(), Some("exit-strategy"));
    let booked_refs: i64 = connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM position_entries WHERE transaction_signature IN (SELECT leg_ref FROM paper_legs))
                  + (SELECT COUNT(*) FROM position_exits WHERE transaction_signature IN (SELECT leg_ref FROM paper_legs))",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(booked_refs, 4, "every record is booked under its leg");
    let states: Vec<String> = connection
        .prepare("SELECT state FROM position_states WHERE position_id = ?1 ORDER BY id")
        .unwrap()
        .query_map(params![id], |row| row.get(0))
        .unwrap()
        .map(std::result::Result::unwrap)
        .collect();
    assert_eq!(states, ["Open", "Closed"]);

    let reloaded = book
        .db
        .positions_entered_since(WALLET, Utc::now() - Duration::days(1))
        .unwrap();
    assert_eq!(reloaded.len(), 1);
    assert_eq!(
        reloaded[0].book,
        Book::Paper,
        "the store sets the book it holds"
    );
}

#[tokio::test]
async fn a_repeated_or_late_booking_writes_nothing() {
    let (book, _directory) = paper_book().await;
    let id = book
        .open(
            WALLET,
            opening("MintB"),
            &entry_fill(),
            &buy_leg(),
            &terms(&book, 5).await,
        )
        .await
        .unwrap()
        .id
        .unwrap();
    let dca = DcaFill {
        tokens_bought: RawAmount::new(10_000_000),
        native_spent: 0.01,
        fee_raw: 5_000,
        dca_time: Utc::now(),
    };
    let leg = buy_leg();
    assert!(matches!(
        book.dca(WALLET, id, &dca, 0.001, DECIMALS, &leg)
            .await
            .unwrap(),
        PaperBooked::Booked(_)
    ));
    assert!(matches!(
        book.dca(WALLET, id, &dca, 0.001, DECIMALS, &leg)
            .await
            .unwrap(),
        PaperBooked::AlreadyBooked
    ));
    assert_eq!(book.position(id).await.unwrap().dca_count, 1);

    let close = CloseFill {
        effective_exit_price: 0.001,
        native_received: 0.1,
        fee_raw: 5_000,
        exit_time: Utc::now(),
    };
    assert!(matches!(
        book.close(WALLET, id, &close, 0.001, "stop_loss", &sell_leg(110.0))
            .await
            .unwrap(),
        PaperBooked::Booked(_)
    ));
    let legs = count(&book, "SELECT COUNT(*) FROM paper_legs");
    assert!(matches!(
        book.close(WALLET, id, &close, 0.001, "stop_loss", &sell_leg(110.0))
            .await
            .unwrap(),
        PaperBooked::AlreadyClosed
    ));
    assert!(matches!(
        book.dca(WALLET, id, &dca, 0.001, DECIMALS, &buy_leg())
            .await
            .unwrap(),
        PaperBooked::AlreadyClosed
    ));
    assert_eq!(count(&book, "SELECT COUNT(*) FROM paper_legs"), legs);
    assert_eq!(count(&book, "SELECT COUNT(*) FROM position_exits"), 1);
}

#[tokio::test]
async fn concurrent_opens_never_exceed_the_capacity() {
    let (book, _directory) = paper_book().await;
    let mints: Vec<String> = (0..8).map(|index| format!("Mint{index}")).collect();
    let legs: Vec<PaperLeg> = mints.iter().map(|_| buy_leg()).collect();
    let fill = entry_fill();
    let terms = terms(&book, 3).await;
    let results = join_all(
        mints
            .iter()
            .zip(&legs)
            .map(|(mint, leg)| book.open(WALLET, opening(mint), &fill, leg, &terms)),
    )
    .await;
    let opened = results.iter().filter(|result| result.is_ok()).count();
    assert_eq!(opened, 3);
    assert!(results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .all(|error| matches!(error, Error::SlotUnavailable { .. })));
    assert_eq!(book.open_positions().await.len(), 3);
    assert_eq!(count(&book, "SELECT COUNT(*) FROM positions"), 3);
    assert_eq!(count(&book, "SELECT COUNT(*) FROM paper_legs"), 3);
}

#[tokio::test]
async fn a_second_round_of_a_held_mint_is_refused() {
    let (book, _directory) = paper_book().await;
    book.open(
        WALLET,
        opening("MintC"),
        &entry_fill(),
        &buy_leg(),
        &terms(&book, 5).await,
    )
    .await
    .unwrap();
    let error = book
        .open(
            WALLET,
            opening("MintC"),
            &entry_fill(),
            &buy_leg(),
            &terms(&book, 5).await,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, Error::AlreadyOpen { .. }), "{error}");
    assert_eq!(count(&book, "SELECT COUNT(*) FROM paper_legs"), 1);
}

#[tokio::test]
async fn a_take_profit_step_is_booked_once_and_only_in_order() {
    let (book, _directory) = paper_book().await;
    let id = book
        .open(
            WALLET,
            opening("MintS"),
            &entry_fill(),
            &buy_leg(),
            &terms(&book, 5).await,
        )
        .await
        .unwrap()
        .id
        .unwrap();
    let partial = PartialExitFill {
        exit_amount: RawAmount::new(25_000_000),
        native_received: 0.05,
        effective_exit_price: 0.002,
        unrealized_pnl: None,
        fee_raw: 5_000,
    };
    assert!(matches!(
        book.partial(WALLET, id, &partial, 25.0, Some(2), &sell_leg(25.0))
            .await
            .unwrap(),
        PaperBooked::StepTaken
    ));
    assert!(matches!(
        book.partial(WALLET, id, &partial, 25.0, Some(1), &sell_leg(25.0))
            .await
            .unwrap(),
        PaperBooked::Booked(_)
    ));
    assert!(matches!(
        book.partial(WALLET, id, &partial, 25.0, Some(1), &sell_leg(25.0))
            .await
            .unwrap(),
        PaperBooked::StepTaken
    ));
    let held = book.position(id).await.unwrap();
    assert_eq!(held.take_profit_level, 1);
    assert_eq!(held.partial_exit_count, 1);
    assert_eq!(
        held.remaining_token_amount,
        Some(RawAmount::new(75_000_000))
    );
}

#[tokio::test]
async fn a_reset_settles_open_rounds_at_their_mark_before_the_next_run() {
    let (book, _directory) = paper_book().await;
    let first = book.account(WALLET, 1.0).await.unwrap();
    let mut entered = opening("MintR");
    entered.entry_time = first.started_at + Duration::seconds(1);
    let id = book
        .open(
            WALLET,
            entered,
            &entry_fill(),
            &buy_leg(),
            &terms(&book, 5).await,
        )
        .await
        .unwrap()
        .id
        .unwrap();
    let later = first.started_at + Duration::seconds(2);

    let refused = book
        .reset_run(WALLET, 2.0, "PaperReset", |_| None, later)
        .await;
    assert!(matches!(refused, Err(Error::PaperFillUnbookable { .. })));
    assert_eq!(book.open_positions().await.len(), 1);
    assert_eq!(book.account(WALLET, 2.0).await.unwrap().epoch, 1);

    let next = book
        .reset_run(WALLET, 2.0, "PaperReset", |_| Some(DECIMALS), later)
        .await
        .unwrap();
    assert_eq!(next.epoch, 2);
    assert_eq!(next.cash_native, 2.0);
    assert!(book.open_positions().await.is_empty());
    let settled = book.stored(id).await.unwrap().unwrap();
    assert_eq!(settled.closed_reason.as_deref(), Some("PaperReset"));
    assert_eq!(settled.exit_price, Some(0.001));
    assert!((settled.native_received.unwrap_or_default() - 0.1).abs() < 1e-12);
}

#[tokio::test]
async fn the_account_cash_follows_the_rounds_of_its_epoch() {
    let (book, _directory) = paper_book().await;
    let first = book.account(WALLET, 1.0).await.unwrap();
    assert_eq!(first.epoch, 1);
    assert_eq!(first.cash_native, 1.0);

    let mut entered = opening("MintD");
    entered.entry_time = first.started_at + Duration::seconds(1);
    book.open(
        WALLET,
        entered,
        &entry_fill(),
        &buy_leg(),
        &terms(&book, 5).await,
    )
    .await
    .unwrap();
    let fee = crate::positions::pnl::calculate_position_total_fees(&book.open_positions().await[0]);
    let after_open = book.account(WALLET, 5.0).await.unwrap();
    assert_eq!(after_open.epoch, 1, "an existing epoch keeps its balance");
    assert!((after_open.cash_native - (1.0 - 0.1 - fee)).abs() < 1e-12);

    let next = book
        .start_epoch(WALLET, 2.0, first.started_at + Duration::seconds(2))
        .await
        .unwrap();
    assert_eq!(next.epoch, 2);
    assert_eq!(next.cash_native, 2.0);
}

#[tokio::test]
async fn a_strategy_counts_the_rounds_it_opened_or_closed_in_the_current_epoch() {
    let (book, _directory) = paper_book().await;
    assert!(book
        .strategy_rounds(WALLET, "entry-strategy")
        .await
        .unwrap()
        .is_empty());
    let epoch = book.account(WALLET, 1.0).await.unwrap();
    let at = |seconds| epoch.started_at + Duration::seconds(seconds);

    let mut before = opening("MintBefore");
    before.entry_time = epoch.started_at - Duration::seconds(5);
    book.open(
        WALLET,
        before,
        &entry_fill(),
        &buy_leg(),
        &terms(&book, 5).await,
    )
    .await
    .unwrap();
    let mut own = opening("MintOwn");
    own.entry_time = at(1);
    book.open(
        WALLET,
        own,
        &entry_fill(),
        &buy_leg(),
        &terms(&book, 5).await,
    )
    .await
    .unwrap();
    let mut other = opening("MintOther");
    other.entry_time = at(2);
    other.origin = PositionOrigin::Auto {
        strategy_id: Some("another-entry".to_owned()),
    };
    let other_id = book
        .open(
            WALLET,
            other,
            &entry_fill(),
            &buy_leg(),
            &terms(&book, 5).await,
        )
        .await
        .unwrap()
        .id
        .unwrap();
    let close = CloseFill {
        effective_exit_price: 0.002,
        native_received: 0.2,
        fee_raw: 5_000,
        exit_time: at(3),
    };
    book.close(
        WALLET,
        other_id,
        &close,
        0.002,
        "strategy_exit",
        &sell_leg(100.0),
    )
    .await
    .unwrap();

    let mints = |rounds: Vec<Position>| -> Vec<String> {
        rounds.into_iter().map(|round| round.mint).collect()
    };
    assert_eq!(
        mints(
            book.strategy_rounds(WALLET, "entry-strategy")
                .await
                .unwrap()
        ),
        ["MintOwn"],
        "a round entered before the epoch or by another strategy is not counted"
    );
    assert_eq!(
        mints(book.strategy_rounds(WALLET, "exit-strategy").await.unwrap()),
        ["MintOther"],
        "an exit strategy counts the rounds its exit leg closed"
    );
    assert!(book
        .strategy_rounds(WALLET, "another-entry")
        .await
        .unwrap()
        .iter()
        .all(|round| round.exit_time.is_some()));
}

#[tokio::test]
async fn concurrent_buys_never_spend_more_than_the_run_cash() {
    let (book, _directory) = paper_book().await;
    let epoch = book.account(WALLET, 0.25).await.unwrap().epoch;
    let terms = PaperOpenTerms {
        max_open: 5,
        open_cooldown_secs: 0,
        epoch,
    };
    let mints: Vec<String> = (0..3).map(|index| format!("CashMint{index}")).collect();
    let legs: Vec<PaperLeg> = mints.iter().map(|_| buy_leg()).collect();
    let fill = entry_fill();
    let results = join_all(
        mints
            .iter()
            .zip(&legs)
            .map(|(mint, leg)| book.open(WALLET, opening(mint), &fill, leg, &terms)),
    )
    .await;
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 2);
    assert!(results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .all(|error| matches!(error, Error::PaperCashShort { .. })));
    assert!(book.account(WALLET, 0.25).await.unwrap().cash_native >= 0.0);

    let id = book.open_positions().await[0].id.unwrap();
    let add = DcaFill {
        tokens_bought: RawAmount::new(10_000_000),
        native_spent: 0.1,
        fee_raw: 5_000,
        dca_time: Utc::now(),
    };
    let refused = book
        .dca(WALLET, id, &add, 0.001, DECIMALS, &buy_leg())
        .await;
    assert!(
        matches!(refused, Err(Error::PaperCashShort { .. })),
        "an add the cash cannot cover was booked: {refused:?}"
    );
    assert_eq!(book.position(id).await.unwrap().dca_count, 0);
}

#[tokio::test]
async fn a_paper_open_within_the_cooldown_is_refused_and_the_live_open_time_is_untouched() {
    let (book, _directory) = paper_book().await;
    let live_before = *crate::positions::state::LAST_OPEN_TIME.read().await;
    let epoch = book.account(WALLET, 10.0).await.unwrap().epoch;
    let cooling = PaperOpenTerms {
        max_open: 5,
        open_cooldown_secs: 60,
        epoch,
    };
    assert_eq!(book.last_open(), None);
    book.open(
        WALLET,
        opening("CoolA"),
        &entry_fill(),
        &buy_leg(),
        &cooling,
    )
    .await
    .unwrap();
    assert!(book.last_open().is_some());
    match book
        .open(
            WALLET,
            opening("CoolB"),
            &entry_fill(),
            &buy_leg(),
            &cooling,
        )
        .await
    {
        Err(Error::PaperOpenCooldown { wait_secs }) => {
            assert!((59..=60).contains(&wait_secs), "{wait_secs}")
        }
        other => panic!("a second paper open within the cooldown was not refused: {other:?}"),
    }
    assert_eq!(book.open_positions().await.len(), 1);
    assert_eq!(
        *crate::positions::state::LAST_OPEN_TIME.read().await,
        live_before,
        "a paper open moved the live open time"
    );

    let cooled = PaperOpenTerms {
        open_cooldown_secs: 0,
        ..cooling
    };
    book.open(WALLET, opening("CoolB"), &entry_fill(), &buy_leg(), &cooled)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_reset_racing_a_buy_leaves_no_open_round_outside_the_new_run() {
    for buy_first in [true, false] {
        let (book, _directory) = paper_book().await;
        let epoch = book.account(WALLET, 1.0).await.unwrap().epoch;
        let terms = PaperOpenTerms {
            max_open: 5,
            open_cooldown_secs: 0,
            epoch,
        };
        let leg = buy_leg();
        let fill = entry_fill();
        let buy = book.open(WALLET, opening("RaceMint"), &fill, &leg, &terms);
        let reset = book.reset_run(WALLET, 1.0, "PaperReset", |_| Some(DECIMALS), Utc::now());
        let (bought, next) = if buy_first {
            tokio::join!(buy, reset)
        } else {
            let (next, bought) = tokio::join!(reset, buy);
            (bought, next)
        };
        let next = next.unwrap();
        assert!(
            matches!(bought, Ok(_) | Err(Error::PaperRunEnded { .. })),
            "buy_first={buy_first}: {bought:?}"
        );
        assert_eq!(next.epoch, epoch + 1);
        for round in book.open_positions().await {
            assert!(
                round.entry_time >= next.started_at,
                "buy_first={buy_first}: round {:?} is open outside the run that counts it",
                round.id
            );
        }
        let counted = book
            .db
            .positions_entered_since(WALLET, next.started_at)
            .unwrap();
        assert!(
            (book.account(WALLET, 1.0).await.unwrap().cash_native - paper_cash(1.0, &counted))
                .abs()
                < 1e-12
        );
        assert_eq!(book.account(WALLET, 1.0).await.unwrap().cash_native, 1.0);
    }
}

#[tokio::test]
async fn a_round_entered_in_the_second_a_run_starts_but_before_it_is_not_counted_in_it() {
    let (book, _directory) = paper_book().await;
    let first = book.account(WALLET, 1.0).await.unwrap();
    let second = first.started_at.timestamp() + 2;
    let at = |millis| chrono::DateTime::from_timestamp_millis(second * 1_000 + millis).unwrap();
    let mut entered = opening("SameSecondMint");
    entered.entry_time = at(400);
    book.open(
        WALLET,
        entered,
        &entry_fill(),
        &buy_leg(),
        &terms(&book, 5).await,
    )
    .await
    .unwrap();
    let next = book.start_epoch(WALLET, 1.0, at(900)).await.unwrap();
    assert!(book
        .db
        .positions_entered_since(WALLET, next.started_at)
        .unwrap()
        .is_empty());
    assert_eq!(next.cash_native, 1.0);
}

#[tokio::test]
async fn a_failed_exit_streak_is_read_back_from_the_store_after_a_restart() {
    let (book, directory) = paper_book().await;
    let id = book
        .open(
            WALLET,
            opening("StreakMint"),
            &entry_fill(),
            &buy_leg(),
            &terms(&book, 5).await,
        )
        .await
        .unwrap()
        .id
        .unwrap();
    book.record_exit_attempt(id, Some("exit-strategy"), false)
        .await
        .unwrap();
    book.record_exit_attempt(id, Some("exit-strategy"), false)
        .await
        .unwrap();
    drop(book);

    let reopened = PaperBook::new(
        PositionsDatabase::open_at(
            Book::Paper,
            directory.path().join("paper_positions.db"),
            ChainId::Solana,
        )
        .await
        .unwrap(),
        Vec::new(),
    );
    for strategy_id in ["entry-strategy", "exit-strategy"] {
        assert_eq!(
            reopened
                .failed_exit_streak(WALLET, strategy_id)
                .await
                .unwrap(),
            2,
            "{strategy_id}"
        );
    }
    assert_eq!(
        reopened
            .failed_exit_streak(
                "AnotherWa11et11111111111111111111111111111",
                "exit-strategy"
            )
            .await
            .unwrap(),
        0,
        "another wallet's rounds are not counted"
    );
    reopened
        .record_exit_attempt(id, Some("exit-strategy"), true)
        .await
        .unwrap();
    assert_eq!(
        reopened
            .failed_exit_streak(WALLET, "entry-strategy")
            .await
            .unwrap(),
        0
    );
}

#[test]
fn paper_cash_takes_every_buy_sale_and_fee_on_realized_terms() {
    let mut round = Position::opening(opening("MintE"));
    round.total_size_native = 0.3;
    round.native_received = Some(0.5);
    round.entry_fee_raw = Some(10_000);
    round.exit_fee_raw = Some(10_000);
    let fees = crate::positions::pnl::calculate_position_total_fees(&round);
    let realized = crate::positions::pnl::realized_pnl(&round).0;
    assert!((paper_cash(1.0, &[round.clone()]) - (1.0 + realized)).abs() < 1e-12);
    assert!((paper_cash(1.0, &[round]) - (1.0 + 0.2 - fees)).abs() < 1e-12);
    assert_eq!(paper_cash(1.0, &[]), 1.0);
}
