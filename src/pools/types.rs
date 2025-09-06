/// Core types for the pools module
///
/// This file contains all the essential data structures used throughout the pools system.
/// These types are designed to be minimal, efficient, and focused on the core functionality.

use serde::{ Deserialize, Serialize };
use solana_sdk::pubkey::Pubkey;
use std::time::{ Instant, SystemTime, UNIX_EPOCH };
use std::collections::VecDeque;

/// The main price result structure - this is the primary data exchange format
///
/// This struct represents a calculated price for a token and is used throughout
/// the trading system for all price-related operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceResult {
    /// Token mint address
    pub mint: String,
    /// Price in USD
    pub price_usd: f64,
    /// Price in SOL (primary trading currency)
    pub price_sol: f64,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Source pool ID that provided this price
    pub source_pool: Option<String>,
    /// Pool address for this price data
    pub pool_address: String,
    /// Blockchain slot when this price was calculated
    pub slot: u64,
    /// Timestamp when this price was calculated (as Unix timestamp)
    #[serde(with = "instant_serde")]
    pub timestamp: Instant,
    /// SOL reserves in the pool
    pub sol_reserves: f64,
    /// Token reserves in the pool
    pub token_reserves: f64,
}

impl Default for PriceResult {
    fn default() -> Self {
        Self {
            mint: String::new(),
            price_usd: 0.0,
            price_sol: 0.0,
            confidence: 0.0,
            source_pool: None,
            pool_address: String::new(),
            slot: 0,
            timestamp: Instant::now(),
            sol_reserves: 0.0,
            token_reserves: 0.0,
        }
    }
}

impl PriceResult {
    /// Create a new price result
    pub fn new(
        mint: String,
        price_usd: f64,
        price_sol: f64,
        sol_reserves: f64,
        token_reserves: f64,
        pool_address: String
    ) -> Self {
        Self {
            mint,
            price_usd,
            price_sol,
            confidence: 1.0,
            source_pool: None,
            pool_address,
            slot: 0,
            timestamp: Instant::now(),
            sol_reserves,
            token_reserves,
        }
    }

    /// Check if this price is fresh (within acceptable time threshold)
    pub fn is_fresh(&self, max_age_seconds: u64) -> bool {
        self.timestamp.elapsed().as_secs() < max_age_seconds
    }
}

/// Custom serde module for Instant serialization
mod instant_serde {
    use serde::{ Deserialize, Deserializer, Serialize, Serializer };
    use std::time::{ Duration, Instant, SystemTime, UNIX_EPOCH };

    pub fn serialize<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        // Convert Instant to unix timestamp (this is an approximation)
        let elapsed = instant.elapsed();
        let now = SystemTime::now();
        let timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs();
        timestamp.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
        where D: Deserializer<'de>
    {
        let timestamp = u64::deserialize(deserializer)?;
        // This is an approximation - we just return current Instant
        // In practice, we should use a different time representation for serialization
        Ok(Instant::now())
    }
}

/// Pool service error types
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("Pool service initialization failed: {0}")] InitializationFailed(String),

    #[error("Pool service not running")]
    ServiceNotRunning,

    #[error("Price not available for token: {0}")] PriceNotAvailable(String),

    #[error("RPC error: {0}")] RpcError(String),

    #[error("Decode error: {0}")] DecodeError(String),
}

/// Program types for different DEX implementations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgramKind {
    RaydiumCpmm,
    RaydiumLegacyAmm,
    RaydiumClmm,
    OrcaWhirlpool,
    MeteoraDamm,
    MeteoraDlmm,
    PumpFun,
    Unknown,
}

impl ProgramKind {
    /// Get the program ID for this pool type
    pub fn program_id(&self) -> &'static str {
        match self {
            ProgramKind::RaydiumCpmm => RAYDIUM_CPMM_PROGRAM_ID,
            ProgramKind::RaydiumLegacyAmm => RAYDIUM_LEGACY_AMM_PROGRAM_ID,
            ProgramKind::RaydiumClmm => RAYDIUM_CLMM_PROGRAM_ID,
            ProgramKind::OrcaWhirlpool => ORCA_WHIRLPOOL_PROGRAM_ID,
            ProgramKind::MeteoraDamm => METEORA_DAMM_PROGRAM_ID,
            ProgramKind::MeteoraDlmm => METEORA_DLMM_PROGRAM_ID,
            ProgramKind::PumpFun => PUMP_FUN_PROGRAM_ID,
            ProgramKind::Unknown => "",
        }
    }

    /// Get display name for this program kind
    pub fn display_name(&self) -> &'static str {
        match self {
            ProgramKind::RaydiumCpmm => "RAYDIUM CPMM",
            ProgramKind::RaydiumLegacyAmm => "RAYDIUM LEGACY AMM",
            ProgramKind::RaydiumClmm => "RAYDIUM CLMM",
            ProgramKind::OrcaWhirlpool => "ORCA WHIRLPOOL",
            ProgramKind::MeteoraDamm => "METEORA DAMM v2",
            ProgramKind::MeteoraDlmm => "METEORA DLMM",
            ProgramKind::PumpFun => "PUMP.FUN AMM",
            ProgramKind::Unknown => "UNKNOWN",
        }
    }

    /// Create ProgramKind from program ID string
    pub fn from_program_id(program_id: &str) -> Self {
        match program_id {
            RAYDIUM_CPMM_PROGRAM_ID => ProgramKind::RaydiumCpmm,
            RAYDIUM_LEGACY_AMM_PROGRAM_ID => ProgramKind::RaydiumLegacyAmm,
            RAYDIUM_CLMM_PROGRAM_ID => ProgramKind::RaydiumClmm,
            ORCA_WHIRLPOOL_PROGRAM_ID => ProgramKind::OrcaWhirlpool,
            METEORA_DAMM_PROGRAM_ID => ProgramKind::MeteoraDamm,
            METEORA_DLMM_PROGRAM_ID => ProgramKind::MeteoraDlmm,
            PUMP_FUN_PROGRAM_ID => ProgramKind::PumpFun,
            PUMP_FUN_LEGACY_PROGRAM_ID => ProgramKind::PumpFun, // Legacy ID also maps to PumpFun
            _ => ProgramKind::Unknown,
        }
    }
}

/// Internal pool descriptor (not exposed in public API)
#[derive(Debug, Clone)]
pub(crate) struct PoolDescriptor {
    pub pool_id: Pubkey,
    pub program_kind: ProgramKind,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub reserve_accounts: Vec<Pubkey>,
    pub liquidity_usd: f64,
    pub last_updated: Instant,
}

/// Price history ring buffer
#[derive(Debug, Clone)]
pub struct PriceHistory {
    pub mint: String,
    pub prices: VecDeque<PriceResult>,
    pub max_entries: usize,
}

impl PriceHistory {
    pub fn new(mint: String, max_entries: usize) -> Self {
        Self {
            mint,
            prices: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }

    pub fn add_price(&mut self, price: PriceResult) {
        if self.prices.len() >= self.max_entries {
            self.prices.pop_front();
        }
        self.prices.push_back(price);
    }

    pub fn get_latest(&self) -> Option<&PriceResult> {
        self.prices.back()
    }

    pub fn to_vec(&self) -> Vec<PriceResult> {
        self.prices.iter().cloned().collect()
    }
}

/// Configuration constants
pub const PRICE_CACHE_TTL_SECONDS: u64 = 30;
pub const PRICE_HISTORY_MAX_ENTRIES: usize = 1000;
pub const MAX_WATCHED_TOKENS: usize = 100;
pub const POOL_REFRESH_INTERVAL_SECONDS: u64 = 3;

/// SOL mint address (primary trading currency)
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Program ID constants for centralized reference
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
pub const RAYDIUM_LEGACY_AMM_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
pub const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const METEORA_DAMM_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
pub const PUMP_FUN_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";

/// Legacy/deprecated program IDs (still used for transaction analysis)
pub const PUMP_FUN_LEGACY_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
