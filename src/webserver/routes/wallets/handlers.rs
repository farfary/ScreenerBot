// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet handlers: list, summary, main wallet, one wallet and its private-key export, and the trading-wallet replacement with its backups.

use axum::{
    extract::{Json, Path, Query},
    response::Response,
};

use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::process::restart_safety::RestartBlocker;
use crate::wallets;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::routes::initialization::validation::consume_validated_key;
use crate::webserver::routes::initialization::CompleteInitializationRequest;
use crate::webserver::utils::success_response;
use axum::response::IntoResponse as _;

use super::types::{
    ListWalletsQuery, WalletBackupsResponse, WalletListEntry, WalletListResponse,
    WalletReplaceResponse,
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

/// Replace the trading wallet with a key that passed setup validation: stage the
/// replacement, then restart so the next start applies it.
pub async fn replace_wallet(Json(request): Json<CompleteInitializationRequest>) -> Response {
    let key = match consume_validated_key(&request) {
        Ok(key) => key,
        Err(error) => return error.into_response(),
    };

    let staged = match wallets::replacement::stage(key).await {
        Ok(staged) => staged,
        Err(e) => {
            logger::warning(
                LogTag::Wallet,
                &format!("Trading wallet replacement refused: {e}"),
            );
            return replacement_error(&e).into_response();
        }
    };

    crate::webserver::routes::system::schedule_graceful_restart("wallet replaced");
    success_response(WalletReplaceResponse {
        restart_required: true,
        instance_id: crate::global::instance_id().to_owned(),
        previous_wallet: staged.previous_wallet,
        new_wallet: staged.new_wallet,
        backup_directory: staged.backup_directory,
    })
}

/// List the wallet-reset backups, newest first, with the replacement a start abandoned
/// when one has not been dismissed yet.
pub async fn list_wallet_backups() -> Response {
    let listed = tokio::task::spawn_blocking(|| {
        (
            wallets::replacement::list_backups(),
            wallets::replacement::abandoned_replacement(),
        )
    })
    .await;
    match listed {
        Ok((backups, abandoned)) => success_response(WalletBackupsResponse { backups, abandoned }),
        Err(e) => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_BACKUPS_FAILED)
            .details(e.to_string())
            .into_response(),
    }
}

/// Forget the abandoned replacement once the Wallets page has shown it.
pub async fn dismiss_abandoned_replacement() -> Response {
    let dismissed =
        tokio::task::spawn_blocking(wallets::replacement::dismiss_abandoned_replacement).await;
    match dismissed {
        Ok(Ok(())) => success_response(serde_json::json!({ "dismissed": true })),
        Ok(Err(e)) => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_BACKUPS_FAILED)
            .details(e.to_string())
            .into_response(),
        Err(e) => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_BACKUPS_FAILED)
            .details(e.to_string())
            .into_response(),
    }
}

/// The API error of a refused or failed replacement, naming its reason.
pub(in crate::webserver::routes) fn replacement_error(error: &wallets::Error) -> ApiError {
    let details = error.to_string();
    let api_error = match error {
        wallets::Error::ReplacementBlocked { reason } => match reason {
            RestartBlocker::OpenPositions { count } => ApiError::new(
                ApiErrorCode::OpenPositions,
                ids::ERRORS_WALLETS_REPLACE_OPEN_POSITIONS,
            )
            .count_arg("count", i64::try_from(*count).unwrap_or(i64::MAX)),
            RestartBlocker::TradeInFlight => ApiError::new(
                ApiErrorCode::Conflict,
                ids::ERRORS_WALLETS_REPLACE_TRADE_IN_FLIGHT,
            ),
            RestartBlocker::ToolInFlight => ApiError::new(
                ApiErrorCode::Conflict,
                ids::ERRORS_WALLETS_REPLACE_TOOL_IN_FLIGHT,
            ),
            RestartBlocker::PositionsLoading => ApiError::new(
                ApiErrorCode::Conflict,
                ids::ERRORS_WALLETS_REPLACE_POSITIONS_LOADING,
            ),
            RestartBlocker::PositionsUnreadable => ApiError::new(
                ApiErrorCode::Conflict,
                ids::ERRORS_WALLETS_REPLACE_POSITIONS_UNREADABLE,
            ),
        },
        wallets::Error::ReplacementRestartInProgress => ApiError::new(
            ApiErrorCode::Conflict,
            ids::ERRORS_WALLETS_REPLACE_RESTART_IN_PROGRESS,
        ),
        wallets::Error::ReplacementPending => {
            ApiError::new(ApiErrorCode::Conflict, ids::ERRORS_WALLETS_REPLACE_PENDING)
        }
        wallets::Error::ReplacementNoTradingWallet => ApiError::new(
            ApiErrorCode::Conflict,
            ids::ERRORS_WALLETS_REPLACE_NO_TRADING_WALLET,
        ),
        wallets::Error::ReplacementSameWallet { .. } => ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_WALLETS_REPLACE_SAME_WALLET,
        ),
        wallets::Error::ReplacementWalletIsWatched { .. } => ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_WALLETS_REPLACE_WATCHED,
        ),
        wallets::Error::ReplacementAddressMismatch { .. } => ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_WALLETS_REPLACE_ADDRESS_MISMATCH,
        ),
        wallets::Error::ReplacementInterrupted {
            copy_trading_off: true,
            ..
        } => ApiError::new(
            ApiErrorCode::Internal,
            ids::ERRORS_WALLETS_REPLACE_INTERRUPTED,
        ),
        wallets::Error::ReplacementInterrupted {
            copy_trading_off: false,
            ..
        } => ApiError::new(
            ApiErrorCode::Internal,
            ids::ERRORS_WALLETS_REPLACE_INTERRUPTED_COPY_TRADING_ON,
        ),
        _ => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_REPLACE_FAILED),
    };
    api_error.details(details)
}
