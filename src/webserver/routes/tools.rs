//! Tools API routes for wallet utilities, token operations, and trading tools

use axum::{response::Response, routing::{get, post}, Json, Router};
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
