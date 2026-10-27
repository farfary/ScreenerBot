// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The swaps a position has submitted but not booked yet, and its partial exits held until the chain proves their outcome, for the position details view.

use super::types::PendingSwapResponse;
use crate::positions::{self, Book};

/// Swaps this position has submitted but not booked yet, and its held partial exits.
///
/// Filtered by position id, not just mint: the registries are per-mint, and handing one
/// position the other's in-flight add would misreport both. A position with no id cannot
/// own a pending swap. The registries hold live swaps only, and paper ids are counted apart
/// from live ones, so a paper position is never matched against them.
pub(super) async fn load_pending_swaps(position: &positions::Position) -> Vec<PendingSwapResponse> {
    let Some(id) = position.id else {
        return Vec::new();
    };
    if position.book != Book::Live {
        return Vec::new();
    }

    let mut pending: Vec<PendingSwapResponse> =
        positions::get_pending_dca_swaps_for_mint(&position.mint)
            .await
            .into_iter()
            .filter(|entry| entry.position_id == id)
            .map(|entry| PendingSwapResponse {
                kind: "dca".to_owned(),
                signature: Some(entry.signature),
                submitted_at: entry.created_at.timestamp(),
                size_sol: (entry.size_sol > 0.0).then_some(entry.size_sol),
                exit_percentage: None,
            })
            .collect();

    pending.extend(
        positions::get_pending_partial_exits_for_mint(&position.mint)
            .await
            .into_iter()
            .filter(|entry| entry.position_id == id)
            .map(|entry| PendingSwapResponse {
                kind: "partial_exit".to_owned(),
                signature: Some(entry.signature),
                submitted_at: entry.created_at.timestamp(),
                size_sol: None,
                exit_percentage: Some(entry.requested_exit_percentage),
            }),
    );

    pending.extend(
        positions::swap_hold::partial_exit_holds_for_mint(&position.mint)
            .await
            .into_iter()
            .filter(|hold| hold.position_id == id)
            .map(|hold| PendingSwapResponse {
                kind: "partial_exit_hold".to_owned(),
                signature: None,
                submitted_at: hold.created_at.timestamp(),
                size_sol: None,
                exit_percentage: Some(hold.requested_exit_percentage),
            }),
    );

    pending.sort_by_key(|entry| entry.submitted_at);
    pending
}
