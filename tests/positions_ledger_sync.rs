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
use std::collections::HashMap;

use screenerbot::positions::ledger::sync::{plan_position_writes, RoundMetadata};
use screenerbot::positions::ledger::LedgerRound;
use screenerbot::positions::{Position, PositionManagement, PositionOrigin};

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

/// The row a first sync would create — used as the "already exists" input everywhere
/// below, so the tests never hand-build a 50-field `Position`.
fn materialise(rounds: &[LedgerRound], meta: &HashMap<String, RoundMetadata>) -> Vec<Position> {
    let plan = plan_position_writes(rounds, &[], meta, now());
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
    let plan = plan_position_writes(&rounds, &[], &metadata(MINT, false), now());

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
    let plan = plan_position_writes(&rounds, &[], &metadata(MINT, false), now());
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
    let plan = plan_position_writes(&rounds, &[], &metadata(MINT, false), now());
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
    let plan = plan_position_writes(&rounds, &[], &metadata(MINT, false), now());

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

    let plan = plan_position_writes(&[source], &[], &metadata(MINT, false), now());
    let position = &plan.inserts[0];

    assert_eq!(position.dca_count, 2);
    assert_eq!(position.partial_exit_count, 2);
}

#[test]
fn an_open_round_counts_every_disposal_as_a_partial() {
    // Nothing closed this round, so none of its sells was the final exit.
    let mut source = open_round(MINT, "open-sig:MINT");
    source.exit_count = 2;

    let plan = plan_position_writes(&[source], &[], &metadata(MINT, false), now());
    assert_eq!(plan.inserts[0].partial_exit_count, 2);
}

#[test]
fn a_mint_with_no_metadata_still_gets_a_readable_row() {
    let plan = plan_position_writes(&[round(MINT, "open-sig:MINT")], &[], &no_metadata(), now());
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

    let plan = plan_position_writes(&[source], &[], &metadata(MINT, false), now());
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

    let plan = plan_position_writes(&[source], &[], &metadata(MINT, false), now());
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

    let plan = plan_position_writes(&[source], &[], &metadata(MINT, false), now());
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

    let plan = plan_position_writes(&rounds, &existing, &meta, now());

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
    let plan = plan_position_writes(&[moved], &existing, &meta, now());

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
    let plan = plan_position_writes(&[moved], &existing, &meta, now());

    let updated = &plan.updates[0];
    // These belong to the price updater. Resetting them would blank the dashboard's
    // current price and P&L on every restart.
    assert_eq!(updated.current_price, Some(9.0));
    assert_eq!(updated.price_highest, 12.0);
    assert_eq!(updated.price_lowest, 1.0);
    assert_eq!(updated.unrealized_pnl, Some(4.0));
}

#[test]
fn a_position_the_bot_executed_is_never_matched_or_rewritten() {
    // Same mint, and even the same round key — but this row is ours, opened by the
    // trader. The ledger must not adopt, rewrite or close it.
    let rounds = vec![open_round(MINT, "open-sig:MINT")];
    let meta = metadata(MINT, false);
    let mut bot_row = materialise(&rounds, &meta)[0].clone();
    bot_row.origin = PositionOrigin::Auto { strategy_id: None };
    bot_row.management = PositionManagement::AutoTrader;

    let plan = plan_position_writes(&rounds, &[bot_row], &meta, now());

    assert!(
        plan.updates.is_empty(),
        "a bot-owned row must never be updated here"
    );
    assert_eq!(plan.inserts.len(), 1);
    assert_eq!(plan.inserts[0].origin, PositionOrigin::External);
    // The insert carries no id: it is a new row, not an overwrite of the bot's.
    assert_eq!(plan.inserts[0].id, None);
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
    let plan = plan_position_writes(&[moved], &existing, &meta, later);

    assert_eq!(plan.updates.len(), 1);
    assert_eq!(plan.updates[0].entry_time, first_entry_time);
}

// =============================================================================
// FROZEN HOLDINGS
// =============================================================================

#[test]
fn a_frozen_holding_is_flagged_but_never_archived_or_closed() {
    let rounds = vec![open_round(MINT, "open-sig:MINT")];
    let plan = plan_position_writes(&rounds, &[], &metadata(MINT, true), now());
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

    let plan = plan_position_writes(&rounds, &existing, &metadata(MINT, false), now());

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
        now(),
    );

    // `get_capacity_consuming_positions` filters on exactly this. A user holding twenty
    // tokens must not exhaust `max_open_positions` before the trader places a trade,
    // and these rows never consumed a semaphore permit to release later.
    assert!(plan.inserts[0].is_wallet_derived());
}
