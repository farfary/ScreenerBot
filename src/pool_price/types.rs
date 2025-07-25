/// Pool Price System Types and Constants
///
/// This module defines all the core types, constants, and data structures
/// used by the new pool price system.

use serde::{ Deserialize, Serialize };
use solana_sdk::pubkey::Pubkey;
use std::time::Instant;

// =============================================================================
// CONSTANTS
// =============================================================================

/// DexScreener API base URL for token pairs
pub const DEXSCREENER_API_BASE: &str = "https://api.dexscreener.com/latest/dex/tokens";

/// Rate limiting constants
pub const DEXSCREENER_RATE_LIMIT_PER_MINUTE: u32 = 30;
pub const SOLANA_RPC_RATE_LIMIT_PER_MINUTE: u32 = 100;

/// Cache TTL for pool addresses (5 minutes)
pub const POOL_ADDRESS_CACHE_TTL_SECS: u64 = 300;

/// Batch size for get_multiple_accounts calls
pub const MULTI_ACCOUNT_BATCH_SIZE: usize = 20;

/// Price validation tolerances
pub const MAX_PRICE_DEVIATION_PERCENT: f64 = 50.0;
pub const MIN_LIQUIDITY_USD: f64 = 100.0;

/// Pool monitoring interval for open positions
pub const POSITION_MONITORING_INTERVAL_SECS: u64 = 30;

// =============================================================================
// PROGRAM IDS - All known DEX program IDs
// =============================================================================

/// Raydium AMM program ID
pub const RAYDIUM_AMM_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Orca program ID
pub const ORCA_PROGRAM_ID: &str = "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP";

/// Meteora DLMM program ID
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

/// PumpFun program ID
pub const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// Jupiter program ID
pub const JUPITER_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

/// SOL and WSOL mint addresses
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/// Pool address cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolAddressEntry {
    pub mint: String,
    pub pool_addresses: Vec<PoolAddressInfo>,
    pub timestamp: u64, // Unix timestamp
    pub ttl_secs: u64,
}

/// Individual pool address information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolAddressInfo {
    pub address: String,
    pub program_id: String,
    pub dex_name: String,
    pub liquidity_usd: f64,
    pub pair_address: String, // From DexScreener
}

/// Pool data fetched from on-chain accounts
#[derive(Debug, Clone)]
pub struct PoolAccountData {
    pub address: Pubkey,
    pub program_id: String,
    pub dex_name: String,
    pub account_data: Vec<u8>,
    pub liquidity_usd: f64,
}

/// Decoded pool reserves and metadata
#[derive(Debug, Clone)]
pub struct DecodedPoolData {
    pub address: String,
    pub program_id: String,
    pub dex_name: String,
    pub token_a_mint: String,
    pub token_b_mint: String,
    pub token_a_reserve: u64,
    pub token_b_reserve: u64,
    pub token_a_decimals: u8,
    pub token_b_decimals: u8,
    pub liquidity_usd: f64,
    pub is_valid: bool,
}

/// Calculated token price from pool data
#[derive(Debug, Clone)]
pub struct CalculatedPrice {
    pub mint: String,
    pub price_sol: f64,
    pub price_usd: Option<f64>,
    pub source_pools: Vec<String>,
    pub weighted_average: bool,
    pub confidence: f64, // 0.0 to 1.0
    pub timestamp: Instant,
}

/// DexScreener API response structures
#[derive(Debug, Deserialize)]
pub struct DexScreenerResponse {
    pub pairs: Vec<DexScreenerPair>,
}

#[derive(Debug, Deserialize)]
pub struct DexScreenerPair {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    #[serde(rename = "dexId")]
    pub dex_id: String,
    pub url: String,
    #[serde(rename = "pairAddress")]
    pub pair_address: String,
    #[serde(rename = "baseToken")]
    pub base_token: DexScreenerToken,
    #[serde(rename = "quoteToken")]
    pub quote_token: DexScreenerToken,
    #[serde(rename = "priceNative")]
    pub price_native: String,
    #[serde(rename = "priceUsd")]
    pub price_usd: Option<String>,
    pub liquidity: Option<DexScreenerLiquidity>,
    pub volume: Option<DexScreenerVolume>,
}

#[derive(Debug, Deserialize)]
pub struct DexScreenerToken {
    pub address: String,
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Deserialize)]
pub struct DexScreenerLiquidity {
    pub usd: Option<f64>,
    pub base: Option<f64>,
    pub quote: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct DexScreenerVolume {
    #[serde(rename = "h24")]
    pub h24: Option<f64>,
    #[serde(rename = "h6")]
    pub h6: Option<f64>,
    #[serde(rename = "h1")]
    pub h1: Option<f64>,
    #[serde(rename = "m5")]
    pub m5: Option<f64>,
}

/// Pool type classification based on program ID
#[derive(Debug, Clone, PartialEq)]
pub enum PoolType {
    RaydiumAmm,
    Orca,
    MeteoraDlmm,
    PumpFun,
    Jupiter,
    Unknown(String),
}

impl PoolType {
    /// Classify pool type from program ID
    pub fn from_program_id(program_id: &str) -> Self {
        match program_id {
            RAYDIUM_AMM_PROGRAM_ID => PoolType::RaydiumAmm,
            ORCA_PROGRAM_ID => PoolType::Orca,
            METEORA_DLMM_PROGRAM_ID => PoolType::MeteoraDlmm,
            PUMPFUN_PROGRAM_ID => PoolType::PumpFun,
            JUPITER_PROGRAM_ID => PoolType::Jupiter,
            _ => PoolType::Unknown(program_id.to_string()),
        }
    }

    /// Get human-readable DEX name
    pub fn dex_name(&self) -> &'static str {
        match self {
            PoolType::RaydiumAmm => "Raydium",
            PoolType::Orca => "Orca",
            PoolType::MeteoraDlmm => "Meteora",
            PoolType::PumpFun => "PumpFun",
            PoolType::Jupiter => "Jupiter",
            PoolType::Unknown(_) => "Unknown",
        }
    }

    /// Check if pool type is supported for decoding
    pub fn is_supported(&self) -> bool {
        match self {
            PoolType::RaydiumAmm | PoolType::Orca | PoolType::MeteoraDlmm | PoolType::PumpFun =>
                true,
            _ => false,
        }
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum PoolPriceError {
    #[error("DexScreener API error: {0}")] DexScreenerApi(String),

    #[error("Solana RPC error: {0}")] SolanaRpc(String),

    #[error("Pool decoding error: {0}")] PoolDecoding(String),

    #[error("Price calculation error: {0}")] PriceCalculation(String),

    #[error("Cache error: {0}")] Cache(String),

    #[error("Rate limit exceeded: {0}")] RateLimit(String),

    #[error("Validation error: {0}")] Validation(String),

    #[error("Network error: {0}")] Network(String),
}

pub type PoolPriceResult<T> = Result<T, PoolPriceError>;
