// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet management operation handlers
//!
//! Handles wallet summary, consolidation, and ATA cleanup operations.

use axum::{response::Response, Json};

use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::tools::multi_wallet::{execute_consolidation, ConsolidateConfig};
use crate::wallets;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;
use axum::response::IntoResponse as _;

use super::super::types::*;
use super::config_error::invalid_config;

// =============================================================================
// Wallet Management Handlers
// =============================================================================

/// Get wallets summary
pub async fn get_wallets_summary() -> Response {
    // Get all wallets
    let all_wallets = match wallets::list_active_wallets().await {
        Ok(w) => w,
        Err(e) => {
            return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_TOOLS_WALLETS_GET_FAILED)
                .details(e.to_string())
                .into_response();
        }
    };

    let rpc = get_rpc_client();
    let mut wallets_info = Vec::new();
    let mut main_wallet_info = None;
    let mut total_native = 0.0;
    let mut secondary_count = 0;

    for wallet in &all_wallets {
        // Get SOL balance via RPC
        let sol_balance = rpc
            .get_sol_balance(&wallet.address)
            .await
            .unwrap_or_default();

        total_native += sol_balance;

        let info = WalletInfoResponse {
            id: wallet.id,
            address: wallet.address.clone(),
            name: wallet.name.clone(),
            role: format!("{:?}", wallet.role).to_lowercase(),
            sol_balance,
            is_active: wallet.is_active,
        };

        if wallet.role == wallets::WalletRole::Main {
            main_wallet_info = Some(info.clone());
        } else if wallet.role == wallets::WalletRole::Secondary {
            secondary_count += 1;
        }

        wallets_info.push(info);
    }

    success_response(WalletsSummaryResponse {
        total_wallets: all_wallets.len(),
        secondary_wallets: secondary_count,
        main_wallet: main_wallet_info,
        total_native,
        wallets: wallets_info,
    })
}

/// Consolidate wallets
pub async fn consolidate_wallets(Json(request): Json<ConsolidateRequest>) -> Response {
    logger::info(
        LogTag::Tools,
        &format!(
            "Starting consolidation: sol={}, tokens={:?}, close_atas={}",
            request.transfer_sol,
            request.transfer_tokens.as_ref().map(|t| t.len()),
            request.close_atas
        ),
    );

    let config = ConsolidateConfig {
        wallet_ids: request.wallet_ids.clone(),
        transfer_sol: request.transfer_sol,
        transfer_tokens: request.transfer_tokens.clone(),
        close_atas: request.close_atas,
        include_token_2022: request.include_token_2022,
        leave_rent_exempt: request.leave_rent_exempt,
    };

    // Validate config
    if let Err(e) = config.validate() {
        return invalid_config(&e).into_response();
    }

    // The transfers run on their own task: a dropped request must not stop
    // the loop between two sends, nor lose the session record of one sent.
    match tokio::spawn(execute_consolidation(config)).await {
        Ok(Ok(result)) => {
            logger::info(
                LogTag::Tools,
                &format!(
                    "Consolidation complete: {}/{} successful, {:.6} SOL recovered",
                    result.successful_ops, result.total_wallets, result.total_sol_recovered
                ),
            );

            success_response(ConsolidateResponse {
                session_id: result.session_id,
                total_wallets: result.total_wallets,
                successful_ops: result.successful_ops,
                failed_ops: result.failed_ops,
                sol_recovered: result.total_sol_recovered,
            })
        }
        Ok(Err(e)) => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_TOOLS_CONSOLIDATE_FAILED)
            .details(e.to_string())
            .into_response(),
        Err(e) => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_TOOLS_CONSOLIDATE_FAILED)
            .details(e.to_string())
            .into_response(),
    }
}

/// Cleanup ATAs on sub-wallets
pub async fn cleanup_subwallet_atas(Json(request): Json<SubWalletAtaCleanupRequest>) -> Response {
    logger::info(
        LogTag::Tools,
        &format!(
            "Starting sub-wallet ATA cleanup: wallets={:?}",
            request.wallet_ids
        ),
    );

    // Use consolidation with only close_atas enabled
    let config = ConsolidateConfig {
        wallet_ids: request.wallet_ids.clone(),
        transfer_sol: false,
        transfer_tokens: None,
        close_atas: true,
        include_token_2022: request.include_token_2022,
        leave_rent_exempt: true,
    };

    // Each account close is a sent transaction; the loop runs on its own task.
    match tokio::spawn(execute_consolidation(config)).await {
        Ok(Ok(result)) => {
            logger::info(
                LogTag::Tools,
                &format!(
                    "Sub-wallet ATA cleanup complete: {}/{} successful, {:.6} SOL recovered",
                    result.successful_ops, result.total_wallets, result.total_sol_recovered
                ),
            );

            success_response(ConsolidateResponse {
                session_id: result.session_id,
                total_wallets: result.total_wallets,
                successful_ops: result.successful_ops,
                failed_ops: result.failed_ops,
                sol_recovered: result.total_sol_recovered,
            })
        }
        Ok(Err(e)) => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_TOOLS_ATA_CLEANUP_FAILED)
            .details(e.to_string())
            .into_response(),
        Err(e) => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_TOOLS_ATA_CLEANUP_FAILED)
            .details(e.to_string())
            .into_response(),
    }
}
