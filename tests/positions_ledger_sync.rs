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
use screenerbot::positions::round_state::attributable_held;
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
        amount_raw: whole(1.0),
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

/// The exact raw size of `amount` whole tokens of a fixture round (6 decimals).
fn whole(amount: f64) -> RawAmount {
    RawAmount::new((amount * 1_000_000.0).round() as u128)
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
        amount_raw: whole(amount),
        balance_after: amount,
        quote: Some(QuoteLeg {
            asset: QuoteAsset::Sol,
            amount: sol,
        }),
        price_native: Some(sol / amount),
        venue: Some("jupiter".to_owned()),
    }
}

/// One priced disposal inside a round: `amount` whole tokens sold for `sol` SOL.
fn disposal(signature: &str, kind: LedgerEventKind, amount: f64, sol: f64) -> LedgerEvent {
    LedgerEvent {
        balance_after: 0.0,
        ..acquisition(signature, kind, amount, sol)
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
            exit_signatures: HashSet::new(),
            // Fee-exact, and slightly above the chain's 2.0 leg because the trader
            // booked what it actually paid.
            booked_invested_native: 2.01,
            booked_acquired: raw(1_000_000),
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
            exit_signatures: HashSet::new(),
            booked_invested_native: 2.0,
            booked_acquired: raw(1_000_000),
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
            common::start_events().await;

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
            assert_eq!(
                common::position_events("closed_externally").await,
                1,
                "the ledger's close is announced once"
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
    sold.events
        .push(disposal("elsewhere-sell", LedgerEventKind::Exit, 3.0, 2.4));
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
            exit_signatures: HashSet::new(),
            booked_invested_native: 2.0,
            booked_acquired: raw(3_000_000),
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

// =============================================================================
// A CLOSED ROW'S PART OF A ROUND STAYS WITH IT
// =============================================================================

/// The round of a closed row `a1` -> `x1` that left 1_000 raw behind, into which the open
/// row's buy `b1` landed: the wallet never emptied, so both are one round, keyed by `a1`.
fn dust_merged_round(mint: &str) -> LedgerRound {
    let mut merged = open_round(mint, &format!("a1:{mint}"));
    merged.entry_signature = Some("a1".to_owned());
    merged.balance_raw = 501_000;
    merged.total_acquired_raw = 1_500_000;
    merged.total_disposed_raw = 999_000;
    merged.entry_count = 2;
    merged.exit_count = 1;
    merged.invested_native = 1.5;
    merged.remaining_basis_native = 0.501;
    merged.realized_proceeds_native = 1.2;
    merged.realized_cost_native = 0.999;
    merged.average_exit_price_native = Some(1.2 / 0.999);
    merged.events = vec![
        acquisition("a1", LedgerEventKind::Entry, 1.0, 1.0),
        disposal("x1", LedgerEventKind::PartialExit, 0.999, 1.2),
        acquisition("b1", LedgerEventKind::Add, 0.5, 0.5),
    ];
    merged
}

/// Adds a priced acquisition to the end of an open round.
fn with_outside_buy(mut round: LedgerRound, signature: &str, amount: f64, sol: f64) -> LedgerRound {
    let bought = whole(amount).raw();
    round.balance_raw += bought;
    round.total_acquired_raw += bought;
    round.entry_count += 1;
    round.invested_native += sol;
    round.remaining_basis_native += sol;
    round
        .events
        .push(acquisition(signature, LedgerEventKind::Add, amount, sol));
    round
}

/// Closes an open round with a priced sale of everything it holds.
fn sold_outside(mut round: LedgerRound, signature: &str, sol: f64) -> LedgerRound {
    let sold = RawAmount::new(round.balance_raw).to_whole_units(round.decimals);
    round.total_disposed_raw += round.balance_raw;
    round.balance_raw = 0;
    round.is_open = false;
    round.closed_at = Some(1_600_002_000);
    round.exit_count += 1;
    round.realized_proceeds_native += sol;
    round.realized_cost_native += round.remaining_basis_native;
    round.remaining_basis_native = 0.0;
    round.average_exit_price_native = Some(
        round.realized_proceeds_native
            / RawAmount::new(round.total_disposed_raw).to_whole_units(round.decimals),
    );
    round.realized_pnl_native = Some(round.realized_proceeds_native - round.invested_native);
    round.exit_signature = Some(signature.to_owned());
    round
        .events
        .push(disposal(signature, LedgerEventKind::Exit, sold, sol));
    round
}

/// The closed row: bought `a1` for 1.0 SOL, sold it `x1` for 1.2 SOL, verified.
fn dust_closed_row(round: &LedgerRound) -> Position {
    let mut row = bot_position(round);
    row.id = Some(1);
    row.entry_transaction_signature = Some("a1".to_owned());
    row.token_amount = Some(raw(1_000_000));
    row.remaining_token_amount = Some(raw(0));
    row.total_exited_amount = raw(1_000_000);
    row.total_size_native = 1.0;
    row.entry_size_native = 1.0;
    row.exit_time = Some(Utc.timestamp_opt(1_600_000_600, 0).unwrap());
    row.exit_transaction_signature = Some("x1".to_owned());
    row.transaction_exit_verified = true;
    row.closed_reason = Some("exit_verified".to_owned());
    row.native_received = Some(1.2);
    row.pnl = Some(0.2);
    row.round_key = Some(round.round_key.clone());
    row
}

/// The open row: bought `b1`, 500_000 raw for 0.5 SOL.
fn merged_open_row(round: &LedgerRound) -> Position {
    let mut row = bot_position(round);
    row.id = Some(2);
    row.entry_transaction_signature = Some("b1".to_owned());
    row.entry_time = Utc.timestamp_opt(1_600_000_700, 0).unwrap();
    row.token_amount = Some(raw(500_000));
    row.remaining_token_amount = Some(raw(500_000));
    row.total_exited_amount = raw(0);
    row.total_size_native = 0.5;
    row.entry_size_native = 0.5;
    row.average_entry_price = 1.0;
    row.dca_count = 0;
    row.partial_exit_count = 0;
    row
}

/// Both rows' booked legs: the closed row's entry and exit, the open row's entry.
fn merged_legs() -> HashMap<i64, TraderLegs> {
    HashMap::from([
        (
            1i64,
            TraderLegs {
                entry_signatures: HashSet::from(["a1".to_owned()]),
                exit_signatures: HashSet::from(["x1".to_owned()]),
                booked_invested_native: 1.0,
                booked_acquired: raw(1_000_000),
            },
        ),
        (
            2i64,
            TraderLegs {
                entry_signatures: HashSet::from(["b1".to_owned()]),
                exit_signatures: HashSet::new(),
                booked_invested_native: 0.5,
                booked_acquired: raw(500_000),
            },
        ),
    ])
}

/// The open row's planned update, which must be the only one.
fn only_update(plan: &screenerbot::positions::ledger::SyncPlan) -> &Position {
    assert!(plan.inserts.is_empty());
    assert_eq!(plan.updates.len(), 1, "{:?}", plan.updates);
    &plan.updates[0].position
}

/// Sizes, basis and P&L of `row`, which the ledger must leave as booked.
fn books(row: &Position) -> impl PartialEq + std::fmt::Debug {
    (
        row.token_amount,
        row.remaining_token_amount,
        row.total_exited_amount,
        row.dca_count,
        row.total_size_native,
        row.average_entry_price,
        row.native_received,
        row.pnl,
        row.exit_time,
        row.basis_complete,
    )
}

#[test]
fn a_closed_row_is_never_resized_by_the_ledger() {
    // A verified close that left dust, and a write-off whose tokens stayed in the wallet:
    // each still carries the key of a round that is open, and an outside buy after the
    // close grew the round. The close is settled, so the ledger may stamp the key and
    // nothing else.
    let mut verified_round = dust_merged_round(MINT);
    verified_round.events.truncate(2);
    verified_round.balance_raw = 1_000;
    verified_round.total_acquired_raw = 1_000_000;
    verified_round.entry_count = 1;
    verified_round.invested_native = 1.0;
    let verified_round = with_outside_buy(verified_round, "u", 2.0, 1.0);
    let verified = dust_closed_row(&verified_round);

    let written_off_round = with_outside_buy(shared_round(MINT), "u", 2.0, 1.0);
    let written_off = written_off_row(
        &written_off_round,
        Some(written_off_round.round_key.as_str()),
    );

    for (round, row) in [(verified_round, verified), (written_off_round, written_off)] {
        let plan = plan_position_writes(
            std::slice::from_ref(&round),
            std::slice::from_ref(&row),
            &metadata(MINT, false),
            &merged_legs(),
            &no_busy(),
            now(),
        );
        assert!(plan.inserts.is_empty());
        for update in &plan.updates {
            assert_eq!(update.position.id, row.id);
            assert_eq!(
                books(&update.position),
                books(&row),
                "the ledger resized a closed row"
            );
        }
    }
}

#[test]
fn a_round_shared_with_a_closed_row_never_charges_its_legs_to_the_open_row() {
    let round = dust_merged_round(MINT);
    let closed = dust_closed_row(&round);
    let open = merged_open_row(&round);

    let plan = plan_position_writes(
        std::slice::from_ref(&round),
        &[closed, open.clone()],
        &metadata(MINT, false),
        &merged_legs(),
        &no_busy(),
        now(),
    );

    let reconciled = only_update(&plan);
    assert_eq!(reconciled.id, Some(2));
    assert_eq!(reconciled.round_key, Some(round.round_key.clone()));
    assert_eq!(
        books(reconciled),
        books(&open),
        "the closed row's legs were charged to the open row"
    );
    assert_eq!(
        plan.round_key_releases,
        vec![RoundKeyRelease {
            position_id: 1,
            round_key: round.round_key.clone(),
        }]
    );
}

#[test]
fn an_outside_buy_during_the_closed_row_s_life_stays_with_the_closed_row() {
    let mut round = dust_merged_round(MINT);
    round
        .events
        .insert(1, acquisition("u0", LedgerEventKind::Add, 0.2, 0.3));
    round.balance_raw += 200_000;
    round.total_acquired_raw += 200_000;
    round.entry_count += 1;
    round.invested_native += 0.3;
    let open = merged_open_row(&round);

    let plan = plan_position_writes(
        std::slice::from_ref(&round),
        &[dust_closed_row(&round), open.clone()],
        &metadata(MINT, false),
        &merged_legs(),
        &no_busy(),
        now(),
    );

    let reconciled = only_update(&plan);
    assert_eq!(reconciled.id, Some(2));
    assert_eq!(
        books(reconciled),
        books(&open),
        "a buy made before the open row's first trade was charged to it"
    );
}

/// The open row after an outside buy of 0.25 tokens for 0.4 SOL, made after its own entry.
fn grown_by_outside_buy() -> (LedgerRound, Vec<Position>) {
    let round = with_outside_buy(dust_merged_round(MINT), "u", 0.25, 0.4);
    let rows = vec![dust_closed_row(&round), merged_open_row(&round)];
    (round, rows)
}

#[test]
fn an_outside_buy_into_a_shared_round_grows_the_open_row_by_that_buy_only() {
    let (round, rows) = grown_by_outside_buy();

    let plan = plan_position_writes(
        std::slice::from_ref(&round),
        &rows,
        &metadata(MINT, false),
        &merged_legs(),
        &no_busy(),
        now(),
    );

    let grown = only_update(&plan);
    assert_eq!(grown.id, Some(2));
    assert_eq!(grown.token_amount, Some(raw(750_000)));
    assert_eq!(grown.remaining_token_amount, Some(raw(750_000)));
    assert_eq!(grown.total_exited_amount, raw(0));
    assert_eq!(grown.dca_count, 1);
    assert!(
        (grown.total_size_native - 0.9).abs() < 1e-12,
        "{}",
        grown.total_size_native
    );
    assert!(
        (grown.average_entry_price - 0.9 / 0.75).abs() < 1e-12,
        "{}",
        grown.average_entry_price
    );
    assert!(grown.basis_complete);
    assert!(grown.exit_time.is_none());
}

/// The handover round with an outside buy of 1.0 token for 0.5 SOL after the open row's
/// own entry.
fn handover_grown_by_outside_buy() -> (LedgerRound, Vec<Position>) {
    let round = with_outside_buy(shared_round(MINT), "outside-add", 1.0, 0.5);
    let rows = vec![
        written_off_row(&round, Some(round.round_key.as_str())),
        open_row(&round),
    ];
    (round, rows)
}

#[test]
fn a_handover_into_the_open_row_is_counted_once() {
    // `a-entry` is the written-off row's entry and an add the open row recorded under it:
    // the open row's legs supply it, so neither the chain's leg nor the record counts twice.
    let (round, rows) = handover_grown_by_outside_buy();

    let plan = plan_position_writes(
        std::slice::from_ref(&round),
        &rows,
        &metadata(MINT, false),
        &open_row_legs(),
        &no_busy(),
        now(),
    );

    let grown = only_update(&plan);
    assert_eq!(grown.id, Some(2));
    assert_eq!(grown.token_amount, Some(raw(4_000_000)));
    assert_eq!(grown.remaining_token_amount, Some(raw(4_000_000)));
    assert_eq!(grown.total_exited_amount, raw(0));
    assert_eq!(grown.dca_count, 2);
    assert!(
        (grown.total_size_native - 2.5).abs() < 1e-12,
        "{}",
        grown.total_size_native
    );
    assert!(
        (grown.average_entry_price - 2.5 / 4.0).abs() < 1e-12,
        "{}",
        grown.average_entry_price
    );
}

/// The dust-merged round after the closed row's late sale `x2` of its dust (booked on the
/// closed row), the open row's own sale `p1` of 0.2 tokens for 0.3 SOL, and the sale `s` of
/// the rest, 0.3 tokens, for 0.6 SOL in another wallet app.
fn shared_round_sold_outside() -> (LedgerRound, Vec<Position>, HashMap<i64, TraderLegs>) {
    let mut round = dust_merged_round(MINT);
    for (signature, amount, sol) in [("x2", 0.001, 0.001), ("p1", 0.2, 0.3)] {
        round.events.push(disposal(
            signature,
            LedgerEventKind::PartialExit,
            amount,
            sol,
        ));
        round.balance_raw -= whole(amount).raw();
        round.total_disposed_raw += whole(amount).raw();
        round.exit_count += 1;
        round.realized_proceeds_native += sol;
    }
    let round = sold_outside(round, "s", 0.6);

    let mut open = merged_open_row(&round);
    open.remaining_token_amount = Some(raw(300_000));
    open.total_exited_amount = raw(200_000);
    open.native_received = Some(0.3);
    open.partial_exit_count = 1;
    open.entry_fee_raw = Some(5_000);
    open.exit_fee_raw = Some(5_000);

    let mut legs = merged_legs();
    legs.get_mut(&1)
        .expect("closed row legs")
        .exit_signatures
        .insert("x2".to_owned());
    legs.get_mut(&2)
        .expect("open row legs")
        .exit_signatures
        .insert("p1".to_owned());
    let rows = vec![dust_closed_row(&round), open];
    (round, rows, legs)
}

#[test]
fn an_outside_sale_closing_a_shared_round_books_only_the_open_row_s_proceeds() {
    let (round, rows, legs) = shared_round_sold_outside();

    let plan = plan_position_writes(
        std::slice::from_ref(&round),
        &rows,
        &metadata(MINT, false),
        &legs,
        &no_busy(),
        now(),
    );

    let sold = only_update(&plan);
    assert_eq!(sold.id, Some(2));
    assert!(
        sold.exit_time.is_some(),
        "the outside sale closes the open row"
    );
    assert_eq!(sold.closed_reason.as_deref(), Some("closed_externally"));
    assert_eq!(sold.remaining_token_amount, Some(raw(0)));
    assert_eq!(sold.total_exited_amount, raw(500_000));
    let received = sold.native_received.expect("proceeds booked");
    assert!(
        (received - 0.9).abs() < 1e-12,
        "the closed row's sale was counted: {received}"
    );
    let exit_price = sold.exit_price.expect("exit price");
    assert!((exit_price - 0.6 / 0.3).abs() < 1e-9, "{exit_price}");
    // Realized P&L of a closed row: received minus invested minus the fees paid.
    let fees = screenerbot::positions::calculate_position_total_fees(sold);
    assert!(fees > 0.0);
    let expected = received - sold.total_size_native - fees;
    let pnl = sold.pnl.expect("pnl");
    assert!((pnl - expected).abs() < 1e-12, "{pnl} != {expected}");
    let percent = sold.pnl_percent.expect("pnl percent");
    assert!((percent - expected / sold.total_size_native * 100.0).abs() < 1e-9);
}

#[test]
fn reconciling_a_shared_round_twice_plans_no_second_write() {
    let (grown_round, grown_rows) = grown_by_outside_buy();
    let (handover_round, handover_rows) = handover_grown_by_outside_buy();
    let (sold_round, sold_rows, sold_legs) = shared_round_sold_outside();
    for (round, mut rows, legs) in [
        (grown_round, grown_rows, merged_legs()),
        (handover_round, handover_rows, open_row_legs()),
        (sold_round, sold_rows, sold_legs),
    ] {
        let first = plan_position_writes(
            std::slice::from_ref(&round),
            &rows,
            &metadata(MINT, false),
            &legs,
            &no_busy(),
            now(),
        );
        let settled = only_update(&first).clone();
        // As written: the closed row gave the key up, the open row took it.
        rows[0].round_key = None;
        rows[1] = settled;

        let second = plan_position_writes(
            std::slice::from_ref(&round),
            &rows,
            &metadata(MINT, false),
            &legs,
            &no_busy(),
            now(),
        );
        assert!(second.is_empty(), "{:?}", second.updates);
    }
}

#[test]
fn a_buy_outside_the_bot_before_its_first_buy_still_belongs_to_its_row() {
    // No other position of the mint booked any of the round: the round is the row's, and an
    // outside buy before the row's own entry is absorbed as it always was.
    let mut round = open_round(MINT, &format!("u0:{MINT}"));
    round.entry_signature = Some("u0".to_owned());
    round.balance_raw = 1_500_000;
    round.total_acquired_raw = 1_500_000;
    round.entry_count = 2;
    round.invested_native = 1.5;
    round.remaining_basis_native = 1.5;
    round.events = vec![
        acquisition("u0", LedgerEventKind::Entry, 1.0, 1.0),
        acquisition("b1", LedgerEventKind::Add, 0.5, 0.5),
    ];
    let open = merged_open_row(&round);
    let legs = HashMap::from([(2i64, merged_legs().remove(&2).expect("open row legs"))]);

    let plan = plan_position_writes(
        std::slice::from_ref(&round),
        &[open],
        &metadata(MINT, false),
        &legs,
        &no_busy(),
        now(),
    );

    let grown = only_update(&plan);
    assert_eq!(grown.id, Some(2));
    assert_eq!(grown.token_amount, Some(raw(1_500_000)));
    assert_eq!(grown.remaining_token_amount, Some(raw(1_500_000)));
    assert_eq!(grown.total_exited_amount, raw(0));
    assert_eq!(grown.dca_count, 1);
    assert!(
        (grown.total_size_native - 1.5).abs() < 1e-12,
        "{}",
        grown.total_size_native
    );
}

#[test]
fn a_shared_round_whose_legs_are_not_all_recorded_leaves_the_row_as_booked() {
    // The open row has no records, so what it booked of the round cannot be told apart from
    // what the closed row booked: its sizes and basis stay as booked.
    let legs = HashMap::from([(1i64, merged_legs().remove(&1).expect("closed row legs"))]);

    let grown = with_outside_buy(dust_merged_round(MINT), "u", 0.25, 0.4);
    let open = merged_open_row(&grown);
    let plan = plan_position_writes(
        std::slice::from_ref(&grown),
        &[dust_closed_row(&grown), open.clone()],
        &metadata(MINT, false),
        &legs,
        &no_busy(),
        now(),
    );
    let reconciled = only_update(&plan);
    assert_eq!(reconciled.round_key, Some(grown.round_key.clone()));
    assert_eq!(books(reconciled), books(&open), "neither grown nor shrunk");

    let sold = sold_outside(dust_merged_round(MINT), "s", 0.6);
    let plan = plan_position_writes(
        std::slice::from_ref(&sold),
        &[dust_closed_row(&sold), merged_open_row(&sold)],
        &metadata(MINT, false),
        &legs,
        &no_busy(),
        now(),
    );
    let closed = only_update(&plan);
    assert!(
        closed.exit_time.is_some(),
        "the outside sale still closes it"
    );
    assert_eq!(
        closed.native_received, None,
        "no proceeds can be attributed"
    );
    assert_eq!(closed.pnl, None);
    assert!(!closed.history_complete);
}

#[test]
fn a_shared_round_whose_open_row_lacks_its_entry_record_is_not_grown() {
    // The open row's entry `b1` has no record. An outside buy larger than what the row holds
    // must not replace the row's own tokens and cost with the outside buy.
    let legs = HashMap::from([(1i64, merged_legs().remove(&1).expect("closed row legs"))]);
    let grown = with_outside_buy(dust_merged_round(MINT), "u", 1.0, 0.4);
    let open = merged_open_row(&grown);

    let plan = plan_position_writes(
        std::slice::from_ref(&grown),
        &[dust_closed_row(&grown), open.clone()],
        &metadata(MINT, false),
        &legs,
        &no_busy(),
        now(),
    );

    let reconciled = only_update(&plan);
    assert_eq!(reconciled.id, Some(2));
    assert_eq!(
        reconciled.remaining_token_amount,
        open.remaining_token_amount
    );
    assert_eq!(reconciled.token_amount, open.token_amount);
    assert_eq!(reconciled.total_size_native, open.total_size_native);
    assert_eq!(reconciled.dca_count, open.dca_count);
}

#[test]
fn an_outside_sell_all_counts_an_unrecorded_partial_exit_once() {
    // The open row booked its partial exit `p1` for 0.3 SOL, but `p1` has no exit record:
    // the round's `p1` cannot be told apart from an outside sale, so the close books no
    // proceeds rather than counting `p1` a second time.
    let (round, rows, mut legs) = shared_round_sold_outside();
    legs.get_mut(&2)
        .expect("open row legs")
        .exit_signatures
        .remove("p1");

    let plan = plan_position_writes(
        std::slice::from_ref(&round),
        &rows,
        &metadata(MINT, false),
        &legs,
        &no_busy(),
        now(),
    );

    let sold = only_update(&plan);
    assert_eq!(sold.id, Some(2));
    assert!(sold.exit_time.is_some(), "the outside sale closes the row");
    assert_eq!(sold.native_received, Some(0.3));
    assert_eq!(sold.pnl, None);
    assert!(!sold.history_complete);
}

#[test]
fn an_outside_sale_of_part_of_a_shared_round_shrinks_the_open_row() {
    // After the open row's own entry, 0.2 tokens are sold for 0.3 SOL in another wallet app:
    // the round stays open and the open row follows the holding down.
    let mut round = dust_merged_round(MINT);
    round
        .events
        .push(disposal("s", LedgerEventKind::PartialExit, 0.2, 0.3));
    round.balance_raw -= whole(0.2).raw();
    round.total_disposed_raw += whole(0.2).raw();
    round.exit_count += 1;
    round.realized_proceeds_native += 0.3;
    let closed = dust_closed_row(&round);
    let open = merged_open_row(&round);

    let plan = plan_position_writes(
        std::slice::from_ref(&round),
        &[closed, open.clone()],
        &metadata(MINT, false),
        &merged_legs(),
        &no_busy(),
        now(),
    );

    let shrunk = only_update(&plan);
    assert_eq!(shrunk.id, Some(2), "the closed row gets no update");
    assert_eq!(
        shrunk.remaining_token_amount,
        Some(attributable_held(
            RawAmount::new(round.balance_raw),
            RawAmount::ZERO,
            raw(500_000),
        ))
    );
    assert!(shrunk.exit_time.is_none());
    assert_eq!(
        shrunk.native_received, open.native_received,
        "proceeds wait for the close"
    );
}

/// Stores a bot position of [`common::TEST_MINT`] whose entry `entry_signature` is not
/// verified yet, and loads it into memory.
async fn store_unverified(entry_signature: &str) -> i64 {
    let mut position = common::test_position(1.0, 1.0);
    position.id = None;
    position.entry_transaction_signature = Some(entry_signature.to_owned());
    position.transaction_entry_verified = false;
    let id = db::save_position(&position)
        .await
        .expect("persist test position");
    position.id = Some(id);
    state::add_position(position).await;
    id
}

/// Books, through the production transitions, a closed position that bought `a1` for 1.0
/// SOL and sold 999_000 raw `x1` for 1.2 SOL, then an open one that bought `b1`, 500_000
/// raw for 0.5 SOL. Returns their ids.
async fn book_dust_merged_rows() -> (i64, i64) {
    let closed = store_unverified("a1").await;
    apply_transition(PositionTransition::EntryVerified {
        position_id: closed,
        effective_entry_price: 1.0,
        token_amount_units: RawAmount::new(1_000_000),
        fee_raw: 5_000,
        native_size: 1.0,
        held_after: None,
    })
    .await
    .expect("the closed row's entry commits");
    apply_transition(PositionTransition::ExitVerified {
        position_id: closed,
        effective_exit_price: 1.2 / 0.999,
        native_received: 1.2,
        fee_raw: 5_000,
        exit_time: Utc::now(),
        exit_signature: "x1".to_owned(),
        exit_amount: RawAmount::new(999_000),
        held_after: None,
    })
    .await
    .expect("the closed row's exit commits");

    let open = store_unverified("b1").await;
    apply_transition(PositionTransition::EntryVerified {
        position_id: open,
        effective_entry_price: 1.0,
        token_amount_units: RawAmount::new(500_000),
        fee_raw: 5_000,
        native_size: 0.5,
        held_after: None,
    })
    .await
    .expect("the open row's entry commits");
    (closed, open)
}

/// The plan a sync makes from storage for `rounds` of the test mint.
async fn plan_from_storage(rounds: &[LedgerRound]) -> screenerbot::positions::ledger::SyncPlan {
    let legs = db::get_trader_swap_legs().await.expect("read booked legs");
    let booked_before: HashSet<String> = legs.iter().map(|leg| leg.signature.clone()).collect();
    let existing = db::load_all_positions().await.expect("load positions");
    plan_position_writes(
        rounds,
        &existing,
        &metadata(common::TEST_MINT, false),
        &TraderLegs::from_rows(legs),
        &no_busy(),
        now(),
    )
    .booked_before(booked_before)
}

#[test]
fn a_sync_charges_the_open_row_only_its_share_of_a_shared_round_in_storage() {
    common::run_isolated(
        "a_sync_charges_the_open_row_only_its_share_of_a_shared_round_in_storage",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            common::seed_decimals(common::TEST_MINT, 6);
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");
            let (closed_id, open_id) = book_dust_merged_rows().await;
            let closed_before = db::get_position_by_id(closed_id)
                .await
                .expect("read closed row")
                .expect("closed row stored");
            let open_before = db::get_position_by_id(open_id)
                .await
                .expect("read open row")
                .expect("open row stored");

            assert_eq!(
                apply_plan(plan_from_storage(&[dust_merged_round(common::TEST_MINT)]).await).await,
                AppliedPlan {
                    inserted: 0,
                    updated: 1,
                    skipped: 0,
                }
            );

            let stored_open = db::get_position_by_id(open_id)
                .await
                .expect("read open row")
                .expect("open row stored");
            assert_eq!(
                stored_open.round_key.as_deref(),
                Some(format!("a1:{}", common::TEST_MINT).as_str())
            );
            assert_eq!(stored_open.token_amount, Some(raw(500_000)));
            assert_eq!(books(&stored_open), books(&open_before));
            let stored_closed = db::get_position_by_id(closed_id)
                .await
                .expect("read closed row")
                .expect("closed row stored");
            assert_eq!(books(&stored_closed), books(&closed_before));
        },
    );
}

#[test]
fn a_record_booked_on_another_row_after_planning_holds_back_the_write() {
    common::run_isolated(
        "a_record_booked_on_another_row_after_planning_holds_back_the_write",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            common::seed_decimals(common::TEST_MINT, 6);
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");
            let (closed_id, open_id) = book_dust_merged_rows().await;
            let plan = plan_from_storage(&[dust_merged_round(common::TEST_MINT)]).await;
            assert_eq!(plan.updates.len(), 1, "the round claims the open row");

            // The closed row's dust is sold by a late bot sale after the history was read:
            // the round the plan was made from does not hold it yet.
            apply_transition(PositionTransition::ExitVerified {
                position_id: closed_id,
                effective_exit_price: 1.0,
                native_received: 0.001,
                fee_raw: 5_000,
                exit_time: Utc::now(),
                exit_signature: "x2".to_owned(),
                exit_amount: RawAmount::new(1_000),
                held_after: Some(RawAmount::new(500_000)),
            })
            .await
            .expect("the late sale commits");

            assert_eq!(
                apply_plan(plan).await,
                AppliedPlan {
                    inserted: 0,
                    updated: 0,
                    skipped: 1,
                }
            );
            let stored_open = db::get_position_by_id(open_id)
                .await
                .expect("read open row")
                .expect("open row stored");
            assert_eq!(stored_open.round_key, None, "the open row was reconciled");
        },
    );
}

#[test]
fn a_non_shared_round_writes_while_another_row_of_the_mint_is_unverified() {
    common::run_isolated(
        "a_non_shared_round_writes_while_another_row_of_the_mint_is_unverified",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            common::seed_decimals(common::TEST_MINT, 6);
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");

            // A closed position that bought `a1` and sold all of it `x1`: its round is its
            // alone, and the sync only stamps the round key on it.
            let closed_id = store_unverified("a1").await;
            apply_transition(PositionTransition::EntryVerified {
                position_id: closed_id,
                effective_entry_price: 1.0,
                token_amount_units: RawAmount::new(1_000_000),
                fee_raw: 5_000,
                native_size: 1.0,
                held_after: None,
            })
            .await
            .expect("the entry commits");
            apply_transition(PositionTransition::ExitVerified {
                position_id: closed_id,
                effective_exit_price: 1.2,
                native_received: 1.2,
                fee_raw: 5_000,
                exit_time: Utc::now(),
                exit_signature: "x1".to_owned(),
                exit_amount: RawAmount::new(1_000_000),
                held_after: None,
            })
            .await
            .expect("the exit commits");

            let round_key = format!("a1:{}", common::TEST_MINT);
            let mut round = round(common::TEST_MINT, &round_key);
            round.entry_signature = Some("a1".to_owned());
            round.exit_signature = Some("x1".to_owned());
            round.events = vec![
                acquisition("a1", LedgerEventKind::Entry, 1.0, 1.0),
                disposal("x1", LedgerEventKind::Exit, 1.0, 1.2),
            ];
            let legs = db::get_trader_swap_legs().await.expect("read booked legs");
            let booked_before: HashSet<String> =
                legs.iter().map(|leg| leg.signature.clone()).collect();
            let existing = db::load_all_positions().await.expect("load positions");
            let plan = plan_position_writes(
                std::slice::from_ref(&round),
                &existing,
                &metadata(common::TEST_MINT, false),
                &TraderLegs::from_rows(legs),
                &no_busy(),
                now(),
            )
            .booked_before(booked_before);
            assert_eq!(plan.updates.len(), 1, "the round claims the closed row");

            // A new position of the mint opens after planning, its entry not verified yet.
            store_unverified("c1").await;

            assert_eq!(
                apply_plan(plan).await,
                AppliedPlan {
                    inserted: 0,
                    updated: 1,
                    skipped: 0,
                }
            );
            let stored = db::get_position_by_id(closed_id)
                .await
                .expect("read closed row")
                .expect("closed row stored");
            assert_eq!(stored.round_key.as_deref(), Some(round_key.as_str()));
        },
    );
}

// =============================================================================
// A BOT ROW FOLLOWS THE ROUND OF ITS LATEST LEG
// =============================================================================

/// The bot's buy `b1` of 1.0 token for 1.0 SOL, all of it sold `s1` for 0.8 SOL in another
/// wallet app: the wallet emptied, so the round closed.
fn sold_out_round(mint: &str) -> LedgerRound {
    let mut bought = open_round(mint, &format!("b1:{mint}"));
    bought.entry_signature = Some("b1".to_owned());
    bought.invested_native = 1.0;
    bought.remaining_basis_native = 1.0;
    bought.average_entry_price_native = Some(1.0);
    bought.events = vec![acquisition("b1", LedgerEventKind::Entry, 1.0, 1.0)];
    sold_outside(bought, "s1", 0.8)
}

/// The next round of the mint: the bot's DCA `d1` of 0.5 tokens for 0.5 SOL into the empty
/// wallet, seen after everything in the round before it.
fn dca_round(mint: &str) -> LedgerRound {
    let mut bought = open_round(mint, &format!("d1:{mint}"));
    bought.opened_at = Some(1_600_003_000);
    bought.entry_signature = Some("d1".to_owned());
    bought.balance_raw = 500_000;
    bought.total_acquired_raw = 500_000;
    bought.invested_native = 0.5;
    bought.remaining_basis_native = 0.5;
    bought.average_entry_price_native = Some(1.0);
    bought.events = vec![LedgerEvent {
        block_time: Some(1_600_003_000),
        ..acquisition("d1", LedgerEventKind::Entry, 0.5, 0.5)
    }];
    bought
}

/// The bot's open row: entry `b1`, 1_000_000 raw for 1.0 SOL, and DCA `d1`, 500_000 raw for
/// 0.5 SOL, as booked before the sale made in another wallet app reached it.
fn following_row(round_key: Option<&str>) -> Position {
    let mut row = bot_position(&sold_out_round(MINT));
    row.id = Some(1);
    row.entry_transaction_signature = Some("b1".to_owned());
    row.token_amount = Some(raw(1_500_000));
    row.remaining_token_amount = Some(raw(1_500_000));
    row.total_exited_amount = raw(0);
    row.total_size_native = 1.5;
    row.entry_size_native = 1.0;
    row.average_entry_price = 1.0;
    row.dca_count = 1;
    row.partial_exit_count = 0;
    row.round_key = round_key.map(str::to_owned);
    row
}

/// The open row's booked legs: its entry `b1` and its DCA `d1`.
fn following_legs() -> HashMap<i64, TraderLegs> {
    HashMap::from([(
        1i64,
        TraderLegs {
            entry_signatures: HashSet::from(["b1".to_owned(), "d1".to_owned()]),
            exit_signatures: HashSet::new(),
            booked_invested_native: 1.5,
            booked_acquired: raw(1_500_000),
        },
    )])
}

/// The bot's buy `b1` of 1.0 token for 1.0 SOL and its full exit `x1` for 1.2 SOL.
fn exited_round(mint: &str) -> LedgerRound {
    let mut exited = round(mint, &format!("b1:{mint}"));
    exited.entry_signature = Some("b1".to_owned());
    exited.exit_signature = Some("x1".to_owned());
    exited.invested_native = 1.0;
    exited.realized_proceeds_native = 1.2;
    exited.realized_cost_native = 1.0;
    exited.average_entry_price_native = Some(1.0);
    exited.average_exit_price_native = Some(1.2);
    exited.realized_pnl_native = Some(0.2);
    exited.events = vec![
        acquisition("b1", LedgerEventKind::Entry, 1.0, 1.0),
        disposal("x1", LedgerEventKind::Exit, 1.0, 1.2),
    ];
    exited
}

/// The row after its full exit `x1` and the late DCA `d1` that reopened it: the closing sale
/// counts as a partial exit, and the row still carries the key of the round it closed.
fn reopened_row() -> Position {
    let mut row = following_row(Some(&format!("b1:{MINT}")));
    row.remaining_token_amount = Some(raw(500_000));
    row.total_exited_amount = raw(1_000_000);
    row.partial_exit_count = 1;
    row.native_received = Some(1.2);
    row
}

/// The reopened row's booked legs: entry `b1`, exit `x1`, DCA `d1`.
fn reopened_legs() -> HashMap<i64, TraderLegs> {
    let mut legs = following_legs();
    legs.get_mut(&1)
        .expect("row legs")
        .exit_signatures
        .insert("x1".to_owned());
    legs
}

/// The rows, legs and rounds of an open row whose holding spans two rounds.
type SpanningCase = (Vec<LedgerRound>, Vec<Position>, HashMap<i64, TraderLegs>);

/// The bot's DCA after a sale of everything in another wallet app.
fn dca_after_outside_sell_all(round_key: Option<&str>) -> SpanningCase {
    (
        vec![sold_out_round(MINT), dca_round(MINT)],
        vec![following_row(round_key)],
        following_legs(),
    )
}

/// The late DCA that reopened the row after its full exit.
fn late_dca_after_full_exit() -> SpanningCase {
    (
        vec![exited_round(MINT), dca_round(MINT)],
        vec![reopened_row()],
        reopened_legs(),
    )
}

/// The bot's DCA after a sale of everything in another wallet app, then the DCA's round sold
/// `s2` for 0.4 SOL in another wallet app too.
fn dca_round_sold_outside() -> SpanningCase {
    let mut row = following_row(Some(&format!("b1:{MINT}")));
    row.entry_fee_raw = Some(5_000);
    row.exit_fee_raw = Some(5_000);
    (
        vec![
            sold_out_round(MINT),
            sold_outside(dca_round(MINT), "s2", 0.4),
        ],
        vec![row],
        following_legs(),
    )
}

fn plan_case(case: &SpanningCase) -> screenerbot::positions::ledger::SyncPlan {
    let (rounds, rows, legs) = case;
    plan_position_writes(
        rounds,
        rows,
        &metadata(MINT, false),
        legs,
        &no_busy(),
        now(),
    )
}

#[test]
fn a_bot_buy_after_an_outside_sell_all_follows_the_new_round() {
    // Keyed by the earlier round, or adoptable by its entry signature there: either way the
    // earlier round neither closes the row nor leaves the DCA's round to a twin.
    let earlier_key = format!("b1:{MINT}");
    for round_key in [Some(earlier_key.as_str()), None] {
        let plan = plan_case(&dca_after_outside_sell_all(round_key));

        assert!(plan.round_key_releases.is_empty(), "key: {round_key:?}");
        let followed = only_update(&plan);
        assert_eq!(followed.id, Some(1));
        assert!(
            followed.exit_time.is_none(),
            "the row was closed against the earlier round, key: {round_key:?}"
        );
        assert_eq!(followed.round_key, Some(format!("d1:{MINT}")));
        assert_eq!(followed.remaining_token_amount, Some(raw(500_000)));
        assert_eq!(followed.total_exited_amount, raw(1_000_000));
        assert_eq!(followed.token_amount, Some(raw(1_500_000)));
        assert!((followed.total_size_native - 1.5).abs() < 1e-12);
        assert_eq!(followed.native_received, None);
        assert_eq!(followed.pnl, None);
        assert_eq!(followed.closed_reason, None);
    }
}

#[test]
fn a_late_dca_after_a_full_exit_keeps_the_reopened_row_on_its_new_round() {
    let case = late_dca_after_full_exit();
    let plan = plan_case(&case);

    let followed = only_update(&plan);
    assert_eq!(followed.id, Some(1));
    assert_eq!(followed.round_key, Some(format!("d1:{MINT}")));
    assert_eq!(
        books(followed),
        books(&case.1[0]),
        "the earlier round resized or closed the reopened row"
    );
    assert!(plan.round_key_releases.is_empty());
}

#[test]
fn a_closed_round_holding_only_a_bot_row_s_legs_is_never_imported() {
    // The reopened row closed again by the bot's own sale `x2` of the DCA's tokens: both
    // rounds are closed and every leg in them is booked on the row.
    let (_, mut rows, mut legs) = late_dca_after_full_exit();
    let row = &mut rows[0];
    row.exit_time = Some(Utc.timestamp_opt(1_600_003_500, 0).unwrap());
    row.exit_transaction_signature = Some("x2".to_owned());
    row.transaction_exit_verified = true;
    row.closed_reason = Some("exit_verified".to_owned());
    row.remaining_token_amount = Some(raw(0));
    row.total_exited_amount = raw(1_500_000);
    let closed = row.clone();
    legs.get_mut(&1)
        .expect("row legs")
        .exit_signatures
        .insert("x2".to_owned());
    // The DCA's round, closed by the bot's sale `x2` of 0.5 tokens for 0.6 SOL.
    let rounds = vec![exited_round(MINT), sold_outside(dca_round(MINT), "x2", 0.6)];

    let plan = plan_case(&(rounds, rows, legs));

    assert!(plan.inserts.is_empty(), "{:?}", plan.inserts);
    for update in &plan.updates {
        assert_eq!(books(&update.position), books(&closed));
    }
}

#[test]
fn an_outside_close_of_the_new_round_books_the_earlier_round_s_outside_proceeds_once() {
    let case = dca_round_sold_outside();
    let plan = plan_case(&case);

    let sold = only_update(&plan);
    assert_eq!(sold.id, Some(1));
    assert!(sold.exit_time.is_some(), "the outside sale closes the row");
    assert_eq!(sold.closed_reason.as_deref(), Some("closed_externally"));
    assert_eq!(sold.remaining_token_amount, Some(raw(0)));
    assert_eq!(sold.total_exited_amount, raw(1_500_000));
    let received = sold.native_received.expect("proceeds booked");
    assert!(
        (received - 1.2).abs() < 1e-12,
        "both outside sales are the row's: {received}"
    );
    let exit_price = sold.exit_price.expect("exit price");
    assert!((exit_price - 0.8).abs() < 1e-9, "{exit_price}");
    let fees = screenerbot::positions::calculate_position_total_fees(sold);
    assert!(fees > 0.0);
    let expected = received - sold.total_size_native - fees;
    let pnl = sold.pnl.expect("pnl");
    assert!((pnl - expected).abs() < 1e-12, "{pnl} != {expected}");

    let (rounds, _, legs) = case;
    let again = plan_case(&(rounds, vec![sold.clone()], legs));
    assert!(again.is_empty(), "{:?} {:?}", again.updates, again.inserts);
}

#[test]
fn a_row_reopened_after_a_ledger_close_is_never_charged_the_earlier_sale_twice() {
    // The ledger closed the row against the outside sale `s1`, and the late DCA `d1` reopened
    // it: `s1` counts as a partial exit the row has no record of, and its 0.8 SOL is booked.
    let mut row = following_row(Some(&format!("b1:{MINT}")));
    row.remaining_token_amount = Some(raw(500_000));
    row.total_exited_amount = raw(1_000_000);
    row.partial_exit_count = 1;
    row.native_received = Some(0.8);

    let held = plan_case(&(
        vec![sold_out_round(MINT), dca_round(MINT)],
        vec![row.clone()],
        following_legs(),
    ));
    let followed = only_update(&held);
    assert_eq!(followed.round_key, Some(format!("d1:{MINT}")));
    assert_eq!(
        books(followed),
        books(&row),
        "the row was not left as booked"
    );

    let sold = plan_case(&(
        vec![
            sold_out_round(MINT),
            sold_outside(dca_round(MINT), "s2", 0.4),
        ],
        vec![row],
        following_legs(),
    ));
    let closed = only_update(&sold);
    assert!(
        closed.exit_time.is_some(),
        "the outside sale closes the row"
    );
    assert_eq!(
        closed.native_received,
        Some(0.8),
        "a sale was counted twice"
    );
    assert_eq!(closed.pnl, None);
    assert!(!closed.history_complete);
}

#[test]
fn reconciling_a_row_across_rounds_twice_plans_no_second_write() {
    for case in [
        dca_after_outside_sell_all(Some(&format!("b1:{MINT}"))),
        late_dca_after_full_exit(),
        dca_round_sold_outside(),
    ] {
        let first = plan_case(&case);
        let settled = only_update(&first).clone();
        let (rounds, _, legs) = case;

        let second = plan_case(&(rounds, vec![settled], legs));
        assert!(
            second.is_empty(),
            "{:?} {:?}",
            second.updates,
            second.inserts
        );
    }
}

#[test]
fn a_late_dca_after_a_full_exit_follows_its_round_in_storage_without_a_loss() {
    common::run_isolated(
        "a_late_dca_after_a_full_exit_follows_its_round_in_storage_without_a_loss",
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
            // The late DCA that reopens the row takes a trading slot back.
            state::init_global_position_semaphore(1);

            let id = store_unverified("b1").await;
            apply_transition(PositionTransition::EntryVerified {
                position_id: id,
                effective_entry_price: 1.0,
                token_amount_units: RawAmount::new(1_000_000),
                fee_raw: 5_000,
                native_size: 1.0,
                held_after: None,
            })
            .await
            .expect("the entry commits");
            apply_transition(PositionTransition::ExitVerified {
                position_id: id,
                effective_exit_price: 1.2,
                native_received: 1.2,
                fee_raw: 5_000,
                exit_time: Utc::now(),
                exit_signature: "x1".to_owned(),
                exit_amount: RawAmount::new(1_000_000),
                held_after: None,
            })
            .await
            .expect("the full exit commits");
            let exited = exited_round(common::TEST_MINT);
            assert_eq!(
                apply_plan(plan_from_storage(std::slice::from_ref(&exited)).await)
                    .await
                    .updated,
                1,
                "the round keys the closed row"
            );

            apply_transition(PositionTransition::DcaVerified {
                position_id: id,
                tokens_bought: RawAmount::new(500_000),
                native_spent: 1.0,
                effective_price: 2.0,
                fee_raw: 5_000,
                dca_time: Utc::now(),
                dca_signature: "d1".to_owned(),
                held_after: Some(RawAmount::new(500_000)),
            })
            .await
            .expect("the late DCA commits");
            let reopened = db::get_position_by_id(id)
                .await
                .expect("read stored position")
                .expect("position stored");
            assert!(reopened.exit_time.is_none(), "the late DCA reopens the row");

            let plan = plan_from_storage(&[exited, dca_round(common::TEST_MINT)]).await;
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
            let live = state::get_position_by_id(id)
                .await
                .expect("position in memory");
            for position in [&stored, &live] {
                assert!(position.exit_time.is_none(), "the ledger closed the row");
                assert_eq!(
                    position.round_key,
                    Some(format!("d1:{}", common::TEST_MINT))
                );
                assert_eq!(position.remaining_token_amount, Some(raw(500_000)));
                assert_eq!(position.pnl, None);
            }
            let rows_of_mint = db::load_all_positions()
                .await
                .expect("load positions")
                .into_iter()
                .filter(|position| position.mint == common::TEST_MINT)
                .count();
            assert_eq!(rows_of_mint, 1, "the DCA's round was imported as a twin");
            assert_eq!(get_loss_limit_status().cumulative_loss_native, 0.0);
            assert!(!is_entry_blocked_by_loss_limit());
        },
    );
}

// =============================================================================
// A CLOSE IS ANNOUNCED BY THE WRITE THAT MADE IT
// =============================================================================

#[test]
fn a_key_stamp_on_a_position_the_bot_closed_announces_no_close() {
    common::run_isolated(
        "a_key_stamp_on_a_position_the_bot_closed_announces_no_close",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            common::seed_decimals(common::TEST_MINT, 6);
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");
            common::start_events().await;

            // A position that bought `a1` and sold all of it `x1`.
            let closed_id = store_unverified("a1").await;
            apply_transition(PositionTransition::EntryVerified {
                position_id: closed_id,
                effective_entry_price: 1.0,
                token_amount_units: RawAmount::new(1_000_000),
                fee_raw: 5_000,
                native_size: 1.0,
                held_after: None,
            })
            .await
            .expect("the entry commits");
            apply_transition(PositionTransition::ExitVerified {
                position_id: closed_id,
                effective_exit_price: 1.2,
                native_received: 1.2,
                fee_raw: 5_000,
                exit_time: Utc::now(),
                exit_signature: "x1".to_owned(),
                exit_amount: RawAmount::new(1_000_000),
                held_after: None,
            })
            .await
            .expect("the exit commits");

            let round_key = format!("a1:{}", common::TEST_MINT);
            let mut round = round(common::TEST_MINT, &round_key);
            round.entry_signature = Some("a1".to_owned());
            round.exit_signature = Some("x1".to_owned());
            round.events = vec![
                acquisition("a1", LedgerEventKind::Entry, 1.0, 1.0),
                disposal("x1", LedgerEventKind::Exit, 1.0, 1.2),
            ];

            assert_eq!(
                apply_plan(plan_from_storage(std::slice::from_ref(&round)).await)
                    .await
                    .updated,
                1,
                "the round keys the closed row"
            );
            let stored = db::get_position_by_id(closed_id)
                .await
                .expect("read closed row")
                .expect("closed row stored");
            assert_eq!(stored.round_key.as_deref(), Some(round_key.as_str()));
            assert_eq!(common::position_events("closed_externally").await, 0);
        },
    );
}

#[test]
fn an_archived_position_the_ledger_shrinks_announces_no_close() {
    common::run_isolated(
        "an_archived_position_the_ledger_shrinks_announces_no_close",
        || async {
            let _dir = common::isolated_env();
            let _cfg = common::config_guard();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            common::configure_own_wallet();
            common::seed_decimals(common::TEST_MINT, 6);
            screenerbot::positions::initialize_positions_database()
                .await
                .expect("initialise positions database");
            common::start_events().await;

            let id = store_unverified("a1").await;
            apply_transition(PositionTransition::EntryVerified {
                position_id: id,
                effective_entry_price: 1.0,
                token_amount_units: RawAmount::new(1_000_000),
                fee_raw: 5_000,
                native_size: 1.0,
                held_after: None,
            })
            .await
            .expect("the entry commits");
            assert!(db::set_position_archived_db(id, true)
                .await
                .expect("archive the row"));
            assert!(state::set_position_archived_in_memory(id, true).await);

            // 0.4 tokens of the holding were sold for 0.3 SOL in another wallet app.
            let mut round = open_round(common::TEST_MINT, &format!("a1:{}", common::TEST_MINT));
            round.entry_signature = Some("a1".to_owned());
            round.invested_native = 1.0;
            round.average_entry_price_native = Some(1.0);
            round.balance_raw = 600_000;
            round.total_disposed_raw = 400_000;
            round.exit_count = 1;
            round.realized_proceeds_native = 0.3;
            round.events = vec![
                acquisition("a1", LedgerEventKind::Entry, 1.0, 1.0),
                disposal("s", LedgerEventKind::PartialExit, 0.4, 0.3),
            ];

            assert_eq!(
                apply_plan(plan_from_storage(std::slice::from_ref(&round)).await)
                    .await
                    .updated,
                1,
                "the round shrinks the archived row"
            );
            let stored = db::get_position_by_id(id)
                .await
                .expect("read stored position")
                .expect("position stored");
            assert!(stored.exit_time.is_none(), "the row stays open");
            assert!(stored.archived);
            assert_eq!(stored.remaining_token_amount, Some(raw(600_000)));
            assert_eq!(common::position_events("closed_externally").await, 0);
        },
    );
}
