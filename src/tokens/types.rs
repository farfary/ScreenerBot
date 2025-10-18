/// Core types for the unified token data system
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::tokens::priorities::Priority;

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

// ============================================================================
// TOKEN METADATA - Lightweight token info for queries
// ============================================================================

/// Token metadata - Minimal token information used for database queries and listings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub created_at: i64,
    pub updated_at: i64,
}

// ============================================================================
// MAIN TOKEN STRUCTURE - Single source of truth used throughout the bot
// ============================================================================

/// Complete token information - THE primary token type used everywhere in the bot
/// 
/// This structure is populated from either DexScreener OR GeckoTerminal based on
/// the `data_source` field, which is determined by config.tokens.preferred_data_source.
/// 
/// All pool information, pricing, volume, and transaction data comes from the chosen source.
/// Security data (Rugcheck) is fetched independently and merged into this structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    // ========================================================================
    // Core Identity & Metadata (Required)
    // ========================================================================
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,

    // Optional metadata
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub header_image_url: Option<String>,
    pub supply: Option<String>,

    // ========================================================================
    // Data Source Configuration
    // ========================================================================
    /// Which API was used to populate price/volume/pool data (DexScreener or GeckoTerminal)
    pub data_source: DataSource,
    
    /// When this token data was fetched
    pub fetched_at: DateTime<Utc>,
    
    /// When this token data was last updated
    pub updated_at: DateTime<Utc>,

    // ========================================================================
    // Price Information (from chosen source)
    // ========================================================================
    pub price_usd: f64,
    pub price_sol: f64,
    pub price_native: String, // High precision as string

    // Price changes (timeframes available depend on source)
    pub price_change_m5: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub price_change_h6: Option<f64>,
    pub price_change_h24: Option<f64>,

    // ========================================================================
    // Market Metrics (from chosen source)
    // ========================================================================
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>, // Fully Diluted Valuation
    pub liquidity_usd: Option<f64>,

    // ========================================================================
    // Volume Data (timeframes from chosen source)
    // ========================================================================
    pub volume_m5: Option<f64>,
    pub volume_h1: Option<f64>,
    pub volume_h6: Option<f64>,
    pub volume_h24: Option<f64>,

    // ========================================================================
    // Transaction Activity (from chosen source)
    // ========================================================================
    pub txns_m5_buys: Option<i64>,
    pub txns_m5_sells: Option<i64>,
    pub txns_h1_buys: Option<i64>,
    pub txns_h1_sells: Option<i64>,
    pub txns_h6_buys: Option<i64>,
    pub txns_h6_sells: Option<i64>,
    pub txns_h24_buys: Option<i64>,
    pub txns_h24_sells: Option<i64>,

    // ========================================================================
    // All Pools (from chosen source)
    // ========================================================================
    /// All available pools for this token from the chosen data source
    /// Sorted by liquidity (highest first)
    pub all_pools: Vec<PoolSummary>,

    // ========================================================================
    // Social & Links (from chosen source)
    // ========================================================================
    pub websites: Vec<WebsiteLink>,
    pub socials: Vec<SocialLink>,

    // ========================================================================
    // Security Information (from Rugcheck - independent of price source)
    // ========================================================================
    // Authorities
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,

    // Security scoring
    pub rugcheck_score: Option<i32>,
    pub rugcheck_score_normalized: Option<i32>,
    pub is_rugged: bool,

    // Risks
    pub security_risks: Vec<RugcheckRisk>,

    // Holder data
    pub total_holders: Option<i64>,
    pub top_holders: Vec<RugcheckHolder>,
    pub creator_balance_pct: Option<f64>,

    // Transfer fee (Token-2022)
    pub transfer_fee_pct: Option<f64>,

    // ========================================================================
    // Bot-Specific State
    // ========================================================================
    pub is_blacklisted: bool,
    pub priority: Priority,
    pub first_seen_at: DateTime<Utc>,
    pub last_price_update: DateTime<Utc>,
}

/// Pool summary for token's all_pools field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSummary {
    pub pool_address: String,
    pub dex_id: String,
    pub liquidity_usd: f64,
    pub liquidity_sol: f64,
    pub volume_h24: Option<f64>,
    pub price_usd: String,
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

// ============================================================================
// API ERROR TYPES
// ============================================================================

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
