// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet handlers: list, summary, main wallet, one wallet and its private-key export.

use axum::{
    extract::{Path, Query},
    response::Response,
};

use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::wallets;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;
use axum::response::IntoResponse as _;

use super::types::{ListWalletsQuery, WalletListEntry, WalletListResponse};

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
