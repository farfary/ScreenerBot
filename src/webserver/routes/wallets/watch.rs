//! Wallet watch API routes — list / add / remove / enable / status for watch
//! targets, mounted under the `wallets` router (`/api/wallets/watch/*`) rather than
//! a copy-trading router: observation is a wallet-system feature and alert-only
//! watching must not require a copy task (PLAN.md §11.1).

use axum::{
    extract::Path,
    response::{IntoResponse as _, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::wallets::watch::{self, WatchTarget};
use crate::wallets::Error as WalletsError;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::state::AppState;
use crate::webserver::utils::{status_for, success_response};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_targets))
        .route("/", post(add_target))
        .route("/{id}", delete(remove_target))
        .route("/{id}/enabled", post(set_target_enabled))
        .route("/{id}/budget", post(set_target_budget))
        .route(
            "/{id}/high-activity-approval",
            post(set_high_activity_approval),
        )
        .route("/{id}/resume", post(resume_target))
        .route("/{id}/status", get(get_status))
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

#[derive(Deserialize)]
struct SetBudgetRequest {
    page_budget: usize,
}

#[derive(Deserialize)]
struct ResumeRequest {
    page_budget: usize,
    acknowledge_missed_activity: bool,
}

#[derive(Deserialize)]
struct HighActivityApprovalRequest {
    approved: bool,
    acknowledge_provider_usage: bool,
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
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLET_WATCH_LIST_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// Add a new watch target. Base58 address validation, the self-copy guard (rejects
/// one of our own wallets) and `wallet.watch_max_targets` enforcement all happen
/// inside `watch::add_target`.
async fn add_target(Json(request): Json<AddTargetRequest>) -> Response {
    let address = request.address.trim();
    if address.is_empty() {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_WALLET_WATCH_ADDRESS_EMPTY,
        )
        .into_response();
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
            ApiError::new(failure_code(&e), ids::ERRORS_WALLET_WATCH_ADD_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// The category for a failed watch operation, carrying the status the typed
/// error already has.
fn failure_code(error: &WalletsError) -> ApiErrorCode {
    ApiErrorCode::for_status(status_for(error).as_u16())
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
            ApiError::new(failure_code(&e), ids::ERRORS_WALLET_WATCH_REMOVE_FAILED)
                .details(e.to_string())
                .into_response()
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
            ApiError::new(failure_code(&e), ids::ERRORS_WALLET_WATCH_UPDATE_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

async fn set_target_budget(Path(id): Path<i64>, Json(request): Json<SetBudgetRequest>) -> Response {
    match watch::update_target_page_budget(id, request.page_budget).await {
        Ok(()) => success_response(MessageResponse {
            message: "Watch budget updated".to_owned(),
        }),
        Err(error) => ApiError::new(failure_code(&error), ids::ERRORS_WALLET_WATCH_BUDGET_FAILED)
            .details(error.to_string())
            .into_response(),
    }
}

async fn resume_target(Path(id): Path<i64>, Json(request): Json<ResumeRequest>) -> Response {
    match watch::resume_target(id, request.page_budget, request.acknowledge_missed_activity).await {
        Ok(()) => success_response(MessageResponse {
            message: "Watch resumed from the current head".to_owned(),
        }),
        Err(error) => ApiError::new(failure_code(&error), ids::ERRORS_WALLET_WATCH_RESUME_FAILED)
            .details(error.to_string())
            .into_response(),
    }
}

async fn set_high_activity_approval(
    Path(id): Path<i64>,
    Json(request): Json<HighActivityApprovalRequest>,
) -> Response {
    match watch::set_target_high_activity_approved(
        id,
        request.approved,
        request.acknowledge_provider_usage,
    )
    .await
    {
        Ok(()) => success_response(MessageResponse {
            message: if request.approved {
                "Helius approval saved; watch restored from its saved cursor".to_owned()
            } else {
                "Helius approval removed".to_owned()
            },
        }),
        Err(error) => ApiError::new(
            failure_code(&error),
            ids::ERRORS_WALLET_WATCH_APPROVAL_FAILED,
        )
        .details(error.to_string())
        .into_response(),
    }
}

/// Per-target status: whether the shared transport is connected, when its cursor
/// last advanced, and to what.
async fn get_status(Path(id): Path<i64>) -> Response {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return match crate::webserver::promo::get_promo_watch_status(id) {
            Some(status) => success_response(status),
            None => ApiError::new(
                ApiErrorCode::NotFound,
                ids::ERRORS_WALLET_WATCH_STATUS_FAILED,
            )
            .details("Watch target not found")
            .into_response(),
        };
    }

    match watch::get_status(id).await {
        Ok(status) => success_response(status),
        Err(e) => ApiError::new(failure_code(&e), ids::ERRORS_WALLET_WATCH_STATUS_FAILED)
            .details(e.to_string())
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    /// The statuses the dashboard depends on, asserted against the typed value
    /// rather than the sentence it renders to: `watched.js` reads 409 to say
    /// "that wallet is already watched", and a missing target must be a 404 so
    /// a stale row in the list is distinguishable from a server fault.
    #[test]
    fn watch_failures_map_to_their_documented_statuses() {
        let cases = [
            (
                WalletsError::WatchTargetAlreadyWatched {
                    address: "addr".to_owned(),
                },
                StatusCode::CONFLICT,
                ApiErrorCode::Conflict,
            ),
            (
                WalletsError::WatchTargetIsOwnWallet {
                    address: "addr".to_owned(),
                },
                StatusCode::CONFLICT,
                ApiErrorCode::Conflict,
            ),
            (
                WalletsError::InvalidWatchAddress {
                    value: "nope".to_owned(),
                },
                StatusCode::BAD_REQUEST,
                ApiErrorCode::InvalidInput,
            ),
            (
                WalletsError::WatchDisabled,
                StatusCode::BAD_REQUEST,
                ApiErrorCode::InvalidInput,
            ),
            (
                WalletsError::WatchTargetLimitReached { max: 5 },
                StatusCode::BAD_REQUEST,
                ApiErrorCode::InvalidInput,
            ),
            (
                WalletsError::WatchHeliusApprovalAcknowledgementRequired,
                StatusCode::BAD_REQUEST,
                ApiErrorCode::InvalidInput,
            ),
        ];

        for (error, status, code) in cases {
            assert_eq!(status_for(&error), status, "status for {error}");
            assert_eq!(failure_code(&error), code, "code for {error}");
            assert_eq!(code.status(), status, "status of {code:?}");
        }
    }

    /// A target that is gone is a 404 on every endpoint addressed by id —
    /// remove, enable/disable and status all read the same typed variant.
    #[test]
    fn a_missing_watch_target_is_not_a_server_error() {
        let error = WalletsError::WatchTargetNotFound {
            address: "id=7".to_owned(),
        };
        assert_eq!(status_for(&error), StatusCode::NOT_FOUND);
    }
}
