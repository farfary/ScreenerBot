// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position bookings and writers against a real positions database: failures change nothing, retries book once, and no writer reverts a committed booking.

mod common;

use std::future::Future;
use std::process::Command;

use chrono::{DateTime, Utc};
use rusqlite::types::Value;
use rusqlite::Connection;
use screenerbot::chains::RawAmount;
use screenerbot::errors::{DatabaseError, ErrorClass};
use screenerbot::positions::apply::apply_transition;
use screenerbot::positions::operations::{force_close_position, mark_exit_submitted};
use screenerbot::positions::price_updater::update_position_price_and_pnl;
use screenerbot::positions::{
    db, state, ApplyFailureDisposition, Error, GiveUpReason, PendingDcaSwap, PendingPartialExit,
    Position, PositionTransition, PriceSource, VerificationItem, VerificationKind,
    FORCE_CLOSED_PREFIX,
};
use screenerbot::trader::safety::loss_limit::{get_loss_limit_status, reset_loss_limit_state};

/// Each test opens the process-wide positions store and in-memory state, so it runs in a
/// child process of its own whichever harness launched it.
const CHILD_ENV: &str = "SCREENERBOT_POSITIONS_APPLY_CHILD";

const HELD: u128 = 1_000_000;
const PARTIAL_SIGNATURE: &str = "partial-exit-sig";
const DCA_SIGNATURE: &str = "dca-sig";
const CLOSE_SIGNATURE: &str = "close-sig";

const INJECT_ROW: &str =
    "CREATE TRIGGER inject_row BEFORE UPDATE ON positions BEGIN SELECT RAISE(ABORT,'injected'); END;";
const INJECT_EXIT: &str = "CREATE TRIGGER inject_exit BEFORE INSERT ON position_exits BEGIN SELECT RAISE(ABORT,'injected'); END;";
const INJECT_ENTRY: &str = "CREATE TRIGGER inject_entry BEFORE INSERT ON position_entries BEGIN SELECT RAISE(ABORT,'injected'); END;";
const INJECT_METADATA: &str = "CREATE TRIGGER inject_meta BEFORE INSERT ON position_metadata BEGIN SELECT RAISE(ABORT,'injected'); END;";

fn run_isolated<F, Fut>(test_name: &str, body: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = ()>,
{
    if std::env::var_os(CHILD_ENV).is_some() {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("create multi-thread runtime")
            .block_on(body());
        return;
    }

    let status = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .status()
        .expect("run isolated child");
    assert!(status.success(), "{test_name} failed in its child process");
}

/// The booking columns a verified transition may change.
#[derive(Debug, Clone, PartialEq)]
struct Booked {
    remaining_token_amount: Option<RawAmount>,
    total_exited_amount: RawAmount,
    native_received: Option<f64>,
    partial_exit_count: u32,
    dca_count: u32,
    total_size_native: f64,
    average_entry_price: f64,
    transaction_exit_verified: bool,
    exit_time: Option<DateTime<Utc>>,
    pnl: Option<f64>,
}

impl Booked {
    fn of(position: &Position) -> Self {
        Self {
            remaining_token_amount: position.remaining_token_amount,
            total_exited_amount: position.total_exited_amount,
            native_received: position.native_received,
            partial_exit_count: position.partial_exit_count as u32,
            dca_count: position.dca_count as u32,
            total_size_native: position.total_size_native,
            average_entry_price: position.average_entry_price,
            transaction_exit_verified: position.transaction_exit_verified,
            // RFC 3339 storage keeps whole microseconds at most; compare at that precision.
            exit_time: position
                .exit_time
                .map(|t| DateTime::from_timestamp_micros(t.timestamp_micros()).unwrap()),
            pnl: position.pnl,
        }
    }
}

/// Opens an isolated store and an OPEN position holding [`HELD`], persisted and in memory.
async fn open_position(configure: impl FnOnce(&mut Position)) -> i64 {
    screenerbot::paths::ensure_all_directories().expect("create data directories");
    common::configure_own_wallet();
    common::seed_decimals(common::TEST_MINT, 6);
    screenerbot::positions::initialize_positions_database()
        .await
        .expect("initialise positions database");

    let mut position = common::test_position(1.0, 1.0);
    position.id = None;
    position.token_amount = Some(RawAmount::new(HELD));
    position.remaining_token_amount = Some(RawAmount::new(HELD));
    configure(&mut position);
    let id = db::save_position(&position)
        .await
        .expect("persist test position");
    position.id = Some(id);
    state::add_position(position).await;
    id
}

/// A second connection to the same store, for fault injection.
fn injector() -> Connection {
    Connection::open(screenerbot::paths::get_positions_db_path()).expect("open injector")
}

async fn in_memory(id: i64) -> Booked {
    Booked::of(
        &state::get_position_by_id(id)
            .await
            .expect("position in memory"),
    )
}

async fn in_storage(id: i64) -> Booked {
    Booked::of(
        &db::get_position_by_id(id)
            .await
            .expect("read stored position")
            .expect("position stored"),
    )
}

/// Memory and storage both still hold `before`.
async fn assert_unchanged(id: i64, before: &Booked) {
    assert_eq!(&in_memory(id).await, before, "memory changed");
    assert_eq!(&in_storage(id).await, before, "stored row changed");
}

async fn exit_records(id: i64) -> usize {
    db::get_exit_history(id).await.expect("read exits").len()
}

async fn entry_records(id: i64) -> usize {
    db::get_entry_history(id).await.expect("read entries").len()
}

fn assert_query_failure(error: &Error) {
    assert!(
        matches!(error, Error::Database(DatabaseError::Query { .. })),
        "an injected failure surfaces as a query error, got {error:?}"
    );
}

async fn register_partial(id: i64) {
    state::register_pending_partial_exit(PendingPartialExit {
        signature: PARTIAL_SIGNATURE.to_owned(),
        mint: common::TEST_MINT.to_owned(),
        position_id: id,
        expected_exit_amount: RawAmount::new(400_000),
        requested_exit_percentage: 40.0,
        expiry_height: None,
        created_at: Utc::now(),
    })
    .await
    .expect("register pending partial exit");
    state::mark_partial_exit_pending(common::TEST_MINT).await;
}

async fn partial_pending() -> bool {
    state::get_pending_partial_exit(PARTIAL_SIGNATURE)
        .await
        .is_some()
        && state::is_partial_exit_pending(common::TEST_MINT).await
}

async fn partial_cleared() -> bool {
    state::get_pending_partial_exit(PARTIAL_SIGNATURE)
        .await
        .is_none()
        && !state::is_partial_exit_pending(common::TEST_MINT).await
}

fn partial_exit(id: i64) -> PositionTransition {
    PositionTransition::PartialExitVerified {
        position_id: id,
        exit_amount: RawAmount::new(400_000),
        native_received: 0.8,
        effective_exit_price: 2.0,
        fee_raw: 5_000,
        exit_time: Utc::now(),
        exit_signature: PARTIAL_SIGNATURE.to_owned(),
        exit_percentage: 40.0,
    }
}

async fn register_dca(id: i64) {
    state::register_pending_dca_swap(PendingDcaSwap {
        signature: DCA_SIGNATURE.to_owned(),
        mint: common::TEST_MINT.to_owned(),
        position_id: id,
        expiry_height: None,
        created_at: Utc::now(),
        size_sol: 0.5,
    })
    .await
    .expect("register pending DCA");
}

async fn dca_pending() -> bool {
    !state::get_pending_dca_swaps_for_mint(common::TEST_MINT)
        .await
        .is_empty()
}

fn dca(id: i64) -> PositionTransition {
    PositionTransition::DcaVerified {
        position_id: id,
        tokens_bought: RawAmount::new(500_000),
        native_spent: 0.5,
        effective_price: 1.0,
        fee_raw: 5_000,
        dca_time: Utc::now(),
        dca_signature: DCA_SIGNATURE.to_owned(),
    }
}

/// Asserts the partial exit is booked exactly once, in memory and in storage.
async fn assert_partial_booked_once(id: i64, before: &Booked) {
    let booked = in_memory(id).await;
    assert_eq!(booked, in_storage(id).await, "memory and storage disagree");
    assert_eq!(booked.remaining_token_amount, Some(RawAmount::new(600_000)));
    assert_eq!(booked.total_exited_amount, RawAmount::new(400_000));
    assert_eq!(booked.partial_exit_count, before.partial_exit_count + 1);
    assert_eq!(booked.native_received, Some(0.8));
    assert_eq!(exit_records(id).await, 1);
    assert!(partial_cleared().await, "pending partial exit not cleared");
}

#[test]
fn a_failed_partial_exit_write_changes_nothing_and_a_retry_books_once() {
    run_isolated(
        "a_failed_partial_exit_write_changes_nothing_and_a_retry_books_once",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            register_partial(id).await;
            let before = in_storage(id).await;
            let injector = injector();
            injector.execute_batch(INJECT_ROW).expect("install trigger");

            let error = apply_transition(partial_exit(id))
                .await
                .expect_err("an injected row failure must fail the apply");
            assert_query_failure(&error);
            assert_unchanged(id, &before).await;
            assert_eq!(exit_records(id).await, 0);
            assert!(partial_pending().await, "pending partial exit lost");

            injector
                .execute_batch("DROP TRIGGER inject_row;")
                .expect("drop trigger");
            apply_transition(partial_exit(id))
                .await
                .expect("retry books the partial exit");
            assert_partial_booked_once(id, &before).await;
            let booked = in_storage(id).await;

            apply_transition(partial_exit(id))
                .await
                .expect("a third apply is a no-op");
            assert_eq!(in_memory(id).await, booked);
            assert_eq!(in_storage(id).await, booked);
            assert_eq!(exit_records(id).await, 1);
        },
    );
}

#[test]
fn a_failed_exit_record_insert_rolls_back_the_partial_exit_row() {
    run_isolated(
        "a_failed_exit_record_insert_rolls_back_the_partial_exit_row",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            register_partial(id).await;
            let before = in_storage(id).await;
            let injector = injector();
            injector
                .execute_batch(INJECT_EXIT)
                .expect("install trigger");

            let error = apply_transition(partial_exit(id))
                .await
                .expect_err("an injected record failure must fail the apply");
            assert_query_failure(&error);
            assert_unchanged(id, &before).await;
            assert_eq!(exit_records(id).await, 0);
            assert!(partial_pending().await, "pending partial exit lost");

            injector
                .execute_batch("DROP TRIGGER inject_exit;")
                .expect("drop trigger");
            apply_transition(partial_exit(id))
                .await
                .expect("retry books the partial exit");
            assert_partial_booked_once(id, &before).await;
            let booked = in_storage(id).await;

            apply_transition(partial_exit(id))
                .await
                .expect("a third apply is a no-op");
            assert_eq!(in_memory(id).await, booked);
            assert_eq!(in_storage(id).await, booked);
            assert_eq!(exit_records(id).await, 1);
        },
    );
}

#[test]
fn a_failed_pending_clear_keeps_the_mint_counter_until_the_detail_is_cleared() {
    run_isolated(
        "a_failed_pending_clear_keeps_the_mint_counter_until_the_detail_is_cleared",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            register_partial(id).await;
            let before = in_storage(id).await;
            let injector = injector();
            injector
                .execute_batch(INJECT_METADATA)
                .expect("install trigger");

            let error = apply_transition(partial_exit(id))
                .await
                .expect_err("an injected metadata failure must fail the apply");
            assert_query_failure(&error);
            let booked = in_memory(id).await;
            assert_eq!(booked, in_storage(id).await, "memory and storage disagree");
            assert_eq!(booked.remaining_token_amount, Some(RawAmount::new(600_000)));
            assert_eq!(booked.partial_exit_count, before.partial_exit_count + 1);
            assert_eq!(exit_records(id).await, 1);
            assert!(
                partial_pending().await,
                "the pending detail and the mint counter must survive a failed clear"
            );

            injector
                .execute_batch("DROP TRIGGER inject_meta;")
                .expect("drop trigger");
            // A second partial exit for the same mint is in flight.
            state::mark_partial_exit_pending(common::TEST_MINT).await;

            apply_transition(partial_exit(id))
                .await
                .expect("retry clears the pending detail");
            assert!(
                state::get_pending_partial_exit(PARTIAL_SIGNATURE)
                    .await
                    .is_none(),
                "pending detail not cleared"
            );
            assert!(
                state::is_partial_exit_pending(common::TEST_MINT).await,
                "the in-flight partial exit lost its mint counter"
            );
            assert_eq!(in_memory(id).await, booked);
            assert_eq!(in_storage(id).await, booked);
            assert_eq!(exit_records(id).await, 1);
        },
    );
}

#[test]
fn a_failed_dca_write_changes_nothing_and_a_retry_books_once() {
    run_isolated(
        "a_failed_dca_write_changes_nothing_and_a_retry_books_once",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            register_dca(id).await;
            let before = in_storage(id).await;
            let injector = injector();

            for (install, drop) in [
                (INJECT_ROW, "DROP TRIGGER inject_row;"),
                (INJECT_ENTRY, "DROP TRIGGER inject_entry;"),
            ] {
                injector.execute_batch(install).expect("install trigger");
                let error = apply_transition(dca(id))
                    .await
                    .expect_err("an injected failure must fail the apply");
                assert_query_failure(&error);
                assert_unchanged(id, &before).await;
                assert_eq!(entry_records(id).await, 0);
                assert!(dca_pending().await, "pending DCA lost");
                injector.execute_batch(drop).expect("drop trigger");
            }

            apply_transition(dca(id))
                .await
                .expect("retry books the DCA");
            let booked = in_memory(id).await;
            assert_eq!(booked, in_storage(id).await, "memory and storage disagree");
            assert_eq!(booked.dca_count, before.dca_count + 1);
            assert_eq!(
                booked.remaining_token_amount,
                Some(RawAmount::new(HELD + 500_000))
            );
            assert_eq!(booked.total_size_native, before.total_size_native + 0.5);
            assert_eq!(entry_records(id).await, 1);
            assert!(!dca_pending().await, "pending DCA not cleared");

            apply_transition(dca(id))
                .await
                .expect("a third apply is a no-op");
            assert_eq!(in_memory(id).await, booked);
            assert_eq!(in_storage(id).await, booked);
            assert_eq!(entry_records(id).await, 1);
        },
    );
}

#[test]
fn a_failed_close_write_leaves_the_position_open_and_a_retry_closes_it_once() {
    run_isolated(
        "a_failed_close_write_leaves_the_position_open_and_a_retry_closes_it_once",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;
            let before = in_storage(id).await;
            assert!(!before.transaction_exit_verified);
            assert_eq!(before.exit_time, None);
            let injector = injector();
            injector.execute_batch(INJECT_ROW).expect("install trigger");

            let close = || PositionTransition::ExitVerified {
                position_id: id,
                effective_exit_price: 1.5,
                native_received: 1.5,
                fee_raw: 5_000,
                exit_time: Utc::now(),
            };

            let error = apply_transition(close())
                .await
                .expect_err("an injected row failure must fail the apply");
            assert_query_failure(&error);
            assert_unchanged(id, &before).await;
            assert_eq!(exit_records(id).await, 0);

            injector
                .execute_batch("DROP TRIGGER inject_row;")
                .expect("drop trigger");
            apply_transition(close())
                .await
                .expect("retry closes the position");
            let closed = in_memory(id).await;
            assert_eq!(closed, in_storage(id).await, "memory and storage disagree");
            assert!(closed.transaction_exit_verified);
            assert!(closed.exit_time.is_some());
            assert_eq!(closed.remaining_token_amount, Some(RawAmount::ZERO));
            assert_eq!(closed.total_exited_amount, RawAmount::new(HELD));
            assert_eq!(closed.native_received, Some(1.5));
            let exits = db::get_exit_history(id).await.expect("read exits");
            assert_eq!(exits.len(), 1);
            assert!(!exits[0].is_partial);

            apply_transition(close())
                .await
                .expect("a third apply is a no-op");
            assert_eq!(in_memory(id).await, closed);
            assert_eq!(in_storage(id).await, closed);
            assert_eq!(exit_records(id).await, 1);
        },
    );
}

#[test]
fn a_busy_store_fails_retryably_and_changes_nothing() {
    run_isolated(
        "a_busy_store_fails_retryably_and_changes_nothing",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            register_partial(id).await;
            let before = in_storage(id).await;
            let blocker = injector();
            blocker
                .execute_batch("BEGIN IMMEDIATE;")
                .expect("hold the write lock");

            let error = apply_transition(partial_exit(id))
                .await
                .expect_err("a held write lock must fail the apply");
            assert!(
                matches!(error, Error::Database(DatabaseError::Busy { .. })),
                "lock contention surfaces as busy, got {error:?}"
            );
            assert!(error.is_retryable());

            blocker
                .execute_batch("ROLLBACK;")
                .expect("release the write lock");
            assert_unchanged(id, &before).await;
            assert_eq!(exit_records(id).await, 0);
            assert!(partial_pending().await, "pending partial exit lost");

            apply_transition(partial_exit(id))
                .await
                .expect("retry books the partial exit");
            assert_partial_booked_once(id, &before).await;
        },
    );
}

#[test]
fn partial_exit_overflow_changes_nothing_and_records_nothing() {
    run_isolated(
        "partial_exit_overflow_changes_nothing_and_records_nothing",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|position| {
                position.total_exited_amount = RawAmount::MAX;
            })
            .await;
            register_partial(id).await;
            let before = in_storage(id).await;

            let error = apply_transition(partial_exit(id))
                .await
                .expect_err("an overflowing booking must fail the apply");
            assert!(
                matches!(error, Error::AmountOverflow { .. }),
                "expected an overflow, got {error:?}"
            );
            assert!(!error.is_retryable());
            assert_unchanged(id, &before).await;
            assert_eq!(exit_records(id).await, 0);
            assert!(partial_pending().await, "pending partial exit lost");
        },
    );
}

#[test]
fn dca_overflow_changes_nothing_and_records_nothing() {
    run_isolated(
        "dca_overflow_changes_nothing_and_records_nothing",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|position| {
                position.remaining_token_amount = Some(RawAmount::MAX);
            })
            .await;
            register_dca(id).await;
            let before = in_storage(id).await;

            let error = apply_transition(dca(id))
                .await
                .expect_err("an overflowing booking must fail the apply");
            assert!(
                matches!(error, Error::AmountOverflow { .. }),
                "expected an overflow, got {error:?}"
            );
            assert!(!error.is_retryable());
            assert_unchanged(id, &before).await;
            assert_eq!(entry_records(id).await, 0);
            assert!(dca_pending().await, "pending DCA lost");
        },
    );
}

/// The columns the price updater owns.
const PRICE_COLUMNS: [&str; 7] = [
    "current_price",
    "current_price_updated",
    "price_highest",
    "price_lowest",
    "unrealized_pnl",
    "unrealized_pnl_percent",
    "updated_at",
];

/// Every column of the stored row, by name.
fn stored_columns(id: i64) -> Vec<(String, Value)> {
    let conn = injector();
    let mut stmt = conn
        .prepare("SELECT * FROM positions WHERE id = ?1")
        .expect("prepare row read");
    let names: Vec<String> = stmt.column_names().iter().map(|n| n.to_string()).collect();
    stmt.query_row([id], |row| {
        names
            .iter()
            .enumerate()
            .map(|(index, name)| Ok((name.clone(), row.get::<_, Value>(index)?)))
            .collect()
    })
    .expect("read stored row")
}

fn stored_column(id: i64, name: &str) -> Value {
    stored_columns(id)
        .into_iter()
        .find(|(column, _)| column == name)
        .map(|(_, value)| value)
        .expect("column exists")
}

async fn stored_position(id: i64) -> Position {
    db::get_position_by_id(id)
        .await
        .expect("read stored position")
        .expect("position stored")
}

async fn memory_position(id: i64) -> Position {
    state::get_position_by_id(id)
        .await
        .expect("position in memory")
}

/// Enables the period loss limit with a budget no test reaches, from a clean slate.
fn enable_loss_limit() {
    common::set_config(|cfg| {
        cfg.trader.loss_limit_enabled = true;
        cfg.trader.loss_limit_sol = 1_000.0;
        cfg.trader.loss_limit_period_hours = 24;
    });
    reset_loss_limit_state();
}

fn recorded_loss() -> f64 {
    get_loss_limit_status().cumulative_loss_native
}

fn verified_close(id: i64) -> PositionTransition {
    PositionTransition::ExitVerified {
        position_id: id,
        effective_exit_price: 1.5,
        native_received: 1.5,
        fee_raw: 5_000,
        exit_time: Utc::now(),
    }
}

#[test]
fn a_price_update_from_a_stale_snapshot_leaves_a_committed_close_intact() {
    run_isolated(
        "a_price_update_from_a_stale_snapshot_leaves_a_committed_close_intact",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;
            let open_snapshot = memory_position(id).await;

            apply_transition(verified_close(id))
                .await
                .expect("the close commits");
            let closed = in_storage(id).await;
            assert!(closed.transaction_exit_verified);
            assert_eq!(closed, in_memory(id).await, "memory and storage disagree");

            // A price tick that read the position before the close was published.
            assert!(
                state::update_position_state_by_id(id, |live| *live = open_snapshot).await,
                "position in memory"
            );
            update_position_price_and_pnl(id, common::TEST_MINT, 2.0, PriceSource::Api)
                .await
                .expect("price update");

            assert_eq!(in_storage(id).await, closed, "the close was reverted");
            assert_eq!(
                stored_column(id, "exit_transaction_signature"),
                Value::Text(CLOSE_SIGNATURE.to_owned())
            );
            assert_eq!(stored_column(id, "unrealized_pnl"), Value::Null);
            assert_eq!(stored_column(id, "current_price"), Value::Real(2.0));
            assert_eq!(exit_records(id).await, 1);
        },
    );
}

#[test]
fn a_price_update_writes_only_the_price_and_pnl_columns() {
    run_isolated(
        "a_price_update_writes_only_the_price_and_pnl_columns",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            let before = stored_columns(id);

            // Memory ahead of the row on every booking column the full-row write carried.
            assert!(
                state::update_position_state_by_id(id, |live| {
                    live.native_received = Some(9.0);
                    live.remaining_token_amount = Some(RawAmount::new(1));
                    live.total_exited_amount = RawAmount::new(HELD - 1);
                    live.dca_count = 7;
                    live.partial_exit_count = 3;
                    live.total_size_native = 4.0;
                    live.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
                    live.exit_price = Some(3.0);
                    live.closed_reason = Some("memory_only".to_owned());
                    live.pnl = Some(-1.0);
                })
                .await,
                "position in memory"
            );

            update_position_price_and_pnl(id, common::TEST_MINT, 2.0, PriceSource::Api)
                .await
                .expect("price update");

            let after = stored_columns(id);
            assert_eq!(before.len(), after.len());
            for ((name, old), (_, new)) in before.iter().zip(after.iter()) {
                if PRICE_COLUMNS.contains(&name.as_str()) {
                    continue;
                }
                assert_eq!(old, new, "the price update wrote column {name}");
            }
            assert_eq!(stored_column(id, "current_price"), Value::Real(2.0));
            assert_eq!(stored_column(id, "price_highest"), Value::Real(2.0));
            let live = memory_position(id).await;
            assert_eq!(
                stored_column(id, "unrealized_pnl"),
                live.unrealized_pnl.map_or(Value::Null, Value::Real)
            );
            assert!(live.unrealized_pnl.is_some(), "unrealized P&L computed");
        },
    );
}

#[test]
fn a_failed_force_close_write_changes_nothing_and_returns_the_error() {
    run_isolated(
        "a_failed_force_close_write_changes_nothing_and_returns_the_error",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = open_position(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;
            let before = in_storage(id).await;
            let before_reason = stored_position(id).await.closed_reason;
            let injector = injector();
            injector.execute_batch(INJECT_ROW).expect("install trigger");

            let error = force_close_position(id, "stuck")
                .await
                .expect_err("an injected row failure must fail the force close");
            assert_query_failure(&error);
            assert_unchanged(id, &before).await;
            assert_eq!(memory_position(id).await.closed_reason, before_reason);
            assert_eq!(stored_position(id).await.closed_reason, before_reason);
            assert_eq!(recorded_loss(), 0.0, "a failed force close recorded a loss");

            // The exit in flight then verifies: it books on the open position, alone.
            injector
                .execute_batch("DROP TRIGGER inject_row;")
                .expect("drop trigger");
            apply_transition(verified_close(id))
                .await
                .expect("the exit verification books the close");
            let booked = in_memory(id).await;
            assert_eq!(booked, in_storage(id).await, "memory and storage disagree");
            assert!(booked.transaction_exit_verified);
            assert_eq!(booked.native_received, Some(1.5));
            assert_eq!(exit_records(id).await, 1);
            assert_eq!(recorded_loss(), 0.0, "only the verified close is booked");

            let error = force_close_position(id, "stuck")
                .await
                .expect_err("a verified position cannot be force-closed");
            assert!(
                matches!(error, Error::AlreadyClosed { position_id } if position_id == id),
                "expected already closed, got {error:?}"
            );
            assert_eq!(in_memory(id).await, booked);
            assert_eq!(in_storage(id).await, booked);
            assert_eq!(recorded_loss(), 0.0);
        },
    );
}

#[test]
fn a_force_close_of_a_row_already_verified_returns_already_closed() {
    run_isolated(
        "a_force_close_of_a_row_already_verified_returns_already_closed",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = open_position(|_| {}).await;
            let before = in_memory(id).await;
            // The row is closed while memory still shows the position open.
            injector()
                .execute(
                    "UPDATE positions SET transaction_exit_verified = 1 WHERE id = ?1",
                    [id],
                )
                .expect("mark the row verified");

            let error = force_close_position(id, "stuck")
                .await
                .expect_err("the commit guard refuses a verified row");
            assert!(
                matches!(error, Error::AlreadyClosed { position_id } if position_id == id),
                "expected already closed, got {error:?}"
            );
            assert_eq!(in_memory(id).await, before, "memory changed");
            assert_eq!(stored_position(id).await.exit_time, None);
            assert_eq!(
                recorded_loss(),
                0.0,
                "a refused force close recorded a loss"
            );
        },
    );
}

#[test]
fn a_force_close_of_a_position_not_in_memory_books_the_row_once() {
    run_isolated(
        "a_force_close_of_a_position_not_in_memory_books_the_row_once",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = open_position(|_| {}).await;
            assert!(
                state::remove_position_by_id(id).await.is_some(),
                "position in memory"
            );

            let closed = force_close_position(id, "stuck")
                .await
                .expect("force close commits from the stored row");
            assert_eq!(closed.position_id, id);
            assert!(
                state::get_position_by_id(id).await.is_none(),
                "nothing is published into memory"
            );
            let booked = in_storage(id).await;
            assert!(booked.transaction_exit_verified);
            assert!(booked.exit_time.is_some());
            assert_eq!(booked.remaining_token_amount, Some(RawAmount::ZERO));
            assert_eq!(booked.total_exited_amount, RawAmount::new(HELD));
            assert_eq!(booked.pnl, Some(-1.0));
            assert_eq!(recorded_loss(), 1.0);

            let error = force_close_position(id, "again")
                .await
                .expect_err("a closed row cannot be force-closed again");
            assert!(
                matches!(error, Error::AlreadyClosed { position_id } if position_id == id),
                "expected already closed, got {error:?}"
            );
            assert_eq!(in_storage(id).await, booked);
            assert_eq!(recorded_loss(), 1.0, "the loss was recorded twice");
        },
    );
}

#[test]
fn a_force_close_then_a_retried_exit_verification_records_the_loss_once() {
    run_isolated(
        "a_force_close_then_a_retried_exit_verification_records_the_loss_once",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = open_position(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;

            force_close_position(id, "stuck")
                .await
                .expect("force close commits");
            let closed = in_storage(id).await;
            assert_eq!(closed, in_memory(id).await, "memory and storage disagree");
            assert!(closed.transaction_exit_verified);
            assert_eq!(closed.native_received, Some(0.0));
            assert_eq!(recorded_loss(), 1.0);

            apply_transition(verified_close(id))
                .await
                .expect("the retried verification is a no-op");
            assert_eq!(in_memory(id).await, closed);
            assert_eq!(in_storage(id).await, closed);
            assert_eq!(exit_records(id).await, 0);
            assert_eq!(recorded_loss(), 1.0, "the loss was recorded twice");

            let error = force_close_position(id, "again")
                .await
                .expect_err("a closed position cannot be force-closed again");
            assert!(
                matches!(error, Error::AlreadyClosed { position_id } if position_id == id),
                "expected already closed, got {error:?}"
            );
            assert_eq!(recorded_loss(), 1.0);
        },
    );
}

#[test]
fn a_failed_exit_clear_after_a_force_close_leaves_the_close_intact() {
    run_isolated(
        "a_failed_exit_clear_after_a_force_close_leaves_the_close_intact",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = open_position(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;
            force_close_position(id, "stuck")
                .await
                .expect("force close commits");
            let closed = stored_position(id).await;
            let booked = in_storage(id).await;
            assert_eq!(booked, in_memory(id).await, "memory and storage disagree");
            assert_eq!(recorded_loss(), 1.0);

            let error =
                apply_transition(PositionTransition::ExitFailedClearForRetry { position_id: id })
                    .await
                    .expect_err("a verified close refuses the failed-exit clear");
            assert!(
                matches!(error, Error::AlreadyClosed { position_id } if position_id == id),
                "expected already closed, got {error:?}"
            );

            for position in [stored_position(id).await, memory_position(id).await] {
                assert!(position.transaction_exit_verified);
                assert!(position.exit_time.is_some());
                assert_eq!(position.closed_reason, closed.closed_reason);
                assert!(position
                    .closed_reason
                    .as_deref()
                    .is_some_and(|reason| reason.starts_with(FORCE_CLOSED_PREFIX)));
                assert_eq!(position.exit_price, closed.exit_price);
                assert_eq!(position.effective_exit_price, closed.effective_exit_price);
                assert_eq!(
                    position.exit_transaction_signature.as_deref(),
                    Some(CLOSE_SIGNATURE)
                );
            }
            assert_unchanged(id, &booked).await;

            let item = VerificationItem::new(
                CLOSE_SIGNATURE.to_owned(),
                common::TEST_MINT.to_owned(),
                Some(id),
                VerificationKind::Exit,
                None,
            );
            let disposition = item.apply_failure_disposition(&error);
            assert!(
                matches!(
                    disposition,
                    ApplyFailureDisposition::Drop(GiveUpReason::ApplyRejected { .. })
                ),
                "the verification item must be dropped, got {disposition:?}"
            );

            let error = force_close_position(id, "again")
                .await
                .expect_err("the position is still closed");
            assert!(
                matches!(error, Error::AlreadyClosed { position_id } if position_id == id),
                "expected already closed, got {error:?}"
            );
            assert_eq!(recorded_loss(), 1.0, "the loss was recorded twice");
        },
    );
}

#[test]
fn a_failed_exit_clear_on_an_unverified_exit_clears_it_for_retry() {
    run_isolated(
        "a_failed_exit_clear_on_an_unverified_exit_clears_it_for_retry",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
                position.exit_price = Some(1.25);
                position.closed_reason = Some("stop_loss_pending_verification".to_owned());
            })
            .await;
            let before = in_storage(id).await;

            let effects =
                apply_transition(PositionTransition::ExitFailedClearForRetry { position_id: id })
                    .await
                    .expect("an unverified exit is cleared");
            assert!(effects.db_updated);

            for position in [stored_position(id).await, memory_position(id).await] {
                assert!(!position.transaction_exit_verified);
                assert!(position.exit_time.is_none());
                assert_eq!(position.exit_transaction_signature, None);
                assert_eq!(position.exit_price, None);
                assert_eq!(position.effective_exit_price, None);
                assert_eq!(
                    position.closed_reason.as_deref(),
                    Some("exit_retry_pending")
                );
            }
            assert_unchanged(id, &before).await;
        },
    );
}

#[test]
fn a_submitted_exit_is_stored_without_a_price_update() {
    run_isolated(
        "a_submitted_exit_is_stored_without_a_price_update",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            let before = in_storage(id).await;

            mark_exit_submitted(id, CLOSE_SIGNATURE, 1.25, "stop_loss_pending_verification")
                .await
                .expect("record the submitted exit");

            for position in [stored_position(id).await, memory_position(id).await] {
                assert_eq!(
                    position.exit_transaction_signature.as_deref(),
                    Some(CLOSE_SIGNATURE)
                );
                assert_eq!(position.exit_price, Some(1.25));
                assert_eq!(
                    position.closed_reason.as_deref(),
                    Some("stop_loss_pending_verification")
                );
            }
            assert_unchanged(id, &before).await;
        },
    );
}

#[test]
fn a_failed_exit_submission_write_still_marks_the_exit_in_memory() {
    run_isolated(
        "a_failed_exit_submission_write_still_marks_the_exit_in_memory",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            let injector = injector();
            injector.execute_batch(INJECT_ROW).expect("install trigger");

            let error =
                mark_exit_submitted(id, CLOSE_SIGNATURE, 1.25, "stop_loss_pending_verification")
                    .await
                    .expect_err("an injected row failure must surface");
            assert_query_failure(&error);
            assert_eq!(stored_position(id).await.exit_transaction_signature, None);
            // The swap is already submitted: memory must carry the exit so it is not sold
            // again and verification can match it.
            assert_eq!(
                memory_position(id)
                    .await
                    .exit_transaction_signature
                    .as_deref(),
                Some(CLOSE_SIGNATURE)
            );
        },
    );
}

#[test]
fn an_exit_submitted_after_a_force_close_leaves_the_close_intact() {
    run_isolated(
        "an_exit_submitted_after_a_force_close_leaves_the_close_intact",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;
            force_close_position(id, "stuck")
                .await
                .expect("force close commits");
            let closed = stored_position(id).await;
            let booked = in_storage(id).await;

            let error =
                mark_exit_submitted(id, "late-exit-sig", 9.75, "stop_loss_pending_verification")
                    .await
                    .expect_err("a verified row refuses the exit submission");
            assert!(
                matches!(error, Error::AlreadyClosed { position_id } if position_id == id),
                "expected already closed, got {error:?}"
            );

            for position in [stored_position(id).await, memory_position(id).await] {
                assert!(position.transaction_exit_verified);
                assert_eq!(position.closed_reason, closed.closed_reason);
                assert!(position
                    .closed_reason
                    .as_deref()
                    .is_some_and(|reason| reason.starts_with(FORCE_CLOSED_PREFIX)));
                assert_eq!(position.exit_price, closed.exit_price);
                assert_eq!(
                    position.exit_transaction_signature.as_deref(),
                    Some(CLOSE_SIGNATURE)
                );
            }
            assert_unchanged(id, &booked).await;
        },
    );
}

#[test]
fn a_busy_exit_submission_write_is_retried_until_it_lands() {
    run_isolated(
        "a_busy_exit_submission_write_is_retried_until_it_lands",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            let blocker = injector();
            blocker
                .execute_batch("BEGIN IMMEDIATE;")
                .expect("hold the write lock");

            let submission = tokio::spawn(mark_exit_submitted(
                id,
                CLOSE_SIGNATURE,
                1.25,
                "stop_loss_pending_verification",
            ));
            // Held past the store's 5 s busy timeout, so the first attempt fails as busy
            // and only a retry can land the write.
            tokio::time::sleep(std::time::Duration::from_millis(5_500)).await;
            blocker
                .execute_batch("ROLLBACK;")
                .expect("release the write lock");

            submission
                .await
                .expect("submission task")
                .expect("the retried write lands");
            for position in [stored_position(id).await, memory_position(id).await] {
                assert_eq!(
                    position.exit_transaction_signature.as_deref(),
                    Some(CLOSE_SIGNATURE)
                );
                assert_eq!(position.exit_price, Some(1.25));
            }
        },
    );
}

#[test]
fn a_price_update_leaves_the_unrealized_pnl_of_a_closed_position_unchanged() {
    run_isolated(
        "a_price_update_leaves_the_unrealized_pnl_of_a_closed_position_unchanged",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;
            apply_transition(verified_close(id))
                .await
                .expect("the close commits");
            let closed = memory_position(id).await;
            assert!(closed.exit_time.is_some());

            update_position_price_and_pnl(id, common::TEST_MINT, 2.0, PriceSource::Api)
                .await
                .expect("price update");

            let live = memory_position(id).await;
            assert_eq!(live.unrealized_pnl, closed.unrealized_pnl);
            assert_eq!(live.unrealized_pnl_percent, closed.unrealized_pnl_percent);
            let stored = stored_position(id).await;
            assert_eq!(live.unrealized_pnl, stored.unrealized_pnl);
            assert_eq!(live.unrealized_pnl_percent, stored.unrealized_pnl_percent);
            assert_eq!(live.current_price, Some(2.0));
        },
    );
}
