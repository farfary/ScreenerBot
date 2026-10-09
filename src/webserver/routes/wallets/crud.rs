// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! CRUD handlers for wallet management
//!
//! Basic wallet operations: list, create, import, get, update, delete, archive, restore.

use axum::{
    extract::{Path, Query},
    response::Response,
    Json,
};

use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::wallets::{
    self, CreateWalletRequest, Error as WalletsError, ImportWalletRequest, UpdateWalletRequest,
};
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::{status_for, success_response};
use axum::response::IntoResponse as _;

use super::types::{
    ListWalletsQuery, SetMainResponse, WalletCreatedResponse, WalletListEntry, WalletListResponse,
};

/// The category for a failed wallet operation, carrying the status the typed
/// error already has.
fn failure_code(error: &WalletsError) -> ApiErrorCode {
    ApiErrorCode::for_status(status_for(error).as_u16())
}

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
            let balances = wallets::get_wallet_sol_balances(&wallets).await;
            let wallets: Vec<WalletListEntry> = wallets
                .into_iter()
                .zip(balances)
                .map(|(wallet, balance)| WalletListEntry { wallet, balance })
                .collect();
            let total = wallets.len();
            success_response(WalletListResponse { wallets, total })
        }
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to list wallets: {e}"));
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_LIST_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// Create a new wallet
pub async fn create_wallet(Json(request): Json<CreateWalletRequest>) -> Response {
    // Validate name
    if request.name.trim().is_empty() {
        return ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_WALLETS_NAME_EMPTY)
            .into_response();
    }

    match wallets::create_wallet(request).await {
        Ok(wallet) => success_response(WalletCreatedResponse { wallet }),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to create wallet: {e}"));
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_CREATE_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// Import an existing wallet
pub async fn import_wallet(Json(request): Json<ImportWalletRequest>) -> Response {
    // Validate name
    if request.name.trim().is_empty() {
        return ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_WALLETS_NAME_EMPTY)
            .into_response();
    }

    // Validate private key is provided
    if request.private_key.trim().is_empty() {
        return ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_WALLETS_KEY_EMPTY)
            .into_response();
    }

    match wallets::import_wallet(request).await {
        Ok(wallet) => success_response(WalletCreatedResponse { wallet }),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to import wallet: {e}"));

            let message = match e {
                WalletsError::WalletAlreadyExists { .. } => ids::ERRORS_WALLETS_ALREADY_EXISTS,
                WalletsError::InvalidPrivateKey { .. } => ids::ERRORS_WALLETS_KEY_INVALID,
                _ => ids::ERRORS_WALLETS_IMPORT_FAILED,
            };

            ApiError::new(failure_code(&e), message)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// Get wallet summary for dashboard
pub async fn get_summary() -> Response {
    match wallets::get_wallets_summary().await {
        Ok(summary) => success_response(summary),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get summary: {e}"));
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_SUMMARY_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// Get main wallet info
pub async fn get_main_wallet() -> Response {
    match wallets::get_main_wallet().await {
        Ok(Some(wallet)) => success_response(wallet),
        Ok(None) => ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_WALLETS_NO_MAIN_WALLET)
            .into_response(),
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get main wallet: {e}"));
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_MAIN_GET_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// Get a specific wallet by ID
pub async fn get_wallet(Path(id): Path<i64>) -> Response {
    match wallets::get_wallet(id).await {
        Ok(Some(wallet)) => success_response(wallet),
        Ok(None) => {
            ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_WALLETS_NOT_FOUND).into_response()
        }
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get wallet {id}: {e}"));
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_GET_FAILED)
                .details(e.to_string())
                .into_response()
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
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_UPDATE_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// Delete a wallet permanently
pub async fn delete_wallet(Path(id): Path<i64>) -> Response {
    match wallets::delete_wallet(id).await {
        Ok(()) => success_response(serde_json::json!({})),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to delete wallet {id}: {e}"),
            );

            ApiError::new(failure_code(&e), ids::ERRORS_WALLETS_DELETE_FAILED)
                .details(e.to_string())
                .into_response()
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
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_EXPORT_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// Set a wallet as the main wallet
pub async fn set_main_wallet(Path(id): Path<i64>) -> Response {
    match wallets::set_main_wallet(id).await {
        Ok(wallet) => success_response(SetMainResponse { wallet }),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to set main wallet {id}: {e}"),
            );
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_SET_MAIN_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// Archive a wallet (soft delete)
pub async fn archive_wallet(Path(id): Path<i64>) -> Response {
    match wallets::archive_wallet(id).await {
        Ok(()) => success_response(serde_json::json!({})),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to archive wallet {id}: {e}"),
            );

            ApiError::new(failure_code(&e), ids::ERRORS_WALLETS_ARCHIVE_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}

/// Restore an archived wallet
pub async fn restore_wallet(Path(id): Path<i64>) -> Response {
    match wallets::restore_wallet(id).await {
        Ok(()) => success_response(serde_json::json!({})),
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to restore wallet {id}: {e}"),
            );

            ApiError::new(failure_code(&e), ids::ERRORS_WALLETS_RESTORE_FAILED)
                .details(e.to_string())
                .into_response()
        }
    }
}
