/// Core types for the unified token data system
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Data source identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataSource {
    DexScreener,
    GeckoTerminal,
    Rugcheck,
}

impl DataSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataSource::DexScreener => "dexscreener",
            DataSource::GeckoTerminal => "geckoterminal",
            DataSource::Rugcheck => "rugcheck",
        }
    }
}

/// Core token metadata (chain-sourced)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub supply: Option<String>,
    pub first_seen_at: DateTime<Utc>,
    pub last_updated_at: DateTime<Utc>,
}

/// DexScreener pool data (per pair)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexScreenerPool {
    pub mint: String,
    pub pair_address: String,
    pub chain_id: String,
    pub dex_id: String,
    pub url: Option<String>,

    // Base and quote tokens
    pub base_token_address: String,
    pub base_token_name: String,
    pub base_token_symbol: String,
    pub quote_token_address: String,
    pub quote_token_name: String,
    pub quote_token_symbol: String,

    // Prices (high precision as strings)
    pub price_native: String,
    pub price_usd: String,

    // Liquidity
    pub liquidity_usd: Option<f64>,
    pub liquidity_base: Option<f64>,
    pub liquidity_quote: Option<f64>,

    // Volume
    pub volume_m5: Option<f64>,
    pub volume_h1: Option<f64>,
    pub volume_h6: Option<f64>,
    pub volume_h24: Option<f64>,

    // Transactions
    pub txns_m5_buys: Option<i64>,
    pub txns_m5_sells: Option<i64>,
    pub txns_h1_buys: Option<i64>,
    pub txns_h1_sells: Option<i64>,
    pub txns_h6_buys: Option<i64>,
    pub txns_h6_sells: Option<i64>,
    pub txns_h24_buys: Option<i64>,
    pub txns_h24_sells: Option<i64>,

    // Price changes
    pub price_change_m5: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub price_change_h6: Option<f64>,
    pub price_change_h24: Option<f64>,

    // Market metrics
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,

    // Metadata
    pub pair_created_at: Option<i64>,
    pub labels: Vec<String>,

    // Info
    pub info_image_url: Option<String>,
    pub info_header: Option<String>,
    pub info_open_graph: Option<String>,
    pub info_websites: Vec<WebsiteLink>,
    pub info_socials: Vec<SocialLink>,

    pub fetched_at: DateTime<Utc>,
}

/// GeckoTerminal pool data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalPool {
    pub mint: String,
    pub pool_address: String,
    pub pool_name: String,
    pub dex_id: String,

    // Token IDs
    pub base_token_id: String,
    pub quote_token_id: String,

    // Prices (high precision)
    pub base_token_price_usd: String,
    pub base_token_price_native: String,
    pub base_token_price_quote: String,
    pub quote_token_price_usd: String,
    pub quote_token_price_native: String,
    pub quote_token_price_base: String,
    pub token_price_usd: String,

    // Market metrics
    pub fdv_usd: Option<f64>,
    pub market_cap_usd: Option<f64>,
    pub reserve_usd: Option<f64>,

    // Volume
    pub volume_m5: Option<f64>,
    pub volume_m15: Option<f64>,
    pub volume_m30: Option<f64>,
    pub volume_h1: Option<f64>,
    pub volume_h6: Option<f64>,
    pub volume_h24: Option<f64>,

    // Price changes
    pub price_change_m5: Option<f64>,
    pub price_change_m15: Option<f64>,
    pub price_change_m30: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub price_change_h6: Option<f64>,
    pub price_change_h24: Option<f64>,

    // Transactions
    pub txns_m5_buys: Option<i64>,
    pub txns_m5_sells: Option<i64>,
    pub txns_m5_buyers: Option<i64>,
    pub txns_m5_sellers: Option<i64>,
    pub txns_m15_buys: Option<i64>,
    pub txns_m15_sells: Option<i64>,
    pub txns_m15_buyers: Option<i64>,
    pub txns_m15_sellers: Option<i64>,
    pub txns_m30_buys: Option<i64>,
    pub txns_m30_sells: Option<i64>,
    pub txns_m30_buyers: Option<i64>,
    pub txns_m30_sellers: Option<i64>,
    pub txns_h1_buys: Option<i64>,
    pub txns_h1_sells: Option<i64>,
    pub txns_h1_buyers: Option<i64>,
    pub txns_h1_sellers: Option<i64>,
    pub txns_h6_buys: Option<i64>,
    pub txns_h6_sells: Option<i64>,
    pub txns_h6_buyers: Option<i64>,
    pub txns_h6_sellers: Option<i64>,
    pub txns_h24_buys: Option<i64>,
    pub txns_h24_sells: Option<i64>,
    pub txns_h24_buyers: Option<i64>,
    pub txns_h24_sellers: Option<i64>,

    // Metadata
    pub pool_created_at: Option<String>,

    pub fetched_at: DateTime<Utc>,
}

/// Rugcheck security data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckInfo {
    pub mint: String,

    // Token program info
    pub token_program: Option<String>,
    pub token_type: Option<String>,

    // Token metadata
    pub token_name: Option<String>,
    pub token_symbol: Option<String>,
    pub token_decimals: Option<u8>,
    pub token_supply: Option<String>,
    pub token_uri: Option<String>,
    pub token_mutable: Option<bool>,
    pub token_update_authority: Option<String>,

    // Authorities
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,

    // Creator
    pub creator: Option<String>,
    pub creator_balance: Option<i64>,
    pub creator_tokens: Option<String>,

    // Scoring
    pub score: Option<i32>,
    pub score_normalised: Option<i32>,
    pub rugged: bool,

    // Risks
    pub risks: Vec<RugcheckRisk>,

    // Market data
    pub total_markets: Option<i64>,
    pub total_market_liquidity: Option<f64>,
    pub total_stable_liquidity: Option<f64>,
    pub total_lp_providers: Option<i64>,

    // Holders
    pub total_holders: Option<i64>,
    pub top_holders: Vec<RugcheckHolder>,
    pub graph_insiders_detected: Option<i64>,

    // Transfer fee
    pub transfer_fee_pct: Option<f64>,
    pub transfer_fee_max_amount: Option<i64>,
    pub transfer_fee_authority: Option<String>,

    // Metadata
    pub detected_at: Option<String>,
    pub analyzed_at: Option<String>,

    pub fetched_at: DateTime<Utc>,
}

/// Rugcheck risk item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckRisk {
    pub name: String,
    pub value: String,
    pub description: String,
    pub score: i32,
    pub level: String,
}

/// Rugcheck holder information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckHolder {
    pub address: String,
    pub amount: String,
    pub pct: f64,
    pub owner: Option<String>,
    pub insider: bool,
}

/// Website link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteLink {
    pub label: Option<String>,
    pub url: String,
}

/// Social link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialLink {
    pub link_type: String,
    pub url: String,
}

/// Complete token data from all sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteTokenData {
    pub mint: String,
    pub metadata: TokenMetadata,

    // Separate data per source
    pub dexscreener_pools: Vec<DexScreenerPool>,
    pub geckoterminal_pools: Vec<GeckoTerminalPool>,
    pub rugcheck_info: Option<RugcheckInfo>,

    // Fetch metadata
    pub sources_fetched: Vec<DataSource>,
    pub fetched_at: DateTime<Utc>,
}

/// API error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiError {
    NetworkError(String),
    RateLimitExceeded,
    InvalidResponse(String),
    NotFound,
    Timeout,
    Disabled,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ApiError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            ApiError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            ApiError::NotFound => write!(f, "Not found"),
            ApiError::Timeout => write!(f, "Request timeout"),
            ApiError::Disabled => write!(f, "API disabled"),
        }
    }
}

impl std::error::Error for ApiError {}

impl From<ApiError> for String {
    fn from(err: ApiError) -> String {
        err.to_string()
    }
}

// Default implementations for database row parsing
impl Default for DexScreenerPool {
    fn default() -> Self {
        Self {
            mint: String::new(),
            pair_address: String::new(),
            chain_id: String::new(),
            dex_id: String::new(),
            url: None,
            base_token_address: String::new(),
            base_token_name: String::new(),
            base_token_symbol: String::new(),
            quote_token_address: String::new(),
            quote_token_name: String::new(),
            quote_token_symbol: String::new(),
            price_native: String::new(),
            price_usd: String::new(),
            liquidity_usd: None,
            liquidity_base: None,
            liquidity_quote: None,
            volume_m5: None,
            volume_h1: None,
            volume_h6: None,
            volume_h24: None,
            txns_m5_buys: None,
            txns_m5_sells: None,
            txns_h1_buys: None,
            txns_h1_sells: None,
            txns_h6_buys: None,
            txns_h6_sells: None,
            txns_h24_buys: None,
            txns_h24_sells: None,
            price_change_m5: None,
            price_change_h1: None,
            price_change_h6: None,
            price_change_h24: None,
            fdv: None,
            market_cap: None,
            pair_created_at: None,
            labels: Vec::new(),
            info_image_url: None,
            info_header: None,
            info_open_graph: None,
            info_websites: Vec::new(),
            info_socials: Vec::new(),
            fetched_at: Utc::now(),
        }
    }
}

impl Default for GeckoTerminalPool {
    fn default() -> Self {
        Self {
            mint: String::new(),
            pool_address: String::new(),
            pool_name: String::new(),
            dex_id: String::new(),
            base_token_id: String::new(),
            quote_token_id: String::new(),
            base_token_price_usd: String::new(),
            base_token_price_native: String::new(),
            base_token_price_quote: String::new(),
            quote_token_price_usd: String::new(),
            quote_token_price_native: String::new(),
            quote_token_price_base: String::new(),
            token_price_usd: String::new(),
            fdv_usd: None,
            market_cap_usd: None,
            reserve_usd: None,
            volume_m5: None,
            volume_m15: None,
            volume_m30: None,
            volume_h1: None,
            volume_h6: None,
            volume_h24: None,
            price_change_m5: None,
            price_change_m15: None,
            price_change_m30: None,
            price_change_h1: None,
            price_change_h6: None,
            price_change_h24: None,
            txns_m5_buys: None,
            txns_m5_sells: None,
            txns_m5_buyers: None,
            txns_m5_sellers: None,
            txns_m15_buys: None,
            txns_m15_sells: None,
            txns_m15_buyers: None,
            txns_m15_sellers: None,
            txns_m30_buys: None,
            txns_m30_sells: None,
            txns_m30_buyers: None,
            txns_m30_sellers: None,
            txns_h1_buys: None,
            txns_h1_sells: None,
            txns_h1_buyers: None,
            txns_h1_sellers: None,
            txns_h6_buys: None,
            txns_h6_sells: None,
            txns_h6_buyers: None,
            txns_h6_sellers: None,
            txns_h24_buys: None,
            txns_h24_sells: None,
            txns_h24_buyers: None,
            txns_h24_sellers: None,
            pool_created_at: None,
            fetched_at: Utc::now(),
        }
    }
}

impl Default for RugcheckInfo {
    fn default() -> Self {
        Self {
            mint: String::new(),
            token_program: None,
            token_type: None,
            token_name: None,
            token_symbol: None,
            token_decimals: None,
            token_supply: None,
            token_uri: None,
            token_mutable: None,
            token_update_authority: None,
            mint_authority: None,
            freeze_authority: None,
            creator: None,
            creator_balance: None,
            creator_tokens: None,
            score: None,
            score_normalised: None,
            rugged: false,
            risks: Vec::new(),
            total_markets: None,
            total_market_liquidity: None,
            total_stable_liquidity: None,
            total_lp_providers: None,
            total_holders: None,
            top_holders: Vec::new(),
            graph_insiders_detected: None,
            transfer_fee_pct: None,
            transfer_fee_max_amount: None,
            transfer_fee_authority: None,
            detected_at: None,
            analyzed_at: None,
            fetched_at: Utc::now(),
        }
    }
}
