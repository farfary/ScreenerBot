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
//! No database, no wallet, no clock: rounds are built inline and `now` is passed in.

use chrono::{DateTime, TimeZone, Utc};
use std::collections::{HashMap, HashSet};

use screenerbot::positions::ledger::reduce_rounds;
use screenerbot::positions::ledger::sync::{plan_position_writes, RoundMetadata};
use screenerbot::positions::ledger::LedgerRound;
use screenerbot::positions::{Position, PositionManagement, PositionOrigin};
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
        invested_sol: 2.0,
        remaining_basis_sol: 0.0,
        realized_proceeds_sol: 3.0,
        realized_cost_sol: 2.0,
        average_entry_price_sol: Some(2.0),
        average_exit_price_sol: Some(3.0),
        realized_pnl_sol: Some(1.0),
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
        remaining_basis_sol: 2.0,
        realized_proceeds_sol: 0.0,
        realized_cost_sol: 0.0,
        average_exit_price_sol: None,
        realized_pnl_sol: None,
        exit_signature: None,
        ..round(mint, round_key)
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
    position.total_size_sol = 2.0;
    position.entry_size_sol = 2.0;
    // Open on our books: no exit was ever executed or recorded by us.
    position.exit_time = None;
    position.exit_price = None;
    position.effective_exit_price = None;
    position.average_exit_price = None;
    position.exit_transaction_signature = None;
    position.transaction_exit_verified = false;
    position.closed_reason = None;
    position.sol_received = None;
    position.pnl = None;
    position.pnl_percent = None;
    position.remaining_token_amount = position.token_amount;
    position.total_exited_amount = 0;
    position
}

/// The row a first sync would create — used as the "already exists" input everywhere
/// below, so the tests never hand-build a 50-field `Position`.
fn materialise(rounds: &[LedgerRound], meta: &HashMap<String, RoundMetadata>) -> Vec<Position> {
    let plan = plan_position_writes(rounds, &[], meta, &no_busy(), now());
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

#[test]
fn a_new_round_becomes_an_external_user_only_position() {
    let rounds = vec![round(MINT, "open-sig:MINT")];
    let plan = plan_position_writes(&rounds, &[], &metadata(MINT, false), &no_busy(), now());

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
    let plan = plan_position_writes(&rounds, &[], &metadata(MINT, false), &no_busy(), now());
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
    let plan = plan_position_writes(&rounds, &[], &metadata(MINT, false), &no_busy(), now());
    let position = &plan.inserts[0];

    assert!(!position.transaction_exit_verified);
    assert!(position.exit_time.is_none());
    assert!(position.exit_transaction_signature.is_none());
    assert_eq!(position.remaining_token_amount, Some(1_000_000));
}

#[test]
fn a_second_round_in_the_same_mint_is_a_separate_row() {
    // Bought, sold everything, bought again. The re-buy must NOT resurrect the closed
    // round or inherit its cost basis — it is a new lifecycle with its own key.
    let rounds = vec![
        round(MINT, "first-sig:MINT"),
        open_round(MINT, "second-sig:MINT"),
    ];
    let plan = plan_position_writes(&rounds, &[], &metadata(MINT, false), &no_busy(), now());

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

    let plan = plan_position_writes(&[source], &[], &metadata(MINT, false), &no_busy(), now());
    let position = &plan.inserts[0];

    assert_eq!(position.dca_count, 2);
    assert_eq!(position.partial_exit_count, 2);
}

#[test]
fn an_open_round_counts_every_disposal_as_a_partial() {
    // Nothing closed this round, so none of its sells was the final exit.
    let mut source = open_round(MINT, "open-sig:MINT");
    source.exit_count = 2;

    let plan = plan_position_writes(&[source], &[], &metadata(MINT, false), &no_busy(), now());
    assert_eq!(plan.inserts[0].partial_exit_count, 2);
}

#[test]
fn a_mint_with_no_metadata_still_gets_a_readable_row() {
    let plan = plan_position_writes(
        &[round(MINT, "open-sig:MINT")],
        &[],
        &no_metadata(),
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
    source.invested_sol = 0.0;
    source.realized_cost_sol = 0.0;
    source.realized_pnl_sol = None;
    source.average_entry_price_sol = None;

    let plan = plan_position_writes(&[source], &[], &metadata(MINT, false), &no_busy(), now());
    let position = &plan.inserts[0];

    assert!(!position.basis_complete);
    assert_eq!(position.pnl, None);
    assert_eq!(position.pnl_percent, None);
    assert!(!position.has_trustworthy_pnl());
    // The proceeds themselves ARE observed, so they stay — it is only the basis and
    // anything derived from it that we refuse to state.
    assert_eq!(position.sol_received, Some(3.0));
    assert_eq!(position.total_size_sol, 0.0);
}

#[test]
fn history_that_does_not_reconcile_suppresses_the_pnl_even_with_a_complete_basis() {
    let mut source = round(MINT, "open-sig:MINT");
    source.history_complete = false;

    let plan = plan_position_writes(&[source], &[], &metadata(MINT, false), &no_busy(), now());
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
    source.realized_cost_sol = 0.0;
    source.realized_pnl_sol = Some(3.0);

    let plan = plan_position_writes(&[source], &[], &metadata(MINT, false), &no_busy(), now());
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

    let plan = plan_position_writes(&rounds, &existing, &meta, &no_busy(), now());

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
    let plan = plan_position_writes(&[moved], &existing, &meta, &no_busy(), now());

    assert_eq!(plan.updates.len(), 1);
    assert!(plan.updates[0].archived);
    assert_eq!(plan.updates[0].archived_at, Some(now()));
    assert_eq!(plan.updates[0].id, Some(1));
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
    let plan = plan_position_writes(&[moved], &existing, &meta, &no_busy(), now());

    let updated = &plan.updates[0];
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
        &no_busy(),
        now(),
    );

    assert!(plan.inserts.is_empty(), "no second row for the same buy");
    assert_eq!(plan.updates.len(), 1);

    let adopted = &plan.updates[0];
    assert_eq!(adopted.id, bot_row.id, "the trader's own row is updated");
    assert_eq!(adopted.round_key.as_deref(), Some("open-sig:MINT"));
    assert_eq!(adopted.origin, bot_row.origin, "origin is the trader's");
    assert_eq!(adopted.management, PositionManagement::AutoTrader);
    assert_eq!(adopted.total_size_sol, 2.0, "the booked basis is untouched");
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
        &no_busy(),
        now(),
    );

    assert!(plan.inserts.is_empty());
    let reconciled = &plan.updates[0];

    assert_eq!(
        reconciled.exit_time,
        Some(Utc.timestamp_opt(1_600_001_000, 0).unwrap()),
        "closed when the chain says it closed"
    );
    assert_eq!(reconciled.remaining_token_amount, Some(0));
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
    assert_eq!(reconciled.sol_received, Some(3.0));
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
fn a_close_we_could_not_time_is_stamped_when_we_noticed_it() {
    // `reconcile_with_wallet` closes a round the wallet no longer holds without
    // inventing a disposal: no closing signature, no block time, no proceeds.
    let mut vanished = round(MINT, "open-sig:MINT");
    vanished.closed_at = None;
    vanished.exit_signature = None;
    vanished.average_exit_price_sol = None;
    vanished.realized_proceeds_sol = 0.0;
    vanished.basis_complete = false;
    vanished.history_complete = false;

    let plan = plan_position_writes(
        &[vanished],
        &[bot_position(&round(MINT, "open-sig:MINT"))],
        &metadata(MINT, false),
        &no_busy(),
        now(),
    );

    let reconciled = &plan.updates[0];
    assert_eq!(reconciled.exit_time, Some(now()));
    assert_eq!(reconciled.exit_transaction_signature, None);
    assert_eq!(
        reconciled.sol_received, None,
        "no proceeds may be invented for a disposal we never saw"
    );
    assert_eq!(reconciled.pnl, None);
}

#[test]
fn a_partial_sale_elsewhere_lowers_the_holding_but_leaves_it_open() {
    let mut partly_sold = open_round(MINT, "open-sig:MINT");
    partly_sold.balance_raw = 400_000;

    let mut bot_row = bot_position(&open_round(MINT, "open-sig:MINT"));
    bot_row.remaining_token_amount = Some(1_000_000);
    bot_row.total_exited_amount = 0;

    let plan = plan_position_writes(
        &[partly_sold],
        &[bot_row],
        &metadata(MINT, false),
        &no_busy(),
        now(),
    );

    let reconciled = &plan.updates[0];
    assert_eq!(reconciled.remaining_token_amount, Some(400_000));
    assert_eq!(reconciled.total_exited_amount, 600_000);
    assert!(reconciled.exit_time.is_none(), "still holding something");
}

#[test]
fn a_holding_that_grew_elsewhere_is_never_claimed_by_the_position() {
    // Tokens bought in another app are not this position's to sell.
    let mut grown = open_round(MINT, "open-sig:MINT");
    grown.balance_raw = 5_000_000;

    let mut bot_row = bot_position(&open_round(MINT, "open-sig:MINT"));
    bot_row.remaining_token_amount = Some(1_000_000);

    let plan = plan_position_writes(
        &[grown],
        &[bot_row],
        &metadata(MINT, false),
        &no_busy(),
        now(),
    );

    assert_eq!(plan.updates.len(), 1, "only the round key is stamped");
    assert_eq!(plan.updates[0].remaining_token_amount, Some(1_000_000));
}

#[test]
fn a_position_that_booked_its_own_exit_is_never_rewritten() {
    let closed = round(MINT, "open-sig:MINT");
    let mut bot_row = bot_position(&closed);
    bot_row.round_key = Some("open-sig:MINT".to_owned());
    bot_row.exit_time = Some(Utc.timestamp_opt(1_600_000_900, 0).unwrap());
    bot_row.exit_transaction_signature = Some("our-own-close".to_owned());
    bot_row.transaction_exit_verified = true;
    bot_row.remaining_token_amount = Some(0);
    bot_row.sol_received = Some(2.9);
    bot_row.pnl = Some(0.85);

    let plan = plan_position_writes(
        &[closed],
        &[bot_row],
        &metadata(MINT, false),
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

    let plan = plan_position_writes(&[closed], &[bot_row], &metadata(MINT, false), &busy, now());

    assert!(plan.updates.is_empty(), "the trader's swap lands first");
    assert!(
        plan.inserts.is_empty(),
        "and it must not be duplicated in the meantime"
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
        &no_busy(),
        now(),
    );

    assert!(plan.updates[0].is_frozen());
    assert!(
        plan.updates[0].exit_time.is_none(),
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
    let plan = plan_position_writes(&[moved], &existing, &meta, &no_busy(), later);

    assert_eq!(plan.updates.len(), 1);
    assert_eq!(plan.updates[0].entry_time, first_entry_time);
}

// =============================================================================
// FROZEN HOLDINGS
// =============================================================================

#[test]
fn a_frozen_holding_is_flagged_but_never_archived_or_closed() {
    let rounds = vec![open_round(MINT, "open-sig:MINT")];
    let plan = plan_position_writes(&rounds, &[], &metadata(MINT, true), &no_busy(), now());
    let position = &plan.inserts[0];

    assert!(position.is_frozen());
    // Frozen means "you cannot sell this", not "we dealt with it for you". The row stays
    // open and visible; archiving it is the user's decision alone.
    assert!(!position.archived);
    assert!(!position.transaction_exit_verified);
    assert_eq!(position.remaining_token_amount, Some(1_000_000));
}

#[test]
fn a_closed_round_is_never_flagged_frozen() {
    // The token account may still be frozen, but a round with nothing left in it is not
    // an unsellable holding — flagging it would put a warning on settled history.
    let plan = plan_position_writes(
        &[round(MINT, "open-sig:MINT")],
        &[],
        &metadata(MINT, true),
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
        &no_busy(),
        now(),
    );

    assert_eq!(plan.updates.len(), 1);
    assert!(!plan.updates[0].is_frozen());
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
        fee_lamports: Some(5_000),
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
    bot_row.total_size_sol = 1.0;
    bot_row.remaining_token_amount = Some(2_000_000);

    let plan = plan_position_writes(
        &rounds,
        std::slice::from_ref(&bot_row),
        &metadata(TRADED_MINT, false),
        &no_busy(),
        now(),
    );

    assert!(
        plan.inserts.is_empty(),
        "the round is the bot's own buy — importing it again is the duplicate bug"
    );
    assert_eq!(plan.updates.len(), 1);

    let closed = &plan.updates[0];
    assert_eq!(closed.id, bot_row.id);
    assert_eq!(closed.round_key, Some(format!("bot-buy:{TRADED_MINT}")));
    assert!(closed.exit_time.is_some(), "the position is closed");
    assert_eq!(closed.remaining_token_amount, Some(0));
    assert_eq!(
        closed.exit_transaction_signature.as_deref(),
        Some("elsewhere-sell")
    );
    assert_eq!(closed.closed_reason.as_deref(), Some("closed_externally"));
    assert_eq!(closed.sol_received, Some(1.5));
    assert_eq!(closed.pnl, Some(0.5));
}
