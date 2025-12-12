//! Wallet management API routes
//!
//! CRUD endpoints for multi-wallet management.

use axum::{
    extract::{Path, Query, State},
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::logger::{self, LogTag};
use crate::wallets::{
    self, CreateWalletRequest, ExportWalletResponse, ImportWalletRequest, UpdateWalletRequest,
    Wallet, WalletsSummary,
};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};
use std::sync::Arc;

// =============================================================================
// RESPONSE TYPES
// =============================================================================

#[derive(Serialize)]
struct WalletListResponse {
    wallets: Vec<Wallet>,
    total: usize,
}

#[derive(Serialize)]
struct WalletCreatedResponse {
    message: String,
    wallet: Wallet,
}

#[derive(Serialize)]
struct SetMainResponse {
    message: String,
    wallet: Wallet,
}

#[derive(Serialize)]
struct DeleteResponse {
    message: String,
}

// =============================================================================
// QUERY PARAMS
// =============================================================================

#[derive(Deserialize)]
pub struct ListWalletsQuery {
    #[serde(default)]
    pub include_inactive: bool,
}

// =============================================================================
// ROUTES
// =============================================================================

/// Create wallet routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_wallets))
        .route("/", post(create_wallet))
        .route("/import", post(import_wallet))
        .route("/summary", get(get_summary))
        .route("/main", get(get_main_wallet))
        .route("/:id", get(get_wallet))
        .route("/:id", put(update_wallet))
        .route("/:id", delete(delete_wallet))
        .route("/:id/export", post(export_wallet))
        .route("/:id/set-main", post(set_main_wallet))
        .route("/:id/archive", post(archive_wallet))
        .route("/:id/restore", post(restore_wallet))
}

// =============================================================================
// HANDLERS
// =============================================================================

/// List all wallets
async fn list_wallets(Query(query): Query<ListWalletsQuery>) -> Response {
    match wallets::list_wallets(query.include_inactive).await {
        Ok(wallets) => {
            let total = wallets.len();
            success_response(WalletListResponse { wallets, total })
        }
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to list wallets: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "LIST_ERROR",
                "Failed to list wallets",
                Some(&e),
            )
        }
    }
}

/// Create a new wallet
async fn create_wallet(Json(request): Json<CreateWalletRequest>) -> Response {
    // Validate name
    if request.name.trim().is_empty() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_NAME",
            "Wallet name cannot be empty",
            None,
        );
    }

    match wallets::create_wallet(request).await {
        Ok(wallet) => success_response(WalletCreatedResponse {
            message: format!("Wallet '{}' created successfully", wallet.name),
            wallet,
        }),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to create wallet: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "CREATE_ERROR",
                "Failed to create wallet",
                Some(&e),
            )
        }
    }
}

/// Import an existing wallet
async fn import_wallet(Json(request): Json<ImportWalletRequest>) -> Response {
    // Validate name
    if request.name.trim().is_empty() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_NAME",
            "Wallet name cannot be empty",
            None,
        );
    }

    // Validate private key is provided
    if request.private_key.trim().is_empty() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_KEY",
            "Private key cannot be empty",
            None,
        );
    }

    match wallets::import_wallet(request).await {
        Ok(wallet) => success_response(WalletCreatedResponse {
            message: format!("Wallet '{}' imported successfully", wallet.name),
            wallet,
        }),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to import wallet: {}", e));

            // Check for specific error types
            let (status, code, msg) = if e.contains("already exists") {
                (
                    axum::http::StatusCode::CONFLICT,
                    "DUPLICATE",
                    "Wallet already exists",
                )
            } else if e.contains("Invalid") {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    "INVALID_KEY",
                    "Invalid private key format",
                )
            } else {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "IMPORT_ERROR",
                    "Failed to import wallet",
                )
            };

            error_response(status, code, msg, Some(&e))
        }
    }
}

/// Get wallet summary for dashboard
async fn get_summary() -> Response {
    match wallets::get_wallets_summary().await {
        Ok(summary) => success_response(summary),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get summary: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "SUMMARY_ERROR",
                "Failed to get wallets summary",
                Some(&e),
            )
        }
    }
}

/// Get main wallet info
async fn get_main_wallet() -> Response {
    match wallets::get_main_wallet().await {
        Ok(Some(wallet)) => success_response(wallet),
        Ok(None) => error_response(
            axum::http::StatusCode::NOT_FOUND,
            "NO_MAIN_WALLET",
            "No main wallet configured",
            None,
        ),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get main wallet: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "MAIN_WALLET_ERROR",
                "Failed to get main wallet",
                Some(&e),
            )
        }
    }
}

/// Get a specific wallet by ID
async fn get_wallet(Path(id): Path<i64>) -> Response {
    match wallets::get_wallet(id).await {
        Ok(Some(wallet)) => success_response(wallet),
        Ok(None) => error_response(
            axum::http::StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Wallet not found",
            None,
        ),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get wallet {}: {}", id, e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "GET_ERROR",
                "Failed to get wallet",
                Some(&e),
            )
        }
    }
}

/// Update wallet metadata
async fn update_wallet(Path(id): Path<i64>, Json(request): Json<UpdateWalletRequest>) -> Response {
    match wallets::update_wallet(id, request).await {
        Ok(wallet) => success_response(wallet),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to update wallet {}: {}", id, e),
            );
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "UPDATE_ERROR",
                "Failed to update wallet",
                Some(&e),
            )
        }
    }
}

/// Delete a wallet permanently
async fn delete_wallet(Path(id): Path<i64>) -> Response {
    match wallets::delete_wallet(id).await {
        Ok(()) => success_response(DeleteResponse {
            message: "Wallet deleted successfully".to_string(),
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to delete wallet {}: {}", id, e),
            );

            let (status, code) = if e.contains("main wallet") {
                (axum::http::StatusCode::BAD_REQUEST, "MAIN_WALLET")
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "DELETE_ERROR")
            };

            error_response(status, code, "Failed to delete wallet", Some(&e))
        }
    }
}

/// Export wallet private key
async fn export_wallet(Path(id): Path<i64>) -> Response {
    match wallets::export_wallet(id).await {
        Ok(export) => success_response(export),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to export wallet {}: {}", id, e),
            );
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "EXPORT_ERROR",
                "Failed to export wallet",
                Some(&e),
            )
        }
    }
}

/// Set a wallet as the main wallet
async fn set_main_wallet(Path(id): Path<i64>) -> Response {
    match wallets::set_main_wallet(id).await {
        Ok(wallet) => success_response(SetMainResponse {
            message: format!("'{}' is now the main wallet", wallet.name),
            wallet,
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to set main wallet {}: {}", id, e),
            );
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "SET_MAIN_ERROR",
                "Failed to set main wallet",
                Some(&e),
            )
        }
    }
}

/// Archive a wallet (soft delete)
async fn archive_wallet(Path(id): Path<i64>) -> Response {
    match wallets::archive_wallet(id).await {
        Ok(()) => success_response(DeleteResponse {
            message: "Wallet archived successfully".to_string(),
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to archive wallet {}: {}", id, e),
            );

            let (status, code) = if e.contains("main wallet") {
                (axum::http::StatusCode::BAD_REQUEST, "MAIN_WALLET")
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "ARCHIVE_ERROR")
            };

            error_response(status, code, "Failed to archive wallet", Some(&e))
        }
    }
}

/// Restore an archived wallet
async fn restore_wallet(Path(id): Path<i64>) -> Response {
    match wallets::restore_wallet(id).await {
        Ok(()) => success_response(DeleteResponse {
            message: "Wallet restored successfully".to_string(),
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to restore wallet {}: {}", id, e),
            );

            let (status, code) = if e.contains("not archived") {
                (axum::http::StatusCode::BAD_REQUEST, "NOT_ARCHIVED")
            } else {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "RESTORE_ERROR")
            };

            error_response(status, code, "Failed to restore wallet", Some(&e))
        }
    }
}
