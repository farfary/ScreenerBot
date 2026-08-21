//! Core types for the pools module
//!
//! This file contains all the essential data structures used throughout the pools system.
//! These types are designed to be minimal, efficient, and focused on the core functionality.

use crate::chains::{AccountId, AssetId, PoolId};
use crate::config::with_config;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Chain-neutral protocol identity for a pool's DEX/AMM implementation.
///
/// This is the smallest normalized identity `PoolDescriptor` needs for
/// persistence, display and routing — concrete DEX program recognition
/// (Solana program IDs, byte-layout classification) stays owned by the
/// Solana adapter's `ProgramKind` enum (under `chains::solana::pools`),
/// which converts to and from this type at the Solana boundary
/// (`ProgramKind::protocol_id` / `ProgramKind::from_protocol_id`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProtocolId(String);

impl ProtocolId {
    /// Creates a protocol identity from its display/persistence string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Returns the canonical identity string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProtocolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

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
///
/// Since `std::time::Instant` is a monotonic clock without a fixed epoch,
/// we serialize it as the Unix timestamp of when the price was recorded.
/// On deserialization, we reconstruct an approximate Instant by calculating
/// how far in the past the timestamp is from now.
mod instant_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Calculate the Unix timestamp for this instant
        // We do this by finding how long ago the instant was from now,
        // then subtracting that from the current Unix timestamp
        let now_instant = Instant::now();
        let now_system = SystemTime::now();
        let elapsed_since_price = now_instant.saturating_duration_since(*instant);

        // Get current Unix timestamp and subtract the elapsed time
        let current_unix = now_system
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        let price_unix = current_unix.saturating_sub(elapsed_since_price.as_secs());

        price_unix.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored_unix = u64::deserialize(deserializer)?;

        // Calculate how far in the past this timestamp is
        let now_system = SystemTime::now();
        let current_unix = now_system
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();

        // Calculate how many seconds ago this price was recorded
        let seconds_ago = current_unix.saturating_sub(stored_unix);

        // Reconstruct an approximate Instant by subtracting from now
        // Note: Instant::now() - Duration will panic if the result would be before
        // the program started, so we use checked_sub and fallback to now
        let now_instant = Instant::now();
        let approximate_instant = now_instant
            .checked_sub(Duration::from_secs(seconds_ago))
            .unwrap_or(now_instant);

        Ok(approximate_instant)
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

/// Pool descriptor containing metadata about a discovered pool.
///
/// Chain-neutral: identities are typed `chains::types` value objects, not
/// any chain's concrete address type. Chain adapters (e.g.
/// `crate::chains::solana::pools`) construct these through explicit
/// conversions from their own concrete representation; nothing outside a
/// chain adapter should need to parse or format a raw address to work with
/// this struct.
#[derive(Debug, Clone)]
pub struct PoolDescriptor {
    pub pool_id: PoolId,
    pub program_kind: ProtocolId,
    pub base_mint: AssetId,
    pub quote_mint: AssetId,
    pub reserve_accounts: Vec<AccountId>,
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
    /// Create a new price history buffer for the given token mint
    pub fn new(mint: String, max_entries: usize) -> Self {
        Self {
            mint,
            prices: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }

    /// Add a price result, evicting the oldest entry if at capacity
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

    /// Return the most recent price result, if any
    pub fn get_latest(&self) -> Option<&PriceResult> {
        self.prices.back()
    }

    /// Convert the price history to a Vec snapshot
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

/// Price cache TTL sourced from configuration
pub fn price_cache_ttl_seconds() -> u64 {
    crate::config::with_config(|cfg| cfg.pools.price_cache_ttl_secs)
}

/// Account blacklist threshold from configuration
pub fn account_blacklist_threshold() -> u32 {
    crate::config::with_config(|cfg| cfg.pools.account_blacklist_threshold)
}

/// Pool blacklist threshold from configuration
pub fn pool_blacklist_threshold() -> u32 {
    crate::config::with_config(|cfg| cfg.pools.pool_blacklist_threshold)
}

/// Failure window in seconds from configuration
pub fn failure_window_secs() -> u64 {
    crate::config::with_config(|cfg| cfg.pools.failure_window_secs)
}

/// Maximum number of tokens the pool service monitors concurrently
pub fn max_watched_tokens() -> usize {
    crate::config::with_config(|cfg| cfg.pools.max_watched_tokens.max(1))
}

/// Maximum allowable gap between consecutive price updates (1 minute)
/// If gap is larger, older data becomes invalid and should be removed
pub const MAX_PRICE_GAP_SECONDS: u64 = 60;

// ============================================================================
// POOL DATA TYPES
// ============================================================================

/// Cache statistics for the pool price cache
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_prices: usize,
    pub fresh_prices: usize,
    pub history_entries: usize,
}

/// Result of mint and vault analysis for a pool
#[derive(Debug, Clone)]
pub struct TokenPairInfo {
    /// The token mint (non-SOL)
    pub token_mint: String,
    /// The SOL mint (always normalized to wrapped SOL)
    pub sol_mint: String,
    /// Vault address for the token
    pub token_vault: String,
    /// Vault address for SOL
    pub sol_vault: String,
    /// Whether the original pool has SOL as the first mint (affects price calculation)
    pub sol_is_first: bool,
    /// Whether this is a valid SOL-based pair
    pub is_sol_pair: bool,
}

/// Pool mint and vault extraction result
#[derive(Debug, Clone)]
pub struct PoolMintVaultInfo {
    pub mint1: String,
    pub mint2: String,
    pub vault1: String,
    pub vault2: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainId;

    fn sample_descriptor() -> PoolDescriptor {
        PoolDescriptor {
            pool_id: PoolId::new(ChainId::Solana, "PoolAddr111").unwrap(),
            program_kind: ProtocolId::new("RAYDIUM CPMM"),
            base_mint: AssetId::new(ChainId::Solana, "TokenMint111").unwrap(),
            quote_mint: AssetId::new(
                ChainId::Solana,
                "So11111111111111111111111111111111111111112",
            )
            .unwrap(),
            reserve_accounts: vec![
                AccountId::new(ChainId::Solana, "Vault1").unwrap(),
                AccountId::new(ChainId::Solana, "Vault2").unwrap(),
            ],
            liquidity_usd: 12_345.0,
            volume_h24_usd: 6789.0,
            last_updated: Instant::now(),
        }
    }

    #[test]
    fn pool_descriptor_carries_only_chain_neutral_identities() {
        let descriptor = sample_descriptor();
        assert_eq!(descriptor.pool_id.address(), "PoolAddr111");
        assert_eq!(descriptor.pool_id.chain(), ChainId::Solana);
        assert_eq!(descriptor.base_mint.address(), "TokenMint111");
        assert_eq!(descriptor.reserve_accounts.len(), 2);
        assert_eq!(descriptor.reserve_accounts[0].address(), "Vault1");
    }

    #[test]
    fn protocol_id_equality_and_hash_are_string_identity() {
        let a = ProtocolId::new("RAYDIUM CPMM");
        let b = ProtocolId::new("RAYDIUM CPMM");
        let c = ProtocolId::new("ORCA WHIRLPOOL");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.to_string(), "RAYDIUM CPMM");

        let mut set = std::collections::HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
        assert!(!set.contains(&c));
    }

    #[test]
    fn pool_descriptor_pool_id_survives_json_round_trip() {
        let descriptor = sample_descriptor();
        let json = serde_json::to_value(&descriptor.pool_id).unwrap();
        let restored: PoolId = serde_json::from_value(json).unwrap();
        assert_eq!(restored, descriptor.pool_id);
    }
}
