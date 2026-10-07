// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure: position lifecycle mechanics — partial-exit sizing, transition classification,
//! and the global position-slot accounting that enforces `max_open_positions`.
//!
//! None of this is arithmetic the user ever sees, which is exactly why it is dangerous:
//! a slot released twice silently raises the position cap, and a transition
//! misclassified as terminal closes a position that is still open.

// See `tests/trader_exits.rs` — the config guard is a `std::sync::Mutex` on purpose so
// synchronous tests can use it too, and holding it across an await cannot deadlock in a
// per-test runtime that nothing else shares.
#![allow(clippy::await_holding_lock)]

mod common;

use chrono::Utc;
use common::config_guard;
use screenerbot::chains::RawAmount;
use screenerbot::positions::state::{
    init_global_position_semaphore, register_position_slot, release_position_slot,
    try_consume_global_position_permit,
};
use screenerbot::positions::transitions::NotLandedEvidence;
use screenerbot::positions::PositionTransition;
use screenerbot::swaps::calculate_partial_amount;

/// A raw token amount in whatever integer type the field under test uses.
fn raw<T: From<u64>>(value: u64) -> T {
    T::from(value)
}

// ==================== PARTIAL EXIT SIZING ====================

#[test]
fn a_partial_amount_is_the_requested_share_of_the_balance() {
    assert_eq!(
        calculate_partial_amount(raw(1_000), 25.0),
        raw::<RawAmount>(250)
    );
    assert_eq!(
        calculate_partial_amount(raw(1_000), 50.0),
        raw::<RawAmount>(500)
    );
    assert_eq!(
        calculate_partial_amount(raw(1_000), 99.0),
        raw::<RawAmount>(990)
    );
}

#[test]
fn a_full_percentage_sells_the_entire_balance() {
    // Exactly 100 must return the balance itself, not a rounded product — selling
    // 999_999_999 of 1_000_000_000 units leaves dust that blocks the account close.
    assert_eq!(
        calculate_partial_amount(raw(1_000_000_000), 100.0),
        raw::<RawAmount>(1_000_000_000)
    );
    assert_eq!(
        calculate_partial_amount(raw(1_000_000_000), 150.0),
        raw::<RawAmount>(1_000_000_000)
    );
}

#[test]
fn a_partial_amount_can_never_exceed_the_balance() {
    // The wallet cannot sell what it does not hold; an over-sized amount fails the
    // swap outright rather than partially filling.
    for pct in [100.0, 100.000_001, 1_000.0, f64::INFINITY] {
        assert!(
            calculate_partial_amount(raw(12_345), pct) <= raw::<RawAmount>(12_345),
            "percentage {pct} produced more than the balance"
        );
    }
}

#[test]
fn a_zero_balance_or_non_positive_percentage_sells_nothing() {
    assert_eq!(calculate_partial_amount(raw(0), 50.0), raw::<RawAmount>(0));
    assert_eq!(
        calculate_partial_amount(raw(1_000), 0.0),
        raw::<RawAmount>(0)
    );
    assert_eq!(
        calculate_partial_amount(raw(1_000), -25.0),
        raw::<RawAmount>(0)
    );
    assert_eq!(
        calculate_partial_amount(raw(1_000), f64::NAN),
        raw::<RawAmount>(0)
    );
}

#[test]
fn a_partial_amount_truncates_rather_than_rounding_up() {
    // Truncating keeps the result inside the balance for every input. Rounding up on
    // the last percent would try to sell one unit more than is held.
    assert_eq!(calculate_partial_amount(raw(7), 50.0), raw::<RawAmount>(3));
    assert_eq!(calculate_partial_amount(raw(3), 99.9), raw::<RawAmount>(2));
}

#[test]
fn a_dust_sized_share_of_a_small_balance_is_zero() {
    // The caller must treat 0 as "do not submit" — `partial_close_position` refuses a
    // zero exit amount rather than sending a swap that cannot fill.
    assert_eq!(calculate_partial_amount(raw(10), 1.0), raw::<RawAmount>(0));
}

#[test]
fn a_partial_amount_stays_exact_above_the_u64_range() {
    assert_eq!(
        calculate_partial_amount(RawAmount::new(1u128 << 70), 50.0),
        RawAmount::new(1u128 << 69)
    );
    assert_eq!(
        calculate_partial_amount(RawAmount::MAX, 100.0),
        RawAmount::MAX
    );
    let just_below_100 = f64::from_bits(100f64.to_bits() - 1);
    assert_eq!(
        calculate_partial_amount(RawAmount::MAX, just_below_100),
        RawAmount::new(u128::MAX >> 75 << 75)
    );
}

// ==================== TRANSITION CLASSIFICATION ====================

fn all_transitions() -> Vec<PositionTransition> {
    let now = Utc::now();
    vec![
        PositionTransition::EntryVerified {
            position_id: 1,
            effective_entry_price: 0.01,
            token_amount_units: raw(100),
            fee_raw: 5_000,
            native_size: 1.0,
        },
        PositionTransition::ExitVerified {
            position_id: 1,
            effective_exit_price: 0.02,
            native_received: 2.0,
            fee_raw: 5_000,
            exit_time: now,
            exit_signature: "exit-sig".to_owned(),
        },
        PositionTransition::ExitFailedClearForRetry {
            position_id: 1,
            exit_signature: "exit-sig".to_owned(),
        },
        PositionTransition::RemoveOrphanEntry {
            position_id: 1,
            signature: "entry-sig".to_owned(),
            evidence: NotLandedEvidence::Expired,
        },
        PositionTransition::UpdatePriceTracking {
            mint: "mint".to_owned(),
            current_price: 0.01,
            highest: Some(0.02),
            lowest: Some(0.005),
        },
        PositionTransition::PartialExitSubmitted {
            position_id: 1,
            exit_signature: "sig".to_owned(),
            exit_amount: raw(50),
            exit_percentage: 50.0,
            market_price: 0.02,
        },
        PositionTransition::PartialExitVerified {
            position_id: 1,
            exit_amount: raw(50),
            native_received: 1.0,
            effective_exit_price: 0.02,
            fee_raw: 5_000,
            exit_time: now,
            exit_signature: "sig".to_owned(),
            exit_percentage: 50.0,
        },
        PositionTransition::PartialExitFailed {
            position_id: 1,
            reason: "boom".to_owned(),
        },
        PositionTransition::ExitResidualClearForRetry {
            position_id: 1,
            exit_amount: raw(50),
            native_received: 1.0,
            effective_exit_price: 0.02,
            fee_raw: 5_000,
            exit_time: now,
            exit_signature: "sig".to_owned(),
            exit_percentage: 50.0,
        },
        PositionTransition::DcaSubmitted {
            position_id: 1,
            dca_signature: "sig".to_owned(),
            dca_amount_native: 0.5,
            market_price: 0.01,
        },
        PositionTransition::DcaVerified {
            position_id: 1,
            tokens_bought: raw(50),
            native_spent: 0.5,
            effective_price: 0.01,
            fee_raw: 5_000,
            dca_time: now,
            dca_signature: "sig".to_owned(),
        },
        PositionTransition::DcaFailed {
            position_id: 1,
            dca_signature: "sig".to_owned(),
            reason: "boom".to_owned(),
        },
    ]
}

fn label(transition: &PositionTransition) -> String {
    format!("{transition:?}")
        .split_whitespace()
        .next()
        .unwrap_or("?")
        .to_owned()
}

#[test]
fn every_position_scoped_transition_carries_its_position_id() {
    // Only price tracking is keyed by mint; everything else must name the position it
    // mutates, or `apply_transition` cannot find the row to update.
    for transition in all_transitions() {
        let expected = !matches!(transition, PositionTransition::UpdatePriceTracking { .. });
        assert_eq!(
            transition.position_id().is_some(),
            expected,
            "{} position_id",
            label(&transition)
        );
    }
}

#[test]
fn only_a_real_ending_is_terminal() {
    // A terminal transition closes the position and frees its slot. A FAILED exit, a
    // failed partial, a residual fill and every DCA leave the position open — treating
    // any of them as terminal would close a position the wallet still holds tokens for.
    for transition in all_transitions() {
        let expected = matches!(
            transition,
            PositionTransition::ExitVerified { .. } | PositionTransition::RemoveOrphanEntry { .. }
        );
        assert_eq!(
            transition.is_terminal(),
            expected,
            "{} is_terminal",
            label(&transition)
        );
    }
}

#[test]
fn only_verified_swaps_move_the_wallet_balance() {
    // This flag fires a wallet refresh, so it must mean "SOL or tokens actually moved".
    // A SUBMITTED swap has not settled and a FAILED one moved nothing — refreshing on
    // either would show a balance that has not changed yet, or churn for no reason.
    for transition in all_transitions() {
        let expected = matches!(
            transition,
            PositionTransition::EntryVerified { .. }
                | PositionTransition::ExitVerified { .. }
                | PositionTransition::PartialExitVerified { .. }
                | PositionTransition::ExitResidualClearForRetry { .. }
                | PositionTransition::DcaVerified { .. }
        );
        assert_eq!(
            transition.affects_wallet_balance(),
            expected,
            "{} affects_wallet_balance",
            label(&transition)
        );
    }
}

#[test]
fn a_submitted_swap_never_claims_to_have_moved_funds() {
    // Called out separately because it is the easy mistake: the *Submitted* variants
    // read like something happened, and they are the ones that have not.
    for transition in all_transitions() {
        if matches!(
            transition,
            PositionTransition::PartialExitSubmitted { .. }
                | PositionTransition::DcaSubmitted { .. }
        ) {
            assert!(
                !transition.affects_wallet_balance(),
                "{} must not trigger a balance refresh",
                label(&transition)
            );
        }
    }
}

// ==================== POSITION SLOT ACCOUNTING ====================
//
// The global semaphore IS the `max_open_positions` limit. Every open position holds
// exactly one permit for its lifetime, and several terminal paths can run for the same
// position — archiving or force-closing frees the slot immediately, and the exit
// verification already queued frees it again when it lands. Each extra release hands
// the trader a slot it does not own, so the bot quietly opens MORE positions than the
// user configured, and nothing repairs it until the next startup reconcile.

/// One test, not several: the semaphore is a process-global `OnceLock`, so a second
/// test's `init_global_position_semaphore` would be a no-op and inherit whatever the
/// first left behind. Keeping the whole invariant in a single test makes it correct
/// under a shared-process runner too, not only under nextest's process-per-test.
#[tokio::test]
async fn a_slot_is_released_at_most_once_however_many_paths_run() {
    let _cfg = config_guard();
    init_global_position_semaphore(2);

    // Consume the whole budget.
    assert!(try_consume_global_position_permit(), "first slot");
    register_position_slot(1).await;
    assert!(try_consume_global_position_permit(), "second slot");
    register_position_slot(2).await;
    assert!(
        !try_consume_global_position_permit(),
        "the cap must be enforced once every slot is taken"
    );

    // Position 1 closes: exactly one slot comes back.
    release_position_slot(1).await;
    assert!(try_consume_global_position_permit(), "the freed slot");
    register_position_slot(1).await;
    assert!(!try_consume_global_position_permit(), "still at the cap");

    // A second terminal path runs for position 1 — but it was re-registered above, so
    // release it and then release it AGAIN, which is the real-world double release.
    release_position_slot(1).await;
    release_position_slot(1).await;

    assert!(try_consume_global_position_permit(), "the one real release");
    assert!(
        !try_consume_global_position_permit(),
        "a repeated release must not manufacture a slot"
    );

    // The reverse mistake: a position that never consumed a permit (rejected at the
    // cap, or its open failed) must not hand one out on the way down either.
    release_position_slot(999).await;
    assert!(
        !try_consume_global_position_permit(),
        "an unheld release must not free a slot"
    );
}

// ==================== "IS THIS POSITION OPEN?" ====================
//
// One predicate decides this for the whole system, and read and write paths must never
// disagree about it. When the by-mint lookup fell back to ANY position with that mint,
// a token you had once closed could never be bought again from the dashboard: the UI
// saw the dead position, treated the token as held, and sent the buy to the DCA
// endpoint, which correctly refused it. The user saw a failed "add to position" they
// had never asked for.

/// Replace the in-memory position set. Serialised by the config guard, which every
/// caller of this holds.
async fn set_positions(positions: Vec<screenerbot::positions::Position>) {
    let mut store = screenerbot::positions::state::POSITIONS.write().await;
    *store = positions;
}

fn position_for(mint: &str) -> screenerbot::positions::Position {
    let mut position = common::test_position(0.01, 1.0);
    position.mint = mint.to_owned();
    position
}

#[tokio::test]
async fn only_genuinely_open_positions_are_reported_as_open() {
    let _cfg = config_guard();

    let open = position_for("OpenMint11111111111111111111111111111111111");

    let mut closed = position_for("ClosedMint111111111111111111111111111111111");
    closed.exit_time = Some(Utc::now());
    closed.exit_transaction_signature = Some("exit".to_owned());
    closed.transaction_exit_verified = true;

    let mut archived = position_for("ArchivedMint1111111111111111111111111111111");
    archived.archived = true;

    // A close that has been SUBMITTED but not yet verified still counts as open: the
    // tokens are in the wallet and the verification path has to be able to find it.
    let mut confirming = position_for("ConfirmMint11111111111111111111111111111111");
    confirming.exit_transaction_signature = Some("pending".to_owned());
    confirming.transaction_exit_verified = false;

    set_positions(vec![
        open.clone(),
        closed.clone(),
        archived.clone(),
        confirming.clone(),
    ])
    .await;

    let open_mints: Vec<String> = screenerbot::positions::get_open_positions()
        .await
        .into_iter()
        .map(|p| p.mint)
        .collect();

    assert!(open_mints.contains(&open.mint), "a plain open position");
    assert!(
        open_mints.contains(&confirming.mint),
        "a close still confirming is still open"
    );
    assert!(!open_mints.contains(&closed.mint), "a closed position");
    assert!(!open_mints.contains(&archived.mint), "an archived position");

    set_positions(Vec::new()).await;
}

#[tokio::test]
async fn a_lookup_by_mint_never_returns_a_closed_position() {
    // History is reached explicitly by position id. If this returned a dead position,
    // the dashboard would treat a token it once traded as permanently held.
    let _cfg = config_guard();

    let mint = "HistoryMint11111111111111111111111111111111";
    let mut closed = position_for(mint);
    closed.id = Some(7);
    closed.exit_time = Some(Utc::now());
    closed.exit_transaction_signature = Some("exit".to_owned());
    closed.transaction_exit_verified = true;

    set_positions(vec![closed]).await;

    assert!(
        screenerbot::positions::get_position_by_mint(mint)
            .await
            .is_none(),
        "a closed position must not answer a by-mint lookup"
    );
    assert!(
        !screenerbot::positions::is_open_position(mint).await,
        "and must not report the token as held"
    );

    set_positions(Vec::new()).await;
}

#[tokio::test]
async fn a_reentered_token_resolves_to_its_live_round() {
    // A token can be entered, exited and entered again, each round its own row. The
    // by-mint lookup must find the LIVE one, not the first match in the vector.
    let _cfg = config_guard();

    let mint = "ReentryMint11111111111111111111111111111111";
    let mut first_round = position_for(mint);
    first_round.id = Some(1);
    first_round.exit_time = Some(Utc::now());
    first_round.exit_transaction_signature = Some("exit".to_owned());
    first_round.transaction_exit_verified = true;

    let mut second_round = position_for(mint);
    second_round.id = Some(2);

    set_positions(vec![first_round, second_round]).await;

    let found = screenerbot::positions::get_position_by_mint(mint)
        .await
        .expect("the live round must be found");
    assert_eq!(found.id, Some(2), "the open round, not the closed one");

    set_positions(Vec::new()).await;
}

// ==================== APPLY-FAILURE DISPOSITION ====================

use screenerbot::errors::DatabaseError;
use screenerbot::positions::queue::{
    VerificationQueue, MAX_REQUEUE_ATTEMPTS, NOT_LANDED_CONFIRM_MIN_SECS,
};
use screenerbot::positions::{
    ApplyFailureDisposition, Error as PositionError, GiveUpReason, VerificationItem,
    VerificationKind,
};

const DISPOSITION_MINT: &str = "DispositionMint1111111111111111111111111111";

fn entry_item(signature: &str, expiry_height: Option<u64>) -> VerificationItem {
    VerificationItem::new(
        signature.to_owned(),
        DISPOSITION_MINT.to_owned(),
        Some(1),
        VerificationKind::Entry,
        expiry_height,
    )
}

fn non_retryable_apply_errors() -> Vec<PositionError> {
    vec![
        PositionError::AmountOverflow {
            mint: DISPOSITION_MINT.to_owned(),
            operation: "booking a partial exit",
        },
        PositionError::Database(DatabaseError::Query {
            operation: "commit_booking".to_owned(),
            message: "constraint failed".to_owned(),
        }),
        PositionError::NotFoundById { position_id: 1 },
        PositionError::TransitionFailed {
            transition: "ExitVerified",
            mint: DISPOSITION_MINT.to_owned(),
            detail: "position has no id".to_owned(),
        },
    ]
}

fn retryable_apply_errors() -> Vec<PositionError> {
    vec![
        PositionError::Database(DatabaseError::Busy {
            operation: "commit_booking".to_owned(),
            message: "database is locked".to_owned(),
        }),
        PositionError::Database(DatabaseError::Connection {
            message: "pool timed out".to_owned(),
        }),
        PositionError::NotInitialised,
    ]
}

#[test]
fn a_non_retryable_apply_error_drops_the_item_on_its_first_attempt() {
    let item = entry_item("sig-rejected", None);
    for error in non_retryable_apply_errors() {
        let disposition = item.apply_failure_disposition(&error);
        assert!(
            matches!(
                disposition,
                ApplyFailureDisposition::Drop(GiveUpReason::ApplyRejected { .. })
            ),
            "{error} must drop as rejected, got {disposition:?}"
        );
    }
}

#[test]
fn a_retryable_apply_error_requeues_the_item_within_its_limits() {
    let item = entry_item("sig-retryable", None);
    for error in retryable_apply_errors() {
        let disposition = item.apply_failure_disposition(&error);
        assert!(
            matches!(disposition, ApplyFailureDisposition::Requeue),
            "{error} must requeue, got {disposition:?}"
        );
    }
}

#[test]
fn a_retryable_apply_error_at_the_requeue_cap_drops_the_item() {
    let mut item = entry_item("sig-capped", None);
    item.attempts = MAX_REQUEUE_ATTEMPTS;
    for error in retryable_apply_errors() {
        let disposition = item.apply_failure_disposition(&error);
        assert!(
            matches!(
                disposition,
                ApplyFailureDisposition::Drop(GiveUpReason::MaxAttemptsReached { max: 12, .. })
            ),
            "{error} at the requeue cap must drop, got {disposition:?}"
        );
    }
}

#[test]
fn a_retryable_apply_error_past_the_verification_age_drops_the_item() {
    let mut item = entry_item("sig-aged", None);
    item.created_at = Utc::now() - chrono::Duration::hours(25);
    for error in retryable_apply_errors() {
        let disposition = item.apply_failure_disposition(&error);
        assert!(
            matches!(
                disposition,
                ApplyFailureDisposition::Drop(GiveUpReason::MaxAgeReached { .. })
            ),
            "{error} past the age limit must drop, got {disposition:?}"
        );
    }
}

#[test]
fn an_abandoned_signature_is_never_enqueued_again() {
    let mut queue = VerificationQueue::new();
    queue.abandon("sig-dropped".to_owned());

    assert!(!queue.enqueue(entry_item("sig-dropped", None)));
    assert_eq!(queue.len(), 0);

    assert!(queue.enqueue(entry_item("sig-other", None)));
    assert_eq!(queue.len(), 1);
    assert!(
        !queue.enqueue(entry_item("sig-other", None)),
        "an already queued signature is not inserted twice"
    );
    assert_eq!(queue.len(), 1);
}

#[test]
fn a_confirmed_swap_never_awaits_settlement() {
    let mut confirmed = entry_item("sig-confirmed", Some(100));
    confirmed.swap_confirmed = true;
    assert!(!confirmed.awaits_settlement());

    let unconfirmed = entry_item("sig-unconfirmed", Some(100));
    assert!(unconfirmed.awaits_settlement());

    let mut confirmed_without_height = entry_item("sig-confirmed-no-height", None);
    confirmed_without_height.swap_confirmed = true;
    confirmed_without_height.created_at = Utc::now() - chrono::Duration::hours(1);
    assert!(!confirmed_without_height.awaits_settlement());

    let retried = confirmed.with_retry();
    assert!(retried.swap_confirmed, "a retry keeps the confirmation");
    assert!(!retried.awaits_settlement());

    let renewed = confirmed.renewed();
    assert!(renewed.swap_confirmed, "a renewal keeps the confirmation");
    assert_eq!(renewed.attempts, 0);
    assert_eq!(renewed.expiry_height, Some(100));
}

#[test]
fn only_unconfirmed_bounded_items_are_settlement_candidates() {
    let mut queue = VerificationQueue::new();
    queue.enqueue(entry_item("sig-bounded", Some(100)));
    queue.enqueue(entry_item("sig-unbounded", None));
    let mut confirmed = entry_item("sig-confirmed", Some(100));
    confirmed.swap_confirmed = true;
    queue.enqueue(confirmed);

    let candidates: Vec<String> = queue
        .settlement_candidates()
        .into_iter()
        .map(|item| item.signature)
        .collect();
    assert_eq!(candidates, ["sig-bounded"]);

    let unbounded = queue.unbounded_signatures();
    assert_eq!(unbounded, ["sig-unbounded"]);
    assert_eq!(queue.assign_expiry_bound(250, &unbounded), 1);
    assert!(queue.unbounded_signatures().is_empty());
    let candidates: Vec<(String, Option<u64>)> = queue
        .settlement_candidates()
        .into_iter()
        .map(|item| (item.signature, item.expiry_height))
        .collect();
    assert_eq!(
        candidates,
        [
            ("sig-bounded".to_owned(), Some(100)),
            ("sig-unbounded".to_owned(), Some(250)),
        ],
        "an assigned bound never replaces one the item already carries"
    );

    assert!(queue.mark_swap_confirmed("sig-bounded"));
    assert!(!queue.mark_swap_confirmed("sig-absent"));
    let candidates: Vec<String> = queue
        .settlement_candidates()
        .into_iter()
        .map(|item| item.signature)
        .collect();
    assert_eq!(candidates, ["sig-unbounded"]);
}

#[test]
fn a_bound_is_assigned_only_to_the_items_queued_before_it_was_read() {
    let mut queue = VerificationQueue::new();
    queue.enqueue(entry_item("sig-before", None));
    let unbounded = queue.unbounded_signatures();

    queue.enqueue(entry_item("sig-after", None));
    assert_eq!(queue.assign_expiry_bound(250, &unbounded), 1);

    let bounds: Vec<(String, Option<u64>)> = ["sig-before", "sig-after"]
        .into_iter()
        .map(|signature| {
            let item = queue.remove(signature).expect("queued");
            (item.signature, item.expiry_height)
        })
        .collect();
    assert_eq!(
        bounds,
        [
            ("sig-before".to_owned(), Some(250)),
            ("sig-after".to_owned(), None),
        ],
        "an item enqueued after the snapshot keeps no bound"
    );
}

#[test]
fn a_deferred_item_is_a_settlement_candidate_only_once_due() {
    let mut queue = VerificationQueue::new();
    let mut item = entry_item("sig-deferred", Some(100));
    item.attempts = 3;
    queue.enqueue(item);

    assert!(queue.defer_settlement("sig-deferred"));
    assert!(!queue.defer_settlement("sig-absent"));
    assert!(
        queue.settlement_candidates().is_empty(),
        "a deferred item is not read again before its retry time"
    );

    let mut deferred = queue.remove("sig-deferred").expect("queued");
    assert_eq!(deferred.attempts, 3, "a deferral spends no attempt");
    assert_eq!(deferred.not_landed_reads, 1);
    assert!(deferred.next_retry_at.is_some_and(|at| at > Utc::now()));

    deferred.next_retry_at = Some(Utc::now() - chrono::Duration::seconds(1));
    queue.enqueue(deferred);
    let candidates: Vec<String> = queue
        .settlement_candidates()
        .into_iter()
        .map(|item| item.signature)
        .collect();
    assert_eq!(
        candidates,
        ["sig-deferred"],
        "a due deferred item is read again"
    );
}

/// The read that confirms a not-landed read comes no earlier than the minimum after it, for
/// any signature (each draws its own jitter) and at every count of earlier deferrals.
#[test]
fn every_deferral_waits_at_least_the_not_landed_minimum() {
    for index in 0..200 {
        let mut item = entry_item(&format!("sig-minimum-{index}"), Some(100));
        for _ in 0..4 {
            let floor = Utc::now() + chrono::Duration::seconds(NOT_LANDED_CONFIRM_MIN_SECS);
            item = item.deferred();
            assert!(
                item.next_retry_at.is_some_and(|at| at >= floor),
                "{} after {} reads is due before the minimum",
                item.signature,
                item.not_landed_reads
            );
        }
    }
}

#[test]
fn only_the_second_not_landed_read_of_an_entry_parks_it() {
    let entry = entry_item("sig-park", Some(100));
    let dca = VerificationItem::new_dca(
        "sig-park-dca".to_owned(),
        DISPOSITION_MINT.to_owned(),
        Some(1),
        Some(100),
    );
    let exit = VerificationItem::new(
        "sig-park-exit".to_owned(),
        DISPOSITION_MINT.to_owned(),
        Some(1),
        VerificationKind::Exit,
        Some(100),
    );
    assert!(!entry.deferral_parks_entry(), "the first read only defers");
    assert!(entry.deferred().deferral_parks_entry());
    assert!(
        !entry.deferred().deferred().deferral_parks_entry(),
        "a parked entry is reported once"
    );
    for other in [dca, exit] {
        assert!(
            !other.deferred().deferral_parks_entry(),
            "{}",
            other.signature
        );
    }
}

#[test]
fn each_consecutive_deferral_waits_longer_and_a_pending_read_restarts_the_count() {
    let item = entry_item("sig-widening", Some(100));
    let first = item.deferred();
    let second = first.deferred();
    let third = second.deferred();
    assert_eq!(
        (
            first.not_landed_reads,
            second.not_landed_reads,
            third.not_landed_reads
        ),
        (1, 2, 3)
    );
    assert!(second.next_retry_at > first.next_retry_at);
    assert!(third.next_retry_at > second.next_retry_at);
    assert_eq!(third.attempts, item.attempts);
    assert!(
        third.renewed().not_landed_seen(),
        "a renewal keeps the count"
    );

    let mut queue = VerificationQueue::new();
    queue.enqueue(third);
    assert!(queue.clear_not_landed("sig-widening"));
    let cleared = queue.remove("sig-widening").expect("queued");
    assert!(!cleared.not_landed_seen());
}
