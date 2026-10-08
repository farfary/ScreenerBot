// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure: how reduced wallet-history rounds become `positions` rows.
//!
//! The reducer decides what a round IS; this planner decides what the database and the
//! dashboard are allowed to say about it. Two classes of bug live here and both are
//! silent:
//!
//!   * **Fabricated money.** A round with no established cost basis must reach the row
//!     with no P&L and no invested figure at all. Writing a zero instead makes the
//!     dashboard read "free entry, pure profit" on an airdrop.
//!   * **Clobbered user state.** The sync runs on every boot. If it rewrote `archived`,
//!     the live price fields, or a position the bot executed itself, a restart would
//!     quietly undo the user's decisions and the trader's own bookkeeping.
//!
//! The planner cases use no database, no wallet and no clock: rounds are built inline
//! and `now` is passed in. The application cases run against a real positions database
//! in a child process of their own.

mod common;

use chrono::{DateTime, TimeZone, Utc};
use std::collections::{HashMap, HashSet};

use screenerbot::chains::{ChainId, RawAmount};
use screenerbot::positions::apply::apply_transition;
use screenerbot::positions::ledger::reduce_rounds;
use screenerbot::positions::ledger::sync::{
    apply_plan, plan_position_writes, AppliedPlan, RoundKeyRelease, RoundMetadata, TraderLegs,
};
use screenerbot::positions::ledger::{
    LedgerEvent, LedgerEventKind, LedgerRound, QuoteAsset, QuoteLeg,
};
use screenerbot::positions::{
    db, state, PendingPartialExit, Position, PositionManagement, PositionOrigin, PositionTransition,
};
use screenerbot::transactions::deltas::{DeltaKind, SubjectAssetDelta, NATIVE_SOL_SENTINEL};

const MINT: &str = "So11111111111111111111111111111111111111112";
const OTHER_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000, 0).unwrap()
}

/// A closed, fully-priced round: bought for 2 SOL, sold for 3.
fn round(mint: &str, round_key: &str) -> LedgerRound {
    LedgerRound {
        mint: mint.to_owned(),
        decimals: 6,
        round_key: round_key.to_owned(),
        opened_at: Some(1_600_000_000),
        closed_at: Some(1_600_001_000),
        is_open: false,
        balance_raw: 0,
        total_acquired_raw: 1_000_000,
        total_disposed_raw: 1_000_000,
        entry_count: 1,
        exit_count: 1,
        invested_native: 2.0,
        remaining_basis_native: 0.0,
        realized_proceeds_native: 3.0,
        realized_cost_native: 2.0,
        average_entry_price_native: Some(2.0),
        average_exit_price_native: Some(3.0),
        realized_pnl_native: Some(1.0),
        basis_complete: true,
        history_complete: true,
        entry_signature: Some("open-sig".to_owned()),
        exit_signature: Some("close-sig".to_owned()),
        events: Vec::new(),
    }
}

/// Still held: bought for 2 SOL, nothing sold.
fn open_round(mint: &str, round_key: &str) -> LedgerRound {
    LedgerRound {
        closed_at: None,
        is_open: true,
        balance_raw: 1_000_000,
        total_disposed_raw: 0,
        exit_count: 0,
        remaining_basis_native: 2.0,
        realized_proceeds_native: 0.0,
        realized_cost_native: 0.0,
        average_exit_price_native: None,
        realized_pnl_native: None,
        exit_signature: None,
        ..round(mint, round_key)
    }
}

/// One observed movement, carrying only the block time these tests care about.
fn event(block_time: i64) -> LedgerEvent {
    LedgerEvent {
        signature: "open-sig".to_owned(),
        slot: Some(1),
        block_time: Some(block_time),
        kind: LedgerEventKind::Entry,
        amount: 1.0,
        balance_after: 1.0,
        quote: None,
        price_native: None,
        venue: None,
    }
}

fn metadata(mint: &str, frozen: bool) -> HashMap<String, RoundMetadata> {
    HashMap::from([(
        mint.to_owned(),
        RoundMetadata {
            symbol: Some("TKN".to_owned()),
            name: Some("Token".to_owned()),
            frozen,
        },
    )])
}

fn no_metadata() -> HashMap<String, RoundMetadata> {
    HashMap::new()
}

/// No swap is in flight — the ordinary case.
fn no_busy() -> HashSet<String> {
    HashSet::new()
}

/// No trader-booked legs: every acquisition in the round is treated as one the bot did
/// not execute itself.
fn no_legs() -> HashMap<i64, TraderLegs> {
    HashMap::new()
}

/// A position the TRADER opened and verified for the same buy the round was reduced
/// from: still open, because the bot never sold it — whatever the chain went on to do.
fn bot_position(round: &LedgerRound) -> Position {
    let mut position =
        materialise(std::slice::from_ref(round), &metadata(&round.mint, false))[0].clone();
    position.origin = PositionOrigin::Auto {
        strategy_id: Some("momentum".to_owned()),
    };
    position.management = PositionManagement::AutoTrader;
    position.round_key = None;
    position.entry_transaction_signature = round.entry_signature.clone();
    position.transaction_entry_verified = true;
    // What the trader booked at entry: fee-exact, and the ledger must never rewrite it.
    position.total_size_native = 2.0;
    position.entry_size_native = 2.0;
    // Open on our books: no exit was ever executed or recorded by us.
    position.exit_time = None;
    position.exit_price = None;
    position.effective_exit_price = None;
    position.average_exit_price = None;
    position.exit_transaction_signature = None;
    position.transaction_exit_verified = false;
    position.closed_reason = None;
    position.native_received = None;
    position.pnl = None;
    position.pnl_percent = None;
    position.remaining_token_amount = position.token_amount;
    position.total_exited_amount = raw(0);
    position
}

/// The row a first sync would create — used as the "already exists" input everywhere
/// below, so the tests never hand-build a 50-field `Position`.
fn materialise(rounds: &[LedgerRound], meta: &HashMap<String, RoundMetadata>) -> Vec<Position> {
    let plan = plan_position_writes(rounds, &[], meta, &no_legs(), &no_busy(), now());
    assert!(plan.updates.is_empty(), "nothing existed to update");
    plan.inserts
        .into_iter()
        .enumerate()
        .map(|(index, mut position)| {
            // The database assigns these on insert.
            position.id = Some(index as i64 + 1);
            position
        })
        .collect()
}

// =============================================================================
// CREATING ROWS
// =============================================================================

/// A raw token amount in whatever integer type the field under test uses.
fn raw<T: From<u64>>(value: u64) -> T {
    T::from(value)
}

#[test]
fn a_new_round_becomes_an_external_user_only_position() {
    let rounds = vec![round(MINT, "open-sig:MINT")];
    let plan = plan_position_writes(
        &rounds,
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert_eq!(plan.inserts.len(), 1);
    assert!(plan.updates.is_empty());

    let position = &plan.inserts[0];
    assert_eq!(position.origin, PositionOrigin::External);
    // UserOnly is the only management an External origin accepts, and it is what keeps
    // every automatic exit and DCA away from a holding the bot never bought.
    assert_eq!(position.management, PositionManagement::UserOnly);
    assert!(position.management.is_valid_for_origin(&position.origin));
    assert_eq!(position.round_key.as_deref(), Some("open-sig:MINT"));
    assert_eq!(position.mint, MINT);
    assert_eq!(position.symbol, "TKN");
}

#[test]
fn a_closed_round_is_marked_exit_verified_so_it_reaches_the_closed_tab() {
    let rounds = vec![round(MINT, "open-sig:MINT")];
    let plan = plan_position_writes(
        &rounds,
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );
    let position = &plan.inserts[0];

    // `get_closed_positions` filters on `transaction_exit_verified`. A round reduced
    // from confirmed on-chain history has nothing left to verify, so leaving this false
    // would strand every imported closed round in neither the Open nor the Closed tab.
    assert!(position.transaction_exit_verified);
    assert!(position.transaction_entry_verified);
    assert!(position.exit_time.is_some());
    assert_eq!(
        position.exit_transaction_signature.as_deref(),
        Some("close-sig")
    );
}

#[test]
fn an_open_round_is_not_marked_exited() {
    let rounds = vec![open_round(MINT, "open-sig:MINT")];
    let plan = plan_position_writes(
        &rounds,
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );
    let position = &plan.inserts[0];

    assert!(!position.transaction_exit_verified);
    assert!(position.exit_time.is_none());
    assert!(position.exit_transaction_signature.is_none());
    assert_eq!(position.remaining_token_amount, Some(raw(1_000_000)));
}

#[test]
fn a_second_round_in_the_same_mint_is_a_separate_row() {
    // Bought, sold everything, bought again. The re-buy must NOT resurrect the closed
    // round or inherit its cost basis — it is a new lifecycle with its own key.
    let rounds = vec![
        round(MINT, "first-sig:MINT"),
        open_round(MINT, "second-sig:MINT"),
    ];
    let plan = plan_position_writes(
        &rounds,
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert_eq!(plan.inserts.len(), 2);
    let keys: Vec<_> = plan
        .inserts
        .iter()
        .filter_map(|p| p.round_key.clone())
        .collect();
    assert_eq!(keys, vec!["first-sig:MINT", "second-sig:MINT"]);
}

#[test]
fn entries_and_exits_are_counted_as_adds_and_partials() {
    // Three buys and three sells that ended flat: the first buy is the entry and the
    // last sell is the exit, so what is left is 2 DCAs and 2 partial exits.
    let mut source = round(MINT, "open-sig:MINT");
    source.entry_count = 3;
    source.exit_count = 3;

    let plan = plan_position_writes(
        &[source],
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );
    let position = &plan.inserts[0];

    assert_eq!(position.dca_count, 2);
    assert_eq!(position.partial_exit_count, 2);
}

#[test]
fn an_open_round_counts_every_disposal_as_a_partial() {
    // Nothing closed this round, so none of its sells was the final exit.
    let mut source = open_round(MINT, "open-sig:MINT");
    source.exit_count = 2;

    let plan = plan_position_writes(
        &[source],
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );
    assert_eq!(plan.inserts[0].partial_exit_count, 2);
}

#[test]
fn a_mint_with_no_metadata_still_gets_a_readable_row() {
    let plan = plan_position_writes(
        &[round(MINT, "open-sig:MINT")],
        &[],
        &no_metadata(),
        &no_legs(),
        &no_busy(),
        now(),
    );
    let position = &plan.inserts[0];

    // A token we have never indexed must not render as an empty symbol.
    assert!(!position.symbol.is_empty());
    assert!(position.symbol.contains('…'));
}

// =============================================================================
// REFUSING TO INVENT MONEY
// =============================================================================

#[test]
fn a_round_without_a_cost_basis_carries_no_pnl_and_no_invested_figure() {
    // An airdropped token: real proceeds when sold, but no cost we ever paid.
    let mut source = round(MINT, "open-sig:MINT");
    source.basis_complete = false;
    source.invested_native = 0.0;
    source.realized_cost_native = 0.0;
    source.realized_pnl_native = None;
    source.average_entry_price_native = None;

    let plan = plan_position_writes(
        &[source],
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );
    let position = &plan.inserts[0];

    assert!(!position.basis_complete);
    assert_eq!(position.pnl, None);
    assert_eq!(position.pnl_percent, None);
    assert!(!position.has_trustworthy_pnl());
    // The proceeds themselves ARE observed, so they stay — it is only the basis and
    // anything derived from it that we refuse to state.
    assert_eq!(position.native_received, Some(3.0));
    assert_eq!(position.total_size_native, 0.0);
}

#[test]
fn history_that_does_not_reconcile_suppresses_the_pnl_even_with_a_complete_basis() {
    let mut source = round(MINT, "open-sig:MINT");
    source.history_complete = false;

    let plan = plan_position_writes(
        &[source],
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );
    let position = &plan.inserts[0];

    assert!(position.basis_complete);
    assert!(!position.history_complete);
    // We know what was paid, but not that we saw every movement — so the realized
    // number could be missing a sell, and stating it would be a guess.
    assert_eq!(position.pnl, None);
    assert!(!position.has_trustworthy_pnl());
}

#[test]
fn a_complete_round_reports_pnl_against_the_cost_actually_released() {
    let plan = plan_position_writes(
        &[round(MINT, "open-sig:MINT")],
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );
    let position = &plan.inserts[0];

    assert_eq!(position.pnl, Some(1.0));
    // 1 SOL gained against the 2 SOL of basis those sells released = 50%, NOT a
    // percentage of the whole original investment.
    assert_eq!(position.pnl_percent, Some(50.0));
    assert!(position.has_trustworthy_pnl());
}

#[test]
fn a_zero_cost_round_reports_no_percentage_rather_than_infinity() {
    let mut source = round(MINT, "open-sig:MINT");
    source.realized_cost_native = 0.0;
    source.realized_pnl_native = Some(3.0);

    let plan = plan_position_writes(
        &[source],
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );
    let position = &plan.inserts[0];

    assert_eq!(position.pnl, Some(3.0));
    assert_eq!(position.pnl_percent, None);
}

// =============================================================================
// NOT CLOBBERING WHAT THE SYNC DOES NOT OWN
// =============================================================================

#[test]
fn resyncing_unchanged_history_writes_nothing() {
    // The sync runs on every boot. If an unchanged wallet produced writes, every restart
    // would churn the whole positions table for nothing.
    let rounds = vec![
        round(MINT, "open-sig:MINT"),
        open_round(OTHER_MINT, "o:MINT"),
    ];
    let meta = metadata(MINT, false);
    let existing = materialise(&rounds, &meta);

    let plan = plan_position_writes(&rounds, &existing, &meta, &no_legs(), &no_busy(), now());

    assert!(plan.is_empty(), "unchanged history must produce no writes");
}

#[test]
fn a_row_the_user_archived_stays_archived_across_a_resync() {
    let rounds = vec![open_round(MINT, "open-sig:MINT")];
    let meta = metadata(MINT, false);
    let mut existing = materialise(&rounds, &meta);
    existing[0].archived = true;
    existing[0].archived_at = Some(now());

    // History moved on, so a write is due — but archival is the user's decision.
    let mut moved = open_round(MINT, "open-sig:MINT");
    moved.balance_raw = 500_000;
    let plan = plan_position_writes(&[moved], &existing, &meta, &no_legs(), &no_busy(), now());

    assert_eq!(plan.updates.len(), 1);
    assert!(plan.updates[0].position.archived);
    assert_eq!(plan.updates[0].position.archived_at, Some(now()));
    assert_eq!(plan.updates[0].position.id, Some(1));
}

#[test]
fn live_price_fields_survive_a_resync() {
    let rounds = vec![open_round(MINT, "open-sig:MINT")];
    let meta = metadata(MINT, false);
    let mut existing = materialise(&rounds, &meta);
    existing[0].current_price = Some(9.0);
    existing[0].price_highest = 12.0;
    existing[0].price_lowest = 1.0;
    existing[0].unrealized_pnl = Some(4.0);

    let mut moved = open_round(MINT, "open-sig:MINT");
    moved.balance_raw = 500_000;
    let plan = plan_position_writes(&[moved], &existing, &meta, &no_legs(), &no_busy(), now());

    let updated = &plan.updates[0].position;
    // These belong to the price updater. Resetting them would blank the dashboard's
    // current price and P&L on every restart.
    assert_eq!(updated.current_price, Some(9.0));
    assert_eq!(updated.price_highest, 12.0);
    assert_eq!(updated.price_lowest, 1.0);
    assert_eq!(updated.unrealized_pnl, Some(4.0));
}

// =============================================================================
// ADOPTING THE BOT'S OWN ROWS
// =============================================================================

#[test]
fn a_round_the_bot_executed_is_adopted_instead_of_duplicated() {
    // The trader's row and the reduced round describe ONE buy. Materialising the round
    // separately is what put every bot trade in the Positions list twice and counted it
    // twice in every portfolio total.
    let open = open_round(MINT, "open-sig:MINT");
    let bot_row = bot_position(&open);

    let plan = plan_position_writes(
        &[open],
        std::slice::from_ref(&bot_row),
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert!(plan.inserts.is_empty(), "no second row for the same buy");
    assert_eq!(plan.updates.len(), 1);

    let adopted = &plan.updates[0].position;
    assert_eq!(adopted.id, bot_row.id, "the trader's own row is updated");
    assert_eq!(adopted.round_key.as_deref(), Some("open-sig:MINT"));
    assert_eq!(adopted.origin, bot_row.origin, "origin is the trader's");
    assert_eq!(adopted.management, PositionManagement::AutoTrader);
    assert_eq!(
        adopted.total_size_native, 2.0,
        "the booked basis is untouched"
    );
    assert!(adopted.exit_time.is_none(), "still held, still open");
}

#[test]
fn an_adopted_row_is_matched_by_round_key_from_then_on() {
    let open = open_round(MINT, "open-sig:MINT");
    let mut adopted = bot_position(&open);
    adopted.round_key = Some("open-sig:MINT".to_owned());

    let plan = plan_position_writes(
        &[open],
        &[adopted],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert!(plan.inserts.is_empty());
    assert!(plan.updates.is_empty(), "nothing left to reconcile");
}

#[test]
fn a_bot_position_sold_somewhere_else_is_closed_from_wallet_history() {
    // The exact bug: sold in another wallet app, so the bot never booked an exit. The
    // round is closed on chain and the position must follow it.
    let closed = round(MINT, "open-sig:MINT");
    let bot_row = bot_position(&closed);

    let plan = plan_position_writes(
        &[closed],
        &[bot_row],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert!(plan.inserts.is_empty());
    let reconciled = &plan.updates[0].position;

    assert_eq!(
        reconciled.exit_time,
        Some(Utc.timestamp_opt(1_600_001_000, 0).unwrap()),
        "closed when the chain says it closed"
    );
    assert_eq!(reconciled.remaining_token_amount, Some(raw(0)));
    assert_eq!(
        reconciled.exit_transaction_signature.as_deref(),
        Some("close-sig")
    );
    assert!(
        reconciled.transaction_exit_verified,
        "a confirmed, fully-processed close needs no further verification"
    );
    assert_eq!(
        reconciled.closed_reason.as_deref(),
        Some("closed_externally")
    );
    assert_eq!(reconciled.native_received, Some(3.0));
    // Proceeds from the chain, basis from what the trader booked: 3 SOL out, 2 SOL in.
    assert_eq!(reconciled.pnl, Some(1.0));
    assert_eq!(reconciled.pnl_percent, Some(50.0));
    assert_eq!(reconciled.unrealized_pnl, None);
    // Still the trader's position in every other respect.
    assert_eq!(
        reconciled.origin,
        PositionOrigin::Auto {
            strategy_id: Some("momentum".to_owned())
        }
    );
}

#[test]
fn a_close_we_could_not_time_is_dated_by_the_last_time_we_saw_the_holding() {
    // `reconcile_with_wallet` closes a round the wallet no longer holds without
    // inventing a disposal: no closing signature, no block time, no proceeds. Stamping
    // "when we noticed" would date a holding abandoned months ago to today and rank it
    // above the wallet's genuinely most recent exit in the Closed tab.
    let mut vanished = round(MINT, "open-sig:MINT");
    vanished.closed_at = None;
    vanished.exit_signature = None;
    vanished.average_exit_price_native = None;
    vanished.realized_proceeds_native = 0.0;
    vanished.basis_complete = false;
    vanished.history_complete = false;
    vanished.events = vec![event(1_600_000_500)];

    let plan = plan_position_writes(
        &[vanished],
        &[bot_position(&round(MINT, "open-sig:MINT"))],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    let reconciled = &plan.updates[0].position;
    assert_eq!(
        reconciled.exit_time,
        Some(Utc.timestamp_opt(1_600_000_500, 0).unwrap()),
        "the last movement we observed is the honest close stamp"
    );
    assert_ne!(reconciled.exit_time, Some(now()));
    assert_eq!(reconciled.exit_transaction_signature, None);
    assert_eq!(
        reconciled.native_received, None,
        "no proceeds may be invented for a disposal we never saw"
    );
    assert_eq!(reconciled.pnl, None);
}

#[test]
fn a_partial_sale_elsewhere_lowers_the_holding_but_leaves_it_open() {
    let mut partly_sold = open_round(MINT, "open-sig:MINT");
    partly_sold.balance_raw = 400_000;

    let mut bot_row = bot_position(&open_round(MINT, "open-sig:MINT"));
    bot_row.remaining_token_amount = Some(raw(1_000_000));
    bot_row.total_exited_amount = raw(0);

    let plan = plan_position_writes(
        &[partly_sold],
        &[bot_row],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    let reconciled = &plan.updates[0].position;
    assert_eq!(reconciled.remaining_token_amount, Some(raw(400_000)));
    assert_eq!(reconciled.total_exited_amount, raw(600_000));
    assert!(reconciled.exit_time.is_none(), "still holding something");
}

/// The same round after the user bought 4x more of the mint in another wallet app: two
/// extra SOL-priced acquisitions the bot never executed, on top of the trader's own.
fn grown_round() -> LedgerRound {
    let mut grown = open_round(MINT, "open-sig:MINT");
    grown.balance_raw = 5_000_000;
    grown.total_acquired_raw = 5_000_000;
    grown.entry_count = 3;
    grown.invested_native = 8.0;
    grown.events = vec![
        acquisition("open-sig", LedgerEventKind::Entry, 1.0, 2.0),
        acquisition("outside-1", LedgerEventKind::Add, 2.0, 3.0),
        acquisition("outside-2", LedgerEventKind::Add, 2.0, 3.0),
    ];
    grown
}

/// One priced acquisition inside a round: `amount` whole tokens for `sol` SOL.
fn acquisition(signature: &str, kind: LedgerEventKind, amount: f64, sol: f64) -> LedgerEvent {
    LedgerEvent {
        signature: signature.to_owned(),
        slot: Some(1),
        block_time: Some(1_600_000_500),
        kind,
        amount,
        balance_after: amount,
        quote: Some(QuoteLeg {
            asset: QuoteAsset::Sol,
            amount: sol,
        }),
        price_native: Some(sol / amount),
        venue: Some("jupiter".to_owned()),
    }
}

#[test]
fn a_buy_made_elsewhere_grows_the_bot_s_own_position() {
    // The user bought more of a mint the bot already holds, from a phone wallet. It is
    // the SAME round, so the bot's row must carry the whole holding and the whole cost —
    // otherwise the Positions tab shows the pre-buy invested figure forever.
    let mut bot_row = bot_position(&open_round(MINT, "open-sig:MINT"));
    bot_row.id = Some(7);
    bot_row.remaining_token_amount = Some(raw(1_000_000));

    let legs = HashMap::from([(
        7i64,
        TraderLegs {
            entry_signatures: HashSet::from(["open-sig".to_owned()]),
            // Fee-exact, and slightly above the chain's 2.0 leg because the trader
            // booked what it actually paid.
            booked_invested_native: 2.01,
        },
    )]);

    let plan = plan_position_writes(
        &[grown_round()],
        &[bot_row],
        &metadata(MINT, false),
        &legs,
        &no_busy(),
        now(),
    );

    assert_eq!(plan.updates.len(), 1);
    let grown = &plan.updates[0].position;
    assert_eq!(grown.remaining_token_amount, Some(raw(5_000_000)));
    assert_eq!(grown.token_amount, Some(raw(5_000_000)));
    assert_eq!(grown.dca_count, 2);
    // The trader's own leg keeps its fee-exact number; the two outside buys come from
    // the chain. Never the round's 8.0, which would discard the fee.
    assert!(
        (grown.total_size_native - 8.01).abs() < 1e-9,
        "{:?}",
        grown.total_size_native
    );
    assert!(grown.exit_time.is_none());
}

#[test]
fn absorbing_an_outside_buy_is_idempotent() {
    // The second resync must plan no write at all: recomputing booked + external gives
    // the same number, where accumulating the difference would inflate it every pass.
    let mut bot_row = bot_position(&open_round(MINT, "open-sig:MINT"));
    bot_row.id = Some(7);
    bot_row.remaining_token_amount = Some(raw(1_000_000));

    let legs = HashMap::from([(
        7i64,
        TraderLegs {
            entry_signatures: HashSet::from(["open-sig".to_owned()]),
            booked_invested_native: 2.0,
        },
    )]);

    let first = plan_position_writes(
        &[grown_round()],
        &[bot_row],
        &metadata(MINT, false),
        &legs,
        &no_busy(),
        now(),
    );
    let settled = first.updates[0].position.clone();

    let second = plan_position_writes(
        &[grown_round()],
        &[settled],
        &metadata(MINT, false),
        &legs,
        &no_busy(),
        now(),
    );
    assert!(second.is_empty(), "{:?}", second.updates);
}

#[test]
fn an_unpriced_outside_buy_takes_the_holding_but_not_a_basis() {
    // An airdrop or a USD-quoted fill grew the holding. The tokens are real; their cost
    // in SOL is not knowable, so the row must stop claiming a P&L rather than show one
    // computed from half a basis.
    let mut grown = grown_round();
    grown.basis_complete = false;

    let mut bot_row = bot_position(&open_round(MINT, "open-sig:MINT"));
    bot_row.id = Some(7);
    bot_row.remaining_token_amount = Some(raw(1_000_000));

    let plan = plan_position_writes(
        &[grown],
        &[bot_row],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    let reconciled = &plan.updates[0].position;
    assert_eq!(reconciled.remaining_token_amount, Some(raw(5_000_000)));
    assert!(!reconciled.basis_complete);
    assert!(!reconciled.has_trustworthy_pnl());
    assert!(
        (reconciled.total_size_native - 2.0).abs() < 1e-9,
        "the trader's basis is left alone, never mixed with an unpriceable leg"
    );
}

#[test]
fn a_holding_that_grew_on_broken_history_is_not_claimed() {
    // The deltas do not reconcile with the balances we saw, so the extra tokens cannot
    // be attributed to anything. Shrink-only is the safe behaviour here.
    let mut grown = grown_round();
    grown.history_complete = false;

    let mut bot_row = bot_position(&open_round(MINT, "open-sig:MINT"));
    bot_row.id = Some(7);
    bot_row.remaining_token_amount = Some(raw(1_000_000));

    let plan = plan_position_writes(
        &[grown],
        &[bot_row],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert_eq!(plan.updates.len(), 1, "only the round key is stamped");
    assert_eq!(
        plan.updates[0].position.remaining_token_amount,
        Some(raw(1_000_000))
    );
    assert!((plan.updates[0].position.total_size_native - 2.0).abs() < 1e-9);
}

#[test]
fn a_position_that_booked_its_own_exit_is_never_rewritten() {
    let closed = round(MINT, "open-sig:MINT");
    let mut bot_row = bot_position(&closed);
    bot_row.round_key = Some("open-sig:MINT".to_owned());
    bot_row.exit_time = Some(Utc.timestamp_opt(1_600_000_900, 0).unwrap());
    bot_row.exit_transaction_signature = Some("our-own-close".to_owned());
    bot_row.transaction_exit_verified = true;
    bot_row.remaining_token_amount = Some(raw(0));
    bot_row.native_received = Some(2.9);
    bot_row.pnl = Some(0.85);

    let plan = plan_position_writes(
        &[closed],
        &[bot_row],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert!(
        plan.updates.is_empty(),
        "the trader's fee-exact close is the settled truth"
    );
    assert!(plan.inserts.is_empty());
}

#[test]
fn a_mint_with_a_swap_in_flight_is_left_entirely_alone() {
    let closed = round(MINT, "open-sig:MINT");
    let bot_row = bot_position(&closed);
    let busy = HashSet::from([MINT.to_owned()]);

    let plan = plan_position_writes(
        &[closed],
        &[bot_row],
        &metadata(MINT, false),
        &no_legs(),
        &busy,
        now(),
    );

    assert!(plan.updates.is_empty(), "the trader's swap lands first");
    assert!(
        plan.inserts.is_empty(),
        "and it must not be duplicated in the meantime"
    );
}

#[test]
fn a_buy_in_flight_before_its_row_is_saved_is_not_imported() {
    // The bot's entry swap confirmed, its row is not saved yet: no row matches the round.
    let open = open_round(MINT, "open-sig:MINT");
    let busy = HashSet::from([MINT.to_owned()]);

    let plan = plan_position_writes(
        std::slice::from_ref(&open),
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &busy,
        now(),
    );
    assert!(
        plan.inserts.is_empty(),
        "the bot's own buy is imported as a second row"
    );

    let settled = plan_position_writes(
        &[open],
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );
    assert_eq!(settled.inserts.len(), 1, "an outside buy is still imported");
}

#[test]
fn an_insert_planned_before_a_bot_swap_started_is_skipped_at_write_time() {
    common::run_isolated(
        "an_insert_planned_before_a_bot_swap_started_is_skipped_at_write_time",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");

            let open = open_round(
                common::TEST_MINT,
                &format!("open-sig:{}", common::TEST_MINT),
            );
            let plan = plan_position_writes(
                &[open],
                &[],
                &metadata(common::TEST_MINT, false),
                &no_legs(),
                &no_busy(),
                now(),
            );
            assert_eq!(plan.inserts.len(), 1);

            let _submitting = state::mark_swap_in_flight(ChainId::Solana, common::TEST_MINT);
            assert_eq!(
                apply_plan(plan).await,
                AppliedPlan {
                    inserted: 0,
                    updated: 0,
                    skipped: 1,
                }
            );
            assert!(
                db::load_all_positions()
                    .await
                    .expect("load positions")
                    .is_empty(),
                "a row was imported while a bot swap of the mint was in flight"
            );
        },
    );
}

#[test]
fn an_unverified_entry_is_left_alone_and_not_duplicated() {
    let open = open_round(MINT, "open-sig:MINT");
    let mut bot_row = bot_position(&open);
    bot_row.transaction_entry_verified = false;

    let plan = plan_position_writes(
        &[open],
        &[bot_row],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert!(plan.updates.is_empty());
    assert!(plan.inserts.is_empty());
}

#[test]
fn an_exit_awaiting_verification_is_left_to_the_verifier() {
    let closed = round(MINT, "open-sig:MINT");
    let mut bot_row = bot_position(&closed);
    bot_row.exit_transaction_signature = Some("submitted-sig".to_owned());
    bot_row.transaction_exit_verified = false;

    let plan = plan_position_writes(
        &[closed],
        &[bot_row],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert!(plan.updates.is_empty());
    assert!(plan.inserts.is_empty());
}

#[test]
fn a_row_is_adopted_by_at_most_one_round() {
    // Bought, sold, bought again with the SAME signature reused (contrived, but the
    // guard is what stops two rounds collapsing onto one row).
    let first = round(MINT, "open-sig:MINT");
    let mut second = open_round(MINT, "second-sig:MINT");
    second.entry_signature = Some("open-sig".to_owned());
    let bot_row = bot_position(&first);

    let plan = plan_position_writes(
        &[first, second],
        std::slice::from_ref(&bot_row),
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert_eq!(plan.updates.len(), 1, "the first round claims the row");
    assert_eq!(plan.inserts.len(), 1, "the second gets its own");
    assert_eq!(
        plan.inserts[0].round_key.as_deref(),
        Some("second-sig:MINT")
    );
}

#[test]
fn a_row_holding_a_different_mint_is_never_adopted() {
    // One signature moves both legs of a token -> token swap; only the row holding this
    // round's mint belongs to it.
    let open = open_round(MINT, "open-sig:MINT");
    let mut other_mint_row = bot_position(&open_round(OTHER_MINT, "open-sig:OTHER"));
    other_mint_row.mint = OTHER_MINT.to_owned();
    other_mint_row.entry_transaction_signature = Some("open-sig".to_owned());

    let plan = plan_position_writes(
        &[open],
        &[other_mint_row],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert!(plan.updates.is_empty());
    assert_eq!(plan.inserts.len(), 1);
    assert_eq!(plan.inserts[0].origin, PositionOrigin::External);
}

#[test]
fn a_frozen_account_flags_the_bot_position_too() {
    let open = open_round(MINT, "open-sig:MINT");
    let bot_row = bot_position(&open);

    let plan = plan_position_writes(
        &[open],
        &[bot_row],
        &metadata(MINT, true),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert!(plan.updates[0].position.is_frozen());
    assert!(
        plan.updates[0].position.exit_time.is_none(),
        "freezing is a flag, never a close"
    );
}

#[test]
fn an_entry_time_is_never_re_invented_for_a_round_with_no_block_time() {
    let mut undated = open_round(MINT, "genesis:MINT");
    undated.opened_at = None;

    let meta = metadata(MINT, false);
    let existing = materialise(&[undated.clone()], &meta);
    let first_entry_time = existing[0].entry_time;
    assert_eq!(
        first_entry_time,
        now(),
        "the first insert falls back to now"
    );

    // A later sync at a different wall-clock time must keep the timestamp the row
    // already has, or the position would appear to march forward on every restart.
    let later = Utc.timestamp_opt(1_800_000_000, 0).unwrap();
    let mut moved = undated;
    moved.balance_raw = 500_000;
    let plan = plan_position_writes(&[moved], &existing, &meta, &no_legs(), &no_busy(), later);

    assert_eq!(plan.updates.len(), 1);
    assert_eq!(plan.updates[0].position.entry_time, first_entry_time);
}

// =============================================================================
// FROZEN HOLDINGS
// =============================================================================

#[test]
fn a_frozen_holding_is_flagged_but_never_archived_or_closed() {
    let rounds = vec![open_round(MINT, "open-sig:MINT")];
    let plan = plan_position_writes(
        &rounds,
        &[],
        &metadata(MINT, true),
        &no_legs(),
        &no_busy(),
        now(),
    );
    let position = &plan.inserts[0];

    assert!(position.is_frozen());
    // Frozen means "you cannot sell this", not "we dealt with it for you". The row stays
    // open and visible; archiving it is the user's decision alone.
    assert!(!position.archived);
    assert!(!position.transaction_exit_verified);
    assert_eq!(position.remaining_token_amount, Some(raw(1_000_000)));
}

#[test]
fn a_closed_round_is_never_flagged_frozen() {
    // The token account may still be frozen, but a round with nothing left in it is not
    // an unsellable holding — flagging it would put a warning on settled history.
    let plan = plan_position_writes(
        &[round(MINT, "open-sig:MINT")],
        &[],
        &metadata(MINT, true),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert!(!plan.inserts[0].is_frozen());
    assert_eq!(plan.inserts[0].holding_state, None);
}

#[test]
fn thawing_a_holding_clears_the_flag() {
    let rounds = vec![open_round(MINT, "open-sig:MINT")];
    let existing = materialise(&rounds, &metadata(MINT, true));
    assert!(existing[0].is_frozen());

    let plan = plan_position_writes(
        &rounds,
        &existing,
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert_eq!(plan.updates.len(), 1);
    assert!(!plan.updates[0].position.is_frozen());
}

// =============================================================================
// CAPACITY
// =============================================================================

#[test]
fn a_wallet_derived_row_is_not_bot_capacity() {
    let plan = plan_position_writes(
        &[open_round(MINT, "open-sig:MINT")],
        &[],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    // `get_capacity_consuming_positions` filters on exactly this. A user holding twenty
    // tokens must not exhaust `max_open_positions` before the trader places a trade,
    // and these rows never consumed a semaphore permit to release later.
    assert!(plan.inserts[0].is_wallet_derived());
}

// =============================================================================
// END TO END, STILL PURE: DELTAS -> ROUNDS -> PLAN
// =============================================================================
//
// The reported failure in one test: the bot buys a token, the user sells it in another
// wallet app, and the position must close. Everything above tests the planner against
// hand-built rounds; this reduces real deltas first, so a change in either half that
// breaks the join shows up here.

/// A mint that is NOT a reference asset: the reducer skips SOL/wSOL/USDC/USDT, which
/// the planner-only tests above can use as placeholders but a reduction cannot.
const TRADED_MINT: &str = "MintA111111111111111111111111111111111111";

/// A trade delta for [`TRADED_MINT`], with the chain's own before/after balances.
fn token_delta(
    signature: &str,
    slot: u64,
    delta: i128,
    before: u128,
    after: u128,
) -> SubjectAssetDelta {
    SubjectAssetDelta {
        chain: ChainId::Solana,
        wallet_address: "wallet".to_owned(),
        signature: signature.to_owned(),
        mint: TRADED_MINT.to_owned(),
        slot: Some(slot),
        block_time: Some(slot as i64 * 100),
        tx_index: 0,
        delta_raw: delta,
        before_raw: Some(before),
        after_raw: Some(after),
        decimals: 6,
        kind: DeltaKind::Trade,
        venue: Some("raydium".to_owned()),
        fee_native_raw: Some(5_000),
        success: true,
    }
}

/// The SOL leg of the same trade, in whole SOL (negative when spending).
fn sol_delta(signature: &str, slot: u64, sol: f64) -> SubjectAssetDelta {
    SubjectAssetDelta {
        mint: NATIVE_SOL_SENTINEL.to_owned(),
        decimals: 9,
        delta_raw: (sol * 1_000_000_000.0) as i128,
        before_raw: None,
        after_raw: None,
        ..token_delta(signature, slot, 0, 0, 0)
    }
}

#[test]
fn a_bot_buy_then_a_sale_made_elsewhere_closes_exactly_one_position() {
    let deltas = vec![
        // The bot's own buy: 1 SOL for 2 tokens.
        token_delta("bot-buy", 100, 2_000_000, 0, 2_000_000),
        sol_delta("bot-buy", 100, -1.0),
        // Sold in another wallet app three slots later, for 1.5 SOL.
        token_delta("elsewhere-sell", 103, -2_000_000, 2_000_000, 0),
        sol_delta("elsewhere-sell", 103, 1.5),
    ];

    let rounds = reduce_rounds(&deltas);
    assert_eq!(rounds.len(), 1, "one buy and one sale is one round");
    assert!(!rounds[0].is_open);
    assert_eq!(rounds[0].round_key, format!("bot-buy:{TRADED_MINT}"));

    // The trader's row for that buy: open, verified, nothing exited.
    let mut bot_row = bot_position(&open_round(TRADED_MINT, "bot-buy:MINT"));
    bot_row.entry_transaction_signature = Some("bot-buy".to_owned());
    bot_row.total_size_native = 1.0;
    bot_row.remaining_token_amount = Some(raw(2_000_000));

    let plan = plan_position_writes(
        &rounds,
        std::slice::from_ref(&bot_row),
        &metadata(TRADED_MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert!(
        plan.inserts.is_empty(),
        "the round is the bot's own buy — importing it again is the duplicate bug"
    );
    assert_eq!(plan.updates.len(), 1);

    let closed = &plan.updates[0].position;
    assert_eq!(closed.id, bot_row.id);
    assert_eq!(closed.round_key, Some(format!("bot-buy:{TRADED_MINT}")));
    assert!(closed.exit_time.is_some(), "the position is closed");
    assert_eq!(closed.remaining_token_amount, Some(raw(0)));
    assert_eq!(
        closed.exit_transaction_signature.as_deref(),
        Some("elsewhere-sell")
    );
    assert_eq!(closed.closed_reason.as_deref(), Some("closed_externally"));
    assert_eq!(closed.native_received, Some(1.5));
    assert_eq!(closed.pnl, Some(0.5));
}

#[test]
fn a_ledger_write_keeps_a_booking_committed_after_the_plan() {
    common::run_isolated(
        "a_ledger_write_keeps_a_booking_committed_after_the_plan",
        || async {
            const PARTIAL: &str = "partial-exit-sig";
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            common::seed_decimals(common::TEST_MINT, 6);
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");

            let mut position = common::test_position(1.0, 1.0);
            position.id = None;
            position.token_amount = Some(RawAmount::new(1_000_000));
            position.remaining_token_amount = Some(RawAmount::new(1_000_000));
            let id = db::save_position(&position)
                .await
                .expect("persist test position");
            position.id = Some(id);
            state::add_position(position.clone()).await;

            // The chain already shows the partial sale the trader has yet to book.
            let round_key = format!("entry-sig:{}", common::TEST_MINT);
            let held = LedgerRound {
                entry_signature: position.entry_transaction_signature.clone(),
                balance_raw: 600_000,
                total_disposed_raw: 400_000,
                exit_count: 1,
                ..open_round(common::TEST_MINT, &round_key)
            };
            let existing = db::load_all_positions().await.expect("load positions");
            let plan = plan_position_writes(
                &[held],
                &existing,
                &metadata(common::TEST_MINT, false),
                &no_legs(),
                &no_busy(),
                now(),
            );
            assert_eq!(plan.updates.len(), 1, "the round claims the bot row");

            state::register_pending_partial_exit(PendingPartialExit {
                signature: PARTIAL.to_owned(),
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
            apply_transition(PositionTransition::PartialExitVerified {
                position_id: id,
                exit_amount: RawAmount::new(400_000),
                native_received: 0.8,
                effective_exit_price: 2.0,
                fee_raw: 5_000,
                exit_time: Utc::now(),
                exit_signature: PARTIAL.to_owned(),
                exit_percentage: 40.0,
                held_after: None,
            })
            .await
            .expect("the partial exit commits");

            assert_eq!(
                apply_plan(plan).await,
                AppliedPlan {
                    inserted: 0,
                    updated: 1,
                    skipped: 0,
                }
            );

            let stored = db::get_position_by_id(id)
                .await
                .expect("read stored position")
                .expect("position stored");
            assert_eq!(
                stored.native_received,
                Some(0.8),
                "the partial exit's proceeds vanished from the row"
            );
            assert_eq!(stored.partial_exit_count, 1);
            assert_eq!(stored.remaining_token_amount, Some(RawAmount::new(600_000)));
            assert_eq!(stored.total_exited_amount, RawAmount::new(400_000));
            assert_eq!(stored.round_key.as_deref(), Some(round_key.as_str()));

            let live = state::get_position_by_id(id)
                .await
                .expect("position in memory");
            assert_eq!(live.native_received, stored.native_received);
            assert_eq!(live.partial_exit_count, stored.partial_exit_count);
            assert_eq!(live.remaining_token_amount, stored.remaining_token_amount);
            assert_eq!(live.total_exited_amount, stored.total_exited_amount);
            assert_eq!(live.round_key, stored.round_key);
        },
    );
}

#[test]
fn a_ledger_close_of_a_bot_position_at_a_loss_past_the_limit_pauses_entries() {
    common::run_isolated(
        "a_ledger_close_of_a_bot_position_at_a_loss_past_the_limit_pauses_entries",
        || async {
            use screenerbot::trader::safety::loss_limit::{
                get_loss_limit_status, is_entry_blocked_by_loss_limit, reset_loss_limit_state,
            };
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            common::seed_decimals(common::TEST_MINT, 6);
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");
            common::set_config(|cfg| {
                cfg.trader.loss_limit_enabled = true;
                cfg.trader.loss_limit_sol = 0.5;
                cfg.trader.loss_limit_period_hours = 24;
                cfg.trader.loss_limit_auto_resume = true;
            });
            reset_loss_limit_state();

            let mut position = common::test_position(1.0, 1.0);
            position.id = None;
            position.token_amount = Some(RawAmount::new(1_000_000));
            position.remaining_token_amount = Some(RawAmount::new(1_000_000));
            let id = db::save_position(&position)
                .await
                .expect("persist test position");
            position.id = Some(id);
            state::add_position(position.clone()).await;

            // The tokens were sold elsewhere for 0.2 SOL, just now.
            let sold_elsewhere = LedgerRound {
                entry_signature: position.entry_transaction_signature.clone(),
                closed_at: Some(Utc::now().timestamp()),
                invested_native: 1.0,
                realized_proceeds_native: 0.2,
                realized_cost_native: 1.0,
                average_entry_price_native: Some(1.0),
                average_exit_price_native: Some(0.2),
                realized_pnl_native: Some(-0.8),
                ..round(
                    common::TEST_MINT,
                    &format!("entry-sig:{}", common::TEST_MINT),
                )
            };
            let existing = db::load_all_positions().await.expect("load positions");
            let plan = plan_position_writes(
                &[sold_elsewhere],
                &existing,
                &metadata(common::TEST_MINT, false),
                &no_legs(),
                &no_busy(),
                Utc::now(),
            );
            assert_eq!(plan.updates.len(), 1, "the round claims the bot row");
            assert!(!is_entry_blocked_by_loss_limit());

            assert_eq!(apply_plan(plan).await.updated, 1);
            let closed = db::get_position_by_id(id)
                .await
                .expect("read stored position")
                .expect("position stored");
            assert_eq!(closed.closed_reason.as_deref(), Some("closed_externally"));
            assert!(
                get_loss_limit_status().cumulative_loss_native > 0.5,
                "the ledger's loss is not counted"
            );
            assert!(
                is_entry_blocked_by_loss_limit(),
                "a loss past the limit closed by the ledger does not pause entries"
            );
        },
    );
}

#[test]
fn a_planned_update_skipped_at_write_time_is_not_counted_as_written() {
    common::run_isolated(
        "a_planned_update_skipped_at_write_time_is_not_counted_as_written",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            common::seed_decimals(common::TEST_MINT, 6);
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");

            let mut position = common::test_position(1.0, 1.0);
            position.id = None;
            position.token_amount = Some(RawAmount::new(1_000_000));
            position.remaining_token_amount = Some(RawAmount::new(1_000_000));
            let id = db::save_position(&position)
                .await
                .expect("persist test position");
            position.id = Some(id);
            state::add_position(position.clone()).await;

            let round_key = format!("entry-sig:{}", common::TEST_MINT);
            let held = LedgerRound {
                entry_signature: position.entry_transaction_signature.clone(),
                balance_raw: 600_000,
                total_disposed_raw: 400_000,
                exit_count: 1,
                ..open_round(common::TEST_MINT, &round_key)
            };
            let existing = db::load_all_positions().await.expect("load positions");
            let plan = plan_position_writes(
                &[held],
                &existing,
                &metadata(common::TEST_MINT, false),
                &no_legs(),
                &no_busy(),
                now(),
            );
            assert_eq!(plan.updates.len(), 1, "the round claims the bot row");

            // A partial exit is submitted after the plan: the row is busy at write time.
            state::mark_partial_exit_pending(common::TEST_MINT).await;

            assert_eq!(
                apply_plan(plan).await,
                AppliedPlan {
                    inserted: 0,
                    updated: 0,
                    skipped: 1,
                }
            );
            let stored = db::get_position_by_id(id)
                .await
                .expect("read stored position")
                .expect("position stored");
            assert_eq!(stored.round_key, None, "a busy row was rewritten");
            assert_eq!(
                stored.remaining_token_amount,
                Some(RawAmount::new(1_000_000))
            );
        },
    );
}

#[test]
fn a_ledger_write_leaves_a_bot_row_whose_fill_was_booked_after_the_history_was_read() {
    common::run_isolated(
        "a_ledger_write_leaves_a_bot_row_whose_fill_was_booked_after_the_history_was_read",
        || async {
            const PARTIAL: &str = "partial-exit-sig";
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            common::seed_decimals(common::TEST_MINT, 6);
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");

            let mut position = common::test_position(1.0, 1.0);
            position.id = None;
            position.token_amount = Some(RawAmount::new(1_000_000));
            position.remaining_token_amount = Some(RawAmount::new(1_000_000));
            let id = db::save_position(&position)
                .await
                .expect("persist test position");
            position.id = Some(id);
            state::add_position(position.clone()).await;

            // The history was read before the partial sale reached it: the round still
            // holds everything the bot bought.
            let round_key = format!("entry-sig:{}", common::TEST_MINT);
            let stale = LedgerRound {
                entry_signature: position.entry_transaction_signature.clone(),
                ..open_round(common::TEST_MINT, &round_key)
            };
            let existing = db::load_all_positions().await.expect("load positions");
            let plan = plan_position_writes(
                &[stale],
                &existing,
                &metadata(common::TEST_MINT, false),
                &no_legs(),
                &no_busy(),
                now(),
            )
            .booked_before(HashSet::new());
            assert_eq!(plan.updates.len(), 1, "the round claims the bot row");

            state::register_pending_partial_exit(PendingPartialExit {
                signature: PARTIAL.to_owned(),
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
            apply_transition(PositionTransition::PartialExitVerified {
                position_id: id,
                exit_amount: RawAmount::new(400_000),
                native_received: 0.8,
                effective_exit_price: 2.0,
                fee_raw: 5_000,
                exit_time: Utc::now(),
                exit_signature: PARTIAL.to_owned(),
                exit_percentage: 40.0,
                held_after: None,
            })
            .await
            .expect("the partial exit commits");
            assert!(
                !state::mints_with_pending_swaps()
                    .await
                    .contains(common::TEST_MINT),
                "nothing is in flight when the ledger writes"
            );

            assert_eq!(
                apply_plan(plan).await,
                AppliedPlan {
                    inserted: 0,
                    updated: 0,
                    skipped: 1,
                }
            );
            let stored = db::get_position_by_id(id)
                .await
                .expect("read stored position")
                .expect("position stored");
            assert_eq!(
                stored.round_key, None,
                "the row was reconciled to a stale round"
            );
            assert_eq!(stored.remaining_token_amount, Some(RawAmount::new(600_000)));
            assert_eq!(stored.total_exited_amount, RawAmount::new(400_000));
            assert_eq!(
                stored.total_size_native, 1.0,
                "the sold tokens came back as a buy"
            );
            assert_eq!(stored.native_received, Some(0.8));
        },
    );
}

/// A fill booked after the history was read holds back only the row it was booked on: the
/// ledger still writes every other row.
#[test]
fn a_fill_booked_after_the_history_was_read_holds_back_only_its_own_row() {
    common::run_isolated(
        "a_fill_booked_after_the_history_was_read_holds_back_only_its_own_row",
        || async {
            const PARTIAL: &str = "other-partial-exit-sig";
            const OTHER_MINT: &str = "OtherMint111111111111111111111111111111111";
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            common::seed_decimals(common::TEST_MINT, 6);
            common::seed_decimals(OTHER_MINT, 6);
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");

            let mut stored_ids = Vec::new();
            for mint in [common::TEST_MINT, OTHER_MINT] {
                let mut position = common::test_position(1.0, 1.0);
                position.id = None;
                position.mint = mint.to_owned();
                position.entry_transaction_signature = Some(format!("entry-sig-{mint}"));
                position.token_amount = Some(RawAmount::new(1_000_000));
                position.remaining_token_amount = Some(RawAmount::new(1_000_000));
                let id = db::save_position(&position)
                    .await
                    .expect("persist test position");
                position.id = Some(id);
                state::add_position(position).await;
                stored_ids.push(id);
            }
            let (id, other_id) = (stored_ids[0], stored_ids[1]);
            let position = state::get_position_by_id(id)
                .await
                .expect("position in memory");

            let round_key = format!("entry-sig:{}", common::TEST_MINT);
            let round = LedgerRound {
                entry_signature: position.entry_transaction_signature.clone(),
                ..open_round(common::TEST_MINT, &round_key)
            };
            let existing = db::load_all_positions().await.expect("load positions");
            let plan = plan_position_writes(
                &[round],
                &existing,
                &metadata(common::TEST_MINT, false),
                &no_legs(),
                &no_busy(),
                now(),
            )
            .booked_before(HashSet::new());
            assert_eq!(plan.updates.len(), 1, "the round claims the bot row");

            state::register_pending_partial_exit(PendingPartialExit {
                signature: PARTIAL.to_owned(),
                mint: OTHER_MINT.to_owned(),
                position_id: other_id,
                expected_exit_amount: RawAmount::new(400_000),
                requested_exit_percentage: 40.0,
                expiry_height: None,
                created_at: Utc::now(),
            })
            .await
            .expect("register pending partial exit");
            state::mark_partial_exit_pending(OTHER_MINT).await;
            apply_transition(PositionTransition::PartialExitVerified {
                position_id: other_id,
                exit_amount: RawAmount::new(400_000),
                native_received: 0.8,
                effective_exit_price: 2.0,
                fee_raw: 5_000,
                exit_time: Utc::now(),
                exit_signature: PARTIAL.to_owned(),
                exit_percentage: 40.0,
                held_after: None,
            })
            .await
            .expect("the other row's partial exit commits");

            assert_eq!(
                apply_plan(plan).await,
                AppliedPlan {
                    inserted: 0,
                    updated: 1,
                    skipped: 0,
                },
                "another row's fresh fill held this row back"
            );
            let stored = db::get_position_by_id(id)
                .await
                .expect("read stored position")
                .expect("position stored");
            assert_eq!(stored.round_key.as_deref(), Some(round_key.as_str()));
        },
    );
}

#[test]
fn a_bot_swap_in_flight_keeps_its_mint_busy_until_its_guard_drops() {
    const IN_FLIGHT_MINT: &str = "InFlightMint11111111111111111111111111111111";
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("create runtime");
    runtime.block_on(async {
        let busy = || async {
            state::mints_with_pending_swaps()
                .await
                .contains(IN_FLIGHT_MINT)
        };
        assert!(!busy().await);

        let first = state::mark_swap_in_flight(ChainId::Solana, IN_FLIGHT_MINT);
        let second = state::mark_swap_in_flight(ChainId::Solana, IN_FLIGHT_MINT);
        assert!(busy().await, "a submitted swap keeps its mint busy");

        drop(first);
        assert!(busy().await, "another swap of the mint is still in flight");
        drop(second);
        assert!(
            !busy().await,
            "the mint is free once every swap is recorded"
        );
    });
}

// =============================================================================
// A ROUND SHARED BY A CLOSED ROW AND THE OPEN ROW
// =============================================================================

/// The round a write-off and the open position of its mint share: the written-off row's
/// entry `a-entry` landed after the write-off and was handed to the open row, which bought
/// `b-entry`. The wallet never emptied in between, so both buys are one round, keyed by
/// the earlier one.
fn shared_round(mint: &str) -> LedgerRound {
    let mut shared = open_round(mint, &format!("a-entry:{mint}"));
    shared.entry_signature = Some("a-entry".to_owned());
    shared.balance_raw = 3_000_000;
    shared.total_acquired_raw = 3_000_000;
    shared.entry_count = 2;
    shared.invested_native = 2.0;
    shared.remaining_basis_native = 2.0;
    shared.events = vec![
        acquisition("a-entry", LedgerEventKind::Entry, 1.0, 1.0),
        acquisition("b-entry", LedgerEventKind::Add, 2.0, 1.0),
    ];
    shared
}

/// The shared round after the open row's whole holding was sold in another wallet app.
fn shared_round_sold_elsewhere(mint: &str) -> LedgerRound {
    let mut sold = shared_round(mint);
    sold.is_open = false;
    sold.closed_at = Some(1_600_002_000);
    sold.balance_raw = 0;
    sold.total_disposed_raw = 3_000_000;
    sold.exit_count = 1;
    sold.remaining_basis_native = 0.0;
    sold.realized_proceeds_native = 2.4;
    sold.realized_cost_native = 2.0;
    sold.average_exit_price_native = Some(0.8);
    sold.realized_pnl_native = Some(0.4);
    sold.exit_signature = Some("elsewhere-sell".to_owned());
    sold
}

/// The written-off row: its entry `a-entry` was handed over, so it holds nothing and is
/// closed and verified.
fn written_off_row(round: &LedgerRound, round_key: Option<&str>) -> Position {
    let mut row = bot_position(round);
    row.id = Some(1);
    row.entry_transaction_signature = Some("a-entry".to_owned());
    row.token_amount = Some(raw(1_000_000));
    row.remaining_token_amount = Some(raw(0));
    row.total_exited_amount = raw(1_000_000);
    row.total_size_native = 0.0;
    row.exit_time = Some(Utc.timestamp_opt(1_600_000_600, 0).unwrap());
    row.transaction_exit_verified = true;
    row.synthetic_exit = true;
    row.closed_reason = Some("force_closed".to_owned());
    row.round_key = round_key.map(str::to_owned);
    row
}

/// The open row of the mint, holding its own buy and the handed-over tokens.
fn open_row(round: &LedgerRound) -> Position {
    let mut row = bot_position(round);
    row.id = Some(2);
    row.entry_transaction_signature = Some("b-entry".to_owned());
    row.entry_time = Utc.timestamp_opt(1_600_000_700, 0).unwrap();
    row.token_amount = Some(raw(3_000_000));
    row.remaining_token_amount = Some(raw(3_000_000));
    row.total_size_native = 2.0;
    row.dca_count = 1;
    row
}

/// The open row's booked legs: its own buy, and the add the handover recorded under the
/// written-off row's entry.
fn open_row_legs() -> HashMap<i64, TraderLegs> {
    HashMap::from([(
        2i64,
        TraderLegs {
            entry_signatures: HashSet::from(["a-entry".to_owned(), "b-entry".to_owned()]),
            booked_invested_native: 2.0,
        },
    )])
}

/// Whichever way the round finds the closed row, by the key an earlier sync stamped on it
/// or by its entry signature, the round is reconciled on the open row: the open row takes
/// the key, the closed row is left as it is settled and gives the key up.
#[test]
fn a_round_shared_with_the_open_row_belongs_to_the_open_row_not_the_closed_one() {
    let round = shared_round(MINT);
    let round_key = round.round_key.clone();
    for keyed in [true, false] {
        let closed = written_off_row(&round, keyed.then_some(round_key.as_str()));
        let open = open_row(&round);

        let plan = plan_position_writes(
            std::slice::from_ref(&round),
            &[closed, open],
            &metadata(MINT, false),
            &open_row_legs(),
            &no_busy(),
            now(),
        );

        assert!(plan.inserts.is_empty(), "keyed: {keyed}");
        assert_eq!(plan.updates.len(), 1, "keyed: {keyed}");
        let reconciled = &plan.updates[0].position;
        assert_eq!(reconciled.id, Some(2), "the open row owns the round");
        assert_eq!(reconciled.round_key.as_deref(), Some(round_key.as_str()));
        assert!(reconciled.exit_time.is_none(), "the open row is still held");
        assert_eq!(reconciled.remaining_token_amount, Some(raw(3_000_000)));
        assert!((reconciled.total_size_native - 2.0).abs() < 1e-12);
        let expected_releases = if keyed {
            vec![RoundKeyRelease {
                position_id: 1,
                round_key: round_key.clone(),
            }]
        } else {
            Vec::new()
        };
        assert_eq!(plan.round_key_releases, expected_releases, "keyed: {keyed}");
    }
}

/// The open row's holding sold in another wallet app closes the open row, even when the
/// closed row still carries the round's key: the rule that an outside sale closes its
/// position holds for the open row too.
#[test]
fn an_outside_sale_of_a_shared_round_closes_the_open_row() {
    let round = shared_round_sold_elsewhere(MINT);
    let round_key = round.round_key.clone();
    let closed = written_off_row(&round, Some(round_key.as_str()));

    let plan = plan_position_writes(
        std::slice::from_ref(&round),
        &[closed, open_row(&round)],
        &metadata(MINT, false),
        &open_row_legs(),
        &no_busy(),
        now(),
    );

    assert_eq!(plan.updates.len(), 1);
    let sold = &plan.updates[0].position;
    assert_eq!(sold.id, Some(2));
    assert!(sold.exit_time.is_some(), "the open row is closed");
    assert_eq!(sold.closed_reason.as_deref(), Some("closed_externally"));
    assert_eq!(sold.native_received, Some(2.4));
    assert_eq!(
        plan.round_key_releases,
        vec![RoundKeyRelease {
            position_id: 1,
            round_key,
        }]
    );
}

/// A closed round never claims the open row of a later round: the open row's entry is not
/// one of its acquisitions, so the closed row keeps its own settled round.
#[test]
fn a_closed_row_s_own_round_never_takes_the_open_row_of_a_later_round() {
    let earlier = round(MINT, "open-sig:MINT");
    let mut closed = bot_position(&earlier);
    closed.round_key = Some("open-sig:MINT".to_owned());
    closed.exit_time = Some(Utc.timestamp_opt(1_600_000_900, 0).unwrap());
    closed.exit_transaction_signature = Some("our-own-close".to_owned());
    closed.transaction_exit_verified = true;
    closed.remaining_token_amount = Some(raw(0));
    let mut later = open_row(&shared_round(MINT));
    later.entry_transaction_signature = Some("later-entry".to_owned());

    let plan = plan_position_writes(
        &[earlier],
        &[closed, later],
        &metadata(MINT, false),
        &no_legs(),
        &no_busy(),
        now(),
    );

    assert!(plan.updates.is_empty());
    assert!(plan.inserts.is_empty());
    assert!(plan.round_key_releases.is_empty());
}

/// Written to storage, the key moves in one sync: the closed row gives it up before the
/// open row takes it, which the unique round-key index would refuse the other way round.
#[test]
fn a_sync_moves_a_shared_round_s_key_from_the_closed_row_to_the_open_row() {
    common::run_isolated(
        "a_sync_moves_a_shared_round_s_key_from_the_closed_row_to_the_open_row",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            common::seed_decimals(common::TEST_MINT, 6);
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");

            let round = shared_round(common::TEST_MINT);
            let round_key = round.round_key.clone();
            let mut closed = written_off_row(&round, Some(round_key.as_str()));
            closed.id = None;
            let closed_id = db::save_position(&closed)
                .await
                .expect("persist closed row");
            let mut open = open_row(&round);
            open.id = None;
            let open_id = db::save_position(&open).await.expect("persist open row");

            let existing = db::load_all_positions().await.expect("load positions");
            let plan = plan_position_writes(
                std::slice::from_ref(&round),
                &existing,
                &metadata(common::TEST_MINT, false),
                &no_legs(),
                &no_busy(),
                now(),
            );
            assert_eq!(
                apply_plan(plan).await,
                AppliedPlan {
                    inserted: 0,
                    updated: 1,
                    skipped: 0,
                }
            );

            let stored_closed = db::get_position_by_id(closed_id)
                .await
                .expect("read closed row")
                .expect("closed row stored");
            let stored_open = db::get_position_by_id(open_id)
                .await
                .expect("read open row")
                .expect("open row stored");
            assert_eq!(stored_closed.round_key, None);
            assert_eq!(stored_closed.exit_time, closed.exit_time);
            assert_eq!(stored_closed.remaining_token_amount, Some(raw(0)));
            assert_eq!(stored_open.round_key.as_deref(), Some(round_key.as_str()));
            assert!(stored_open.exit_time.is_none());
        },
    );
}
