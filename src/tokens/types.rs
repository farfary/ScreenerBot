/// Core types for the unified token data system
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Re-export Priority for external modules
pub use crate::tokens::priorities::Priority;

/// Data source identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataSource {
    DexScreener,
    GeckoTerminal,
    Rugcheck,
    Unknown, // For tokens without market data
}

impl DataSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataSource::DexScreener => "dexscreener",
            DataSource::GeckoTerminal => "geckoterminal",
            DataSource::Rugcheck => "rugcheck",
            DataSource::Unknown => "unknown",
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
    pub first_discovered_at: i64,
    pub metadata_last_fetched_at: i64,
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
///
/// **TIMESTAMP FIELDS:**
/// All timestamp fields follow the pattern: {what}_{when}_{action}_at
/// - first_discovered_at: When bot first saw this token (immutable)
/// - blockchain_created_at: When token was created on-chain (if known)
/// - metadata_last_fetched_at: When symbol/name/decimals last fetched
/// - decimals_last_fetched_at: When decimals specifically fetched
/// - market_data_last_fetched_at: When market API data last fetched
/// - security_data_last_fetched_at: When Rugcheck data last fetched
/// - pool_price_last_calculated_at: When pool service last calculated price
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

    // ========================================================================
    // Discovery & Creation Timestamps
    // ========================================================================
    /// When bot first discovered this token (immutable after first insert)
    pub first_discovered_at: DateTime<Utc>,

    /// When this token/pair was created on blockchain (if known from APIs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blockchain_created_at: Option<DateTime<Utc>>,

    // ========================================================================
    // Metadata Timestamps
    // ========================================================================
    /// When symbol/name/decimals metadata was last fetched
    pub metadata_last_fetched_at: DateTime<Utc>,

    /// When decimals specifically was last fetched (can differ from metadata)
    pub decimals_last_fetched_at: DateTime<Utc>,

    // ========================================================================
    // Market Data Timestamps
    // ========================================================================
    /// When market API data (price/volume/liquidity) was last fetched from data_source
    pub market_data_last_fetched_at: DateTime<Utc>,

    // ========================================================================
    // Security Data Timestamps
    // ========================================================================
    /// When security data (Rugcheck) was last fetched
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_data_last_fetched_at: Option<DateTime<Utc>>,

    // ========================================================================
    // Pool Price Timestamps
    // ========================================================================
    /// When pool service last calculated price from on-chain data
    pub pool_price_last_calculated_at: DateTime<Utc>,

    /// Which pool address was used for the last price calculation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_price_last_used_pool: Option<String>,

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
    #[serde(rename = "volume_5m")]
    pub volume_m5: Option<f64>,
    #[serde(rename = "volume_1h")]
    pub volume_h1: Option<f64>,
    #[serde(rename = "volume_6h")]
    pub volume_h6: Option<f64>,
    #[serde(rename = "volume_24h")]
    pub volume_h24: Option<f64>,

    // Pool metrics (source-specific)
    pub pool_count: Option<u32>,
    pub reserve_in_usd: Option<f64>,

    // ========================================================================
    // Transaction Activity (from chosen source)
    // ========================================================================
    #[serde(rename = "txns_5m_buys")]
    pub txns_m5_buys: Option<i64>,
    #[serde(rename = "txns_5m_sells")]
    pub txns_m5_sells: Option<i64>,
    #[serde(rename = "txns_1h_buys")]
    pub txns_h1_buys: Option<i64>,
    #[serde(rename = "txns_1h_sells")]
    pub txns_h1_sells: Option<i64>,
    #[serde(rename = "txns_6h_buys")]
    pub txns_h6_buys: Option<i64>,
    #[serde(rename = "txns_6h_sells")]
    pub txns_h6_sells: Option<i64>,
    #[serde(rename = "txns_24h_buys")]
    pub txns_h24_buys: Option<i64>,
    #[serde(rename = "txns_24h_sells")]
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
    pub token_type: Option<String>,
    pub graph_insiders_detected: Option<i64>,
    pub lp_provider_count: Option<i64>,

    // Security risks
    pub security_risks: Vec<SecurityRisk>,

    // Holder distribution
    pub total_holders: Option<i64>,
    pub top_holders: Vec<TokenHolder>,
    pub creator_balance_pct: Option<f64>,

    // Token-2022 transfer fee
    pub transfer_fee_pct: Option<f64>,
    pub transfer_fee_max_amount: Option<i64>,
    pub transfer_fee_authority: Option<String>,

    // ========================================================================
    // Bot-Specific State
    // ========================================================================
    pub is_blacklisted: bool,
    pub priority: Priority,
}

impl Token {
    /// Calculate total transactions (buys + sells) for 5 minute range
    #[allow(dead_code)]
    pub fn txns_5m_total(&self) -> Option<i64> {
        match (self.txns_m5_buys, self.txns_m5_sells) {
            (Some(b), Some(s)) => Some(b + s),
            (Some(b), None) => Some(b),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        }
    }

    /// Calculate total transactions (buys + sells) for 1 hour range
    #[allow(dead_code)]
    pub fn txns_1h_total(&self) -> Option<i64> {
        match (self.txns_h1_buys, self.txns_h1_sells) {
            (Some(b), Some(s)) => Some(b + s),
            (Some(b), None) => Some(b),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        }
    }

    /// Calculate total transactions (buys + sells) for 6 hour range
    #[allow(dead_code)]
    pub fn txns_6h_total(&self) -> Option<i64> {
        match (self.txns_h6_buys, self.txns_h6_sells) {
            (Some(b), Some(s)) => Some(b + s),
            (Some(b), None) => Some(b),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        }
    }

    /// Calculate total transactions (buys + sells) for 24 hour range
    #[allow(dead_code)]
    pub fn txns_24h_total(&self) -> Option<i64> {
        match (self.txns_h24_buys, self.txns_h24_sells) {
            (Some(b), Some(s)) => Some(b + s),
            (Some(b), None) => Some(b),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        }
    }
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

// ============================================================================
// MARKET DATA TYPES (per source)
// ============================================================================

/// DexScreener market data for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexScreenerData {
    pub price_usd: f64,
    pub price_sol: f64,
    pub price_native: String,
    pub price_change_5m: Option<f64>,
    pub price_change_1h: Option<f64>,
    pub price_change_6h: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_5m: Option<f64>,
    pub volume_1h: Option<f64>,
    pub volume_6h: Option<f64>,
    pub volume_24h: Option<f64>,
    pub txns_5m: Option<(u32, u32)>, // (buys, sells)
    pub txns_1h: Option<(u32, u32)>,
    pub txns_6h: Option<(u32, u32)>,
    pub txns_24h: Option<(u32, u32)>,
    pub pair_address: Option<String>,
    pub chain_id: Option<String>,
    pub dex_id: Option<String>,
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pair_blockchain_created_at: Option<DateTime<Utc>>,
    pub market_data_last_fetched_at: DateTime<Utc>,
    pub market_data_first_fetched_at: DateTime<Utc>,
}

/// GeckoTerminal market data for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalData {
    pub price_usd: f64,
    pub price_sol: f64,
    pub price_native: String,
    pub price_change_5m: Option<f64>,
    pub price_change_1h: Option<f64>,
    pub price_change_6h: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_5m: Option<f64>,
    pub volume_1h: Option<f64>,
    pub volume_6h: Option<f64>,
    pub volume_24h: Option<f64>,
    pub pool_count: Option<u32>,
    pub top_pool_address: Option<String>,
    pub reserve_in_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    pub market_data_last_fetched_at: DateTime<Utc>,
    pub market_data_first_fetched_at: DateTime<Utc>,
}

/// Bundle of market data from all sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataBundle {
    pub dexscreener: Option<DexScreenerData>,
    pub geckoterminal: Option<GeckoTerminalData>,
}

// ============================================================================
// TOKEN POOL SNAPSHOTS (multi-source aggregated pool data)
// ============================================================================

/// Raw source payloads captured for each pool entry (serialized JSON blobs).
///
/// We keep source data in JSON form to avoid circular dependencies between the
/// tokens module and API client type definitions while still exposing full
/// payloads for debugging and UI purposes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenPoolSources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dexscreener: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geckoterminal: Option<Value>,
}

/// Aggregated pool information for a specific AMM pool.
///
/// Fields are normalized across providers so downstream systems (pools module,
/// OHLCV monitor, dashboard) can consume a single representation without
/// duplicating merge logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPoolInfo {
    pub pool_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dex: Option<String>,
    pub base_mint: String,
    pub quote_mint: String,
    pub is_sol_pair: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidity_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidity_token: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidity_sol: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_h24: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_sol: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_native: Option<String>,
    #[serde(default)]
    pub sources: TokenPoolSources,
    pub pool_data_last_fetched_at: DateTime<Utc>,
    pub pool_data_first_seen_at: DateTime<Utc>,
}

impl Default for TokenPoolInfo {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            pool_address: String::new(),
            dex: None,
            base_mint: String::new(),
            quote_mint: String::new(),
            is_sol_pair: false,
            liquidity_usd: None,
            liquidity_token: None,
            liquidity_sol: None,
            volume_h24: None,
            price_usd: None,
            price_sol: None,
            price_native: None,
            sources: TokenPoolSources::default(),
            pool_data_last_fetched_at: now,
            pool_data_first_seen_at: now,
        }
    }
}

/// Complete pool snapshot for a token across all discovery sources.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenPoolsSnapshot {
    pub mint: String,
    pub pools: Vec<TokenPoolInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_pool_address: Option<String>,
    pub pool_data_last_fetched_at: DateTime<Utc>,
}

// ============================================================================
// SECURITY DATA TYPES
// ============================================================================

/// Rugcheck security data for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckData {
    pub token_type: Option<String>,
    pub token_decimals: Option<u8>,
    pub score: Option<i32>,
    pub score_description: Option<String>,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub top_10_holders_pct: Option<f64>,
    pub total_holders: Option<i64>,
    pub total_lp_providers: Option<i64>,
    pub graph_insiders_detected: Option<i64>,
    pub total_market_liquidity: Option<f64>,
    pub total_stable_liquidity: Option<f64>,
    pub total_supply: Option<String>,
    pub creator_balance_pct: Option<f64>,
    pub transfer_fee_pct: Option<f64>,
    pub transfer_fee_max_amount: Option<i64>,
    pub transfer_fee_authority: Option<String>,
    pub rugged: bool,
    pub risks: Vec<SecurityRisk>,
    pub top_holders: Vec<TokenHolder>,
    pub markets: Option<serde_json::Value>, // Raw market data from rugcheck
    pub security_data_last_fetched_at: DateTime<Utc>,
    pub security_data_first_fetched_at: DateTime<Utc>,
}

/// On-chain security verification (future)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainSecurityData {
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub total_supply: String,
    pub holder_count: Option<u64>,
    pub verified_at: DateTime<Utc>,
}

/// Combined security assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityBundle {
    pub rugcheck: Option<RugcheckData>,
    pub onchain: Option<OnChainSecurityData>,
    pub combined_score: SecurityScore,
}

/// Security score (0-100, higher = safer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScore {
    pub score: i32,
    pub level: SecurityLevel,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    Safe,      // 80-100
    Good,      // 60-79
    Moderate,  // 40-59
    Risky,     // 20-39
    Dangerous, // 0-19
}

// ============================================================================
// UPDATE TRACKING
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTrackingInfo {
    pub mint: String,
    pub priority: i32,
    pub market_data_last_updated_at: Option<DateTime<Utc>>,
    pub market_data_update_count: u64,
    pub security_data_last_updated_at: Option<DateTime<Utc>>,
    pub security_data_update_count: u64,
    pub metadata_last_updated_at: Option<DateTime<Utc>>,
    pub decimals_last_updated_at: Option<DateTime<Utc>>,
    pub pool_price_last_calculated_at: Option<DateTime<Utc>>,
    pub pool_price_last_used_pool_address: Option<String>,
    pub last_error: Option<String>,
    pub last_error_at: Option<DateTime<Utc>>,
    pub market_error_count: u64,
    pub security_error_count: u64,
}

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Debug)]
pub enum TokenError {
    Database(String),
    Api {
        source: String,
        message: String,
    },
    RateLimit {
        source: String,
        message: String,
    },
    NotFound(String),
    InvalidMint(String),
    Blacklisted {
        mint: String,
        reason: String,
    },
    RateLimitExceeded {
        source: String,
    },
    PartialFailure {
        successful: usize,
        failed: usize,
        details: Vec<String>,
    },
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenError::Database(msg) => write!(f, "Database error: {}", msg),
            TokenError::Api { source, message } => write!(f, "API error ({}): {}", source, message),
            TokenError::RateLimit { source, message } => {
                write!(f, "Rate limit ({}): {}", source, message)
            }
            TokenError::NotFound(mint) => write!(f, "Token not found: {}", mint),
            TokenError::InvalidMint(mint) => write!(f, "Invalid mint address: {}", mint),
            TokenError::Blacklisted { mint, reason } => {
                write!(f, "Blacklisted {}: {}", mint, reason)
            }
            TokenError::RateLimitExceeded { source } => {
                write!(f, "Rate limit exceeded for {}", source)
            }
            TokenError::PartialFailure {
                successful,
                failed,
                details,
            } => {
                write!(
                    f,
                    "Partial failure: {} succeeded, {} failed. Details: {}",
                    successful,
                    failed,
                    details.join("; ")
                )
            }
        }
    }
}

impl std::error::Error for TokenError {}

pub type TokenResult<T> = Result<T, TokenError>;
