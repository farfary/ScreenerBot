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

use axum::{extract::Path, http::StatusCode, response::Response};
use serde::Serialize;

use crate::logger::{self, LogTag};
use crate::positions;
use crate::webserver::utils::{error_response, success_response};

#[derive(Debug, Serialize)]
pub struct ArchiveResponse {
    pub success: bool,
    pub position_id: i64,
    pub archived: bool,
    pub freed_slot: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub position_id: i64,
    pub freed_slot: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BulkDeleteResponse {
    pub success: bool,
    pub deleted: usize,
    pub freed_slots: usize,
    pub message: String,
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
            return error_response(
                StatusCode::NOT_FOUND,
                "POSITION_NOT_FOUND",
                "Position not found",
                Some(&format!("No position found with ID {position_id}")),
            );
        }
    };

    if position.archived {
        return error_response(
            StatusCode::BAD_REQUEST,
            "ALREADY_ARCHIVED",
            "Position is already archived",
            None,
        );
    }

    let was_open = holds_open_slot(&position);

    // Persist first, then mirror into memory so a failed write doesn't desync state.
    if let Err(e) = positions::set_position_archived_db(position_id, true).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ARCHIVE_FAILED",
            "Failed to archive position",
            Some(&e),
        );
    }
    positions::set_position_archived_in_memory(position_id, true).await;

    // Archiving an open position removes it from active management — free its slot.
    if was_open {
        positions::state::release_global_position_permit();
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
        message: "Position archived".to_owned(),
    })
}

/// POST /positions/:id/unarchive — restore a position from the Archived tab.
pub(super) async fn unarchive_position(Path(position_id): Path<i64>) -> Response {
    let position = match positions::get_position_by_id(position_id).await {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::NOT_FOUND,
                "POSITION_NOT_FOUND",
                "Position not found",
                Some(&format!("No position found with ID {position_id}")),
            );
        }
    };

    if !position.archived {
        return error_response(
            StatusCode::BAD_REQUEST,
            "NOT_ARCHIVED",
            "Position is not archived",
            None,
        );
    }

    if let Err(e) = positions::set_position_archived_db(position_id, false).await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "UNARCHIVE_FAILED",
            "Failed to unarchive position",
            Some(&e),
        );
    }
    positions::set_position_archived_in_memory(position_id, false).await;

    // If this position is still open it re-enters active management — reclaim a slot.
    let reclaimed = if holds_open_slot(&position) {
        let ok = positions::state::try_consume_global_position_permit();
        if !ok {
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
        message: "Position restored".to_owned(),
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
                    return error_response(
                        StatusCode::NOT_FOUND,
                        "POSITION_NOT_FOUND",
                        "Position not found",
                        Some(&format!("No position found with ID {position_id}")),
                    );
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
            return error_response(
                StatusCode::NOT_FOUND,
                "POSITION_NOT_FOUND",
                "Position not found",
                None,
            );
        }
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DELETE_FAILED",
                "Failed to delete position",
                Some(&e),
            );
        }
    }

    positions::remove_position_by_id(position_id).await;

    if was_open {
        positions::state::release_global_position_permit();
    }

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
        message: "Position permanently deleted".to_owned(),
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
            message: "No archived positions to delete".to_owned(),
        });
    }

    // Archived positions already released their slot on archive, so no slot frees here.
    let ids: Vec<i64> = archived.iter().filter_map(|p| p.id).collect();

    let deleted = match positions::delete_archived_positions().await {
        Ok(n) => n,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "BULK_DELETE_FAILED",
                "Failed to delete archived positions",
                Some(&e),
            );
        }
    };

    for id in ids {
        positions::remove_position_by_id(id).await;
    }

    logger::info(
        LogTag::Positions,
        &format!("Permanently deleted {deleted} archived position(s)"),
    );

    success_response(BulkDeleteResponse {
        success: true,
        deleted,
        freed_slots: 0,
        message: format!("Deleted {deleted} archived position(s)"),
    })
}
