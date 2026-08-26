//! CRUD handlers for wallet management
//!
//! Basic wallet operations: list, create, import, get, update, delete, archive, restore.

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Response,
    Json,
};

use crate::logger::{self, LogTag};
use crate::wallets::{
    self, CreateWalletRequest, Error as WalletsError, ImportWalletRequest, UpdateWalletRequest,
};
use crate::webserver::utils::{error_response, status_for, success_response};

use super::types::{
    DeleteResponse, ListWalletsQuery, SetMainResponse, WalletCreatedResponse, WalletListResponse,
};

// =============================================================================
// HANDLERS
// =============================================================================

/// List all wallets
pub async fn list_wallets(Query(query): Query<ListWalletsQuery>) -> Response {
    // Return promotional fixtures only for owner-initiated media capture — the real
    // list is the operator's own wallet records.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        let wallets = crate::webserver::promo::get_promo_wallets(query.include_inactive);
        let total = wallets.len();
        return success_response(WalletListResponse { wallets, total });
    }

    match wallets::list_wallets(query.include_inactive).await {
        Ok(wallets) => {
            let total = wallets.len();
            success_response(WalletListResponse { wallets, total })
        }
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to list wallets: {e}"));
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "LIST_ERROR",
                "Failed to list wallets",
                Some(&e.to_string()),
            )
        }
    }
}

/// Create a new wallet
pub async fn create_wallet(Json(request): Json<CreateWalletRequest>) -> Response {
    // Validate name
    if request.name.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
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
            logger::error(LogTag::Wallet, &format!("Failed to create wallet: {e}"));
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "CREATE_ERROR",
                "Failed to create wallet",
                Some(&e.to_string()),
            )
        }
    }
}

/// Import an existing wallet
pub async fn import_wallet(Json(request): Json<ImportWalletRequest>) -> Response {
    // Validate name
    if request.name.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_NAME",
            "Wallet name cannot be empty",
            None,
        );
    }

    // Validate private key is provided
    if request.private_key.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
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
            logger::error(LogTag::Wallet, &format!("Failed to import wallet: {e}"));

            let e_str = e.to_string();
            let (code, msg) = match e {
                WalletsError::WalletAlreadyExists { .. } => ("DUPLICATE", "Wallet already exists"),
                WalletsError::InvalidPrivateKey { .. } => {
                    ("INVALID_KEY", "Invalid private key format")
                }
                _ => ("IMPORT_ERROR", "Failed to import wallet"),
            };

            error_response(status_for(&e), code, msg, Some(&e_str))
        }
    }
}

/// Get wallet summary for dashboard
pub async fn get_summary() -> Response {
    match wallets::get_wallets_summary().await {
        Ok(summary) => success_response(summary),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get summary: {e}"));
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SUMMARY_ERROR",
                "Failed to get wallets summary",
                Some(&e.to_string()),
            )
        }
    }
}

/// Get main wallet info
pub async fn get_main_wallet() -> Response {
    match wallets::get_main_wallet().await {
        Ok(Some(wallet)) => success_response(wallet),
        Ok(None) => error_response(
            StatusCode::NOT_FOUND,
            "NO_MAIN_WALLET",
            "No main wallet configured",
            None,
        ),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get main wallet: {e}"));
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "MAIN_WALLET_ERROR",
                "Failed to get main wallet",
                Some(&e.to_string()),
            )
        }
    }
}

/// Get a specific wallet by ID
pub async fn get_wallet(Path(id): Path<i64>) -> Response {
    match wallets::get_wallet(id).await {
        Ok(Some(wallet)) => success_response(wallet),
        Ok(None) => error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "Wallet not found", None),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get wallet {id}: {e}"));
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "GET_ERROR",
                "Failed to get wallet",
                Some(&e.to_string()),
            )
        }
    }
}

/// Update wallet metadata
pub async fn update_wallet(
    Path(id): Path<i64>,
    Json(request): Json<UpdateWalletRequest>,
) -> Response {
    match wallets::update_wallet(id, request).await {
        Ok(wallet) => success_response(wallet),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to update wallet {id}: {e}"),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "UPDATE_ERROR",
                "Failed to update wallet",
                Some(&e.to_string()),
            )
        }
    }
}

/// Delete a wallet permanently
pub async fn delete_wallet(Path(id): Path<i64>) -> Response {
    match wallets::delete_wallet(id).await {
        Ok(()) => success_response(DeleteResponse {
            message: "Wallet deleted successfully".to_owned(),
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to delete wallet {id}: {e}"),
            );

            let e_str = e.to_string();
            let code = match e {
                WalletsError::InvalidWalletState { .. } => "MAIN_WALLET",
                _ => "DELETE_ERROR",
            };

            error_response(
                status_for(&e),
                code,
                "Failed to delete wallet",
                Some(&e_str),
            )
        }
    }
}

/// Export wallet private key
pub async fn export_wallet(Path(id): Path<i64>) -> Response {
    match wallets::export_wallet(id).await {
        Ok(export) => success_response(export),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to export wallet {id}: {e}"),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "EXPORT_ERROR",
                "Failed to export wallet",
                Some(&e.to_string()),
            )
        }
    }
}

/// Set a wallet as the main wallet
pub async fn set_main_wallet(Path(id): Path<i64>) -> Response {
    match wallets::set_main_wallet(id).await {
        Ok(wallet) => success_response(SetMainResponse {
            message: format!("'{}' is now the main wallet", wallet.name),
            wallet,
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to set main wallet {id}: {e}"),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SET_MAIN_ERROR",
                "Failed to set main wallet",
                Some(&e.to_string()),
            )
        }
    }
}

/// Archive a wallet (soft delete)
pub async fn archive_wallet(Path(id): Path<i64>) -> Response {
    match wallets::archive_wallet(id).await {
        Ok(()) => success_response(DeleteResponse {
            message: "Wallet archived successfully".to_owned(),
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to archive wallet {id}: {e}"),
            );

            let e_str = e.to_string();
            let code = match e {
                WalletsError::InvalidWalletState { .. } => "MAIN_WALLET",
                _ => "ARCHIVE_ERROR",
            };

            error_response(
                status_for(&e),
                code,
                "Failed to archive wallet",
                Some(&e_str),
            )
        }
    }
}

/// Restore an archived wallet
pub async fn restore_wallet(Path(id): Path<i64>) -> Response {
    match wallets::restore_wallet(id).await {
        Ok(()) => success_response(DeleteResponse {
            message: "Wallet restored successfully".to_owned(),
        }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to restore wallet {id}: {e}"),
            );

            let e_str = e.to_string();
            let code = match e {
                WalletsError::InvalidWalletState { .. } => "NOT_ARCHIVED",
                _ => "RESTORE_ERROR",
            };

            error_response(
                status_for(&e),
                code,
                "Failed to restore wallet",
                Some(&e_str),
            )
        }
    }
}
