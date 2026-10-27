// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pending swap queries — whether a position or a mint has a bot swap in flight, read across the pending partial-exit, DCA and in-flight registries.

use super::state_pending::{
    get_pending_dca_swaps_for_mint, get_pending_partial_exits_for_mint, PENDING_DCA_SWAPS,
    PENDING_PARTIAL_EXITS, PENDING_PARTIAL_EXIT_DETAILS, SWAPS_IN_FLIGHT,
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// True while a DCA add or a partial exit of position `position_id` of `mint` is in flight
/// (submitted, not yet verified) or a partial exit of it is held unproven. None of them is
/// on the position row until its verification or the wallet books it, so these registries
/// are the only proof a fill may still land on the row.
pub async fn position_has_pending_swap(mint: &str, position_id: i64) -> bool {
    get_pending_dca_swaps_for_mint(mint)
        .await
        .iter()
        .any(|entry| entry.position_id == position_id)
        || get_pending_partial_exits_for_mint(mint)
            .await
            .iter()
            .any(|entry| entry.position_id == position_id)
        || super::swap_hold::holds_partial_exit_of(mint, position_id).await
}

/// Whether a bot swap of `mint` that is not one of `position_id`'s may still move the
/// wallet's holding of the mint: a pending DCA or partial exit of another position, a
/// swap submitted whose pending state is not recorded yet (a partial exit counted on the
/// mint before its details are registered, or a marked swap), or a held partial exit of the
/// mint, whose unproven sale may still move the holding until its bound passes. Its tokens
/// may or may not be in a holding read now, so such a reading cannot be attributed to one
/// position.
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
        || super::swap_hold::holds_partial_exit(mint).await
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

/// Every mint with a swap in flight — a pending partial exit, a held partial exit, a
/// pending DCA add, a bot swap submitted whose pending state is not recorded yet, or a
/// position whose memory holds a submitted full exit that is not verified. The last one covers a submission whose row
/// write failed: the database row shows no exit in flight, and the memory copy is the only
/// record of it.
///
/// The wallet-history ledger reads this before reconciling a bot-executed position
/// against the chain: a mint whose balance is about to move again must be left to the
/// trader, or the ledger would close or resize a position out from under a swap that has
/// already been submitted.
///
/// A swap whose verification was abandoned does not count: the session never books it,
/// so the ledger is what reconciles its mint instead of skipping it until a restart.
pub async fn mints_with_pending_swaps() -> std::collections::HashSet<String> {
    let abandoned = super::queue::abandoned_signatures().await;

    let mut abandoned_partials: HashMap<String, u32> = HashMap::new();
    for entry in PENDING_PARTIAL_EXIT_DETAILS
        .read()
        .await
        .iter()
        .filter(|(signature, _)| abandoned.contains(*signature))
        .map(|(_, entry)| entry)
    {
        *abandoned_partials.entry(entry.mint.clone()).or_default() += 1;
    }
    let mut mints: std::collections::HashSet<String> = PENDING_PARTIAL_EXITS
        .read()
        .await
        .iter()
        .filter(|(mint, count)| **count > abandoned_partials.get(*mint).copied().unwrap_or(0))
        .map(|(mint, _)| mint.clone())
        .collect();

    mints.extend(
        PENDING_DCA_SWAPS
            .read()
            .await
            .values()
            .filter(|entry| !abandoned.contains(&entry.signature))
            .map(|entry| entry.mint.clone()),
    );
    mints.extend(super::swap_hold::held_mints().await);
    mints.extend(
        SWAPS_IN_FLIGHT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .keys()
            .map(|(_, mint)| mint.clone()),
    );
    mints.extend(
        super::state::POSITIONS
            .read()
            .await
            .iter()
            .filter(|position| {
                !position.transaction_exit_verified
                    && position
                        .exit_transaction_signature
                        .as_ref()
                        .is_some_and(|signature| !abandoned.contains(signature))
            })
            .map(|position| position.mint.clone()),
    );

    mints
}
