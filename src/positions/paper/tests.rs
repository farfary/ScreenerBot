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
use crate::positions::db::PaperExitOwner;
use crate::positions::types::{Opening, Position, PositionManagement, PositionOrigin};
use crate::positions::Error;
use crate::trader::paper::{PaperFill, PaperModel, PaperSellFill};

/// The configured trading wallet. A stored round is read back under the configured
/// wallet's address, so every round is booked under it.
fn wallet() -> &'static str {
    static WALLET: std::sync::LazyLock<String> =
        std::sync::LazyLock::new(crate::chains::solana::accounts::configure_test_trading_wallet);
    &WALLET
}
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
    PaperLeg::buy(&fill, Some("entry-strategy".to_owned()), Utc::now(), None).unwrap()
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
    PaperLeg::sell(&fill, Some("exit-strategy".to_owned()), Utc::now(), None).unwrap()
}

/// Open terms within the current run of `wallet()`, starting it with ten SOL when none
/// has started, with no open cooldown.
async fn terms(book: &PaperBook, max_open: usize) -> PaperOpenTerms {
    let epoch = book.account(wallet(), 10.0).await.unwrap().epoch;
    PaperOpenTerms {
        max_open,
        open_cooldown_secs: 0,
        epoch,
    }
}

/// Opens a round through [`PaperBook::open`], expecting a new round rather than a replay.
async fn open_round(
    book: &PaperBook,
    wallet_address: &str,
    opening: Opening,
    fill: &EntryFill,
    leg: &PaperLeg,
    terms: &PaperOpenTerms,
) -> crate::positions::Result<Position> {
    match book.open(wallet_address, opening, fill, leg, terms).await? {
        PaperOpened::Opened(position) => Ok(position),
        PaperOpened::AlreadyBooked(position) => {
            panic!("expected a new round, the leg had opened {:?}", position.id)
        }
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
    let opened = open_round(
        &book,
        wallet(),
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
        .dca(wallet(), id, &dca, 0.0005, DECIMALS, &buy_leg())
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
        .partial(wallet(), id, &partial, 50.0, Some(1), &sell_leg(75.0))
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
        .close(wallet(), id, &close, 0.002, "take_profit", &sell_leg(75.0))
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
        .positions_entered_since(wallet(), Utc::now() - Duration::days(1))
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
    let id = open_round(
        &book,
        wallet(),
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
        book.dca(wallet(), id, &dca, 0.001, DECIMALS, &leg)
            .await
            .unwrap(),
        PaperBooked::Booked(_)
    ));
    assert!(matches!(
        book.dca(wallet(), id, &dca, 0.001, DECIMALS, &leg)
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
        book.close(wallet(), id, &close, 0.001, "stop_loss", &sell_leg(110.0))
            .await
            .unwrap(),
        PaperBooked::Booked(_)
    ));
    let legs = count(&book, "SELECT COUNT(*) FROM paper_legs");
    assert!(matches!(
        book.close(wallet(), id, &close, 0.001, "stop_loss", &sell_leg(110.0))
            .await
            .unwrap(),
        PaperBooked::AlreadyClosed
    ));
    assert!(matches!(
        book.dca(wallet(), id, &dca, 0.001, DECIMALS, &buy_leg())
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
            .map(|(mint, leg)| open_round(&book, wallet(), opening(mint), &fill, leg, &terms)),
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
    open_round(
        &book,
        wallet(),
        opening("MintC"),
        &entry_fill(),
        &buy_leg(),
        &terms(&book, 5).await,
    )
    .await
    .unwrap();
    let error = open_round(
        &book,
        wallet(),
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
    let id = open_round(
        &book,
        wallet(),
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
        book.partial(wallet(), id, &partial, 25.0, Some(2), &sell_leg(25.0))
            .await
            .unwrap(),
        PaperBooked::StepTaken
    ));
    assert!(matches!(
        book.partial(wallet(), id, &partial, 25.0, Some(1), &sell_leg(25.0))
            .await
            .unwrap(),
        PaperBooked::Booked(_)
    ));
    assert!(matches!(
        book.partial(wallet(), id, &partial, 25.0, Some(1), &sell_leg(25.0))
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

#[tokio::test(flavor = "multi_thread")]
async fn a_reset_settles_open_rounds_at_their_mark_before_the_next_run() {
    let (book, _directory) = paper_book().await;
    let first = book.account(wallet(), 1.0).await.unwrap();
    let mut entered = opening("MintR");
    entered.entry_time = first.started_at + Duration::seconds(1);
    let id = open_round(
        &book,
        wallet(),
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
        .reset_run(wallet(), 2.0, "PaperReset", |_| None, later)
        .await;
    assert!(matches!(refused, Err(Error::PaperFillUnbookable { .. })));
    assert_eq!(book.open_positions().await.len(), 1);
    assert_eq!(book.account(wallet(), 2.0).await.unwrap().epoch, 1);

    let next = book
        .reset_run(wallet(), 2.0, "PaperReset", |_| Some(DECIMALS), later)
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
    let first = book.account(wallet(), 1.0).await.unwrap();
    assert_eq!(first.epoch, 1);
    assert_eq!(first.cash_native, 1.0);

    let mut entered = opening("MintD");
    entered.entry_time = first.started_at + Duration::seconds(1);
    open_round(
        &book,
        wallet(),
        entered,
        &entry_fill(),
        &buy_leg(),
        &terms(&book, 5).await,
    )
    .await
    .unwrap();
    let fee = crate::positions::pnl::calculate_position_total_fees(&book.open_positions().await[0]);
    let after_open = book.account(wallet(), 5.0).await.unwrap();
    assert_eq!(after_open.epoch, 1, "an existing epoch keeps its balance");
    assert!((after_open.cash_native - (1.0 - 0.1 - fee)).abs() < 1e-12);

    let next = book
        .start_epoch(wallet(), 2.0, first.started_at + Duration::seconds(2))
        .await
        .unwrap();
    assert_eq!(next.epoch, 2);
    assert_eq!(next.cash_native, 2.0);
}

#[tokio::test]
async fn a_strategy_counts_the_rounds_it_opened_or_closed_in_the_current_epoch() {
    let (book, _directory) = paper_book().await;
    assert!(book
        .strategy_rounds(wallet(), "entry-strategy")
        .await
        .unwrap()
        .is_empty());
    let epoch = book.account(wallet(), 1.0).await.unwrap();
    let at = |seconds| epoch.started_at + Duration::seconds(seconds);

    let mut before = opening("MintBefore");
    before.entry_time = epoch.started_at - Duration::seconds(5);
    open_round(
        &book,
        wallet(),
        before,
        &entry_fill(),
        &buy_leg(),
        &terms(&book, 5).await,
    )
    .await
    .unwrap();
    let mut own = opening("MintOwn");
    own.entry_time = at(1);
    open_round(
        &book,
        wallet(),
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
    let other_id = open_round(
        &book,
        wallet(),
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
        wallet(),
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
            book.strategy_rounds(wallet(), "entry-strategy")
                .await
                .unwrap()
        ),
        ["MintOwn"],
        "a round entered before the epoch or by another strategy is not counted"
    );
    assert_eq!(
        mints(
            book.strategy_rounds(wallet(), "exit-strategy")
                .await
                .unwrap()
        ),
        ["MintOther"],
        "an exit strategy counts the rounds its exit leg closed"
    );
    assert!(book
        .strategy_rounds(wallet(), "another-entry")
        .await
        .unwrap()
        .iter()
        .all(|round| round.exit_time.is_some()));
}

#[tokio::test]
async fn concurrent_buys_never_spend_more_than_the_run_cash() {
    let (book, _directory) = paper_book().await;
    let epoch = book.account(wallet(), 0.25).await.unwrap().epoch;
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
            .map(|(mint, leg)| open_round(&book, wallet(), opening(mint), &fill, leg, &terms)),
    )
    .await;
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 2);
    assert!(results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .all(|error| matches!(error, Error::PaperCashShort { .. })));
    assert!(book.account(wallet(), 0.25).await.unwrap().cash_native >= 0.0);

    let id = book.open_positions().await[0].id.unwrap();
    let add = DcaFill {
        tokens_bought: RawAmount::new(10_000_000),
        native_spent: 0.1,
        fee_raw: 5_000,
        dca_time: Utc::now(),
    };
    let refused = book
        .dca(wallet(), id, &add, 0.001, DECIMALS, &buy_leg())
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
    let epoch = book.account(wallet(), 10.0).await.unwrap().epoch;
    let cooling = PaperOpenTerms {
        max_open: 5,
        open_cooldown_secs: 60,
        epoch,
    };
    assert_eq!(book.last_open(), None);
    open_round(
        &book,
        wallet(),
        opening("CoolA"),
        &entry_fill(),
        &buy_leg(),
        &cooling,
    )
    .await
    .unwrap();
    assert!(book.last_open().is_some());
    match open_round(
        &book,
        wallet(),
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
    open_round(
        &book,
        wallet(),
        opening("CoolB"),
        &entry_fill(),
        &buy_leg(),
        &cooled,
    )
    .await
    .unwrap();
}

/// A reset reads its time before it waits for the capacity lock, so a buy that takes the
/// lock first can book a round at that instant or after it. The round belongs to the run
/// that ends: the new run starts after it and keeps its whole starting balance.
#[tokio::test]
async fn a_round_booked_at_or_after_the_reset_time_stays_in_the_run_that_ends() {
    for offset in [Duration::zero(), Duration::milliseconds(5)] {
        let (book, _directory) = paper_book().await;
        let terms = terms(&book, 5).await;
        let reset_at = Utc::now();
        let round = open_round(
            &book,
            wallet(),
            Opening {
                entry_time: reset_at + offset,
                ..opening("LateMint")
            },
            &entry_fill(),
            &buy_leg(),
            &terms,
        )
        .await
        .unwrap();

        let next = book
            .reset_run(wallet(), 1.0, "PaperReset", |_| Some(DECIMALS), reset_at)
            .await
            .unwrap();

        assert!(next.started_at > round.entry_time, "offset {offset}");
        assert_eq!(next.cash_native, 1.0, "offset {offset}");
        assert!(book
            .db
            .positions_entered_since(wallet(), next.started_at)
            .unwrap()
            .is_empty());
    }
}

#[tokio::test]
async fn a_reset_racing_a_buy_leaves_no_open_round_outside_the_new_run() {
    for buy_first in [true, false] {
        let (book, _directory) = paper_book().await;
        let epoch = book.account(wallet(), 1.0).await.unwrap().epoch;
        let terms = PaperOpenTerms {
            max_open: 5,
            open_cooldown_secs: 0,
            epoch,
        };
        let leg = buy_leg();
        let fill = entry_fill();
        let buy = open_round(&book, wallet(), opening("RaceMint"), &fill, &leg, &terms);
        let reset = book.reset_run(wallet(), 1.0, "PaperReset", |_| Some(DECIMALS), Utc::now());
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
            .positions_entered_since(wallet(), next.started_at)
            .unwrap();
        assert!(
            (book.account(wallet(), 1.0).await.unwrap().cash_native - paper_cash(1.0, &counted))
                .abs()
                < 1e-12
        );
        assert_eq!(book.account(wallet(), 1.0).await.unwrap().cash_native, 1.0);
    }
}

#[tokio::test]
async fn a_round_entered_in_the_second_a_run_starts_but_before_it_is_not_counted_in_it() {
    let (book, _directory) = paper_book().await;
    let first = book.account(wallet(), 1.0).await.unwrap();
    let second = first.started_at.timestamp() + 2;
    let at = |millis| chrono::DateTime::from_timestamp_millis(second * 1_000 + millis).unwrap();
    let mut entered = opening("SameSecondMint");
    entered.entry_time = at(400);
    open_round(
        &book,
        wallet(),
        entered,
        &entry_fill(),
        &buy_leg(),
        &terms(&book, 5).await,
    )
    .await
    .unwrap();
    let next = book.start_epoch(wallet(), 1.0, at(900)).await.unwrap();
    assert!(book
        .db
        .positions_entered_since(wallet(), next.started_at)
        .unwrap()
        .is_empty());
    assert_eq!(next.cash_native, 1.0);
}

#[tokio::test]
async fn a_failed_exit_streak_is_read_back_from_the_store_after_a_restart() {
    let (book, directory) = paper_book().await;
    let id = open_round(
        &book,
        wallet(),
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
                .failed_exit_streak(wallet(), &strategy(strategy_id))
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
                &strategy("exit-strategy")
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
            .failed_exit_streak(wallet(), &strategy("entry-strategy"))
            .await
            .unwrap(),
        0
    );
}

fn strategy(strategy_id: &str) -> PaperExitOwner {
    PaperExitOwner::Strategy(strategy_id.to_owned())
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

fn copy_opening(mint: &str, task_id: i64) -> Opening {
    Opening {
        origin: PositionOrigin::Copy {
            task_id,
            source_wallet: "SourceWa11et111111111111111111111111111111".to_owned(),
        },
        management: PositionManagement::CopyTask,
        ..opening(mint)
    }
}

fn keyed_buy_leg(key: &str) -> PaperLeg {
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
    PaperLeg::buy(&fill, None, Utc::now(), Some(key)).unwrap()
}

fn keyed_sell_leg(key: &str, tokens: f64) -> PaperLeg {
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
    PaperLeg::sell(&fill, None, Utc::now(), Some(key)).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_buy_booked_twice_under_one_key_opens_one_round() {
    let (book, _directory) = paper_book().await;
    let terms = terms(&book, 5).await;
    let first = open_round(
        &book,
        wallet(),
        copy_opening("KeyMint", 7),
        &entry_fill(),
        &keyed_buy_leg("copy-7-sig"),
        &terms,
    )
    .await
    .unwrap();
    let replay = book
        .open(
            wallet(),
            copy_opening("KeyMint", 7),
            &entry_fill(),
            &keyed_buy_leg("copy-7-sig"),
            &terms,
        )
        .await
        .unwrap();
    let PaperOpened::AlreadyBooked(stored) = replay else {
        panic!("a replayed keyed buy opened a second round");
    };
    assert_eq!(stored.id, first.id);
    assert_eq!(count(&book, "SELECT COUNT(*) FROM positions"), 1);
    assert_eq!(count(&book, "SELECT COUNT(*) FROM paper_legs"), 1);
    assert_eq!(count(&book, "SELECT COUNT(*) FROM position_entries"), 1);
    let (position_id, leg) = book
        .paper_leg(&PaperLeg::reference("copy-7-sig"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(Some(position_id), first.id);
    assert_eq!(leg.kind, "entry");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_replayed_keyed_buy_is_answered_before_the_capacity_checks() {
    let (book, _directory) = paper_book().await;
    let terms = terms(&book, 1).await;
    open_round(
        &book,
        wallet(),
        copy_opening("FullMint", 7),
        &entry_fill(),
        &keyed_buy_leg("copy-7-full"),
        &terms,
    )
    .await
    .unwrap();
    let replay = book
        .open(
            wallet(),
            copy_opening("FullMint", 7),
            &entry_fill(),
            &keyed_buy_leg("copy-7-full"),
            &terms,
        )
        .await
        .unwrap();
    assert!(matches!(replay, PaperOpened::AlreadyBooked(_)));
}

#[tokio::test]
async fn a_sale_booked_twice_under_one_key_books_one_exit() {
    let (book, _directory) = paper_book().await;
    let id = open_round(
        &book,
        wallet(),
        copy_opening("SaleMint", 3),
        &entry_fill(),
        &keyed_buy_leg("copy-3-buy"),
        &terms(&book, 5).await,
    )
    .await
    .unwrap()
    .id
    .unwrap();
    let partial = PartialExitFill {
        exit_amount: RawAmount::new(50_000_000),
        native_received: 0.1,
        effective_exit_price: 0.002,
        unrealized_pnl: None,
        fee_raw: 5_000,
    };
    let leg = keyed_sell_leg("copy-3-sell", 50.0);
    assert!(matches!(
        book.partial(wallet(), id, &partial, 50.0, None, &leg)
            .await
            .unwrap(),
        PaperBooked::Booked(_)
    ));
    let replayed = keyed_sell_leg("copy-3-sell", 50.0);
    assert!(matches!(
        book.partial(wallet(), id, &partial, 50.0, None, &replayed)
            .await
            .unwrap(),
        PaperBooked::AlreadyBooked
    ));
    assert_eq!(count(&book, "SELECT COUNT(*) FROM position_exits"), 1);
    assert_eq!(book.position(id).await.unwrap().partial_exit_count, 1);

    let other = keyed_sell_leg("copy-3-other", 25.0);
    assert!(matches!(
        book.partial(wallet(), id, &partial, 50.0, None, &other)
            .await
            .unwrap(),
        PaperBooked::Booked(_)
    ));
    assert_eq!(count(&book, "SELECT COUNT(*) FROM position_exits"), 2);
    assert_ne!(leg.leg_ref, other.leg_ref);
}

#[tokio::test]
async fn copy_task_rounds_are_only_that_tasks_rounds_since_the_start() {
    let (book, _directory) = paper_book().await;
    let terms = terms(&book, 10).await;
    let before = open_round(
        &book,
        wallet(),
        copy_opening("EarlyMint", 4),
        &entry_fill(),
        &buy_leg(),
        &terms,
    )
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let start = Utc::now();
    let own = open_round(
        &book,
        wallet(),
        copy_opening("OwnMint", 4),
        &entry_fill(),
        &buy_leg(),
        &terms,
    )
    .await
    .unwrap();
    open_round(
        &book,
        wallet(),
        copy_opening("OtherTaskMint", 40),
        &entry_fill(),
        &buy_leg(),
        &terms,
    )
    .await
    .unwrap();
    open_round(
        &book,
        wallet(),
        opening("StrategyMint"),
        &entry_fill(),
        &buy_leg(),
        &terms,
    )
    .await
    .unwrap();
    let closed = open_round(
        &book,
        wallet(),
        copy_opening("ClosedMint", 4),
        &entry_fill(),
        &buy_leg(),
        &terms,
    )
    .await
    .unwrap();
    let close = CloseFill {
        effective_exit_price: 0.001,
        native_received: 0.1,
        fee_raw: 5_000,
        exit_time: Utc::now(),
    };
    book.close(
        wallet(),
        closed.id.unwrap(),
        &close,
        0.001,
        "stop_loss",
        &sell_leg(100.0),
    )
    .await
    .unwrap();

    let rounds = book.copy_task_rounds(wallet(), 4, start).await.unwrap();
    let ids: Vec<_> = rounds.iter().map(|round| round.id).collect();
    assert_eq!(ids, vec![own.id, closed.id]);
    assert!(!ids.contains(&before.id));
    assert!(book
        .copy_task_rounds(wallet(), 5, start)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn settling_a_subset_leaves_the_other_rounds_open() {
    let (book, _directory) = paper_book().await;
    let terms = terms(&book, 10).await;
    let settled = open_round(
        &book,
        wallet(),
        copy_opening("SettleA", 9),
        &entry_fill(),
        &buy_leg(),
        &terms,
    )
    .await
    .unwrap();
    let kept = open_round(
        &book,
        wallet(),
        opening("SettleB"),
        &entry_fill(),
        &buy_leg(),
        &terms,
    )
    .await
    .unwrap();
    let positions_before = count(&book, "SELECT COUNT(*) FROM positions");
    let closed = book
        .settle_at_mark(
            wallet(),
            &[settled.clone()],
            "PaperReset",
            |_| Some(DECIMALS),
            Utc::now(),
        )
        .await
        .unwrap();
    assert_eq!(closed, 1);
    assert_eq!(
        book.open_positions()
            .await
            .iter()
            .map(|round| round.id)
            .collect::<Vec<_>>(),
        vec![kept.id]
    );
    let stored = book.stored(settled.id.unwrap()).await.unwrap().unwrap();
    assert_eq!(stored.closed_reason.as_deref(), Some("PaperReset"));
    assert_eq!(
        count(&book, "SELECT COUNT(*) FROM positions"),
        positions_before
    );
    let again = book
        .settle_at_mark(
            wallet(),
            &[settled],
            "PaperReset",
            |_| Some(DECIMALS),
            Utc::now(),
        )
        .await
        .unwrap();
    assert_eq!(again, 0, "a round already closed is not settled twice");
}

#[tokio::test(flavor = "multi_thread")]
async fn set_management_changes_the_store_and_the_open_round() {
    let (book, _directory) = paper_book().await;
    let id = open_round(
        &book,
        wallet(),
        copy_opening("ManagedMint", 2),
        &entry_fill(),
        &buy_leg(),
        &terms(&book, 5).await,
    )
    .await
    .unwrap()
    .id
    .unwrap();
    assert!(book
        .set_management(id, PositionManagement::Hybrid)
        .await
        .unwrap());
    assert_eq!(
        book.position(id).await.unwrap().management,
        PositionManagement::Hybrid
    );
    assert_eq!(
        book.stored(id).await.unwrap().unwrap().management,
        PositionManagement::Hybrid
    );
    assert!(!book
        .set_management(id + 100, PositionManagement::Hybrid)
        .await
        .unwrap());
}

#[tokio::test]
async fn a_copy_tasks_failed_exits_are_counted_for_that_task_only() {
    let (book, _directory) = paper_book().await;
    let terms = terms(&book, 5).await;
    let own = open_round(
        &book,
        wallet(),
        copy_opening("TaskStreakA", 11),
        &entry_fill(),
        &buy_leg(),
        &terms,
    )
    .await
    .unwrap()
    .id
    .unwrap();
    let other = open_round(
        &book,
        wallet(),
        copy_opening("TaskStreakB", 111),
        &entry_fill(),
        &buy_leg(),
        &terms,
    )
    .await
    .unwrap()
    .id
    .unwrap();
    book.record_exit_attempt(own, None, false).await.unwrap();
    book.record_exit_attempt(own, None, false).await.unwrap();
    book.record_exit_attempt(other, None, false).await.unwrap();
    assert_eq!(
        book.failed_exit_streak(wallet(), &PaperExitOwner::CopyTask(11))
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        book.failed_exit_streak(wallet(), &PaperExitOwner::CopyTask(111))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        book.failed_exit_streak(wallet(), &PaperExitOwner::CopyTask(1))
            .await
            .unwrap(),
        0,
        "task 1 does not match the rounds of task 11 or 111"
    );
    book.record_exit_attempt(own, None, true).await.unwrap();
    assert_eq!(
        book.failed_exit_streak(wallet(), &PaperExitOwner::CopyTask(11))
            .await
            .unwrap(),
        0
    );
}

#[test]
fn the_exit_owner_of_a_round_is_its_strategy_or_its_copy_task() {
    assert_eq!(
        PaperExitOwner::of_round(&PositionOrigin::Auto {
            strategy_id: Some("s".to_owned())
        }),
        Some(PaperExitOwner::Strategy("s".to_owned()))
    );
    assert_eq!(
        PaperExitOwner::of_round(&copy_opening("M", 5).origin),
        Some(PaperExitOwner::CopyTask(5))
    );
    assert_eq!(PaperExitOwner::of_round(&PositionOrigin::Manual), None);
    assert_eq!(PaperExitOwner::of_round(&PositionOrigin::External), None);
}
