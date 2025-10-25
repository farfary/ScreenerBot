/// Core types for the pools module
///
/// This file contains all the essential data structures used throughout the pools system.
/// These types are designed to be minimal, efficient, and focused on the core functionality.
use crate::config::with_config;
use crate::constants::{
    FLUXBEAM_AMM_PROGRAM_ID, METEORA_DAMM_PROGRAM_ID, METEORA_DBC_PROGRAM_ID,
    METEORA_DLMM_PROGRAM_ID, MOONIT_AMM_PROGRAM_ID, ORCA_WHIRLPOOL_PROGRAM_ID,
    PUMP_FUN_AMM_PROGRAM_ID, PUMP_FUN_LEGACY_PROGRAM_ID, RAYDIUM_CLMM_PROGRAM_ID,
    RAYDIUM_CPMM_PROGRAM_ID, RAYDIUM_LEGACY_AMM_PROGRAM_ID,
};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::VecDeque;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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
        pool_address: String,
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

    /// Get UTC timestamp for this price result for time series analysis
    pub fn get_utc_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        // Convert Instant to UTC timestamp by calculating the offset from now
        let now_instant = std::time::Instant::now();
        let now_utc = chrono::Utc::now();

        // Calculate how long ago this price was recorded
        let age_duration = now_instant.saturating_duration_since(self.timestamp);

        // Subtract that duration from current UTC time to get the price timestamp
        now_utc - chrono::Duration::from_std(age_duration).unwrap_or(chrono::Duration::zero())
    }

    /// Check if this price result is fresh (within specified age limit)
    pub fn is_fresh(&self, max_age_seconds: u64) -> bool {
        let now = chrono::Utc::now();
        let price_time = self.get_utc_timestamp();
        let age = (now - price_time).num_seconds();
        age >= 0 && age <= (max_age_seconds as i64)
    }

    /// Check if this price result is stale (older than specified limit)
    pub fn is_stale(&self, max_age_seconds: u64) -> bool {
        !self.is_fresh(max_age_seconds)
    }
}

/// Custom serde module for Instant serialization
mod instant_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Convert Instant to unix timestamp (this is an approximation)
        let elapsed = instant.elapsed();
        let now = SystemTime::now();
        let timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs();
        timestamp.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
    where
        D: Deserializer<'de>,
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
    #[error("Pool service initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Pool service not running")]
    ServiceNotRunning,

    #[error("Price not available for token: {0}")]
    PriceNotAvailable(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Decode error: {0}")]
    DecodeError(String),
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
    MeteoraDbc,
    PumpFunAmm,
    PumpFunLegacy,
    Moonit,
    FluxbeamAmm,
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
            ProgramKind::MeteoraDbc => METEORA_DBC_PROGRAM_ID,
            ProgramKind::PumpFunAmm => PUMP_FUN_AMM_PROGRAM_ID,
            ProgramKind::PumpFunLegacy => PUMP_FUN_LEGACY_PROGRAM_ID,
            ProgramKind::Moonit => MOONIT_AMM_PROGRAM_ID,
            ProgramKind::FluxbeamAmm => FLUXBEAM_AMM_PROGRAM_ID,
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
            ProgramKind::MeteoraDbc => "METEORA DBC",
            ProgramKind::PumpFunAmm => "PUMP.FUN AMM",
            ProgramKind::PumpFunLegacy => "PUMP.FUN",
            ProgramKind::Moonit => "MOONIT AMM",
            ProgramKind::FluxbeamAmm => "FLUXBEAM AMM",
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
            METEORA_DBC_PROGRAM_ID => ProgramKind::MeteoraDbc,
            PUMP_FUN_AMM_PROGRAM_ID => ProgramKind::PumpFunAmm,
            PUMP_FUN_LEGACY_PROGRAM_ID => ProgramKind::PumpFunLegacy,
            MOONIT_AMM_PROGRAM_ID => ProgramKind::Moonit,
            FLUXBEAM_AMM_PROGRAM_ID => ProgramKind::FluxbeamAmm,
            _ => ProgramKind::Unknown,
        }
    }

    /// Classify a program id (Pubkey) quickly without allocations
    /// This is a lightweight helper intended for debug / analysis tools to avoid
    /// duplicating the mapping logic scattered across modules.
    pub fn classify(program_pubkey: &solana_sdk::pubkey::Pubkey) -> Self {
        Self::from_program_id(&program_pubkey.to_string())
    }
}

/// Pool descriptor containing metadata about a discovered pool
#[derive(Debug, Clone)]
pub struct PoolDescriptor {
    pub pool_id: Pubkey,
    pub program_kind: ProgramKind,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub reserve_accounts: Vec<Pubkey>,
    pub liquidity_usd: f64,
    pub volume_h24_usd: f64,
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
        // Check for gaps before adding new price
        if let Some(gap_index) = self.detect_gap_before_price(&price) {
            // Remove all data older than the gap
            self.remove_data_before_gap(gap_index);
        }

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

    /// Detect if there's a gap larger than MAX_PRICE_GAP_SECONDS before the new price
    /// Returns the index where the gap starts (all data before this index should be removed)
    fn detect_gap_before_price(&self, new_price: &PriceResult) -> Option<usize> {
        if self.prices.is_empty() {
            return None;
        }

        // Get the timestamp of the new price (convert Instant to approximate unix timestamp)
        let new_timestamp = self.approximate_timestamp(new_price);

        // Check gap from the most recent price
        if let Some(latest_price) = self.prices.back() {
            let latest_timestamp = self.approximate_timestamp(latest_price);

            let time_gap = new_timestamp - latest_timestamp;

            if time_gap > (MAX_PRICE_GAP_SECONDS as i64) {
                // There's a gap - find where continuous data starts from the newest entry
                return self.find_continuous_data_start_index();
            }
        }

        None
    }

    /// Find the starting index of continuous data (without gaps > 1 minute)
    fn find_continuous_data_start_index(&self) -> Option<usize> {
        if self.prices.len() <= 1 {
            return None;
        }

        // Work backwards from the newest data to find where continuous data starts
        for i in (1..self.prices.len()).rev() {
            let current_time = self.approximate_timestamp(&self.prices[i]);
            let prev_time = self.approximate_timestamp(&self.prices[i - 1]);

            let gap = current_time - prev_time;

            if gap > (MAX_PRICE_GAP_SECONDS as i64) {
                // Found a gap - return the index after the gap
                return Some(i);
            }
        }

        None
    }

    /// Remove all data before the specified index (due to detected gap)
    fn remove_data_before_gap(&mut self, gap_index: usize) {
        if gap_index >= self.prices.len() {
            return;
        }

        // Keep only data from gap_index onwards
        let mut new_prices = VecDeque::with_capacity(self.max_entries);
        for i in gap_index..self.prices.len() {
            if let Some(price) = self.prices.get(i) {
                new_prices.push_back(price.clone());
            }
        }

        self.prices = new_prices;
    }

    /// Approximate timestamp from Instant (helper method)
    fn approximate_timestamp(&self, price: &PriceResult) -> i64 {
        // Convert the price's Instant to a unix timestamp by calculating elapsed time
        // This is more accurate than always returning current time
        let now = std::time::SystemTime::now();
        let unix_now = now
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Calculate how long ago this price was created
        let elapsed = price.timestamp.elapsed().as_secs() as i64;

        // Return the approximate timestamp when the price was created
        unix_now - elapsed
    }

    /// Check if current price history has any gaps larger than 1 minute
    pub fn has_significant_gaps(&self) -> bool {
        if self.prices.len() <= 1 {
            return false;
        }

        for i in 1..self.prices.len() {
            let current_time = self.approximate_timestamp(&self.prices[i]);
            let prev_time = self.approximate_timestamp(&self.prices[i - 1]);

            let gap = current_time - prev_time;

            if gap > (MAX_PRICE_GAP_SECONDS as i64) {
                return true;
            }
        }

        false
    }

    /// Remove all data with gaps, keeping only the most recent continuous segment
    pub fn cleanup_gapped_data(&mut self) -> usize {
        let original_len = self.prices.len();

        if let Some(start_index) = self.find_continuous_data_start_index() {
            self.remove_data_before_gap(start_index);
        }

        original_len - self.prices.len()
    }
}

/// Configuration constants
pub const PRICE_HISTORY_MAX_ENTRIES: usize = 1000;
pub const PRICE_CACHE_TTL_SECS: u64 = 30;

/// Price cache TTL sourced from configuration
pub fn price_cache_ttl_seconds() -> u64 {
    PRICE_CACHE_TTL_SECS
}

/// Maximum number of tokens the pool service monitors concurrently
pub fn max_watched_tokens() -> usize {
    crate::config::with_config(|cfg| cfg.pools.max_watched_tokens.max(1))
}

/// Maximum allowable gap between consecutive price updates (1 minute)
/// If gap is larger, older data becomes invalid and should be removed
pub const MAX_PRICE_GAP_SECONDS: u64 = 60;
