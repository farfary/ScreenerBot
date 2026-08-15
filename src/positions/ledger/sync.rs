//! Materialise reduced wallet-history rounds as `positions` rows.
//!
//! [`reduce_rounds`](super::reduce_rounds) answers "what rounds does this wallet's
//! history contain"; this module answers "which position rows must exist so the
//! dashboard shows them". It deliberately splits into a PURE planner
//! ([`plan_position_writes`]) and a thin impure applier ([`sync_wallet_history`]), so
//! every rule about what a wallet-derived row may claim is testable without a database,
//! a wallet or a clock.
//!
//! # Ownership boundary
//!
//! The planner only ever touches rows whose origin is [`PositionOrigin::External`] and
//! whose `round_key` matches a reduced round. A position the bot executed itself is
//! never read, rewritten or closed by this path — those rows are owned by the trader and
//! its verification queue.
//!
//! Within an external row, the ledger owns the *history* fields (amounts, basis,
//! realized P&L, signatures, open/closed) and nothing else. Live-price fields
//! (`current_price`, `price_highest`, `price_lowest`, unrealized P&L) belong to the price
//! updater, and `archived` belongs to the user — a resync must never move a row the user
//! archived back into the open list, and must never archive one on its own.
//!
//! Freezing is a FLAG, never an action: a frozen token account sets
//! `holding_state = "frozen"` so the user can see the holding cannot be sold and archive
//! it if they choose. Nothing here archives, hides or closes a frozen position.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};

use super::{reconcile_with_wallet, reduce_rounds, LedgerRound, WalletHolding};
use crate::logger::{self, LogTag};
use crate::positions::types::{Position, PositionManagement, PositionOrigin, HOLDING_STATE_FROZEN};

/// Display metadata for a mint, resolved once per sync from the tokens database.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoundMetadata {
    pub symbol: Option<String>,
    pub name: Option<String>,
    /// The wallet's token account for this mint is frozen and cannot be sold.
    pub frozen: bool,
}

/// What a sync must write. Nothing else in `positions` is touched.
#[derive(Debug, Default)]
pub struct SyncPlan {
    pub inserts: Vec<Position>,
    pub updates: Vec<Position>,
}

impl SyncPlan {
    pub fn is_empty(&self) -> bool {
        self.inserts.is_empty() && self.updates.is_empty()
    }
}

/// Outcome of one wallet-history sync, for logging and the debug API.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SyncSummary {
    pub rounds: usize,
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
}

/// Decide which external position rows must be created or rewritten.
///
/// Pure: the same rounds, existing rows, metadata and `now` always produce the same
/// plan. `now` is only ever used as the entry timestamp of a round whose history
/// contains no block time at all, and even then only for a row being created for the
/// first time — an existing row keeps the timestamp it already has, so repeated syncs
/// never make a position appear to drift forward in time.
pub fn plan_position_writes(
    rounds: &[LedgerRound],
    existing: &[Position],
    metadata: &HashMap<String, RoundMetadata>,
    now: DateTime<Utc>,
) -> SyncPlan {
    let by_round_key: HashMap<&str, &Position> = existing
        .iter()
        .filter(|position| position.is_wallet_derived())
        .filter_map(|position| {
            position
                .round_key
                .as_deref()
                .map(|round_key| (round_key, position))
        })
        .collect();

    let mut plan = SyncPlan::default();

    for round in rounds {
        let meta = metadata.get(&round.mint).cloned().unwrap_or_default();
        let current = by_round_key.get(round.round_key.as_str()).copied();
        let fresh = build_position(round, &meta, current, now);

        match current {
            Some(existing_row) => {
                if differs(existing_row, &fresh) {
                    plan.updates.push(fresh);
                }
            }
            None => plan.inserts.push(fresh),
        }
    }

    plan
}

/// Build the position row a round implies, carrying over everything the ledger does not
/// own from the row that already exists.
fn build_position(
    round: &LedgerRound,
    meta: &RoundMetadata,
    existing: Option<&Position>,
    now: DateTime<Utc>,
) -> Position {
    let entry_time = round
        .opened_at
        .and_then(|ts| DateTime::from_timestamp(ts, 0))
        .or_else(|| existing.map(|p| p.entry_time))
        .unwrap_or(now);
    let exit_time = round
        .closed_at
        .and_then(|ts| DateTime::from_timestamp(ts, 0))
        // A round the wallet no longer holds but whose closing transaction we never saw
        // is still closed; keep whatever close time the row already carried rather than
        // inventing one, and fall back to the entry time so ordering stays sane.
        .or_else(|| {
            if round.is_open {
                None
            } else {
                existing.and_then(|p| p.exit_time).or(Some(entry_time))
            }
        });

    // Only a complete basis may be presented as money. Without one, invested SOL is
    // zero (the reducer already zeroes it) and there is no P&L to show at all.
    let realized_pnl = if round.basis_complete && round.history_complete {
        round.realized_pnl_sol
    } else {
        None
    };
    let realized_pnl_percent = realized_pnl.and_then(|pnl| {
        (round.realized_cost_sol > super::DUST).then(|| pnl / round.realized_cost_sol * 100.0)
    });

    let entry_price = round.average_entry_price_sol.unwrap_or(0.0);

    Position {
        id: existing.and_then(|p| p.id),
        mint: round.mint.clone(),
        symbol: meta
            .symbol
            .clone()
            .or_else(|| existing.map(|p| p.symbol.clone()))
            .unwrap_or_else(|| short_mint(&round.mint)),
        name: meta
            .name
            .clone()
            .or_else(|| existing.map(|p| p.name.clone()))
            .unwrap_or_else(|| short_mint(&round.mint)),
        entry_price,
        entry_time,
        exit_price: round.average_exit_price_sol,
        exit_time,
        position_type: "buy".to_owned(),
        entry_size_sol: round.invested_sol,
        total_size_sol: round.invested_sol,
        // Price extremes belong to the price updater; seed them from the entry price so
        // a brand-new row is not stuck at zero.
        price_highest: existing.map(|p| p.price_highest).unwrap_or(entry_price),
        price_lowest: existing.map(|p| p.price_lowest).unwrap_or(entry_price),
        entry_transaction_signature: round.entry_signature.clone(),
        exit_transaction_signature: round.exit_signature.clone(),
        token_amount: Some(clamp_raw(round.total_acquired_raw)),
        effective_entry_price: round.average_entry_price_sol,
        effective_exit_price: round.average_exit_price_sol,
        sol_received: (round.exit_count > 0).then_some(round.realized_proceeds_sol),
        profit_target_min: None,
        profit_target_max: None,
        liquidity_tier: existing.and_then(|p| p.liquidity_tier.clone()),
        // The chain is the verification: these rounds are reduced from confirmed,
        // fully-processed transactions, so there is nothing left to verify. A closed
        // round MUST be marked verified or it would never appear in the Closed tab.
        transaction_entry_verified: true,
        transaction_exit_verified: !round.is_open,
        entry_fee_lamports: None,
        exit_fee_lamports: None,
        current_price: existing.and_then(|p| p.current_price),
        current_price_updated: existing.and_then(|p| p.current_price_updated),
        phantom_remove: false,
        phantom_confirmations: 0,
        phantom_first_seen: None,
        synthetic_exit: false,
        closed_reason: (!round.is_open).then(|| "wallet_history".to_owned()),
        pnl: realized_pnl,
        pnl_percent: realized_pnl_percent,
        unrealized_pnl: existing.and_then(|p| p.unrealized_pnl),
        unrealized_pnl_percent: existing.and_then(|p| p.unrealized_pnl_percent),
        remaining_token_amount: Some(clamp_raw(round.balance_raw)),
        total_exited_amount: clamp_raw(round.total_disposed_raw),
        average_exit_price: round.average_exit_price_sol,
        // The FIRST traded acquisition is the entry; the rest are adds. Likewise the
        // disposal that closed the round is the exit and the rest are partials.
        partial_exit_count: round
            .exit_count
            .saturating_sub(if round.is_open { 0 } else { 1 }),
        dca_count: round.entry_count.saturating_sub(1),
        average_entry_price: entry_price,
        last_dca_time: existing.and_then(|p| p.last_dca_time),
        // Archival is the user's decision alone — a resync never sets or clears it.
        archived: existing.is_some_and(|p| p.archived),
        archived_at: existing.and_then(|p| p.archived_at),
        origin: PositionOrigin::External,
        management: PositionManagement::UserOnly,
        round_key: Some(round.round_key.clone()),
        basis_complete: round.basis_complete,
        history_complete: round.history_complete,
        holding_state: (meta.frozen && round.is_open).then(|| HOLDING_STATE_FROZEN.to_owned()),
    }
}

/// True when the ledger-owned fields of `fresh` say something different from `existing`.
///
/// Deliberately ignores the price/archival fields `build_position` carries over, so an
/// idle wallet produces an EMPTY plan and the sync writes nothing at all.
fn differs(existing: &Position, fresh: &Position) -> bool {
    existing.entry_time != fresh.entry_time
        || existing.exit_time != fresh.exit_time
        || existing.transaction_exit_verified != fresh.transaction_exit_verified
        || existing.remaining_token_amount != fresh.remaining_token_amount
        || existing.total_exited_amount != fresh.total_exited_amount
        || existing.token_amount != fresh.token_amount
        || existing.dca_count != fresh.dca_count
        || existing.partial_exit_count != fresh.partial_exit_count
        || existing.basis_complete != fresh.basis_complete
        || existing.history_complete != fresh.history_complete
        || existing.holding_state != fresh.holding_state
        || existing.entry_transaction_signature != fresh.entry_transaction_signature
        || existing.exit_transaction_signature != fresh.exit_transaction_signature
        || existing.symbol != fresh.symbol
        || existing.name != fresh.name
        || !same_money(existing.total_size_sol, fresh.total_size_sol)
        || !same_money(existing.entry_price, fresh.entry_price)
        || !same_opt_money(existing.exit_price, fresh.exit_price)
        || !same_opt_money(existing.sol_received, fresh.sol_received)
        || !same_opt_money(existing.pnl, fresh.pnl)
}

fn same_money(a: f64, b: f64) -> bool {
    (a - b).abs() <= super::DUST
}

fn same_opt_money(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => same_money(a, b),
        _ => false,
    }
}

/// Raw base units are `u128` and exact; the position schema stores `u64`. A balance that
/// genuinely exceeds `u64::MAX` cannot exist for any real SPL mint (supply itself is
/// `u64`), so saturating is the correct total behaviour rather than a panic.
fn clamp_raw(raw: u128) -> u64 {
    u64::try_from(raw).unwrap_or(u64::MAX)
}

fn short_mint(mint: &str) -> String {
    if mint.len() <= 8 {
        return mint.to_owned();
    }
    format!("{}…{}", &mint[..4], &mint[mint.len() - 4..])
}

// =============================================================================
// APPLICATION
// =============================================================================

/// Reduce the wallet's processed history into rounds and write the missing/changed
/// external position rows.
///
/// Never fails the caller in a way that matters: every step degrades to "sync nothing"
/// rather than corrupting an existing position. Safe to call repeatedly — a wallet whose
/// history has not moved produces an empty plan and writes nothing.
pub async fn sync_wallet_history() -> Result<SyncSummary, String> {
    let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

    let Some(transactions_db) = crate::transactions::database::get_transaction_database().await
    else {
        return Err("Transaction database is not initialised".to_owned());
    };

    let deltas = transactions_db.get_subject_deltas(&wallet_address).await?;
    if deltas.is_empty() {
        return Ok(SyncSummary::default());
    }

    let mut rounds = reduce_rounds(&deltas);

    // On-chain balances win over anything we inferred. A failure here is not fatal: the
    // reduced rounds are still the best truth we have, they simply keep their observed
    // balances (and `reconcile_with_wallet` is what would have flagged a mismatch).
    let holdings = match crate::ata_operations::get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            logger::warning(
                LogTag::Positions,
                &format!("Wallet-history sync could not read token accounts: {e}"),
            );
            Vec::new()
        }
    };

    let frozen_mints: HashSet<String> = holdings
        .iter()
        .filter(|account| account.is_frozen)
        .map(|account| account.mint.clone())
        .collect();

    if !holdings.is_empty() {
        let wallet_holdings: Vec<WalletHolding> = holdings
            .iter()
            .filter(|account| !account.is_nft && account.balance > 0)
            .map(|account| WalletHolding {
                mint: account.mint.clone(),
                amount_raw: account.balance as u128,
                decimals: account.decimals,
            })
            .collect();
        reconcile_with_wallet(&mut rounds, &wallet_holdings);
    }

    if rounds.is_empty() {
        return Ok(SyncSummary::default());
    }

    let metadata = resolve_metadata(&rounds, &frozen_mints).await;

    // Existing rows come from the DATABASE, not in-memory state, so this sync is
    // independent of whether the positions service has finished loading yet. Both
    // orderings converge: a row inserted here is picked up by the later load, and a row
    // already in memory is updated in place below.
    let existing = crate::positions::db::load_all_positions().await?;
    let plan = plan_position_writes(&rounds, &existing, &metadata, Utc::now());

    let summary = SyncSummary {
        rounds: rounds.len(),
        inserted: plan.inserts.len(),
        updated: plan.updates.len(),
        unchanged: rounds.len() - plan.inserts.len() - plan.updates.len(),
    };

    apply_plan(plan).await;

    if summary.inserted > 0 || summary.updated > 0 {
        logger::info(
            LogTag::Positions,
            &format!(
                "Wallet-history sync: {} rounds ({} new, {} updated, {} unchanged)",
                summary.rounds, summary.inserted, summary.updated, summary.unchanged
            ),
        );
    }

    Ok(summary)
}

/// Resolve symbol/name/frozen for every mint in the plan in ONE query, not one per round.
async fn resolve_metadata(
    rounds: &[LedgerRound],
    frozen_mints: &HashSet<String>,
) -> HashMap<String, RoundMetadata> {
    let mints: Vec<String> = rounds
        .iter()
        .map(|round| round.mint.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let info = crate::tokens::database::get_token_info_batch_async(mints.clone())
        .await
        .unwrap_or_default();

    mints
        .into_iter()
        .map(|mint| {
            let (symbol, name) = info
                .get(&mint)
                .map(|(symbol, name, _image)| (symbol.clone(), name.clone()))
                .unwrap_or((None, None));
            let frozen = frozen_mints.contains(&mint);
            (
                mint,
                RoundMetadata {
                    symbol,
                    name,
                    frozen,
                },
            )
        })
        .collect()
}

/// Write the plan to the database and mirror it into in-memory state.
///
/// A single failed row is logged and skipped: one unwritable position must not abort the
/// import of the rest of the wallet's history.
async fn apply_plan(plan: SyncPlan) {
    for mut position in plan.inserts {
        match crate::positions::db::save_position(&position).await {
            Ok(id) => {
                position.id = Some(id);
                // Mirror into memory. If the positions service has not loaded yet, its
                // load replaces the whole vector from the database (which now contains
                // this row), so neither ordering can duplicate it.
                crate::positions::state::add_position(position).await;
            }
            Err(e) => logger::warning(
                LogTag::Positions,
                &format!(
                    "Wallet-history sync failed to insert round {}: {e}",
                    position.round_key.as_deref().unwrap_or("?")
                ),
            ),
        }
    }

    for position in plan.updates {
        let Some(id) = position.id else { continue };
        if let Err(e) = crate::positions::db::update_position(&position).await {
            logger::warning(
                LogTag::Positions,
                &format!("Wallet-history sync failed to update position {id}: {e}"),
            );
            continue;
        }
        crate::positions::state::update_position_state_by_id(id, |stored| {
            *stored = position.clone();
        })
        .await;
    }
}
