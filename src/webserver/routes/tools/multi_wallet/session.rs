//! Multi-wallet session management
//!
//! Provides global session state tracking and management utilities for multi-wallet operations.

use axum::response::Response;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use tokio::sync::RwLock;

use crate::logger::{self, LogTag};
use crate::tools::multi_wallet::{SessionResult, SessionStatus};
use crate::webserver::utils::{error_response, success_response};

use super::super::types::*;

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

// =============================================================================
// Session Management Functions
// =============================================================================

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
// Session Helper Functions
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
