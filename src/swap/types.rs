use anyhow::Result;
use serde::{ Deserialize, Serialize };
use solana_sdk::{ pubkey::Pubkey, signature::Signature };
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwapProvider {
    Jupiter,
    Gmgn,
}

impl std::fmt::Display for SwapProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapProvider::Jupiter => write!(f, "Jupiter"),
            SwapProvider::Gmgn => write!(f, "GMGN"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SwapRequest {
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub amount: u64,
    pub slippage_bps: u16,
    pub user_public_key: Pubkey,
    pub preferred_provider: Option<SwapProvider>,
    pub priority_fee: Option<u64>,
    pub compute_unit_price: Option<u64>,
    pub wrap_unwrap_sol: bool,
    pub use_shared_accounts: bool,
}

#[derive(Debug, Clone)]
pub struct SwapQuote {
    pub provider: SwapProvider,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub in_amount: u64,
    pub out_amount: u64,
    pub price_impact_pct: f64,
    pub slippage_bps: u16,
    pub route_steps: u32,
    pub estimated_fee: u64,
    pub compute_unit_limit: Option<u32>,
    pub priority_fee: u64,
    pub raw_response: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct SwapTransaction {
    pub provider: SwapProvider,
    pub quote: SwapQuote,
    pub serialized_transaction: String,
    pub last_valid_block_height: Option<u64>,
    pub recent_blockhash: Option<String>,
    pub compute_unit_limit: Option<u32>,
    pub priority_fee: u64,
}

#[derive(Debug, Clone)]
pub struct SwapExecutionResult {
    pub provider: SwapProvider,
    pub signature: Signature,
    pub input_amount: u64,
    pub output_amount: u64,
    pub actual_fee: u64,
    pub execution_time_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub mint: Pubkey,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub verified: bool,
    pub coingecko_id: Option<String>,
}

#[derive(Debug)]
pub enum SwapError {
    ProviderNotAvailable(SwapProvider),
    InvalidAmount(String),
    SlippageExceeded(f64, f64), // expected, actual
    InsufficientBalance(u64, u64), // required, available
    QuoteFailed(SwapProvider, String),
    TransactionFailed(SwapProvider, String),
    NetworkError(String),
    ConfigurationError(String),
    RateLimited(SwapProvider),
    InvalidToken(String),
    PriceImpactTooHigh(f64),
}

impl std::fmt::Display for SwapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SwapError::ProviderNotAvailable(provider) => {
                write!(f, "Provider {} is not available", provider)
            }
            SwapError::InvalidAmount(msg) => write!(f, "Invalid amount: {}", msg),
            SwapError::SlippageExceeded(expected, actual) => {
                write!(f, "Slippage exceeded: expected {:.2}%, got {:.2}%", expected, actual)
            }
            SwapError::InsufficientBalance(required, available) => {
                write!(f, "Insufficient balance: required {}, available {}", required, available)
            }
            SwapError::QuoteFailed(provider, msg) => {
                write!(f, "Quote failed for {}: {}", provider, msg)
            }
            SwapError::TransactionFailed(provider, msg) => {
                write!(f, "Transaction failed for {}: {}", provider, msg)
            }
            SwapError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            SwapError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            SwapError::RateLimited(provider) => write!(f, "Rate limited by {}", provider),
            SwapError::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            SwapError::PriceImpactTooHigh(impact) => {
                write!(f, "Price impact too high: {:.2}%", impact)
            }
        }
    }
}

impl std::error::Error for SwapError {}

pub type SwapResult<T> = Result<T, SwapError>;

// Provider-specific request/response types

// Jupiter types
#[derive(Debug, Serialize, Deserialize)]
pub struct JupiterQuoteRequest {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    pub amount: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JupiterQuoteResponse {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<serde_json::Value>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JupiterSwapRequest {
    #[serde(rename = "userPublicKey")]
    pub user_public_key: String,
    #[serde(rename = "quoteResponse")]
    pub quote_response: JupiterQuoteResponse,
    #[serde(rename = "wrapAndUnwrapSol")]
    pub wrap_and_unwrap_sol: bool,
    #[serde(rename = "useSharedAccounts")]
    pub use_shared_accounts: bool,
    #[serde(rename = "feeAccount")]
    pub fee_account: Option<String>,
    #[serde(rename = "trackingAccount")]
    pub tracking_account: Option<String>,
    #[serde(rename = "computeUnitPriceMicroLamports")]
    pub compute_unit_price_micro_lamports: Option<u64>,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
    #[serde(rename = "asLegacyTransaction")]
    pub as_legacy_transaction: bool,
    #[serde(rename = "useTokenLedger")]
    pub use_token_ledger: bool,
    #[serde(rename = "destinationTokenAccount")]
    pub destination_token_account: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JupiterSwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: u64,
    #[serde(rename = "computeUnitLimit")]
    pub compute_unit_limit: u32,
}

// GMGN types
#[derive(Debug, Serialize, Deserialize)]
pub struct GmgnQuoteRequest {
    pub token_in_address: String,
    pub token_out_address: String,
    pub in_amount: String,
    pub from_address: String,
    pub slippage: f64,
    pub swap_mode: String,
    pub fee: f64,
    pub is_anti_mev: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GmgnQuoteResponse {
    pub code: i32,
    pub msg: String,
    pub data: GmgnQuoteData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GmgnQuoteData {
    pub quote: GmgnQuote,
    pub raw_tx: GmgnRawTransaction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GmgnQuote {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: String,
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<serde_json::Value>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<serde_json::Value>,
    #[serde(rename = "contextSlot")]
    pub context_slot: Option<u64>,
    #[serde(rename = "timeTaken")]
    pub time_taken: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GmgnRawTransaction {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: u64,
    #[serde(rename = "recentBlockhash")]
    pub recent_blockhash: String,
}

// Statistics and monitoring
#[derive(Debug, Clone)]
pub struct SwapStats {
    pub total_swaps: u64,
    pub successful_swaps: u64,
    pub failed_swaps: u64,
    pub total_volume: f64,
    pub provider_stats: HashMap<SwapProvider, ProviderStats>,
    pub average_execution_time_ms: u64,
    pub average_slippage: f64,
}

#[derive(Debug, Clone)]
pub struct ProviderStats {
    pub swaps_count: u64,
    pub success_rate: f64,
    pub average_price_impact: f64,
    pub average_execution_time_ms: u64,
    pub total_volume: f64,
    pub error_count: u64,
    pub last_error: Option<String>,
}

impl Default for SwapStats {
    fn default() -> Self {
        Self {
            total_swaps: 0,
            successful_swaps: 0,
            failed_swaps: 0,
            total_volume: 0.0,
            provider_stats: HashMap::new(),
            average_execution_time_ms: 0,
            average_slippage: 0.0,
        }
    }
}

impl Default for ProviderStats {
    fn default() -> Self {
        Self {
            swaps_count: 0,
            success_rate: 0.0,
            average_price_impact: 0.0,
            average_execution_time_ms: 0,
            total_volume: 0.0,
            error_count: 0,
            last_error: None,
        }
    }
}
