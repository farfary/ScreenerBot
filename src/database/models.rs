use chrono::{ DateTime, Utc };

/// Represents a tracked token with its current market data
#[derive(Debug, Clone)]
pub struct TrackedToken {
    pub address: String,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub price_usd: f64,
    pub last_updated: u64,
}

/// Database configuration and connection details
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub path: String,
    pub pool_size: Option<u32>,
    pub timeout_seconds: Option<u64>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "screener.db".to_string(),
            pool_size: Some(10),
            timeout_seconds: Some(30),
        }
    }
}

/// Token priority tracking information
#[derive(Debug, Clone)]
pub struct TokenPriority {
    pub token_address: String,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub priority_score: f64,
    pub update_interval_secs: u64,
    pub last_updated: DateTime<Utc>,
    pub consecutive_failures: u32,
}

/// Blacklisted token information
#[derive(Debug, Clone)]
pub struct BlacklistedToken {
    pub token_address: String,
    pub reason: String,
    pub blacklisted_at: DateTime<Utc>,
    pub last_liquidity: f64,
}

/// Database operation result statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_tokens: u64,
    pub active_tokens: u64,
    pub blacklisted_tokens: u64,
    pub total_pools: u64,
    pub total_price_records: u64,
    pub last_updated: DateTime<Utc>,
}

/// Database query parameters for filtering
#[derive(Debug, Clone)]
pub struct QueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub min_liquidity: Option<f64>,
    pub max_age_hours: Option<u64>,
    pub order_by: Option<String>,
    pub order_desc: bool,
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            limit: None,
            offset: None,
            min_liquidity: None,
            max_age_hours: None,
            order_by: None,
            order_desc: true,
        }
    }
}

/// Result wrapper for database operations
pub type DatabaseResult<T> = anyhow::Result<T>;
