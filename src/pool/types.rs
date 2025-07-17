use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;

/// Pool type enumeration for different DEX protocols
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PoolType {
    Raydium,
    Orca,
    Meteora,
    PumpFun,
    Jupiter,
    Serum,
    Unknown,
}

impl From<&str> for PoolType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "raydium" => PoolType::Raydium,
            "orca" => PoolType::Orca,
            "meteora" => PoolType::Meteora,
            "pumpfun" | "pump_fun" => PoolType::PumpFun,
            "jupiter" => PoolType::Jupiter,
            "serum" => PoolType::Serum,
            _ => PoolType::Unknown,
        }
    }
}

impl std::fmt::Display for PoolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolType::Raydium => write!(f, "Raydium"),
            PoolType::Orca => write!(f, "Orca"),
            PoolType::Meteora => write!(f, "Meteora"),
            PoolType::PumpFun => write!(f, "PumpFun"),
            PoolType::Jupiter => write!(f, "Jupiter"),
            PoolType::Serum => write!(f, "Serum"),
            PoolType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Pool information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub pool_address: String,
    pub pool_type: PoolType,
    pub base_token_mint: String,
    pub quote_token_mint: String,
    pub base_token_decimals: u8,
    pub quote_token_decimals: u8,
    pub liquidity_usd: f64,
    pub fee_rate: f64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub is_active: bool,
}

/// Pool reserves at a specific point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolReserve {
    pub pool_address: String,
    pub base_token_amount: u64,
    pub quote_token_amount: u64,
    pub timestamp: DateTime<Utc>,
    pub slot: u64,
}

/// Pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub total_pools: u64,
    pub active_pools: u64,
    pub pools_by_type: HashMap<String, u64>,
    pub total_reserves_history: u64,
    pub last_update: DateTime<Utc>,
    pub update_rate_per_hour: f64,
}

/// Raw pool data from on-chain account
#[derive(Debug, Clone)]
pub struct RawPoolData {
    pub pool_address: String,
    pub data: Vec<u8>,
    pub slot: u64,
}

/// Token balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub mint: String,
    pub amount: u64,
    pub decimals: u8,
}

/// Pool update event
#[derive(Debug, Clone)]
pub struct PoolUpdateEvent {
    pub pool_address: String,
    pub pool_type: PoolType,
    pub reserves: PoolReserve,
    pub price_change: Option<f64>,
}

/// Price calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceResult {
    pub token_mint: String,
    pub price_usd: f64,
    pub price_sol: f64,
    pub pool_address: String,
    pub pool_type: PoolType,
    pub confidence: f64, // 0.0 to 1.0, based on liquidity and freshness
    pub timestamp: DateTime<Utc>,
}

/// Pool monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMonitorConfig {
    pub update_interval_seconds: u64,
    pub max_concurrent_updates: usize,
    pub enable_raydium: bool,
    pub enable_orca: bool,
    pub enable_meteora: bool,
    pub enable_pumpfun: bool,
    pub enable_jupiter: bool,
    pub enable_serum: bool,
    pub min_liquidity_usd: f64,
}

impl Default for PoolMonitorConfig {
    fn default() -> Self {
        Self {
            update_interval_seconds: 5,
            max_concurrent_updates: 10,
            enable_raydium: true,
            enable_orca: true,
            enable_meteora: true,
            enable_pumpfun: true,
            enable_jupiter: false,
            enable_serum: false,
            min_liquidity_usd: 100.0,
        }
    }
}

/// Error types for pool operations
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Pool not found: {0}")] PoolNotFound(String),

    #[error("Invalid pool data: {0}")] InvalidPoolData(String),

    #[error("Decoder error: {0}")] DecoderError(String),

    #[error("RPC error: {0}")] RpcError(String),

    #[error("Database error: {0}")] DatabaseError(String),

    #[error("Price calculation error: {0}")] PriceCalculationError(String),
}

pub type PoolResult<T> = std::result::Result<T, PoolError>;
