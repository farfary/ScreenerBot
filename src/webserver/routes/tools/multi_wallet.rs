//! Multi-wallet operation handlers

use axum::extract::Path;
use axum::response::Response;
use axum::Json;
use std::sync::LazyLock;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::tokens::decimals;
use crate::tools::multi_wallet::{
    execute_consolidation, execute_multi_buy, execute_multi_sell, ConsolidateConfig,
    MultiBuyConfig, MultiSellConfig, SessionResult, SessionStatus,
};
use crate::tools::DelayConfig;
use crate::wallets;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// Multi-Wallet Global State
// =============================================================================

/// Session tracking for multi-wallet operations
pub struct MultiWalletSession {
    /// Session result (updated as operations progress)
    pub result: SessionResult,
    /// Current status
    pub status: SessionStatus,
    /// Abort flag for the running session
    pub abort_flag: Arc<AtomicBool>,
    /// Operation type for display
    pub operation_type: String,
    /// Token mint being traded
    pub token_mint: String,
    /// Started timestamp
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Global multi-wallet sessions state
pub static MULTI_WALLET_SESSIONS: LazyLock<Arc<RwLock<HashMap<String, MultiWalletSession>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Session cleanup interval (1 hour in seconds)
const SESSION_CLEANUP_INTERVAL_SECS: i64 = 3600;

/// Check if there's an active (non-completed) multi-wallet session
pub async fn has_active_multi_wallet_session() -> bool {
    let sessions = MULTI_WALLET_SESSIONS.read().await;
    sessions.values().any(|s| {
        matches!(
            s.status,
            SessionStatus::Pending
                | SessionStatus::Funding
                | SessionStatus::Executing
                | SessionStatus::Consolidating
        )
    })
}

/// Cleanup old completed sessions (older than 1 hour)
pub async fn cleanup_old_sessions() {
    let now = chrono::Utc::now();
    let mut sessions = MULTI_WALLET_SESSIONS.write().await;

    let old_session_ids: Vec<String> = sessions
        .iter()
        .filter(|(_, s)| {
            // Only clean up completed sessions older than 1 hour
            let is_complete = matches!(
                s.status,
                SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Aborted
            );
            let age_secs = (now - s.started_at).num_seconds();
            is_complete && age_secs > SESSION_CLEANUP_INTERVAL_SECS
        })
        .map(|(id, _)| id.clone())
        .collect();

    for id in old_session_ids {
        sessions.remove(&id);
        logger::debug(
            LogTag::Tools,
            &format!("Cleaned up old multi-wallet session: {}", &id[..8]),
        );
    }
}

// =============================================================================
// Multi-Wallet Handlers
// =============================================================================

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
    if Pubkey::from_str(&request.token_mint).is_err() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_MINT",
            "Invalid token mint address",
            None,
        );
    }

    // Get main wallet balance
    let main_wallet = match wallets::get_main_wallet().await {
        Ok(Some(w)) => w,
        Ok(None) => {
            return error_response(
                axum::http::StatusCode::BAD_REQUEST,
                "NO_MAIN_WALLET",
                "No main wallet configured",
                None,
            );
        }
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "WALLET_ERROR",
                "Failed to get main wallet",
                Some(&e),
            );
        }
    };

    // Get main wallet SOL balance
    let rpc = get_rpc_client();
    let main_balance = match rpc.get_sol_balance(&main_wallet.address).await {
        Ok(sol) => sol,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "RPC_ERROR",
                "Failed to get wallet balance",
                Some(&e),
            );
        }
    };

    // Get existing secondary wallets
    let existing_wallets = match wallets::get_wallets_with_keys().await {
        Ok(w) => w
            .into_iter()
            .filter(|w| w.wallet.role == wallets::WalletRole::Secondary && w.wallet.is_active)
            .collect::<Vec<_>>(),
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "WALLET_ERROR",
                "Failed to get wallets",
                Some(&e),
            );
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
        Some(format!(
            "Insufficient balance. Need {:.4} SOL, have {:.4} SOL",
            total_sol_needed, main_balance
        ))
    } else if let Some(limit) = request.total_sol_limit {
        if total_sol_needed > limit {
            Some(format!(
                "Total SOL needed ({:.4}) exceeds limit ({:.4})",
                total_sol_needed, limit
            ))
        } else {
            None
        }
    } else {
        None
    };

    // Build wallet plans (preview) - fetch balances for each wallet
    let mut wallet_plans = Vec::new();
    for w in existing_wallets.iter().take(request.wallet_count) {
        let sol_balance = rpc.get_sol_balance(&w.wallet.address).await.unwrap_or(0.0);
        let needs_funding = sol_balance < per_wallet_sol;
        let funding_amount = if needs_funding {
            per_wallet_sol - sol_balance
        } else {
            0.0
        };
        wallet_plans.push(WalletPlanResponse {
            wallet_id: w.wallet.id,
            wallet_address: w.wallet.address.clone(),
            wallet_name: w.wallet.name.clone(),
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
        return error_response(
            axum::http::StatusCode::CONFLICT,
            "SESSION_ACTIVE",
            "Another multi-wallet operation is already in progress",
            None,
        );
    }

    // Cleanup old sessions
    cleanup_old_sessions().await;

    // Validate token mint
    if Pubkey::from_str(&request.token_mint).is_err() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
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
    let mut config = MultiBuyConfig {
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
    };

    // Validate config
    if let Err(e) = config.validate() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_CONFIG",
            &e,
            None,
        );
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
                operation_type: "multi_buy".to_string(),
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

        // Execute multi-buy
        let result = execute_multi_buy(config).await;

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
                            &format!("Multi-buy session {} failed: {}", &session_id_clone[..8], e),
                        );
                    }
                }
            }
        }
    });

    success_response(SessionStartResponse {
        session_id,
        message: "Multi-buy session started".to_string(),
    })
}

/// Get multi-buy session status
pub async fn get_multi_buy_status(Path(id): Path<String>) -> Response {
    get_session_status(&id, "multi_buy").await
}

/// Abort multi-buy session
pub async fn abort_multi_buy(Path(id): Path<String>) -> Response {
    abort_session(&id).await
}

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
    if Pubkey::from_str(&request.token_mint).is_err() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_MINT",
            "Invalid token mint address",
            None,
        );
    }

    // Get wallets with their balances
    let all_wallets = match wallets::get_wallets_with_keys().await {
        Ok(w) => w,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
            if w.wallet.role != wallets::WalletRole::Secondary || !w.wallet.is_active {
                return false;
            }
            // Filter by specific IDs if provided
            if let Some(ref ids) = request.wallet_ids {
                return ids.contains(&w.wallet.id);
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
            warning: Some("No secondary wallets found".to_string()),
            wallets: vec![],
        });
    }

    // Get token balances for each wallet
    let rpc = get_rpc_client();
    let mut wallets_with_balance = Vec::new();
    let mut total_token_balance = 0.0;

    // Fetch token decimals once for display conversion
    let token_decimals = decimals::get(&request.token_mint).await.unwrap_or(9);
    let divisor = 10f64.powi(token_decimals as i32);

    for wallet in secondary_wallets {
        // Get token balance (returns raw amount in smallest units)
        let token_balance_raw = match rpc
            .get_token_balance(&wallet.wallet.address, &request.token_mint)
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
                .get_sol_balance(&wallet.wallet.address)
                .await
                .unwrap_or(0.0);

            wallets_with_balance.push(WalletTokenBalanceResponse {
                wallet_id: wallet.wallet.id,
                wallet_address: wallet.wallet.address.clone(),
                wallet_name: wallet.wallet.name.clone(),
                sol_balance,
                token_balance,
                needs_sol_topup: sol_balance < 0.01,
            });
        }
    }

    let token_to_sell = total_token_balance * (request.sell_percentage / 100.0);
    let can_proceed = !wallets_with_balance.is_empty();
    let warning = if !can_proceed {
        Some("No wallets have token balance".to_string())
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
            axum::http::StatusCode::CONFLICT,
            "SESSION_ACTIVE",
            "Another multi-wallet operation is already in progress",
            None,
        );
    }

    // Cleanup old sessions
    cleanup_old_sessions().await;

    // Validate token mint
    if Pubkey::from_str(&request.token_mint).is_err() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
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
    let mut config = MultiSellConfig {
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
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_CONFIG",
            &e,
            None,
        );
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
                operation_type: "multi_sell".to_string(),
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
        message: "Multi-sell session started".to_string(),
    })
}

/// Get multi-sell session status
pub async fn get_multi_sell_status(Path(id): Path<String>) -> Response {
    get_session_status(&id, "multi_sell").await
}

/// Abort multi-sell session
pub async fn abort_multi_sell(Path(id): Path<String>) -> Response {
    abort_session(&id).await
}

/// Get wallets summary
pub async fn get_wallets_summary() -> Response {
    // Get all wallets
    let all_wallets = match wallets::list_active_wallets().await {
        Ok(w) => w,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "WALLET_ERROR",
                "Failed to get wallets",
                Some(&e),
            );
        }
    };

    let rpc = get_rpc_client();
    let mut wallets_info = Vec::new();
    let mut main_wallet_info = None;
    let mut total_sol = 0.0;
    let mut secondary_count = 0;

    for wallet in &all_wallets {
        // Get SOL balance via RPC
        let sol_balance = rpc.get_sol_balance(&wallet.address).await.unwrap_or(0.0);

        total_sol += sol_balance;

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
        total_sol,
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
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_CONFIG",
            &e,
            None,
        );
    }

    // Execute consolidation
    match execute_consolidation(config).await {
        Ok(result) => {
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
                message: format!(
                    "Consolidated {} wallets, recovered {:.6} SOL",
                    result.successful_ops, result.total_sol_recovered
                ),
            })
        }
        Err(e) => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "CONSOLIDATION_FAILED",
            "Failed to consolidate wallets",
            Some(&e),
        ),
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

    match execute_consolidation(config).await {
        Ok(result) => {
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
                message: format!(
                    "Cleaned up ATAs on {} wallets, reclaimed {:.6} SOL",
                    result.successful_ops, result.total_sol_recovered
                ),
            })
        }
        Err(e) => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "CLEANUP_FAILED",
            "Failed to cleanup ATAs",
            Some(&e),
        ),
    }
}

/// Get multi-wallet sessions list
pub async fn get_multi_wallet_sessions() -> Response {
    let sessions = MULTI_WALLET_SESSIONS.read().await;

    let mut session_list: Vec<SessionSummaryResponse> = sessions
        .values()
        .map(|s| {
            let is_complete = matches!(
                s.status,
                SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Aborted
            );
            SessionSummaryResponse {
                session_id: s.result.session_id.clone(),
                operation_type: s.operation_type.clone(),
                token_mint: s.token_mint.clone(),
                status: s.status.to_string(),
                total_wallets: s.result.total_wallets,
                successful_ops: s.result.successful_ops,
                failed_ops: s.result.failed_ops,
                started_at: s.started_at.to_rfc3339(),
                is_complete,
            }
        })
        .collect();

    // Sort by started_at descending
    session_list.sort_by(|a, b| b.started_at.cmp(&a.started_at));

    // Limit to recent sessions
    let total = session_list.len();
    session_list.truncate(50);

    success_response(SessionsListResponse {
        sessions: session_list,
        total,
    })
}

// =============================================================================
// Multi-Wallet Helper Functions
// =============================================================================

/// Get session status by ID
pub async fn get_session_status(id: &str, expected_type: &str) -> Response {
    let sessions = MULTI_WALLET_SESSIONS.read().await;

    match sessions.get(id) {
        Some(session) => {
            if session.operation_type != expected_type {
                return error_response(
                    axum::http::StatusCode::BAD_REQUEST,
                    "TYPE_MISMATCH",
                    &format!(
                        "Session is {} not {}",
                        session.operation_type, expected_type
                    ),
                    None,
                );
            }

            let is_complete = matches!(
                session.status,
                SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Aborted
            );

            success_response(SessionStatusResponse {
                session_id: session.result.session_id.clone(),
                status: session.status.to_string(),
                operation_type: session.operation_type.clone(),
                token_mint: session.token_mint.clone(),
                total_wallets: session.result.total_wallets,
                successful_ops: session.result.successful_ops,
                failed_ops: session.result.failed_ops,
                total_sol_spent: session.result.total_sol_spent,
                total_sol_recovered: session.result.total_sol_recovered,
                started_at: session.started_at.to_rfc3339(),
                is_complete,
                error: session.result.error.clone(),
            })
        }
        None => error_response(
            axum::http::StatusCode::NOT_FOUND,
            "SESSION_NOT_FOUND",
            "Session not found",
            Some(id),
        ),
    }
}

/// Abort a session by ID
pub async fn abort_session(id: &str) -> Response {
    let mut sessions = MULTI_WALLET_SESSIONS.write().await;

    match sessions.get_mut(id) {
        Some(session) => {
            if matches!(
                session.status,
                SessionStatus::Completed | SessionStatus::Failed | SessionStatus::Aborted
            ) {
                return error_response(
                    axum::http::StatusCode::BAD_REQUEST,
                    "SESSION_COMPLETE",
                    "Session is already complete",
                    None,
                );
            }

            // Set abort flag
            session.abort_flag.store(true, Ordering::SeqCst);
            session.status = SessionStatus::Aborted;

            logger::info(
                LogTag::Tools,
                &format!("Aborted {} session {}", session.operation_type, &id[..8]),
            );

            success_response(serde_json::json!({
                "success": true,
                "message": "Session aborted"
            }))
        }
        None => error_response(
            axum::http::StatusCode::NOT_FOUND,
            "SESSION_NOT_FOUND",
            "Session not found",
            Some(id),
        ),
    }
}
