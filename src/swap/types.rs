/// Type definitions for the swap module
use serde::{ Deserialize, Serialize };

// Token mint constants
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

/// Error types for swap operations
#[derive(Debug, thiserror::Error)]
pub enum SwapError {
    #[error("DEX not available: {0}")] DexNotAvailable(String),

    #[error("Invalid route: {0}")] InvalidRoute(String),

    #[error("Slippage too high: expected {expected}, got {actual}")] SlippageTooHigh {
        expected: f64,
        actual: f64,
    },

    #[error(
        "Insufficient balance: required {required}, available {available}"
    )] InsufficientBalance {
        required: u64,
        available: u64,
    },

    #[error("Transaction failed: {0}")] TransactionFailed(String),

    #[error("Network error: {0}")] NetworkError(String),

    #[error("API error: {0}")] ApiError(String),

    #[error("Serialization error: {0}")] SerializationError(String),
}

/// Supported DEX types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DexType {
    Jupiter,
    Raydium,
    Gmgn,
}

impl std::fmt::Display for DexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DexType::Jupiter => write!(f, "Jupiter"),
            DexType::Raydium => write!(f, "Raydium"),
            DexType::Gmgn => write!(f, "GMGN"),
        }
    }
}

/// Main swap configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapConfig {
    pub enabled: bool,
    pub default_dex: String,
    pub is_anti_mev: bool,
    pub max_slippage: f64,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub dex_preferences: Vec<String>,
    pub jupiter: JupiterConfig,
    pub raydium: RaydiumConfig,
    pub gmgn: GmgnConfig,
}

impl Default for SwapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_dex: "jupiter".to_string(),
            is_anti_mev: false,
            max_slippage: 0.01, // 1%
            timeout_seconds: 30,
            retry_attempts: 3,
            dex_preferences: vec!["jupiter".to_string(), "raydium".to_string()],
            jupiter: JupiterConfig::default(),
            raydium: RaydiumConfig::default(),
            gmgn: GmgnConfig::default(),
        }
    }
}

/// Jupiter DEX configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JupiterConfig {
    pub enabled: bool,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub max_accounts: u32,
    pub only_direct_routes: bool,
    pub as_legacy_transaction: bool,
}

impl Default for JupiterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: "https://quote-api.jup.ag/v6".to_string(),
            timeout_seconds: 15,
            max_accounts: 64,
            only_direct_routes: false,
            as_legacy_transaction: false,
        }
    }
}

/// Raydium DEX configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumConfig {
    pub enabled: bool,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub pool_type: String,
}

impl Default for RaydiumConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: "https://api.raydium.io/v2".to_string(),
            timeout_seconds: 15,
            pool_type: "all".to_string(),
        }
    }
}

/// GMGN DEX configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnConfig {
    pub enabled: bool,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub referral_fee_bps: u32,
}

impl Default for GmgnConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default
            base_url: "https://gmgn.ai/api".to_string(),
            timeout_seconds: 15,
            referral_fee_bps: 0,
        }
    }
}

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub chain_id: u32,
    pub decimals: u8,
    pub name: String,
    pub symbol: String,
    pub logo_uri: Option<String>,
    pub tags: Vec<String>,
}

/// Platform fee information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFee {
    pub amount: String,
    pub fee_bps: u32,
}

/// Swap request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRequest {
    pub input_mint: String,
    pub output_mint: String,
    pub amount: u64,
    pub slippage_bps: u32,
    pub user_public_key: String,
    pub dex_preference: Option<DexType>,
    pub is_anti_mev: bool,
}

/// Swap route information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRoute {
    pub dex: DexType,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub other_amount_threshold: String,
    pub swap_mode: String,
    pub slippage_bps: u32,
    pub price_impact_pct: String,
    pub route_plan: Vec<RoutePlan>,
    pub platform_fee: Option<PlatformFee>,
    pub context_slot: Option<u64>,
    pub time_taken: Option<f64>,
}

/// Route plan step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePlan {
    pub swap_info: SwapInfo,
    pub percent: u32,
}

/// Swap information for a route step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapInfo {
    pub amm_key: String,
    pub label: String,
    pub input_mint: String,
    pub output_mint: String,
    pub in_amount: String,
    pub out_amount: String,
    pub fee_amount: String,
    pub fee_mint: String,
}

/// Swap transaction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapTransaction {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
    pub priority_fee_info: Option<PriorityFeeInfo>,
}

/// Priority fee information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityFeeInfo {
    pub compute_budget_instructions: Vec<serde_json::Value>,
    pub priority_fee_estimate: Option<u64>,
}

/// Swap execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResult {
    pub success: bool,
    pub signature: Option<String>,
    pub dex_used: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub slippage: f64,
    pub fee: u64,
    pub fee_lamports: u64,
    pub price_impact: f64,
    pub execution_time_ms: u64,
    pub error: Option<String>,
    pub route: SwapRoute,
    pub block_height: Option<u64>,
}

/// GMGN API response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnApiResponse {
    pub code: i32,
    pub msg: String,
    pub data: GmgnApiData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnApiData {
    pub raw_tx: Option<GmgnRawTx>,
    pub quote: Option<GmgnQuote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnRawTx {
    pub swap_transaction: String,
    pub last_valid_block_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnQuote {
    pub input_amount: String,
    pub output_amount: String,
    pub price_impact: Option<String>,
}
