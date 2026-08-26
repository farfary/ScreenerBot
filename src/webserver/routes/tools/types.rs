//! Type definitions for Tools API routes

use serde::{Deserialize, Serialize};

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
    /// Router to use (jupiter, raydium)
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
