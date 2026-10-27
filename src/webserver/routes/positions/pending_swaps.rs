// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The swaps a position has submitted but not booked yet, and its swaps held until the chain proves their outcome, for the position details view.

use super::types::PendingSwapResponse;
use crate::positions::swap_hold::{self, HeldSwap, SwapHold};
use crate::positions::{self, Book};

/// Swaps this position has submitted but not booked yet, and its held swaps.
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
        swap_hold::holds_for_mint(&position.mint)
            .await
            .into_iter()
            .filter(|hold| hold.position_id == id)
            .map(held_row),
    );

    pending.sort_by_key(|entry| entry.submitted_at);
    pending
}

/// The row of a held swap: it has no signature, and its kind says what it was sent to do.
fn held_row(hold: SwapHold) -> PendingSwapResponse {
    let submitted_at = hold.created_at.timestamp();
    match hold.swap {
        HeldSwap::PartialExit {
            requested_exit_percentage,
            ..
        } => PendingSwapResponse {
            kind: "partial_exit_hold".to_owned(),
            signature: None,
            submitted_at,
            size_sol: None,
            exit_percentage: Some(requested_exit_percentage),
        },
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::held_row;
    use crate::chains::RawAmount;
    use crate::positions::swap_hold::{HeldSwap, SwapHold};

    #[test]
    fn a_held_partial_exit_is_a_row_without_a_signature() {
        let created_at = Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap();
        let row = held_row(SwapHold {
            mint: "mint".to_owned(),
            position_id: 7,
            holding_before: RawAmount::new(1_000),
            expiry_height: Some(500),
            created_at,
            swap: HeldSwap::PartialExit {
                exit_amount: RawAmount::new(250),
                requested_exit_percentage: 25.0,
                take_profit_level: Some(2),
            },
        });
        assert_eq!(row.kind, "partial_exit_hold");
        assert_eq!(row.signature, None);
        assert_eq!(row.submitted_at, created_at.timestamp());
        assert_eq!(row.size_sol, None);
        assert_eq!(row.exit_percentage, Some(25.0));
    }
}
