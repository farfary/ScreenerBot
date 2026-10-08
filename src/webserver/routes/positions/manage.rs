// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position management routes — archive, unarchive, delete, and bulk-clear.
//!
//! Archive is a reversible flag (position hidden from open/closed lists, surfaced
//! in the Archived tab). Delete is a permanent hard-delete that cascades ONLY to
//! the position's own child rows (states, exits, entries, tracking, snapshots).
//! Transactions, tokens, wallets, and events are never touched here.
//!
//! Removing a position that is still OPEN frees its trading slot (semaphore
//! permit) so the bot can open a new position; it does NOT sell — tokens stay in
//! the wallet.

use axum::{
    extract::Path,
    response::{IntoResponse as _, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::positions;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;

#[derive(Debug, Serialize)]
pub struct ArchiveResponse {
    pub success: bool,
    pub position_id: i64,
    pub archived: bool,
    pub freed_slot: bool,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub position_id: i64,
    pub freed_slot: bool,
}

#[derive(Debug, Deserialize)]
pub struct ManagementRequest {
    pub management: positions::PositionManagement,
}

#[derive(Debug, Serialize)]
pub struct ManagementResponse {
    pub success: bool,
    pub position_id: i64,
    pub management: positions::PositionManagement,
}

#[derive(Debug, Serialize)]
pub struct BulkDeleteResponse {
    pub success: bool,
    pub deleted: usize,
    pub freed_slots: usize,
}

/// POST /positions/:id/archive — hide a position into the Archived tab.
pub(super) async fn archive_position(Path(position_id): Path<i64>) -> Response {
    let position = match positions::get_position_by_id(position_id).await {
        Some(p) => p,
        None => {
            return ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_POSITIONS_NOT_FOUND)
                .details(position_id.to_string())
                .into_response();
        }
    };

    if position.archived {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_POSITIONS_ALREADY_ARCHIVED,
        )
        .into_response();
    }

    // Persist first, then mirror into memory so a failed write doesn't desync state.
    if let Err(e) = positions::set_position_archived_db(position_id, true).await {
        if matches!(e, positions::Error::UnverifiedEntryArchive { .. }) {
            return ApiError::new(
                ApiErrorCode::Conflict,
                ids::ERRORS_POSITIONS_UNVERIFIED_ENTRY_ARCHIVE,
            )
            .into_response();
        }
        return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_POSITIONS_ARCHIVE_FAILED)
            .details(e.to_string())
            .into_response();
    }
    positions::set_position_archived_in_memory(position_id, true).await;

    // Archiving removes the position from active management: the slot it holds, if any,
    // is freed.
    let freed_slot = positions::state::release_position_slot(position_id).await;

    logger::info(
        LogTag::Positions,
        &format!(
            "Archived position {position_id} ({}) — freed_slot={freed_slot}",
            position.symbol
        ),
    );

    success_response(ArchiveResponse {
        success: true,
        position_id,
        archived: true,
        freed_slot,
    })
}

/// POST /positions/:id/unarchive — restore a position from the Archived tab.
pub(super) async fn unarchive_position(Path(position_id): Path<i64>) -> Response {
    let position = match positions::get_position_by_id(position_id).await {
        Some(p) => p,
        None => {
            return ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_POSITIONS_NOT_FOUND)
                .details(position_id.to_string())
                .into_response();
        }
    };

    if !position.archived {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_POSITIONS_NOT_ARCHIVED,
        )
        .into_response();
    }

    if let Err(e) = positions::set_position_archived_db(position_id, false).await {
        if let positions::Error::DuplicateOpenRound { position_ids, .. } = &e {
            let positions = position_ids
                .iter()
                .map(|id| format!("#{id}"))
                .collect::<Vec<_>>()
                .join(", ");
            return ApiError::new(
                ApiErrorCode::Conflict,
                ids::ERRORS_POSITIONS_UNARCHIVE_DUPLICATE_OPEN,
            )
            .text_arg("positions", positions)
            .into_response();
        }
        return ApiError::new(
            ApiErrorCode::Internal,
            ids::ERRORS_POSITIONS_UNARCHIVE_FAILED,
        )
        .details(e.to_string())
        .into_response();
    }
    positions::set_position_archived_in_memory(position_id, false).await;

    // If this position is still open it re-enters active management — reclaim a slot.
    let reclaimed = positions::state::is_open_round(&position)
        && positions::state::reclaim_position_slot(&position).await;

    logger::info(
        LogTag::Positions,
        &format!(
            "Unarchived position {position_id} ({}) — reclaimed_slot={reclaimed}",
            position.symbol
        ),
    );

    success_response(ArchiveResponse {
        success: true,
        position_id,
        archived: false,
        freed_slot: false,
    })
}

/// POST /positions/:id/management — change action ownership without changing provenance.
pub(super) async fn set_management(
    Path(position_id): Path<i64>,
    Json(req): Json<ManagementRequest>,
) -> Response {
    let position = match positions::get_position_by_id(position_id).await {
        Some(p) => p,
        None => {
            return ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_POSITIONS_NOT_FOUND)
                .details(position_id.to_string())
                .into_response();
        }
    };

    if !req.management.is_valid_for_origin(&position.origin) {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_POSITIONS_MANAGEMENT_INVALID,
        )
        .into_response();
    }

    // Persist first, then mirror into memory so a failed write doesn't desync state.
    if let Err(e) = positions::set_position_management_db(position_id, req.management).await {
        return ApiError::new(
            ApiErrorCode::Internal,
            ids::ERRORS_POSITIONS_MANAGEMENT_FAILED,
        )
        .details(e.to_string())
        .into_response();
    }
    positions::set_position_management_in_memory(position_id, req.management).await;

    logger::info(
        LogTag::Positions,
        &format!(
            "Position {position_id} ({}) management set to {}",
            position.symbol,
            req.management.as_str()
        ),
    );

    success_response(ManagementResponse {
        success: true,
        position_id,
        management: req.management,
    })
}

/// Drops the pending DCA and partial-exit markers of a deleted position, so they never hold
/// a later fill of its mint as unattributable. The row is already gone, so a failure to
/// persist the cleared set is logged rather than answered.
async fn clear_pending_swaps_of_deleted(position_id: i64) {
    if let Err(error) = positions::state::clear_pending_swaps_of_position(position_id).await {
        logger::warning(
            LogTag::Positions,
            &format!(
                "Pending swaps of deleted position {position_id} were cleared in memory but not in storage: {error}"
            ),
        );
    }
}

/// The position from memory, or from storage when it is not held in memory.
async fn find_position(position_id: i64) -> Option<positions::Position> {
    match positions::get_position_by_id(position_id).await {
        Some(position) => Some(position),
        None => positions::get_db_position_by_id(position_id)
            .await
            .ok()
            .flatten(),
    }
}

/// DELETE /positions/:id — permanently delete a position and its history.
///
/// The mint's position lock is held from the read to the marker clear: a DCA or partial
/// exit registers its pending marker under that lock after its swap is sent, so a delete
/// either clears that marker or runs before the swap starts.
pub(super) async fn delete_position(Path(position_id): Path<i64>) -> Response {
    let not_found = || {
        ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_POSITIONS_NOT_FOUND)
            .details(position_id.to_string())
            .into_response()
    };
    let Some(position) = find_position(position_id).await else {
        return not_found();
    };
    let _lock = positions::acquire_position_lock(&position.mint).await;
    let Some(position) = find_position(position_id).await else {
        return not_found();
    };

    match positions::delete_position_by_id(position_id).await {
        Ok(true) => {}
        Ok(false) => {
            return ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_POSITIONS_NOT_FOUND)
                .into_response();
        }
        Err(e) => {
            return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_POSITIONS_DELETE_FAILED)
                .details(e.to_string())
                .into_response();
        }
    }

    positions::remove_position_by_id(position_id).await;
    clear_pending_swaps_of_deleted(position_id).await;

    // The slot the position holds, if any, is freed; an archived or closed one holds none.
    let freed_slot = positions::state::release_position_slot(position_id).await;
    // A deleted closed position's realized loss no longer counts toward the loss limit.
    crate::trader::safety::loss_limit::sync_from_books().await;

    logger::info(
        LogTag::Positions,
        &format!(
            "Permanently deleted position {position_id} ({}) — freed_slot={freed_slot}",
            position.symbol
        ),
    );

    success_response(DeleteResponse {
        success: true,
        position_id,
        freed_slot,
    })
}

/// DELETE /positions/archived — permanently delete ALL archived positions. The ids the
/// delete returns drive the memory removal and the marker clear, so a row archived after a
/// memory read, or one not held in memory, is cleaned up like the rest.
pub(super) async fn delete_all_archived() -> Response {
    // Archived positions already released their slot on archive, so no slot frees here.
    let deleted = match positions::delete_archived_positions().await {
        Ok(ids) => ids,
        Err(e) => {
            return ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_POSITIONS_BULK_DELETE_FAILED,
            )
            .details(e.to_string())
            .into_response();
        }
    };

    for &id in &deleted {
        positions::remove_position_by_id(id).await;
        clear_pending_swaps_of_deleted(id).await;
    }
    if !deleted.is_empty() {
        crate::trader::safety::loss_limit::sync_from_books().await;
    }

    logger::info(
        LogTag::Positions,
        &format!("Permanently deleted {} archived position(s)", deleted.len()),
    );

    success_response(BulkDeleteResponse {
        success: true,
        deleted: deleted.len(),
        freed_slots: 0,
    })
}
