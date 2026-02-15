//! Volume aggregator handlers

use axum::response::Response;
use axum::Json;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::logger::{self, LogTag};
use crate::tools::{DelayConfig, ToolStatus, VolumeAggregator, VolumeConfig, VolumeSession};
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// Volume Aggregator Global State
// =============================================================================

/// Global state for active volume aggregator session
pub struct VolumeAggregatorState {
    /// Current session data (if running or recently completed)
    pub session: Option<VolumeSession>,
    /// Current status
    pub status: ToolStatus,
    /// Abort flag for the running session
    pub abort_flag: Option<Arc<AtomicBool>>,
}

impl Default for VolumeAggregatorState {
    fn default() -> Self {
        Self {
            session: None,
            status: ToolStatus::Ready,
            abort_flag: None,
        }
    }
}

/// Global volume aggregator state
pub static VOLUME_AGGREGATOR_STATE: Lazy<Arc<RwLock<VolumeAggregatorState>>> =
    Lazy::new(|| Arc::new(RwLock::new(VolumeAggregatorState::default())));

// =============================================================================
// Volume Aggregator Handlers
// =============================================================================

/// Start a volume aggregator session
pub async fn start_volume_aggregator(
    Json(request): Json<StartVolumeAggregatorRequest>,
) -> Response {
    // Parse token mint
    let token_mint = match Pubkey::from_str(&request.token_mint) {
        Ok(pk) => pk,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::BAD_REQUEST,
                "INVALID_MINT",
                "Invalid token mint address",
                Some(&e.to_string()),
            );
        }
    };

    // Check if already running
    {
        let state = VOLUME_AGGREGATOR_STATE.read().await;
        if state.status == ToolStatus::Running {
            return error_response(
                axum::http::StatusCode::CONFLICT,
                "ALREADY_RUNNING",
                "Volume aggregator is already running",
                None,
            );
        }
    }

    // Build config using new builder pattern
    use crate::tools::{DelayConfig, DistributionStrategy, SizingConfig};

    // Determine sizing config based on min/max
    let sizing_config = if request.min_amount_sol == request.max_amount_sol {
        SizingConfig::fixed(request.min_amount_sol)
    } else {
        SizingConfig::random(request.min_amount_sol, request.max_amount_sol)
    };

    // Determine delay config
    let delay_config = if let Some(max_ms) = request.delay_max_ms {
        DelayConfig::random(request.delay_between_ms, max_ms)
    } else {
        DelayConfig::fixed(request.delay_between_ms)
    };

    // Parse strategy
    let strategy = DistributionStrategy::from_db_value(&request.strategy);

    let config = VolumeConfig::new(token_mint, request.total_volume_sol)
        .with_num_wallets(request.num_wallets)
        .with_sizing(sizing_config)
        .with_delay(delay_config)
        .with_strategy(strategy);

    // Validate config
    if let Err(e) = config.validate() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_CONFIG",
            "Invalid configuration",
            Some(&e),
        );
    }

    // Create aggregator and prepare
    let mut aggregator = VolumeAggregator::new(config);

    if let Err(e) = aggregator.prepare().await {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "PREPARE_FAILED",
            "Failed to prepare volume aggregator",
            Some(&e),
        );
    }

    // Store abort flag
    let abort_flag = aggregator.get_abort_flag();

    // Update state to running
    {
        let mut state = VOLUME_AGGREGATOR_STATE.write().await;
        state.status = ToolStatus::Running;
        state.abort_flag = Some(abort_flag);
        state.session = None;
    }

    logger::info(
        LogTag::Tools,
        &format!(
            "Starting volume aggregator for token {} with {} SOL target volume",
            request.token_mint, request.total_volume_sol
        ),
    );

    // Mark tool as started (pauses background token updates)
    crate::global::tool_started();

    // Spawn execution task
    tokio::spawn(async move {
        let result = aggregator.execute().await;

        // Update state with result
        let mut state = VOLUME_AGGREGATOR_STATE.write().await;
        match result {
            Ok(session) => {
                use crate::tools::SessionStatus;
                state.status = match session.status {
                    SessionStatus::Completed => ToolStatus::Completed,
                    SessionStatus::Aborted => ToolStatus::Aborted,
                    SessionStatus::Failed => ToolStatus::Failed,
                    _ => ToolStatus::Completed,
                };
                state.session = Some(session);
            }
            Err(e) => {
                logger::error(LogTag::Tools, &format!("Volume aggregator failed: {}", e));
                state.status = ToolStatus::Failed;
            }
        }
        state.abort_flag = None;

        // Mark tool as finished (resumes background token updates)
        crate::global::tool_finished();
    });

    success_response(serde_json::json!({
        "message": "Volume aggregator started",
        "status": "running"
    }))
}

/// Get volume aggregator status
pub async fn get_volume_aggregator_status() -> Response {
    let state = VOLUME_AGGREGATOR_STATE.read().await;

    let session = state.session.as_ref().map(VolumeSessionResponse::from);

    success_response(VolumeAggregatorStatusResponse {
        status: state.status.to_string(),
        session,
    })
}

/// Stop a running volume aggregator session
pub async fn stop_volume_aggregator() -> Response {
    let mut state = VOLUME_AGGREGATOR_STATE.write().await;

    if state.status != ToolStatus::Running {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "NOT_RUNNING",
            "Volume aggregator is not running",
            None,
        );
    }

    // Set abort flag
    if let Some(abort_flag) = &state.abort_flag {
        abort_flag.store(true, Ordering::SeqCst);
        logger::info(LogTag::Tools, "Volume aggregator stop requested via API");

        success_response(serde_json::json!({
            "message": "Stop request sent",
            "status": "stopping"
        }))
    } else {
        error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "NO_ABORT_FLAG",
            "Cannot stop - no abort flag available",
            None,
        )
    }
}

/// Get volume aggregator session history
pub async fn get_volume_aggregator_sessions() -> Response {
    use crate::tools::database::{get_recent_va_sessions, get_va_sessions_analytics};

    // Fetch recent sessions (limit 50)
    let sessions = match get_recent_va_sessions(50) {
        Ok(rows) => rows
            .into_iter()
            .map(VaSessionSummary::from)
            .collect::<Vec<_>>(),
        Err(e) => {
            logger::error(LogTag::Tools, &format!("Failed to get VA sessions: {}", e));
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "Failed to fetch session history",
                Some(&e),
            );
        }
    };

    // Fetch analytics
    let analytics = match get_va_sessions_analytics() {
        Ok(summary) => VaAnalyticsSummaryResponse {
            total_sessions: summary.total_sessions,
            total_volume_sol: summary.total_volume_sol,
            avg_success_rate: summary.avg_success_rate,
            completed_sessions: summary.completed_sessions,
            failed_sessions: summary.failed_sessions,
            aborted_sessions: summary.aborted_sessions,
        },
        Err(e) => {
            logger::warning(LogTag::Tools, &format!("Failed to get VA analytics: {}", e));
            VaAnalyticsSummaryResponse {
                total_sessions: 0,
                total_volume_sol: 0.0,
                avg_success_rate: 0.0,
                completed_sessions: 0,
                failed_sessions: 0,
                aborted_sessions: 0,
            }
        }
    };

    let total = sessions.len();

    success_response(VaSessionHistoryResponse {
        sessions,
        analytics,
        total,
    })
}
