//! Type definitions for Tools API routes

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

pub fn default_sol_buffer() -> f64 {
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

pub fn default_delay_ms() -> u64 {
    1000
}

pub fn default_concurrency() -> usize {
    1
}

pub fn default_slippage() -> u64 {
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

pub fn default_sell_percentage() -> f64 {
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

pub fn default_min_sol_fee() -> f64 {
    0.01
}

pub fn default_auto_topup() -> bool {
    true
}

pub fn default_consolidate_after() -> bool {
    true
}

pub fn default_close_atas() -> bool {
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

pub fn default_transfer_sol() -> bool {
    true
}

pub fn default_include_token_2022() -> bool {
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
// ATA Cleanup Types
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

pub fn default_keypair_count() -> usize {
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

pub fn default_num_wallets() -> usize {
    5
}
pub fn default_min_amount() -> f64 {
    0.05
}
pub fn default_max_amount() -> f64 {
    0.2
}
pub fn default_delay() -> u64 {
    3000
}
pub fn default_strategy() -> String {
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

impl From<&crate::tools::VolumeSession> for VolumeSessionResponse {
    fn from(s: &crate::tools::VolumeSession) -> Self {
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

// =============================================================================
// Trade Watcher Types
// =============================================================================

/// Request to add a watched token
#[derive(Debug, Deserialize)]
pub struct AddWatchedTokenRequest {
    pub mint: String,
    pub symbol: Option<String>,
    pub pool_address: String,
    pub pool_source: String,
    pub pool_dex: Option<String>,
    pub watch_type: String,
    pub trigger_amount_sol: Option<f64>,
    pub action_amount_sol: Option<f64>,
}

// =============================================================================
// Burn Tokens Types
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
