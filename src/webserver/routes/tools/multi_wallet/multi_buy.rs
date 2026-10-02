// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Multi-buy operation handlers
//!
//! Handles preview, start, status, and abort for multi-buy operations.

use axum::{extract::Path, response::Response, Json};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::actions::ActionFailure;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::i18n::{ids, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::tools::multi_wallet::{
    execute_multi_buy, MultiBuyConfig, SessionProgress, SessionResult, SessionStatus,
};
use crate::tools::DelayConfig;
use crate::wallets;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;
use axum::response::IntoResponse as _;

use super::super::types::*;
use super::config_error::invalid_config;
use super::session::{
    cleanup_old_sessions, get_session_status, has_active_multi_wallet_session,
    spawn_progress_drain, validate_router_choice, MultiWalletSession, MULTI_WALLET_SESSIONS,
};

// =============================================================================
// Multi-Buy Handlers
// =============================================================================

/// SOL amounts keep four decimals and travel as text so they are never regrouped.
fn insufficient_balance_warning(needed: f64, have: f64) -> UiText {
    UiText::new(ids::TOOLS_MULTI_BUY_WARNING_INSUFFICIENT)
        .arg("needed", UiArg::Text(format!("{needed:.4}")))
        .arg("have", UiArg::Text(format!("{have:.4}")))
}

fn over_limit_warning(needed: f64, limit: f64) -> UiText {
    UiText::new(ids::TOOLS_MULTI_BUY_WARNING_OVER_LIMIT)
        .arg("needed", UiArg::Text(format!("{needed:.4}")))
        .arg("limit", UiArg::Text(format!("{limit:.4}")))
}

/// Preview multi-buy operation
pub async fn preview_multi_buy(Json(request): Json<MultiBuyPreviewRequest>) -> Response {
    logger::debug(
        LogTag::Tools,
        &format!(
            "Multi-buy preview: token={}, wallets={}, amount={}-{} SOL",
            &request.token_mint,
            request.wallet_count,
            request.min_amount_sol,
            request.max_amount_sol
        ),
    );

    // Validate token mint
    if crate::chains::adapter()
        .validate_address(&request.token_mint)
        .is_err()
    {
        return ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_TOOLS_MINT_INVALID)
            .into_response();
    }

    // Get main wallet balance
    let main_wallet = match wallets::get_main_wallet().await {
        Ok(Some(w)) => w,
        Ok(None) => {
            return ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_WALLETS_NO_MAIN_WALLET,
            )
            .into_response();
        }
        Err(e) => {
            return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_WALLETS_MAIN_GET_FAILED)
                .details(e.to_string())
                .into_response();
        }
    };

    // Get main wallet SOL balance
    let rpc = get_rpc_client();
    let main_balance = match rpc.get_sol_balance(&main_wallet.address).await {
        Ok(sol) => sol,
        Err(e) => {
            return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_TOOLS_BALANCE_FAILED)
                .details(e.to_string())
                .into_response();
        }
    };

    // Get existing secondary wallets
    let existing_wallets = match wallets::list_active_wallets().await {
        Ok(w) => w
            .into_iter()
            .filter(|w| w.role == wallets::WalletRole::Secondary)
            .collect::<Vec<_>>(),
        Err(e) => {
            return ApiError::new(ApiErrorCode::Internal, ids::ERRORS_TOOLS_WALLETS_GET_FAILED)
                .details(e.to_string())
                .into_response();
        }
    };

    let existing_count = existing_wallets.len();
    let wallets_to_create = if request.wallet_count > existing_count {
        request.wallet_count - existing_count
    } else {
        0
    };

    // Calculate SOL needed
    let avg_buy = (request.min_amount_sol + request.max_amount_sol) / 2.0;
    let per_wallet_sol = avg_buy + request.sol_buffer;
    let total_sol_needed = per_wallet_sol * request.wallet_count as f64;

    // Check if we can proceed
    let can_proceed = main_balance >= total_sol_needed;
    let warning = if !can_proceed {
        Some(insufficient_balance_warning(total_sol_needed, main_balance))
    } else if let Some(limit) = request.total_sol_limit {
        if total_sol_needed > limit {
            Some(over_limit_warning(total_sol_needed, limit))
        } else {
            None
        }
    } else {
        None
    };

    // Build wallet plans (preview) - fetch balances for each wallet
    let mut wallet_plans = Vec::new();
    for w in existing_wallets.iter().take(request.wallet_count) {
        let sol_balance = rpc.get_sol_balance(&w.address).await.unwrap_or_default();
        let needs_funding = sol_balance < per_wallet_sol;
        let funding_amount = if needs_funding {
            per_wallet_sol - sol_balance
        } else {
            0.0
        };
        wallet_plans.push(WalletPlanResponse {
            wallet_id: w.id,
            wallet_address: w.address.clone(),
            wallet_name: w.name.clone(),
            current_sol_balance: sol_balance,
            planned_buy_amount: avg_buy,
            needs_funding,
            funding_amount,
        });
    }

    success_response(MultiBuyPreviewResponse {
        wallets_to_create,
        existing_wallets: existing_count,
        total_sol_needed,
        per_wallet_sol,
        main_wallet_balance: main_balance,
        can_proceed,
        warning,
        wallet_plans,
    })
}

/// Start multi-buy operation
pub async fn start_multi_buy(Json(request): Json<MultiBuyStartRequest>) -> Response {
    logger::info(
        LogTag::Tools,
        &format!(
            "Starting multi-buy: token={}, wallets={}, amount={}-{} SOL",
            &request.token_mint,
            request.wallet_count,
            request.min_amount_sol,
            request.max_amount_sol
        ),
    );

    // Check for concurrent sessions
    if has_active_multi_wallet_session().await {
        return ApiError::new(ApiErrorCode::Conflict, ids::ERRORS_TOOLS_SESSION_ACTIVE)
            .into_response();
    }

    // Cleanup old sessions
    cleanup_old_sessions().await;

    // Validate token mint
    if crate::chains::adapter()
        .validate_address(&request.token_mint)
        .is_err()
    {
        return ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_TOOLS_MINT_INVALID)
            .into_response();
    }

    if let Err(response) = validate_router_choice(request.router.as_deref()) {
        return response;
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

    // The caller owns the session identity, and the sink carries each
    // per-wallet outcome back while the run is still going.
    let (progress, completed) = SessionProgress::channel(session_id.clone());

    // Build config with abort flag
    let config = MultiBuyConfig {
        token_mint: request.token_mint.clone(),
        wallet_count: request.wallet_count,
        total_sol_limit: request.total_sol_limit,
        min_amount_sol: request.min_amount_sol,
        max_amount_sol: request.max_amount_sol,
        sol_buffer: request.sol_buffer,
        delay,
        concurrency: request.concurrency,
        slippage_bps: request.slippage_bps,
        router: request.router.clone(),
        abort_flag: Some(abort_flag.clone()),
        progress: Some(progress),
    };

    // Validate config
    if let Err(e) = config.validate() {
        return invalid_config(&e).into_response();
    }

    // Create session entry. The planned wallet count is seeded now so progress
    // has a real denominator before any operation has finished; `finalize` later
    // replaces it with what actually ran.
    {
        let mut result = SessionResult::new(session_id.clone());
        result.total_wallets = request.wallet_count;
        let mut sessions = MULTI_WALLET_SESSIONS.write().await;
        sessions.insert(
            session_id.clone(),
            MultiWalletSession {
                result,
                status: SessionStatus::Pending,
                abort_flag: abort_flag.clone(),
                operation_type: "multi_buy".to_owned(),
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

        // Mirror finished operations into the session while the run is going.
        let drain = spawn_progress_drain(session_id_clone.clone(), completed);

        // Execute multi-buy
        let result = execute_multi_buy(config).await;
        // The executor has dropped its sink, so the drain is finishing its last
        // operation; wait for it before replacing the session's result.
        let _ = drain.await;

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
                        session.result.error = Some(ActionFailure::with_details(
                            ids::TOOLS_MULTI_BUY_SESSION_FAILED,
                            e.to_string(),
                        ));
                        session.result.success = false;
                        session.status = SessionStatus::Failed;
                        logger::error(
                            LogTag::Tools,
                            &format!("Multi-buy session {} failed: {}", &session_id_clone[..8], e),
                        );
                    }
                }
            }
        }
    });

    success_response(SessionStartResponse { session_id })
}

/// Get multi-buy session status
pub async fn get_multi_buy_status(Path(id): Path<String>) -> Response {
    get_session_status(&id, "multi_buy").await
}

/// Abort multi-buy session
pub async fn abort_multi_buy(Path(id): Path<String>) -> Response {
    super::session::abort_session(&id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_warnings_render_their_english() {
        assert_eq!(
            insufficient_balance_warning(0.5, 0.25).render_source_plain(),
            "Insufficient balance. Need 0.5000 SOL, have 0.2500 SOL"
        );
        assert_eq!(
            over_limit_warning(0.5, 0.25).render_source_plain(),
            "Total SOL needed (0.5000) exceeds limit (0.2500)"
        );
    }
}
