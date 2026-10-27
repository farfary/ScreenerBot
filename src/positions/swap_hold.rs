// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Holds on position swaps that ended with no proof either way: a partial exit whose last send failed with no signature holds the mint's partial exits and its wallet reconciliation until the chain proves whether it sold.

use std::sync::LazyLock;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::chains::{RawAmount, SettlementReader};
use crate::logger::{self, LogTag};

use super::db;
use super::error::{Error, Result};
pub use super::types::PartialExitHold;

/// How long a hold waits for a settled reading before it is released as unknown. A Solana
/// transaction stays valid for 150 blocks, one to two minutes, so the limit spans several
/// validity windows plus finalization lag; without it a broken read path would block the
/// mint's partial exits and its reconciliation for good. A chain-semantics constant, like
/// the pending-open wait.
pub const UNPROVEN_HOLD_LIMIT: Duration = Duration::from_secs(10 * 60);

static PARTIAL_EXIT_HOLDS: LazyLock<RwLock<Vec<PartialExitHold>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// What the chain proves about a held partial exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldOutcome {
    /// No settled reading yet, and the limit has not been reached.
    Wait,
    /// The settled holding equals the holding before the swap: nothing was sold.
    NotSold,
    /// The settled holding is lower by exactly the exit amount: the share was sold.
    Sold,
    /// Any other settled holding, or no settled reading within the limit.
    Unknown,
}

/// A hold the settlement pass released, with the outcome that released it.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleasedHold {
    pub hold: PartialExitHold,
    pub outcome: HoldOutcome,
}

/// The outcome of `hold` given the `settled` holding read once no transaction sent before
/// its bound can still land, at `now`. Only an exact raw equality proves nothing was sold,
/// and only a drop of exactly the exit amount proves the share was sold.
pub fn hold_outcome(
    hold: &PartialExitHold,
    settled: Option<RawAmount>,
    now: DateTime<Utc>,
) -> HoldOutcome {
    match settled {
        None => {
            let limit_reached = now
                .signed_duration_since(hold.created_at)
                .to_std()
                .is_ok_and(|held_for| held_for >= UNPROVEN_HOLD_LIMIT);
            if limit_reached {
                HoldOutcome::Unknown
            } else {
                HoldOutcome::Wait
            }
        }
        Some(after) if after == hold.holding_before => HoldOutcome::NotSold,
        Some(after) if hold.holding_before.checked_sub(after) == Some(hold.exit_amount) => {
            HoldOutcome::Sold
        }
        Some(_) => HoldOutcome::Unknown,
    }
}

/// Registers `hold` and persists the set of holds. The swap may have sold, so memory keeps
/// the hold even when persisting fails; the failure is logged and the next persist of the
/// set stores it.
pub async fn hold_partial_exit(hold: PartialExitHold) {
    let (position_id, mint) = (hold.position_id, hold.mint.clone());
    PARTIAL_EXIT_HOLDS.write().await.push(hold);
    if let Err(error) = persist_partial_exit_holds().await {
        logger::error(
            LogTag::Positions,
            &format!(
                "Partial exit hold of position {position_id} (mint {mint}) is held in memory only, not persisted: {error}"
            ),
        );
    }
}

/// Every partial-exit hold of `mint`.
pub async fn partial_exit_holds_for_mint(mint: &str) -> Vec<PartialExitHold> {
    PARTIAL_EXIT_HOLDS
        .read()
        .await
        .iter()
        .filter(|hold| hold.mint == mint)
        .cloned()
        .collect()
}

/// True while a partial exit of `mint` is held.
pub async fn holds_partial_exit(mint: &str) -> bool {
    PARTIAL_EXIT_HOLDS
        .read()
        .await
        .iter()
        .any(|hold| hold.mint == mint)
}

/// True while a partial exit of position `position_id` of `mint` is held.
pub async fn holds_partial_exit_of(mint: &str, position_id: i64) -> bool {
    PARTIAL_EXIT_HOLDS
        .read()
        .await
        .iter()
        .any(|hold| hold.mint == mint && hold.position_id == position_id)
}

/// Every mint with a held partial exit.
pub async fn held_mints() -> Vec<String> {
    let mut mints: Vec<String> = PARTIAL_EXIT_HOLDS
        .read()
        .await
        .iter()
        .map(|hold| hold.mint.clone())
        .collect();
    mints.sort_unstable();
    mints.dedup();
    mints
}

/// Drops the holds of position `position_id`, whose row is deleted, from memory and then
/// from storage. Memory is cleared even when persisting fails; the error reports that
/// storage still lists them until the next persist, and the startup sweep drops them then.
pub async fn drop_holds_of_position(position_id: i64) -> Result<()> {
    let dropped = {
        let mut holds = PARTIAL_EXIT_HOLDS.write().await;
        let before = holds.len();
        holds.retain(|hold| hold.position_id != position_id);
        before != holds.len()
    };
    if dropped {
        persist_partial_exit_holds().await
    } else {
        Ok(())
    }
}

/// Replaces the holds in memory with the stored ones (used at startup) and returns how many
/// are held. A hold whose position row no longer exists is dropped from memory and storage.
pub async fn rehydrate_partial_exit_holds() -> Result<usize> {
    let raw = db::get_metadata(db::PARTIAL_EXIT_HOLD_METADATA_KEY).await?;
    let mut holds: Vec<PartialExitHold> = match raw {
        Some(payload) if !payload.is_empty() => {
            serde_json::from_str(&payload).map_err(|e| Error::RowDecode {
                column: "partial_exit_hold_metadata",
                detail: e.to_string(),
            })?
        }
        _ => Vec::new(),
    };
    let dropped = super::state_pending::drop_swaps_of_deleted_positions(
        &mut holds,
        |hold| hold.position_id,
        "partial exit hold",
    )
    .await;
    let held = holds.len();
    *PARTIAL_EXIT_HOLDS.write().await = holds;
    if dropped > 0 {
        if let Err(error) = persist_partial_exit_holds().await {
            logger::warning(
                LogTag::Positions,
                &format!("Partial exit holds of deleted positions are still stored: {error}"),
            );
        }
    }
    Ok(held)
}

/// Settles the holds through the settlement reader of the positions store's chain and the
/// trading wallet. Reads nothing when no partial exit is held.
pub async fn settle_partial_exit_holds() {
    if PARTIAL_EXIT_HOLDS.read().await.is_empty() {
        return;
    }
    let reader = match super::settle::reader().await {
        Ok(reader) => reader,
        Err(error) => {
            logger::warning(
                LogTag::Positions,
                &format!("Partial exit holds cannot be settled: no settlement reader: {error}"),
            );
            return;
        }
    };
    let wallet = match crate::utils::get_wallet_address() {
        Ok(wallet) => wallet,
        Err(error) => {
            logger::warning(
                LogTag::Positions,
                &format!("Partial exit holds cannot be settled: no trading wallet: {error}"),
            );
            return;
        }
    };
    settle_partial_exit_holds_on(reader.as_ref(), &wallet, Utc::now()).await;
}

/// Settles each hold from `reader`'s reading of `wallet` at `now` and returns the holds it
/// released. A hold without a bound is given one first; a later bound is a safe upper bound.
/// A hold with no outcome yet, or whose mint's position lock is not free within the
/// settlement wait, is left for the next pass.
///
/// Under the lock, a hold proven not sold first returns its take-profit step to the ladder
/// of its open row, while the hold still blocks partial exits, and is then removed. A hold
/// found sold or released without proof is removed and the wallet-history resync books
/// whatever the wallet shows. The outcome is recorded only when the removal returned the
/// hold, so each hold leaves exactly once.
pub async fn settle_partial_exit_holds_on(
    reader: &dyn SettlementReader,
    wallet: &str,
    now: DateTime<Utc>,
) -> Vec<ReleasedHold> {
    let holds = PARTIAL_EXIT_HOLDS.read().await.clone();
    let mut released = Vec::new();
    for mut hold in holds {
        if hold.expiry_height.is_none() {
            match reader.expiry_bound().await {
                Ok(bound) => {
                    assign_expiry_bound(&hold, bound).await;
                    hold.expiry_height = Some(bound);
                }
                Err(error) => logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Partial exit hold of position {} has no expiry bound yet: {error}",
                        hold.position_id
                    ),
                ),
            }
        }
        let settled = match hold.expiry_height {
            Some(bound) => reader
                .settled_holding(wallet, &hold.mint, bound)
                .await
                .ok()
                .flatten()
                .map(|holding| holding.amount),
            None => None,
        };
        let outcome = hold_outcome(&hold, settled, now);
        if outcome == HoldOutcome::Wait {
            continue;
        }
        let Some(_lock) = super::worker::settlement_lock(&hold.mint).await else {
            continue;
        };
        if outcome == HoldOutcome::NotSold {
            if let Some(level) = hold.take_profit_level {
                if position_is_open(hold.position_id).await {
                    super::operations::release_take_profit_step(
                        &hold.mint,
                        hold.position_id,
                        level,
                    )
                    .await;
                }
            }
        }
        let Some(removed) = take_hold(&hold).await else {
            continue;
        };
        if let Err(error) = persist_partial_exit_holds().await {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Released partial exit hold of position {} (mint {}) is still stored: {error}",
                    removed.position_id, removed.mint
                ),
            );
        }
        if outcome != HoldOutcome::NotSold {
            super::ledger::schedule_resync();
        }
        record_outcome(&removed, outcome, settled).await;
        released.push(ReleasedHold {
            hold: removed,
            outcome,
        });
    }
    released
}

/// Records the event and the log line of a released hold.
async fn record_outcome(hold: &PartialExitHold, outcome: HoldOutcome, settled: Option<RawAmount>) {
    let (event, reason) = match outcome {
        HoldOutcome::NotSold => (
            "partial_exit_hold_not_sold",
            "the settled holding equals the holding before the swap",
        ),
        HoldOutcome::Sold => (
            "partial_exit_hold_sold",
            "the settled holding is lower by exactly the exit amount",
        ),
        HoldOutcome::Unknown if settled.is_some() => (
            "partial_exit_hold_released",
            "the settled holding moved by another amount",
        ),
        HoldOutcome::Unknown | HoldOutcome::Wait => (
            "partial_exit_hold_released",
            "no settled holding could be read within the hold limit",
        ),
    };
    let entry_signature = super::state::get_position_by_id(hold.position_id)
        .await
        .and_then(|position| position.entry_transaction_signature);
    crate::events::record_position_event(
        &hold.position_id.to_string(),
        &hold.mint,
        event,
        entry_signature.as_deref(),
        None,
        0.0,
        hold.exit_amount,
        None,
        Some(hold.requested_exit_percentage),
    )
    .await;
    let message = format!(
        "Partial exit hold of position {} (mint {}) released as {outcome:?}: {reason} (before={} after={} exit={})",
        hold.position_id,
        hold.mint,
        hold.holding_before,
        settled.map_or_else(|| "unread".to_owned(), |after| after.to_string()),
        hold.exit_amount
    );
    if outcome == HoldOutcome::Unknown {
        logger::warning(LogTag::Positions, &message);
    } else {
        logger::info(LogTag::Positions, &message);
    }
}

/// True when the position is in memory and not closed.
async fn position_is_open(position_id: i64) -> bool {
    super::state::get_position_by_id(position_id)
        .await
        .is_some_and(|position| !super::round_state::is_closed(&position))
}

/// Stores `bound` on the hold of the same identity as `hold` and persists the set.
async fn assign_expiry_bound(hold: &PartialExitHold, bound: u64) {
    {
        let mut holds = PARTIAL_EXIT_HOLDS.write().await;
        if let Some(stored) = holds.iter_mut().find(|stored| same_hold(stored, hold)) {
            stored.expiry_height = Some(bound);
        }
    }
    if let Err(error) = persist_partial_exit_holds().await {
        logger::warning(
            LogTag::Positions,
            &format!(
                "Expiry bound of the partial exit hold of position {} is held in memory only: {error}",
                hold.position_id
            ),
        );
    }
}

/// Removes the hold of the same identity as `hold`, returning it when it was still held.
async fn take_hold(hold: &PartialExitHold) -> Option<PartialExitHold> {
    let mut holds = PARTIAL_EXIT_HOLDS.write().await;
    remove_hold(&mut holds, hold)
}

/// Removes from `holds` the hold of the same identity as `hold`.
fn remove_hold(
    holds: &mut Vec<PartialExitHold>,
    hold: &PartialExitHold,
) -> Option<PartialExitHold> {
    let index = holds.iter().position(|stored| same_hold(stored, hold))?;
    Some(holds.remove(index))
}

/// A hold's identity: its position and the time it was registered.
fn same_hold(a: &PartialExitHold, b: &PartialExitHold) -> bool {
    a.position_id == b.position_id && a.created_at == b.created_at
}

async fn persist_partial_exit_holds() -> Result<()> {
    let holds = PARTIAL_EXIT_HOLDS.read().await.clone();
    let serialized = serde_json::to_string(&holds).map_err(|e| Error::Maintenance {
        operation: "persist_partial_exit_holds",
        detail: e.to_string(),
    })?;
    db::set_metadata(db::PARTIAL_EXIT_HOLD_METADATA_KEY, &serialized).await
}

#[cfg(test)]
mod tests;
