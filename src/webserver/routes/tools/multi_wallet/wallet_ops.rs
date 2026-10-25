// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet management operation handlers
//!
//! Handles the wallet summary and sub-wallet consolidation.

use axum::{response::Response, Json};

use std::collections::HashSet;

use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::tools::multi_wallet::{execute_consolidation, ConsolidateConfig, TokenHoldings};
use crate::tools::Error;
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
///
/// Every active wallet with its SOL balance; sub-wallets also carry what a
/// consolidation can move (token balances by mint) and what a cleanup can
/// reclaim (empty token accounts and the rent deposits they hold).
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
    let mut sub_wallet_mints = HashSet::new();
    let mut reclaimable_rent = 0.0;

    for wallet in &all_wallets {
        // Get SOL balance via RPC
        let sol_balance = rpc
            .get_sol_balance(&wallet.address)
            .await
            .unwrap_or_default();

        total_native += sol_balance;

        let holdings = if wallet.role == wallets::WalletRole::Secondary {
            match rpc.get_all_token_accounts_str(&wallet.address).await {
                Ok(accounts) => TokenHoldings::from_accounts(&accounts),
                Err(e) => {
                    return ApiError::new(
                        ApiErrorCode::Internal,
                        ids::ERRORS_TOOLS_WALLETS_GET_FAILED,
                    )
                    .details(e.to_string())
                    .into_response();
                }
            }
        } else {
            TokenHoldings::default()
        };
        sub_wallet_mints.extend(holdings.mints.iter().cloned());
        reclaimable_rent += holdings.reclaimable_rent;

        let info = WalletInfoResponse {
            id: wallet.id,
            address: wallet.address.clone(),
            name: wallet.name.clone(),
            role: format!("{:?}", wallet.role).to_lowercase(),
            sol_balance,
            is_active: wallet.is_active,
            token_count: holdings.mints.len(),
            token_mints: holdings.mints,
            empty_atas: holdings.empty_accounts,
            reclaimable_rent: holdings.reclaimable_rent,
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
        token_types: sub_wallet_mints.len(),
        reclaimable_rent,
        wallets: wallets_info,
    })
}

/// Consolidate the selected sub-wallets: the operations the request names, on
/// the wallets it names, and nothing else.
pub async fn consolidate_wallets(Json(request): Json<ConsolidateRequest>) -> Response {
    logger::info(
        LogTag::Tools,
        &format!(
            "Starting consolidation: wallets={}, sol={}, tokens={}, close_atas={}",
            request.wallet_ids.len(),
            request.transfer_sol,
            request.transfer_tokens.len(),
            request.close_atas
        ),
    );

    let config = ConsolidateConfig {
        wallet_ids: request.wallet_ids,
        transfer_sol: request.transfer_sol,
        transfer_tokens: request.transfer_tokens,
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
        Ok(Ok(result)) => success_response(ConsolidateResponse {
            session_id: result.session.session_id,
            total_wallets: result.wallets,
            successful_ops: result.session.successful_ops,
            failed_ops: result.session.failed_ops,
            sol_recovered: result.sol_transferred,
            tokens_transferred: result.tokens_transferred,
            accounts_closed: result.accounts_closed,
            rent_reclaimed: result.rent_reclaimed_sol,
        }),
        Ok(Err(e @ Error::InvalidConfig { .. })) => invalid_config(&e).into_response(),
        Ok(Err(e)) => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_TOOLS_CONSOLIDATE_FAILED)
            .details(e.to_string())
            .into_response(),
        Err(e) => ApiError::new(ApiErrorCode::Internal, ids::ERRORS_TOOLS_CONSOLIDATE_FAILED)
            .details(e.to_string())
            .into_response(),
    }
}
