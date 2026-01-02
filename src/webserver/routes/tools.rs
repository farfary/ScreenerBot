//! Tools API routes for wallet utilities, token operations, and trading tools

use axum::{
    extract::Path,
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::DateTime;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ata_cleanup::{
    clear_failed_ata_cache, get_ata_cleanup_statistics, get_failed_ata_count,
    trigger_immediate_ata_cleanup,
};
use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use crate::tokens::decimals;
use crate::tools::multi_wallet::{
    execute_consolidation, execute_multi_buy, execute_multi_sell, ConsolidateConfig,
    MultiBuyConfig, MultiSellConfig, SessionResult, SessionStatus,
};
use crate::tools::{DelayConfig, ToolStatus, VolumeAggregator, VolumeConfig, VolumeSession};
use crate::utils::{get_all_token_accounts, get_wallet_address};
use crate::wallets;
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
// Multi-Wallet Global State
// =============================================================================

/// Session tracking for multi-wallet operations
struct MultiWalletSession {
    /// Session result (updated as operations progress)
    result: SessionResult,
    /// Current status
    status: SessionStatus,
    /// Abort flag for the running session
    abort_flag: Arc<AtomicBool>,
    /// Operation type for display
    operation_type: String,
    /// Token mint being traded
    token_mint: String,
    /// Started timestamp
    started_at: chrono::DateTime<chrono::Utc>,
}

/// Global multi-wallet sessions state
static MULTI_WALLET_SESSIONS: Lazy<Arc<RwLock<HashMap<String, MultiWalletSession>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Session cleanup interval (1 hour in seconds)
const SESSION_CLEANUP_INTERVAL_SECS: i64 = 3600;

/// Check if there's an active (non-completed) multi-wallet session
async fn has_active_multi_wallet_session() -> bool {
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
async fn cleanup_old_sessions() {
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
// Multi-Wallet Request/Response Types
// =============================================================================

/// Request to preview multi-buy operation
#[derive(Debug, Deserialize)]
pub struct MultiBuyPreviewRequest {
    /// Token mint address
    pub token_mint: String,
    /// Number of wallets to use
    pub wallet_count: usize,
    /// Minimum SOL per buy
    pub min_amount_sol: f64,
    /// Maximum SOL per buy
    pub max_amount_sol: f64,
    /// SOL buffer to leave in each wallet (default 0.015)
    #[serde(default = "default_sol_buffer")]
    pub sol_buffer: f64,
    /// Maximum total SOL to spend
    pub total_sol_limit: Option<f64>,
}

fn default_sol_buffer() -> f64 {
    0.015
}

/// Response for multi-buy preview
#[derive(Debug, Serialize)]
pub struct MultiBuyPreviewResponse {
    /// Number of wallets that will be created/used
    pub wallets_to_create: usize,
    /// Existing secondary wallets available
    pub existing_wallets: usize,
    /// Total SOL needed for operation
    pub total_sol_needed: f64,
    /// Average SOL per wallet buy
    pub per_wallet_sol: f64,
    /// Current main wallet balance
    pub main_wallet_balance: f64,
    /// Whether operation can proceed
    pub can_proceed: bool,
    /// Warning message if any
    pub warning: Option<String>,
    /// Wallet plans (preview of what will happen)
    pub wallet_plans: Vec<WalletPlanResponse>,
}

/// Wallet plan for API response
#[derive(Debug, Serialize)]
pub struct WalletPlanResponse {
    pub wallet_id: i64,
    pub wallet_address: String,
    pub wallet_name: String,
    pub current_sol_balance: f64,
    pub planned_buy_amount: f64,
    pub needs_funding: bool,
    pub funding_amount: f64,
}

/// Request to start multi-buy operation
#[derive(Debug, Deserialize)]
pub struct MultiBuyStartRequest {
    /// Token mint address
    pub token_mint: String,
    /// Number of wallets to use
    pub wallet_count: usize,
    /// Minimum SOL per buy
    pub min_amount_sol: f64,
    /// Maximum SOL per buy
    pub max_amount_sol: f64,
    /// SOL buffer to leave in each wallet
    #[serde(default = "default_sol_buffer")]
    pub sol_buffer: f64,
    /// Maximum total SOL to spend
    pub total_sol_limit: Option<f64>,
    /// Delay between operations in milliseconds
    #[serde(default = "default_delay_ms")]
    pub delay_ms: u64,
    /// Maximum delay for random mode
    pub delay_max_ms: Option<u64>,
    /// Number of concurrent operations
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Slippage in basis points
    #[serde(default = "default_slippage")]
    pub slippage_bps: u64,
    /// Router to use (jupiter, raydium, gmgn)
    pub router: Option<String>,
}

fn default_delay_ms() -> u64 {
    1000
}

fn default_concurrency() -> usize {
    1
}

fn default_slippage() -> u64 {
    500
}

/// Request to preview multi-sell operation
#[derive(Debug, Deserialize)]
pub struct MultiSellPreviewRequest {
    /// Token mint address
    pub token_mint: String,
    /// Specific wallet IDs to sell from (None = all with balance)
    pub wallet_ids: Option<Vec<i64>>,
    /// Percentage to sell (1-100)
    #[serde(default = "default_sell_percentage")]
    pub sell_percentage: f64,
}

fn default_sell_percentage() -> f64 {
    100.0
}

/// Response for multi-sell preview
#[derive(Debug, Serialize)]
pub struct MultiSellPreviewResponse {
    /// Token symbol (if known)
    pub token_symbol: Option<String>,
    /// Number of wallets with token balance
    pub wallets_with_balance: usize,
    /// Total token balance across all wallets
    pub total_token_balance: f64,
    /// Token amount to be sold
    pub token_to_sell: f64,
    /// Estimated SOL proceeds (if available)
    pub estimated_sol: Option<f64>,
    /// Whether operation can proceed
    pub can_proceed: bool,
    /// Warning message if any
    pub warning: Option<String>,
    /// Wallet details
    pub wallets: Vec<WalletTokenBalanceResponse>,
}

/// Wallet token balance for API response
#[derive(Debug, Serialize)]
pub struct WalletTokenBalanceResponse {
    pub wallet_id: i64,
    pub wallet_address: String,
    pub wallet_name: String,
    pub sol_balance: f64,
    pub token_balance: f64,
    pub needs_sol_topup: bool,
}

/// Request to start multi-sell operation
#[derive(Debug, Deserialize)]
pub struct MultiSellStartRequest {
    /// Token mint address
    pub token_mint: String,
    /// Specific wallet IDs to sell from
    pub wallet_ids: Option<Vec<i64>>,
    /// Percentage to sell (1-100)
    #[serde(default = "default_sell_percentage")]
    pub sell_percentage: f64,
    /// Minimum SOL for transaction fees
    #[serde(default = "default_min_sol_fee")]
    pub min_sol_for_fee: f64,
    /// Auto top-up wallets with insufficient SOL
    #[serde(default = "default_auto_topup")]
    pub auto_topup: bool,
    /// Delay between operations in milliseconds
    #[serde(default = "default_delay_ms")]
    pub delay_ms: u64,
    /// Maximum delay for random mode
    pub delay_max_ms: Option<u64>,
    /// Number of concurrent operations
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Slippage in basis points
    #[serde(default = "default_slippage")]
    pub slippage_bps: u64,
    /// Consolidate SOL to main wallet after sell
    #[serde(default = "default_consolidate_after")]
    pub consolidate_after: bool,
    /// Close token ATAs after selling
    #[serde(default = "default_close_atas")]
    pub close_atas_after: bool,
    /// Router to use
    pub router: Option<String>,
}

fn default_min_sol_fee() -> f64 {
    0.01
}

fn default_auto_topup() -> bool {
    true
}

fn default_consolidate_after() -> bool {
    true
}

fn default_close_atas() -> bool {
    true
}

/// Response for session start (both buy and sell)
#[derive(Debug, Serialize)]
pub struct SessionStartResponse {
    /// Unique session ID
    pub session_id: String,
    /// Status message
    pub message: String,
}

/// Response for session status
#[derive(Debug, Serialize)]
pub struct SessionStatusResponse {
    /// Session ID
    pub session_id: String,
    /// Current status
    pub status: String,
    /// Operation type (multi_buy, multi_sell, consolidate)
    pub operation_type: String,
    /// Token mint
    pub token_mint: String,
    /// Total wallets involved
    pub total_wallets: usize,
    /// Successful operations
    pub successful_ops: usize,
    /// Failed operations
    pub failed_ops: usize,
    /// Total SOL spent
    pub total_sol_spent: f64,
    /// Total SOL recovered
    pub total_sol_recovered: f64,
    /// Started timestamp
    pub started_at: String,
    /// Whether operation is complete
    pub is_complete: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Response for wallets summary
#[derive(Debug, Serialize)]
pub struct WalletsSummaryResponse {
    /// Total wallets
    pub total_wallets: usize,
    /// Active secondary wallets
    pub secondary_wallets: usize,
    /// Main wallet info
    pub main_wallet: Option<WalletInfoResponse>,
    /// Total SOL across all wallets
    pub total_sol: f64,
    /// Per-wallet details
    pub wallets: Vec<WalletInfoResponse>,
}

/// Wallet info for API response
#[derive(Debug, Clone, Serialize)]
pub struct WalletInfoResponse {
    pub id: i64,
    pub address: String,
    pub name: String,
    pub role: String,
    pub sol_balance: f64,
    pub is_active: bool,
}

/// Request for consolidation
#[derive(Debug, Deserialize)]
pub struct ConsolidateRequest {
    /// Specific wallet IDs to consolidate (None = all)
    pub wallet_ids: Option<Vec<i64>>,
    /// Transfer SOL to main wallet
    #[serde(default = "default_transfer_sol")]
    pub transfer_sol: bool,
    /// Token mints to transfer
    pub transfer_tokens: Option<Vec<String>>,
    /// Close empty ATAs
    #[serde(default = "default_close_atas")]
    pub close_atas: bool,
    /// Include Token-2022 accounts
    #[serde(default = "default_include_token_2022")]
    pub include_token_2022: bool,
    /// Leave rent-exempt amount in wallets
    #[serde(default)]
    pub leave_rent_exempt: bool,
}

fn default_transfer_sol() -> bool {
    true
}

fn default_include_token_2022() -> bool {
    true
}

/// Response for consolidation
#[derive(Debug, Serialize)]
pub struct ConsolidateResponse {
    /// Session ID
    pub session_id: String,
    /// Total wallets processed
    pub total_wallets: usize,
    /// Successful operations
    pub successful_ops: usize,
    /// Failed operations
    pub failed_ops: usize,
    /// SOL recovered
    pub sol_recovered: f64,
    /// Status message
    pub message: String,
}

/// Request for ATA cleanup on sub-wallets
#[derive(Debug, Deserialize)]
pub struct SubWalletAtaCleanupRequest {
    /// Specific wallet IDs (None = all secondary)
    pub wallet_ids: Option<Vec<i64>>,
    /// Include Token-2022 accounts
    #[serde(default = "default_include_token_2022")]
    pub include_token_2022: bool,
}

/// Response for sessions list
#[derive(Debug, Serialize)]
pub struct SessionsListResponse {
    /// Recent sessions
    pub sessions: Vec<SessionSummaryResponse>,
    /// Total count
    pub total: usize,
}

/// Session summary for list
#[derive(Debug, Serialize)]
pub struct SessionSummaryResponse {
    pub session_id: String,
    pub operation_type: String,
    pub token_mint: String,
    pub status: String,
    pub total_wallets: usize,
    pub successful_ops: usize,
    pub failed_ops: usize,
    pub started_at: String,
    pub is_complete: bool,
}

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

/// Create multi-wallet routes
fn multi_wallet_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Multi-Buy
        .route("/multi-buy/preview", post(preview_multi_buy))
        .route("/multi-buy/start", post(start_multi_buy))
        .route("/multi-buy/:id", get(get_multi_buy_status))
        .route("/multi-buy/:id/abort", post(abort_multi_buy))
        // Multi-Sell
        .route("/multi-sell/preview", post(preview_multi_sell))
        .route("/multi-sell/start", post(start_multi_sell))
        .route("/multi-sell/:id", get(get_multi_sell_status))
        .route("/multi-sell/:id/abort", post(abort_multi_sell))
        // Wallet Management
        .route("/wallets/summary", get(get_wallets_summary))
        .route("/wallets/consolidate", post(consolidate_wallets))
        .route("/wallets/cleanup-atas", post(cleanup_subwallet_atas))
        // Sessions
        .route("/multi-wallet/sessions", get(get_multi_wallet_sessions))
}

/// Create tools routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Wallet Cleanup (ATA management)
        .route("/ata-scan", get(scan_atas))
        .route("/ata-stats", get(get_ata_stats))
        .route("/ata-cleanup", post(cleanup_atas))
        .route("/ata-clear-cache", post(clear_ata_cache))
        // Burn Tokens
        .route("/burn-tokens/scan", get(scan_burnable_tokens))
        .route("/burn-tokens/burn", post(burn_selected_tokens))
        // Wallet Generator
        .route("/generate-keypair", post(generate_keypair))
        .route("/generate-keypairs", post(generate_keypairs))
        // Volume Aggregator
        .route("/volume-aggregator/start", post(start_volume_aggregator))
        .route(
            "/volume-aggregator/status",
            get(get_volume_aggregator_status),
        )
        .route("/volume-aggregator/stop", post(stop_volume_aggregator))
        .route(
            "/volume-aggregator/sessions",
            get(get_volume_aggregator_sessions),
        )
        // Tool Favorites
        .route("/favorites", get(get_favorites_list))
        .route("/favorites", post(add_favorite))
        .route("/favorites/:id", axum::routing::patch(update_favorite))
        .route("/favorites/:id", axum::routing::delete(delete_favorite))
        .route("/favorites/:id/use", post(mark_favorite_used))
        // Trade Watcher
        .route("/search-pools/:mint", get(search_pools_handler))
        .route("/watched-tokens", get(get_watched_tokens_handler))
        .route("/watched-tokens", post(add_watched_token_handler))
        .route("/watched-tokens/:id", delete(delete_watched_token_handler))
        .route("/trade-watcher/start", post(start_trade_watcher_handler))
        .route("/trade-watcher/stop", post(stop_trade_watcher_handler))
        .route(
            "/trade-watcher/status",
            get(get_trade_watcher_status_handler),
        )
        // Merge multi-wallet routes
        .merge(multi_wallet_routes())
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
            logger::error(
                LogTag::Wallet,
                &format!("Failed to get wallet address: {}", e),
            );
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
            logger::error(
                LogTag::Wallet,
                &format!("Failed to get token accounts: {}", e),
            );
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
    let reclaimable_sol =
        (empty_accounts.len() as f64 * ATA_RENT_LAMPORTS as f64) / 1_000_000_000.0;

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

// =============================================================================
// Tool Favorites Handlers
// =============================================================================

use crate::tools::database::{
    get_tool_favorites, increment_tool_favorite_use, remove_tool_favorite,
    update_tool_favorite as db_update_tool_favorite, upsert_tool_favorite,
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
    let valid_types = [
        "volume_aggregator",
        "buy_multi",
        "sell_multi",
        "token_watch",
    ];
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
                &format!(
                    "Added tool favorite: {} for {}",
                    request.mint, request.tool_type
                ),
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

// =============================================================================
// Multi-Wallet Handlers
// =============================================================================

/// Preview multi-buy operation
async fn preview_multi_buy(Json(request): Json<MultiBuyPreviewRequest>) -> Response {
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
async fn start_multi_buy(Json(request): Json<MultiBuyStartRequest>) -> Response {
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
async fn get_multi_buy_status(Path(id): Path<String>) -> Response {
    get_session_status(&id, "multi_buy").await
}

/// Abort multi-buy session
async fn abort_multi_buy(Path(id): Path<String>) -> Response {
    abort_session(&id).await
}

/// Preview multi-sell operation
async fn preview_multi_sell(Json(request): Json<MultiSellPreviewRequest>) -> Response {
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
async fn start_multi_sell(Json(request): Json<MultiSellStartRequest>) -> Response {
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
async fn get_multi_sell_status(Path(id): Path<String>) -> Response {
    get_session_status(&id, "multi_sell").await
}

/// Abort multi-sell session
async fn abort_multi_sell(Path(id): Path<String>) -> Response {
    abort_session(&id).await
}

/// Get wallets summary
async fn get_wallets_summary() -> Response {
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
async fn consolidate_wallets(Json(request): Json<ConsolidateRequest>) -> Response {
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
async fn cleanup_subwallet_atas(Json(request): Json<SubWalletAtaCleanupRequest>) -> Response {
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
async fn get_multi_wallet_sessions() -> Response {
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
async fn get_session_status(id: &str, expected_type: &str) -> Response {
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
async fn abort_session(id: &str) -> Response {
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

// =============================================================================
// Trade Watcher Handlers
// =============================================================================

/// Request to add a watched token
#[derive(Debug, Deserialize)]
struct AddWatchedTokenRequest {
    mint: String,
    symbol: Option<String>,
    pool_address: String,
    pool_source: String,
    pool_dex: Option<String>,
    watch_type: String,
    trigger_amount_sol: Option<f64>,
    action_amount_sol: Option<f64>,
}

/// Search pools for a token
async fn search_pools_handler(Path(mint): Path<String>) -> Response {
    use crate::tools::trade_watcher::search_pools;

    logger::debug(
        LogTag::Tools,
        &format!("[TRADE_WATCHER] API: Searching pools for mint={}", mint),
    );

    match search_pools(&mint).await {
        Ok(pools) => {
            logger::debug(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Found {} pools for mint={}",
                    pools.len(),
                    mint
                ),
            );
            success_response(serde_json::json!({ "pools": pools }))
        }
        Err(e) => {
            logger::warning(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Pool search failed for mint={}: {}",
                    mint, e
                ),
            );
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "POOL_SEARCH_ERROR",
                &e,
                Some(&mint),
            )
        }
    }
}

/// Get all watched tokens
async fn get_watched_tokens_handler() -> Response {
    use crate::tools::database::get_watched_tokens;

    match get_watched_tokens() {
        Ok(tokens) => success_response(serde_json::json!({ "tokens": tokens })),
        Err(e) => error_response(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "DATABASE_ERROR",
            &e,
            None,
        ),
    }
}

/// Add a watched token
async fn add_watched_token_handler(Json(req): Json<AddWatchedTokenRequest>) -> Response {
    use crate::tools::database::{add_watched_token, WatchedTokenConfig};

    logger::info(
        LogTag::Tools,
        &format!(
            "[TRADE_WATCHER] Adding watched token: mint={}, pool={}, watch_type={}",
            req.mint, req.pool_address, req.watch_type
        ),
    );

    let config = WatchedTokenConfig {
        mint: req.mint.clone(),
        symbol: req.symbol,
        pool_address: req.pool_address,
        pool_source: req.pool_source,
        pool_dex: req.pool_dex,
        pool_pair: None,
        pool_liquidity: None,
        watch_type: req.watch_type,
        trigger_amount_sol: req.trigger_amount_sol,
        action_amount_sol: req.action_amount_sol,
        slippage_bps: Some(500),
    };

    match add_watched_token(&config) {
        Ok(id) => {
            logger::info(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Added watched token: id={}, mint={}",
                    id, req.mint
                ),
            );
            success_response(serde_json::json!({ "id": id, "success": true }))
        }
        Err(e) => {
            logger::error(
                LogTag::Tools,
                &format!("[TRADE_WATCHER] Failed to add watched token: {}", e),
            );
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                &e,
                None,
            )
        }
    }
}

/// Delete a watched token
async fn delete_watched_token_handler(Path(id): Path<i64>) -> Response {
    use crate::tools::database::delete_watched_token;

    logger::info(
        LogTag::Tools,
        &format!("[TRADE_WATCHER] Deleting watched token: id={}", id),
    );

    match delete_watched_token(id) {
        Ok(()) => {
            logger::info(
                LogTag::Tools,
                &format!("[TRADE_WATCHER] Deleted watched token: id={}", id),
            );
            success_response(serde_json::json!({ "success": true }))
        }
        Err(e) => {
            logger::error(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] Failed to delete watched token id={}: {}",
                    id, e
                ),
            );
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                &e,
                None,
            )
        }
    }
}

/// Start the trade watcher monitor
async fn start_trade_watcher_handler() -> Response {
    use crate::tools::trade_watcher::start_trade_monitor;

    logger::info(
        LogTag::Tools,
        "[TRADE_WATCHER] Starting trade monitor via API",
    );
    start_trade_monitor().await;

    success_response(serde_json::json!({
        "success": true,
        "message": "Trade watcher started"
    }))
}

/// Stop the trade watcher monitor
async fn stop_trade_watcher_handler() -> Response {
    use crate::tools::trade_watcher::stop_trade_monitor;

    logger::info(
        LogTag::Tools,
        "[TRADE_WATCHER] Stopping trade monitor via API",
    );
    stop_trade_monitor().await;

    success_response(serde_json::json!({
        "success": true,
        "message": "Trade watcher stopped"
    }))
}

/// Get trade watcher status
async fn get_trade_watcher_status_handler() -> Response {
    use crate::tools::trade_watcher::get_trade_monitor_status;

    let status = get_trade_monitor_status().await;
    success_response(serde_json::json!({
        "is_running": status.is_running,
        "watched_count": status.watched_count,
        "active_count": status.active_count,
        "total_trades_detected": status.total_trades_detected,
        "total_actions_triggered": status.total_actions_triggered
    }))
}

// =============================================================================
// Burn Tokens Handlers
// =============================================================================

/// Token category for burn tokens UI
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TokenCategory {
    /// Token from an open position (should not burn)
    OpenPosition,
    /// Token from a closed position
    ClosedPosition,
    /// Token with known liquidity/value
    HasValue,
    /// Zero liquidity/dust token
    ZeroLiquidity,
}

/// Burnable token info for the UI
#[derive(Debug, Serialize)]
pub struct BurnableTokenInfo {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub balance: u64,
    pub ui_amount: f64,
    pub decimals: u8,
    pub is_token_2022: bool,
    pub category: TokenCategory,
    pub category_label: String,
    pub price_sol: Option<f64>,
    pub value_sol: Option<f64>,
    pub has_liquidity: bool,
    pub can_burn: bool,
    pub burn_warning: Option<String>,
    /// Estimated SOL to reclaim from closing ATA after burn
    pub rent_reclaimable_sol: f64,
}

/// Response for burn tokens scan
#[derive(Debug, Serialize)]
pub struct BurnTokensScanResponse {
    pub tokens: Vec<BurnableTokenInfo>,
    pub categories: BurnTokensCategories,
    pub total_rent_reclaimable_sol: f64,
}

/// Category counts for summary
#[derive(Debug, Serialize)]
pub struct BurnTokensCategories {
    pub open_positions: usize,
    pub closed_positions: usize,
    pub has_value: usize,
    pub zero_liquidity: usize,
}

/// Request to burn selected tokens
#[derive(Debug, Deserialize)]
pub struct BurnTokensRequest {
    pub mints: Vec<String>,
}

/// Individual burn result
#[derive(Debug, Serialize)]
pub struct BurnResult {
    pub mint: String,
    pub success: bool,
    pub signature: Option<String>,
    pub error: Option<String>,
}

/// Response for burn execution
#[derive(Debug, Serialize)]
pub struct BurnTokensResponse {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<BurnResult>,
    pub sol_reclaimed: f64,
}

/// Scan wallet for tokens that can be burned, with categorization
async fn scan_burnable_tokens() -> Response {
    use crate::constants::SOL_MINT;
    use crate::pools;
    use crate::positions;

    // ATA rent is approximately 0.00203928 SOL
    const ATA_RENT_SOL: f64 = 0.00203928;

    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            logger::error(
                LogTag::Tools,
                &format!("Failed to get wallet address: {}", e),
            );
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "WALLET_ERROR",
                "Failed to get wallet address",
                Some(&e.to_string()),
            );
        }
    };

    // Get all token accounts
    let all_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            logger::error(
                LogTag::Tools,
                &format!("Failed to get token accounts: {}", e),
            );
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "SCAN_ERROR",
                "Failed to scan token accounts",
                Some(&e.to_string()),
            );
        }
    };

    // Get open and closed positions for categorization
    let open_positions = positions::get_open_positions().await;
    let closed_positions = positions::get_closed_positions().await;

    let open_position_mints: std::collections::HashSet<String> =
        open_positions.iter().map(|p| p.mint.clone()).collect();
    let closed_position_mints: std::collections::HashSet<String> =
        closed_positions.iter().map(|p| p.mint.clone()).collect();

    // Get token metadata in batch
    let mints: Vec<String> = all_accounts.iter().map(|acc| acc.mint.clone()).collect();
    let mut metadata_map: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();

    if let Some(db) = crate::tokens::database::get_global_database() {
        for mint in &mints {
            if let Ok(Some(meta)) = db.get_token(mint) {
                metadata_map.insert(mint.clone(), (meta.symbol.clone(), meta.name.clone()));
            }
        }
    }

    // Build token list with categorization
    let mut tokens: Vec<BurnableTokenInfo> = Vec::new();
    let mut categories = BurnTokensCategories {
        open_positions: 0,
        closed_positions: 0,
        has_value: 0,
        zero_liquidity: 0,
    };
    let mut total_rent_reclaimable = 0.0f64;

    for account in &all_accounts {
        // Skip SOL itself and NFTs
        if account.mint == SOL_MINT || account.is_nft {
            continue;
        }

        // Skip empty accounts (they should use ATA cleanup instead)
        if account.balance == 0 {
            continue;
        }

        let (symbol, name) = metadata_map
            .get(&account.mint)
            .cloned()
            .unwrap_or((None, None));

        // Get price from pools module
        let price_result = pools::get_pool_price(&account.mint);
        let price_sol = price_result.as_ref().map(|p| p.price_sol);
        let has_liquidity = price_result
            .as_ref()
            .map(|p| p.sol_reserves > 0.0)
            .unwrap_or(false);

        // Calculate UI amount and value
        let ui_amount = account.balance as f64 / 10f64.powi(account.decimals as i32);
        let value_sol = price_sol.map(|p| p * ui_amount);

        // Determine category
        let (category, category_label, can_burn, burn_warning) =
            if open_position_mints.contains(&account.mint) {
                categories.open_positions += 1;
                (
                    TokenCategory::OpenPosition,
                    "Open Position".to_string(),
                    false,
                    Some("Cannot burn tokens from open positions".to_string()),
                )
            } else if closed_position_mints.contains(&account.mint) {
                categories.closed_positions += 1;
                (
                    TokenCategory::ClosedPosition,
                    "Closed Position".to_string(),
                    true,
                    Some("Leftover from closed position".to_string()),
                )
            } else if has_liquidity && value_sol.map(|v| v > 0.0001).unwrap_or(false) {
                categories.has_value += 1;
                (
                    TokenCategory::HasValue,
                    "Has Value".to_string(),
                    true,
                    Some(format!("Worth ~{:.6} SOL", value_sol.unwrap_or(0.0))),
                )
            } else {
                categories.zero_liquidity += 1;
                (
                    TokenCategory::ZeroLiquidity,
                    "Zero Liquidity".to_string(),
                    true,
                    None,
                )
            };

        // Only count rent reclaimable for tokens we can burn
        if can_burn {
            total_rent_reclaimable += ATA_RENT_SOL;
        }

        tokens.push(BurnableTokenInfo {
            mint: account.mint.clone(),
            symbol,
            name,
            balance: account.balance,
            ui_amount,
            decimals: account.decimals,
            is_token_2022: account.is_token_2022,
            category,
            category_label,
            price_sol,
            value_sol,
            has_liquidity,
            can_burn,
            burn_warning,
            rent_reclaimable_sol: if can_burn { ATA_RENT_SOL } else { 0.0 },
        });
    }

    // Sort tokens: Open positions first (can't burn), then by category
    tokens.sort_by(|a, b| {
        // Sort by category priority (open positions first as warning, then value, then zero liquidity)
        let priority = |t: &BurnableTokenInfo| match t.category {
            TokenCategory::OpenPosition => 0,
            TokenCategory::HasValue => 1,
            TokenCategory::ClosedPosition => 2,
            TokenCategory::ZeroLiquidity => 3,
        };

        let p1 = priority(a);
        let p2 = priority(b);

        if p1 != p2 {
            return p1.cmp(&p2);
        }

        // Within same category, sort by value (highest first)
        b.value_sol
            .unwrap_or(0.0)
            .partial_cmp(&a.value_sol.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    logger::info(
        LogTag::Tools,
        &format!(
            "Burn tokens scan: {} tokens found (open={}, closed={}, value={}, zero={})",
            tokens.len(),
            categories.open_positions,
            categories.closed_positions,
            categories.has_value,
            categories.zero_liquidity
        ),
    );

    success_response(BurnTokensScanResponse {
        tokens,
        categories,
        total_rent_reclaimable_sol: total_rent_reclaimable,
    })
}

/// Burn selected tokens
async fn burn_selected_tokens(Json(request): Json<BurnTokensRequest>) -> Response {
    use crate::constants::SOL_MINT;
    use crate::positions;
    use solana_sdk::transaction::Transaction;
    use spl_token::instruction as spl_instruction;

    const ATA_RENT_SOL: f64 = 0.00203928;

    if request.mints.is_empty() {
        return error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "NO_TOKENS",
            "No tokens selected for burning",
            None,
        );
    }

    // Get wallet address and keypair
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "WALLET_ERROR",
                "Failed to get wallet address",
                Some(&e.to_string()),
            );
        }
    };

    let wallet_keypair = match crate::config::get_wallet_keypair() {
        Ok(kp) => kp,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "KEYPAIR_ERROR",
                "Failed to load wallet keypair",
                Some(&e.to_string()),
            );
        }
    };

    // Check for open positions - prevent burning these
    let open_positions = positions::get_open_positions().await;
    let open_position_mints: std::collections::HashSet<String> =
        open_positions.iter().map(|p| p.mint.clone()).collect();

    // Get all token accounts
    let all_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "SCAN_ERROR",
                "Failed to get token accounts",
                Some(&e.to_string()),
            );
        }
    };

    // Build account map for quick lookup
    let account_map: HashMap<String, _> = all_accounts
        .iter()
        .map(|acc| (acc.mint.clone(), acc))
        .collect();

    let rpc_client = get_rpc_client();
    let mut results: Vec<BurnResult> = Vec::new();
    let mut successful = 0;
    let mut failed = 0;
    let mut sol_reclaimed = 0.0f64;

    for mint in &request.mints {
        // Skip SOL
        if mint == SOL_MINT {
            results.push(BurnResult {
                mint: mint.clone(),
                success: false,
                signature: None,
                error: Some("Cannot burn SOL".to_string()),
            });
            failed += 1;
            continue;
        }

        // Prevent burning open position tokens
        if open_position_mints.contains(mint) {
            results.push(BurnResult {
                mint: mint.clone(),
                success: false,
                signature: None,
                error: Some("Cannot burn tokens from open positions".to_string()),
            });
            failed += 1;
            continue;
        }

        // Find the account for this mint
        let account = match account_map.get(mint) {
            Some(acc) => acc,
            None => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some("Token account not found".to_string()),
                });
                failed += 1;
                continue;
            }
        };

        // Skip if balance is 0
        if account.balance == 0 {
            results.push(BurnResult {
                mint: mint.clone(),
                success: false,
                signature: None,
                error: Some("Token balance is already zero".to_string()),
            });
            failed += 1;
            continue;
        }

        // Parse addresses
        let wallet_pubkey = match Pubkey::from_str(&wallet_address) {
            Ok(pk) => pk,
            Err(e) => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Invalid wallet address: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        let mint_pubkey = match Pubkey::from_str(mint) {
            Ok(pk) => pk,
            Err(e) => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Invalid mint address: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        let ata_pubkey = match Pubkey::from_str(&account.account) {
            Ok(pk) => pk,
            Err(e) => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Invalid ATA address: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        // Determine token program
        let token_program_id = if account.is_token_2022 {
            spl_token_2022::id()
        } else {
            spl_token::id()
        };

        // Create burn instruction
        let burn_instruction = match spl_instruction::burn(
            &token_program_id,
            &ata_pubkey,
            &mint_pubkey,
            &wallet_pubkey,
            &[&wallet_pubkey],
            account.balance,
        ) {
            Ok(ix) => ix,
            Err(e) => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Failed to create burn instruction: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        // Get recent blockhash
        let recent_blockhash = match rpc_client.get_latest_blockhash().await {
            Ok(bh) => bh,
            Err(e) => {
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Failed to get blockhash: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        // Create and sign transaction
        let transaction = Transaction::new_signed_with_payer(
            &[burn_instruction],
            Some(&wallet_pubkey),
            &[&wallet_keypair],
            recent_blockhash,
        );

        // Send and confirm transaction
        match rpc_client
            .send_and_confirm_signed_transaction(&transaction)
            .await
        {
            Ok(signature) => {
                logger::info(
                    LogTag::Tools,
                    &format!(
                        "Burned {} tokens of {}. TX: {}",
                        account.balance, mint, signature
                    ),
                );
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: true,
                    signature: Some(signature.to_string()),
                    error: None,
                });
                successful += 1;
                sol_reclaimed += ATA_RENT_SOL; // Will be reclaimed when ATA is closed
            }
            Err(e) => {
                logger::error(
                    LogTag::Tools,
                    &format!("Failed to burn tokens for {}: {}", mint, e),
                );
                results.push(BurnResult {
                    mint: mint.clone(),
                    success: false,
                    signature: None,
                    error: Some(format!("Transaction failed: {}", e)),
                });
                failed += 1;
            }
        }

        // Small delay between burns to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    logger::info(
        LogTag::Tools,
        &format!(
            "Burn tokens complete: {}/{} successful, ~{:.6} SOL to reclaim via ATA cleanup",
            successful,
            request.mints.len(),
            sol_reclaimed
        ),
    );

    success_response(BurnTokensResponse {
        total: request.mints.len(),
        successful,
        failed,
        results,
        sol_reclaimed,
    })
}
