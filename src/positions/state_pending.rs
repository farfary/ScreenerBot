// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pending swap state — tracks in-flight partial exits and DCA swaps with persistence.

use super::db;
use super::error::{Error, Result};
pub use super::types::{PendingDcaSwap, PendingPartialExit};
use crate::chains::ChainId;
use crate::logger::{self, LogTag};
use chrono::{DateTime, Utc};
use std::{collections::HashMap, sync::LazyLock};
use tokio::sync::RwLock;

// Pending partial exits registry (mint -> count of pending partial exits)
// We serialize to a single pending at a time, but using a count keeps API flexible
static PENDING_PARTIAL_EXITS: LazyLock<RwLock<HashMap<String, u32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static PENDING_PARTIAL_EXIT_DETAILS: LazyLock<RwLock<HashMap<String, PendingPartialExit>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

// Pending DCA swaps registry: ensures DCA verifications survive restarts and duplicate submissions
static PENDING_DCA_SWAPS: LazyLock<RwLock<HashMap<String, PendingDcaSwap>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

const PENDING_DCA_METADATA_KEY: &str = "pending_dca_swaps";

// Bot swaps submitted on a mint whose pending state is not recorded yet ((chain, mint) ->
// count). A
// swap that confirms moves the wallet before its pending marker or exit signature is
// written, and the wallet-history sync must not take that movement for an outside trade.
static SWAPS_IN_FLIGHT: LazyLock<std::sync::Mutex<HashMap<(ChainId, String), u32>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// A bot swap on a mint, in flight from before its submission until the guard drops, which
/// is after its pending state is recorded.
#[derive(Debug)]
pub struct SwapInFlight {
    key: (ChainId, String),
}

impl Drop for SwapInFlight {
    fn drop(&mut self) {
        let mut map = SWAPS_IN_FLIGHT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(count) = map.get_mut(&self.key) {
            if *count > 1 {
                *count -= 1;
            } else {
                map.remove(&self.key);
            }
        }
    }
}

/// Marks a bot swap on `mint` of `chain` as in flight until the returned guard drops. Take
/// it before the swap is submitted and hold it until the swap's pending state is recorded.
pub fn mark_swap_in_flight(chain: ChainId, mint: &str) -> SwapInFlight {
    let key = (chain, mint.to_owned());
    let mut map = SWAPS_IN_FLIGHT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let count = map.entry(key.clone()).or_default();
    *count = count.saturating_add(1);
    SwapInFlight { key }
}

/// Mark that a partial exit is pending for a mint (increments count)
pub async fn mark_partial_exit_pending(mint: &str) {
    let mut map = PENDING_PARTIAL_EXITS.write().await;
    let counter = map.entry(mint.to_string()).or_default();
    *counter = counter.saturating_add(1);
}

/// Clear pending mark for a partial exit for a mint (decrements count and removes if zero)
pub async fn clear_partial_exit_pending(mint: &str) {
    let mut map = PENDING_PARTIAL_EXITS.write().await;
    if let Some(counter) = map.get_mut(mint) {
        if *counter > 1 {
            *counter -= 1;
        } else {
            map.remove(mint);
        }
    }
}

/// Is a partial exit currently in flight (submitted, not yet verified) for this mint?
///
/// This registry is what SERIALIZES exits: the per-mint position lock only covers the
/// submit, and is released while verification is still pending for several seconds, so
/// the lock alone cannot stop a second exit from being sized against a
/// `remaining_token_amount` that the first exit has not decremented yet.
///
/// The counter existed but NOTHING read it — it was write-only, and both
/// `close_position_direct` and `partial_close_position` believed something else was
/// serializing them. Nothing was.
pub async fn is_partial_exit_pending(mint: &str) -> bool {
    let map = PENDING_PARTIAL_EXITS.read().await;
    map.get(mint).is_some_and(|count| *count > 0)
}

/// Drop every pending partial exit recorded for a mint (counter + durable details).
///
/// Used by failure handling, which knows the position but not necessarily the
/// signature — a partial exit is no longer recorded on the position itself.
pub async fn clear_pending_partial_exits_for_mint(mint: &str) -> Result<()> {
    let removed: Vec<String> = {
        let mut details = PENDING_PARTIAL_EXIT_DETAILS.write().await;
        let signatures: Vec<String> = details
            .iter()
            .filter(|(_, entry)| entry.mint == mint)
            .map(|(signature, _)| signature.clone())
            .collect();
        for signature in &signatures {
            details.remove(signature);
        }
        signatures
    };

    {
        let mut counters = PENDING_PARTIAL_EXITS.write().await;
        counters.remove(mint);
    }

    if removed.is_empty() {
        return Ok(());
    }

    persist_pending_partial_exits().await
}

/// Persist current pending DCA map to the database metadata store
async fn persist_pending_dca_swaps() -> Result<()> {
    let pending: Vec<PendingDcaSwap> = {
        let map = PENDING_DCA_SWAPS.read().await;
        map.values().cloned().collect()
    };

    let serialized = serde_json::to_string(&pending).map_err(|e| Error::Maintenance {
        operation: "persist_pending_dca",
        detail: e.to_string(),
    })?;

    db::set_metadata(PENDING_DCA_METADATA_KEY, &serialized).await
}

/// Registers a sent DCA swap as pending and persists the pending set. The swap is already
/// sent, so memory keeps the entry even when persisting fails: the DCA stays pending for
/// this run, and the next persist of the set stores it. The error reports only that it is
/// not durable yet.
pub async fn register_pending_dca_swap(entry: PendingDcaSwap) -> Result<()> {
    PENDING_DCA_SWAPS
        .write()
        .await
        .insert(entry.signature.clone(), entry);
    persist_pending_dca_swaps().await
}

/// Clear a pending DCA swap once processed
pub async fn clear_pending_dca_swap(signature: &str) -> Result<Option<PendingDcaSwap>> {
    let removed = {
        let mut map = PENDING_DCA_SWAPS.write().await;
        map.remove(signature)
    };

    if let Some(entry) = removed.clone() {
        if let Err(err) = persist_pending_dca_swaps().await {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Failed to persist pending DCA metadata after clearing {}: {}",
                    signature, err
                ),
            );
            // Reinsert to keep in-memory state consistent if persistence fails
            {
                let mut map = PENDING_DCA_SWAPS.write().await;
                map.insert(entry.signature.clone(), entry);
            }
            return Err(err);
        }
    }

    Ok(removed)
}

/// Every DCA swap currently in flight for a mint (submitted, not yet verified).
///
/// Symmetric to [`get_pending_partial_exits_for_mint`]. Until `DcaVerified` writes its
/// entry record, a DCA's signature lives ONLY here — it is never stamped on the position
/// (`entry_transaction_signature` is the original entry and never changes). Without this
/// read an in-flight add was invisible in the position's activity list until verification
/// landed seconds later, even though the notification centre already announced it.
pub async fn get_pending_dca_swaps_for_mint(mint: &str) -> Vec<PendingDcaSwap> {
    let map = PENDING_DCA_SWAPS.read().await;
    map.values()
        .filter(|entry| entry.mint == mint)
        .cloned()
        .collect()
}

/// Load pending DCA swaps from metadata into memory (used at startup)
pub async fn rehydrate_pending_dca_swaps() -> Result<Vec<PendingDcaSwap>> {
    let raw = db::get_metadata(PENDING_DCA_METADATA_KEY).await?;

    let entries: Vec<PendingDcaSwap> = match raw {
        Some(payload) if !payload.is_empty() => {
            serde_json::from_str(&payload).map_err(|e| Error::RowDecode {
                column: "pending_dca_metadata",
                detail: e.to_string(),
            })?
        }
        _ => Vec::new(),
    };

    {
        let mut map = PENDING_DCA_SWAPS.write().await;
        map.clear();
        for entry in &entries {
            map.insert(entry.signature.clone(), entry.clone());
        }
    }

    Ok(entries)
}

async fn persist_pending_partial_exits() -> Result<()> {
    let pending: Vec<PendingPartialExit> = {
        let map = PENDING_PARTIAL_EXIT_DETAILS.read().await;
        map.values().cloned().collect()
    };

    let serialized = serde_json::to_string(&pending).map_err(|e| Error::Maintenance {
        operation: "persist_pending_partial_exits",
        detail: e.to_string(),
    })?;

    db::set_metadata(db::PENDING_PARTIAL_EXIT_METADATA_KEY, &serialized).await
}

/// Registers a sent partial exit as pending and persists the pending set. The swap is
/// already sent, so memory keeps the entry even when persisting fails: the partial exit
/// stays pending for this run, and the next persist of the set stores it. The error
/// reports only that it is not durable yet.
pub async fn register_pending_partial_exit(entry: PendingPartialExit) -> Result<()> {
    PENDING_PARTIAL_EXIT_DETAILS
        .write()
        .await
        .insert(entry.signature.clone(), entry);
    persist_pending_partial_exits().await
}

/// Clear a pending partial exit once processed
pub async fn clear_pending_partial_exit(signature: &str) -> Result<Option<PendingPartialExit>> {
    let removed = {
        let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
        map.remove(signature)
    };

    if let Some(entry) = removed.clone() {
        if let Err(err) = persist_pending_partial_exits().await {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Failed to persist pending partial exit metadata after clearing {}: {}",
                    signature, err
                ),
            );

            let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
            map.insert(entry.signature.clone(), entry);
            return Err(err);
        }
    }

    Ok(removed)
}

/// Fetch a pending partial exit by signature
pub async fn get_pending_partial_exit(signature: &str) -> Option<PendingPartialExit> {
    let map = PENDING_PARTIAL_EXIT_DETAILS.read().await;
    map.get(signature).cloned()
}

/// Every partial exit currently in flight for a mint (submitted, not yet verified).
///
/// Until verification writes its exit record, this registry is the ONLY place a partial
/// exit's signature lives — it is deliberately not stamped on the position (see
/// partial_close.rs). The position-details view reads this so an in-flight partial shows
/// up immediately instead of appearing out of nowhere seconds later.
pub async fn get_pending_partial_exits_for_mint(mint: &str) -> Vec<PendingPartialExit> {
    let map = PENDING_PARTIAL_EXIT_DETAILS.read().await;
    map.values()
        .filter(|entry| entry.mint == mint)
        .cloned()
        .collect()
}

/// True while a DCA add or a partial exit of position `position_id` of `mint` is in flight
/// (submitted, not yet verified). Neither is on the position row until its verification
/// books it, so these maps are the only proof a fill may still land on the row.
pub async fn position_has_pending_swap(mint: &str, position_id: i64) -> bool {
    get_pending_dca_swaps_for_mint(mint)
        .await
        .iter()
        .any(|entry| entry.position_id == position_id)
        || get_pending_partial_exits_for_mint(mint)
            .await
            .iter()
            .any(|entry| entry.position_id == position_id)
}

/// Whether a bot swap of `mint` that is not one of `position_id`'s may still move the
/// wallet's holding of the mint: a pending DCA or partial exit of another position, or a
/// swap submitted whose pending state is not recorded yet (a partial exit counted on the
/// mint before its details are registered, or a marked swap). Its tokens may or may not be
/// in a holding read now, so such a reading cannot be attributed to one position.
///
/// Late fills on closed positions are ordered by send: when `signature` is a pending DCA or
/// partial exit of this position, a pending swap of another closed position sent after it does not
/// count. That swap waits for this one instead, so two late fills of one mint never wait
/// for each other. A pending swap of an open position, or of one whose state is unknown,
/// always counts.
pub async fn other_swap_in_flight(mint: &str, position_id: i64, signature: Option<&str>) -> bool {
    let partials = get_pending_partial_exits_for_mint(mint).await;
    let partials_counted = PENDING_PARTIAL_EXITS
        .read()
        .await
        .get(mint)
        .copied()
        .unwrap_or(0);
    if partials_counted as usize > partials.len()
        || SWAPS_IN_FLIGHT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .keys()
            .any(|(_, in_flight)| in_flight == mint)
    {
        return true;
    }

    let dcas = get_pending_dca_swaps_for_mint(mint).await;
    let pending: Vec<(i64, &str, DateTime<Utc>)> = dcas
        .iter()
        .map(|entry| {
            (
                entry.position_id,
                entry.signature.as_str(),
                entry.created_at,
            )
        })
        .chain(partials.iter().map(|entry| {
            (
                entry.position_id,
                entry.signature.as_str(),
                entry.created_at,
            )
        }))
        .collect();
    let own_sent = pending
        .iter()
        .find(|(id, sig, _)| *id == position_id && Some(*sig) == signature)
        .map(|(_, sig, at)| (*at, *sig));
    for (other_id, other_signature, other_sent) in &pending {
        if *other_id == position_id {
            continue;
        }
        let sent_after_own = own_sent.is_some_and(|own| (*other_sent, *other_signature) > own);
        if !sent_after_own || !position_is_closed(*other_id).await {
            return true;
        }
    }
    false
}

/// True when the position is known and closed.
async fn position_is_closed(position_id: i64) -> bool {
    super::state::get_position_by_id(position_id)
        .await
        .is_some_and(|position| super::round_state::is_closed(&position))
}

/// Every mint with a swap in flight — a pending partial exit, a pending DCA add, or a bot
/// swap submitted whose pending state is not recorded yet.
///
/// The wallet-history ledger reads this before reconciling a bot-executed position
/// against the chain: a mint whose balance is about to move again must be left to the
/// trader, or the ledger would close or resize a position out from under a swap that has
/// already been submitted.
pub async fn mints_with_pending_swaps() -> std::collections::HashSet<String> {
    let mut mints: std::collections::HashSet<String> = PENDING_PARTIAL_EXITS
        .read()
        .await
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(mint, _)| mint.clone())
        .collect();

    mints.extend(
        PENDING_DCA_SWAPS
            .read()
            .await
            .values()
            .map(|entry| entry.mint.clone()),
    );
    mints.extend(
        SWAPS_IN_FLIGHT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .keys()
            .map(|(_, mint)| mint.clone()),
    );

    mints
}

/// Load pending partial exits from metadata into memory (used at startup)
pub async fn rehydrate_pending_partial_exits() -> Result<Vec<PendingPartialExit>> {
    let raw = db::get_metadata(db::PENDING_PARTIAL_EXIT_METADATA_KEY).await?;

    let entries: Vec<PendingPartialExit> = match raw {
        Some(payload) if !payload.is_empty() => {
            serde_json::from_str(&payload).map_err(|e| Error::RowDecode {
                column: "pending_partial_exit_metadata",
                detail: e.to_string(),
            })?
        }
        _ => Vec::new(),
    };

    {
        let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
        map.clear();
        for entry in &entries {
            map.insert(entry.signature.clone(), entry.clone());
        }
    }

    {
        let mut counters = PENDING_PARTIAL_EXITS.write().await;
        counters.clear();
        for entry in &entries {
            let counter = counters.entry(entry.mint.clone()).or_default();
            *counter = counter.saturating_add(1);
        }
    }

    Ok(entries)
}
