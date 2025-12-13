//! Tools API routes for wallet utilities, token operations, and trading tools

use axum::{response::Response, routing::{get, post}, Json, Router};
use chrono::DateTime;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ata_cleanup::{
    clear_failed_ata_cache, get_ata_cleanup_statistics, get_failed_ata_count,
    trigger_immediate_ata_cleanup,
};
use crate::logger::{self, LogTag};
use crate::tools::{ToolStatus, VolumeAggregator, VolumeConfig, VolumeSession};
use crate::utils::{get_all_token_accounts, get_wallet_address};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

// =============================================================================
// Volume Aggregator Global State
// =============================================================================

/// Global state for active volume aggregator session
struct VolumeAggregatorState {
    /// Current session data (if running or recently completed)
    session: Option<VolumeSession>,
    /// Current status
    status: ToolStatus,
    /// Abort flag for the running session
    abort_flag: Option<Arc<AtomicBool>>,
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
static VOLUME_AGGREGATOR_STATE: Lazy<Arc<RwLock<VolumeAggregatorState>>> =
    Lazy::new(|| Arc::new(RwLock::new(VolumeAggregatorState::default())));

// =============================================================================
// Response Types
// =============================================================================

/// ATA scan results for wallet cleanup tool
#[derive(Debug, Serialize)]
pub struct AtaScanResponse {
    pub total_atas: usize,
    pub empty_count: usize,
    pub non_empty_count: usize,
    pub failed_count: usize,
    pub reclaimable_sol: f64,
    pub empty_atas: Vec<EmptyAtaInfo>,
}

/// Information about a single empty ATA
#[derive(Debug, Serialize)]
pub struct EmptyAtaInfo {
    pub mint: String,
    pub ata_address: String,
    pub rent_lamports: u64,
}

/// ATA cleanup execution result
#[derive(Debug, Serialize)]
pub struct AtaCleanupResponse {
    pub closed_count: u32,
    pub failed_count: u32,
    pub rent_reclaimed: f64,
    pub signatures: Vec<String>,
}

/// Statistics for ATA cleanup history
#[derive(Debug, Serialize)]
pub struct AtaStatsResponse {
    pub total_closed: u32,
    pub total_rent_reclaimed: f64,
    pub failed_attempts: u32,
    pub cached_failures: usize,
    pub last_cleanup_time: Option<String>,
}

/// Keypair generation result
#[derive(Debug, Serialize)]
pub struct KeypairResponse {
    pub pubkey: String,
    pub secret: String,
}

/// Request for generating multiple keypairs
#[derive(Debug, Deserialize)]
pub struct GenerateKeypairsRequest {
    #[serde(default = "default_keypair_count")]
    pub count: usize,
}

fn default_keypair_count() -> usize {
    1
}

// =============================================================================
// Volume Aggregator Request/Response Types
// =============================================================================

/// Request to start a volume aggregator session
#[derive(Debug, Deserialize)]
pub struct StartVolumeAggregatorRequest {
    /// Token mint address to generate volume for
    pub token_mint: String,
    /// Total SOL volume to generate
    pub total_volume_sol: f64,
    /// Number of wallets to use (max)
    #[serde(default = "default_num_wallets")]
    pub num_wallets: usize,
    /// Minimum SOL per transaction
    #[serde(default = "default_min_amount")]
    pub min_amount_sol: f64,
    /// Maximum SOL per transaction
    #[serde(default = "default_max_amount")]
    pub max_amount_sol: f64,
    /// Delay between transactions in milliseconds
    #[serde(default = "default_delay")]
    pub delay_between_ms: u64,
    /// Maximum delay (for random mode)
    pub delay_max_ms: Option<u64>,
    /// Distribution strategy: "round_robin", "random", or "burst:N"
    #[serde(default = "default_strategy")]
    pub strategy: String,
}

fn default_num_wallets() -> usize {
    5
}
fn default_min_amount() -> f64 {
    0.05
}
fn default_max_amount() -> f64 {
    0.2
}
fn default_delay() -> u64 {
    3000
}
fn default_strategy() -> String {
    "round_robin".to_string()
}

/// Response for volume aggregator status
#[derive(Debug, Serialize)]
pub struct VolumeAggregatorStatusResponse {
    /// Current status
    pub status: String,
    /// Session data if running or completed
    pub session: Option<VolumeSessionResponse>,
}

/// Serialized volume session for API response
#[derive(Debug, Serialize)]
pub struct VolumeSessionResponse {
    pub session_id: String,
    pub token_mint: String,
    pub target_volume_sol: f64,
    pub actual_volume_sol: f64,
    pub successful_buys: usize,
    pub successful_sells: usize,
    pub failed_count: usize,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub status: String,
    pub success_rate: f64,
    pub duration_secs: i64,
    pub progress_pct: f64,
    pub transaction_count: usize,
}

impl From<&VolumeSession> for VolumeSessionResponse {
    fn from(s: &VolumeSession) -> Self {
        Self {
            session_id: s.session_id.clone(),
            token_mint: s.token_mint.clone(),
            target_volume_sol: s.target_volume_sol,
            actual_volume_sol: s.actual_volume_sol,
            successful_buys: s.successful_buys,
            successful_sells: s.successful_sells,
            failed_count: s.failed_count,
            started_at: s.started_at.to_rfc3339(),
            ended_at: s.ended_at.map(|t| t.to_rfc3339()),
            status: format!("{:?}", s.status).to_lowercase(),
            success_rate: s.success_rate(),
            duration_secs: s.duration_secs(),
            progress_pct: s.progress_pct(),
            transaction_count: s.transactions.len(),
        }
    }
}

// =============================================================================
// Volume Aggregator Session History Types
// =============================================================================

/// Response for VA session history
#[derive(Debug, Serialize)]
pub struct VaSessionHistoryResponse {
    pub sessions: Vec<VaSessionSummary>,
    pub analytics: VaAnalyticsSummaryResponse,
    pub total: usize,
}

/// Summary of a single VA session for history view
#[derive(Debug, Serialize)]
pub struct VaSessionSummary {
    pub session_id: String,
    pub token_mint: String,
    pub target_volume_sol: f64,
    pub actual_volume_sol: f64,
    pub successful_buys: i32,
    pub successful_sells: i32,
    pub failed_count: i32,
    pub success_rate: f64,
    pub status: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_secs: i64,
    pub created_at: String,
    pub can_resume: bool,
}

/// Analytics summary response
#[derive(Debug, Serialize)]
pub struct VaAnalyticsSummaryResponse {
    pub total_sessions: i64,
    pub total_volume_sol: f64,
    pub avg_success_rate: f64,
    pub completed_sessions: i64,
    pub failed_sessions: i64,
    pub aborted_sessions: i64,
}

// =============================================================================
// Tool Favorites Types
// =============================================================================

/// Response for tool favorites list
#[derive(Debug, Serialize)]
pub struct ToolFavoritesListResponse {
    pub favorites: Vec<crate::tools::database::ToolFavoriteRow>,
    pub total: usize,
}

/// Request to add a tool favorite
#[derive(Debug, Deserialize)]
pub struct AddToolFavoriteRequest {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub tool_type: String,
    pub config_json: Option<String>,
    pub label: Option<String>,
    pub notes: Option<String>,
}

/// Request to update a tool favorite
#[derive(Debug, Deserialize)]
pub struct UpdateToolFavoriteRequest {
    pub config_json: Option<String>,
    pub label: Option<String>,
    pub notes: Option<String>,
}

impl From<crate::tools::database::VaSessionRow> for VaSessionSummary {
    fn from(row: crate::tools::database::VaSessionRow) -> Self {
        let total_ops = row.successful_buys + row.successful_sells + row.failed_count;
        let success_rate = if total_ops > 0 {
            (row.successful_buys + row.successful_sells) as f64 / total_ops as f64 * 100.0
        } else {
            0.0
        };

        // Calculate duration
        let duration_secs = match (&row.started_at, &row.ended_at) {
            (Some(start), Some(end)) => {
                if let (Ok(s), Ok(e)) = (
                    DateTime::parse_from_rfc3339(start),
                    DateTime::parse_from_rfc3339(end),
                ) {
                    (e - s).num_seconds()
                } else {
                    0
                }
            }
            _ => 0,
        };

        // Can resume if not completed and has remaining volume
        let can_resume = matches!(row.status.as_str(), "failed" | "aborted")
            && row.actual_volume_sol < row.target_volume_sol;

        Self {
            session_id: row.session_id,
            token_mint: row.token_mint,
            target_volume_sol: row.target_volume_sol,
            actual_volume_sol: row.actual_volume_sol,
            successful_buys: row.successful_buys,
            successful_sells: row.successful_sells,
            failed_count: row.failed_count,
            success_rate,
            status: row.status,
            started_at: row.started_at,
            ended_at: row.ended_at,
            duration_secs,
            created_at: row.created_at,
            can_resume,
        }
    }
}

// =============================================================================
// Routes
// =============================================================================

/// Create tools routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Wallet Cleanup (ATA management)
        .route("/ata-scan", get(scan_atas))
        .route("/ata-stats", get(get_ata_stats))
        .route("/ata-cleanup", post(cleanup_atas))
        .route("/ata-clear-cache", post(clear_ata_cache))
        // Wallet Generator
        .route("/generate-keypair", post(generate_keypair))
        .route("/generate-keypairs", post(generate_keypairs))
        // Volume Aggregator
        .route("/volume-aggregator/start", post(start_volume_aggregator))
        .route("/volume-aggregator/status", get(get_volume_aggregator_status))
        .route("/volume-aggregator/stop", post(stop_volume_aggregator))
        .route("/volume-aggregator/sessions", get(get_volume_aggregator_sessions))
        // Tool Favorites
        .route("/favorites", get(get_favorites_list))
        .route("/favorites", post(add_favorite))
        .route("/favorites/:id", axum::routing::patch(update_favorite))
        .route("/favorites/:id", axum::routing::delete(delete_favorite))
        .route("/favorites/:id/use", post(mark_favorite_used))
}

// =============================================================================
// Wallet Cleanup Handlers
// =============================================================================

/// Scan wallet for empty ATAs without closing them
async fn scan_atas() -> Response {
    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get wallet address: {}", e));
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "WALLET_ERROR",
                "Failed to get wallet",
                Some(&e.to_string()),
            );
        }
    };

    // Get all token accounts
    let all_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to get token accounts: {}", e));
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "SCAN_ERROR",
                "Failed to scan accounts",
                Some(&e.to_string()),
            );
        }
    };

    // Separate empty and non-empty
    let empty_accounts: Vec<_> = all_accounts.iter().filter(|acc| acc.balance == 0).collect();
    let non_empty_count = all_accounts.len() - empty_accounts.len();
    let failed_count = get_failed_ata_count();

    // Estimate rent reclaimable (approximately 0.00203928 SOL per ATA)
    const ATA_RENT_LAMPORTS: u64 = 2_039_280;
    let reclaimable_sol = (empty_accounts.len() as f64 * ATA_RENT_LAMPORTS as f64) / 1_000_000_000.0;

    // Build empty ATA info list
    let empty_atas: Vec<EmptyAtaInfo> = empty_accounts
        .iter()
        .map(|acc| EmptyAtaInfo {
            mint: acc.mint.clone(),
            ata_address: acc.account.clone(),
            rent_lamports: ATA_RENT_LAMPORTS,
        })
        .collect();

    logger::info(
        LogTag::Wallet,
        &format!(
            "ATA scan complete: {} total, {} empty (reclaimable: {:.6} SOL), {} non-empty",
            all_accounts.len(),
            empty_accounts.len(),
            reclaimable_sol,
            non_empty_count
        ),
    );

    success_response(AtaScanResponse {
        total_atas: all_accounts.len(),
        empty_count: empty_accounts.len(),
        non_empty_count,
        failed_count,
        reclaimable_sol,
        empty_atas,
    })
}

/// Get ATA cleanup statistics
async fn get_ata_stats() -> Response {
    let stats = get_ata_cleanup_statistics();
    let cached_failures = get_failed_ata_count();

    success_response(AtaStatsResponse {
        total_closed: stats.total_closed,
        total_rent_reclaimed: stats.total_rent_reclaimed,
        failed_attempts: stats.failed_attempts,
        cached_failures,
        last_cleanup_time: stats.last_cleanup_time,
    })
}

/// Execute ATA cleanup (close empty ATAs)
async fn cleanup_atas() -> Response {
    logger::info(LogTag::Wallet, "Manual ATA cleanup requested via API");

    match trigger_immediate_ata_cleanup().await {
        Ok((closed_count, signatures)) => {
            // Get updated stats for rent reclaimed
            let stats = get_ata_cleanup_statistics();

            success_response(AtaCleanupResponse {
                closed_count,
                failed_count: stats.failed_attempts,
                rent_reclaimed: stats.total_rent_reclaimed,
                signatures,
            })
        }
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("ATA cleanup failed: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "CLEANUP_ERROR",
                "Cleanup failed",
                Some(&e.to_string()),
            )
        }
    }
}

/// Clear the failed ATA cache to retry previously failed closures
async fn clear_ata_cache() -> Response {
    match clear_failed_ata_cache().await {
        Ok(()) => {
            logger::info(LogTag::Wallet, "Failed ATA cache cleared via API");
            success_response(serde_json::json!({
                "message": "Failed ATA cache cleared - previously failed ATAs will be retried"
            }))
        }
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to clear ATA cache: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "CACHE_ERROR",
                "Failed to clear cache",
                Some(&e.to_string()),
            )
        }
    }
}

// =============================================================================
// Wallet Generator Handlers
// =============================================================================

/// Generate a single new Solana keypair
async fn generate_keypair() -> Response {
    use solana_sdk::signature::Keypair;

    let keypair = Keypair::new();
    let pubkey = keypair.pubkey().to_string();
    let secret = bs58::encode(keypair.to_bytes()).into_string();

    logger::info(
        LogTag::Wallet,
        &format!("Generated new keypair via API: {}", pubkey),
    );

    success_response(KeypairResponse { pubkey, secret })
}

/// Generate multiple new Solana keypairs
async fn generate_keypairs(Json(request): Json<GenerateKeypairsRequest>) -> Response {
    use solana_sdk::signature::Keypair;

    // Limit to reasonable number
    let count = request.count.min(10);

    let keypairs: Vec<KeypairResponse> = (0..count)
        .map(|_| {
            let keypair = Keypair::new();
            KeypairResponse {
                pubkey: keypair.pubkey().to_string(),
                secret: bs58::encode(keypair.to_bytes()).into_string(),
            }
        })
        .collect();

    logger::info(
        LogTag::Wallet,
        &format!("Generated {} new keypairs via API", keypairs.len()),
    );

    success_response(keypairs)
}

// =============================================================================
// Volume Aggregator Handlers
// =============================================================================

/// Start a volume aggregator session
async fn start_volume_aggregator(Json(request): Json<StartVolumeAggregatorRequest>) -> Response {
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
    use crate::tools::{DelayConfig, SizingConfig, DistributionStrategy};
    
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
async fn get_volume_aggregator_status() -> Response {
    let state = VOLUME_AGGREGATOR_STATE.read().await;
    
    let session = state.session.as_ref().map(VolumeSessionResponse::from);
    
    success_response(VolumeAggregatorStatusResponse {
        status: state.status.to_string(),
        session,
    })
}

/// Stop a running volume aggregator session
async fn stop_volume_aggregator() -> Response {
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
async fn get_volume_aggregator_sessions() -> Response {
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
            logger::warning(
                LogTag::Tools,
                &format!("Failed to get VA analytics: {}", e),
            );
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

// =============================================================================
// Tool Favorites Handlers
// =============================================================================

use axum::extract::Path;
use crate::tools::database::{
    get_tool_favorites, upsert_tool_favorite, remove_tool_favorite,
    update_tool_favorite as db_update_tool_favorite, increment_tool_favorite_use,
};

/// Get all tool favorites (optionally filtered by tool_type query param)
async fn get_favorites_list(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let tool_type = params.get("tool_type").map(|s| s.as_str());
    
    match get_tool_favorites(tool_type) {
        Ok(favorites) => {
            let total = favorites.len();
            success_response(ToolFavoritesListResponse { favorites, total })
        }
        Err(e) => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to get favorites",
            Some(&e),
        ),
    }
}

/// Add a new tool favorite
async fn add_favorite(Json(request): Json<AddToolFavoriteRequest>) -> Response {
    // Validate tool_type
    let valid_types = ["volume_aggregator", "buy_multi", "sell_multi", "token_watch"];
    if !valid_types.contains(&request.tool_type.as_str()) {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_TOOL_TYPE",
            "Invalid tool type",
            Some(&format!("Must be one of: {:?}", valid_types)),
        );
    }

    match upsert_tool_favorite(
        &request.mint,
        request.symbol.as_deref(),
        request.name.as_deref(),
        request.logo_url.as_deref(),
        &request.tool_type,
        request.config_json.as_deref(),
        request.label.as_deref(),
        request.notes.as_deref(),
    ) {
        Ok(id) => {
            logger::info(
                LogTag::Tools,
                &format!("Added tool favorite: {} for {}", request.mint, request.tool_type),
            );
            success_response(serde_json::json!({ "id": id, "success": true }))
        }
        Err(e) => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to add favorite",
            Some(&e),
        ),
    }
}

/// Update a tool favorite
async fn update_favorite(
    Path(id): Path<i64>,
    Json(request): Json<UpdateToolFavoriteRequest>,
) -> Response {
    match db_update_tool_favorite(
        id,
        request.config_json.as_deref(),
        request.label.as_deref(),
        request.notes.as_deref(),
    ) {
        Ok(true) => success_response(serde_json::json!({ "success": true })),
        Ok(false) => error_response(
            axum::http::StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Favorite not found",
            None,
        ),
        Err(e) => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to update favorite",
            Some(&e),
        ),
    }
}

/// Delete a tool favorite
async fn delete_favorite(Path(id): Path<i64>) -> Response {
    match remove_tool_favorite(id) {
        Ok(true) => {
            logger::info(LogTag::Tools, &format!("Removed tool favorite: {}", id));
            success_response(serde_json::json!({ "success": true }))
        }
        Ok(false) => error_response(
            axum::http::StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Favorite not found",
            None,
        ),
        Err(e) => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to delete favorite",
            Some(&e),
        ),
    }
}

/// Mark a favorite as used (increment counter)
async fn mark_favorite_used(Path(id): Path<i64>) -> Response {
    match increment_tool_favorite_use(id) {
        Ok(()) => success_response(serde_json::json!({ "success": true })),
        Err(e) => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            "Failed to update use count",
            Some(&e),
        ),
    }
}
