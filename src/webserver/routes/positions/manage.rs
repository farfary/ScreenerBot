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

/// A position counts toward the open-slot semaphore while it is a buy that has not
/// been exit-verified and has no exit time. Used to decide whether removing it
/// should free a trading slot.
fn holds_open_slot(p: &positions::Position) -> bool {
    p.position_type == "buy" && !p.transaction_exit_verified && p.exit_time.is_none()
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

    let was_open = holds_open_slot(&position);

    // Persist first, then mirror into memory so a failed write doesn't desync state.
    if let Err(e) = positions::set_position_archived_db(position_id, true).await {
        return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_POSITIONS_ARCHIVE_FAILED)
            .details(e.to_string())
            .into_response();
    }
    positions::set_position_archived_in_memory(position_id, true).await;

    // Archiving an open position removes it from active management — free its slot.
    if was_open {
        positions::state::release_position_slot(position_id).await;
    }

    logger::info(
        LogTag::Positions,
        &format!(
            "Archived position {position_id} ({}) — freed_slot={was_open}",
            position.symbol
        ),
    );

    success_response(ArchiveResponse {
        success: true,
        position_id,
        archived: true,
        freed_slot: was_open,
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
        return ApiError::new(
            ApiErrorCode::Internal,
            ids::ERRORS_POSITIONS_UNARCHIVE_FAILED,
        )
        .details(e.to_string())
        .into_response();
    }
    positions::set_position_archived_in_memory(position_id, false).await;

    // If this position is still open it re-enters active management — reclaim a slot.
    let reclaimed = if holds_open_slot(&position) {
        let ok = positions::state::try_consume_global_position_permit();
        if ok {
            // Register it as a slot holder, or the release when it eventually closes finds
            // no holder, does nothing, and the slot stays consumed forever.
            positions::state::register_position_slot(position_id).await;
        } else {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "Unarchived open position {position_id} but no free slot to reclaim (at capacity)"
                ),
            );
        }
        ok
    } else {
        false
    };

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

/// DELETE /positions/:id — permanently delete a position and its history.
pub(super) async fn delete_position(Path(position_id): Path<i64>) -> Response {
    let position = match positions::get_position_by_id(position_id).await {
        Some(p) => p,
        None => {
            // Fall back to DB in case it's not in memory.
            match positions::get_db_position_by_id(position_id).await {
                Ok(Some(p)) => p,
                _ => {
                    return ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_POSITIONS_NOT_FOUND)
                        .details(position_id.to_string())
                        .into_response();
                }
            }
        }
    };

    // Only release a slot for a position that is open AND not already archived
    // (archiving an open position already released its slot).
    let was_open = holds_open_slot(&position) && !position.archived;

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

    if was_open {
        positions::state::release_position_slot(position_id).await;
    }
    // A deleted closed position's realized loss no longer counts toward the loss limit.
    crate::trader::safety::loss_limit::sync_from_books().await;

    logger::info(
        LogTag::Positions,
        &format!(
            "Permanently deleted position {position_id} ({}) — freed_slot={was_open}",
            position.symbol
        ),
    );

    success_response(DeleteResponse {
        success: true,
        position_id,
        freed_slot: was_open,
    })
}

/// DELETE /positions/archived — permanently delete ALL archived positions.
pub(super) async fn delete_all_archived() -> Response {
    let archived = positions::get_archived_positions().await;
    if archived.is_empty() {
        return success_response(BulkDeleteResponse {
            success: true,
            deleted: 0,
            freed_slots: 0,
        });
    }

    // Archived positions already released their slot on archive, so no slot frees here.
    let ids: Vec<i64> = archived.iter().filter_map(|p| p.id).collect();

    let deleted = match positions::delete_archived_positions().await {
        Ok(n) => n,
        Err(e) => {
            return ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_POSITIONS_BULK_DELETE_FAILED,
            )
            .details(e.to_string())
            .into_response();
        }
    };

    for id in ids {
        positions::remove_position_by_id(id).await;
    }
    crate::trader::safety::loss_limit::sync_from_books().await;

    logger::info(
        LogTag::Positions,
        &format!("Permanently deleted {deleted} archived position(s)"),
    );

    success_response(BulkDeleteResponse {
        success: true,
        deleted,
        freed_slots: 0,
    })
}
