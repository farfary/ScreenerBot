//! Multi-sell operation handlers
//!
//! Handles preview, start, status, and abort for multi-sell operations.

use axum::{extract::Path, http::StatusCode, response::Response, Json};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::logger::{self, LogTag};
use crate::tokens::decimals;
use crate::tools::multi_wallet::{
    execute_multi_sell, MultiSellConfig, SessionResult, SessionStatus,
};
use crate::tools::DelayConfig;
use crate::wallets;
use crate::webserver::utils::{error_response, success_response};

use super::super::types::*;
use super::session::{
    cleanup_old_sessions, get_session_status, has_active_multi_wallet_session, MultiWalletSession,
    MULTI_WALLET_SESSIONS,
};

// =============================================================================
// Multi-Sell Handlers
// =============================================================================

/// Preview multi-sell operation
pub async fn preview_multi_sell(Json(request): Json<MultiSellPreviewRequest>) -> Response {
    logger::debug(
        LogTag::Tools,
        &format!(
            "Multi-sell preview: token={}, wallets={:?}, {}%",
            &request.token_mint, request.wallet_ids, request.sell_percentage
        ),
    );

    // Validate token mint
    if wallets::validate_address(&request.token_mint).is_err() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_MINT",
            "Invalid token mint address",
            None,
        );
    }

    // Get wallets with their balances
    let all_wallets = match wallets::list_active_wallets().await {
        Ok(w) => w,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "WALLET_ERROR",
                "Failed to get wallets",
                Some(&e),
            );
        }
    };

    // Filter to secondary wallets
    let secondary_wallets: Vec<_> = all_wallets
        .into_iter()
        .filter(|w| {
            if w.role != wallets::WalletRole::Secondary {
                return false;
            }
            // Filter by specific IDs if provided
            if let Some(ref ids) = request.wallet_ids {
                return ids.contains(&w.id);
            }
            true
        })
        .collect();

    if secondary_wallets.is_empty() {
        return success_response(MultiSellPreviewResponse {
            token_symbol: None,
            wallets_with_balance: 0,
            total_token_balance: 0.0,
            token_to_sell: 0.0,
            estimated_sol: None,
            can_proceed: false,
            warning: Some("No secondary wallets found".to_owned()),
            wallets: vec![],
        });
    }

    // Get token balances for each wallet
    let rpc = get_rpc_client();
    let mut wallets_with_balance = Vec::new();
    let mut total_token_balance = 0.0;

    // Fetch token decimals once for display conversion
    let token_decimals = decimals::get(crate::chains::ChainId::Solana, &request.token_mint)
        .await
        .unwrap_or(9);
    let divisor = 10f64.powi(token_decimals as i32);

    for wallet in secondary_wallets {
        // Get token balance (returns raw amount in smallest units)
        let token_balance_raw = match rpc
            .get_token_balance(&wallet.address, &request.token_mint)
            .await
        {
            Ok(amount) => amount,
            Err(_) => 0,
        };

        // Convert to UI amount using actual decimals
        let token_balance = token_balance_raw as f64 / divisor;

        if token_balance > 0.0 {
            total_token_balance += token_balance;

            // Get SOL balance
            let sol_balance = rpc
                .get_sol_balance(&wallet.address)
                .await
                .unwrap_or_default();

            wallets_with_balance.push(WalletTokenBalanceResponse {
                wallet_id: wallet.id,
                wallet_address: wallet.address.clone(),
                wallet_name: wallet.name.clone(),
                sol_balance,
                token_balance,
                needs_sol_topup: sol_balance < 0.01,
            });
        }
    }

    let token_to_sell = total_token_balance * (request.sell_percentage / 100.0);
    let can_proceed = !wallets_with_balance.is_empty();
    let warning = if !can_proceed {
        Some("No wallets have token balance".to_owned())
    } else {
        None
    };

    success_response(MultiSellPreviewResponse {
        token_symbol: None, // Could fetch from tokens DB
        wallets_with_balance: wallets_with_balance.len(),
        total_token_balance,
        token_to_sell,
        estimated_sol: None, // Would need price oracle
        can_proceed,
        warning,
        wallets: wallets_with_balance,
    })
}

/// Start multi-sell operation
pub async fn start_multi_sell(Json(request): Json<MultiSellStartRequest>) -> Response {
    logger::info(
        LogTag::Tools,
        &format!(
            "Starting multi-sell: token={}, {}%, consolidate={}",
            &request.token_mint, request.sell_percentage, request.consolidate_after
        ),
    );

    // Check for concurrent sessions
    if has_active_multi_wallet_session().await {
        return error_response(
            StatusCode::CONFLICT,
            "SESSION_ACTIVE",
            "Another multi-wallet operation is already in progress",
            None,
        );
    }

    // Cleanup old sessions
    cleanup_old_sessions().await;

    // Validate token mint
    if wallets::validate_address(&request.token_mint).is_err() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_MINT",
            "Invalid token mint address",
            None,
        );
    }

    // Build delay config
    let delay = if let Some(max_ms) = request.delay_max_ms {
        DelayConfig::Random {
            min_ms: request.delay_ms,
            max_ms,
        }
    } else {
        DelayConfig::Fixed {
            delay_ms: request.delay_ms,
        }
    };

    // Generate session ID and abort flag first
    let session_id = uuid::Uuid::new_v4().to_string();
    let abort_flag = Arc::new(AtomicBool::new(false));
    let token_mint = request.token_mint.clone();

    // Build config with abort flag
    let config = MultiSellConfig {
        token_mint: request.token_mint.clone(),
        wallet_ids: request.wallet_ids.clone(),
        sell_percentage: request.sell_percentage,
        min_sol_for_fee: request.min_sol_for_fee,
        auto_topup: request.auto_topup,
        delay,
        concurrency: request.concurrency,
        slippage_bps: request.slippage_bps,
        consolidate_after: request.consolidate_after,
        close_atas_after: request.close_atas_after,
        router: request.router.clone(),
        abort_flag: Some(abort_flag.clone()),
    };

    // Validate config
    if let Err(e) = config.validate() {
        return error_response(StatusCode::BAD_REQUEST, "INVALID_CONFIG", &e, None);
    }

    // Create session entry
    {
        let mut sessions = MULTI_WALLET_SESSIONS.write().await;
        sessions.insert(
            session_id.clone(),
            MultiWalletSession {
                result: SessionResult::new(session_id.clone()),
                status: SessionStatus::Pending,
                abort_flag: abort_flag.clone(),
                operation_type: "multi_sell".to_owned(),
                token_mint: token_mint.clone(),
                started_at: chrono::Utc::now(),
            },
        );
    }

    // Spawn background task
    let session_id_clone = session_id.clone();
    tokio::spawn(async move {
        // Update status to executing
        {
            let mut sessions = MULTI_WALLET_SESSIONS.write().await;
            if let Some(session) = sessions.get_mut(&session_id_clone) {
                session.status = SessionStatus::Executing;
            }
        }

        // Execute multi-sell
        let result = execute_multi_sell(config).await;

        // Update session with result
        {
            let mut sessions = MULTI_WALLET_SESSIONS.write().await;
            if let Some(session) = sessions.get_mut(&session_id_clone) {
                match result {
                    Ok(res) => {
                        session.result = res;
                        session.status = SessionStatus::Completed;
                    }
                    Err(e) => {
                        session.result.error = Some(e.clone());
                        session.result.success = false;
                        session.status = SessionStatus::Failed;
                        logger::error(
                            LogTag::Tools,
                            &format!(
                                "Multi-sell session {} failed: {}",
                                &session_id_clone[..8],
                                e
                            ),
                        );
                    }
                }
            }
        }
    });

    success_response(SessionStartResponse {
        session_id,
        message: "Multi-sell session started".to_owned(),
    })
}

/// Get multi-sell session status
pub async fn get_multi_sell_status(Path(id): Path<String>) -> Response {
    get_session_status(&id, "multi_sell").await
}

/// Abort multi-sell session
pub async fn abort_multi_sell(Path(id): Path<String>) -> Response {
    super::session::abort_session(&id).await
}
