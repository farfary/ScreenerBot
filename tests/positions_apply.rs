// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position bookings and writers against a real positions database: failures change nothing, retries book once, and no writer reverts a committed booking.

mod common;

use chrono::{DateTime, Utc};
use rusqlite::types::Value;
use rusqlite::Connection;
use screenerbot::chains::RawAmount;
use screenerbot::errors::{DatabaseError, ErrorClass};
use screenerbot::positions::apply::apply_transition;
use screenerbot::positions::operations::{force_close_position, mark_exit_submitted};
use screenerbot::positions::price_updater::update_position_price_and_pnl;
use screenerbot::positions::round_state::exit_awaiting_verification;
use screenerbot::positions::transitions::NotLandedEvidence;
use screenerbot::positions::verifier::residual_balance_requires_retry;
use screenerbot::positions::{
    db, state, ApplyFailureDisposition, Error, GiveUpReason, PendingDcaSwap, PendingPartialExit,
    Position, PositionManagement, PositionTransition, PriceSource, VerificationItem,
    VerificationKind, FORCE_CLOSED_PREFIX,
};
use screenerbot::trader::safety::loss_limit::{
    get_loss_limit_status, initialize_from_history, is_entry_blocked_by_loss_limit,
    reset_loss_limit_state,
};

const HELD: u128 = 1_000_000;
const PARTIAL_SIGNATURE: &str = "partial-exit-sig";
const DCA_SIGNATURE: &str = "dca-sig";
const CLOSE_SIGNATURE: &str = "close-sig";

const INJECT_ROW: &str =
    "CREATE TRIGGER inject_row BEFORE UPDATE ON positions BEGIN SELECT RAISE(ABORT,'injected'); END;";
const INJECT_EXIT: &str = "CREATE TRIGGER inject_exit BEFORE INSERT ON position_exits BEGIN SELECT RAISE(ABORT,'injected'); END;";
const INJECT_ENTRY: &str = "CREATE TRIGGER inject_entry BEFORE INSERT ON position_entries BEGIN SELECT RAISE(ABORT,'injected'); END;";
const INJECT_METADATA: &str = "CREATE TRIGGER inject_meta BEFORE INSERT ON position_metadata BEGIN SELECT RAISE(ABORT,'injected'); END;";

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
    store_position(configure).await
}

/// Persists one more OPEN position holding [`HELD`] in the store [`open_position`] opened,
/// and loads it into memory.
async fn store_position(configure: impl FnOnce(&mut Position)) -> i64 {
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
        held_after: None,
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
        held_after: None,
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
    common::run_isolated(
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
    common::run_isolated(
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
    common::run_isolated(
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
    common::run_isolated(
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
    common::run_isolated(
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

            let close = || verified_close(id);

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
    common::run_isolated(
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
    common::run_isolated(
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
    common::run_isolated(
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
    enable_loss_limit_at(1_000.0);
}

/// Enables the period loss limit at `limit_sol`, from a clean slate.
fn enable_loss_limit_at(limit_sol: f64) {
    common::set_config(|cfg| {
        cfg.trader.loss_limit_enabled = true;
        cfg.trader.loss_limit_sol = limit_sol;
        cfg.trader.loss_limit_period_hours = 24;
        cfg.trader.loss_limit_auto_resume = true;
    });
    reset_loss_limit_state();
}

fn recorded_loss() -> f64 {
    get_loss_limit_status().cumulative_loss_native
}

fn failed_close(id: i64) -> PositionTransition {
    PositionTransition::ExitFailedClearForRetry {
        position_id: id,
        exit_signature: CLOSE_SIGNATURE.to_owned(),
    }
}

fn verified_close(id: i64) -> PositionTransition {
    PositionTransition::ExitVerified {
        position_id: id,
        effective_exit_price: 1.5,
        native_received: 1.5,
        fee_raw: 5_000,
        exit_time: Utc::now(),
        exit_signature: CLOSE_SIGNATURE.to_owned(),
        exit_amount: RawAmount::new(HELD),
        held_after: None,
    }
}

#[test]
fn a_price_update_from_a_stale_snapshot_leaves_a_committed_close_intact() {
    common::run_isolated(
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
    common::run_isolated(
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
    common::run_isolated(
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
    common::run_isolated(
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
    common::run_isolated(
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
fn a_failed_sale_after_a_force_close_is_dropped_and_the_write_off_stands() {
    common::run_isolated(
        "a_failed_sale_after_a_force_close_is_dropped_and_the_write_off_stands",
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
            assert_eq!(
                exit_awaiting_verification(&closed),
                Some(CLOSE_SIGNATURE),
                "the sale in flight at the write-off is still to be verified"
            );

            let effects = apply_transition(failed_close(id))
                .await
                .expect("the failed sale is settled on the written-off row");
            assert!(effects.db_updated);

            for position in [stored_position(id).await, memory_position(id).await] {
                assert!(position.transaction_exit_verified);
                assert!(position.synthetic_exit);
                assert!(position.exit_time.is_some());
                assert_eq!(position.closed_reason, closed.closed_reason);
                assert!(position
                    .closed_reason
                    .as_deref()
                    .is_some_and(|reason| reason.starts_with(FORCE_CLOSED_PREFIX)));
                assert_eq!(position.exit_price, closed.exit_price);
                assert_eq!(position.effective_exit_price, closed.effective_exit_price);
                assert_eq!(
                    position.exit_transaction_signature, None,
                    "the failed sale stays on the row"
                );
                assert_eq!(
                    exit_awaiting_verification(&position),
                    None,
                    "a sale the chain proved failed is verified again"
                );
            }
            assert_unchanged(id, &booked).await;

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
fn a_failed_exit_clear_on_a_verified_sale_keeps_the_close_and_its_signature() {
    common::run_isolated(
        "a_failed_exit_clear_on_a_verified_sale_keeps_the_close_and_its_signature",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;
            apply_transition(verified_close(id))
                .await
                .expect("the sale is booked as the close");
            let booked = in_storage(id).await;
            let before = stored_columns(id);

            let error = apply_transition(failed_close(id))
                .await
                .expect_err("a verified close refuses the failed-exit clear");
            assert!(
                matches!(error, Error::AlreadyClosed { position_id } if position_id == id),
                "expected already closed, got {error:?}"
            );
            assert_eq!(stored_columns(id), before, "the verified close changed");
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
        },
    );
}

#[test]
fn a_failed_exit_clear_on_an_unverified_exit_clears_it_for_retry() {
    common::run_isolated(
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

            let effects = apply_transition(failed_close(id))
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
    common::run_isolated(
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
    common::run_isolated(
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
fn a_full_exit_whose_submission_write_failed_is_booked_in_full_at_verification() {
    common::run_isolated(
        "a_full_exit_whose_submission_write_failed_is_booked_in_full_at_verification",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = open_position(|_| {}).await;
            let injector = injector();
            injector.execute_batch(INJECT_ROW).expect("install trigger");
            mark_exit_submitted(id, CLOSE_SIGNATURE, 1.25, "stop_loss_pending_verification")
                .await
                .expect_err("an injected row failure must surface");
            injector
                .execute_batch("DROP TRIGGER inject_row;")
                .expect("remove trigger");

            apply_transition(PositionTransition::ExitVerified {
                position_id: id,
                effective_exit_price: 0.6,
                native_received: 0.6,
                fee_raw: 5_000,
                exit_time: Utc::now(),
                exit_signature: CLOSE_SIGNATURE.to_owned(),
                exit_amount: RawAmount::new(HELD),
                held_after: None,
            })
            .await
            .expect("the close commits");

            let stored = stored_position(id).await;
            assert!(stored.transaction_exit_verified);
            assert_eq!(
                stored.exit_transaction_signature.as_deref(),
                Some(CLOSE_SIGNATURE),
                "the verified exit signature is not on the row"
            );
            assert_eq!(
                memory_position(id)
                    .await
                    .exit_transaction_signature
                    .as_deref(),
                Some(CLOSE_SIGNATURE),
                "memory lost the exit signature"
            );
            assert_eq!(exit_records(id).await, 1, "no exit record was written");
            // Proceeds minus the invested total minus the 5 000-lamport exit fee.
            let expected = 0.6 - 1.0 - 0.000_005;
            let pnl = stored.pnl.expect("a closed row carries its P&L");
            assert!((pnl - expected).abs() < 1e-9, "P&L {pnl}");
            assert!(
                (recorded_loss() - expected.abs()).abs() < 1e-9,
                "the loss limiter recorded {}",
                recorded_loss()
            );
        },
    );
}

#[test]
fn an_exit_submitted_after_a_force_close_leaves_the_close_intact() {
    common::run_isolated(
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
    common::run_isolated(
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
    common::run_isolated(
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

/// Replaces the in-memory position with `stale`, as a transition that read memory before
/// an earlier booking was published would see it.
async fn make_memory_stale(id: i64, stale: Position) {
    assert!(
        state::update_position_state_by_id(id, |live| *live = stale).await,
        "position in memory"
    );
}

fn entry_verified(id: i64) -> PositionTransition {
    PositionTransition::EntryVerified {
        position_id: id,
        effective_entry_price: 1.0,
        token_amount_units: RawAmount::new(HELD),
        fee_raw: 5_000,
        native_size: 1.0,
        held_after: None,
    }
}

#[test]
fn a_close_booked_from_a_stale_snapshot_keeps_a_committed_dca_in_the_row() {
    common::run_isolated(
        "a_close_booked_from_a_stale_snapshot_keeps_a_committed_dca_in_the_row",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;
            let stale = memory_position(id).await;
            register_dca(id).await;
            apply_transition(dca(id)).await.expect("the DCA commits");

            make_memory_stale(id, stale).await;
            apply_transition(verified_close(id))
                .await
                .expect("the close commits");

            let booked = in_storage(id).await;
            assert_eq!(booked, in_memory(id).await, "memory and storage disagree");
            assert!(booked.transaction_exit_verified);
            assert_eq!(booked.dca_count, 1, "the DCA was reverted");
            assert_eq!(booked.total_size_native, 1.5, "the DCA cost was reverted");
            assert_eq!(booked.total_exited_amount, RawAmount::new(1_500_000));
            assert_eq!(booked.remaining_token_amount, Some(RawAmount::ZERO));
            assert_eq!(entry_records(id).await, 1);
            assert_eq!(exit_records(id).await, 1);
        },
    );
}

#[test]
fn a_force_close_from_a_stale_snapshot_keeps_a_committed_partial_exit() {
    common::run_isolated(
        "a_force_close_from_a_stale_snapshot_keeps_a_committed_partial_exit",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            let stale = memory_position(id).await;
            register_partial(id).await;
            apply_transition(partial_exit(id))
                .await
                .expect("the partial exit commits");

            make_memory_stale(id, stale).await;
            force_close_position(id, "stuck")
                .await
                .expect("the force close commits");

            let booked = in_storage(id).await;
            assert_eq!(booked, in_memory(id).await, "memory and storage disagree");
            assert!(booked.transaction_exit_verified);
            assert_eq!(booked.partial_exit_count, 1);
            assert_eq!(
                booked.native_received,
                Some(0.8),
                "the partial exit's proceeds were reverted"
            );
            assert_eq!(booked.total_exited_amount, RawAmount::new(HELD));
            assert_eq!(booked.remaining_token_amount, Some(RawAmount::ZERO));
            let pnl = booked.pnl.expect("a written-off row carries its P&L");
            assert!((pnl - (0.8 - 1.0)).abs() < 1e-9, "P&L {pnl}");
            assert_eq!(exit_records(id).await, 1);
        },
    );
}

#[test]
fn a_dca_from_a_stale_snapshot_keeps_a_committed_partial_exit() {
    common::run_isolated(
        "a_dca_from_a_stale_snapshot_keeps_a_committed_partial_exit",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            let stale = memory_position(id).await;
            register_partial(id).await;
            apply_transition(partial_exit(id))
                .await
                .expect("the partial exit commits");

            make_memory_stale(id, stale).await;
            register_dca(id).await;
            apply_transition(dca(id)).await.expect("the DCA commits");

            let booked = in_storage(id).await;
            assert_eq!(booked, in_memory(id).await, "memory and storage disagree");
            assert_eq!(
                booked.partial_exit_count, 1,
                "the partial exit was reverted"
            );
            assert_eq!(booked.native_received, Some(0.8));
            assert_eq!(booked.total_exited_amount, RawAmount::new(400_000));
            assert_eq!(
                booked.remaining_token_amount,
                Some(RawAmount::new(1_100_000))
            );
            assert_eq!(booked.dca_count, 1);
            assert_eq!(booked.total_size_native, 1.5);
            assert_eq!(exit_records(id).await, 1);
            assert_eq!(entry_records(id).await, 1);
        },
    );
}

#[test]
fn an_entry_verified_twice_books_once() {
    common::run_isolated("an_entry_verified_twice_books_once", || async {
        let _dir = common::isolated_env();
        let _cfg = common::config_guard();
        let id = open_position(|position| {
            position.transaction_entry_verified = false;
        })
        .await;
        apply_transition(entry_verified(id))
            .await
            .expect("the entry commits");
        register_partial(id).await;
        apply_transition(partial_exit(id))
            .await
            .expect("the partial exit commits");
        let booked = in_storage(id).await;

        apply_transition(entry_verified(id))
            .await
            .expect("a repeated entry verification is a no-op");

        assert_eq!(in_storage(id).await, booked, "the entry was booked twice");
        assert_eq!(in_memory(id).await, booked, "memory changed");
        assert_eq!(booked.remaining_token_amount, Some(RawAmount::new(600_000)));
        assert_eq!(entry_records(id).await, 1);
    });
}

#[test]
fn memory_adopts_the_committed_row_in_commit_order() {
    common::run_isolated(
        "memory_adopts_the_committed_row_in_commit_order",
        || async {
            const ADDS: u32 = 8;
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;

            let mut bookings = tokio::task::JoinSet::new();
            for add in 0..ADDS {
                bookings.spawn(apply_transition(PositionTransition::DcaVerified {
                    position_id: id,
                    tokens_bought: RawAmount::new(500_000),
                    native_spent: 0.5,
                    effective_price: 1.0,
                    fee_raw: 5_000,
                    dca_time: Utc::now(),
                    dca_signature: format!("{DCA_SIGNATURE}-{add}"),
                    held_after: None,
                }));
            }
            while let Some(booked) = bookings.join_next().await {
                booked.expect("booking task").expect("the DCA commits");
            }

            let booked = in_storage(id).await;
            assert_eq!(booked, in_memory(id).await, "memory and storage disagree");
            assert_eq!(booked.dca_count, ADDS, "a concurrent DCA was lost");
            assert_eq!(booked.total_size_native, 1.0 + 0.5 * f64::from(ADDS));
            assert_eq!(
                booked.remaining_token_amount,
                Some(RawAmount::new(HELD + 500_000 * u128::from(ADDS)))
            );
            assert_eq!(entry_records(id).await, ADDS as usize);
        },
    );
}

#[test]
fn adopting_a_committed_row_keeps_the_columns_other_writers_own() {
    common::run_isolated(
        "adopting_a_committed_row_keeps_the_columns_other_writers_own",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            let priced_at = Utc::now();
            let archived_at = Utc::now();
            // Writes that landed after the booking below read its row: a price tick and
            // the operator archiving the position and taking it over.
            assert!(
                state::update_position_state_by_id(id, |live| {
                    live.current_price = Some(2.5);
                    live.current_price_updated = Some(priced_at);
                    live.current_price_source = Some(PriceSource::Api);
                    live.price_highest = 3.0;
                    live.price_lowest = 0.5;
                    live.archived = true;
                    live.archived_at = Some(archived_at);
                    live.management = PositionManagement::UserOnly;
                })
                .await,
                "position in memory"
            );

            register_dca(id).await;
            apply_transition(dca(id)).await.expect("the DCA commits");

            let live = memory_position(id).await;
            assert_eq!(live.dca_count, 1, "memory did not adopt the DCA");
            assert_eq!(live.current_price, Some(2.5), "the live price was reverted");
            assert_eq!(live.current_price_updated, Some(priced_at));
            assert_eq!(live.current_price_source, Some(PriceSource::Api));
            assert_eq!(live.price_highest, 3.0);
            assert_eq!(live.price_lowest, 0.5);
            assert!(live.archived, "the archive flag was reverted");
            assert_eq!(live.archived_at, Some(archived_at));
            assert_eq!(
                live.management,
                PositionManagement::UserOnly,
                "the management mode was reverted"
            );
        },
    );
}

#[test]
fn an_exit_submission_waits_for_a_committed_booking_to_be_adopted() {
    common::run_isolated(
        "an_exit_submission_waits_for_a_committed_booking_to_be_adopted",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            register_dca(id).await;

            // Holding memory keeps the DCA between its commit and its publish.
            let memory = state::POSITIONS.read().await;
            let booking = tokio::spawn(apply_transition(dca(id)));
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while in_storage(id).await.dca_count == 0 {
                assert!(
                    std::time::Instant::now() < deadline,
                    "the DCA never committed"
                );
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }

            let submission = tokio::spawn(async move {
                mark_exit_submitted(id, CLOSE_SIGNATURE, 1.25, "stop_loss_pending_verification")
                    .await
            });
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            assert_eq!(
                stored_position(id).await.exit_transaction_signature,
                None,
                "the exit submission was written while a booking that read the row before it was unpublished"
            );

            drop(memory);
            booking
                .await
                .expect("booking task")
                .expect("the DCA commits");
            submission
                .await
                .expect("submission task")
                .expect("the exit submission is written");

            let live = memory_position(id).await;
            assert_eq!(
                live.exit_transaction_signature.as_deref(),
                Some(CLOSE_SIGNATURE),
                "memory lost the exit in flight"
            );
            assert_eq!(
                stored_position(id)
                    .await
                    .exit_transaction_signature
                    .as_deref(),
                Some(CLOSE_SIGNATURE)
            );
            assert_eq!(in_storage(id).await, in_memory(id).await);
            assert_eq!(live.dca_count, 1);
        },
    );
}

// ==================== ENTRY NOT LANDED ====================

const ORPHAN_ENTRY: &str = "orphan-entry-sig";

/// A position whose entry `signature` was submitted and never verified.
fn unverified_entry(signature: &str) -> impl FnOnce(&mut Position) + '_ {
    move |position| {
        position.entry_transaction_signature = Some(signature.to_owned());
        position.transaction_entry_verified = false;
    }
}

fn orphan_removal(id: i64, signature: &str) -> PositionTransition {
    PositionTransition::RemoveOrphanEntry {
        position_id: id,
        signature: signature.to_owned(),
        evidence: NotLandedEvidence::Expired,
    }
}

async fn stored(id: i64) -> bool {
    db::get_position_by_id(id)
        .await
        .expect("read stored position")
        .is_some()
}

/// The rows of `table` that belong to position `id`.
fn child_rows(table: &str, id: i64) -> i64 {
    injector()
        .query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE position_id = ?1"),
            [id],
            |row| row.get(0),
        )
        .expect("count child rows")
}

/// The removal of `id` as a not-landed entry is refused, and the row and its records stay.
async fn assert_removal_refused(id: i64) {
    let entries = entry_records(id).await;
    let exits = exit_records(id).await;
    let error = apply_transition(orphan_removal(id, ORPHAN_ENTRY))
        .await
        .expect_err("the removal is refused");
    assert!(
        matches!(
            &error,
            Error::EntryLanded { position_id, signature }
                if *position_id == id && signature == ORPHAN_ENTRY
        ),
        "expected entry landed, got {error:?}"
    );
    assert!(!error.is_retryable());
    assert!(state::get_position_by_id(id).await.is_some());
    assert!(stored(id).await);
    assert_eq!(entry_records(id).await, entries, "entry records changed");
    assert_eq!(exit_records(id).await, exits, "exit records changed");
}

#[test]
fn an_orphan_removal_targets_its_own_id_not_the_first_position_of_the_mint() {
    common::run_isolated(
        "an_orphan_removal_targets_its_own_id_not_the_first_position_of_the_mint",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let earlier = open_position(|position| {
                position.entry_transaction_signature = Some("earlier-entry".to_owned());
                position.exit_transaction_signature = Some("earlier-exit".to_owned());
                position.transaction_exit_verified = true;
                position.exit_time = Some(Utc::now());
            })
            .await;
            let orphan = store_position(unverified_entry(ORPHAN_ENTRY)).await;

            let effects = apply_transition(orphan_removal(orphan, ORPHAN_ENTRY))
                .await
                .expect("the orphan is removed");
            assert!(effects.position_removed);

            assert!(state::get_position_by_id(orphan).await.is_none());
            assert!(!stored(orphan).await, "the orphan row is deleted");
            assert!(
                state::get_position_by_id(earlier).await.is_some(),
                "the earlier round of the mint stays in memory"
            );
            assert!(stored(earlier).await, "the earlier round stays stored");
        },
    );
}

#[test]
fn an_orphan_removal_deletes_the_row_and_releases_the_slot_once() {
    common::run_isolated(
        "an_orphan_removal_deletes_the_row_and_releases_the_slot_once",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            state::init_global_position_semaphore(1);
            let id = open_position(unverified_entry(ORPHAN_ENTRY)).await;
            assert!(state::try_consume_global_position_permit());
            state::register_position_slot(id).await;
            assert!(!state::try_consume_global_position_permit());
            let injector = injector();
            injector
                .execute(
                    "INSERT INTO position_states (position_id, state) VALUES (?1, 'Open')",
                    [id],
                )
                .expect("record a state");
            injector
                .execute(
                    "INSERT INTO position_tracking (position_id, price, price_source) VALUES (?1, 1.0, 'pool')",
                    [id],
                )
                .expect("record a price");

            apply_transition(orphan_removal(id, ORPHAN_ENTRY))
                .await
                .expect("the orphan is removed");
            assert!(
                state::try_consume_global_position_permit(),
                "the removal frees the slot"
            );
            assert!(
                !state::try_consume_global_position_permit(),
                "exactly one slot came back"
            );

            let error = apply_transition(orphan_removal(id, ORPHAN_ENTRY))
                .await
                .expect_err("a removed row cannot be removed again");
            assert!(
                matches!(error, Error::NotFoundById { position_id } if position_id == id),
                "expected not found, got {error:?}"
            );
            assert!(
                !state::try_consume_global_position_permit(),
                "a repeated removal frees no second slot"
            );

            assert!(state::get_position_by_id(id).await.is_none());
            assert!(!stored(id).await);
            assert!(
                db::load_all_positions()
                    .await
                    .expect("reload the store")
                    .iter()
                    .all(|position| position.id != Some(id)),
                "a reload finds no row to verify again"
            );
            for table in ["position_states", "position_tracking"] {
                assert_eq!(child_rows(table, id), 0, "the delete cascades to {table}");
            }
            assert_eq!(recorded_loss(), 0.0, "a removal records no loss");
        },
    );
}

#[test]
fn an_orphan_removal_refuses_a_position_whose_entry_landed() {
    common::run_isolated(
        "an_orphan_removal_refuses_a_position_whose_entry_landed",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(unverified_entry(ORPHAN_ENTRY)).await;

            let refused = |error: Error| {
                assert!(
                    matches!(
                        &error,
                        Error::EntryLanded { position_id, signature }
                            if *position_id == id && signature == ORPHAN_ENTRY
                    ),
                    "expected entry landed, got {error:?}"
                );
                assert!(!error.is_retryable());
            };

            // The row's entry is a different signature.
            let error = apply_transition(orphan_removal(id, "another-sig"))
                .await
                .expect_err("a removal for another signature is refused");
            assert!(
                matches!(error, Error::EntryLanded { position_id, .. } if position_id == id),
                "expected entry landed, got {error:?}"
            );

            apply_transition(PositionTransition::EntryVerified {
                position_id: id,
                effective_entry_price: 1.0,
                token_amount_units: RawAmount::new(HELD),
                fee_raw: 5_000,
                native_size: 1.0,
                held_after: None,
            })
            .await
            .expect("the entry is booked");
            assert_eq!(entry_records(id).await, 1);

            // The row is verified.
            refused(
                apply_transition(orphan_removal(id, ORPHAN_ENTRY))
                    .await
                    .expect_err("a verified entry is refused"),
            );

            // Only the entry record proves the entry landed.
            injector()
                .execute(
                    "UPDATE positions SET transaction_entry_verified = 0 WHERE id = ?1",
                    [id],
                )
                .expect("clear the verified flag");
            refused(
                apply_transition(orphan_removal(id, ORPHAN_ENTRY))
                    .await
                    .expect_err("a recorded entry is refused"),
            );

            assert!(state::get_position_by_id(id).await.is_some());
            assert!(stored(id).await);
            assert_eq!(entry_records(id).await, 1);
        },
    );
}

#[test]
fn an_orphan_removal_refuses_a_position_with_a_booked_dca() {
    common::run_isolated(
        "an_orphan_removal_refuses_a_position_with_a_booked_dca",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(unverified_entry(ORPHAN_ENTRY)).await;
            register_dca(id).await;
            apply_transition(dca(id)).await.expect("the DCA is booked");
            assert_eq!(entry_records(id).await, 1);

            assert_removal_refused(id).await;
            assert_eq!(in_storage(id).await.dca_count, 1, "the DCA stays booked");

            // The DCA's record alone refuses the removal.
            injector()
                .execute("UPDATE positions SET dca_count = 0 WHERE id = ?1", [id])
                .expect("clear the DCA count");
            assert_removal_refused(id).await;
            assert_eq!(entry_records(id).await, 1, "the DCA record survives");
        },
    );
}

/// A DCA or partial exit in flight is on no row column or record until its verification
/// books it, so only its pending entry proves a fill may still land on the row. The slot the
/// row holds stays held while the removal is refused.
#[test]
fn an_orphan_removal_refuses_a_position_with_a_dca_or_partial_exit_in_flight() {
    common::run_isolated(
        "an_orphan_removal_refuses_a_position_with_a_dca_or_partial_exit_in_flight",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            state::init_global_position_semaphore(2);
            let with_dca = open_position(unverified_entry(ORPHAN_ENTRY)).await;
            let with_partial = store_position(unverified_entry(ORPHAN_ENTRY)).await;
            for id in [with_dca, with_partial] {
                assert!(state::try_consume_global_position_permit());
                state::register_position_slot(id).await;
            }
            register_dca(with_dca).await;
            register_partial(with_partial).await;

            for id in [with_dca, with_partial] {
                let before = in_storage(id).await;
                assert_removal_refused(id).await;
                assert_eq!(in_storage(id).await, before, "the row is unchanged");
            }
            assert!(dca_pending().await, "the pending DCA is kept");
            assert!(partial_pending().await, "the pending partial exit is kept");
            assert!(
                !state::try_consume_global_position_permit(),
                "both slots stay held"
            );

            // The pending entries of another position of the mint refuse nothing.
            let unrelated = store_position(unverified_entry(ORPHAN_ENTRY)).await;
            apply_transition(orphan_removal(unrelated, ORPHAN_ENTRY))
                .await
                .expect("a row with no fill in flight is removed");
            assert!(!stored(unrelated).await);
        },
    );
}

#[test]
fn an_orphan_removal_refuses_a_position_with_an_exit_submitted() {
    common::run_isolated(
        "an_orphan_removal_refuses_a_position_with_an_exit_submitted",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let full_exit = open_position(|position| {
                unverified_entry(ORPHAN_ENTRY)(position);
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;
            let partial_exit = store_position(|position| {
                unverified_entry(ORPHAN_ENTRY)(position);
                position.partial_exit_count = 1;
            })
            .await;

            assert_removal_refused(full_exit).await;
            assert_removal_refused(partial_exit).await;
        },
    );
}

#[test]
fn an_orphan_removal_refuses_a_position_whose_exit_is_verified() {
    common::run_isolated(
        "an_orphan_removal_refuses_a_position_whose_exit_is_verified",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|position| {
                unverified_entry(ORPHAN_ENTRY)(position);
                position.transaction_exit_verified = true;
            })
            .await;

            assert_removal_refused(id).await;
        },
    );
}

#[test]
fn removing_a_row_keeps_the_signatures_of_other_rows_of_the_mint() {
    common::run_isolated(
        "removing_a_row_keeps_the_signatures_of_other_rows_of_the_mint",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let kept = open_position(|position| {
                position.entry_transaction_signature = Some("kept-entry".to_owned());
                position.exit_transaction_signature = Some("kept-exit".to_owned());
            })
            .await;
            state::add_signature_to_index(PARTIAL_SIGNATURE, common::TEST_MINT).await;
            let removed = store_position(unverified_entry(ORPHAN_ENTRY)).await;

            assert!(state::remove_position_by_id(removed).await.is_some());
            assert_eq!(state::get_mint_by_signature(ORPHAN_ENTRY).await, None);
            for signature in ["kept-entry", "kept-exit", PARTIAL_SIGNATURE] {
                assert_eq!(
                    state::get_mint_by_signature(signature).await.as_deref(),
                    Some(common::TEST_MINT),
                    "{signature} stays indexed"
                );
            }

            assert!(state::remove_position_by_id(kept).await.is_some());
            for signature in ["kept-entry", "kept-exit", PARTIAL_SIGNATURE] {
                assert_eq!(
                    state::get_mint_by_signature(signature).await,
                    None,
                    "{signature} leaves with the mint's last row"
                );
            }
        },
    );
}

#[test]
fn removing_a_row_never_drops_a_held_mint_lock() {
    common::run_isolated("removing_a_row_never_drops_a_held_mint_lock", || async {
        let _dir = common::isolated_env();
        let _cfg = common::config_guard();
        let id = open_position(|_| {}).await;
        let held = state::acquire_position_lock(common::TEST_MINT).await;

        assert!(state::remove_position_by_id(id).await.is_some());
        let wait = std::time::Duration::from_millis(200);
        assert!(
            tokio::time::timeout(wait, state::acquire_position_lock(common::TEST_MINT))
                .await
                .is_err(),
            "a second holder acquired the mint lock while the first held it"
        );

        drop(held);
        assert!(
            tokio::time::timeout(wait, state::acquire_position_lock(common::TEST_MINT))
                .await
                .is_ok(),
            "the lock is free once its holder drops it"
        );
    });
}

// ==================== LATE FILLS ON CLOSED POSITIONS ====================

/// The fee of every verified swap in these tests, in raw native units.
const SWAP_FEE_RAW: u64 = 5_000;

/// A fee in native units.
fn fee(raw: u64) -> f64 {
    raw as f64 / 1e9
}

/// Opens an OPEN position holding [`HELD`] for 1 SOL, configured by `configure`, holding a
/// trading slot of a one-slot semaphore, and writes it off by force close.
async fn written_off(configure: impl FnOnce(&mut Position)) -> i64 {
    state::init_global_position_semaphore(1);
    let id = open_position(configure).await;
    assert!(state::try_consume_global_position_permit());
    state::register_position_slot(id).await;
    force_close_position(id, "stuck")
        .await
        .expect("force close commits");
    assert!(
        state::try_consume_global_position_permit(),
        "the write-off frees the slot"
    );
    state::release_global_position_permit();
    id
}

fn late_sell(id: i64, held_after: Option<RawAmount>) -> PositionTransition {
    PositionTransition::ExitVerified {
        position_id: id,
        effective_exit_price: 1.5,
        native_received: 1.5,
        fee_raw: SWAP_FEE_RAW,
        exit_time: Utc::now(),
        exit_signature: CLOSE_SIGNATURE.to_owned(),
        exit_amount: RawAmount::new(HELD),
        held_after,
    }
}

fn late_partial(id: i64, held_after: Option<RawAmount>) -> PositionTransition {
    match partial_exit(id) {
        PositionTransition::PartialExitVerified {
            position_id,
            exit_amount,
            native_received,
            effective_exit_price,
            fee_raw,
            exit_time,
            exit_signature,
            exit_percentage,
            ..
        } => PositionTransition::PartialExitVerified {
            position_id,
            exit_amount,
            native_received,
            effective_exit_price,
            fee_raw,
            exit_time,
            exit_signature,
            exit_percentage,
            held_after,
        },
        _ => unreachable!("a partial exit"),
    }
}

fn late_dca(id: i64, held_after: Option<RawAmount>) -> PositionTransition {
    match dca(id) {
        PositionTransition::DcaVerified {
            position_id,
            tokens_bought,
            native_spent,
            effective_price,
            fee_raw,
            dca_time,
            dca_signature,
            ..
        } => PositionTransition::DcaVerified {
            position_id,
            tokens_bought,
            native_spent,
            effective_price,
            fee_raw,
            dca_time,
            dca_signature,
            held_after,
        },
        _ => unreachable!("a DCA"),
    }
}

/// The acquired amount equals what is held plus what has exited, in memory and storage.
async fn assert_acquired_balances(id: i64, acquired: u128) {
    for position in [stored_position(id).await, memory_position(id).await] {
        let held = position.remaining_token_amount.unwrap_or_default();
        assert_eq!(
            held.raw() + position.total_exited_amount.raw(),
            acquired,
            "held {held} + exited {} != acquired {acquired}",
            position.total_exited_amount
        );
    }
}

/// Turns event recording on and starts the events store.
async fn start_events() {
    common::set_config(|cfg| cfg.events.enabled = true);
    screenerbot::paths::ensure_all_directories().expect("create data directories");
    screenerbot::events::init().await.expect("start events");
}

/// The position events of the test mint with `subtype`, once the events writer flushed.
async fn position_events(subtype: &str) -> usize {
    tokio::time::sleep(std::time::Duration::from_millis(2_500)).await;
    screenerbot::events::by_mint(common::TEST_MINT, 500)
        .await
        .expect("read events")
        .iter()
        .filter(|event| event.subtype.as_deref() == Some(subtype))
        .count()
}

#[test]
fn a_sell_verified_after_a_force_close_books_its_proceeds_and_restates_the_loss() {
    common::run_isolated(
        "a_sell_verified_after_a_force_close_books_its_proceeds_and_restates_the_loss",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            start_events().await;
            let id = written_off(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;
            assert_eq!(recorded_loss(), 1.0);

            apply_transition(late_sell(id, Some(RawAmount::ZERO)))
                .await
                .expect("the late sell is booked");

            let exits = db::get_exit_history(id).await.expect("read exits");
            assert_eq!(exits.len(), 1, "one exit record for the sale");
            assert_eq!(exits[0].amount, RawAmount::new(HELD), "the sold amount");
            assert!(!exits[0].is_partial);
            let booked = in_storage(id).await;
            assert_eq!(booked, in_memory(id).await, "memory and storage disagree");
            assert_eq!(booked.native_received, Some(1.5));
            assert_eq!(booked.total_exited_amount, RawAmount::new(HELD));
            assert_eq!(booked.remaining_token_amount, Some(RawAmount::ZERO));
            assert!(booked.transaction_exit_verified && booked.exit_time.is_some());
            let expected_pnl = 1.5 - 1.0 - fee(SWAP_FEE_RAW);
            assert!(
                (booked.pnl.expect("a realized P&L") - expected_pnl).abs() < 1e-12,
                "pnl {:?} is not the booked legs' {expected_pnl}",
                booked.pnl
            );
            assert_eq!(recorded_loss(), 0.0, "the loss is restated away");
            assert_eq!(position_events("fill_after_force_close").await, 1);
        },
    );
}

#[test]
fn a_sale_in_flight_at_a_write_off_is_verified_after_a_restart_and_booked_once() {
    common::run_isolated(
        "a_sale_in_flight_at_a_write_off_is_verified_after_a_restart_and_booked_once",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = written_off(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;

            screenerbot::positions::initialize_positions_system()
                .await
                .expect("the positions system restarts");
            let (_, queued) = screenerbot::positions::queue::get_queue_status().await;
            assert!(
                queued.iter().any(|signature| signature == CLOSE_SIGNATURE),
                "the restart lost the sale in flight at the write-off: {queued:?}"
            );

            for _ in 0..2 {
                apply_transition(late_sell(id, Some(RawAmount::ZERO)))
                    .await
                    .expect("the late sell is booked");
            }
            assert_eq!(exit_records(id).await, 1, "the sale is booked once");
            let booked = in_storage(id).await;
            assert_eq!(booked, in_memory(id).await, "memory and storage disagree");
            assert_eq!(booked.native_received, Some(1.5));
            assert_eq!(recorded_loss(), 0.0, "the loss is restated away");
            for position in [stored_position(id).await, memory_position(id).await] {
                assert!(!position.synthetic_exit);
                assert_eq!(
                    exit_awaiting_verification(&position),
                    None,
                    "a booked sale is verified again"
                );
            }
        },
    );
}

#[test]
fn a_sale_with_a_residual_on_a_written_off_row_is_booked_and_not_retried() {
    common::run_isolated(
        "a_sale_with_a_residual_on_a_written_off_row_is_booked_and_not_retried",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = written_off(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;

            let residual = PositionTransition::ExitResidualClearForRetry {
                position_id: id,
                exit_amount: RawAmount::new(HELD / 2),
                native_received: 0.75,
                effective_exit_price: 1.5,
                fee_raw: SWAP_FEE_RAW,
                exit_time: Utc::now(),
                exit_signature: CLOSE_SIGNATURE.to_owned(),
                exit_percentage: 50.0,
                held_after: Some(RawAmount::ZERO),
            };
            apply_transition(residual)
                .await
                .expect("the sale is booked and nothing is left to retry");

            assert_eq!(exit_records(id).await, 1, "the sale's leg is booked once");
            let booked = in_storage(id).await;
            assert_eq!(booked, in_memory(id).await, "memory and storage disagree");
            assert_eq!(booked.native_received, Some(0.75));
            for position in [stored_position(id).await, memory_position(id).await] {
                assert!(position.transaction_exit_verified && position.exit_time.is_some());
                assert_eq!(
                    exit_awaiting_verification(&position),
                    None,
                    "a booked sale is verified again"
                );
            }
        },
    );
}

#[test]
fn a_full_exit_residual_counts_only_the_positions_own_tokens() {
    common::run_isolated(
        "a_full_exit_residual_counts_only_the_positions_own_tokens",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|_| {}).await;
            store_position(|_| {}).await;

            assert!(
                !residual_balance_requires_retry(Some(id), RawAmount::new(HELD)).await,
                "the other position's tokens are taken for a residual of this one"
            );
            assert!(
                residual_balance_requires_retry(Some(id), RawAmount::new(HELD + HELD / 2)).await,
                "a residual of the position's own is missed beside another position"
            );
        },
    );
}

#[test]
fn a_partial_exit_after_a_close_keeps_acquired_equal_to_held_plus_exited() {
    common::run_isolated(
        "a_partial_exit_after_a_close_keeps_acquired_equal_to_held_plus_exited",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = written_off(|_| {}).await;

            apply_transition(late_partial(id, Some(RawAmount::ZERO)))
                .await
                .expect("the late partial exit is booked");

            let booked = in_storage(id).await;
            assert_eq!(booked, in_memory(id).await, "memory and storage disagree");
            assert_eq!(booked.native_received, Some(0.8));
            assert_eq!(booked.partial_exit_count, 1);
            assert!(booked.exit_time.is_some(), "the position stays closed");
            assert_acquired_balances(id, HELD).await;
            assert_eq!(exit_records(id).await, 1);
            assert!((recorded_loss() - 0.2).abs() < 1e-12);
        },
    );
}

#[test]
fn a_dca_after_a_write_off_with_tokens_held_reopens_the_position() {
    common::run_isolated(
        "a_dca_after_a_write_off_with_tokens_held_reopens_the_position",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            start_events().await;
            let id = written_off(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
                position.management = PositionManagement::UserOnly;
            })
            .await;
            assert_eq!(recorded_loss(), 1.0);
            let price_before = memory_position(id).await.current_price;

            apply_transition(late_dca(id, Some(RawAmount::new(HELD + 500_000))))
                .await
                .expect("the late DCA is booked");

            for position in [stored_position(id).await, memory_position(id).await] {
                assert!(position.exit_time.is_none(), "the position is open again");
                assert!(!position.transaction_exit_verified);
                assert!(!position.synthetic_exit);
                assert_eq!(position.exit_transaction_signature, None);
                assert_eq!(position.closed_reason, None);
                assert_eq!(position.pnl, None);
                assert_eq!(position.management, PositionManagement::UserOnly);
                assert!(!position.archived);
                assert_eq!(
                    position.remaining_token_amount,
                    Some(RawAmount::new(HELD + 500_000))
                );
                assert_eq!(position.total_exited_amount, RawAmount::ZERO);
                assert_eq!(position.total_size_native, 1.5);
                assert_eq!(position.dca_count, 1);
                assert!(
                    (position.average_entry_price - 1.0).abs() < 1e-12,
                    "the cost is averaged over what is held, got {}",
                    position.average_entry_price
                );
            }
            assert_eq!(memory_position(id).await.current_price, price_before);
            assert!(
                state::get_open_positions()
                    .await
                    .iter()
                    .any(|position| position.id == Some(id)),
                "the position is in the open list"
            );
            assert!(
                !state::try_consume_global_position_permit(),
                "the reopened position took the free slot"
            );
            assert_eq!(recorded_loss(), 0.0, "an open position realizes no loss");
            assert_eq!(entry_records(id).await, 1);
            assert_eq!(position_events("fill_after_force_close").await, 1);
            assert_eq!(position_events("dca_verified").await, 1);
        },
    );
}

#[test]
fn a_dca_whose_tokens_the_close_sold_stays_closed_and_restates_the_loss() {
    common::run_isolated(
        "a_dca_whose_tokens_the_close_sold_stays_closed_and_restates_the_loss",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = written_off(|_| {}).await;

            apply_transition(late_dca(id, Some(RawAmount::ZERO)))
                .await
                .expect("the late DCA is booked");

            let booked = in_storage(id).await;
            assert_eq!(booked, in_memory(id).await, "memory and storage disagree");
            assert!(booked.exit_time.is_some(), "the position stays closed");
            assert_eq!(booked.total_size_native, 1.5);
            assert_eq!(booked.remaining_token_amount, Some(RawAmount::ZERO));
            assert_acquired_balances(id, HELD + 500_000).await;
            assert_eq!(booked.pnl, Some(-1.5));
            assert_eq!(recorded_loss(), 1.5);
            assert!(
                state::try_consume_global_position_permit(),
                "a position that stays closed takes no slot"
            );
        },
    );
}

#[test]
fn an_entry_after_a_write_off_reopens_with_the_real_amount() {
    common::run_isolated(
        "an_entry_after_a_write_off_reopens_with_the_real_amount",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = written_off(|position| {
                position.transaction_entry_verified = false;
                position.token_amount = None;
                position.remaining_token_amount = None;
            })
            .await;

            apply_transition(PositionTransition::EntryVerified {
                position_id: id,
                effective_entry_price: 0.9 / 0.7,
                token_amount_units: RawAmount::new(700_000),
                fee_raw: SWAP_FEE_RAW,
                native_size: 0.9,
                held_after: Some(RawAmount::new(700_000)),
            })
            .await
            .expect("the late entry is booked");

            for position in [stored_position(id).await, memory_position(id).await] {
                assert!(position.exit_time.is_none(), "the position is open again");
                assert!(position.transaction_entry_verified);
                assert_eq!(position.token_amount, Some(RawAmount::new(700_000)));
                assert_eq!(
                    position.remaining_token_amount,
                    Some(RawAmount::new(700_000))
                );
                assert_eq!(position.total_exited_amount, RawAmount::ZERO);
                assert_eq!(position.total_size_native, 0.9);
            }
            assert_eq!(entry_records(id).await, 1);
            assert_eq!(recorded_loss(), 0.0);
        },
    );
}

#[test]
fn an_archived_row_reopened_by_a_late_buy_stays_archived_and_takes_no_slot() {
    common::run_isolated(
        "an_archived_row_reopened_by_a_late_buy_stays_archived_and_takes_no_slot",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = written_off(|_| {}).await;
            // Archiving is the user's, outside any booking: set on the row and in memory.
            injector()
                .execute(
                    "UPDATE positions SET archived = 1, archived_at = ?2 WHERE id = ?1",
                    rusqlite::params![id, Utc::now().to_rfc3339()],
                )
                .expect("archive the row");
            assert!(state::update_position_state_by_id(id, |live| live.archived = true).await);

            apply_transition(late_dca(id, Some(RawAmount::new(HELD + 500_000))))
                .await
                .expect("the late DCA is booked");

            for position in [stored_position(id).await, memory_position(id).await] {
                assert!(position.exit_time.is_none(), "the position is open again");
                assert!(position.archived, "the position stays archived");
            }
            assert!(
                !state::get_open_positions()
                    .await
                    .iter()
                    .any(|position| position.id == Some(id)),
                "an archived position stays out of the open list"
            );
            assert!(
                state::try_consume_global_position_permit(),
                "an archived position takes no slot"
            );
        },
    );
}

#[test]
fn a_late_fill_without_a_balance_reading_is_requeued_not_guessed() {
    common::run_isolated(
        "a_late_fill_without_a_balance_reading_is_requeued_not_guessed",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = written_off(|_| {}).await;
            let before = in_storage(id).await;

            for transition in [
                late_dca(id, None),
                late_partial(id, None),
                late_sell(id, None),
            ] {
                let error = apply_transition(transition)
                    .await
                    .expect_err("a late fill needs the holding after it");
                assert!(
                    matches!(
                        error,
                        Error::Chain(screenerbot::chains::Error::SettlementRead { .. })
                    ),
                    "expected a settlement read error, got {error:?}"
                );
                assert!(error.is_retryable());
                let item = VerificationItem::new_dca(
                    DCA_SIGNATURE.to_owned(),
                    common::TEST_MINT.to_owned(),
                    Some(id),
                    None,
                );
                assert!(matches!(
                    item.apply_failure_disposition(&error),
                    ApplyFailureDisposition::Requeue
                ));
            }
            assert_unchanged(id, &before).await;
            assert_eq!(entry_records(id).await, 0);
            assert_eq!(exit_records(id).await, 0);
            assert_eq!(recorded_loss(), 1.0);
        },
    );
}

/// Asserts every late fill on `id` is refused retryably, with nothing booked, because the
/// wallet's holding cannot be split between the positions of the mint yet.
async fn assert_late_fills_wait(id: i64, held_after: RawAmount) {
    let before = in_storage(id).await;
    for transition in [
        late_dca(id, Some(held_after)),
        late_partial(id, Some(held_after)),
        late_sell(id, Some(held_after)),
    ] {
        let error = apply_transition(transition)
            .await
            .expect_err("the holding cannot be attributed yet");
        assert!(
            matches!(error, Error::HoldingUnattributable { .. }),
            "expected an unattributable holding, got {error:?}"
        );
        let item = VerificationItem::new_dca(
            DCA_SIGNATURE.to_owned(),
            common::TEST_MINT.to_owned(),
            Some(id),
            None,
        );
        assert!(matches!(
            item.apply_failure_disposition(&error),
            ApplyFailureDisposition::Requeue
        ));
    }
    assert_unchanged(id, &before).await;
    assert_eq!(entry_records(id).await, 0);
    assert_eq!(exit_records(id).await, 0);
}

#[test]
fn a_late_fill_waits_while_another_position_of_the_mint_has_an_unverified_entry() {
    common::run_isolated(
        "a_late_fill_waits_while_another_position_of_the_mint_has_an_unverified_entry",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = written_off(|_| {}).await;
            // A bot entry's row holds no amount until its entry is verified.
            let sibling = store_position(|position| {
                unverified_entry("sibling-entry-sig")(position);
                position.token_amount = None;
                position.remaining_token_amount = None;
            })
            .await;

            assert_late_fills_wait(id, RawAmount::new(HELD + 500_000 + HELD)).await;

            apply_transition(entry_verified(sibling))
                .await
                .expect("the sibling's entry is booked");
            apply_transition(late_dca(id, Some(RawAmount::new(HELD + 500_000 + HELD))))
                .await
                .expect("the late DCA is booked once the sibling's holding is known");
            for position in [stored_position(id).await, memory_position(id).await] {
                assert!(position.exit_time.is_none(), "the position is open again");
                assert_eq!(
                    position.remaining_token_amount,
                    Some(RawAmount::new(HELD + 500_000)),
                    "the reopened position holds the sibling's tokens"
                );
            }
        },
    );
}

#[test]
fn a_late_fill_waits_while_another_swap_of_the_mint_is_in_flight() {
    common::run_isolated(
        "a_late_fill_waits_while_another_swap_of_the_mint_is_in_flight",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = written_off(|_| {}).await;
            let sibling = store_position(|_| {}).await;

            register_partial(sibling).await;
            assert_late_fills_wait(id, RawAmount::new(HELD)).await;
            state::clear_pending_partial_exit(PARTIAL_SIGNATURE)
                .await
                .expect("clear the sibling's partial exit");
            state::clear_partial_exit_pending(common::TEST_MINT).await;

            {
                let _submitting = state::mark_swap_in_flight(
                    screenerbot::chains::ChainId::Solana,
                    common::TEST_MINT,
                );
                assert_late_fills_wait(id, RawAmount::new(HELD)).await;
            }

            apply_transition(late_sell(id, Some(RawAmount::new(HELD))))
                .await
                .expect("the late sell is booked once no other swap is in flight");
            assert_eq!(exit_records(id).await, 1);
        },
    );
}

#[test]
fn each_late_fill_is_booked_once() {
    common::run_isolated("each_late_fill_is_booked_once", || async {
        let _dir = common::isolated_env();
        let _cfg = common::config_guard();
        enable_loss_limit();
        let id = written_off(|position| {
            position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
        })
        .await;

        for _ in 0..2 {
            apply_transition(late_dca(id, Some(RawAmount::ZERO)))
                .await
                .expect("the late DCA is booked");
            apply_transition(late_partial(id, Some(RawAmount::ZERO)))
                .await
                .expect("the late partial exit is booked");
            apply_transition(late_sell(id, Some(RawAmount::ZERO)))
                .await
                .expect("the late sell is booked");
        }

        let booked = in_storage(id).await;
        assert_eq!(booked, in_memory(id).await, "memory and storage disagree");
        assert_eq!(entry_records(id).await, 1);
        assert_eq!(exit_records(id).await, 2);
        assert_eq!(booked.dca_count, 1);
        assert_eq!(booked.partial_exit_count, 1);
        assert_eq!(booked.total_size_native, 1.5);
        assert_eq!(booked.native_received, Some(0.8 + 1.5));
        assert_acquired_balances(id, HELD + 500_000).await;
    });
}

#[test]
fn the_loss_figure_after_a_restart_equals_the_live_figure_after_restatement() {
    common::run_isolated(
        "the_loss_figure_after_a_restart_equals_the_live_figure_after_restatement",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = written_off(|_| {}).await;
            // The write-off happened an hour before the limiter's current start.
            injector()
                .execute(
                    "UPDATE positions SET exit_time = ?2 WHERE id = ?1",
                    rusqlite::params![id, (Utc::now() - chrono::Duration::hours(1)).to_rfc3339()],
                )
                .expect("age the write-off");
            initialize_from_history().await;
            assert_eq!(recorded_loss(), 1.0, "a startup counts the earlier loss");

            apply_transition(late_partial(id, Some(RawAmount::ZERO)))
                .await
                .expect("the late partial exit is booked");
            let live = recorded_loss();
            assert!((live - 0.2).abs() < 1e-12, "live figure {live}");

            initialize_from_history().await;
            assert_eq!(recorded_loss(), live, "a restart finds another figure");
        },
    );
}

#[test]
fn a_restatement_that_lowers_the_loss_keeps_a_pause_that_already_fired() {
    common::run_isolated(
        "a_restatement_that_lowers_the_loss_keeps_a_pause_that_already_fired",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit_at(0.5);
            let id = written_off(|position| {
                position.exit_transaction_signature = Some(CLOSE_SIGNATURE.to_owned());
            })
            .await;
            assert!(is_entry_blocked_by_loss_limit(), "the write-off pauses");

            apply_transition(late_sell(id, Some(RawAmount::ZERO)))
                .await
                .expect("the late sell is booked");
            assert_eq!(recorded_loss(), 0.0, "the figure follows the books");
            assert!(
                is_entry_blocked_by_loss_limit(),
                "a correction never lifts a pause"
            );
        },
    );
}

#[test]
fn a_restatement_that_raises_the_loss_past_the_limit_pauses_entries() {
    common::run_isolated(
        "a_restatement_that_raises_the_loss_past_the_limit_pauses_entries",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit_at(1.2);
            let id = written_off(|_| {}).await;
            assert!(!is_entry_blocked_by_loss_limit());

            apply_transition(late_dca(id, Some(RawAmount::ZERO)))
                .await
                .expect("the late DCA is booked");
            assert_eq!(recorded_loss(), 1.5);
            assert!(is_entry_blocked_by_loss_limit(), "the restated loss pauses");
        },
    );
}

#[test]
fn a_synthetic_close_of_a_wallet_derived_row_records_no_loss() {
    common::run_isolated(
        "a_synthetic_close_of_a_wallet_derived_row_records_no_loss",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            enable_loss_limit();
            let id = written_off(|position| {
                position.origin = screenerbot::positions::PositionOrigin::External;
            })
            .await;
            assert_eq!(stored_position(id).await.pnl, Some(-1.0));
            assert_eq!(recorded_loss(), 0.0);
        },
    );
}

#[test]
fn an_exit_clear_with_a_stale_signature_leaves_the_newer_exit_untouched() {
    common::run_isolated(
        "an_exit_clear_with_a_stale_signature_leaves_the_newer_exit_untouched",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position(|position| {
                position.exit_transaction_signature = Some("newer-exit-sig".to_owned());
                position.exit_price = Some(1.25);
                position.closed_reason = Some("stop_loss_pending_verification".to_owned());
            })
            .await;
            let before = stored_columns(id);

            let effects = apply_transition(failed_close(id))
                .await
                .expect("a stale clear is skipped");
            assert!(!effects.db_updated);
            assert_eq!(stored_columns(id), before, "the newer exit was cleared");
            for position in [stored_position(id).await, memory_position(id).await] {
                assert_eq!(
                    position.exit_transaction_signature.as_deref(),
                    Some("newer-exit-sig")
                );
                assert_eq!(position.exit_price, Some(1.25));
            }
        },
    );
}
