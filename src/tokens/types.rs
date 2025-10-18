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
/// **DATA SOURCE STRATEGY:**
/// - Market data (price, volume, liquidity, transactions): From config.tokens.preferred_market_data_source
///   (either "dexscreener" or "geckoterminal")
/// - Security data (authorities, risks, holders): Always from Rugcheck API
/// - Real-time pricing: Use pools module (src/pools/) which fetches on-chain pool data
///   and provides current pricing via pools::get_pool_price()
/// 
/// The `data_source` field indicates which API was used for market data.
/// Rugcheck is always fetched separately for security information.
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
    // Social & Links (from chosen source)
    // ========================================================================
    pub websites: Vec<WebsiteLink>,
    pub socials: Vec<SocialLink>,

    // ========================================================================
    // Security Information (from various sources - typically Rugcheck)
    // ========================================================================
    // Token authorities
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,

    // Security assessment
    pub security_score: Option<i32>,
    pub is_rugged: bool,

    // Security risks
    pub security_risks: Vec<SecurityRisk>,

    // Holder distribution
    pub total_holders: Option<i64>,
    pub top_holders: Vec<TokenHolder>,
    pub creator_balance_pct: Option<f64>,

    // Token-2022 transfer fee
    pub transfer_fee_pct: Option<f64>,

    // ========================================================================
    // Bot-Specific State
    // ========================================================================
    pub is_blacklisted: bool,
    pub priority: Priority,
    pub first_seen_at: DateTime<Utc>,
    pub last_price_update: DateTime<Utc>,
}

/// Security risk item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRisk {
    pub name: String,
    pub value: String,
    pub description: String,
    pub score: i32,
    pub level: String,
}

/// Token holder information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHolder {
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
