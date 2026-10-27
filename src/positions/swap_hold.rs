// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Holds on position swaps that ended with no proof either way: a swap whose last send failed with no signature holds its mint until the chain proves whether it moved the wallet's holding.

use std::sync::LazyLock;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::chains::RawAmount;
use crate::logger::{self, LogTag};

use super::db;
use super::error::{Error, Result};

mod release;

pub use release::{
    hold_outcome, settle_swap_holds, settle_swap_holds_on, HoldOutcome, ReleasedHold,
};

/// How long a hold waits for a settled reading before it is released as unknown. A Solana
/// transaction stays valid for 150 blocks, one to two minutes, so the limit spans several
/// validity windows plus finalization lag; without it a broken read path would block the
/// mint's swaps and its reconciliation for good. A chain-semantics constant, like the
/// pending-open wait.
pub const UNPROVEN_HOLD_LIMIT: Duration = Duration::from_secs(10 * 60);

static SWAP_HOLDS: LazyLock<RwLock<Vec<SwapHold>>> = LazyLock::new(|| RwLock::new(Vec::new()));

/// A position swap whose last send failed with no signature, so nothing proves whether it
/// moved the wallet's holding. It holds the mint until the chain proves the outcome.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwapHold {
    pub mint: String,
    pub position_id: i64,
    /// The wallet's holding of the mint read just before the swap.
    pub holding_before: RawAmount,
    /// The bound after which no transaction sent before the hold can still land; assigned
    /// later when it could not be read at registration.
    pub expiry_height: Option<u64>,
    pub created_at: DateTime<Utc>,
    /// What the held swap was sent to do.
    pub swap: HeldSwap,
}

/// What a held swap was sent to do.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HeldSwap {
    /// A partial exit selling `exit_amount`, `requested_exit_percentage` of the position,
    /// that took take-profit step `take_profit_level` before it was sent.
    PartialExit {
        exit_amount: RawAmount,
        requested_exit_percentage: f64,
        take_profit_level: Option<u32>,
    },
}

/// Registers `hold` and persists the set of holds. The swap may have moved the holding, so
/// memory keeps the hold even when persisting fails; the failure is logged and the next
/// persist of the set stores it.
pub async fn hold_swap(hold: SwapHold) {
    let (position_id, mint) = (hold.position_id, hold.mint.clone());
    SWAP_HOLDS.write().await.push(hold);
    if let Err(error) = persist_swap_holds().await {
        logger::error(
            LogTag::Positions,
            &format!(
                "Swap hold of position {position_id} (mint {mint}) is held in memory only, not persisted: {error}"
            ),
        );
    }
}

/// Every swap hold of `mint`, of every kind.
pub async fn holds_for_mint(mint: &str) -> Vec<SwapHold> {
    SWAP_HOLDS
        .read()
        .await
        .iter()
        .filter(|hold| hold.mint == mint)
        .cloned()
        .collect()
}

/// True while a partial exit of `mint` is held.
pub async fn holds_partial_exit(mint: &str) -> bool {
    SWAP_HOLDS.read().await.iter().any(|hold| {
        hold.mint == mint
            && match hold.swap {
                HeldSwap::PartialExit { .. } => true,
            }
    })
}

/// True while a swap of `mint`, of any kind, is held.
pub async fn holds_swap(mint: &str) -> bool {
    SWAP_HOLDS.read().await.iter().any(|hold| hold.mint == mint)
}

/// True while a swap of position `position_id` of `mint`, of any kind, is held.
pub async fn holds_swap_of(mint: &str, position_id: i64) -> bool {
    SWAP_HOLDS
        .read()
        .await
        .iter()
        .any(|hold| hold.mint == mint && hold.position_id == position_id)
}

/// Every mint with a held swap.
pub async fn held_mints() -> Vec<String> {
    let mut mints: Vec<String> = SWAP_HOLDS
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
        let mut holds = SWAP_HOLDS.write().await;
        let before = holds.len();
        holds.retain(|hold| hold.position_id != position_id);
        before != holds.len()
    };
    if dropped {
        persist_swap_holds().await
    } else {
        Ok(())
    }
}

/// Replaces the holds in memory with the stored ones (used at startup) and returns how many
/// are held. A hold whose position row no longer exists is dropped from memory and storage.
pub async fn rehydrate_swap_holds() -> Result<usize> {
    let raw = db::get_metadata(db::SWAP_HOLD_METADATA_KEY).await?;
    let mut holds: Vec<SwapHold> = match raw {
        Some(payload) if !payload.is_empty() => {
            serde_json::from_str(&payload).map_err(|e| Error::RowDecode {
                column: "swap_hold_metadata",
                detail: e.to_string(),
            })?
        }
        _ => Vec::new(),
    };
    let dropped = super::state_pending::drop_swaps_of_deleted_positions(
        &mut holds,
        |hold| hold.position_id,
        "swap hold",
    )
    .await;
    let held = holds.len();
    *SWAP_HOLDS.write().await = holds;
    if dropped > 0 {
        if let Err(error) = persist_swap_holds().await {
            logger::warning(
                LogTag::Positions,
                &format!("Swap holds of deleted positions are still stored: {error}"),
            );
        }
    }
    Ok(held)
}

/// Stores `bound` on the hold of the same identity as `hold` and persists the set.
async fn assign_expiry_bound(hold: &SwapHold, bound: u64) {
    {
        let mut holds = SWAP_HOLDS.write().await;
        if let Some(stored) = holds.iter_mut().find(|stored| same_hold(stored, hold)) {
            stored.expiry_height = Some(bound);
        }
    }
    if let Err(error) = persist_swap_holds().await {
        logger::warning(
            LogTag::Positions,
            &format!(
                "Expiry bound of the swap hold of position {} is held in memory only: {error}",
                hold.position_id
            ),
        );
    }
}

/// Removes the hold of the same identity as `hold`, returning it when it was still held.
async fn take_hold(hold: &SwapHold) -> Option<SwapHold> {
    let mut holds = SWAP_HOLDS.write().await;
    remove_hold(&mut holds, hold)
}

/// Removes from `holds` the hold of the same identity as `hold`.
fn remove_hold(holds: &mut Vec<SwapHold>, hold: &SwapHold) -> Option<SwapHold> {
    let index = holds.iter().position(|stored| same_hold(stored, hold))?;
    Some(holds.remove(index))
}

/// A hold's identity: its position, the time it was registered and the kind of swap held.
fn same_hold(a: &SwapHold, b: &SwapHold) -> bool {
    a.position_id == b.position_id
        && a.created_at == b.created_at
        && std::mem::discriminant(&a.swap) == std::mem::discriminant(&b.swap)
}

async fn persist_swap_holds() -> Result<()> {
    let holds = SWAP_HOLDS.read().await.clone();
    let serialized = serde_json::to_string(&holds).map_err(|e| Error::Maintenance {
        operation: "persist_swap_holds",
        detail: e.to_string(),
    })?;
    db::set_metadata(db::SWAP_HOLD_METADATA_KEY, &serialized).await
}

#[cfg(test)]
mod tests;
