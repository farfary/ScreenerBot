//! Wallet watch API routes — list / add / remove / enable / status for watch
//! targets, mounted under the `wallets` router (`/api/wallets/watch/*`) rather than
//! a copy-trading router: observation is a wallet-system feature and alert-only
//! watching must not require a copy task (PLAN.md §11.1).

use axum::{
    extract::Path,
    http::StatusCode,
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::logger::{self, LogTag};
use crate::wallets::watch::{self, WatchTarget};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_targets))
        .route("/", post(add_target))
        .route("/:id", delete(remove_target))
        .route("/:id/enabled", post(set_target_enabled))
        .route("/:id/status", get(get_status))
}

// =============================================================================
// TYPES
// =============================================================================

#[derive(Serialize)]
struct TargetListResponse {
    targets: Vec<WatchTarget>,
    total: usize,
}

#[derive(Deserialize)]
struct AddTargetRequest {
    address: String,
    #[serde(default)]
    label: Option<String>,
}

#[derive(Serialize)]
struct TargetResponse {
    message: String,
    target: WatchTarget,
}

#[derive(Deserialize)]
struct SetEnabledRequest {
    enabled: bool,
}

#[derive(Serialize)]
struct MessageResponse {
    message: String,
}

// =============================================================================
// HANDLERS
// =============================================================================

/// List every watch target (alert-only in this phase; the own wallet is not a row
/// here, see `wallets::watch`'s module doc).
async fn list_targets() -> Response {
    // Return promotional fixtures only for owner-initiated media capture. The real
    // call also fails outright when the watch database was never opened, which is
    // what puts "Watched addresses could not be loaded" on the tab.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        let targets = crate::webserver::promo::get_promo_watch_targets();
        let total = targets.len();
        return success_response(TargetListResponse { targets, total });
    }

    match watch::list_targets().await {
        Ok(targets) => {
            let total = targets.len();
            success_response(TargetListResponse { targets, total })
        }
        Err(e) => {
            logger::error(
                LogTag::WalletWatch,
                &format!("Failed to list watch targets: {e}"),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "LIST_ERROR",
                "Failed to list watch targets",
                Some(&e.to_string()),
            )
        }
    }
}

/// Add a new watch target. Base58 address validation, the self-copy guard (rejects
/// one of our own wallets) and `wallet.watch_max_targets` enforcement all happen
/// inside `watch::add_target`.
async fn add_target(Json(request): Json<AddTargetRequest>) -> Response {
    let address = request.address.trim();
    if address.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_ADDRESS",
            "Address cannot be empty",
            None,
        );
    }

    let label = request
        .label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    match watch::add_target(address, label).await {
        Ok(target) => success_response(TargetResponse {
            message: format!("Now watching {address}"),
            target,
        }),
        Err(e) => {
            logger::warning(
                LogTag::WalletWatch,
                &format!("Failed to add watch target {address}: {e}"),
            );
            let msg = e.to_string();
            let (status, code) = if msg.contains("already watched")
                || msg.contains("already one of your own wallets")
            {
                (StatusCode::CONFLICT, "DUPLICATE")
            } else if msg.contains("Invalid Solana address") {
                (StatusCode::BAD_REQUEST, "INVALID_ADDRESS")
            } else if msg.contains("limit reached") || msg.contains("disabled") {
                (StatusCode::BAD_REQUEST, "REJECTED")
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "ADD_ERROR")
            };
            error_response(status, code, "Failed to add watch target", Some(&msg))
        }
    }
}

/// Remove a watch target permanently (also drops its cursor).
async fn remove_target(Path(id): Path<i64>) -> Response {
    match watch::remove_target(id).await {
        Ok(()) => success_response(MessageResponse {
            message: "Watch target removed".to_owned(),
        }),
        Err(e) => {
            logger::warning(
                LogTag::WalletWatch,
                &format!("Failed to remove watch target {id}: {e}"),
            );
            let msg = e.to_string();
            let status = if msg.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            error_response(
                status,
                "REMOVE_ERROR",
                "Failed to remove watch target",
                Some(&msg),
            )
        }
    }
}

/// Enable or disable a target without deleting it (keeps its cursor, so
/// re-enabling resumes rather than re-scanning history).
async fn set_target_enabled(
    Path(id): Path<i64>,
    Json(request): Json<SetEnabledRequest>,
) -> Response {
    match watch::set_target_enabled(id, request.enabled).await {
        Ok(()) => success_response(MessageResponse {
            message: if request.enabled {
                "Watch target enabled".to_owned()
            } else {
                "Watch target disabled".to_owned()
            },
        }),
        Err(e) => {
            logger::warning(
                LogTag::WalletWatch,
                &format!("Failed to update watch target {id}: {e}"),
            );
            let msg = e.to_string();
            let status = if msg.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            error_response(
                status,
                "UPDATE_ERROR",
                "Failed to update watch target",
                Some(&msg),
            )
        }
    }
}

/// Per-target status: whether the shared transport is connected, when its cursor
/// last advanced, and to what.
async fn get_status(Path(id): Path<i64>) -> Response {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return match crate::webserver::promo::get_promo_watch_status(id) {
            Some(status) => success_response(status),
            None => error_response(
                StatusCode::NOT_FOUND,
                "STATUS_ERROR",
                "Failed to get watch status",
                Some("Watch target not found"),
            ),
        };
    }

    match watch::get_status(id).await {
        Ok(status) => success_response(status),
        Err(e) => {
            let msg = e.to_string();
            let status_code = if msg.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            error_response(
                status_code,
                "STATUS_ERROR",
                "Failed to get watch status",
                Some(&msg),
            )
        }
    }
}
