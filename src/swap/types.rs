use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SwapConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_dex")]
    pub default_dex: String,
    #[serde(default)]
    pub is_anti_mev: bool,
    #[serde(default = "default_max_slippage")]
    pub max_slippage: f64,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
    #[serde(default = "default_dex_preferences")]
    pub dex_preferences: Vec<String>,
    #[serde(default)]
    pub jupiter: JupiterConfig,
    #[serde(default)]
    pub raydium: RaydiumConfig,
    #[serde(default)]
    pub gmgn: GmgnConfig,
}

fn default_enabled() -> bool { true }
fn default_dex() -> String { "jupiter".to_string() }
fn default_max_slippage() -> f64 { 0.01 }
fn default_timeout_seconds() -> u64 { 30 }
fn default_retry_attempts() -> u32 { 3 }
fn default_dex_preferences() -> Vec<String> { 
    vec!["jupiter".to_string(), "raydium".to_string(), "gmgn".to_string()] 
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JupiterConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_jupiter_url")]
    pub base_url: String,
    #[serde(default = "default_timeout_seconds_15")]
    pub timeout_seconds: u64,
    #[serde(default = "default_max_accounts")]
    pub max_accounts: u32,
    #[serde(default)]
    pub only_direct_routes: bool,
    #[serde(default)]
    pub as_legacy_transaction: bool,
}

fn default_jupiter_url() -> String { "https://quote-api.jup.ag/v6".to_string() }
fn default_timeout_seconds_15() -> u64 { 15 }
fn default_max_accounts() -> u32 { 64 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RaydiumConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_raydium_url")]
    pub base_url: String,
    #[serde(default = "default_timeout_seconds_15")]
    pub timeout_seconds: u64,
    #[serde(default = "default_pool_type")]
    pub pool_type: String,
}

fn default_raydium_url() -> String { "https://api.raydium.io/v2".to_string() }
fn default_pool_type() -> String { "all".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GmgnConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_gmgn_url")]
    pub base_url: String,
    #[serde(default = "default_timeout_seconds_15")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub referral_account: String,
    #[serde(default)]
    pub referral_fee_bps: u32,
}

fn default_gmgn_url() -> String { "https://gmgn.ai/defi/quoterv1".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DexType {
    Jupiter,
    Raydium,
    Gmgn,
}

impl std::fmt::Display for DexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DexType::Jupiter => write!(f, "jupiter"),
            DexType::Raydium => write!(f, "raydium"),
            DexType::Gmgn => write!(f, "gmgn"),
        }
    }
}

impl std::str::FromStr for DexType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "jupiter" => Ok(DexType::Jupiter),
            "raydium" => Ok(DexType::Raydium),
            "gmgn" => Ok(DexType::Gmgn),
            _ => Err(anyhow::anyhow!("Unknown DEX type: {}", s)),
        }
    }
}

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
    pub platform_fee: Option<PlatformFee>,
    pub price_impact_pct: String,
    pub route_plan: Vec<RoutePlan>,
    pub context_slot: Option<u64>,
    pub time_taken: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformFee {
    pub amount: String,
    pub fee_bps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePlan {
    pub swap_info: SwapInfo,
    pub percent: u32,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapTransaction {
    pub swap_transaction: String, // Base64 encoded transaction
    pub last_valid_block_height: u64,
    pub priority_fee_info: Option<PriorityFeeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityFeeInfo {
    pub compute_budget_instructions: Vec<ComputeBudgetInstruction>,
    pub prioritization_fee_lamports: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeBudgetInstruction {
    pub program_id: String,
    pub accounts: Vec<String>,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct SwapResult {
    pub success: bool,
    pub signature: Option<String>,
    pub dex_used: DexType,
    pub input_amount: u64,
    pub output_amount: u64,
    pub price_impact: f64,
    pub fee_lamports: u64,
    pub route: SwapRoute,
    pub error: Option<String>,
    pub block_height: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub logo_uri: Option<String>,
}

// GMGN specific types (Updated API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnApiResponse {
    pub code: i32,
    #[serde(default)]
    pub msg: String,
    #[serde(default)]
    pub tid: String,
    pub data: GmgnApiData,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GmgnApiData {
    #[serde(default)]
    pub quote: Option<GmgnQuote>,
    #[serde(default)]
    pub raw_tx: Option<GmgnRawTx>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnQuote {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inputAmount")]
    pub input_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outputAmount")]
    pub output_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u32,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<GmgnRoutePlan>,
    #[serde(rename = "contextSlot")]
    pub context_slot: u64,
    #[serde(rename = "timeTaken")]
    pub time_taken: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnRawTx {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
}

// Legacy GMGN types (keeping for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnQuoteRequest {
    pub from: String,
    pub to: String,
    pub amount: String,
    pub slippage: f64,
    pub txVersion: String,
    pub referral: Option<String>,
    pub referralFeeBps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnQuoteResponse {
    pub inputMint: String,
    pub inputAmount: String,
    pub outputMint: String,
    pub outputAmount: String,
    pub otherAmountThreshold: String,
    pub slippageBps: u32,
    pub priceImpactPct: String,
    pub routePlan: Vec<GmgnRoutePlan>,
    pub contextSlot: u64,
    pub timeTaken: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnRoutePlan {
    pub swapInfo: GmgnSwapInfo,
    pub percent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnSwapInfo {
    pub ammKey: String,
    pub label: String,
    pub inputMint: String,
    pub outputMint: String,
    pub inAmount: String,
    pub outAmount: String,
    pub feeAmount: String,
    pub feeMint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnSwapRequest {
    pub from: String,
    pub to: String,
    pub fromAmount: String,
    pub slippage: f64,
    pub userPublicKey: String,
    pub txVersion: String,
    pub referral: Option<String>,
    pub referralFeeBps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmgnSwapResponse {
    pub swapTransaction: String,
    pub lastValidBlockHeight: u64,
}

// Error types
#[derive(Debug, thiserror::Error)]
pub enum SwapError {
    #[error("DEX not available: {0}")]
    DexNotAvailable(String),
    #[error("Invalid route: {0}")]
    InvalidRoute(String),
    #[error("Slippage too high: expected {expected}, got {actual}")]
    SlippageTooHigh { expected: f64, actual: f64 },
    #[error("Insufficient liquidity")]
    InsufficientLiquidity,
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    #[error("Timeout")]
    Timeout,
    #[error("API error: {0}")]
    ApiError(String),
}

impl From<solana_client::client_error::ClientError> for SwapError {
    fn from(err: solana_client::client_error::ClientError) -> Self {
        SwapError::TransactionFailed(err.to_string())
    }
}

// Constants for common tokens
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";

pub fn get_common_tokens() -> HashMap<String, TokenInfo> {
    let mut tokens = HashMap::new();
    
    tokens.insert(
        SOL_MINT.to_string(),
        TokenInfo {
            mint: SOL_MINT.to_string(),
            symbol: "SOL".to_string(),
            name: "Wrapped SOL".to_string(),
            decimals: 9,
            logo_uri: Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png".to_string()),
        },
    );
    
    tokens.insert(
        USDC_MINT.to_string(),
        TokenInfo {
            mint: USDC_MINT.to_string(),
            symbol: "USDC".to_string(),
            name: "USD Coin".to_string(),
            decimals: 6,
            logo_uri: Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png".to_string()),
        },
    );
    
    tokens.insert(
        USDT_MINT.to_string(),
        TokenInfo {
            mint: USDT_MINT.to_string(),
            symbol: "USDT".to_string(),
            name: "Tether USD".to_string(),
            decimals: 6,
            logo_uri: Some("https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB/logo.png".to_string()),
        },
    );
    
    tokens
}
