// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Partial-exit holds: a held partial exit blocks the mint's partial exits and its
//! reconciliation, and leaves only on a settled proof or the hold limit, returning its
//! take-profit step only on proof that nothing was sold.

use chrono::{DateTime, Duration, Utc};
use screenerbot::chains::RawAmount;
use screenerbot::positions::swap_hold::{
    self, HeldSwap, HoldOutcome, SwapHold, UNPROVEN_HOLD_LIMIT,
};
use screenerbot::positions::{books, db, partial_close_position, state, Book, Error};

use super::{
    common, mint_busy, open_position, outcomes, reader, settle, store_position, stored_holds_of,
    ScriptedReader, BOUND, HELD, STEP,
};

const EXIT: u128 = 250_000;

fn hold(
    position_id: i64,
    created_at: DateTime<Utc>,
    take_profit_level: Option<u32>,
    expiry_height: Option<u64>,
) -> SwapHold {
    SwapHold {
        mint: common::TEST_MINT.to_owned(),
        position_id,
        holding_before: RawAmount::new(HELD),
        expiry_height,
        created_at,
        swap: HeldSwap::PartialExit {
            exit_amount: RawAmount::new(EXIT),
            requested_exit_percentage: 25.0,
            take_profit_level,
        },
    }
}

/// The take-profit step of `id` in memory and in storage, which must agree.
async fn step_of(id: i64) -> u32 {
    let memory = state::get_position_by_id(id)
        .await
        .expect("position in memory")
        .take_profit_level;
    let stored = db::get_position_by_id(id)
        .await
        .expect("read the row")
        .expect("row exists")
        .take_profit_level;
    assert_eq!(memory, stored, "memory and storage disagree on the step");
    stored
}

/// Every reading that treats the mint or the position as busy.
async fn busy(position_id: i64) -> [bool; 5] {
    let mint = common::TEST_MINT;
    let [reconciliation, other_swap, own_swap] = mint_busy(position_id).await;
    [
        state::is_partial_exit_pending(mint).await,
        reconciliation,
        other_swap,
        own_swap,
        books::partial_exit_pending(Book::Live, mint).await,
    ]
}

/// A partial exit whose send failed with no signature is held: every busy reading holds
/// the mint, a second partial exit is refused before any quote, the reserved step stays
/// taken, and the paper book never reads the hold. The pending-partial counter carries
/// nothing for it, so dropping the hold frees the mint.
#[test]
fn an_unproven_partial_exit_holds_the_mint_and_refuses_the_next_partial() {
    common::run_isolated(
        module_path!(),
        "an_unproven_partial_exit_holds_the_mint_and_refuses_the_next_partial",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position().await;

            swap_hold::hold_swap(hold(id, Utc::now(), Some(STEP), Some(BOUND))).await;

            assert_eq!(busy(id).await, [true; 5]);
            assert!(!books::partial_exit_pending(Book::Paper, common::TEST_MINT).await);
            let refused = partial_close_position(common::TEST_MINT, 25.0, "test", None, None)
                .await
                .expect_err("a second partial exit is refused while one is held");
            assert!(
                matches!(refused, Error::PartialExitPending { ref mint } if mint == common::TEST_MINT),
                "refused with {refused:?}"
            );
            assert_eq!(step_of(id).await, STEP, "the reserved step was released");
            assert_eq!(stored_holds_of(id).await.len(), 1, "the hold is not stored");

            swap_hold::drop_holds_of_position(id)
                .await
                .expect("drop the hold");
            assert_eq!(busy(id).await, [false; 5], "the counter carried the hold");
            assert!(stored_holds_of(id).await.is_empty());
        },
    );
}

/// A settled holding lower by exactly the exit amount proves the sale: the hold leaves once,
/// the step stays taken, and the outcome is recorded once.
#[test]
fn a_sale_found_by_the_settled_holding_releases_the_hold_once_and_keeps_the_step() {
    common::run_isolated(
        module_path!(),
        "a_sale_found_by_the_settled_holding_releases_the_hold_once_and_keeps_the_step",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            common::start_events().await;
            let id = open_position().await;
            swap_hold::hold_swap(hold(id, Utc::now(), Some(STEP), Some(BOUND))).await;
            let sold = reader(Some(HELD - EXIT));

            assert_eq!(
                outcomes(&settle(&sold, Utc::now()).await),
                [HoldOutcome::Landed]
            );
            assert!(
                settle(&sold, Utc::now()).await.is_empty(),
                "a hold leaves once"
            );
            assert_eq!(busy(id).await, [false; 5]);
            assert_eq!(
                step_of(id).await,
                STEP,
                "a sold step was returned to the ladder"
            );
            assert!(stored_holds_of(id).await.is_empty());
            assert_eq!(common::position_events("partial_exit_hold_sold").await, 1);
        },
    );
}

/// A settled holding equal to the holding before the swap proves nothing sold: the step
/// returns to the ladder and the mint is free. A hold with no step releases none.
#[test]
fn an_unsent_sale_returns_its_step_and_frees_the_mint() {
    common::run_isolated(
        module_path!(),
        "an_unsent_sale_returns_its_step_and_frees_the_mint",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            common::start_events().await;
            let id = open_position().await;
            swap_hold::hold_swap(hold(id, Utc::now(), Some(STEP), Some(BOUND))).await;
            let unsold = reader(Some(HELD));

            assert_eq!(
                outcomes(&settle(&unsold, Utc::now()).await),
                [HoldOutcome::NotLanded]
            );
            assert_eq!(step_of(id).await, STEP - 1, "the step was not returned");
            assert_eq!(busy(id).await, [false; 5]);
            assert_eq!(
                common::position_events("partial_exit_hold_not_sold").await,
                1
            );

            // A stop-loss partial takes no step, so its proof releases none.
            swap_hold::hold_swap(hold(id, Utc::now(), None, Some(BOUND))).await;
            assert_eq!(
                outcomes(&settle(&unsold, Utc::now()).await),
                [HoldOutcome::NotLanded]
            );
            assert_eq!(step_of(id).await, STEP - 1);
        },
    );
}

/// With no settled reading, a hold waits until the hold limit and is then released as
/// unknown: the step stays taken.
#[test]
fn a_hold_with_no_proof_is_released_unknown_only_at_the_limit() {
    common::run_isolated(
        module_path!(),
        "a_hold_with_no_proof_is_released_unknown_only_at_the_limit",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position().await;
            let limit = Duration::from_std(UNPROVEN_HOLD_LIMIT).unwrap();
            let registered = Utc::now();
            swap_hold::hold_swap(hold(id, registered, Some(STEP), Some(BOUND))).await;
            let unread = reader(None);

            assert!(settle(&unread, registered + limit - Duration::seconds(1))
                .await
                .is_empty());
            assert_eq!(busy(id).await, [true; 5], "a young hold was released");

            assert_eq!(
                outcomes(&settle(&unread, registered + limit).await),
                [HoldOutcome::Unknown]
            );
            assert_eq!(busy(id).await, [false; 5]);
            assert_eq!(step_of(id).await, STEP, "a step was released without proof");
        },
    );
}

/// Holds survive a restart: rehydration restores them from storage, drops the hold of a
/// deleted row from memory and storage, and the first pass assigns and stores a missing
/// bound.
#[test]
fn holds_survive_a_restart_and_a_missing_bound_is_assigned_once() {
    common::run_isolated(
        module_path!(),
        "holds_survive_a_restart_and_a_missing_bound_is_assigned_once",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let id = open_position().await;
            let deleted = store_position(|position| {
                position.exit_time = Some(Utc::now());
                position.transaction_exit_verified = true;
            })
            .await;
            swap_hold::hold_swap(hold(id, Utc::now(), Some(STEP), None)).await;
            swap_hold::hold_swap(hold(deleted, Utc::now(), None, Some(BOUND))).await;
            assert!(db::delete_position_by_id(deleted)
                .await
                .expect("delete the row"));
            state::remove_position_by_id(deleted).await;

            assert_eq!(
                swap_hold::rehydrate_swap_holds()
                    .await
                    .expect("rehydrate holds"),
                1
            );
            assert_eq!(busy(id).await, [true; 5]);
            assert!(!state::position_has_pending_swap(common::TEST_MINT, deleted).await);
            assert!(stored_holds_of(deleted).await.is_empty());

            let unread = ScriptedReader {
                bound: 777,
                settled: None,
            };
            assert!(settle(&unread, Utc::now()).await.is_empty());
            let stored = stored_holds_of(id).await;
            assert_eq!(stored.len(), 1);
            assert_eq!(
                stored[0].expiry_height,
                Some(777),
                "the bound was not stored"
            );
            assert_eq!(
                swap_hold::holds_for_mint(common::TEST_MINT).await[0].expiry_height,
                Some(777)
            );
        },
    );
}

/// A full close stays open while a partial exit is held. Once the close emptied the wallet
/// and booked the row closed, the settled holding proves neither outcome: the hold is
/// released unknown and the closed row's step is left as it is.
#[test]
fn a_hold_on_a_closed_row_is_released_unknown_without_a_step_write() {
    common::run_isolated(
        module_path!(),
        "a_hold_on_a_closed_row_is_released_unknown_without_a_step_write",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            let _ = open_position().await;
            let closed = store_position(|position| {
                position.exit_time = Some(Utc::now());
                position.transaction_exit_verified = true;
                position.remaining_token_amount = Some(RawAmount::ZERO);
                position.total_exited_amount = RawAmount::new(HELD);
            })
            .await;
            swap_hold::hold_swap(hold(closed, Utc::now(), Some(STEP), Some(BOUND))).await;

            assert_eq!(
                outcomes(&settle(&reader(Some(0)), Utc::now()).await),
                [HoldOutcome::Unknown]
            );
            assert_eq!(step_of(closed).await, STEP);
            assert!(!swap_hold::holds_partial_exit(common::TEST_MINT).await);
        },
    );
}
