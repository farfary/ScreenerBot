use crate::arguments::is_debug_pool_calculator_enabled;
use crate::global::{ is_debug_pool_prices_enabled, CACHE_POOL_DIR };
/// Pool Price System
///
/// This module provides a comprehensive pool-based price calculation system with caching,
/// background monitoring, and API fallback. It fetches pool data from DexScreener API
/// and calculates prices from pool reserves. Token selection for monitoring now relies
/// on the centralized price service priority list (no internal pool watch list).
use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use crate::tokens::decimals::get_cached_decimals;
use crate::tokens::dexscreener::{ get_token_pairs_from_api, TokenPair };
use crate::tokens::is_system_or_stable_token;
use crate::tokens::pool_db::{
    init_pool_db_service,
    store_price_entry,
    get_price_history_for_token,
};
use crate::utils::safe_truncate;
use chrono::{ DateTime, Utc };
use futures;
use serde::{ Deserialize, Serialize };
use solana_sdk::{ account::Account, commitment_config::CommitmentConfig, pubkey::Pubkey };
use std::collections::{ HashMap, HashSet };
use std::fs;
use std::hash::{ Hash, Hasher };
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Pool cache TTL (10 minutes)
/// Requirement: cache all tokens pools addresses and infos for maximum 10 minutes
const POOL_CACHE_TTL_SECONDS: i64 = 600;

/// Price cache TTL - increased to handle async task delays between pool calculation and entry checks
/// Raising to reduce "expired" misses during heavy scanning; pool monitor keeps it fresh.
const PRICE_CACHE_TTL_SECONDS: i64 = 240; // 4 minutes

// =============================================================================
// BATCH PROCESSING CONFIGURATION
// =============================================================================

/// Priority tokens update interval (5 seconds exactly)
const PRIORITY_UPDATE_INTERVAL_SECS: u64 = 1;

/// Watchlist batch size for random updates
/// Larger batches help rotate through more tokens per second under load
const WATCHLIST_BATCH_SIZE: usize = 150;

/// Watchlist update interval (spread updates over time)
const WATCHLIST_UPDATE_INTERVAL_SECS: u64 = 1; // keep at 1s, rely on bigger batches

/// Maximum tokens per DexScreener API call
const MAX_TOKENS_PER_BATCH: usize = 30;

/// Maximum watchlist size (user requirement)
/// Increased to reduce eviction thrash while trader schedules many tokens
const MAX_WATCHLIST_SIZE: usize = 800;

// Watchlist cleanup policies
/// Remove tokens from watchlist if not accessed for this many hours
const WATCHLIST_EXPIRY_HOURS: i64 = 24;

/// Remove tokens from watchlist after this many consecutive failures
const MAX_CONSECUTIVE_FAILURES: u32 = 5;

/// Run watchlist cleanup every N seconds (during monitoring loop)
const WATCHLIST_CLEANUP_INTERVAL_SECS: u64 = 300; // 5 minutes

// Ad-hoc warm-up batching
const ADHOC_BATCH_SIZE: usize = 300; // how many tokens to warm per tick (increased)
const ADHOC_UPDATE_INTERVAL_SECS: u64 = 1; // batch ad-hoc warms every 1s (faster warms)

/// Minimum liquidity (USD) required to consider a pool usable for price calculation.
/// Lower this for testing environments if you want stats to increment sooner.
pub const MIN_POOL_LIQUIDITY_USD: f64 = 10.0;

// Monitoring concurrency & performance budgeting
const POOL_MONITOR_CONCURRENCY: usize = 64; // Max concurrent token updates per cycle
const POOL_MONITOR_CYCLE_BUDGET_MS: u128 = 6000; // Soft per-cycle time budget
const POOL_MONITOR_PER_TOKEN_TIMEOUT_SECS: u64 = 10; // Guard per token update future

// Pool price history settings (in-memory + database persistence)
const POOL_PRICE_HISTORY_MAX_AGE_HOURS: i64 = 24; // Keep 24 hours of history
const POOL_PRICE_HISTORY_MAX_ENTRIES: usize = 1000; // Max entries per pool cache
const POOL_PRICE_HISTORY_SAVE_INTERVAL_SECONDS: u64 = 300; // 5 minute intervals

/// SOL mint address
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Raydium CPMM Program ID
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

/// Meteora DAMM v2 Program ID
pub const METEORA_DAMM_V2_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";

/// Meteora DLMM Program ID
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

/// Orca Whirlpool Program ID
pub const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

/// Pump.fun AMM Program ID
pub const PUMP_FUN_AMM_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";

/// Raydium Legacy AMM Program ID
pub const RAYDIUM_LEGACY_AMM_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Raydium CLMM (Concentrated Liquidity Market Maker) Program ID
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get display name for pool program ID
pub fn get_pool_program_display_name(program_id: &str) -> String {
    match program_id {
        RAYDIUM_CPMM_PROGRAM_ID => "RAYDIUM CPMM".to_string(),
        RAYDIUM_LEGACY_AMM_PROGRAM_ID => "RAYDIUM LEGACY AMM".to_string(),
        RAYDIUM_CLMM_PROGRAM_ID => "RAYDIUM CLMM".to_string(),
        METEORA_DAMM_V2_PROGRAM_ID => "METEORA DAMM v2".to_string(),
        METEORA_DLMM_PROGRAM_ID => "METEORA DLMM".to_string(),
        ORCA_WHIRLPOOL_PROGRAM_ID => "ORCA WHIRLPOOL".to_string(),
        PUMP_FUN_AMM_PROGRAM_ID => "PUMP.FUN AMM".to_string(),
        _ => format!("UNKNOWN ({})", &program_id[..8]), // Show first 8 chars for unknown programs
    }
}

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Pool price information from on-chain calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPriceInfo {
    pub pool_address: String,
    pub pool_program_id: String,
    pub pool_type: String,
    pub token_mint: String,
    pub price_sol: f64,
    pub token_reserve: u64,
    pub sol_reserve: u64,
    pub token_decimals: u8,
    pub sol_decimals: u8,
}

/// Basic pool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub pool_address: String,
    pub pool_program_id: String,
    pub pool_type: String,
    pub token_0_mint: String,
    pub token_1_mint: String,
    pub token_0_vault: Option<String>,
    pub token_1_vault: Option<String>,
    pub token_0_reserve: u64,
    pub token_1_reserve: u64,
    pub token_0_decimals: u8,
    pub token_1_decimals: u8,
    pub lp_mint: Option<String>,
    pub lp_supply: Option<u64>,
    pub creator: Option<String>,
    pub status: Option<u32>,
    pub liquidity_usd: Option<f64>,
    /// sqrt_price for concentrated liquidity pools (Orca Whirlpool)
    pub sqrt_price: Option<u128>,
}

/// Cacheable pool metadata (addresses and static info) - NO RESERVE VALUES
/// This is what gets cached for 10 minutes per requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetadata {
    pub pool_address: String,
    pub pool_program_id: String,
    pub pool_type: String,
    pub token_0_mint: String,
    pub token_1_mint: String,
    pub token_0_vault: Option<String>,
    pub token_1_vault: Option<String>,
    pub token_0_decimals: u8,
    pub token_1_decimals: u8,
    pub lp_mint: Option<String>,
    pub creator: Option<String>,
    pub status: Option<u32>,
    /// sqrt_price for concentrated liquidity pools (Orca Whirlpool)
    pub sqrt_price: Option<u128>,
}

impl PoolMetadata {
    /// Convert to PoolInfo by adding fresh reserve data
    pub fn with_reserves(
        self,
        token_0_reserve: u64,
        token_1_reserve: u64,
        lp_supply: Option<u64>,
        liquidity_usd: Option<f64>
    ) -> PoolInfo {
        PoolInfo {
            pool_address: self.pool_address,
            pool_program_id: self.pool_program_id,
            pool_type: self.pool_type,
            token_0_mint: self.token_0_mint,
            token_1_mint: self.token_1_mint,
            token_0_vault: self.token_0_vault,
            token_1_vault: self.token_1_vault,
            token_0_reserve,
            token_1_reserve,
            token_0_decimals: self.token_0_decimals,
            token_1_decimals: self.token_1_decimals,
            lp_mint: self.lp_mint,
            lp_supply,
            creator: self.creator,
            status: self.status,
            liquidity_usd,
            sqrt_price: self.sqrt_price,
        }
    }
}

/// Raydium CPMM pool data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaydiumCpmmPoolData {
    pub pool_address: String,
    pub amm_config: String,
    pub pool_creator: String,
    pub token_0_vault: String,
    pub token_1_vault: String,
    pub lp_mint: String,
    pub token_0_mint: String,
    pub token_1_mint: String,
    pub token_0_program: String,
    pub token_1_program: String,
    pub observation_key: String,
    pub auth_bump: u8,
    pub status: u32,
    pub lp_mint_decimals: u8,
    pub mint_0_decimals: u8,
    pub mint_1_decimals: u8,
    pub token_a_reserve: u64,
    pub token_b_reserve: u64,
    pub token_a_decimals: u8,
    pub token_b_decimals: u8,
    pub lp_supply: u64,
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,
    pub fund_fees_token_0: u64,
    pub fund_fees_token_1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
}

/// Cached pool information with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPoolInfo {
    pub pair_address: String,
    pub dex_id: String,
    pub base_token: String,
    pub quote_token: String,
    pub price_native: f64,
    pub price_usd: f64,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub created_at: u64,
    pub cached_at: DateTime<Utc>,
}

impl CachedPoolInfo {
    pub fn from_token_pair(pair: &TokenPair) -> Result<Self, String> {
        let price_native = pair.price_native
            .parse::<f64>()
            .map_err(|e| format!("Invalid price_native: {}", e))?;
        let price_usd = if let Some(usd_str) = &pair.price_usd {
            usd_str.parse::<f64>().map_err(|e| format!("Invalid price_usd: {}", e))?
        } else {
            0.0 // Default to 0.0 if no USD price available
        };

        Ok(Self {
            pair_address: pair.pair_address.clone(),
            dex_id: pair.dex_id.clone(),
            base_token: pair.base_token.address.clone(),
            quote_token: pair.quote_token.address.clone(),
            price_native,
            price_usd,
            liquidity_usd: pair.liquidity
                .as_ref()
                .map(|l| l.usd)
                .unwrap_or(0.0),
            volume_24h: pair.volume.h24.unwrap_or(0.0),
            created_at: pair.pair_created_at.unwrap_or(0), // Default to 0 if not available
            cached_at: Utc::now(),
        })
    }

    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.cached_at;
        age.num_seconds() > POOL_CACHE_TTL_SECONDS
    }
}

/// Pool price calculation result
#[derive(Debug, Clone)]
pub struct PoolPriceResult {
    pub pool_address: String,
    pub dex_id: String,
    pub pool_type: Option<String>, // Actual pool type from decoder (e.g., "RAYDIUM CPMM", "METEORA DAMM v2", "METEORA DLMM")
    pub token_address: String,
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub api_price_sol: Option<f64>, // API price for comparison and fallback
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub source: String, // "pool" or "api"
    pub calculated_at: DateTime<Utc>,
    pub sol_reserve: Option<f64>, // SOL reserve amount in pool
    pub token_reserve: Option<f64>, // Token reserve amount in pool
    pub error: Option<String>, // Error message for failed calculations
}

/// Token availability for pool price calculation
#[derive(Debug, Clone)]
pub struct TokenAvailability {
    pub token_address: String,
    pub has_pools: bool,
    pub best_pool_address: Option<String>,
    pub best_liquidity_usd: f64,
    pub can_calculate_price: bool,
    pub last_checked: DateTime<Utc>,
}

impl TokenAvailability {
    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.last_checked;
        age.num_seconds() > POOL_CACHE_TTL_SECONDS
    }
}

/// Information about a token that failed pool validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidPoolTokenInfo {
    pub token_address: String,
    pub symbol: Option<String>,
    pub error_reason: String,
    pub first_failed: DateTime<Utc>,
    pub last_attempt: DateTime<Utc>,
    pub attempt_count: u32,
}

impl InvalidPoolTokenInfo {
    pub fn new(token_address: String, symbol: Option<String>, error_reason: String) -> Self {
        let now = Utc::now();
        Self {
            token_address,
            symbol,
            error_reason,
            first_failed: now,
            last_attempt: now,
            attempt_count: 1,
        }
    }

    pub fn increment_attempt(&mut self) {
        self.last_attempt = Utc::now();
        self.attempt_count += 1;
    }
}

/// Pool-specific price history entry for persistent caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPriceHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub price_sol: f64,
    pub price_usd: Option<f64>,
    pub reserves_token: Option<f64>,
    pub reserves_sol: Option<f64>,
    pub liquidity_usd: f64,
    pub volume_24h: Option<f64>,
    pub source: String, // "pool", "api", "pool_direct"
}

/// Pool-specific price history cache for a single token-pool pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPriceHistoryCache {
    pub token_mint: String,
    pub pool_address: String,
    pub dex_id: String,
    pub pool_type: Option<String>,
    pub entries: Vec<PoolPriceHistoryEntry>,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl PoolPriceHistoryCache {
    pub fn new(
        token_mint: String,
        pool_address: String,
        dex_id: String,
        pool_type: Option<String>
    ) -> Self {
        let now = Utc::now();
        Self {
            token_mint,
            pool_address,
            dex_id,
            pool_type,
            entries: Vec::new(),
            last_updated: now,
            created_at: now,
        }
    }

    /// Add a new price entry only if the price has changed significantly
    pub fn add_price_if_changed(
        &mut self,
        price_sol: f64,
        price_usd: Option<f64>,
        reserves_token: Option<f64>,
        reserves_sol: Option<f64>,
        liquidity_usd: f64,
        volume_24h: Option<f64>,
        source: String
    ) -> bool {
        // Check if price has changed from the last entry
        if let Some(last_entry) = self.entries.last() {
            // Use appropriate epsilon based on price magnitude for floating point comparison
            let price_diff = (price_sol - last_entry.price_sol).abs();

            // For very small prices (< 0.001 SOL), use absolute difference
            // For larger prices, use relative difference
            let is_significant_change = if last_entry.price_sol < 0.001 {
                // For micro-cap tokens: require at least 0.1% absolute change
                price_diff >= (last_entry.price_sol * 0.001).max(0.000000001) // min 1e-9 change
            } else if last_entry.price_sol != 0.0 {
                // For normal tokens: require 0.05% relative change (looser than before)
                let relative_diff = price_diff / last_entry.price_sol.abs();
                relative_diff >= 0.0005 // 0.05% instead of 0.01%
            } else {
                // If last price was zero, any non-zero price is significant
                price_sol != 0.0
            };

            // Check time since last entry for forced insertion (reduced to every 15 seconds)
            let time_since_last = Utc::now() - last_entry.timestamp;
            let force_by_time = time_since_last.num_seconds() >= 15;

            // Only record if price changed significantly OR if enough time passed
            if !is_significant_change && !force_by_time {
                return false; // Price hasn't changed significantly and not enough time passed
            }
        }

        // Add new entry
        let entry = PoolPriceHistoryEntry {
            timestamp: Utc::now(),
            price_sol,
            price_usd,
            reserves_token,
            reserves_sol,
            liquidity_usd,
            volume_24h,
            source,
        };

        self.entries.push(entry);
        self.last_updated = Utc::now();

        // Clean up old entries and enforce max entries limit
        self.cleanup_old_entries();

        true // Price was recorded
    }

    /// Remove old entries by age and cap total entries
    pub fn cleanup_old_entries(&mut self) {
        let cutoff_time = Utc::now() - chrono::Duration::hours(POOL_PRICE_HISTORY_MAX_AGE_HOURS);
        // Remove entries older than cutoff
        self.entries.retain(|entry| entry.timestamp > cutoff_time);
        // Enforce max entries limit (keep newest entries)
        if self.entries.len() > POOL_PRICE_HISTORY_MAX_ENTRIES {
            let excess = self.entries.len() - POOL_PRICE_HISTORY_MAX_ENTRIES;
            self.entries.drain(0..excess);
        }
    }

    /// Get price history as tuples
    pub fn get_price_history(&self) -> Vec<(DateTime<Utc>, f64)> {
        self.entries
            .iter()
            .map(|entry| (entry.timestamp, entry.price_sol))
            .collect()
    }

    /// Get detailed price history with all data
    pub fn get_detailed_price_history(
        &self
    ) -> Vec<(DateTime<Utc>, f64, Option<f64>, Option<f64>, Option<f64>, f64, Option<f64>)> {
        self.entries
            .iter()
            .map(|entry| {
                (
                    entry.timestamp,
                    entry.price_sol,
                    entry.price_usd,
                    entry.reserves_token,
                    entry.reserves_sol,
                    entry.liquidity_usd,
                    entry.volume_24h,
                )
            })
            .collect()
    }

    /// Check if cache is expired (older than 24 hours)
    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.last_updated;
        age.num_hours() >= 24 // 24 hours
    }
}

/// Token-level aggregated price history cache that combines all pools for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAggregatedPriceHistoryCache {
    pub token_mint: String,
    pub pool_caches: HashMap<String, PoolPriceHistoryCache>, // pool_address -> cache
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl TokenAggregatedPriceHistoryCache {
    pub fn new(token_mint: String) -> Self {
        let now = Utc::now();
        Self {
            token_mint,
            pool_caches: HashMap::new(),
            last_updated: now,
            created_at: now,
        }
    }

    /// Get combined price history from all pools for this token
    pub fn get_combined_price_history(&self) -> Vec<(DateTime<Utc>, f64)> {
        let mut all_entries = Vec::new();

        for pool_cache in self.pool_caches.values() {
            all_entries.extend(pool_cache.get_price_history());
        }

        // Sort by timestamp
        all_entries.sort_by(|a, b| a.0.cmp(&b.0));

        // Remove duplicates and smooth out data if needed
        let mut deduped = Vec::new();
        for (timestamp, price) in all_entries {
            if let Some((last_timestamp, _)) = deduped.last() {
                // Only add if at least 5 seconds apart to avoid spam
                let time_diff = timestamp.signed_duration_since(*last_timestamp);
                if time_diff.num_seconds() >= 5 {
                    deduped.push((timestamp, price));
                }
            } else {
                deduped.push((timestamp, price));
            }
        }

        deduped
    }

    /// Get the best pool address based on most recent activity and liquidity
    pub fn get_best_pool_address(&self) -> Option<String> {
        let mut best_pool = None;
        let mut best_score = 0.0;

        for (pool_address, pool_cache) in &self.pool_caches {
            if pool_cache.entries.is_empty() {
                continue;
            }

            // Score based on recency and entry count
            let last_entry_age = (Utc::now() - pool_cache.last_updated).num_seconds() as f64;
            let recency_score = 1.0 / (1.0 + last_entry_age / 3600.0); // Decay over hours
            let activity_score = pool_cache.entries.len() as f64;
            let liquidity_score = pool_cache.entries
                .last()
                .map(|e| e.liquidity_usd.log10().max(0.0))
                .unwrap_or(0.0);

            let total_score = recency_score * 100.0 + activity_score + liquidity_score;

            if total_score > best_score {
                best_score = total_score;
                best_pool = Some(pool_address.clone());
            }
        }

        best_pool
    }

    /// Add or update a pool cache
    pub fn add_or_update_pool_cache(&mut self, pool_cache: PoolPriceHistoryCache) {
        self.pool_caches.insert(pool_cache.pool_address.clone(), pool_cache);
        self.last_updated = Utc::now();
    }
}

// =============================================================================
// MAIN POOL PRICE SERVICE
// =============================================================================

pub struct PoolPriceService {
    pool_cache: Arc<RwLock<HashMap<String, Vec<CachedPoolInfo>>>>,
    price_cache: Arc<RwLock<HashMap<String, PoolPriceResult>>>,
    availability_cache: Arc<RwLock<HashMap<String, TokenAvailability>>>,
    price_history: Arc<RwLock<HashMap<String, Vec<(DateTime<Utc>, f64)>>>>,
    // New pool-specific memory-based price history cache
    pool_price_history: Arc<RwLock<HashMap<String, TokenAggregatedPriceHistoryCache>>>,
    monitoring_active: Arc<RwLock<bool>>,
    // Enhanced statistics tracking
    stats: Arc<RwLock<PoolServiceStats>>,
    // Track tokens currently being refreshed to deduplicate background updates
    in_flight_updates: Arc<RwLock<HashSet<String>>>,
    // Unsupported program IDs encountered (log once per program)
    unsupported_programs: Arc<RwLock<HashSet<String>>>,
    // Per-token backoff after repeated timeouts
    backoff_state: Arc<RwLock<HashMap<String, BackoffEntry>>>,

    // Token-level blacklist for invalid/unsupported pools
    invalid_pool_tokens: Arc<RwLock<HashMap<String, InvalidPoolTokenInfo>>>,

    // NEW WATCHLIST MANAGEMENT: Background service with batch updates
    /// Priority tokens (positions) - updated every 5 seconds exactly
    priority_tokens: Arc<RwLock<HashSet<String>>>,
    /// Watchlist tokens (20-200 tokens) - randomly updated in batches
    watchlist_tokens: Arc<RwLock<HashSet<String>>>,
    /// Track last update time for watchlist tokens to implement random rotation
    watchlist_last_updated: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    /// Track request counts for watchlist tokens to manage capacity
    watchlist_request_counts: Arc<RwLock<HashMap<String, u64>>>,
    /// Track last access time for each watchlist token (for time-based expiry)
    watchlist_last_accessed: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
    /// Track consecutive failures for each watchlist token (for failure-based removal)
    watchlist_failure_counts: Arc<RwLock<HashMap<String, u32>>>,

    // Ad-hoc, one-shot warm-up requests (not persisted to watchlist)
    ad_hoc_refresh_tokens: Arc<RwLock<HashSet<String>>>,
}

/// Pool service comprehensive statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolServiceStats {
    pub total_price_requests: u64,
    pub successful_calculations: u64,
    pub failed_calculations: u64,
    pub cache_hits: u64,
    pub blockchain_calculations: u64,
    pub api_fallbacks: u64,
    pub tokens_with_price_history: u64,
    pub total_price_history_entries: u64,
    // Runtime monitoring metrics ("chunks" = monitoring cycles)
    pub monitoring_cycles: u64,
    pub last_cycle_tokens: u64,
    pub total_cycle_tokens: u64,
    pub avg_tokens_per_cycle: f64,
    pub last_cycle_duration_ms: f64,
    pub total_cycle_duration_ms: f64,
    pub avg_cycle_duration_ms: f64,
    // Watch list snapshot (sourced from price service)
    pub watch_total: u64,
    pub watch_expired: u64,
    pub watch_never_checked: u64,
    pub last_updated: DateTime<Utc>,
    // Failure classification metrics
    pub vault_timeouts: u64,
    pub unsupported_programs_count: u64,
    pub calc_skips_backoff: u64,
}

impl Default for PoolServiceStats {
    fn default() -> Self {
        Self {
            total_price_requests: 0,
            successful_calculations: 0,
            failed_calculations: 0,
            cache_hits: 0,
            blockchain_calculations: 0,
            api_fallbacks: 0,
            tokens_with_price_history: 0,
            total_price_history_entries: 0,
            monitoring_cycles: 0,
            last_cycle_tokens: 0,
            total_cycle_tokens: 0,
            avg_tokens_per_cycle: 0.0,
            last_cycle_duration_ms: 0.0,
            total_cycle_duration_ms: 0.0,
            avg_cycle_duration_ms: 0.0,
            watch_total: 0,
            watch_expired: 0,
            watch_never_checked: 0,
            last_updated: Utc::now(),
            vault_timeouts: 0,
            unsupported_programs_count: 0,
            calc_skips_backoff: 0,
        }
    }
}

impl PoolServiceStats {
    pub fn get_success_rate(&self) -> f64 {
        if self.total_price_requests == 0 {
            0.0
        } else {
            ((self.successful_calculations as f64) / (self.total_price_requests as f64)) * 100.0
        }
    }

    pub fn get_cache_hit_rate(&self) -> f64 {
        if self.total_price_requests == 0 {
            0.0
        } else {
            ((self.cache_hits as f64) / (self.total_price_requests as f64)) * 100.0
        }
    }

    pub fn record_failure(&mut self) {
        self.total_price_requests += 1;
        self.failed_calculations += 1;
    }

    pub fn record_success(&mut self, cache_hit: bool) {
        self.total_price_requests += 1;
        self.successful_calculations += 1;
        if cache_hit {
            self.cache_hits += 1;
        }
    }
}

/// Pool memory cache statistics for detailed reporting
impl PoolPriceService {
    /// Create new pool price service
    pub fn new() -> Self {
        // Initialize database service for price history persistence
        if let Err(e) = init_pool_db_service() {
            log(
                LogTag::Pool,
                "DB_INIT_ERROR",
                &format!("Failed to initialize pool database: {}", e)
            );
        } else {
            log(LogTag::Pool, "DB_INIT", "✅ Pool database service initialized");
        }

        let service = Self {
            pool_cache: Arc::new(RwLock::new(HashMap::new())),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            availability_cache: Arc::new(RwLock::new(HashMap::new())),
            price_history: Arc::new(RwLock::new(HashMap::new())),
            pool_price_history: Arc::new(RwLock::new(HashMap::new())),
            monitoring_active: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(PoolServiceStats::default())),
            in_flight_updates: Arc::new(RwLock::new(HashSet::new())),
            unsupported_programs: Arc::new(RwLock::new(HashSet::new())),
            backoff_state: Arc::new(RwLock::new(HashMap::new())),
            invalid_pool_tokens: Arc::new(RwLock::new(HashMap::new())),
            // Initialize watchlist management
            priority_tokens: Arc::new(RwLock::new(HashSet::new())),
            watchlist_tokens: Arc::new(RwLock::new(HashSet::new())),
            watchlist_last_updated: Arc::new(RwLock::new(HashMap::new())),
            watchlist_request_counts: Arc::new(RwLock::new(HashMap::new())),
            watchlist_last_accessed: Arc::new(RwLock::new(HashMap::new())),
            watchlist_failure_counts: Arc::new(RwLock::new(HashMap::new())),

            // Ad-hoc warm-up queue
            ad_hoc_refresh_tokens: Arc::new(RwLock::new(HashSet::new())),
        };

        // Load existing price history from database on startup
        tokio::spawn({
            let price_history = service.price_history.clone();
            let pool_price_history = service.pool_price_history.clone();
            async move {
                if
                    let Err(e) = Self::load_price_history_from_db(
                        price_history,
                        pool_price_history
                    ).await
                {
                    log(
                        LogTag::Pool,
                        "DB_LOAD_ERROR",
                        &format!("Failed to load price history from database: {}", e)
                    );
                } else {
                    log(LogTag::Pool, "DB_LOAD", "📊 Price history loaded from database");
                }
            }
        });

        service
    }

    /// Load existing price history from database with gap detection
    async fn load_price_history_from_db(
        price_history: Arc<RwLock<HashMap<String, Vec<(DateTime<Utc>, f64)>>>>,
        pool_price_history: Arc<RwLock<HashMap<String, TokenAggregatedPriceHistoryCache>>>
    ) -> Result<(), String> {
        // Get tokens that have price history in database
        let tokens_with_history = match crate::tokens::pool_db::get_tokens_with_price_history() {
            Ok(tokens) => tokens,
            Err(e) => {
                log(
                    LogTag::Pool,
                    "DB_LOAD_ERROR",
                    &format!("Failed to get tokens with history: {}", e)
                );
                return Err(e);
            }
        };

        let mut loaded_tokens = 0;
        let mut loaded_entries = 0;

        for token_mint in tokens_with_history {
            // Load price history with gap detection
            match get_price_history_for_token(&token_mint) {
                Ok(history) => {
                    if !history.is_empty() {
                        // Update simple price history (keep last 10 entries)
                        {
                            let mut price_hist = price_history.write().await;
                            let mut token_history = history.clone();

                            // Keep only last 10 entries
                            if token_history.len() > 10 {
                                token_history = token_history.into_iter().rev().take(10).collect();
                                token_history.reverse();
                            }

                            price_hist.insert(token_mint.clone(), token_history);
                        }

                        loaded_tokens += 1;
                        loaded_entries += history.len();

                        if is_debug_pool_prices_enabled() {
                            log(
                                LogTag::Pool,
                                "DB_LOAD_TOKEN",
                                &format!(
                                    "📊 Loaded {} price entries for token {}",
                                    history.len(),
                                    &token_mint[..8]
                                )
                            );
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "DB_LOAD_TOKEN_ERROR",
                        &format!("Failed to load history for {}: {}", &token_mint[..8], e)
                    );
                }
            }
        }

        log(
            LogTag::Pool,
            "DB_LOAD_COMPLETE",
            &format!(
                "📊 Loaded {} price entries for {} tokens from database",
                loaded_entries,
                loaded_tokens
            )
        );

        Ok(())
    }

    /// Trigger background refresh of a token price with in-flight deduplication (stale-while-revalidate)
    async fn trigger_background_refresh(&self, token_address: &str) {
        // Try to mark as in-flight; if already in set, skip
        {
            let mut inflight = self.in_flight_updates.write().await;
            if inflight.contains(token_address) {
                return;
            }
            inflight.insert(token_address.to_string());
        }

        let token = token_address.to_string();
        let price_cache = self.price_cache.clone();
        let in_flight_updates = self.in_flight_updates.clone();
        tokio::spawn(async move {
            // Perform fresh calculation; ignore result on error
            if let Ok(fresh) = get_pool_service().calculate_pool_price(&token, None).await {
                // Update cache
                let mut pc = price_cache.write().await;
                pc.insert(token.clone(), fresh);
            }
            // Clear in-flight flag
            let mut inflight = in_flight_updates.write().await;
            inflight.remove(&token);
        });
    }

    /// Internal cleanup helper: remove tokens whose pools infos are all expired
    async fn cleanup_expired_pools_infos_internal(
        pool_cache: &Arc<RwLock<HashMap<String, Vec<CachedPoolInfo>>>>
    ) -> usize {
        let mut cache = pool_cache.write().await;
        let now = Utc::now();
        let mut to_remove: Vec<String> = Vec::new();
        for (mint, pools) in cache.iter() {
            let any_fresh = pools
                .iter()
                .any(|p| (now - p.cached_at).num_seconds() <= POOL_CACHE_TTL_SECONDS);
            if !any_fresh {
                to_remove.push(mint.clone());
            }
        }
        let removed = to_remove.len();
        for mint in to_remove {
            cache.remove(&mint);
        }
        removed
    }

    /// Public cleanup: remove expired entries
    pub async fn cleanup_expired_pools_infos(&self) -> usize {
        Self::cleanup_expired_pools_infos_internal(&self.pool_cache).await
    }

    /// Get tokens with pools infos updated within given seconds (e.g., last 10 min)
    pub async fn get_tokens_with_recent_pools_infos(&self, window_seconds: i64) -> Vec<String> {
        let cache = self.pool_cache.read().await;
        let now = Utc::now();
        cache
            .iter()
            .filter_map(|(mint, pools)| {
                let recent = pools
                    .iter()
                    .any(|p| (now - p.cached_at).num_seconds() <= window_seconds);
                if recent {
                    Some(mint.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Refresh pools infos for tokens that are missing or expired; limit the number processed
    pub async fn refresh_pools_infos_for_tokens(
        &self,
        mints: &[String],
        max_tokens: usize
    ) -> usize {
        let now = Utc::now();
        // Determine which tokens need refresh
        let mut to_refresh: Vec<String> = Vec::new();
        {
            let cache = self.pool_cache.read().await;
            for mint in mints {
                match cache.get(mint) {
                    Some(pools) if !pools.is_empty() => {
                        let fresh = pools
                            .iter()
                            .any(|p| (now - p.cached_at).num_seconds() <= POOL_CACHE_TTL_SECONDS);
                        if !fresh {
                            to_refresh.push(mint.clone());
                        }
                    }
                    _ => to_refresh.push(mint.clone()),
                }
                if to_refresh.len() >= max_tokens {
                    break;
                }
            }
        }

        let mut updated_count = 0usize;
        for mint in to_refresh {
            if self.refresh_pools_infos(&mint).await.is_ok() {
                updated_count += 1;
            }
        }
        updated_count
    }

    /// Start background monitoring service with batch updates
    pub async fn start_monitoring(&self) {
        let mut monitoring_active = self.monitoring_active.write().await;
        if *monitoring_active {
            log(LogTag::Pool, "WARNING", "Pool monitoring already active");
            return;
        }
        *monitoring_active = true;
        drop(monitoring_active);

        log(LogTag::Pool, "START", "Starting pool price monitoring service with batch updates");

        // Emit an immediate state summary so operators don't wait for the first interval tick
        // This is not gated by debug flags and helps confirm wiring on startup
        self.log_state_summary().await;

        // Clone all necessary Arc references for the background task
        let price_cache = self.price_cache.clone();
        let monitoring_active = self.monitoring_active.clone();
        let stats_arc = self.stats.clone();
        let priority_tokens = self.priority_tokens.clone();
        let watchlist_tokens = self.watchlist_tokens.clone();
        let watchlist_last_updated = self.watchlist_last_updated.clone();
        let watchlist_last_accessed = self.watchlist_last_accessed.clone();
        let watchlist_failure_counts = self.watchlist_failure_counts.clone();
        let watchlist_request_counts = self.watchlist_request_counts.clone();
        let ad_hoc_refresh_tokens = self.ad_hoc_refresh_tokens.clone();

        // Start main monitoring loop with batch processing
        tokio::spawn(async move {
            let mut priority_interval = tokio::time::interval(
                Duration::from_secs(PRIORITY_UPDATE_INTERVAL_SECS)
            );
            let mut watchlist_interval = tokio::time::interval(
                Duration::from_secs(WATCHLIST_UPDATE_INTERVAL_SECS)
            );
            let mut summary_interval = tokio::time::interval(Duration::from_secs(30));
            let mut cleanup_interval = tokio::time::interval(Duration::from_secs(3600)); // Cleanup every hour
            let mut watchlist_cleanup_interval = tokio::time::interval(
                Duration::from_secs(WATCHLIST_CLEANUP_INTERVAL_SECS)
            );
            let mut ad_hoc_interval = tokio::time::interval(
                Duration::from_secs(ADHOC_UPDATE_INTERVAL_SECS)
            );
            let mut pool_refresh_interval = tokio::time::interval(Duration::from_secs(300)); // Refresh stale pools every 5 minutes

            loop {
                tokio::select! {
                    // Priority tokens - update every 5 seconds exactly
                    _ = priority_interval.tick() => {
                        // Check if monitoring should continue
                        {
                            let active = monitoring_active.read().await;
                            if !*active {
                                break;
                            }
                        }

                        // Update priority tokens (open positions)
                        let open_positions = match crate::positions::get_open_mints().await {
                            mints => mints,
                        };

                        // Add open positions to priority tokens
                        {
                            let mut priority = priority_tokens.write().await;
                            for token in &open_positions {
                                priority.insert(token.clone());
                            }
                        }

                        // Get current priority tokens for batch update
                        let priority_tokens_list: Vec<String> = {
                            let priority = priority_tokens.read().await;
                            priority.iter().cloned().collect()
                        };

                        if !priority_tokens_list.is_empty() {
                            let _ = Self::batch_update_token_prices(
                                &priority_tokens_list,
                                &price_cache,
                                &stats_arc,
                                "PRIORITY"
                            ).await;
                        }
                    }

                    // Watchlist tokens - random batch updates
                    _ = watchlist_interval.tick() => {
                        // Check if monitoring should continue
                        {
                            let active = monitoring_active.read().await;
                            if !*active {
                                break;
                            }
                        }

                        // Get random batch of watchlist tokens for update
                        let watchlist_batch = Self::get_random_watchlist_batch(
                            &watchlist_tokens,
                            &watchlist_last_updated,
                            WATCHLIST_BATCH_SIZE
                        ).await;

                        if !watchlist_batch.is_empty() {
                            let successful_tokens = Self::batch_update_token_prices(
                                &watchlist_batch,
                                &price_cache,
                                &stats_arc,
                                "WATCHLIST"
                            ).await;

                            // Update last updated times ONLY for successful tokens
                            if !successful_tokens.is_empty() {
                                let mut last_updated = watchlist_last_updated.write().await;
                                let now = Utc::now();
                                for token in successful_tokens {
                                    last_updated.insert(token, now);
                                }
                            }
                        }
                    }

                    // Periodic state summary - every 30 seconds
                    _ = summary_interval.tick() => {
                        // Check if monitoring should continue
                        {
                            let active = monitoring_active.read().await;
                            if !*active {
                                break;
                            }
                        }

                        let service = get_pool_service();
                        service.log_state_summary().await;
                    }

                    // Periodic cleanup - every hour
                    _ = cleanup_interval.tick() => {
                        // Check if monitoring should continue
                        {
                            let active = monitoring_active.read().await;
                            if !*active {
                                break;
                            }
                        }

                        // Note: We can't call self.cleanup_price_history() here because we're in a static context
                        // Instead, we'll spawn the cleanup directly
                        tokio::spawn(async {
                            match crate::tokens::pool_db::cleanup_old_price_entries() {
                                Ok(deleted_count) => {
                                    if deleted_count > 0 {
                                        log(
                                            LogTag::Pool,
                                            "DB_CLEANUP",
                                            &format!("🧹 Periodic cleanup removed {} old database entries", deleted_count),
                                        );
                                    }
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Pool,
                                        "DB_CLEANUP_ERROR",
                                        &format!("Failed to cleanup database entries: {}", e),
                                    );
                                }
                            }
                        });
                    }

                    // Watchlist cleanup - every 5 minutes
                    _ = watchlist_cleanup_interval.tick() => {
                        // Check if monitoring should continue
                        {
                            let active = monitoring_active.read().await;
                            if !*active {
                                break;
                            }
                        }

                        Self::cleanup_watchlist_tokens(
                            &watchlist_tokens,
                            &watchlist_last_accessed,
                            &watchlist_failure_counts,
                            &watchlist_request_counts,
                            &watchlist_last_updated
                        ).await;
                    }

                    // Ad-hoc warm-up processing - every 2 seconds
                    _ = ad_hoc_interval.tick() => {
                        // Check if monitoring should continue
                        {
                            let active = monitoring_active.read().await;
                            if !*active {
                                break;
                            }
                        }

                        // Drain up to ADHOC_BATCH_SIZE tokens from ad-hoc queue
                        let batch: Vec<String> = {
                            let mut set = ad_hoc_refresh_tokens.write().await;
                            if set.is_empty() { Vec::new() } else {
                                let take_n = set.len().min(ADHOC_BATCH_SIZE);
                                let mut out = Vec::with_capacity(take_n);
                                for token in set.iter().take(take_n).cloned().collect::<Vec<_>>() {
                                    out.push(token.clone());
                                    set.remove(&token);
                                }
                                out
                            }
                        };

                        if !batch.is_empty() {
                            let _ = Self::batch_update_token_prices(
                                &batch,
                                &price_cache,
                                &stats_arc,
                                "ADHOC"
                            ).await;
                        }
                    }

                    // Pool refresh task - update stale pools from database every 5 minutes
                    _ = pool_refresh_interval.tick() => {
                        // Check if monitoring should continue
                        {
                            let active = monitoring_active.read().await;
                            if !*active {
                                break;
                            }
                        }

                        // Refresh stale pools from the database
                        tokio::spawn(async {
                            match crate::tokens::pool_db::get_stale_pools(50) { // Refresh up to 50 stale pools per cycle
                                Ok(stale_pools) => {
                                    if !stale_pools.is_empty() {
                                        log(
                                            LogTag::Pool,
                                            "REFRESH_STALE",
                                            &format!("🔄 Refreshing {} stale pools from database", stale_pools.len())
                                        );

                                        // Update stale pools by fetching fresh data from API
                                        for db_pool in stale_pools.iter().take(10) { // Limit to 10 per cycle to avoid rate limits
                                            if let Ok(pairs) = crate::tokens::dexscreener::get_token_pairs_from_api(&db_pool.token_mint).await {
                                                if !pairs.is_empty() {
                                                    if let Err(e) = crate::tokens::pool_db::store_pools_from_dexscreener_response(&pairs) {
                                                        log(
                                                            LogTag::Pool,
                                                            "REFRESH_ERROR",
                                                            &format!("Failed to update stale pool for {}: {}", db_pool.token_mint, e)
                                                        );
                                                    }
                                                }
                                            }
                                            // Add small delay to avoid overwhelming the API
                                            tokio::time::sleep(Duration::from_millis(100)).await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Pool,
                                        "REFRESH_ERROR",
                                        &format!("Failed to get stale pools: {}", e)
                                    );
                                }
                            }

                            // Also cleanup expired pool metadata
                            if let Err(e) = crate::tokens::pool_db::cleanup_expired_pool_metadata() {
                                log(
                                    LogTag::Pool,
                                    "CLEANUP_ERROR",
                                    &format!("Failed to cleanup expired pool metadata: {}", e)
                                );
                            }
                        });
                    }
                }
            }

            log(LogTag::Pool, "STOP", "Pool price monitoring service stopped");
        });
    }

    /// Batch refresh pools from both DexScreener and GeckoTerminal APIs for better coverage
    /// This function fetches pools from both sources concurrently and combines the results
    pub async fn batch_refresh_pools_dual_api(&self, token_addresses: &[String]) -> (usize, usize, usize) {
        if token_addresses.is_empty() {
            return (0, 0, 0);
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "DUAL_API_BATCH_START",
                &format!("🔄 Starting dual API batch refresh for {} tokens", token_addresses.len())
            );
        }

        let mut dexscreener_successful = 0;
        let mut geckoterminal_successful = 0;
        let mut combined_successful = 0;

        // Split tokens into batches of 30 (max for both APIs)
        for chunk in token_addresses.chunks(MAX_TOKENS_PER_BATCH) {
            let chunk_start = Instant::now();

            // Fetch from DexScreener (existing function)
            let dexscreener_task = async {
                let mut successful = 0;
                for token in chunk {
                    if let Ok(pools) = self.fetch_and_cache_pools(token).await {
                        if !pools.is_empty() {
                            successful += 1;
                        }
                    }
                }
                successful
            };

            // Fetch from GeckoTerminal in parallel
            let geckoterminal_task = async {
                let gecko_result = crate::tokens::geckoterminal::get_batch_token_pools_from_geckoterminal(chunk).await;
                
                // Update memory cache with GeckoTerminal results
                let mut pool_cache = self.pool_cache.write().await;
                for (token_address, gecko_pools) in gecko_result.pools {
                    if !gecko_pools.is_empty() {
                        // Convert GeckoTerminal pools to CachedPoolInfo format
                        let cached_pools: Vec<CachedPoolInfo> = gecko_pools.into_iter().map(|gecko_pool| {
                            CachedPoolInfo {
                                pair_address: gecko_pool.pool_address,
                                dex_id: format!("gecko_{}", gecko_pool.dex_id),
                                base_token: gecko_pool.base_token,
                                quote_token: gecko_pool.quote_token,
                                price_native: gecko_pool.price_native,
                                price_usd: gecko_pool.price_usd,
                                liquidity_usd: gecko_pool.liquidity_usd,
                                volume_24h: gecko_pool.volume_24h,
                                created_at: gecko_pool.created_at,
                                cached_at: Utc::now(),
                            }
                        }).collect();

                        // Merge with existing DexScreener pools if any
                        let existing_pools = pool_cache.get(&token_address).cloned().unwrap_or_default();
                        let mut combined_pools = existing_pools;
                        combined_pools.extend(cached_pools);

                        // Sort by liquidity
                        combined_pools.sort_by(|a, b| b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal));

                        pool_cache.insert(token_address, combined_pools);
                    }
                }
                
                gecko_result.successful_tokens
            };

            // Run both APIs concurrently
            let (dx_success, gt_success) = tokio::join!(dexscreener_task, geckoterminal_task);
            
            dexscreener_successful += dx_success;
            geckoterminal_successful += gt_success;

            // Count tokens that got pools from either source
            {
                let pool_cache = self.pool_cache.read().await;
                for token in chunk {
                    if let Some(pools) = pool_cache.get(token) {
                        if !pools.is_empty() {
                            combined_successful += 1;
                        }
                    }
                }
            }

            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "DUAL_API_BATCH_CHUNK",
                    &format!(
                        "🔄 Processed {} tokens in {}ms: DexScreener {}, GeckoTerminal {}, Combined {}",
                        chunk.len(),
                        chunk_start.elapsed().as_millis(),
                        dx_success,
                        gt_success,
                        combined_successful
                    )
                );
            }
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "DUAL_API_BATCH_COMPLETE",
                &format!(
                    "✅ Dual API batch complete: DexScreener {}/{}, GeckoTerminal {}/{}, Combined {}/{}",
                    dexscreener_successful,
                    token_addresses.len(),
                    geckoterminal_successful,
                    token_addresses.len(),
                    combined_successful,
                    token_addresses.len()
                )
            );
        }

        (dexscreener_successful, geckoterminal_successful, combined_successful)
    }

    /// Batch refresh pools from DexScreener, GeckoTerminal, and Raydium APIs for maximum coverage
    /// This function fetches pools from all three sources concurrently and combines the results
    pub async fn batch_refresh_pools_triple_api(&self, token_addresses: &[String]) -> (usize, usize, usize, usize) {
        if token_addresses.is_empty() {
            return (0, 0, 0, 0);
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "TRIPLE_API_BATCH_START",
                &format!("🚀 Starting concurrent triple API batch refresh for {} tokens", token_addresses.len())
            );
        }

        let mut dexscreener_successful = 0;
        let mut geckoterminal_successful = 0;
        let mut raydium_successful = 0;
        let mut combined_successful = 0;

        // Split tokens into batches for optimal performance (larger batches for better concurrency)
        for chunk in token_addresses.chunks(10) {
            let chunk_start = Instant::now();
            
            // Convert chunk to Vec<String> for API calls
            let chunk_vec: Vec<String> = chunk.iter().map(|s| s.clone()).collect();
            
            // Launch ALL THREE API calls concurrently for maximum speed
            let dex_futures: Vec<_> = chunk_vec.iter()
                .map(|token| get_token_pairs_from_api(token))
                .collect();
            
            let gecko_future = crate::tokens::geckoterminal::get_batch_token_pools_from_geckoterminal(&chunk_vec);
            let raydium_future = crate::tokens::raydium::get_batch_token_pools_from_raydium(&chunk_vec);
            
            // Execute all DexScreener calls concurrently
            let dex_future = async {
                let results = futures::future::join_all(dex_futures).await;
                results.into_iter()
                    .zip(chunk_vec.iter())
                    .filter_map(|(result, token)| {
                        result.ok().map(|pairs| (token.clone(), pairs))
                    })
                    .collect::<Vec<(String, Vec<_>)>>()
            };

            // Wait for ALL THREE APIs to complete concurrently
            let (dex_results, gecko_result, raydium_result) = tokio::join!(
                dex_future,
                gecko_future,
                raydium_future
            );

            let mut dx_success = 0;
            let mut gt_success = 0;
            let mut ray_success = 0;

            // Get current pool cache
            let mut pool_cache = self.pool_cache.write().await;

            // Process DexScreener results
            for (token_address, pairs) in dex_results {
                if !pairs.is_empty() {
                    dx_success += 1;
                    
                    let cached_pools: Vec<CachedPoolInfo> = pairs.into_iter().map(|pair| {
                        let price_usd = pair.price_usd
                            .and_then(|p| p.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        let liquidity_usd = pair.liquidity.as_ref().map(|l| l.usd).unwrap_or(0.0);
                        
                        CachedPoolInfo {
                            pair_address: pair.pair_address,
                            dex_id: format!("dx_{}", pair.dex_id),
                            base_token: pair.base_token.address,
                            quote_token: pair.quote_token.address,
                            price_native: pair.price_native.parse().unwrap_or(0.0),
                            price_usd,
                            liquidity_usd,
                            volume_24h: pair.volume.h24.unwrap_or(0.0),
                            created_at: pair.pair_created_at.unwrap_or(0),
                            cached_at: Utc::now(),
                        }
                    }).collect();

                    pool_cache.insert(token_address, cached_pools);
                }
            }
            dexscreener_successful += dx_success;

            // Process GeckoTerminal results
            for (token_address, gecko_pools) in gecko_result.pools {
                if !gecko_pools.is_empty() {
                    gt_success += 1;
                    
                    let cached_pools: Vec<CachedPoolInfo> = gecko_pools.into_iter().map(|gecko_pool| {
                        CachedPoolInfo {
                            pair_address: gecko_pool.pool_address,
                            dex_id: format!("gt_{}", gecko_pool.dex_id),
                            base_token: gecko_pool.base_token,
                            quote_token: gecko_pool.quote_token,
                            price_native: gecko_pool.price_native,
                            price_usd: gecko_pool.price_usd,
                            liquidity_usd: gecko_pool.liquidity_usd,
                            volume_24h: gecko_pool.volume_24h,
                            created_at: gecko_pool.created_at,
                            cached_at: Utc::now(),
                        }
                    }).collect();

                    // Merge with existing pools if any
                    let existing_pools = pool_cache.get(&token_address).cloned().unwrap_or_default();
                    let mut combined_pools = existing_pools;
                    combined_pools.extend(cached_pools);
                    pool_cache.insert(token_address, combined_pools);
                }
            }
            geckoterminal_successful += gt_success;

            // Process Raydium results
            for (token_address, raydium_pools) in raydium_result.pools {
                if !raydium_pools.is_empty() {
                    ray_success += 1;
                    
                    let cached_pools: Vec<CachedPoolInfo> = raydium_pools.into_iter().map(|raydium_pool| {
                        CachedPoolInfo {
                            pair_address: raydium_pool.pool_address,
                            dex_id: format!("ray_{}", raydium_pool.dex_id),
                            base_token: raydium_pool.base_token,
                            quote_token: raydium_pool.quote_token,
                            price_native: raydium_pool.price_native,
                            price_usd: raydium_pool.price_usd,
                            liquidity_usd: raydium_pool.liquidity_usd,
                            volume_24h: raydium_pool.volume_24h,
                            created_at: 0, // Raydium doesn't provide created_at in the same format
                            cached_at: Utc::now(),
                        }
                    }).collect();

                    // Merge with existing pools if any
                    let existing_pools = pool_cache.get(&token_address).cloned().unwrap_or_default();
                    let mut combined_pools = existing_pools;
                    combined_pools.extend(cached_pools);
                    pool_cache.insert(token_address, combined_pools);
                }
            }
            raydium_successful += ray_success;

            // Count tokens that have pools from any source
            for token_address in chunk {
                if pool_cache.get(token_address).map(|pools| !pools.is_empty()).unwrap_or(false) {
                    combined_successful += 1;
                }
            }

            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "TRIPLE_API_BATCH_CHUNK",
                    &format!(
                        "🚀 Processed {} tokens in {}ms: DX {}, GT {}, Ray {}, Combined {}",
                        chunk.len(),
                        chunk_start.elapsed().as_millis(),
                        dexscreener_successful,
                        gt_success,
                        ray_success,
                        combined_successful
                    )
                );
            }

            // Rate limiting between chunks
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "TRIPLE_API_BATCH_COMPLETE",
                &format!(
                    "🚀 Triple API batch complete: DX {}/{}, GT {}/{}, Ray {}/{}, Combined {}/{}",
                    dexscreener_successful,
                    token_addresses.len(),
                    geckoterminal_successful,
                    token_addresses.len(),
                    raydium_successful,
                    token_addresses.len(),
                    combined_successful,
                    token_addresses.len()
                )
            );
        }

        (dexscreener_successful, geckoterminal_successful, raydium_successful, combined_successful)
    }

    /// Batch update token prices using pool calculations (OPTIMIZED)
    async fn batch_update_token_prices(
        tokens: &[String],
        price_cache: &Arc<RwLock<HashMap<String, PoolPriceResult>>>,
        stats_arc: &Arc<RwLock<PoolServiceStats>>,
        batch_type: &str
    ) -> Vec<String> {
        if tokens.is_empty() {
            return Vec::new();
        }

        let start_time = Instant::now();

        if is_debug_pool_calculator_enabled() {
            log(
                LogTag::Pool,
                "BATCH_START",
                &format!(
                    "Starting {} batch update for {} tokens using batch pool calculation",
                    batch_type,
                    tokens.len()
                )
            );
        }

        // Get pool service for collecting pool addresses
        let service = get_pool_service();
        let mut pool_token_pairs = Vec::new();
        let mut tokens_with_pools = 0;

        // Collect pool addresses for all tokens
        for token_address in tokens {
            if let Some(cached_pools) = service.get_cached_pools_infos(token_address).await {
                if let Some(best_pool) = cached_pools.first() {
                    pool_token_pairs.push((best_pool.pair_address.clone(), token_address.clone()));
                    tokens_with_pools += 1;
                }
            } else {
                // Try to fetch pools for tokens that don't have cached data
                if let Ok(pools) = service.fetch_and_cache_pools(token_address).await {
                    if let Some(best_pool) = pools.first() {
                        pool_token_pairs.push((
                            best_pool.pair_address.clone(),
                            token_address.clone(),
                        ));
                        tokens_with_pools += 1;
                    }
                }
            }
        }

        if pool_token_pairs.is_empty() {
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "BATCH_NO_POOLS",
                    &format!("No pools found for {} batch tokens", batch_type)
                );
            }
            return Vec::new();
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "BATCH_POOLS_READY",
                &format!(
                    "{} batch: {} tokens with pools ready for calculation",
                    batch_type,
                    tokens_with_pools
                )
            );
        }

        // Use batch pool price calculation
        let calculator = get_global_pool_price_calculator();
        match calculator.calculate_multiple_token_prices(&pool_token_pairs).await {
            Ok(price_results) => {
                let mut successful_updates = 0;
                let mut successful_tokens: Vec<String> = Vec::new();
                // Defer history writes until after cache lock is released
                let mut history_updates: Vec<
                    (
                        String, // token_address
                        String, // pool_address
                        String, // dex_id
                        Option<String>, // pool_type
                        f64, // price_sol
                        Option<f64>, // price_usd
                        Option<f64>, // reserves_token
                        Option<f64>, // reserves_sol
                        f64, // liquidity_usd
                        Option<f64>, // volume_24h
                        String, // source
                    )
                > = Vec::new();

                // Update price cache for successful calculations
                {
                    let mut cache = price_cache.write().await;
                    // FIXED: Use proper key-based lookup instead of zip with values() which doesn't preserve order
                    for (pool_address, token_address) in pool_token_pairs.iter() {
                        let lookup_key = format!("{}_{}", pool_address, token_address);
                        if let Some(Some(info)) = price_results.get(&lookup_key) {
                            cache.insert(token_address.clone(), PoolPriceResult {
                                pool_address: pool_address.clone(),
                                dex_id: format!("batch_{}", batch_type),
                                pool_type: Some(
                                    get_pool_program_display_name(&info.pool_program_id)
                                ),
                                token_address: token_address.clone(),
                                price_sol: Some(info.price_sol),
                                price_usd: None,
                                api_price_sol: None,
                                liquidity_usd: 0.0, // Would need calculation
                                volume_24h: 0.0,
                                source: "pool_batch".to_string(),
                                calculated_at: chrono::Utc::now(),
                                sol_reserve: Some(
                                    (info.sol_reserve as f64) /
                                        (10_f64).powi(info.sol_decimals as i32)
                                ),
                                token_reserve: Some(
                                    (info.token_reserve as f64) /
                                        (10_f64).powi(info.token_decimals as i32)
                                ),
                                error: None, // No error for successful batch calculation
                            });

                            successful_updates += 1;
                            successful_tokens.push(token_address.clone());

                            // Queue history persistence for this successful update
                            let pool_type = Some(
                                get_pool_program_display_name(&info.pool_program_id)
                            );
                            let reserves_token = Some(
                                (info.token_reserve as f64) /
                                    (10_f64).powi(info.token_decimals as i32)
                            );
                            let reserves_sol = Some(
                                (info.sol_reserve as f64) / (10_f64).powi(info.sol_decimals as i32)
                            );
                            history_updates.push((
                                token_address.clone(),
                                pool_address.clone(),
                                format!("batch_{}", batch_type),
                                pool_type,
                                info.price_sol,
                                None,
                                reserves_token,
                                reserves_sol,
                                0.0,
                                None,
                                "pool_batch".to_string(),
                            ));

                            // Reset failure count on successful update
                            {
                                let mut failure_counts =
                                    service.watchlist_failure_counts.write().await;
                                failure_counts.remove(token_address);
                            }
                        } else {
                            // Track failure for tokens that didn't get price updates
                            {
                                let mut failure_counts =
                                    service.watchlist_failure_counts.write().await;
                                let current_failures = *failure_counts
                                    .get(token_address)
                                    .unwrap_or(&0);
                                failure_counts.insert(token_address.clone(), current_failures + 1);
                            }
                        }
                    }
                }

                // Persist price history outside of cache lock
                if !history_updates.is_empty() {
                    let service_ref = get_pool_service();
                    for (
                        token_address,
                        pool_address,
                        dex_id,
                        pool_type,
                        price_sol,
                        price_usd,
                        reserves_token,
                        reserves_sol,
                        liquidity_usd,
                        volume_24h,
                        source,
                    ) in history_updates {
                        service_ref.add_price_to_pool_history(
                            &token_address,
                            &pool_address,
                            &dex_id,
                            pool_type.clone(),
                            price_sol,
                            price_usd,
                            reserves_token,
                            reserves_sol,
                            liquidity_usd,
                            volume_24h,
                            &source
                        ).await;
                    }
                }

                // Update stats for request outcomes
                {
                    let mut stats = stats_arc.write().await;
                    for _ in 0..successful_updates {
                        stats.record_success(false); // Not from cache
                        stats.blockchain_calculations += 1;
                    }
                    for _ in 0..tokens.len() - successful_updates {
                        stats.record_failure();
                    }
                }

                // Update monitoring cycle metrics (treat each batch as one cycle)
                {
                    let mut stats = stats_arc.write().await;
                    let elapsed_ms = start_time.elapsed().as_millis() as f64;
                    stats.monitoring_cycles = stats.monitoring_cycles.saturating_add(1);
                    stats.last_cycle_tokens = tokens.len() as u64;
                    stats.total_cycle_tokens = stats.total_cycle_tokens.saturating_add(
                        tokens.len() as u64
                    );
                    stats.last_cycle_duration_ms = elapsed_ms;
                    stats.total_cycle_duration_ms += elapsed_ms;
                    if stats.monitoring_cycles > 0 {
                        stats.avg_tokens_per_cycle =
                            (stats.total_cycle_tokens as f64) / (stats.monitoring_cycles as f64);
                        stats.avg_cycle_duration_ms =
                            stats.total_cycle_duration_ms / (stats.monitoring_cycles as f64);
                    }
                }

                if is_debug_pool_calculator_enabled() {
                    log(
                        LogTag::Pool,
                        "BATCH_SUCCESS",
                        &format!(
                            "{} batch completed: {}/{} prices calculated in {:.2}ms using batch pool calculation",
                            batch_type,
                            successful_updates,
                            tokens.len(),
                            start_time.elapsed().as_millis()
                        )
                    );
                }

                return successful_tokens;
            }
            Err(e) => {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "BATCH_ERROR",
                        &format!("{} batch failed: {}", batch_type, e)
                    );
                }

                // Update stats for failures
                {
                    let mut stats = stats_arc.write().await;
                    for _ in 0..tokens.len() {
                        stats.record_failure();
                    }
                }

                // Update monitoring cycle metrics even on failures
                {
                    let mut stats = stats_arc.write().await;
                    let elapsed_ms = start_time.elapsed().as_millis() as f64;
                    stats.monitoring_cycles = stats.monitoring_cycles.saturating_add(1);
                    stats.last_cycle_tokens = tokens.len() as u64;
                    stats.total_cycle_tokens = stats.total_cycle_tokens.saturating_add(
                        tokens.len() as u64
                    );
                    stats.last_cycle_duration_ms = elapsed_ms;
                    stats.total_cycle_duration_ms += elapsed_ms;
                    if stats.monitoring_cycles > 0 {
                        stats.avg_tokens_per_cycle =
                            (stats.total_cycle_tokens as f64) / (stats.monitoring_cycles as f64);
                        stats.avg_cycle_duration_ms =
                            stats.total_cycle_duration_ms / (stats.monitoring_cycles as f64);
                    }
                }
                return Vec::new();
            }
        }
    }

    /// Clean up stale, failed, or irrelevant tokens from watchlist
    /// Implements time-based expiry, failure-based removal, and relevance cleanup
    async fn cleanup_watchlist_tokens(
        watchlist_tokens: &Arc<RwLock<HashSet<String>>>,
        watchlist_last_accessed: &Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
        watchlist_failure_counts: &Arc<RwLock<HashMap<String, u32>>>,
        watchlist_request_counts: &Arc<RwLock<HashMap<String, u64>>>,
        watchlist_last_updated: &Arc<RwLock<HashMap<String, DateTime<Utc>>>>
    ) {
        let now = Utc::now();
        let mut removed_count = 0;
        let mut expired_count = 0;
        let mut failed_count = 0;
        let mut stale_count = 0;

        let tokens_to_remove = {
            let watchlist = watchlist_tokens.read().await;
            let last_accessed = watchlist_last_accessed.read().await;
            let failure_counts = watchlist_failure_counts.read().await;
            let request_counts = watchlist_request_counts.read().await;
            let last_updated = watchlist_last_updated.read().await;

            let mut to_remove = Vec::new();

            for token in watchlist.iter() {
                let mut remove_reason = None;

                // 1. Time-based expiry: Remove if not accessed for WATCHLIST_EXPIRY_HOURS
                if let Some(last_access) = last_accessed.get(token) {
                    let hours_since_access = now.signed_duration_since(*last_access).num_hours();
                    if hours_since_access > WATCHLIST_EXPIRY_HOURS {
                        remove_reason = Some(
                            format!("expired ({} hours since last access)", hours_since_access)
                        );
                        expired_count += 1;
                    }
                } else {
                    // Never accessed - consider it expired if it's been in watchlist for more than expiry time
                    if let Some(added_time) = last_updated.get(token) {
                        let hours_since_added = now.signed_duration_since(*added_time).num_hours();
                        if hours_since_added > WATCHLIST_EXPIRY_HOURS {
                            remove_reason = Some(
                                format!("never accessed in {} hours", hours_since_added)
                            );
                            expired_count += 1;
                        }
                    }
                }

                // 2. Failure-based removal: Remove if consecutive failures exceed threshold
                if remove_reason.is_none() {
                    if let Some(failure_count) = failure_counts.get(token) {
                        if *failure_count >= MAX_CONSECUTIVE_FAILURES {
                            remove_reason = Some(
                                format!("failed {} consecutive times", failure_count)
                            );
                            failed_count += 1;
                        }
                    }
                }

                // 3. Token relevance: Remove tokens with very low activity and no recent requests
                if remove_reason.is_none() {
                    let request_count = request_counts.get(token).unwrap_or(&0);
                    let last_update_age = last_updated
                        .get(token)
                        .map(|t| now.signed_duration_since(*t).num_hours())
                        .unwrap_or(WATCHLIST_EXPIRY_HOURS + 1);

                    // Remove if: very few requests AND no recent activity AND old
                    if *request_count <= 1 && last_update_age > 12 {
                        remove_reason = Some(
                            format!(
                                "low activity ({} requests, {} hours old)",
                                request_count,
                                last_update_age
                            )
                        );
                        stale_count += 1;
                    }
                }

                if let Some(reason) = remove_reason {
                    to_remove.push((token.clone(), reason));
                }
            }

            to_remove
        };

        if !tokens_to_remove.is_empty() {
            // Remove tokens from all tracking structures
            {
                let mut watchlist = watchlist_tokens.write().await;
                let mut last_accessed = watchlist_last_accessed.write().await;
                let mut failure_counts = watchlist_failure_counts.write().await;
                let mut request_counts = watchlist_request_counts.write().await;
                let mut last_updated = watchlist_last_updated.write().await;

                for (token, reason) in &tokens_to_remove {
                    watchlist.remove(token);
                    last_accessed.remove(token);
                    failure_counts.remove(token);
                    request_counts.remove(token);
                    last_updated.remove(token);
                    removed_count += 1;

                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "WATCHLIST_CLEANUP_REMOVE",
                            &format!(
                                "Removed {} from watchlist: {}",
                                safe_truncate(token, 8),
                                reason
                            )
                        );
                    }
                }
            }

            log(
                LogTag::Pool,
                "WATCHLIST_CLEANUP",
                &format!(
                    "🧹 Watchlist cleanup: removed {} tokens (expired: {}, failed: {}, stale: {})",
                    removed_count,
                    expired_count,
                    failed_count,
                    stale_count
                )
            );
        }
    }

    /// Get random batch of watchlist tokens for update (prioritize least recently updated)
    async fn get_random_watchlist_batch(
        watchlist_tokens: &Arc<RwLock<HashSet<String>>>,
        watchlist_last_updated: &Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
        batch_size: usize
    ) -> Vec<String> {
        let watchlist = watchlist_tokens.read().await;
        let last_updated = watchlist_last_updated.read().await;

        if watchlist.is_empty() {
            return Vec::new();
        }

        // Get tokens with their last update times (never updated = highest priority)
        let mut token_priorities: Vec<(String, DateTime<Utc>)> = watchlist
            .iter()
            .map(|token| {
                let last_update = last_updated
                    .get(token)
                    .copied()
                    .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap_or_default());
                (token.clone(), last_update)
            })
            .collect();

        // Sort by last update time (oldest first) and add some randomness
        token_priorities.sort_by_key(|(_token, last_update)| *last_update);

        // Take up to batch_size tokens, preferring least recently updated
        // Add some randomness by taking from the oldest 50% randomly
        let max_random_selection = (token_priorities.len() / 2).max(batch_size);
        let selection_pool = &token_priorities[..max_random_selection.min(token_priorities.len())];

        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let mut selected = selection_pool.to_vec();
        selected.shuffle(&mut thread_rng());

        selected
            .into_iter()
            .take(batch_size)
            .map(|(token, _)| token)
            .collect()
    }

    // =============================================================================
    // WATCHLIST MANAGEMENT FUNCTIONS
    // =============================================================================

    /// Add token to priority list (positions)
    pub async fn add_priority_token(&self, token_address: &str) {
        let mut priority = self.priority_tokens.write().await;
        priority.insert(token_address.to_string());

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "PRIORITY_ADD",
                &format!("Added {} to priority tokens", &token_address[..8])
            );
        }
    }

    /// Remove token from priority list
    pub async fn remove_priority_token(&self, token_address: &str) {
        let mut priority = self.priority_tokens.write().await;
        priority.remove(token_address);

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "PRIORITY_REMOVE",
                &format!("Removed {} from priority tokens", &token_address[..8])
            );
        }
    }

    /// Add token to watchlist
    pub async fn add_watchlist_token(&self, token_address: &str) {
        // Don't add blacklisted tokens to watchlist
        if self.is_in_invalid_pool_blacklist(token_address).await {
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "WATCHLIST_BLACKLIST_SKIP",
                    &format!(
                        "🚫 Skipping blacklisted token {} for watchlist",
                        safe_truncate(token_address, 8)
                    )
                );
            }
            return;
        }

        let mut watchlist = self.watchlist_tokens.write().await;
        let is_new_token = !watchlist.contains(token_address);
        watchlist.insert(token_address.to_string());
        drop(watchlist);

        if is_new_token {
            // Initialize tracking for new token
            let now = Utc::now();

            let mut request_counts = self.watchlist_request_counts.write().await;
            request_counts.insert(token_address.to_string(), 0);
            drop(request_counts);

            let mut last_accessed = self.watchlist_last_accessed.write().await;
            last_accessed.insert(token_address.to_string(), now);
            drop(last_accessed);

            let mut failure_counts = self.watchlist_failure_counts.write().await;
            failure_counts.insert(token_address.to_string(), 0);
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "WATCHLIST_ADD",
                &format!("Added {} to watchlist tokens", &token_address[..8])
            );
        }
    }

    /// Remove token from watchlist
    pub async fn remove_watchlist_token(&self, token_address: &str) {
        let mut watchlist = self.watchlist_tokens.write().await;
        watchlist.remove(token_address);
        drop(watchlist);

        // Remove from all tracking structures
        let mut last_updated = self.watchlist_last_updated.write().await;
        last_updated.remove(token_address);
        drop(last_updated);

        let mut request_counts = self.watchlist_request_counts.write().await;
        request_counts.remove(token_address);
        drop(request_counts);

        let mut last_accessed = self.watchlist_last_accessed.write().await;
        last_accessed.remove(token_address);
        drop(last_accessed);

        let mut failure_counts = self.watchlist_failure_counts.write().await;
        failure_counts.remove(token_address);

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "WATCHLIST_REMOVE",
                &format!("Removed {} from watchlist tokens", &token_address[..8])
            );
        }
    }

    /// Clear all cache entries for a specific token (for cleanup purposes)
    pub async fn clear_token_from_all_caches(&self, token_address: &str) {
        // Remove from priority and watchlist
        self.remove_priority_token(token_address).await;
        self.remove_watchlist_token(token_address).await;

        // Clear pool cache
        let mut pool_cache = self.pool_cache.write().await;
        pool_cache.remove(token_address);
        drop(pool_cache);

        // Clear price cache
        let mut price_cache = self.price_cache.write().await;
        price_cache.remove(token_address);
        drop(price_cache);

        // Clear availability cache
        let mut availability_cache = self.availability_cache.write().await;
        availability_cache.remove(token_address);
        drop(availability_cache);

        // Clear price history
        let mut price_history = self.price_history.write().await;
        price_history.remove(token_address);
        drop(price_history);

        // Clear pool-specific price history
        let mut pool_price_history = self.pool_price_history.write().await;
        pool_price_history.remove(token_address);
        drop(pool_price_history);

        // Remove from invalid pool tokens blacklist
        let mut invalid_pool_tokens = self.invalid_pool_tokens.write().await;
        invalid_pool_tokens.remove(token_address);
        drop(invalid_pool_tokens);

        // Remove from backoff state
        let mut backoff_state = self.backoff_state.write().await;
        backoff_state.remove(token_address);

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "CACHE_CLEAR",
                &format!("Cleared all caches for token {}", safe_truncate(token_address, 8))
            );
        }
    }

    /// Add multiple tokens to watchlist (batch operation)
    pub async fn add_watchlist_tokens(&self, token_addresses: &[String]) {
        let mut watchlist = self.watchlist_tokens.write().await;
        let mut request_counts = self.watchlist_request_counts.write().await;
        let mut actually_added = 0;

        for token_address in token_addresses {
            // Skip if already in watchlist
            if watchlist.contains(token_address) {
                continue;
            }

            // Skip if blacklisted
            if self.is_in_invalid_pool_blacklist(token_address).await {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "BATCH_BLACKLIST_SKIP",
                        &format!(
                            "🚫 Skipping blacklisted token {} in batch add",
                            safe_truncate(token_address, 8)
                        )
                    );
                }
                continue;
            }

            // Check if we need to make room
            if watchlist.len() >= MAX_WATCHLIST_SIZE {
                // Find token with lowest request count (LRU)
                let mut min_count = u64::MAX;
                let mut min_token = String::new();

                for watchlist_token in watchlist.iter() {
                    let count = request_counts.get(watchlist_token).unwrap_or(&0);
                    if *count < min_count {
                        min_count = *count;
                        min_token = watchlist_token.clone();
                    }
                }

                if !min_token.is_empty() {
                    watchlist.remove(&min_token);
                    request_counts.remove(&min_token);

                    log(
                        LogTag::Pool,
                        "WATCHLIST_EVICT",
                        &format!(
                            "Evicted {} with {} requests to make room",
                            safe_truncate(&min_token, 8),
                            min_count
                        )
                    );
                }
            }

            // Add new token
            watchlist.insert(token_address.clone());
            request_counts.insert(token_address.clone(), 0);
            actually_added += 1;
        }

        drop(watchlist);
        drop(request_counts);

        // Initialize tracking for newly added tokens
        if actually_added > 0 {
            let now = Utc::now();
            let mut last_accessed = self.watchlist_last_accessed.write().await;
            let mut failure_counts = self.watchlist_failure_counts.write().await;

            for token_address in token_addresses {
                if !last_accessed.contains_key(token_address) {
                    last_accessed.insert(token_address.clone(), now);
                    failure_counts.insert(token_address.clone(), 0);
                }
            }
        }

        if is_debug_pool_prices_enabled() {
            let watchlist_size = {
                let watchlist = self.watchlist_tokens.read().await;
                watchlist.len()
            };
            log(
                LogTag::Pool,
                "WATCHLIST_BATCH_ADD",
                &format!("Added {} tokens to watchlist (size: {})", actually_added, watchlist_size)
            );
        }
    }

    /// Get current priority tokens
    pub async fn get_priority_tokens(&self) -> Vec<String> {
        let priority = self.priority_tokens.read().await;
        priority.iter().cloned().collect()
    }

    /// Get current watchlist tokens
    pub async fn get_watchlist_tokens(&self) -> Vec<String> {
        let watchlist = self.watchlist_tokens.read().await;
        watchlist.iter().cloned().collect()
    }

    /// Get watchlist status (total, never updated, last update stats)
    pub async fn get_watchlist_status(&self) -> (usize, usize, Option<DateTime<Utc>>) {
        let watchlist = self.watchlist_tokens.read().await;
        let last_updated = self.watchlist_last_updated.read().await;

        let total = watchlist.len();
        let never_updated = watchlist
            .iter()
            .filter(|token| !last_updated.contains_key(*token))
            .count();
        let most_recent_update = last_updated.values().max().copied();

        (total, never_updated, most_recent_update)
    }

    /// Clear priority tokens (for testing/reset)
    pub async fn clear_priority_tokens(&self) {
        let mut priority = self.priority_tokens.write().await;
        let count = priority.len();
        priority.clear();

        log(LogTag::Pool, "PRIORITY_CLEAR", &format!("Cleared {} priority tokens", count));
    }

    /// Clear watchlist tokens (for testing/reset)
    pub async fn clear_watchlist_tokens(&self) {
        let mut watchlist = self.watchlist_tokens.write().await;
        let mut last_updated = self.watchlist_last_updated.write().await;

        let count = watchlist.len();
        watchlist.clear();
        last_updated.clear();

        log(LogTag::Pool, "WATCHLIST_CLEAR", &format!("Cleared {} watchlist tokens", count));
    }

    /// Public wrapper for stats recording (used by monitoring loop to count cache hits / availability failures)
    pub async fn record_stats_event(&self, success: bool, cache_hit: bool, blockchain: bool) {
        self.record_price_request(success, cache_hit, blockchain).await;
    }

    /// Ensure stats reflect monitoring activity.
    /// - Fresh cache hit: count success (cache_hit).
    /// - No fresh cache but availability passes: delegate to full get_pool_price (which records internally).
    /// - Availability fails: record failed attempt (no pools / liquidity gate) once per invocation.
    pub async fn ensure_pool_price_for_monitor(&self, token_address: &str) {
        // Check for fresh cached price
        let mut fresh_cache_hit = false;
        {
            let price_cache = self.price_cache.read().await;
            if let Some(cached_price) = price_cache.get(token_address) {
                let age = Utc::now() - cached_price.calculated_at;
                if age.num_seconds() <= PRICE_CACHE_TTL_SECONDS {
                    fresh_cache_hit = true;
                }
            }
        }
        if fresh_cache_hit {
            self.record_stats_event(true, true, false).await;
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "MONITOR_CACHE_HIT",
                    &format!("🛰️ Monitor cache hit for {}", token_address)
                );
            }
            return;
        }

        // Availability gate
        if !self.check_token_availability(token_address).await {
            self.record_stats_event(false, false, false).await;
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "MONITOR_UNAVAILABLE",
                    &format!("🛰️ Monitor unavailable (liquidity gate) for {}", token_address)
                );
            }
            return;
        }

        // Perform full calculation (stats recorded inside get_pool_price)
        let _ = self.get_pool_price(token_address, None, &PriceOptions::default()).await;
        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "MONITOR_CALC",
                &format!("🛰️ Monitor triggered calc for {}", token_address)
            );
        }
    }

    /// Stop background monitoring service
    pub async fn stop_monitoring(&self) {
        let mut monitoring_active = self.monitoring_active.write().await;
        *monitoring_active = false;
        log(LogTag::Pool, "STOP", "Stopping pool price monitoring service");
    }

    /// Request immediate priority price updates for specific tokens (BATCH OPTIMIZED)
    /// This bypasses normal caching and forces fresh price calculations
    pub async fn request_priority_price_updates(&self, token_addresses: &[String]) -> usize {
        if token_addresses.is_empty() {
            return 0;
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "PRIORITY_UPDATE",
                &format!(
                    "Requesting batch priority price updates for {} tokens",
                    token_addresses.len()
                )
            );
        }

        // First, collect all pool addresses needed for these tokens
        let mut pool_token_pairs = Vec::new();
        let mut tokens_with_pools = 0;

        for token_address in token_addresses {
            // Get cached pools for this token
            if let Some(cached_pools) = self.get_cached_pools_infos(token_address).await {
                if let Some(best_pool) = cached_pools.first() {
                    pool_token_pairs.push((best_pool.pair_address.clone(), token_address.clone()));
                    tokens_with_pools += 1;
                } else {
                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "NO_POOLS",
                            &format!("No cached pools for token {}", &token_address[..8])
                        );
                    }
                }
            } else {
                // Try to fetch pools for this token
                if let Ok(pools) = self.fetch_and_cache_pools(token_address).await {
                    if let Some(best_pool) = pools.first() {
                        pool_token_pairs.push((
                            best_pool.pair_address.clone(),
                            token_address.clone(),
                        ));
                        tokens_with_pools += 1;
                    }
                }
            }
        }

        if pool_token_pairs.is_empty() {
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "PRIORITY_NO_POOLS",
                    &format!(
                        "No pools found for any of the {} priority tokens",
                        token_addresses.len()
                    )
                );
            }
            return 0;
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "PRIORITY_BATCH_READY",
                &format!(
                    "Batch calculating prices for {}/{} tokens with pools",
                    tokens_with_pools,
                    token_addresses.len()
                )
            );
        }

        // Use batch price calculation
        let calculator = get_global_pool_price_calculator();
        let start_time = std::time::Instant::now();

        let successful_updates = match
            tokio::time::timeout(
                Duration::from_secs(30), // Longer timeout for batch operation
                calculator.calculate_multiple_token_prices(&pool_token_pairs)
            ).await
        {
            Ok(Ok(price_results)) => {
                let mut success_count = 0;

                // Update cache and price history for each successful calculation
                // FIXED: Use proper key-based lookup instead of zip with values() which doesn't preserve order
                for (pool_address, token_address) in pool_token_pairs.iter() {
                    let lookup_key = format!("{}_{}", pool_address, token_address);
                    if let Some(Some(info)) = price_results.get(&lookup_key) {
                        // Update price cache
                        {
                            let mut cache = self.price_cache.write().await;
                            cache.insert(token_address.clone(), PoolPriceResult {
                                pool_address: pool_address.clone(),
                                dex_id: "batch_priority".to_string(),
                                pool_type: Some(
                                    get_pool_program_display_name(&info.pool_program_id)
                                ),
                                token_address: token_address.clone(),
                                price_sol: Some(info.price_sol),
                                price_usd: None,
                                api_price_sol: None,
                                liquidity_usd: 0.0, // Would need to calculate from pool data
                                volume_24h: 0.0,
                                source: "pool_batch".to_string(),
                                calculated_at: chrono::Utc::now(),
                                sol_reserve: Some(
                                    (info.sol_reserve as f64) /
                                        (10_f64).powi(info.sol_decimals as i32)
                                ),
                                token_reserve: Some(
                                    (info.token_reserve as f64) /
                                        (10_f64).powi(info.token_decimals as i32)
                                ),
                                error: None, // No error for successful priority batch calculation
                            });
                        }

                        // Add to price history
                        self.add_price_to_pool_history(
                            token_address,
                            pool_address,
                            "batch_priority",
                            Some(get_pool_program_display_name(&info.pool_program_id)),
                            info.price_sol,
                            None,
                            Some(
                                (info.token_reserve as f64) /
                                    (10_f64).powi(info.token_decimals as i32)
                            ),
                            Some(
                                (info.sol_reserve as f64) / (10_f64).powi(info.sol_decimals as i32)
                            ),
                            0.0, // liquidity_usd would need calculation
                            None,
                            "pool_batch"
                        ).await;

                        success_count += 1;

                        if is_debug_pool_prices_enabled() {
                            log(
                                LogTag::Pool,
                                "PRIORITY_BATCH_SUCCESS",
                                &format!(
                                    "Priority batch update successful for {}: {:.9} SOL",
                                    &token_address[..8],
                                    info.price_sol
                                )
                            );
                        }
                    }
                }

                success_count
            }
            Ok(Err(e)) => {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "PRIORITY_BATCH_ERROR",
                        &format!("Priority batch calculation failed: {}", e)
                    );
                }
                0
            }
            Err(_) => {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "PRIORITY_BATCH_TIMEOUT",
                        "Priority batch calculation timed out"
                    );
                }
                0
            }
        };

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "PRIORITY_BATCH_COMPLETE",
                &format!(
                    "Priority batch update completed: {}/{} successful in {:.2}ms",
                    successful_updates,
                    pool_token_pairs.len(),
                    start_time.elapsed().as_millis()
                )
            );
        }

        successful_updates
    }

    /// Check if background monitoring is currently active
    pub async fn is_monitoring_active(&self) -> bool {
        *self.monitoring_active.read().await
    }

    /// Get complete price history combining database and in-memory data (no duplicates)
    pub async fn get_price_history(&self, token_address: &str) -> Vec<(DateTime<Utc>, f64)> {
        use std::collections::HashMap;

        // Collect all price history from different sources
        let mut all_prices: HashMap<i64, f64> = HashMap::new(); // timestamp_millis -> price

        // 1. Get aggregated pool history (comprehensive, from all pools)
        {
            let pool_cache = self.pool_price_history.read().await;
            if let Some(cache) = pool_cache.get(token_address) {
                let hist = cache.get_combined_price_history();
                for (ts, price) in hist {
                    let timestamp_millis = ts.timestamp_millis();
                    all_prices.insert(timestamp_millis, price);
                }
            }
        }

        // 2. Get simple in-memory history (recent points)
        {
            let history = self.price_history.read().await;
            if let Some(simple_history) = history.get(token_address) {
                for (ts, price) in simple_history {
                    let timestamp_millis = ts.timestamp_millis();
                    all_prices.insert(timestamp_millis, *price);
                }
            }
        }

        // 3. Get database history (persistent storage)
        if let Ok(db_history) = get_price_history_for_token(token_address) {
            for (ts, price) in db_history {
                let timestamp_millis = ts.timestamp_millis();
                all_prices.insert(timestamp_millis, price);
            }
        }

        // Convert back to Vec and sort by timestamp
        let mut combined_history: Vec<(DateTime<Utc>, f64)> = all_prices
            .into_iter()
            .map(|(timestamp_millis, price)| {
                let timestamp = DateTime::from_timestamp(
                    timestamp_millis / 1000,
                    ((timestamp_millis % 1000) * 1_000_000) as u32
                ).unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());
                (timestamp, price)
            })
            .collect();

        combined_history.sort_by_key(|(timestamp, _)| *timestamp);

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "PRICE_HISTORY_COMBINED",
                &format!(
                    "� Complete history for {}: {} unique price points from all sources",
                    &token_address[..8],
                    combined_history.len()
                )
            );
        }

        combined_history
    }

    /// Get the best pool for a token based on activity and liquidity
    pub async fn get_best_pool_for_token(&self, token_address: &str) -> Option<String> {
        let cache = self.pool_price_history.read().await;
        if let Some(token_cache) = cache.get(token_address) {
            token_cache.get_best_pool_address()
        } else {
            // No cache available
            None
        }
    }

    /// Add price to pool-specific history (called internally when prices are updated)
    /// Now uses pool-specific disk-based caching with detailed data
    async fn add_price_to_pool_history(
        &self,
        token_address: &str,
        pool_address: &str,
        dex_id: &str,
        pool_type: Option<String>,
        price_sol: f64,
        price_usd: Option<f64>,
        reserves_token: Option<f64>,
        reserves_sol: Option<f64>,
        liquidity_usd: f64,
        volume_24h: Option<f64>,
        source: &str
    ) {
        // Update in-memory price history
        {
            let mut history = self.price_history.write().await;
            let entry = history.entry(token_address.to_string()).or_insert_with(Vec::new);

            entry.push((Utc::now(), price_sol));

            // Keep only last 10 price points for -10% drop detection
            if entry.len() > 10 {
                entry.remove(0);
            }
        }

        // Update pool-specific memory-based price history cache
        {
            let mut pool_cache = self.pool_price_history.write().await;
            let token_cache = pool_cache
                .entry(token_address.to_string())
                .or_insert_with(|| {
                    TokenAggregatedPriceHistoryCache::new(token_address.to_string())
                });

            // Get or create pool-specific cache
            let pool_specific_cache = token_cache.pool_caches
                .entry(pool_address.to_string())
                .or_insert_with(|| {
                    PoolPriceHistoryCache::new(
                        token_address.to_string(),
                        pool_address.to_string(),
                        dex_id.to_string(),
                        pool_type.clone()
                    )
                });

            // Only add if price has changed significantly
            let price_added = pool_specific_cache.add_price_if_changed(
                price_sol,
                price_usd,
                reserves_token,
                reserves_sol,
                liquidity_usd,
                volume_24h,
                source.to_string()
            );

            if price_added {
                token_cache.last_updated = Utc::now();

                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "POOL_PRICE_HISTORY_ADDED",
                        &format!(
                            "💾 Added price {:.9} SOL to pool cache for {}/{} (total entries: {})",
                            price_sol,
                            token_address,
                            pool_address,
                            pool_specific_cache.entries.len()
                        )
                    );
                }

                // Save to database for persistence (async, non-blocking)
                tokio::spawn({
                    let token_address = token_address.to_string();
                    let pool_address = pool_address.to_string();
                    let dex_id = dex_id.to_string();
                    let pool_type_clone = pool_type.clone();
                    let source = source.to_string();

                    async move {
                        if
                            let Err(e) = store_price_entry(
                                &token_address,
                                &pool_address,
                                &dex_id,
                                pool_type_clone,
                                price_sol,
                                price_usd,
                                Some(liquidity_usd),
                                volume_24h,
                                &source
                            )
                        {
                            log(
                                LogTag::Pool,
                                "DB_STORE_ERROR",
                                &format!("Failed to store price to database: {}", e)
                            );
                        } else if is_debug_pool_prices_enabled() {
                            log(
                                LogTag::Pool,
                                "DB_STORE_SUCCESS",
                                &format!(
                                    "💾 Stored price {:.9} SOL to database for {}",
                                    price_sol,
                                    &token_address[..8]
                                )
                            );
                        }
                    }
                });
            }
        }
    }

    /// Clean up old price history entries (both in-memory and database)
    async fn cleanup_price_history(&self) {
        // Clean up in-memory history
        let mut history = self.price_history.write().await;
        let cutoff = Utc::now() - chrono::Duration::hours(1); // Keep 1 hour of history

        for entry in history.values_mut() {
            entry.retain(|(timestamp, _)| *timestamp > cutoff);
        }

        // Remove empty entries
        history.retain(|_, entry| !entry.is_empty());
        drop(history);

        // Clean up database entries (async, non-blocking)
        tokio::spawn(async move {
            match crate::tokens::pool_db::cleanup_old_price_entries() {
                Ok(deleted_count) => {
                    if deleted_count > 0 {
                        log(
                            LogTag::Pool,
                            "DB_CLEANUP",
                            &format!("🧹 Cleaned up {} old database entries", deleted_count)
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "DB_CLEANUP_ERROR",
                        &format!("Failed to cleanup database entries: {}", e)
                    );
                }
            }
        });
    }

    /// Get pool price for a token (main entry point)
    pub async fn get_pool_price(
        &self,
        token_address: &str,
        api_price_sol: Option<f64>,
        options: &PriceOptions
    ) -> Option<PoolPriceResult> {
        // First check if token is in invalid pool blacklist
        if self.is_in_invalid_pool_blacklist(token_address).await {
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "BLACKLIST_SKIP",
                    &format!(
                        "🚫 Skipping blacklisted token {} (invalid pool)",
                        safe_truncate(token_address, 8)
                    )
                );
            }
            return None;
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "PRICE_REQUEST",
                &format!(
                    "🎯 POOL PRICE REQUEST for {}: API_price={:.9} SOL (NO CACHING - ALWAYS FRESH)",
                    token_address,
                    api_price_sol.unwrap_or(0.0)
                )
            );
        }

        let was_cache_hit = false; // Always false since no caching
        let mut was_blockchain = false;

        // Check if token has available pools
        if !self.check_token_availability(token_address).await {
            self.record_price_request(false, was_cache_hit, was_blockchain).await;
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "NO_POOLS",
                    &format!("❌ NO POOLS available for {}", token_address)
                );
            }
            return None;
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "FRESH_CALC_START",
                &format!("🔄 STARTING FRESH CALCULATION for {} - will get REAL-TIME price from blockchain pools", token_address)
            );
        }

        // Calculate pool price
        match self.calculate_pool_price(token_address, api_price_sol).await {
            Ok(pool_result) => {
                let has_price = pool_result.price_sol.is_some();
                was_blockchain = pool_result.source == "pool";
                self.record_price_request(has_price, was_cache_hit, was_blockchain).await;

                if is_debug_pool_prices_enabled() {
                    if let Some(price_sol) = pool_result.price_sol {
                        // Log the pool price calculation
                        if is_debug_pool_calculator_enabled() {
                            log(
                                LogTag::Pool,
                                "FRESH_CALC_SUCCESS",
                                &format!(
                                    "✅ FRESH POOL PRICE calculated for {}: {:.9} SOL from pool {} ({})",
                                    token_address,
                                    price_sol,
                                    pool_result.pool_address,
                                    pool_result.pool_type
                                        .as_ref()
                                        .unwrap_or(&"Unknown Pool".to_string())
                                )
                            );
                        }

                        // Show diff between API and pool price if both available
                        if let Some(api_price_sol) = api_price_sol {
                            let price_diff = price_sol - api_price_sol;
                            let price_diff_percent = if api_price_sol != 0.0 {
                                ((price_sol - api_price_sol) / api_price_sol) * 100.0
                            } else {
                                0.0
                            };

                            log(
                                LogTag::Pool,
                                "PRICE_COMPARISON",
                                &format!(
                                    "💰 PRICE COMPARISON for {}: \
                                     📊 API={:.9} SOL vs 🏊 POOL={:.9} SOL \
                                     📈 Diff={:.9} SOL ({:+.2}%) - Pool: {} ({})",
                                    token_address,
                                    api_price_sol,
                                    price_sol,
                                    price_diff,
                                    price_diff_percent,
                                    pool_result.pool_address,
                                    pool_result.pool_type
                                        .as_ref()
                                        .unwrap_or(&"Unknown Pool".to_string())
                                )
                            );

                            // Flag significant differences
                            if price_diff_percent.abs() > 10.0 {
                                let flag = if price_diff_percent.abs() > 50.0 {
                                    "🚨 CRITICAL"
                                } else {
                                    "⚠️  WARNING"
                                };
                                log(
                                    LogTag::Pool,
                                    "PRICE_DIVERGENCE",
                                    &format!(
                                        "{} PRICE DIVERGENCE for {}: {:.2}% difference detected! \
                                         💧 Liquidity: ${:.2}, 📊 Volume 24h: ${:.2}, 🔄 Source: {}",
                                        flag,
                                        token_address,
                                        price_diff_percent,
                                        pool_result.liquidity_usd,
                                        pool_result.volume_24h,
                                        pool_result.source
                                    )
                                );
                            }
                        } else {
                            log(
                                LogTag::Pool,
                                "POOL_ONLY_PRICE",
                                &format!(
                                    "🏊 POOL-ONLY PRICE for {}: {:.9} SOL (no API price for comparison)",
                                    token_address,
                                    price_sol
                                )
                            );
                        }
                    } else {
                        if is_debug_pool_calculator_enabled() {
                            log(
                                LogTag::Pool,
                                "CALC_NO_PRICE",
                                &format!(
                                    "❌ CALCULATION FAILED: No price could be calculated for {} from pool {}",
                                    token_address,
                                    pool_result.pool_address
                                )
                            );
                        }
                    }
                }

                // FIXED: Store result in main price cache for get_price() to retrieve
                {
                    let mut price_cache = self.price_cache.write().await;
                    price_cache.insert(token_address.to_string(), pool_result.clone());

                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "MONITOR_CACHE_STORED",
                            &format!(
                                "💾 CACHED monitor price for {}: {:.9} SOL from pool {}",
                                token_address,
                                pool_result.price_sol.unwrap_or(0.0),
                                pool_result.pool_address
                            )
                        );
                    }
                }

                // Add price to pool-specific history and manage watch list for -10% drop detection
                if let Some(price_sol) = pool_result.price_sol {
                    if price_sol > 0.0 && price_sol.is_finite() {
                        self.add_price_to_pool_history(
                            token_address,
                            &pool_result.pool_address,
                            &pool_result.dex_id,
                            pool_result.pool_type.clone(),
                            price_sol,
                            pool_result.price_usd,
                            pool_result.token_reserve, // propagate token reserve if present
                            pool_result.sol_reserve, // propagate sol reserve if present
                            pool_result.liquidity_usd,
                            Some(pool_result.volume_24h),
                            &pool_result.source
                        ).await;

                        if is_debug_pool_prices_enabled() {
                            log(
                                LogTag::Pool,
                                "HISTORY_ADD",
                                &format!(
                                    "📈 Added price {:.9} SOL to history for {}",
                                    price_sol,
                                    token_address
                                )
                            );
                        }
                    }
                }

                Some(pool_result)
            }
            Err(e) => {
                self.record_price_request(false, was_cache_hit, was_blockchain).await;
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "CALC_ERROR",
                        &format!("❌ CALCULATION ERROR for {}: {}", token_address, e)
                    );
                }

                // Check if this is an error type that should trigger permanent blacklisting
                // Only blacklist on structural/permanent pool issues, NOT temporary data unavailability
                let should_blacklist =
                    e.contains("Unsupported pool program") ||
                    e.contains("unknown pool program") ||
                    e.contains("decode failed") ||
                    e.contains("Failed to decode pool") ||
                    e.contains("Invalid pool program") ||
                    e.contains("Pool parsing failed");

                if should_blacklist {
                    // Add to permanent blacklist to avoid repeated processing
                    self.add_to_invalid_pool_blacklist(token_address, None, &e).await;
                }

                // Store error result in cache so get_price() can retrieve it with error details
                let error_result = PoolPriceResult {
                    pool_address: "".to_string(), // No pool address for error
                    dex_id: "".to_string(),
                    pool_type: None,
                    token_address: token_address.to_string(),
                    price_sol: None,
                    price_usd: None,
                    api_price_sol,
                    liquidity_usd: 0.0,
                    volume_24h: 0.0,
                    source: "error".to_string(),
                    calculated_at: Utc::now(),
                    sol_reserve: None,
                    token_reserve: None,
                    error: Some(e), // Store the specific error message
                };

                {
                    let mut price_cache = self.price_cache.write().await;
                    price_cache.insert(token_address.to_string(), error_result.clone());

                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "ERROR_CACHE_STORED",
                            &format!(
                                "💾 CACHED error for {}: {}",
                                token_address,
                                error_result.error.as_ref().unwrap()
                            )
                        );
                    }
                }

                Some(error_result)
            }
        }
    }

    /// Check if token has available pools for price calculation
    pub async fn check_token_availability(&self, token_address: &str) -> bool {
        // Check availability cache first
        {
            let availability_cache = self.availability_cache.read().await;
            if let Some(availability) = availability_cache.get(token_address) {
                if !availability.is_expired() {
                    return availability.can_calculate_price;
                }
            }
        }

        // Fetch and cache availability
        match self.fetch_and_cache_pools(token_address).await {
            Ok(pools) => {
                let has_pools = !pools.is_empty();
                let best_pool = pools
                    .iter()
                    .max_by(|a, b| {
                        a.liquidity_usd
                            .partial_cmp(&b.liquidity_usd)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });

                let availability = TokenAvailability {
                    token_address: token_address.to_string(),
                    has_pools,
                    best_pool_address: best_pool.map(|p| p.pair_address.clone()),
                    best_liquidity_usd: best_pool.map(|p| p.liquidity_usd).unwrap_or(0.0),
                    can_calculate_price: has_pools &&
                    best_pool.map(|p| p.liquidity_usd > MIN_POOL_LIQUIDITY_USD).unwrap_or(false),
                    last_checked: Utc::now(),
                };

                {
                    let mut availability_cache = self.availability_cache.write().await;
                    availability_cache.insert(token_address.to_string(), availability.clone());
                }

                if !availability.can_calculate_price && is_debug_pool_prices_enabled() {
                    if let Some(liq) = availability.best_liquidity_usd.into() {
                    }
                    log(
                        LogTag::Pool,
                        "LIQUIDITY_GATE",
                        &format!(
                            "⛔ Liquidity gate: {} best_liquidity=${:.2} < required ${:.2}",
                            token_address,
                            availability.best_liquidity_usd,
                            MIN_POOL_LIQUIDITY_USD
                        )
                    );
                }
                availability.can_calculate_price
            }
            Err(e) => {
                log(
                    LogTag::Pool,
                    "AVAILABILITY_ERROR",
                    &format!("Failed to check availability for {}: {}", token_address, e)
                );
                false
            }
        }
    }

    /// Fetch pools from API and cache them
    async fn fetch_and_cache_pools(
        &self,
        token_address: &str
    ) -> Result<Vec<CachedPoolInfo>, String> {
        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "FETCH_START",
                &format!("🌐 STARTING to fetch pools for {}", token_address)
            );
        }

        // Check memory pool cache first
        {
            let pool_cache = self.pool_cache.read().await;
            if let Some(cached_pools) = pool_cache.get(token_address) {
                if !cached_pools.is_empty() && !cached_pools[0].is_expired() {
                    if is_debug_pool_prices_enabled() {
                        let age = Utc::now() - cached_pools[0].cached_at;
                        log(
                            LogTag::Pool,
                            "FETCH_CACHE_HIT",
                            &format!(
                                "💾 Using CACHED pools for {}: {} pools, age={}s (max={}s)",
                                token_address,
                                cached_pools.len(),
                                age.num_seconds(),
                                POOL_CACHE_TTL_SECONDS
                            )
                        );
                    }
                    return Ok(cached_pools.clone());
                } else if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "FETCH_CACHE_EXPIRED",
                        &format!("⏰ Memory pool cache EXPIRED for {}, checking database cache", token_address)
                    );
                }
            } else if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "FETCH_CACHE_MISS",
                    &format!("❓ No cached pools in memory for {}, checking database cache", token_address)
                );
            }
        }

        // Check database cache for fresh pools
        if
            let Ok(fresh_db_pools) =
                crate::tokens::pool_db::get_fresh_pools_for_token(token_address)
        {
            if !fresh_db_pools.is_empty() {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "FETCH_DB_HIT",
                        &format!(
                            "💾 Found {} fresh pools in database for {}, converting to memory cache",
                            fresh_db_pools.len(),
                            token_address
                        )
                    );
                }

                // Convert database pools to memory cache format
                let mut cached_pools = Vec::new();
                for db_pool in fresh_db_pools {
                    let cached_pool = CachedPoolInfo {
                        pair_address: db_pool.pool_address,
                        dex_id: db_pool.dex_id,
                        base_token: db_pool.base_token_address,
                        quote_token: db_pool.quote_token_address,
                        price_native: db_pool.price_native.unwrap_or(0.0),
                        price_usd: db_pool.price_usd.unwrap_or(0.0),
                        liquidity_usd: db_pool.liquidity_usd.unwrap_or(0.0),
                        volume_24h: db_pool.volume_24h.unwrap_or(0.0),
                        created_at: db_pool.pair_created_at
                            .map(|dt| dt.timestamp() as u64)
                            .unwrap_or(0),
                        cached_at: db_pool.last_updated,
                    };
                    cached_pools.push(cached_pool);
                }

                // Sort by liquidity (highest first)
                cached_pools.sort_by(|a, b| {
                    b.liquidity_usd
                        .partial_cmp(&a.liquidity_usd)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                // Store in memory cache for fast access
                {
                    let mut pool_cache = self.pool_cache.write().await;
                    pool_cache.insert(token_address.to_string(), cached_pools.clone());

                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "FETCH_DB_CACHED",
                            &format!(
                                "💾 Loaded {} pools from database to memory cache for {}",
                                cached_pools.len(),
                                token_address
                            )
                        );
                    }
                }

                return Ok(cached_pools);
            } else if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "FETCH_DB_MISS",
                    &format!("❓ No fresh pools in database for {}, will fetch from API", token_address)
                );
            }
        } else if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "FETCH_DB_ERROR",
                &format!("❌ Error checking database cache for {}, will fetch from API", token_address)
            );
        }

        // Fetch from both APIs for better coverage
        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "FETCH_DUAL_API_START",
                &format!("🔄 Fetching pools from DexScreener + GeckoTerminal for {}", token_address)
            );
        }

        let api_start_time = Utc::now();
        
        // Fetch from DexScreener API
        let (dexscreener_pairs, dexscreener_error) = match get_token_pairs_from_api(token_address).await {
            Ok(pairs) => (pairs, None),
            Err(e) => {
                if is_debug_pool_prices_enabled() {
                    log(LogTag::Pool, "DEXSCREENER_ERROR", &format!("DexScreener API error for {}: {}", token_address, e));
                }
                (Vec::new(), Some(e))
            }
        };

        // Fetch from GeckoTerminal API
        let (geckoterminal_pools, geckoterminal_error) = match crate::tokens::geckoterminal::get_token_pools_from_geckoterminal(token_address).await {
            Ok(pools) => (pools, None),
            Err(e) => {
                if is_debug_pool_prices_enabled() {
                    log(LogTag::Pool, "GECKOTERMINAL_ERROR", &format!("GeckoTerminal API error for {}: {}", token_address, e));
                }
                (Vec::new(), Some(e))
            }
        };

        let api_duration = Utc::now() - api_start_time;

        // Check if both APIs failed
        if dexscreener_pairs.is_empty() && geckoterminal_pools.is_empty() {
            let combined_error = match (dexscreener_error, geckoterminal_error) {
                (Some(dx_err), Some(gt_err)) => format!("Both APIs failed - DexScreener: {}, GeckoTerminal: {}", dx_err, gt_err),
                (Some(dx_err), None) => format!("DexScreener failed: {}, GeckoTerminal returned no pools", dx_err),
                (None, Some(gt_err)) => format!("GeckoTerminal failed: {}, DexScreener returned no pools", gt_err),
                (None, None) => "Both APIs returned no pools".to_string(),
            };
            
            // Handle API timeouts gracefully - this is often normal during shutdown
            if combined_error.contains("timeout") || combined_error.contains("shutting down") {
                log(
                    LogTag::Pool,
                    "INFO",
                    &format!(
                        "API timeout for {} (system may be shutting down): {}",
                        token_address,
                        combined_error
                    )
                );
            } else {
                log(LogTag::Pool, "ERROR", &format!("Dual API error for {}: {}", token_address, combined_error));
            }
            return Err(format!("Failed to fetch pools from APIs: {}", combined_error));
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "FETCH_DUAL_API_COMPLETE",
                &format!(
                    "✅ Dual API fetch complete for {}: DexScreener {} pairs, GeckoTerminal {} pools in {}ms",
                    token_address,
                    dexscreener_pairs.len(),
                    geckoterminal_pools.len(),
                    api_duration.num_milliseconds()
                )
            );
        }

        // Convert and combine pools from both sources
        let mut cached_pools = Vec::new();
        
        // Process DexScreener pools
        for (index, pair) in dexscreener_pairs.iter().enumerate() {
            match CachedPoolInfo::from_token_pair(&pair) {
                Ok(cached_pool) => {
                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "FETCH_PARSE_SUCCESS",
                            &format!(
                                "✅ [DexScreener] Pool #{} for {}: {} ({}, liquidity: ${:.2})",
                                index + 1,
                                token_address,
                                cached_pool.pair_address,
                                cached_pool.dex_id,
                                cached_pool.liquidity_usd
                            )
                        );
                    }
                    cached_pools.push(cached_pool);
                }
                Err(e) => {
                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "FETCH_PARSE_ERROR",
                            &format!(
                                "❌ [DexScreener] Failed to parse pool #{} for {}: {} - Error: {}",
                                index + 1,
                                token_address,
                                pair.pair_address,
                                e
                            )
                        );
                    }
                }
            }
        }

        // Process GeckoTerminal pools (convert to CachedPoolInfo format)
        for (index, gecko_pool) in geckoterminal_pools.iter().enumerate() {
            let cached_pool = CachedPoolInfo {
                pair_address: gecko_pool.pool_address.clone(),
                dex_id: format!("gecko_{}", gecko_pool.dex_id), // Prefix to distinguish from DexScreener
                base_token: gecko_pool.base_token.clone(),
                quote_token: gecko_pool.quote_token.clone(),
                price_native: gecko_pool.price_native,
                price_usd: gecko_pool.price_usd,
                liquidity_usd: gecko_pool.liquidity_usd,
                volume_24h: gecko_pool.volume_24h,
                created_at: gecko_pool.created_at,
                cached_at: Utc::now(),
            };

            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "FETCH_PARSE_SUCCESS",
                    &format!(
                        "✅ [GeckoTerminal] Pool #{} for {}: {} ({}, liquidity: ${:.2})",
                        index + 1,
                        token_address,
                        cached_pool.pair_address,
                        cached_pool.dex_id,
                        cached_pool.liquidity_usd
                    )
                );
            }
            cached_pools.push(cached_pool);
        }

        // Sort by liquidity (highest first)
        cached_pools.sort_by(|a, b| {
            b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
        });

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "FETCH_SORTED",
                &format!(
                    "📊 Sorted {} pools for {} by liquidity (highest first)",
                    cached_pools.len(),
                    token_address
                )
            );

            // Log top 3 pools for debugging
            for (i, pool) in cached_pools.iter().take(3).enumerate() {
                log(
                    LogTag::Pool,
                    "FETCH_TOP_POOLS",
                    &format!(
                        "🏆 Pool #{}: {} ({}, liquidity: ${:.2}, native_price: {:.12})",
                        i + 1,
                        pool.pair_address,
                        pool.dex_id, // Keep API dex_id for debugging pool fetching
                        pool.liquidity_usd,
                        pool.price_native
                    )
                );
            }
        }

        // Cache the results in memory
        {
            let mut pool_cache = self.pool_cache.write().await;
            pool_cache.insert(token_address.to_string(), cached_pools.clone());

            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "FETCH_CACHED",
                    &format!(
                        "💾 CACHED {} pools for {} (TTL={}s) at {}",
                        cached_pools.len(),
                        token_address,
                        POOL_CACHE_TTL_SECONDS,
                        Utc::now().format("%H:%M:%S%.3f")
                    )
                );
            }
        }

        // Store pools in database for persistent caching
        let total_stored = if !dexscreener_pairs.is_empty() || !geckoterminal_pools.is_empty() {
            let mut total_stored = 0;
            
            // Store DexScreener pools
            if !dexscreener_pairs.is_empty() {
                match crate::tokens::pool_db::store_pools_from_dexscreener_response(&dexscreener_pairs) {
                    Ok(stored_count) => {
                        total_stored += stored_count;
                        if stored_count > 0 && is_debug_pool_prices_enabled() {
                            log(
                                LogTag::Pool,
                                "DB_STORED_DEXSCREENER",
                                &format!(
                                    "💾 Stored {} DexScreener pools for {} in database",
                                    stored_count,
                                    token_address
                                )
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "DB_STORE_ERROR",
                            &format!("Failed to store DexScreener pools for {} in database: {}", token_address, e)
                        );
                    }
                }
            }

            // Store GeckoTerminal pools (we'll need to create a similar function for GeckoTerminal format)
            // For now, we'll just cache them in memory since they're already included in cached_pools
            if !geckoterminal_pools.is_empty() && is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "DB_STORED_GECKOTERMINAL",
                    &format!(
                        "💾 Cached {} GeckoTerminal pools for {} in memory (DB storage TODO)",
                        geckoterminal_pools.len(),
                        token_address
                    )
                );
            }

            total_stored
        } else {
            0
        };

        Ok(cached_pools)
    }

    /// Get cached pools infos for a token (if available and not necessarily fresh)
    pub async fn get_cached_pools_infos(&self, token_address: &str) -> Option<Vec<CachedPoolInfo>> {
        let cache = self.pool_cache.read().await;
        cache.get(token_address).cloned()
    }

    /// Force refresh pools infos for a token (honors rate-limits internally)
    pub async fn refresh_pools_infos(
        &self,
        token_address: &str
    ) -> Result<Vec<CachedPoolInfo>, String> {
        self.fetch_and_cache_pools(token_address).await
    }

    /// Calculate pool price for a token
    async fn calculate_pool_price(
        &self,
        token_address: &str,
        api_price_sol: Option<f64>
    ) -> Result<PoolPriceResult, String> {
        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "CALC_START",
                &format!("🔍 STARTING pool price calculation for {}", token_address)
            );
        }

        let pools = self.fetch_and_cache_pools(token_address).await?;

        if pools.is_empty() {
            let error_msg = format!("No pools available for {}", token_address);
            if is_debug_pool_prices_enabled() {
                log(LogTag::Pool, "CALC_NO_POOLS", &format!("❌ {}", error_msg));
            }
            return Err(error_msg);
        }

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "CALC_POOLS_FOUND",
                &format!(
                    "📊 Found {} pools for {}, selecting highest liquidity pool",
                    pools.len(),
                    token_address
                )
            );
        }

        // Use the pool with highest liquidity
        let best_pool = &pools[0];

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "CALC_SELECTED_POOL",
                &format!(
                    "🏆 Selected best pool for {}: {} ({}, liquidity: ${:.2}, volume_24h: ${:.2})",
                    token_address,
                    best_pool.pair_address,
                    best_pool.dex_id, // Keep API dex_id for debugging pool selection
                    best_pool.liquidity_usd,
                    best_pool.volume_24h
                )
            );
        }

        // Calculate REAL price from blockchain pool reserves instead of using API data
        let pool_calculation_result = self.calculate_real_pool_price_from_reserves(
            &best_pool.pair_address,
            token_address
        ).await;

        let (price_sol, actual_pool_type, sol_reserve, token_reserve) = match
            pool_calculation_result
        {
            Ok(Some(pool_price_info)) => {
                if is_debug_pool_calculator_enabled() {
                    log(
                        LogTag::Pool,
                        "CALC_REAL_BLOCKCHAIN",
                        &format!(
                            "✅ REAL BLOCKCHAIN PRICE calculated for {}: {:.12} SOL from reserves in pool {} ({})",
                            token_address,
                            pool_price_info.price_sol,
                            best_pool.pair_address,
                            pool_price_info.pool_type
                        )
                    );
                }
                (
                    Some(pool_price_info.price_sol),
                    Some(pool_price_info.pool_type.clone()),
                    Some(
                        (pool_price_info.sol_reserve as f64) /
                            (10_f64).powi(pool_price_info.sol_decimals as i32)
                    ),
                    Some(
                        (pool_price_info.token_reserve as f64) /
                            (10_f64).powi(pool_price_info.token_decimals as i32)
                    ),
                )
            }
            Ok(None) => {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "CALC_NO_POOL_PRICE",
                        &format!(
                            "❌ POOL calculation returned None for {}: cannot decode pool {} - returning None",
                            token_address,
                            best_pool.pair_address
                        )
                    );
                }
                // Return None - price_service.rs will decide whether to use API fallback
                (None, None, None, None)
            }
            Err(e) => {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "CALC_POOL_ERROR",
                        &format!(
                            "❌ POOL calculation FAILED for {}: {} - returning None (price_service.rs will handle fallback)",
                            token_address,
                            e
                        )
                    );
                }
                // Return None - price_service.rs will decide whether to use API fallback
                (None, None, None, None)
            }
        };

        // If no pool price could be calculated, return error
        let price_sol = match price_sol {
            Some(price) => price,
            None => {
                let error_msg = format!(
                    "Pool calculation failed for {} from pool {}",
                    token_address,
                    best_pool.pair_address
                );
                if is_debug_pool_prices_enabled() {
                    log(LogTag::Pool, "CALC_FAILED", &format!("❌ {}", error_msg));
                }
                return Err(error_msg);
            }
        };

        let calculation_time = Utc::now();
        let result = PoolPriceResult {
            pool_address: best_pool.pair_address.clone(),
            dex_id: best_pool.dex_id.clone(), // Keep for internal tracking, but use pool_type for display
            pool_type: actual_pool_type, // Use actual pool type from decoder, not API dex_id
            token_address: token_address.to_string(),
            price_sol: Some(price_sol),
            price_usd: None, // We don't calculate USD prices from pools - only SOL prices
            api_price_sol, // Include API price for comparison
            liquidity_usd: best_pool.liquidity_usd,
            volume_24h: best_pool.volume_24h,
            source: "pool".to_string(),
            calculated_at: calculation_time,
            sol_reserve,
            token_reserve,
            error: None, // No error for successful pool calculation
        };

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "CALC_COMPLETE",
                &format!(
                    "✅ CALCULATION COMPLETE for {}: price={:.12} SOL, pool={}, calculated_at={}",
                    token_address,
                    price_sol,
                    best_pool.pair_address,
                    result.calculated_at.format("%H:%M:%S%.3f")
                )
            );

            // Log detailed pool info for debugging
            log(
                LogTag::Pool,
                "CALC_POOL_DETAILS",
                &format!(
                    "🔬 POOL DETAILS for {}: \
                     🎯 Pool Address: {}, \
                     🏪 DEX: {}, \
                     💰 Liquidity: ${:.2}, \
                     📊 Volume 24h: ${:.2}, \
                     🪙 Base Token: {}, \
                     💱 Quote Token: {}, \
                     💲 Native Price: {:.12}, \
                     ⏰ Created: {}",
                    token_address,
                    best_pool.pair_address,
                    best_pool.dex_id, // Keep API dex_id for debugging pool details
                    best_pool.liquidity_usd,
                    best_pool.volume_24h,
                    best_pool.base_token,
                    best_pool.quote_token,
                    best_pool.price_native,
                    best_pool.created_at
                )
            );
        }

        Ok(result)
    }

    /// Calculate REAL pool price from blockchain reserves instead of API data
    async fn calculate_real_pool_price_from_reserves(
        &self,
        pool_address: &str,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "REAL_CALC_START",
                &format!(
                    "🔗 STARTING REAL blockchain calculation for pool {} token {}",
                    pool_address,
                    token_mint
                )
            );
        }

        // Reuse global calculator instance to benefit from cached pool & vault data
        let calculator = get_global_pool_price_calculator();
        // (Debug flag is managed elsewhere; no need to recreate calculator repeatedly)

        match calculator.calculate_token_price(pool_address, token_mint).await {
            Ok(Some(pool_price_info)) => {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "REAL_CALC_SUCCESS",
                        &format!(
                            "✅ REAL price from reserves: {:.12} SOL for {} \
                             (sol_reserve: {}, token_reserve: {}, sol_decimals: {}, token_decimals: {})",
                            pool_price_info.price_sol,
                            token_mint,
                            pool_price_info.sol_reserve,
                            pool_price_info.token_reserve,
                            pool_price_info.sol_decimals,
                            pool_price_info.token_decimals
                        )
                    );
                }
                Ok(Some(pool_price_info))
            }
            Ok(None) => {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "REAL_CALC_NONE",
                        &format!(
                            "❓ REAL calculation returned None for pool {} token {}",
                            pool_address,
                            token_mint
                        )
                    );
                }
                Ok(None)
            }
            Err(e) => {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "REAL_CALC_ERROR",
                        &format!(
                            "❌ REAL calculation FAILED for pool {} token {}: {}",
                            pool_address,
                            token_mint,
                            e
                        )
                    );
                }
                Err(e)
            }
        }
    }

    /// Internal method for background price updates
    async fn update_token_price_internal(
        pool_cache: &Arc<RwLock<HashMap<String, Vec<CachedPoolInfo>>>>,
        price_cache: &Arc<RwLock<HashMap<String, PoolPriceResult>>>,
        token_address: &str
    ) -> Result<bool, String> {
        // Check if we have cached pools available for this token
        let has_cached_pools = {
            let pool_cache = pool_cache.read().await;
            if let Some(cached_pools) = pool_cache.get(token_address) {
                !cached_pools.is_empty() && !cached_pools[0].is_expired()
            } else {
                false
            }
        };

        if !has_cached_pools {
            // No pools available - this indicates the token might not be suitable for monitoring
            return Ok(false);
        }

        // Check if we can get a recent price from cache
        let has_recent_price = {
            let price_cache = price_cache.read().await;
            if let Some(cached_price) = price_cache.get(token_address) {
                let age = Utc::now() - cached_price.calculated_at;
                age.num_seconds() <= PRICE_CACHE_TTL_SECONDS && cached_price.price_sol.is_some()
            } else {
                false
            }
        };

        // Return success if we have pools and recent price data
        Ok(has_cached_pools)
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize, usize) {
        let pool_cache = self.pool_cache.read().await;
        let price_cache = self.price_cache.read().await;
        let availability_cache = self.availability_cache.read().await;

        (pool_cache.len(), price_cache.len(), availability_cache.len())
    }

    /// Get enhanced pool service statistics
    pub async fn get_enhanced_stats(&self) -> PoolServiceStats {
        // Update real-time statistics
        let mut stats = self.stats.write().await;
        let price_history = self.price_history.read().await;

        stats.tokens_with_price_history = price_history.len() as u64;
        stats.total_price_history_entries = price_history
            .values()
            .map(|v| v.len() as u64)
            .sum();

        stats.last_updated = Utc::now();

        stats.clone()
    }

    /// Get comprehensive pool statistics including database metrics
    pub async fn get_comprehensive_pool_statistics(&self) -> Result<serde_json::Value, String> {
        let (pool_cache_size, price_cache_size, availability_cache_size) =
            self.get_cache_stats().await;
        let enhanced_stats = self.get_enhanced_stats().await;

        // Get database statistics
        let (total_db_pools, active_db_pools, fresh_db_pools, unique_tokens) =
            crate::tokens::pool_db::get_pool_metadata_statistics()?;

        let (total_price_entries, recent_price_entries, unique_price_tokens) =
            crate::tokens::pool_db::get_pool_db_statistics()?;

        // Get watchlist statistics
        let priority_count = { self.priority_tokens.read().await.len() };
        let watchlist_count = { self.watchlist_tokens.read().await.len() };

        let stats =
            serde_json::json!({
            "memory_cache": {
                "pool_cache_tokens": pool_cache_size,
                "price_cache_tokens": price_cache_size,
                "availability_cache_tokens": availability_cache_size
            },
            "database_cache": {
                "total_pools": total_db_pools,
                "active_pools": active_db_pools,
                "fresh_pools": fresh_db_pools,
                "unique_tokens_with_pools": unique_tokens,
                "total_price_entries": total_price_entries,
                "recent_price_entries": recent_price_entries,
                "unique_tokens_with_prices": unique_price_tokens
            },
            "monitoring": {
                "priority_tokens": priority_count,
                "watchlist_tokens": watchlist_count,
                "monitoring_active": self.is_monitoring_active().await
            },
            "performance": {
                "total_price_requests": enhanced_stats.total_price_requests,
                "successful_calculations": enhanced_stats.successful_calculations,
                "failed_calculations": enhanced_stats.failed_calculations,
                "cache_hits": enhanced_stats.cache_hits,
                "blockchain_calculations": enhanced_stats.blockchain_calculations,
                "api_fallbacks": enhanced_stats.api_fallbacks,
                "success_rate": enhanced_stats.get_success_rate(),
                "cache_hit_rate": enhanced_stats.get_cache_hit_rate()
            },
            "monitoring_cycles": {
                "total_cycles": enhanced_stats.monitoring_cycles,
                "last_cycle_tokens": enhanced_stats.last_cycle_tokens,
                "avg_tokens_per_cycle": enhanced_stats.avg_tokens_per_cycle,
                "last_cycle_duration_ms": enhanced_stats.last_cycle_duration_ms,
                "avg_cycle_duration_ms": enhanced_stats.avg_cycle_duration_ms
            },
            "errors": {
                "vault_timeouts": enhanced_stats.vault_timeouts,
                "unsupported_programs": enhanced_stats.unsupported_programs_count,
                "backoff_skips": enhanced_stats.calc_skips_backoff
            },
            "last_updated": enhanced_stats.last_updated
        });

        Ok(stats)
    }

    /// Record an unsupported pool program encountered during price requests
    pub async fn record_unsupported_program(&self, program_id: &str) {
        {
            let mut set = self.unsupported_programs.write().await;
            if set.insert(program_id.to_string()) {
                let mut stats = self.stats.write().await;
                stats.unsupported_programs_count += 1;
            }
        }
    }

    /// Add a token to the invalid pool blacklist
    pub async fn add_to_invalid_pool_blacklist(
        &self,
        token_address: &str,
        symbol: Option<&str>,
        error_reason: &str
    ) {
        let mut blacklist = self.invalid_pool_tokens.write().await;
        if let Some(existing) = blacklist.get_mut(token_address) {
            existing.increment_attempt();
        } else {
            let info = InvalidPoolTokenInfo::new(
                token_address.to_string(),
                symbol.map(|s| s.to_string()),
                error_reason.to_string()
            );
            blacklist.insert(token_address.to_string(), info);

            log(
                LogTag::Pool,
                "INVALID_POOL_BLACKLIST_ADD",
                &format!(
                    "🚫 Added {} to invalid pool blacklist: {}",
                    safe_truncate(token_address, 8),
                    error_reason
                )
            );
        }
    }

    /// Check if a token is in the invalid pool blacklist
    pub async fn is_in_invalid_pool_blacklist(&self, token_address: &str) -> bool {
        let blacklist = self.invalid_pool_tokens.read().await;
        blacklist.contains_key(token_address)
    }

    /// Get invalid pool blacklist stats for summary
    pub async fn get_invalid_pool_blacklist_stats(&self) -> (usize, Vec<String>) {
        let blacklist = self.invalid_pool_tokens.read().await;
        let count = blacklist.len();
        let recent_errors: Vec<String> = blacklist
            .values()
            .take(5) // Show up to 5 recent errors in summary
            .map(|info| {
                format!(
                    "{}:{}",
                    safe_truncate(&info.token_address, 8),
                    &info.error_reason[..std::cmp::min(20, info.error_reason.len())]
                )
            })
            .collect();
        (count, recent_errors)
    }

    /// Clear the invalid pool blacklist (for testing/reset)
    pub async fn clear_invalid_pool_blacklist(&self) {
        let mut blacklist = self.invalid_pool_tokens.write().await;
        let count = blacklist.len();
        blacklist.clear();

        if count > 0 {
            log(
                LogTag::Pool,
                "INVALID_POOL_BLACKLIST_CLEAR",
                &format!("🗑️ Cleared {} tokens from invalid pool blacklist", count)
            );
        }
    }

    /// Emit a single consolidated state summary log (not gated by debug)
    pub async fn log_state_summary(&self) {
        // Gather cache sizes
        let (pool_cache_len, price_cache_len, availability_cache_len) =
            self.get_cache_stats().await;

        // Gather watchlist/priority sizes and never updated
        let (watch_total, watch_never_updated, _last_update) = self.get_watchlist_status().await;
        let priority_size = {
            let p = self.priority_tokens.read().await;
            p.len()
        };

        // Gather stats snapshot
        let stats = self.get_enhanced_stats().await;

        // Unsupported programs list
        let unsupported_list: Vec<String> = {
            let set = self.unsupported_programs.read().await;
            set.iter().cloned().collect()
        };

        // Invalid pool blacklist stats
        let (blacklist_count, blacklist_examples) = self.get_invalid_pool_blacklist_stats().await;

        // Calculate rates and performance metrics
        let success_rate = if stats.total_price_requests > 0 {
            ((stats.successful_calculations as f64) * 100.0) / (stats.total_price_requests as f64)
        } else {
            0.0
        };
        let cache_hit_rate = if stats.total_price_requests > 0 {
            ((stats.cache_hits as f64) * 100.0) / (stats.total_price_requests as f64)
        } else {
            0.0
        };

        let total_cache_entries = pool_cache_len + price_cache_len + availability_cache_len;
        let total_failed = stats.failed_calculations;
        let never_checked_pct = if watch_total > 0 {
            ((watch_never_updated as f64) / (watch_total as f64)) * 100.0
        } else {
            0.0
        };

        // Create performance indicators
        let performance_emoji = if success_rate >= 90.0 {
            "🟢"
        } else if success_rate >= 70.0 {
            "🟡"
        } else {
            "🔴"
        };
        let cache_emoji = if cache_hit_rate >= 50.0 {
            "⚡"
        } else if cache_hit_rate >= 20.0 {
            "🔋"
        } else {
            "💾"
        };

        let blacklist_status = if blacklist_count > 0 {
            format!("⚠️  {} invalid pools filtered", blacklist_count)
        } else {
            "✅ No invalid pools detected".to_string()
        };

        let programs_status = if unsupported_list.is_empty() {
            "✅ All pool programs supported".to_string()
        } else {
            format!(
                "🚫 {} unsupported programs: {}",
                unsupported_list.len(),
                unsupported_list.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
            )
        };

        // Single comprehensive log call with all information
        log(
            LogTag::Pool,
            "SUMMARY",
            &format!(
                "═══════════════════════════════════════════════════════════════\n\
            🏊 POOL SERVICE STATE - Comprehensive Price & Cache System\n\
            ═══════════════════════════════════════════════════════════════\n\
            📊 Cycle #{:<3} | Active Tokens: {} | Total Cache: {} entries\n\
            {} PERFORMANCE: {:.1}% success | {} Cache Hit: {:.1}%\n\
            \n\
            💾 MEMORY CACHE BREAKDOWN ({} total entries):\n\
            🔸 Pools: {} cached | Prices: {} cached | Availability: {} tokens\n\
            🔸 History: {} tokens tracked | {} price history entries\n\
            \n\
            📈 REQUEST STATISTICS (Lifecycle totals):\n\
            🔸 Total Requests: {} | ✅ Success: {} | ❌ Failed: {}\n\
            🔸 💻 Blockchain Direct: {} | 🌐 API Fallbacks: {} | {} Cache Hits: {}\n\
            \n\
            ⚙️  MONITORING PERFORMANCE:\n\
            🔸 Cycles: {} | Last Tokens: {} | Avg/Cycle: {:.1} tokens\n\
            🔸 Timing: Last {:.1}ms | Avg {:.1}ms | Total {:.1}ms\n\
            \n\
            📡 WATCHLIST STATUS:\n\
            🔸 Total Watched: {} | 🎯 Priority: {} | ⏸️  Never Updated: {} ({:.1}%)\n\
            \n\
            🛡️  SECURITY & FILTERING:\n\
            🔸 {}\n\
            🔸 {}\n\
            ═══════════════════════════════════════════════════════════════",
                stats.monitoring_cycles,
                watch_total,
                total_cache_entries,
                performance_emoji,
                success_rate,
                cache_emoji,
                cache_hit_rate,
                total_cache_entries,
                pool_cache_len,
                price_cache_len,
                availability_cache_len,
                stats.tokens_with_price_history,
                stats.total_price_history_entries,
                stats.total_price_requests,
                stats.successful_calculations,
                total_failed,
                stats.blockchain_calculations,
                stats.api_fallbacks,
                cache_emoji,
                stats.cache_hits,
                stats.monitoring_cycles,
                stats.last_cycle_tokens,
                stats.avg_tokens_per_cycle,
                stats.last_cycle_duration_ms,
                stats.avg_cycle_duration_ms,
                stats.total_cycle_duration_ms,
                watch_total,
                priority_size,
                watch_never_updated,
                never_checked_pct,
                blacklist_status,
                programs_status
            )
        );
    }

    /// Record a price request (internal tracking)
    async fn record_price_request(&self, success: bool, was_cache_hit: bool, was_blockchain: bool) {
        let mut stats = self.stats.write().await;
        stats.total_price_requests += 1;

        if success {
            stats.successful_calculations += 1;
        } else {
            stats.failed_calculations += 1;
        }

        if was_cache_hit {
            stats.cache_hits += 1;
        }

        if was_blockchain {
            stats.blockchain_calculations += 1;
        } else if success {
            stats.api_fallbacks += 1;
        }
    }

    /// Calculate price directly from a specific pool address (bypasses API discovery)
    /// This function provides direct blockchain decoding of any pool program
    pub async fn get_pool_price_direct(
        &self,
        pool_address: &str,
        token_mint: &str,
        api_price_sol: Option<f64>
    ) -> Option<PoolPriceResult> {
        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "DIRECT_CALC_START",
                &format!(
                    "🎯 DIRECT pool calculation for pool {} token {} (bypassing API discovery)",
                    pool_address,
                    token_mint
                )
            );
        }

        // Calculate REAL price directly from blockchain pool reserves
        match self.calculate_real_pool_price_from_reserves(pool_address, token_mint).await {
            Ok(Some(pool_price_info)) => {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "DIRECT_CALC_SUCCESS",
                        &format!(
                            "✅ DIRECT price calculated: {:.12} SOL for {} from pool {} ({})",
                            pool_price_info.price_sol,
                            token_mint,
                            pool_address,
                            pool_price_info.pool_type
                        )
                    );
                }

                let calculation_time = Utc::now();
                let result = PoolPriceResult {
                    pool_address: pool_address.to_string(),
                    dex_id: "Direct".to_string(), // Mark as direct calculation
                    pool_type: Some(pool_price_info.pool_type.clone()),
                    token_address: token_mint.to_string(),
                    price_sol: Some(pool_price_info.price_sol),
                    price_usd: None, // We don't calculate USD prices from pools - only SOL prices
                    api_price_sol, // Include API price for comparison
                    liquidity_usd: 0.0, // No API data for liquidity in direct mode
                    volume_24h: 0.0, // No API data for volume in direct mode
                    source: "pool_direct".to_string(),
                    calculated_at: calculation_time,
                    sol_reserve: Some(
                        (pool_price_info.sol_reserve as f64) /
                            (10_f64).powi(pool_price_info.sol_decimals as i32)
                    ),
                    token_reserve: Some(
                        (pool_price_info.token_reserve as f64) /
                            (10_f64).powi(pool_price_info.token_decimals as i32)
                    ),
                    error: None, // No error for successful direct pool calculation
                };

                // Cache the result
                {
                    let mut price_cache = self.price_cache.write().await;
                    price_cache.insert(token_mint.to_string(), result.clone());

                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "DIRECT_CACHE_STORED",
                            &format!(
                                "💾 CACHED direct price for {}: {:.12} SOL from pool {}",
                                token_mint,
                                pool_price_info.price_sol,
                                pool_address
                            )
                        );
                    }
                }

                Some(result)
            }
            Ok(None) => {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "DIRECT_CALC_NONE",
                        &format!(
                            "❓ DIRECT calculation returned None for pool {} token {}",
                            pool_address,
                            token_mint
                        )
                    );
                }
                None
            }
            Err(e) => {
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "DIRECT_CALC_ERROR",
                        &format!(
                            "❌ DIRECT calculation FAILED for pool {} token {}: {}",
                            pool_address,
                            token_mint,
                            e
                        )
                    );
                }
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
struct BackoffEntry {
    consecutive_failures: u32,
    next_retry: Instant,
}

impl BackoffEntry {
    fn new() -> Self {
        Self {
            consecutive_failures: 0,
            next_retry: Instant::now(),
        }
    }
    fn register_failure(&mut self) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        let base: u64 = 6; // seconds
        let exp = self.consecutive_failures.min(5); // cap
        let delay = (base * (2u64).saturating_pow(exp)).min(60);
        self.next_retry = Instant::now() + Duration::from_secs(delay);
    }
    fn reset(&mut self) {
        self.consecutive_failures = 0;
        self.next_retry = Instant::now();
    }
}

// =============================================================================
// GLOBAL POOL PRICE SERVICE
// =============================================================================

static mut GLOBAL_POOL_SERVICE: Option<PoolPriceService> = None;
static POOL_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global pool price service (idempotent)
pub fn init_pool_service() -> &'static PoolPriceService {
    unsafe {
        POOL_INIT.call_once(|| {
            GLOBAL_POOL_SERVICE = Some(PoolPriceService::new());
        });
        match GLOBAL_POOL_SERVICE.as_ref() {
            Some(svc) => svc,
            None => {
                log(LogTag::Pool, "INIT_ERROR", "PoolPriceService failed to initialize");
                panic!("PoolPriceService failed to initialize");
            }
        }
    }
}

/// Get or initialize service, logging on failure
pub fn get_pool_service() -> &'static PoolPriceService {
    init_pool_service()
}

/// Request priority price updates for open positions (global function)
/// This function is designed to be called by the positions system when fresh prices are critically needed
pub async fn request_priority_updates_for_open_positions() -> usize {
    // Get current open position mints
    let open_mints = match crate::positions::get_open_mints().await {
        mints if !mints.is_empty() => mints,
        _ => {
            // No open positions, nothing to update
            return 0;
        }
    };

    if is_debug_pool_prices_enabled() {
        log(
            LogTag::Pool,
            "PRIORITY_REQUEST",
            &format!("Requesting priority updates for {} open position tokens", open_mints.len())
        );
    }

    let service = get_pool_service();
    service.request_priority_price_updates(&open_mints).await
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get token decimals for pool calculations (unified approach)
async fn get_token_decimals_with_cache(mint: &str) -> Option<u8> {
    // Use the unified decimal function (cache + blockchain if needed)
    crate::tokens::get_token_decimals(mint).await
}

// =============================================================================
// POOL STATISTICS AND ANALYTICS
// =============================================================================

/// Pool calculation statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub calculations_attempted: u64,
    pub calculations_successful: u64,
    pub calculations_failed: u64,
    pub cache_hits: u64,
    pub average_calculation_time_ms: f64,
    pub pools_by_program: HashMap<String, u64>,
}

impl PoolStats {
    pub fn new() -> Self {
        Self {
            calculations_attempted: 0,
            calculations_successful: 0,
            calculations_failed: 0,
            cache_hits: 0,
            average_calculation_time_ms: 0.0,
            pools_by_program: HashMap::new(),
        }
    }

    pub fn record_calculation(&mut self, success: bool, time_ms: f64, program_id: &str) {
        self.calculations_attempted += 1;
        if success {
            self.calculations_successful += 1;
        } else {
            self.calculations_failed += 1;
        }

        // Track by program ID
        *self.pools_by_program.entry(program_id.to_string()).or_insert(0) += 1;

        // Update average time
        let total_time =
            self.average_calculation_time_ms * ((self.calculations_attempted - 1) as f64);
        self.average_calculation_time_ms =
            (total_time + time_ms) / (self.calculations_attempted as f64);
    }

    pub fn get_success_rate(&self) -> f64 {
        if self.calculations_attempted == 0 {
            0.0
        } else {
            ((self.calculations_successful as f64) / (self.calculations_attempted as f64)) * 100.0
        }
    }
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pool Stats - Attempted: {}, Success Rate: {:.1}%, Avg Time: {:.1}ms, Programs: {}",
            self.calculations_attempted,
            self.get_success_rate(),
            self.average_calculation_time_ms,
            self.pools_by_program.len()
        )
    }
}

// =============================================================================
// POOL PRICE CALCULATOR
// =============================================================================

/// Advanced pool price calculator with multi-program support
pub struct PoolPriceCalculator {
    rpc_client: &'static crate::rpc::RpcClient,
    stats: Arc<RwLock<PoolStats>>,
    debug_enabled: bool,
}

impl PoolPriceCalculator {
    /// Create new pool price calculator (always uses centralized RPC system).
    pub fn new() -> Self {
        let rpc_client = get_rpc_client();
        let debug_enabled = is_debug_pool_calculator_enabled();

        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Pool calculator debug mode enabled");
        }

        Self {
            rpc_client,
            stats: Arc::new(RwLock::new(PoolStats::new())),
            debug_enabled,
        }
    }

    /// Enable debug mode (overrides global setting)
    pub fn enable_debug(&mut self) {
        self.debug_enabled = true;
        log(LogTag::Pool, "DEBUG", "Pool calculator debug mode enabled (overridden)");
    }

    /// Refresh only the reserve values in a PoolInfo (keeps cached metadata, fetches fresh reserves)
    async fn refresh_pool_reserves(
        &self,
        pool_info: &PoolInfo,
        account: &Account
    ) -> Result<PoolInfo, String> {
        let mut updated_pool = pool_info.clone();

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "REFRESH_RESERVES",
                &format!(
                    "Refreshing reserves for pool {} ({})",
                    pool_info.pool_address,
                    pool_info.pool_type
                )
            );
        }

        // Refresh reserves based on pool type
        match pool_info.pool_program_id.as_str() {
            | RAYDIUM_CPMM_PROGRAM_ID
            | RAYDIUM_CLMM_PROGRAM_ID
            | METEORA_DAMM_V2_PROGRAM_ID
            | ORCA_WHIRLPOOL_PROGRAM_ID => {
                // These pools use vault balances
                if
                    let (Some(vault_0), Some(vault_1)) = (
                        &pool_info.token_0_vault,
                        &pool_info.token_1_vault,
                    )
                {
                    let (reserve_0, reserve_1) = self.get_vault_balances(vault_0, vault_1).await?;
                    updated_pool.token_0_reserve = reserve_0;
                    updated_pool.token_1_reserve = reserve_1;
                }
            }
            METEORA_DLMM_PROGRAM_ID => {
                // DLMM uses different reserve accounts
                if
                    let (Some(vault_0), Some(vault_1)) = (
                        &pool_info.token_0_vault,
                        &pool_info.token_1_vault,
                    )
                {
                    let (reserve_0, reserve_1) = self.get_dlmm_vault_balances(
                        vault_0,
                        vault_1
                    ).await?;
                    updated_pool.token_0_reserve = reserve_0;
                    updated_pool.token_1_reserve = reserve_1;
                }
            }
            RAYDIUM_LEGACY_AMM_PROGRAM_ID => {
                if
                    let (Some(vault_0), Some(vault_1)) = (
                        &pool_info.token_0_vault,
                        &pool_info.token_1_vault,
                    )
                {
                    // Try vault balances first
                    match self.get_vault_balances(vault_0, vault_1).await {
                        Ok((reserve_0, reserve_1)) => {
                            updated_pool.token_0_reserve = reserve_0;
                            updated_pool.token_1_reserve = reserve_1;
                        }
                        Err(_) => {
                            // Fallback to pool data extraction
                            if
                                let Ok(reserve_pairs) =
                                    self.extract_raydium_legacy_reserves_from_data(&account.data)
                            {
                                if let Some((reserve_0, reserve_1)) = reserve_pairs.first() {
                                    updated_pool.token_0_reserve = *reserve_0;
                                    updated_pool.token_1_reserve = *reserve_1;
                                }
                            }
                        }
                    }
                }
            }
            PUMP_FUN_AMM_PROGRAM_ID => {
                // For Pump.fun, we need to re-extract vault addresses from pool data and fetch fresh balances
                if
                    let Ok((base_vault, quote_vault)) = self.extract_pump_fun_vault_addresses(
                        &account.data
                    )
                {
                    match self.get_vault_balances(&base_vault, &quote_vault).await {
                        Ok((base_reserve, quote_reserve)) => {
                            updated_pool.token_0_reserve = base_reserve;
                            updated_pool.token_1_reserve = quote_reserve;

                            if self.debug_enabled {
                                log(
                                    LogTag::Pool,
                                    "PUMP_RESERVES_REFRESHED",
                                    &format!(
                                        "Fresh Pump.fun reserves: base_vault={} ({}), quote_vault={} ({})",
                                        base_vault,
                                        base_reserve,
                                        quote_vault,
                                        quote_reserve
                                    )
                                );
                            }
                        }
                        Err(e) => {
                            if self.debug_enabled {
                                log(
                                    LogTag::Pool,
                                    "PUMP_VAULT_ERROR",
                                    &format!("Failed to refresh Pump.fun vault balances: {}", e)
                                );
                            }
                        }
                    }
                } else {
                    if self.debug_enabled {
                        log(
                            LogTag::Pool,
                            "PUMP_EXTRACT_ERROR",
                            "Failed to extract vault addresses from Pump.fun pool data"
                        );
                    }
                }
            }
            _ => {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "UNKNOWN_RESERVES",
                        &format!(
                            "Unknown pool type for reserve refresh: {}",
                            pool_info.pool_program_id
                        )
                    );
                }
            }
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "RESERVES_REFRESHED",
                &format!(
                    "Fresh reserves: token_0={}, token_1={}",
                    updated_pool.token_0_reserve,
                    updated_pool.token_1_reserve
                )
            );
        }

        Ok(updated_pool)
    }

    /// Get pool information from on-chain data
    pub async fn get_pool_info(&self, pool_address: &str) -> Result<Option<PoolInfo>, String> {
        // For single pool, use the batch method with single address
        let pool_infos = self.get_multiple_pool_infos(&[pool_address.to_string()]).await?;
        Ok(
            pool_infos
                .into_iter()
                .next()
                .map(|(_, info)| info)
                .flatten()
        )
    }

    /// Get multiple pool information from on-chain data (BATCH OPTIMIZED)
    pub async fn get_multiple_pool_infos(
        &self,
        pool_addresses: &[String]
    ) -> Result<HashMap<String, Option<PoolInfo>>, String> {
        if pool_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        let start_time = Instant::now();

        // Parse all pool addresses
        let pool_pubkeys: Result<Vec<(String, Pubkey)>, String> = pool_addresses
            .iter()
            .map(|addr| {
                let pubkey = Pubkey::from_str(addr).map_err(|e|
                    format!("Invalid pool address {}: {}", addr, e)
                )?;
                Ok((addr.clone(), pubkey))
            })
            .collect();

        let pool_pubkeys = pool_pubkeys?;
        let pubkeys: Vec<Pubkey> = pool_pubkeys
            .iter()
            .map(|(_, pk)| *pk)
            .collect();

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "BATCH_POOL_FETCH",
                &format!(
                    "🔍 Batch fetching {} pool accounts with PROCESSED commitment",
                    pool_addresses.len()
                )
            );
        }

        // Batch fetch all pool accounts in chunks to leverage getMultipleAccounts throughput
        let rpc_phase_start = Instant::now();
        let mut accounts: Vec<Option<Account>> = Vec::with_capacity(pubkeys.len());
        let chunk_size = 100usize;
        let mut idx = 0usize;
        while idx < pubkeys.len() {
            let end = (idx + chunk_size).min(pubkeys.len());
            let chunk = &pubkeys[idx..end];
            match
                tokio::time::timeout(
                    Duration::from_secs(12),
                    self.rpc_client.get_multiple_accounts(chunk)
                ).await
            {
                Ok(Ok(accs)) => {
                    accounts.extend(accs);
                }
                Ok(Err(e)) => {
                    return Err(format!("Failed to batch get pool accounts: {}", e));
                }
                Err(_) => {
                    return Err("RPC get_multiple_accounts chunk timed out after 12s".to_string());
                }
            }
            idx = end;
        }
        if self.debug_enabled {
            log(
                LogTag::Pool,
                "RPC_BATCH_ACCOUNTS",
                &format!(
                    "Fetched {} accounts in {:.2}ms across chunks",
                    accounts.len(),
                    rpc_phase_start.elapsed().as_millis()
                )
            );
        }

        // Process each account and decode pool info
        let mut result = HashMap::new();
        let mut successful_decodes = 0;

        for (i, (pool_address, _)) in pool_pubkeys.iter().enumerate() {
            let pool_info = if let Some(account) = &accounts[i] {
                // Determine pool type by owner (program ID)
                let program_id = account.owner.to_string();

                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "POOL_PROGRAM_ID",
                        &format!(
                            "Pool {} owned by program {} (data length: {} bytes)",
                            pool_address,
                            program_id,
                            account.data.len()
                        )
                    );
                }

                // Decode based on program ID
                let decoded_info = match program_id.as_str() {
                    RAYDIUM_CPMM_PROGRAM_ID => {
                        if self.debug_enabled {
                            log(LogTag::Pool, "DECODER_SELECT", "Using Raydium CPMM decoder");
                        }
                        self.decode_raydium_cpmm_pool(pool_address, account).await
                    }
                    RAYDIUM_LEGACY_AMM_PROGRAM_ID => {
                        if self.debug_enabled {
                            log(LogTag::Pool, "DECODER_SELECT", "Using Raydium Legacy AMM decoder");
                        }
                        self.decode_raydium_legacy_amm_pool(pool_address, account).await
                    }
                    RAYDIUM_CLMM_PROGRAM_ID => {
                        if self.debug_enabled {
                            log(LogTag::Pool, "DECODER_SELECT", "Using Raydium CLMM decoder");
                        }
                        self.decode_raydium_clmm_pool(pool_address, account).await
                    }
                    METEORA_DAMM_V2_PROGRAM_ID => {
                        if self.debug_enabled {
                            log(LogTag::Pool, "DECODER_SELECT", "Using Meteora DAMM v2 decoder");
                        }
                        self.decode_meteora_damm_v2_pool(pool_address, account).await
                    }
                    METEORA_DLMM_PROGRAM_ID => {
                        if self.debug_enabled {
                            log(LogTag::Pool, "DECODER_SELECT", "Using Meteora DLMM decoder");
                        }
                        self.decode_meteora_dlmm_pool(pool_address, account).await
                    }
                    ORCA_WHIRLPOOL_PROGRAM_ID => {
                        if self.debug_enabled {
                            log(LogTag::Pool, "DECODER_SELECT", "Using Orca Whirlpool decoder");
                        }
                        self.decode_orca_whirlpool_pool(pool_address, account).await
                    }
                    PUMP_FUN_AMM_PROGRAM_ID => {
                        if self.debug_enabled {
                            log(LogTag::Pool, "DECODER_SELECT", "Using Pump.fun AMM decoder");
                        }
                        self.decode_pump_fun_amm_pool(pool_address, account).await
                    }
                    _ => {
                        if self.debug_enabled {
                            log(
                                LogTag::Pool,
                                "UNSUPPORTED_PROGRAM",
                                &format!("Unsupported pool program: {}", program_id)
                            );
                        }
                        // Track unsupported program ID for consolidated summary logs
                        get_pool_service().record_unsupported_program(&program_id).await;
                        Err(format!("Unsupported pool program: {}", program_id))
                    }
                };

                match decoded_info {
                    Ok(info) => {
                        successful_decodes += 1;
                        Some(info)
                    }
                    Err(e) => {
                        if self.debug_enabled {
                            log(
                                LogTag::Pool,
                                "DECODE_ERROR",
                                &format!("Failed to decode pool {}: {}", pool_address, e)
                            );
                        }
                        None
                    }
                }
            } else {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "ACCOUNT_NOT_FOUND",
                        &format!("Pool account not found: {}", pool_address)
                    );
                }
                None
            };

            result.insert(pool_address.clone(), pool_info);
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            for _ in 0..successful_decodes {
                stats.record_calculation(true, start_time.elapsed().as_millis() as f64, "batch");
            }
            for _ in 0..pool_addresses.len() - successful_decodes {
                stats.record_calculation(false, start_time.elapsed().as_millis() as f64, "batch");
            }
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "BATCH_SUCCESS",
                &format!(
                    "Batch decoded {}/{} pools in {:.2}ms",
                    successful_decodes,
                    pool_addresses.len(),
                    start_time.elapsed().as_millis()
                )
            );
        }

        Ok(result)
    }

    /// Calculate token price from pool reserves
    pub async fn calculate_token_price(
        &self,
        pool_address: &str,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        let cache_key = format!("{}_{}", pool_address, token_mint);

        // NO CACHING - ALWAYS CALCULATE FRESH FOR REAL-TIME PRICES

        let start_time = Instant::now();

        // Get pool information
        let pool_info = match self.get_pool_info(pool_address).await? {
            Some(info) => info,
            None => {
                return Ok(None);
            }
        };

        // Calculate price based on pool type
        let price_info = match pool_info.pool_program_id.as_str() {
            RAYDIUM_CPMM_PROGRAM_ID => {
                self.calculate_raydium_cpmm_price(&pool_info, token_mint).await?
            }
            RAYDIUM_LEGACY_AMM_PROGRAM_ID => {
                self.calculate_raydium_legacy_amm_price(&pool_info, token_mint).await?
            }
            RAYDIUM_CLMM_PROGRAM_ID => {
                self.calculate_raydium_clmm_price(&pool_info, token_mint).await?
            }
            METEORA_DAMM_V2_PROGRAM_ID => {
                self.calculate_meteora_damm_v2_price(&pool_info, token_mint).await?
            }
            METEORA_DLMM_PROGRAM_ID => {
                self.calculate_meteora_dlmm_price(&pool_info, token_mint).await?
            }
            ORCA_WHIRLPOOL_PROGRAM_ID => {
                self.calculate_orca_whirlpool_price(&pool_info, token_mint).await?
            }
            PUMP_FUN_AMM_PROGRAM_ID => {
                self.calculate_pump_fun_amm_price(&pool_info, token_mint).await?
            }
            _ => {
                return Err(
                    format!(
                        "Price calculation not supported for program: {}",
                        pool_info.pool_program_id
                    )
                );
            }
        };

        // NO CACHING - NO STORAGE OF RESULTS

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.record_calculation(
                price_info.is_some(),
                start_time.elapsed().as_millis() as f64,
                &pool_info.pool_program_id
            );
        }

        if self.debug_enabled && price_info.is_some() {
            log(
                LogTag::Pool,
                "SUCCESS",
                &format!(
                    "Price calculated: {:.12} SOL for {} in {:.2}ms",
                    price_info.as_ref().unwrap().price_sol,
                    token_mint,
                    start_time.elapsed().as_millis()
                )
            );
        }

        Ok(price_info)
    }

    /// Calculate token prices from pool reserves (BATCH OPTIMIZED)
    /// Takes a list of (pool_address, token_mint) pairs and returns prices for all
    pub async fn calculate_multiple_token_prices(
        &self,
        pool_token_pairs: &[(String, String)] // (pool_address, token_mint)
    ) -> Result<HashMap<String, Option<PoolPriceInfo>>, String> {
        if pool_token_pairs.is_empty() {
            return Ok(HashMap::new());
        }

        let start_time = Instant::now();

        // Extract unique pool addresses for batch fetching
        let pool_addresses: Vec<String> = pool_token_pairs
            .iter()
            .map(|(pool_addr, _)| pool_addr.clone())
            .collect::<std::collections::HashSet<_>>() // Deduplicate
            .into_iter()
            .collect();

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "BATCH_PRICE_CALC",
                &format!(
                    "🔍 Batch calculating prices for {} token-pool pairs ({} unique pools)",
                    pool_token_pairs.len(),
                    pool_addresses.len()
                )
            );
        }

        // Batch fetch all pool infos with timing
        let fetch_pools_start = Instant::now();
        let pool_infos = self.get_multiple_pool_infos(&pool_addresses).await?;
        let fetch_pools_ms = fetch_pools_start.elapsed().as_millis();

        // Calculate prices for each token-pool pair
        let mut results = HashMap::new();
        let mut successful_calculations = 0;

        let calc_phase_start = Instant::now();
        for (pool_address, token_mint) in pool_token_pairs {
            let cache_key = format!("{}_{}", pool_address, token_mint);

            let price_info = if let Some(Some(pool_info)) = pool_infos.get(pool_address) {
                // Calculate price based on pool type
                match pool_info.pool_program_id.as_str() {
                    RAYDIUM_CPMM_PROGRAM_ID => {
                        self.calculate_raydium_cpmm_price(pool_info, token_mint).await
                    }
                    RAYDIUM_LEGACY_AMM_PROGRAM_ID => {
                        self.calculate_raydium_legacy_amm_price(pool_info, token_mint).await
                    }
                    RAYDIUM_CLMM_PROGRAM_ID => {
                        self.calculate_raydium_clmm_price(pool_info, token_mint).await
                    }
                    METEORA_DAMM_V2_PROGRAM_ID => {
                        self.calculate_meteora_damm_v2_price(pool_info, token_mint).await
                    }
                    METEORA_DLMM_PROGRAM_ID => {
                        self.calculate_meteora_dlmm_price(pool_info, token_mint).await
                    }
                    ORCA_WHIRLPOOL_PROGRAM_ID => {
                        self.calculate_orca_whirlpool_price(pool_info, token_mint).await
                    }
                    PUMP_FUN_AMM_PROGRAM_ID => {
                        self.calculate_pump_fun_amm_price(pool_info, token_mint).await
                    }
                    _ => {
                        if self.debug_enabled {
                            log(
                                LogTag::Pool,
                                "UNSUPPORTED_PROGRAM_PRICE",
                                &format!(
                                    "Price calculation not supported for program: {}",
                                    pool_info.pool_program_id
                                )
                            );
                        }
                        // Track unsupported program for price path as well
                        get_pool_service().record_unsupported_program(
                            &pool_info.pool_program_id
                        ).await;
                        Ok(None)
                    }
                }
            } else {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "POOL_INFO_MISSING",
                        &format!("Pool info missing for price calculation: {}", pool_address)
                    );
                }
                Ok(None)
            };

            match price_info {
                Ok(Some(info)) => {
                    successful_calculations += 1;
                    results.insert(cache_key, Some(info));
                }
                Ok(None) => {
                    results.insert(cache_key, None);
                }
                Err(e) => {
                    if self.debug_enabled {
                        log(
                            LogTag::Pool,
                            "PRICE_CALC_ERROR",
                            &format!(
                                "Failed to calculate price for {}-{}: {}",
                                pool_address,
                                token_mint,
                                e
                            )
                        );
                    }
                    results.insert(cache_key, None);
                }
            }
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            for _ in 0..successful_calculations {
                stats.record_calculation(
                    true,
                    start_time.elapsed().as_millis() as f64,
                    "batch_price"
                );
            }
            for _ in 0..pool_token_pairs.len() - successful_calculations {
                stats.record_calculation(
                    false,
                    start_time.elapsed().as_millis() as f64,
                    "batch_price"
                );
            }
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "BATCH_PRICE_SUCCESS",
                &format!(
                    "Batch calculated {}/{} prices in {:.2}ms (fetch_pools: {}ms, calc: {}ms)",
                    successful_calculations,
                    pool_token_pairs.len(),
                    start_time.elapsed().as_millis(),
                    fetch_pools_ms,
                    calc_phase_start.elapsed().as_millis()
                )
            );
        }

        Ok(results)
    }

    /// Get statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }

    /// Get raw pool account data for debugging
    pub async fn get_raw_pool_data(&self, pool_address: &str) -> Result<Option<Vec<u8>>, String> {
        let pool_pubkey = Pubkey::from_str(pool_address).map_err(|e|
            format!("Invalid pool address: {}", e)
        )?;

        match self.rpc_client.get_account(&pool_pubkey).await {
            Ok(account) => Ok(Some(account.data)),
            Err(e) => {
                if e.contains("not found") {
                    Ok(None)
                } else {
                    Err(format!("Failed to fetch account data: {}", e))
                }
            }
        }
    }
}

// =============================================================================
// GLOBAL SHARED POOL PRICE CALCULATOR
// =============================================================================
static mut GLOBAL_POOL_PRICE_CALCULATOR: Option<PoolPriceCalculator> = None;
static POOL_PRICE_CALCULATOR_INIT: std::sync::Once = std::sync::Once::new();

pub fn init_global_pool_price_calculator() -> &'static PoolPriceCalculator {
    unsafe {
        POOL_PRICE_CALCULATOR_INIT.call_once(|| {
            let mut calc = PoolPriceCalculator::new();
            if is_debug_pool_prices_enabled() {
                calc.enable_debug();
            }
            GLOBAL_POOL_PRICE_CALCULATOR = Some(calc);
        });
        match GLOBAL_POOL_PRICE_CALCULATOR.as_ref() {
            Some(calc) => calc,
            None => {
                log(LogTag::Pool, "INIT_ERROR", "PoolPriceCalculator failed to initialize");
                panic!("PoolPriceCalculator failed to initialize");
            }
        }
    }
}

pub fn try_get_global_pool_price_calculator() -> Option<&'static PoolPriceCalculator> {
    unsafe { GLOBAL_POOL_PRICE_CALCULATOR.as_ref() }
}

pub fn get_global_pool_price_calculator() -> &'static PoolPriceCalculator {
    init_global_pool_price_calculator()
}

// =============================================================================
// POOL DECODERS
// =============================================================================

impl PoolPriceCalculator {
    /// Decode Raydium CPMM pool data from account bytes
    async fn decode_raydium_cpmm_pool(
        &self,
        pool_address: &str,
        account: &Account
    ) -> Result<PoolInfo, String> {
        if account.data.len() < 8 + 32 * 10 + 8 * 10 {
            return Err("Invalid Raydium CPMM pool account data length".to_string());
        }

        let data = &account.data;
        let mut offset = 8; // Skip discriminator

        // Decode pool data according to Raydium CPMM layout
        let amm_config = Self::read_pubkey_at_offset(data, &mut offset)?;
        let pool_creator = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_0_vault = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_1_vault = Self::read_pubkey_at_offset(data, &mut offset)?;
        let lp_mint = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_0_mint = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_1_mint = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_0_program = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_1_program = Self::read_pubkey_at_offset(data, &mut offset)?;
        let observation_key = Self::read_pubkey_at_offset(data, &mut offset)?;

        let auth_bump = Self::read_u8_at_offset(data, &mut offset)?;
        let status = Self::read_u8_at_offset(data, &mut offset)?;
        let lp_mint_decimals = Self::read_u8_at_offset(data, &mut offset)?;
        let pool_mint_0_decimals = Self::read_u8_at_offset(data, &mut offset)?;
        let pool_mint_1_decimals = Self::read_u8_at_offset(data, &mut offset)?;

        // Use decimal cache system with pool data as fallback
        let mint_0_decimals = get_cached_decimals(&token_0_mint.to_string()).unwrap_or(
            pool_mint_0_decimals
        );
        let mint_1_decimals = get_cached_decimals(&token_1_mint.to_string()).unwrap_or(
            pool_mint_1_decimals
        );

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "DECIMALS",
                &format!(
                    "Decimal Analysis for {}:\n  \
                     Token0 {} decimals: {} (cached) vs {} (pool)\n  \
                     Token1 {} decimals: {} (cached) vs {} (pool)\n  \
                     Cached decimals source: decimal_cache.json",
                    pool_address,
                    token_0_mint.to_string().chars().take(8).collect::<String>(),
                    mint_0_decimals,
                    pool_mint_0_decimals,
                    token_1_mint.to_string().chars().take(8).collect::<String>(),
                    mint_1_decimals,
                    pool_mint_1_decimals
                )
            );

            // Warning if cached and pool decimals don't match
            if mint_0_decimals != pool_mint_0_decimals {
                log(
                    LogTag::Pool,
                    "DECIMAL_MISMATCH",
                    &format!(
                        "DECIMAL MISMATCH Token0 {}: cache={}, pool={}",
                        token_0_mint,
                        mint_0_decimals,
                        pool_mint_0_decimals
                    )
                );
            }
            if mint_1_decimals != pool_mint_1_decimals {
                log(
                    LogTag::Pool,
                    "DECIMAL_MISMATCH",
                    &format!(
                        "DECIMAL MISMATCH Token1 {}: cache={}, pool={}",
                        token_1_mint,
                        mint_1_decimals,
                        pool_mint_1_decimals
                    )
                );
            }
        }

        // Skip padding
        offset += 3;

        let lp_supply = Self::read_u64_at_offset(data, &mut offset)?;
        let _protocol_fees_token_0 = Self::read_u64_at_offset(data, &mut offset)?;
        let _protocol_fees_token_1 = Self::read_u64_at_offset(data, &mut offset)?;
        let _fund_fees_token_0 = Self::read_u64_at_offset(data, &mut offset)?;
        let _fund_fees_token_1 = Self::read_u64_at_offset(data, &mut offset)?;
        let _open_time = Self::read_u64_at_offset(data, &mut offset)?;

        // Get vault balances to calculate reserves
        let (token_0_reserve, token_1_reserve) = self.get_vault_balances(
            &token_0_vault,
            &token_1_vault
        ).await?;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "DECODE",
                &format!(
                    "Raydium CPMM Pool {} - Token0: {} ({} decimals, {} reserve), Token1: {} ({} decimals, {} reserve)",
                    pool_address,
                    token_0_mint,
                    mint_0_decimals,
                    token_0_reserve,
                    token_1_mint,
                    mint_1_decimals,
                    token_1_reserve
                )
            );
        }

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_program_id: RAYDIUM_CPMM_PROGRAM_ID.to_string(),
            pool_type: get_pool_program_display_name(RAYDIUM_CPMM_PROGRAM_ID),
            token_0_mint,
            token_1_mint,
            token_0_vault: Some(token_0_vault),
            token_1_vault: Some(token_1_vault),
            token_0_reserve,
            token_1_reserve,
            token_0_decimals: mint_0_decimals,
            token_1_decimals: mint_1_decimals,
            lp_mint: Some(lp_mint),
            lp_supply: Some(lp_supply),
            creator: Some(pool_creator),
            status: Some(status.into()),
            liquidity_usd: None, // Will be calculated separately
            sqrt_price: None, // Not applicable to AMM pools
        })
    }

    /// Calculate price for Raydium CPMM pool
    async fn calculate_raydium_cpmm_price(
        &self,
        pool_info: &PoolInfo,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // Determine which token is SOL and which is the target token
        let (sol_reserve, token_reserve, sol_decimals, token_decimals, is_token_0) = if
            pool_info.token_0_mint == SOL_MINT &&
            pool_info.token_1_mint == token_mint
        {
            (
                pool_info.token_0_reserve,
                pool_info.token_1_reserve,
                pool_info.token_0_decimals,
                pool_info.token_1_decimals,
                false,
            )
        } else if pool_info.token_1_mint == SOL_MINT && pool_info.token_0_mint == token_mint {
            (
                pool_info.token_1_reserve,
                pool_info.token_0_reserve,
                pool_info.token_1_decimals,
                pool_info.token_0_decimals,
                true,
            )
        } else {
            return Err(format!("Pool does not contain SOL or target token {}", token_mint));
        };

        // Validate reserves
        if sol_reserve == 0 || token_reserve == 0 {
            return Err("Pool has zero reserves".to_string());
        }

        // Calculate price: price = sol_reserve / token_reserve (adjusted for decimals)
        let sol_adjusted = (sol_reserve as f64) / (10_f64).powi(sol_decimals as i32);
        let token_adjusted = (token_reserve as f64) / (10_f64).powi(token_decimals as i32);

        let price_sol = sol_adjusted / token_adjusted;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "CALC",
                &format!(
                    "Raydium CPMM Price Calculation for {}:\n  \
                     SOL Reserve: {} ({:.9} adjusted, {} decimals)\n  \
                     Token Reserve: {} ({:.9} adjusted, {} decimals)\n  \
                     Price: {:.9} SOL\n  \
                     Pool: {} ({})",
                    token_mint,
                    sol_reserve,
                    sol_adjusted,
                    sol_decimals,
                    token_reserve,
                    token_adjusted,
                    token_decimals,
                    price_sol,
                    pool_info.pool_address,
                    pool_info.pool_type
                )
            );

            // Additional validation checks
            if sol_adjusted <= 0.0 || token_adjusted <= 0.0 {
                log(
                    LogTag::Pool,
                    "CALC_WARN",
                    &format!(
                        "WARNING: Zero or negative adjusted values detected! \
                         SOL_adj: {:.9}, Token_adj: {:.9}",
                        sol_adjusted,
                        token_adjusted
                    )
                );
            }

            // Check for extremely small or large prices that might indicate decimal issues
            if price_sol < 0.000000001 || price_sol > 1000.0 {
                log(
                    LogTag::Pool,
                    "CALC_WARN",
                    &format!(
                        "WARNING: Unusual price detected: {:.12} SOL. \
                         Check if decimals are correct (SOL: {}, Token: {})",
                        price_sol,
                        sol_decimals,
                        token_decimals
                    )
                );
            }
        }

        Ok(
            Some(PoolPriceInfo {
                pool_address: pool_info.pool_address.clone(),
                pool_program_id: pool_info.pool_program_id.clone(),
                pool_type: pool_info.pool_type.clone(),
                token_mint: token_mint.to_string(),
                price_sol,
                token_reserve,
                sol_reserve,
                token_decimals,
                sol_decimals,
            })
        )
    }

    /// Decode Raydium CLMM (Concentrated Liquidity Market Maker) pool data from account bytes
    async fn decode_raydium_clmm_pool(
        &self,
        pool_address: &str,
        account: &Account
    ) -> Result<PoolInfo, String> {
        if account.data.len() < 300 {
            return Err("Invalid Raydium CLMM pool account data length".to_string());
        }

        let data = &account.data;
        let mut offset = 8; // Skip discriminator

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "CLMM_DEBUG",
                &format!(
                    "Raydium CLMM pool {} - data length: {} bytes, decoding structure...",
                    pool_address,
                    data.len()
                )
            );
        }

        // Decode Raydium CLMM pool structure based on provided schema:
        // blob(8), u8("bump"), publicKey("ammConfig"), publicKey("creator"),
        // publicKey("mintA"), publicKey("mintB"), publicKey("vaultA"), publicKey("vaultB"),
        // publicKey("observationId"), u8("mintDecimalsA"), u8("mintDecimalsB"),
        // u16("tickSpacing"), u128("liquidity"), u128("sqrtPriceX64"), s32("tickCurrent"),
        // u32(), u128("feeGrowthGlobalX64A"), u128("feeGrowthGlobalX64B"),
        // u64("protocolFeesTokenA"), u64("protocolFeesTokenB"),
        // u128("swapInAmountTokenA"), u128("swapOutAmountTokenB")

        let _bump = Self::read_u8_at_offset(data, &mut offset)?; // bump
        let _amm_config = Self::read_pubkey_at_offset(data, &mut offset)?; // ammConfig
        let _creator = Self::read_pubkey_at_offset(data, &mut offset)?; // creator
        let mint_a = Self::read_pubkey_at_offset(data, &mut offset)?; // mintA (SOL)
        let mint_b = Self::read_pubkey_at_offset(data, &mut offset)?; // mintB (target token)
        let vault_a = Self::read_pubkey_at_offset(data, &mut offset)?; // vaultA (SOL vault)
        let vault_b = Self::read_pubkey_at_offset(data, &mut offset)?; // vaultB (token vault)
        let _observation_id = Self::read_pubkey_at_offset(data, &mut offset)?; // observationId

        let mint_decimals_a = Self::read_u8_at_offset(data, &mut offset)?; // mintDecimalsA
        let mint_decimals_b = Self::read_u8_at_offset(data, &mut offset)?; // mintDecimalsB

        let _tick_spacing = u16::from_le_bytes(
            data[offset..offset + 2].try_into().unwrap_or([0; 2])
        ); // tickSpacing
        offset += 2;

        let liquidity = u128::from_le_bytes(
            data[offset..offset + 16].try_into().unwrap_or([0; 16])
        ); // liquidity
        offset += 16;

        let sqrt_price_x64 = u128::from_le_bytes(
            data[offset..offset + 16].try_into().unwrap_or([0; 16])
        ); // sqrtPriceX64
        offset += 16;

        let _tick_current = i32::from_le_bytes(
            data[offset..offset + 4].try_into().unwrap_or([0; 4])
        ); // tickCurrent
        offset += 4;

        // Skip u32() padding
        offset += 4;

        let _fee_growth_global_a = u128::from_le_bytes(
            data[offset..offset + 16].try_into().unwrap_or([0; 16])
        ); // feeGrowthGlobalX64A
        offset += 16;

        let _fee_growth_global_b = u128::from_le_bytes(
            data[offset..offset + 16].try_into().unwrap_or([0; 16])
        ); // feeGrowthGlobalX64B
        offset += 16;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "CLMM_EXTRACT",
                &format!(
                    "Extracted Raydium CLMM pool structure:\n\
                    - Mint A (SOL): {}\n\
                    - Mint B (target): {}\n\
                    - Vault A (SOL): {}\n\
                    - Vault B (target): {}\n\
                    - Mint Decimals A: {}\n\
                    - Mint Decimals B: {}\n\
                    - Liquidity: {}\n\
                    - Sqrt Price X64: {}",
                    mint_a,
                    mint_b,
                    vault_a,
                    vault_b,
                    mint_decimals_a,
                    mint_decimals_b,
                    liquidity,
                    sqrt_price_x64
                )
            );
        }

        // For Raydium CLMM, get reserves from the vault accounts
        let vault_a_str = vault_a.to_string();
        let vault_b_str = vault_b.to_string();

        let (vault_a_balance, vault_b_balance) = match
            self.get_vault_balances(&vault_a_str, &vault_b_str).await
        {
            Ok((va, vb)) => {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "CLMM_VAULT_SUCCESS",
                        &format!(
                            "Successfully fetched Raydium CLMM vault balances:\n\
                            - Vault A {} (SOL) balance: {}\n\
                            - Vault B {} (token) balance: {}",
                            vault_a_str,
                            va,
                            vault_b_str,
                            vb
                        )
                    );
                }
                (va, vb)
            }
            Err(e) => {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "CLMM_VAULT_ERROR",
                        &format!("Vault balance fetch failed: {}", e)
                    );
                }
                return Err(format!("Failed to get vault balances for Raydium CLMM pool: {}", e));
            }
        };

        // Use decimal cache system with pool data as fallback
        let mint_a_decimals = get_cached_decimals(&mint_a.to_string()).unwrap_or(mint_decimals_a);
        let mint_b_decimals = get_cached_decimals(&mint_b.to_string()).unwrap_or(mint_decimals_b);

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "CLMM_DECODE",
                &format!(
                    "Raydium CLMM pool {} decoded:\n\
                    - Token A (SOL): {} ({} decimals, {} reserve)\n\
                    - Token B (target): {} ({} decimals, {} reserve)\n\
                    - Token Vault A: {}\n\
                    - Token Vault B: {}\n\
                    - Liquidity: {}\n\
                    - Sqrt Price X64: {}",
                    pool_address,
                    mint_a,
                    mint_a_decimals,
                    vault_a_balance,
                    mint_b,
                    mint_b_decimals,
                    vault_b_balance,
                    vault_a,
                    vault_b,
                    liquidity,
                    sqrt_price_x64
                )
            );
        }

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_program_id: RAYDIUM_CLMM_PROGRAM_ID.to_string(),
            pool_type: get_pool_program_display_name(RAYDIUM_CLMM_PROGRAM_ID),
            token_0_mint: mint_a.to_string(), // SOL
            token_1_mint: mint_b.to_string(), // Target token
            token_0_vault: Some(vault_a.to_string()),
            token_1_vault: Some(vault_b.to_string()),
            token_0_reserve: vault_a_balance, // SOL reserve
            token_1_reserve: vault_b_balance, // Token reserve
            token_0_decimals: mint_a_decimals,
            token_1_decimals: mint_b_decimals,
            lp_mint: None, // CLMM uses concentrated liquidity
            lp_supply: Some(liquidity as u64), // Use liquidity value
            creator: Some(_creator),
            status: None,
            liquidity_usd: None,
            sqrt_price: Some(sqrt_price_x64), // Store sqrt_price for concentrated liquidity calculation
        })
    }

    /// Calculate price for Raydium CLMM pool
    async fn calculate_raydium_clmm_price(
        &self,
        pool_info: &PoolInfo,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // Determine which token is SOL and which is the target token
        let (sol_reserve, token_reserve, sol_decimals, token_decimals, _is_token_0) = if
            pool_info.token_0_mint == SOL_MINT &&
            pool_info.token_1_mint == token_mint
        {
            (
                pool_info.token_0_reserve,
                pool_info.token_1_reserve,
                pool_info.token_0_decimals,
                pool_info.token_1_decimals,
                false,
            )
        } else if pool_info.token_1_mint == SOL_MINT && pool_info.token_0_mint == token_mint {
            (
                pool_info.token_1_reserve,
                pool_info.token_0_reserve,
                pool_info.token_1_decimals,
                pool_info.token_0_decimals,
                true,
            )
        } else {
            return Err(format!("Pool does not contain SOL or target token {}", token_mint));
        };

        // Validate reserves
        if sol_reserve == 0 || token_reserve == 0 {
            return Err("Pool has zero reserves".to_string());
        }

        // For CLMM pools, we can use the vault balances for basic price calculation
        // The sqrt_price is also available for more precise calculations, but for now
        // we'll use the same approach as regular AMM pools
        let sol_adjusted = (sol_reserve as f64) / (10_f64).powi(sol_decimals as i32);
        let token_adjusted = (token_reserve as f64) / (10_f64).powi(token_decimals as i32);

        let price_sol = sol_adjusted / token_adjusted;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "CALC",
                &format!(
                    "Raydium CLMM Price Calculation for {}:\n  \
                     SOL Reserve: {} ({:.9} adjusted, {} decimals)\n  \
                     Token Reserve: {} ({:.9} adjusted, {} decimals)\n  \
                     Price: {:.9} SOL\n  \
                     Pool: {} ({})\n  \
                     Sqrt Price X64: {:?}",
                    token_mint,
                    sol_reserve,
                    sol_adjusted,
                    sol_decimals,
                    token_reserve,
                    token_adjusted,
                    token_decimals,
                    price_sol,
                    pool_info.pool_address,
                    pool_info.pool_type,
                    pool_info.sqrt_price
                )
            );

            // Additional validation checks
            if sol_adjusted <= 0.0 || token_adjusted <= 0.0 {
                log(
                    LogTag::Pool,
                    "CALC_WARN",
                    &format!(
                        "WARNING: Zero or negative adjusted values detected! \
                         SOL_adj: {:.9}, Token_adj: {:.9}",
                        sol_adjusted,
                        token_adjusted
                    )
                );
            }

            // Check for extremely small or large prices that might indicate decimal issues
            if price_sol < 0.000000001 || price_sol > 1000.0 {
                log(
                    LogTag::Pool,
                    "CALC_WARN",
                    &format!(
                        "WARNING: Unusual price detected: {:.12} SOL. \
                         Check if decimals are correct (SOL: {}, Token: {})",
                        price_sol,
                        sol_decimals,
                        token_decimals
                    )
                );
            }
        }

        Ok(
            Some(PoolPriceInfo {
                pool_address: pool_info.pool_address.clone(),
                pool_program_id: pool_info.pool_program_id.clone(),
                pool_type: pool_info.pool_type.clone(),
                token_mint: token_mint.to_string(),
                price_sol,
                token_reserve,
                sol_reserve,
                token_decimals,
                sol_decimals,
            })
        )
    }

    /// Get vault token balances
    /// Extract reserves directly from Raydium Legacy AMM pool data
    /// This is a fallback when vault addresses are incorrect or inaccessible
    fn extract_raydium_legacy_reserves_from_data(
        &self,
        data: &[u8]
    ) -> Result<Vec<(u64, u64)>, String> {
        let mut reserve_pairs = Vec::new();

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "LEGACY_EXTRACT_START",
                "🔍 Starting Raydium Legacy reserve extraction from pool data"
            );
        }

        // Based on hex analysis, the mints are at:
        // - SOL mint at offset 400 (0x190)
        // - Token mint at offset 432 (0x1b0)

        // And potential reserves are at:
        // - Offset 208-216: Most promising pair (33547458368970, 40683086513379)
        // These numbers make sense for a pool with ~$20M liquidity

        let promising_offsets = [
            (208, 216), // Primary candidate: Found these in hex dump analysis
            (256, 272), // Secondary: Also large values that could be reserves
            (288, 296), // Backup: Alternative reserve location
        ];

        for &(offset1, offset2) in &promising_offsets {
            if offset1 + 8 <= data.len() && offset2 + 8 <= data.len() {
                let reserve1 = u64::from_le_bytes(
                    data[offset1..offset1 + 8].try_into().map_err(|_| "Failed to read reserve1")?
                );
                let reserve2 = u64::from_le_bytes(
                    data[offset2..offset2 + 8].try_into().map_err(|_| "Failed to read reserve2")?
                );

                // For Raydium Legacy with ~$20M liquidity, reserves should be substantial
                // but not astronomical
                if
                    reserve1 > 10_000_000 &&
                    reserve1 < 1_000_000_000_000_000 &&
                    reserve2 > 10_000_000 &&
                    reserve2 < 1_000_000_000_000_000
                {
                    if self.debug_enabled {
                        log(
                            LogTag::Pool,
                            "LEGACY_RESERVES_FOUND",
                            &format!(
                                "Found reserves at offsets {} and {}: {} and {}",
                                offset1,
                                offset2,
                                reserve1,
                                reserve2
                            )
                        );
                    }

                    reserve_pairs.push((reserve1, reserve2));
                }
            }
        }

        if reserve_pairs.is_empty() {
            return Err("No reasonable reserve pairs found in Raydium Legacy pool data".to_string());
        }

        // Return the most promising pair first (offset 208-216)
        Ok(reserve_pairs)
    }

    async fn get_vault_balances(&self, vault_0: &str, vault_1: &str) -> Result<(u64, u64), String> {
        let vault_0_pubkey = Pubkey::from_str(vault_0).map_err(|e|
            format!("Invalid vault 0 address {}: {}", vault_0, e)
        )?;
        let vault_1_pubkey = Pubkey::from_str(vault_1).map_err(|e|
            format!("Invalid vault 1 address {}: {}", vault_1, e)
        )?;

        // Always fetch fresh vault balances - NO CACHING of balance values per requirements
        let pubkeys = vec![vault_0_pubkey, vault_1_pubkey];

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "VAULT_FETCH_FRESH",
                &format!("Fetching fresh vault balances for 2 accounts (no cache)")
            );
        }

        let accounts = self.rpc_client
            .get_multiple_accounts(&pubkeys).await
            .map_err(|e| format!("Failed to get vault accounts: {}", e))?;

        let balance_0 = if let Some(acct) = &accounts[0] {
            Self::decode_token_account_amount(&acct.data).map_err(|e|
                format!("Failed to decode vault 0 balance: {}", e)
            )?
        } else {
            return Err("Vault 0 account not found".to_string());
        };

        let balance_1 = if let Some(acct) = &accounts[1] {
            Self::decode_token_account_amount(&acct.data).map_err(|e|
                format!("Failed to decode vault 1 balance: {}", e)
            )?
        } else {
            return Err("Vault 1 account not found".to_string());
        };

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "VAULT",
                &format!(
                    "Vault balances - Vault0 ({}): {}, Vault1 ({}): {}",
                    vault_0,
                    balance_0,
                    vault_1,
                    balance_1
                )
            );
        }

        // EXTREME DEBUG: Log the actual account data hashes to detect if RPC is returning cached data
        if is_debug_pool_prices_enabled() {
            let hash_0 = if let Some(acct) = &accounts[0] {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                std::hash::Hash::hash(&acct.data, &mut hasher);
                std::hash::Hasher::finish(&hasher)
            } else {
                0
            };

            let hash_1 = if let Some(acct) = &accounts[1] {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                std::hash::Hash::hash(&acct.data, &mut hasher);
                std::hash::Hasher::finish(&hasher)
            } else {
                0
            };

            log(
                LogTag::Pool,
                "VAULT_DEBUG",
                &format!(
                    "Account data hashes - Vault0: {}, Vault1: {} (if same every time = RPC caching)",
                    hash_0,
                    hash_1
                )
            );
        }

        Ok((balance_0, balance_1))
    }

    /// Get DLMM token reserve balances from token accounts
    async fn get_dlmm_vault_balances(
        &self,
        reserve_0: &str,
        reserve_1: &str
    ) -> Result<(u64, u64), String> {
        // Always fetch fresh DLMM reserve balances - NO CACHING of balance values per requirements
        let reserve_0_pubkey = Pubkey::from_str(reserve_0).map_err(|e|
            format!("Invalid DLMM reserve 0 address {}: {}", reserve_0, e)
        )?;
        let reserve_1_pubkey = Pubkey::from_str(reserve_1).map_err(|e|
            format!("Invalid DLMM reserve 1 address {}: {}", reserve_1, e)
        )?;

        let pubkeys = vec![reserve_0_pubkey, reserve_1_pubkey];

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "DLMM_FETCH_FRESH",
                &format!("Fetching fresh DLMM reserve balances for 2 accounts (no cache)")
            );
        }

        let accounts = self.rpc_client
            .get_multiple_accounts(&pubkeys).await
            .map_err(|e| format!("Failed to get DLMM reserve accounts: {}", e))?;

        let balance_0 = if let Some(acct) = &accounts[0] {
            Self::decode_token_account_amount(&acct.data).map_err(|e|
                format!("Failed to decode DLMM reserve 0 balance: {}", e)
            )?
        } else {
            return Err(format!("DLMM reserve 0 account {} not found", reserve_0));
        };

        let balance_1 = if let Some(acct) = &accounts[1] {
            Self::decode_token_account_amount(&acct.data).map_err(|e|
                format!("Failed to decode DLMM reserve 1 balance: {}", e)
            )?
        } else {
            return Err(format!("DLMM reserve 1 account {} not found", reserve_1));
        };

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "DLMM_RESERVE",
                &format!(
                    "DLMM reserve balances - Reserve0 ({}): {}, Reserve1 ({}): {}",
                    reserve_0,
                    balance_0,
                    reserve_1,
                    balance_1
                )
            );
        }

        Ok((balance_0, balance_1))
    }

    /// Extract Pump.fun vault addresses from pool account data
    /// Returns (base_vault, quote_vault) addresses
    fn extract_pump_fun_vault_addresses(&self, data: &[u8]) -> Result<(String, String), String> {
        if data.len() < 200 {
            return Err("Invalid Pump.fun pool account data length".to_string());
        }

        let mut offset = 8; // Skip discriminator

        // Skip pool_bump (u8) and index (u16)
        offset += 1 + 2;

        // Skip creator pubkey
        offset += 32;

        // Skip base_mint and quote_mint
        offset += 32 + 32;

        // Skip lp_mint
        offset += 32;

        // Extract vault addresses
        let base_vault = Self::read_pubkey_at_offset(data, &mut offset).map_err(|e|
            format!("Failed to read base vault: {}", e)
        )?;
        let quote_vault = Self::read_pubkey_at_offset(data, &mut offset).map_err(|e|
            format!("Failed to read quote vault: {}", e)
        )?;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "PUMP_EXTRACT_VAULTS",
                &format!(
                    "Extracted Pump.fun vault addresses: base={}, quote={}",
                    base_vault,
                    quote_vault
                )
            );
        }

        Ok((base_vault.to_string(), quote_vault.to_string()))
    }

    /// Decode Raydium Legacy AMM pool data from account bytes
    async fn decode_raydium_legacy_amm_pool(
        &self,
        pool_address: &str,
        account: &Account
    ) -> Result<PoolInfo, String> {
        if account.data.len() < 752 {
            return Err("Invalid Raydium Legacy AMM pool account data length".to_string());
        }

        let data = &account.data;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "LEGACY_DEBUG",
                &format!(
                    "Raydium Legacy AMM pool {} - data length: {} bytes, analyzing structure...",
                    pool_address,
                    data.len()
                )
            );

            // Hex dump for structure analysis
            let hex_sample = data
                .iter()
                .take(200)
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ");
            log(LogTag::Pool, "LEGACY_HEX", &format!("First 200 bytes: {}", hex_sample));
        }

        // Raydium Legacy AMM structure (based on actual data analysis)
        // From hex analysis, the real mints are at different offsets than CPMM
        // Skip the initial structure fields and go directly to the important addresses
        let mut offset = 0x190; // Jump to where SOL mint is found

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "LEGACY_PARSE_START",
                &format!(
                    "Starting Legacy AMM parsing at offset 0x{:x} (based on pubkey scan)",
                    offset
                )
            );
        }

        // Extract addresses at the known correct offsets
        let pc_mint = Self::read_pubkey_at_offset(data, &mut offset)?; // SOL mint at 0x190
        offset = 0x1b0; // Jump to token mint location
        let coin_mint = Self::read_pubkey_at_offset(data, &mut offset)?; // Token mint at 0x1b0

        // Vault addresses are at the correct offsets we found from hex analysis
        offset = 0x150; // Base vault (SOL) at offset 0x150
        let base_vault = Self::read_pubkey_at_offset(data, &mut offset)?;
        let quote_vault = Self::read_pubkey_at_offset(data, &mut offset)?; // Quote vault (Token) at offset 0x160

        // Map vaults correctly: pc_mint=SOL uses base_vault, coin_mint=token uses quote_vault
        let (coin_vault, pc_vault) = (quote_vault.clone(), base_vault.clone());

        // Use decimal values from cache (since we skipped the pool structure parsing)
        let token_0_decimals = get_cached_decimals(&coin_mint.to_string()).unwrap_or(9);
        let token_1_decimals = get_cached_decimals(&pc_mint.to_string()).unwrap_or(9); // SOL is 9 decimals

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "LEGACY_DECODE",
                &format!(
                    "Raydium Legacy AMM pool {} parsed:\n  \
                     - Coin Mint: {} (decimals: {})\n  \
                     - PC Mint: {} (decimals: {})\n  \
                     - Coin Vault: {}\n  \
                     - PC Vault: {}",
                    pool_address,
                    coin_mint,
                    token_0_decimals,
                    pc_mint,
                    token_1_decimals,
                    coin_vault,
                    pc_vault
                )
            );

            // Additional debugging: scan for pubkeys at various offsets
            log(LogTag::Pool, "LEGACY_PUBKEY_SCAN", "Scanning for pubkeys at various offsets:");
            for test_offset in [
                0x150, 0x160, 0x170, 0x180, 0x190, 0x1a0, 0x1b0, 0x1c0, 0x1d0, 0x1e0, 0x1f0, 0x200,
            ].iter() {
                if *test_offset + 32 <= data.len() {
                    if let Ok(pubkey_bytes) = data[*test_offset..*test_offset + 32].try_into() {
                        let test_pubkey = Pubkey::new_from_array(pubkey_bytes);
                        log(
                            LogTag::Pool,
                            "LEGACY_PUBKEY_SCAN",
                            &format!("  Offset 0x{:x}: {}", test_offset, test_pubkey)
                        );
                    }
                }
            }
        }

        // Get vault balances to calculate reserves - vault balances are the TRUE trading reserves
        let (token_0_reserve, token_1_reserve) = {
            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "LEGACY_VAULT_PRIORITY",
                    "Vault balances are the actual trading reserves, not PnL values from pool data"
                );
            }

            // ALWAYS try vault balance fetch first - this is the accurate method
            match
                tokio::time::timeout(
                    Duration::from_secs(10), // Increased timeout for better reliability
                    self.get_vault_balances(&coin_vault.to_string(), &pc_vault.to_string())
                ).await
            {
                Ok(Ok((coin_reserve, pc_reserve))) => {
                    if self.debug_enabled {
                        log(
                            LogTag::Pool,
                            "LEGACY_VAULT_SUCCESS",
                            &format!(
                                "Vault balances fetched successfully:\n  \
                                     - Coin Vault ({}): {} {} tokens\n  \
                                     - PC Vault ({}): {} {} tokens",
                                coin_vault,
                                coin_reserve,
                                coin_mint,
                                pc_vault,
                                pc_reserve,
                                pc_mint
                            )
                        );
                    }
                    (coin_reserve, pc_reserve)
                }
                Ok(Err(e)) => {
                    if self.debug_enabled {
                        log(
                            LogTag::Pool,
                            "LEGACY_VAULT_ERROR",
                            &format!("Vault balance fetch failed: {}. Using PnL fallback (less accurate)...", e)
                        );
                    }

                    // Fallback: Use PnL values but with correct assignment based on pool data structure
                    // From pool data: quoteTotalPnl=33547458368970, baseTotalPnl=40683086513379
                    // base=SOL (9 decimals), quote=token (6 decimals)
                    match self.extract_raydium_legacy_reserves_from_data(data) {
                        Ok(reserves) if !reserves.is_empty() => {
                            let (pnl_val1, pnl_val2) = reserves[0];
                            // These are likely quoteTotalPnl and baseTotalPnl
                            // Based on pool structure: coin_mint=token, pc_mint=SOL
                            // So coin_reserve should be smaller (token), pc_reserve should be larger (SOL)
                            let (coin_reserve, pc_reserve) = if pnl_val2 > pnl_val1 {
                                (pnl_val1, pnl_val2) // Assign smaller to token, larger to SOL
                            } else {
                                (pnl_val2, pnl_val1) // Ensure SOL gets the larger value
                            };

                            if self.debug_enabled {
                                log(
                                    LogTag::Pool,
                                    "LEGACY_PNL_FALLBACK",
                                    &format!(
                                        "Using PnL values as fallback:\n  \
                                             - Coin (token): {}\n  \
                                             - PC (SOL): {} (assigned larger value)",
                                        coin_reserve,
                                        pc_reserve
                                    )
                                );
                            }
                            (coin_reserve, pc_reserve)
                        }
                        _ => {
                            return Err(
                                format!("Both vault balance fetch and PnL extraction failed: {}", e)
                            );
                        }
                    }
                }
                Err(_) => {
                    return Err("Vault balance fetch timed out after 5 seconds".to_string());
                }
            }
        };

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "LEGACY_RESERVES",
                &format!(
                    "Raydium Legacy AMM {} reserves:\n  \
                     - Coin Reserve: {} (vault: {})\n  \
                     - PC Reserve: {} (vault: {})",
                    pool_address,
                    token_0_reserve,
                    coin_vault,
                    token_1_reserve,
                    pc_vault
                )
            );
        }

        // Map coin/pc reserves back to token_0/token_1 for consistent interface
        let (token_0_reserve, token_1_reserve) = (token_0_reserve, token_1_reserve);

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_program_id: RAYDIUM_LEGACY_AMM_PROGRAM_ID.to_string(),
            pool_type: get_pool_program_display_name(RAYDIUM_LEGACY_AMM_PROGRAM_ID),
            token_0_mint: coin_mint.to_string(),
            token_1_mint: pc_mint.to_string(),
            token_0_vault: Some(coin_vault.to_string()),
            token_1_vault: Some(pc_vault.to_string()),
            token_0_reserve,
            token_1_reserve,
            token_0_decimals,
            token_1_decimals,
            lp_mint: None, // Legacy AMM might not have LP mint in same structure
            lp_supply: None,
            creator: None,
            status: None, // We skipped status parsing for now
            liquidity_usd: None,
            sqrt_price: None, // Not applicable to AMM pools
        })
    }

    /// Calculate price for Raydium Legacy AMM pool
    async fn calculate_raydium_legacy_amm_price(
        &self,
        pool_info: &PoolInfo,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        if self.debug_enabled {
            log(
                LogTag::Pool,
                "LEGACY_PRICE_CALC",
                &format!(
                    "Calculating Raydium Legacy AMM price for token {} in pool {}",
                    token_mint,
                    pool_info.pool_address
                )
            );
        }

        // Determine which token is SOL and which is the target token
        let (sol_reserve, token_reserve, sol_decimals, token_decimals, is_token_0) = if
            pool_info.token_0_mint == SOL_MINT &&
            pool_info.token_1_mint == token_mint
        {
            (
                pool_info.token_0_reserve,
                pool_info.token_1_reserve,
                pool_info.token_0_decimals,
                pool_info.token_1_decimals,
                false,
            )
        } else if pool_info.token_1_mint == SOL_MINT && pool_info.token_0_mint == token_mint {
            (
                pool_info.token_1_reserve,
                pool_info.token_0_reserve,
                pool_info.token_1_decimals,
                pool_info.token_0_decimals,
                true,
            )
        } else {
            return Err(
                format!("Legacy AMM pool does not contain SOL or target token {}", token_mint)
            );
        };

        // Validate reserves
        if sol_reserve == 0 || token_reserve == 0 {
            return Err("Legacy AMM pool has zero reserves".to_string());
        }

        // Calculate price: price = sol_reserve / token_reserve (adjusted for decimals)
        let sol_adjusted = (sol_reserve as f64) / (10_f64).powi(sol_decimals as i32);
        let token_adjusted = (token_reserve as f64) / (10_f64).powi(token_decimals as i32);

        let price_sol = sol_adjusted / token_adjusted;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "LEGACY_PRICE",
                &format!(
                    "Raydium Legacy AMM price calculation:\n  \
                     - SOL Reserve: {} (decimals: {})\n  \
                     - Token Reserve: {} (decimals: {})\n  \
                     - SOL Adjusted: {:.12}\n  \
                     - Token Adjusted: {:.12}\n  \
                     - Final Price: {:.12} SOL",
                    sol_reserve,
                    sol_decimals,
                    token_reserve,
                    token_decimals,
                    sol_adjusted,
                    token_adjusted,
                    price_sol
                )
            );
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "LEGACY_SUCCESS",
                &format!("Raydium Legacy AMM price calculated: {:.12} SOL", price_sol)
            );
        }

        Ok(
            Some(PoolPriceInfo {
                pool_address: pool_info.pool_address.clone(),
                pool_program_id: pool_info.pool_program_id.clone(),
                pool_type: pool_info.pool_type.clone(),
                token_mint: token_mint.to_string(),
                price_sol,
                token_reserve,
                sol_reserve,
                token_decimals,
                sol_decimals,
            })
        )
    }

    /// Decode Meteora DAMM v2 pool data from account bytes
    async fn decode_meteora_damm_v2_pool(
        &self,
        pool_address: &str,
        account: &Account
    ) -> Result<PoolInfo, String> {
        if account.data.len() < 1112 {
            // Meteora pools should be at least 1112 bytes based on your data
            return Err(
                format!(
                    "Invalid Meteora DAMM v2 pool account data length: {} (expected >= 1112)",
                    account.data.len()
                )
            );
        }

        let data = &account.data;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DECODE",
                &format!(
                    "Starting Meteora DAMM v2 decode for pool {}, data length: {}",
                    pool_address,
                    data.len()
                )
            );
        }

        // Decode Meteora DAMM v2 pool structure based on the provided JSON data
        // The structure appears to follow this layout:

        // Skip the complex pool_fees structure (starts at offset 8, quite large)
        // Jump to token mints which start around offset 136 based on your data
        let mut offset = 136;

        let token_a_mint = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_b_mint = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_a_vault = Self::read_pubkey_at_offset(data, &mut offset)?;
        let token_b_vault = Self::read_pubkey_at_offset(data, &mut offset)?;

        // Skip whitelisted_vault and partner
        offset += 64;

        // Read liquidity (u128)
        let liquidity = Self::read_u128_at_offset(data, &mut offset)?;

        // Skip _padding (u128)
        offset += 16;

        // Read protocol fees
        let protocol_a_fee = Self::read_u64_at_offset(data, &mut offset)?;
        let protocol_b_fee = Self::read_u64_at_offset(data, &mut offset)?;

        // Skip partner fees
        offset += 16;

        // Read sqrt prices
        let sqrt_min_price = Self::read_u128_at_offset(data, &mut offset)?;
        let sqrt_max_price = Self::read_u128_at_offset(data, &mut offset)?;
        let sqrt_price = Self::read_u128_at_offset(data, &mut offset)?;

        // Read activation point
        let activation_point = Self::read_u64_at_offset(data, &mut offset)?;

        // Read status flags
        let activation_type = Self::read_u8_at_offset(data, &mut offset)?;
        let pool_status = Self::read_u8_at_offset(data, &mut offset)?;
        let token_a_flag = Self::read_u8_at_offset(data, &mut offset)?;
        let token_b_flag = Self::read_u8_at_offset(data, &mut offset)?;
        let collect_fee_mode = Self::read_u8_at_offset(data, &mut offset)?;
        let pool_type = Self::read_u8_at_offset(data, &mut offset)?;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_TOKENS",
                &format!(
                    "Token A: {}, Token B: {}, Vaults A: {}, B: {}",
                    token_a_mint,
                    token_b_mint,
                    token_a_vault,
                    token_b_vault
                )
            );

            log(
                LogTag::Pool,
                "METEORA_PRICE",
                &format!(
                    "sqrt_price: {}, liquidity: {}, status: {}",
                    sqrt_price,
                    liquidity,
                    pool_status
                )
            );
        }

        // Get token decimals
        let token_a_decimals_opt = get_token_decimals_with_cache(&token_a_mint).await;
        let token_b_decimals_opt = get_token_decimals_with_cache(&token_b_mint).await;

        // Check if decimals are available for both tokens
        let token_a_decimals = match token_a_decimals_opt {
            Some(decimals) => decimals,
            None => {
                return Err(format!("Cannot determine decimals for token A: {}", token_a_mint));
            }
        };

        let token_b_decimals = match token_b_decimals_opt {
            Some(decimals) => decimals,
            None => {
                return Err(format!("Cannot determine decimals for token B: {}", token_b_mint));
            }
        };

        // Get vault balances to calculate reserves
        let (token_a_reserve, token_b_reserve) = self.get_vault_balances(
            &token_a_vault,
            &token_b_vault
        ).await?;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_RESERVES",
                &format!(
                    "Token A reserve: {} (decimals: {}), Token B reserve: {} (decimals: {})",
                    token_a_reserve,
                    token_a_decimals,
                    token_b_reserve,
                    token_b_decimals
                )
            );
        }

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_program_id: METEORA_DAMM_V2_PROGRAM_ID.to_string(),
            pool_type: get_pool_program_display_name(METEORA_DAMM_V2_PROGRAM_ID),
            token_0_mint: token_a_mint,
            token_1_mint: token_b_mint,
            token_0_vault: Some(token_a_vault),
            token_1_vault: Some(token_b_vault),
            token_0_reserve: token_a_reserve,
            token_1_reserve: token_b_reserve,
            token_0_decimals: token_a_decimals,
            token_1_decimals: token_b_decimals,
            lp_mint: None, // Meteora DAMM v2 doesn't use traditional LP tokens
            lp_supply: Some(liquidity as u64), // Store liquidity here
            creator: None,
            status: Some(pool_status as u32),
            liquidity_usd: None, // Will be calculated separately
            sqrt_price: None, // Not applicable to AMM pools
        })
    }

    /// Calculate price for Meteora DAMM v2 pool
    async fn calculate_meteora_damm_v2_price(
        &self,
        pool_info: &PoolInfo,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // Determine which token is SOL and which is the target token
        let (sol_reserve, token_reserve, sol_decimals, token_decimals, _is_token_a) = if
            pool_info.token_0_mint == SOL_MINT &&
            pool_info.token_1_mint == token_mint
        {
            (
                pool_info.token_0_reserve,
                pool_info.token_1_reserve,
                pool_info.token_0_decimals,
                pool_info.token_1_decimals,
                false,
            )
        } else if pool_info.token_1_mint == SOL_MINT && pool_info.token_0_mint == token_mint {
            (
                pool_info.token_1_reserve,
                pool_info.token_0_reserve,
                pool_info.token_1_decimals,
                pool_info.token_0_decimals,
                true,
            )
        } else {
            return Err(format!("Pool does not contain SOL or target token {}", token_mint));
        };

        // Validate reserves
        if sol_reserve == 0 || token_reserve == 0 {
            return Err("Pool has zero reserves".to_string());
        }

        // Calculate price: price = sol_reserve / token_reserve (adjusted for decimals)
        let sol_adjusted = (sol_reserve as f64) / (10_f64).powi(sol_decimals as i32);
        let token_adjusted = (token_reserve as f64) / (10_f64).powi(token_decimals as i32);

        let price_sol = sol_adjusted / token_adjusted;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "CALC",
                &format!(
                    "Meteora DAMM v2 Price Calculation for {}:\n  \
                     SOL Reserve: {} ({:.12} adjusted, {} decimals)\n  \
                     Token Reserve: {} ({:.12} adjusted, {} decimals)\n  \
                     Price: {:.12} SOL\n  \
                     Pool: {} ({})",
                    token_mint,
                    sol_reserve,
                    sol_adjusted,
                    sol_decimals,
                    token_reserve,
                    token_adjusted,
                    token_decimals,
                    price_sol,
                    pool_info.pool_address,
                    pool_info.pool_type
                )
            );
        }

        Ok(
            Some(PoolPriceInfo {
                pool_address: pool_info.pool_address.clone(),
                pool_program_id: pool_info.pool_program_id.clone(),
                pool_type: pool_info.pool_type.clone(),
                token_mint: token_mint.to_string(),
                price_sol,
                token_reserve,
                sol_reserve,
                token_decimals,
                sol_decimals,
            })
        )
    }

    /// Decode Meteora DLMM pool data from account bytes
    async fn decode_meteora_dlmm_pool(
        &self,
        pool_address: &str,
        account: &Account
    ) -> Result<PoolInfo, String> {
        let data = &account.data;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_DECODE",
                &format!(
                    "Starting Meteora DLMM decode for pool {}, data length: {}",
                    pool_address,
                    data.len()
                )
            );
        }

        if data.len() < 216 {
            return Err(format!("DLMM pool data too short: {} bytes", data.len()));
        }

        // Extract pubkeys at known offsets (from hex dump analysis)
        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_EXTRACT",
                &format!("Extracting pubkeys from data length {}", data.len())
            );
        }

        let token_x_mint = extract_pubkey_at_offset(data, 88)?;
        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_TOKEN_X",
                &format!("Extracted token_x_mint: {}", token_x_mint)
            );
        }

        let token_y_mint = extract_pubkey_at_offset(data, 120)?;
        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_TOKEN_Y",
                &format!("Extracted token_y_mint: {}", token_y_mint)
            );
        }

        let reserve_x = extract_pubkey_at_offset(data, 152)?;
        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_RESERVE_X",
                &format!("Extracted reserve_x: {}", reserve_x)
            );
        }

        let reserve_y = extract_pubkey_at_offset(data, 184)?;
        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_RESERVE_Y",
                &format!("Extracted reserve_y: {}", reserve_y)
            );
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_STRUCT",
                &format!(
                    "DLMM Pool Structure - token_x: {}, token_y: {}, reserve_x: {}, reserve_y: {}",
                    token_x_mint,
                    token_y_mint,
                    reserve_x,
                    reserve_y
                )
            );
        }

        // Verify we have SOL as one of the tokens
        let sol_mint = "So11111111111111111111111111111111111111112";
        let (token_mint, sol_reserve, token_reserve, token_decimals_to_use) = if
            token_y_mint.to_string() == sol_mint
        {
            // token_x is the custom token, token_y is SOL
            let token_decimals = get_token_decimals_with_cache(
                &token_x_mint.to_string()
            ).await.unwrap_or(9);
            (token_x_mint.to_string(), reserve_y.to_string(), reserve_x.to_string(), token_decimals)
        } else if token_x_mint.to_string() == sol_mint {
            // token_x is SOL, token_y is the custom token
            let token_decimals = get_token_decimals_with_cache(
                &token_y_mint.to_string()
            ).await.unwrap_or(9);
            (token_y_mint.to_string(), reserve_x.to_string(), reserve_y.to_string(), token_decimals)
        } else {
            return Err("Pool doesn't contain SOL".to_string());
        };

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_PAIR",
                &format!(
                    "Identified token: {}, SOL reserve: {}, Token reserve: {}",
                    token_mint,
                    sol_reserve,
                    token_reserve
                )
            );
        }

        // Get reserve balances from vault accounts
        let (sol_balance, token_balance) = if token_y_mint.to_string() == sol_mint {
            // token_x is the custom token, token_y is SOL -> (sol_balance, token_balance)
            let (token_bal, sol_bal) = self.get_vault_balances(
                &reserve_x.to_string(),
                &reserve_y.to_string()
            ).await?;
            (sol_bal, token_bal)
        } else {
            // token_x is SOL, token_y is the custom token -> (sol_balance, token_balance)
            let (sol_bal, token_bal) = self.get_vault_balances(
                &reserve_x.to_string(),
                &reserve_y.to_string()
            ).await?;
            (sol_bal, token_bal)
        };

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_BALANCES",
                &format!(
                    "SOL balance: {} lamports, Token balance: {} raw units",
                    sol_balance,
                    token_balance
                )
            );
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_DECIMALS",
                &format!("Token {} has {} decimals", token_mint, token_decimals_to_use)
            );
        }

        // Calculate price: SOL per token
        let sol_decimals = 9u8;

        if token_balance == 0 {
            return Err("Token reserve is empty".to_string());
        }

        let price_sol =
            (sol_balance as f64) /
            (10_f64).powi(sol_decimals as i32) /
            ((token_balance as f64) / (10_f64).powi(token_decimals_to_use as i32));

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_PRICE",
                &format!("Calculated price: {:.12} SOL per token", price_sol)
            );
        }

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_program_id: METEORA_DLMM_PROGRAM_ID.to_string(),
            pool_type: get_pool_program_display_name(METEORA_DLMM_PROGRAM_ID),
            token_0_mint: token_x_mint.to_string(),
            token_1_mint: token_y_mint.to_string(),
            token_0_vault: Some(reserve_x.to_string()),
            token_1_vault: Some(reserve_y.to_string()),
            token_0_reserve: if token_x_mint.to_string() == sol_mint {
                sol_balance
            } else {
                token_balance
            },
            token_1_reserve: if token_y_mint.to_string() == sol_mint {
                sol_balance
            } else {
                token_balance
            },
            token_0_decimals: if token_x_mint.to_string() == sol_mint {
                9
            } else {
                token_decimals_to_use
            },
            token_1_decimals: if token_y_mint.to_string() == sol_mint {
                9
            } else {
                token_decimals_to_use
            },
            lp_mint: None,
            lp_supply: None,
            creator: None,
            status: Some(0),
            liquidity_usd: None,
            sqrt_price: None, // Meteora DLMM is concentrated liquidity but we don't extract sqrt_price yet
        })
    }

    /// Calculate price for Meteora DLMM pool
    async fn calculate_meteora_dlmm_price(
        &self,
        pool_info: &PoolInfo,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // Determine which token is SOL and which is the target token
        let (sol_reserve, token_reserve, sol_decimals, token_decimals, _is_token_x) = if
            pool_info.token_0_mint == SOL_MINT &&
            pool_info.token_1_mint == token_mint
        {
            (
                pool_info.token_0_reserve,
                pool_info.token_1_reserve,
                pool_info.token_0_decimals,
                pool_info.token_1_decimals,
                true,
            )
        } else if pool_info.token_1_mint == SOL_MINT && pool_info.token_0_mint == token_mint {
            (
                pool_info.token_1_reserve,
                pool_info.token_0_reserve,
                pool_info.token_1_decimals,
                pool_info.token_0_decimals,
                false,
            )
        } else {
            return Err(
                format!(
                    "DLMM pool {} does not contain SOL mint. Token0: {}, Token1: {}, Target: {}",
                    pool_info.pool_address,
                    pool_info.token_0_mint,
                    pool_info.token_1_mint,
                    token_mint
                )
            );
        };

        // Validate reserves
        if sol_reserve == 0 || token_reserve == 0 {
            return Err(
                format!(
                    "DLMM pool {} has zero reserves. SOL: {}, Token: {}",
                    pool_info.pool_address,
                    sol_reserve,
                    token_reserve
                )
            );
        }

        // Calculate price in SOL: price = (SOL reserves * 10^token_decimals) / (token reserves * 10^SOL_decimals)
        let price_sol =
            ((sol_reserve as f64) * (10f64).powi(token_decimals as i32)) /
            ((token_reserve as f64) * (10f64).powi(sol_decimals as i32));

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "METEORA_DLMM_PRICE",
                &format!(
                    "DLMM Price calculation:\n\
                    - SOL Reserve: {} (decimals: {})\n\
                    - Token Reserve: {} (decimals: {})\n\
                    - Price SOL: {:.12}",
                    sol_reserve,
                    sol_decimals,
                    token_reserve,
                    token_decimals,
                    price_sol
                )
            );
        }

        Ok(
            Some(PoolPriceInfo {
                pool_address: pool_info.pool_address.clone(),
                pool_program_id: pool_info.pool_program_id.clone(),
                pool_type: pool_info.pool_type.clone(),
                token_mint: token_mint.to_string(),
                price_sol,
                token_reserve,
                sol_reserve,
                token_decimals,
                sol_decimals,
            })
        )
    }

    /// Calculate token price for Orca Whirlpool pools
    ///
    /// Orca Whirlpool uses concentrated liquidity with sqrt price
    /// We can calculate the price from vault balances for simplicity
    async fn calculate_orca_whirlpool_price(
        &self,
        pool_info: &PoolInfo,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        if self.debug_enabled {
            log(
                LogTag::Pool,
                "WHIRLPOOL_PRICE_CALC",
                &format!(
                    "Calculating Orca Whirlpool price for token {} in pool {}",
                    token_mint,
                    pool_info.pool_address
                )
            );
        }

        // Get correct decimals for the target token
        let target_token_decimals = get_cached_decimals(token_mint).unwrap_or(9);
        let sol_decimals = 9;

        // For Orca Whirlpool, token_0 is SOL, token_1 is the target token
        let (sol_reserve, token_reserve, final_sol_decimals, final_token_decimals) = if
            pool_info.token_0_mint == SOL_MINT
        {
            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "WHIRLPOOL_STRUCTURE",
                    &format!(
                        "Orca Whirlpool pool structure: token_0={} (SOL), token_1={} (target), using target token: {}",
                        pool_info.token_0_mint,
                        pool_info.token_1_mint,
                        token_mint
                    )
                );
            }
            (
                pool_info.token_0_reserve, // SOL reserve
                pool_info.token_1_reserve, // Token reserve
                sol_decimals,
                target_token_decimals, // Use correct decimals for target token
            )
        } else if pool_info.token_1_mint == SOL_MINT {
            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "WHIRLPOOL_STRUCTURE",
                    &format!(
                        "Orca Whirlpool pool structure: token_0={} (target), token_1={} (SOL), using target token: {}",
                        pool_info.token_0_mint,
                        pool_info.token_1_mint,
                        token_mint
                    )
                );
            }
            (
                pool_info.token_1_reserve, // SOL reserve
                pool_info.token_0_reserve, // Token reserve
                sol_decimals,
                target_token_decimals, // Use correct decimals for target token
            )
        } else {
            return Err(
                format!(
                    "Orca Whirlpool pool {} does not contain SOL. Token0: {}, Token1: {}",
                    pool_info.pool_address,
                    pool_info.token_0_mint,
                    pool_info.token_1_mint
                )
            );
        };

        // Validate reserves
        if sol_reserve == 0 || token_reserve == 0 {
            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "WHIRLPOOL_ZERO_RESERVES",
                    &format!(
                        "Orca Whirlpool pool {} has zero reserves, cannot calculate price",
                        pool_info.pool_address
                    )
                );
            }
            return Ok(None);
        }

        // For Orca Whirlpool concentrated liquidity, use sqrt_price instead of simple reserve ratios
        let price_sol = if let Some(sqrt_price_value) = pool_info.sqrt_price {
            // Orca Whirlpool sqrt_price calculation
            // The sqrt_price is stored as a Q64.64 fixed point number
            // Price = (sqrt_price / 2^64)^2
            // Then we need to adjust for the token ordering and decimals

            let sqrt_price_scaled = (sqrt_price_value as f64) / (2_f64).powi(64);
            let raw_price = sqrt_price_scaled * sqrt_price_scaled;

            // The price represents token_B/token_A ratio
            // Since token_A is SOL and token_B is our target token, raw_price = target_token/SOL
            // We want SOL/target_token, so we need to invert: 1/raw_price
            let inverted_price = 1.0 / raw_price;

            // Adjust for decimal differences
            // If SOL has 9 decimals and token has 6 decimals:
            // We need to multiply by (10^6 / 10^9) = 0.001
            let decimal_adjustment =
                (10_f64).powi(final_token_decimals as i32) /
                (10_f64).powi(final_sol_decimals as i32);
            let adjusted_price = inverted_price * decimal_adjustment;

            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "WHIRLPOOL_SQRT_PRICE",
                    &format!(
                        "Orca Whirlpool sqrt_price calculation:\n\
                        - sqrt_price: {}\n\
                        - sqrt_price_scaled: {:.16}\n\
                        - raw_price (token_B/token_A): {:.16}\n\
                        - inverted_price (token_A/token_B): {:.16}\n\
                        - decimal_adjustment: {:.16} (token_decimals: {}, sol_decimals: {})\n\
                        - final_price: {:.12} SOL",
                        sqrt_price_value,
                        sqrt_price_scaled,
                        raw_price,
                        inverted_price,
                        decimal_adjustment,
                        final_token_decimals,
                        final_sol_decimals,
                        adjusted_price
                    )
                );
            }

            adjusted_price
        } else {
            // Fallback to reserve ratio calculation (less accurate for concentrated liquidity)
            let sol_adjusted = (sol_reserve as f64) / (10f64).powi(final_sol_decimals as i32);
            let token_adjusted = (token_reserve as f64) / (10f64).powi(final_token_decimals as i32);
            let fallback_price = sol_adjusted / token_adjusted;

            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "WHIRLPOOL_FALLBACK",
                    &format!(
                        "⚠️  Using fallback reserve ratio calculation (less accurate):\n\
                        - SOL Reserve: {} (adjusted: {:.12})\n\
                        - Token Reserve: {} (adjusted: {:.12})\n\
                        - Fallback Price: {:.12} SOL",
                        sol_reserve,
                        sol_adjusted,
                        token_reserve,
                        token_adjusted,
                        fallback_price
                    )
                );
            }

            fallback_price
        };

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "WHIRLPOOL_PRICE",
                &format!(
                    "Orca Whirlpool final price calculation:\n\
                    - SOL Reserve: {} (decimals: {})\n\
                    - Token Reserve: {} (decimals: {})\n\
                    - sqrt_price available: {}\n\
                    - Final Price SOL: {:.12}\n\
                    - Target Token: {}",
                    sol_reserve,
                    final_sol_decimals,
                    token_reserve,
                    final_token_decimals,
                    pool_info.sqrt_price.is_some(),
                    price_sol,
                    token_mint
                )
            );
        }

        Ok(
            Some(PoolPriceInfo {
                pool_address: pool_info.pool_address.clone(),
                pool_program_id: pool_info.pool_program_id.clone(),
                pool_type: pool_info.pool_type.clone(),
                token_mint: token_mint.to_string(),
                price_sol,
                token_reserve,
                sol_reserve,
                token_decimals: final_token_decimals,
                sol_decimals: final_sol_decimals,
            })
        )
    }

    /// Calculate token price for Pump.fun AMM pools
    ///
    /// Pump.fun uses bonding curves but we can still calculate basic price
    /// from the reserves if we have the correct pool structure
    async fn calculate_pump_fun_amm_price(
        &self,
        pool_info: &PoolInfo,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        if self.debug_enabled {
            log(
                LogTag::Pool,
                "PUMP_PRICE_CALC",
                &format!(
                    "Calculating Pump.fun price for token {} in pool {}",
                    token_mint,
                    pool_info.pool_address
                )
            );
        }

        // For PUMP.FUN, the pool structure uses placeholder token mint
        // We need to use the target token mint and get correct decimals
        let target_token_decimals = get_cached_decimals(token_mint).unwrap_or(9);
        let sol_decimals = 9;

        // For PUMP.FUN pools, token_0 is always the target token, token_1 is always SOL
        let (sol_reserve, token_reserve, final_sol_decimals, final_token_decimals) = if
            pool_info.token_1_mint == SOL_MINT
        {
            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "PUMP_STRUCTURE",
                    &format!(
                        "PUMP.FUN pool structure: token_0={} (target), token_1={} (SOL), using target token: {}",
                        pool_info.token_0_mint,
                        pool_info.token_1_mint,
                        token_mint
                    )
                );
            }
            (
                pool_info.token_1_reserve, // SOL reserve
                pool_info.token_0_reserve, // Token reserve
                sol_decimals,
                target_token_decimals, // Use correct decimals for target token
            )
        } else {
            return Err(
                format!(
                    "PUMP.FUN pool {} does not contain SOL as token_1. Token0: {}, Token1: {}",
                    pool_info.pool_address,
                    pool_info.token_0_mint,
                    pool_info.token_1_mint
                )
            );
        };

        // Validate reserves - for pump.fun, we might have placeholder values
        // If reserves are the placeholders we set (1000000 and 1000), calculate from API native price
        if
            (sol_reserve == 1000 && token_reserve == 1_000_000) ||
            sol_reserve == 0 ||
            token_reserve == 0
        {
            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "PUMP_PLACEHOLDER",
                    &format!(
                        "Pump.fun pool {} has placeholder/zero reserves, attempting alternative calculation",
                        pool_info.pool_address
                    )
                );
            }

            // Try to get the native price from the pool API data we have
            // This is a workaround until we can properly decode pump.fun pool structure
            return Ok(None); // Let price_service.rs handle this
        }

        // Calculate price in SOL: price = (SOL reserves / 10^SOL_decimals) / (token reserves / 10^token_decimals)
        let sol_adjusted = (sol_reserve as f64) / (10f64).powi(final_sol_decimals as i32);
        let token_adjusted = (token_reserve as f64) / (10f64).powi(final_token_decimals as i32);
        let price_sol = sol_adjusted / token_adjusted;

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "PUMP_PRICE",
                &format!(
                    "Pump.fun price calculation:\n\
                    - SOL Reserve: {} (decimals: {}, adjusted: {:.12})\n\
                    - Token Reserve: {} (decimals: {}, adjusted: {:.12})\n\
                    - Price SOL: {:.12}\n\
                    - Target Token: {}",
                    sol_reserve,
                    final_sol_decimals,
                    sol_adjusted,
                    token_reserve,
                    final_token_decimals,
                    token_adjusted,
                    price_sol,
                    token_mint
                )
            );
        }

        Ok(
            Some(PoolPriceInfo {
                pool_address: pool_info.pool_address.clone(),
                pool_program_id: pool_info.pool_program_id.clone(),
                pool_type: pool_info.pool_type.clone(),
                token_mint: token_mint.to_string(),
                price_sol,
                token_reserve,
                sol_reserve,
                token_decimals: final_token_decimals,
                sol_decimals: final_sol_decimals,
            })
        )
    }

    /// Decode token account amount from account data
    fn decode_token_account_amount(data: &[u8]) -> Result<u64, String> {
        if data.len() < 72 {
            return Err("Invalid token account data length".to_string());
        }

        // Token account amount is at offset 64 (8 bytes)
        let amount_bytes = &data[64..72];
        let amount = u64::from_le_bytes(
            amount_bytes.try_into().map_err(|_| "Failed to parse token account amount".to_string())?
        );

        Ok(amount)
    }

    // Helper functions for reading pool data
    fn read_pubkey_at_offset(data: &[u8], offset: &mut usize) -> Result<String, String> {
        if *offset + 32 > data.len() {
            return Err("Insufficient data for pubkey".to_string());
        }

        let pubkey_bytes = &data[*offset..*offset + 32];
        *offset += 32;

        let pubkey = Pubkey::new_from_array(
            pubkey_bytes.try_into().map_err(|_| "Failed to parse pubkey".to_string())?
        );

        Ok(pubkey.to_string())
    }

    fn read_u8_at_offset(data: &[u8], offset: &mut usize) -> Result<u8, String> {
        if *offset >= data.len() {
            return Err("Insufficient data for u8".to_string());
        }

        let value = data[*offset];
        *offset += 1;
        Ok(value)
    }

    fn read_u64_at_offset(data: &[u8], offset: &mut usize) -> Result<u64, String> {
        if *offset + 8 > data.len() {
            return Err("Insufficient data for u64".to_string());
        }

        let bytes = &data[*offset..*offset + 8];
        *offset += 8;

        let value = u64::from_le_bytes(
            bytes.try_into().map_err(|_| "Failed to parse u64".to_string())?
        );

        Ok(value)
    }

    fn read_u128_at_offset(data: &[u8], offset: &mut usize) -> Result<u128, String> {
        if *offset + 16 > data.len() {
            return Err("Insufficient data for u128".to_string());
        }

        let bytes = &data[*offset..*offset + 16];
        *offset += 16;

        let value = u128::from_le_bytes(
            bytes.try_into().map_err(|_| "Failed to parse u128".to_string())?
        );

        Ok(value)
    }

    /// Decode Orca Whirlpool pool data structure
    ///
    /// Based on the provided schema, Orca Whirlpool pools have a specific structure
    /// with sqrtPrice, liquidity, and token vaults that we need to decode
    async fn decode_orca_whirlpool_pool(
        &self,
        pool_address: &str,
        account: &Account
    ) -> Result<PoolInfo, String> {
        if account.data.len() < 300 {
            return Err("Invalid Orca Whirlpool pool account data length".to_string());
        }

        let data = &account.data;
        let mut offset = 8; // Skip discriminator

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "WHIRLPOOL_DEBUG",
                &format!(
                    "Orca Whirlpool pool {} - data length: {} bytes, decoding structure...",
                    pool_address,
                    data.len()
                )
            );
        }

        // Decode Orca Whirlpool pool structure based on provided schema:
        // whirlpoolsConfig (publicKey), whirlpoolBump ([u8;1]), tickSpacing (u16),
        // feeTierIndexSeed ([u8;2]), feeRate (u16), protocolFeeRate (u16),
        // liquidity (u128), sqrtPrice (u128), tickCurrentIndex (i32),
        // protocolFeeOwedA (u64), protocolFeeOwedB (u64),
        // tokenMintA (publicKey), tokenVaultA (publicKey), feeGrowthGlobalA (u128),
        // tokenMintB (publicKey), tokenVaultB (publicKey), feeGrowthGlobalB (u128)

        let _whirlpools_config = Self::read_pubkey_at_offset(data, &mut offset)?; // whirlpoolsConfig

        let _whirlpool_bump = data[offset]; // whirlpoolBump [u8;1]
        offset += 1;

        let _tick_spacing = u16::from_le_bytes(
            data[offset..offset + 2].try_into().unwrap_or([0; 2])
        ); // tickSpacing
        offset += 2;

        let _fee_tier_index_seed = [data[offset], data[offset + 1]]; // feeTierIndexSeed [u8;2]
        offset += 2;

        let _fee_rate = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap_or([0; 2])); // feeRate
        offset += 2;

        let _protocol_fee_rate = u16::from_le_bytes(
            data[offset..offset + 2].try_into().unwrap_or([0; 2])
        ); // protocolFeeRate
        offset += 2;

        let liquidity = u128::from_le_bytes(
            data[offset..offset + 16].try_into().unwrap_or([0; 16])
        ); // liquidity
        offset += 16;

        let sqrt_price = u128::from_le_bytes(
            data[offset..offset + 16].try_into().unwrap_or([0; 16])
        ); // sqrtPrice
        offset += 16;

        let _tick_current_index = i32::from_le_bytes(
            data[offset..offset + 4].try_into().unwrap_or([0; 4])
        ); // tickCurrentIndex
        offset += 4;

        let _protocol_fee_owed_a = u64::from_le_bytes(
            data[offset..offset + 8].try_into().unwrap_or([0; 8])
        ); // protocolFeeOwedA
        offset += 8;

        let _protocol_fee_owed_b = u64::from_le_bytes(
            data[offset..offset + 8].try_into().unwrap_or([0; 8])
        ); // protocolFeeOwedB
        offset += 8;

        let token_mint_a = Self::read_pubkey_at_offset(data, &mut offset)?; // tokenMintA (SOL)
        let token_vault_a = Self::read_pubkey_at_offset(data, &mut offset)?; // tokenVaultA (SOL vault)

        let _fee_growth_global_a = u128::from_le_bytes(
            data[offset..offset + 16].try_into().unwrap_or([0; 16])
        ); // feeGrowthGlobalA
        offset += 16;

        let token_mint_b = Self::read_pubkey_at_offset(data, &mut offset)?; // tokenMintB (our token)
        let token_vault_b = Self::read_pubkey_at_offset(data, &mut offset)?; // tokenVaultB (token vault)

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "WHIRLPOOL_EXTRACT",
                &format!(
                    "Extracted Orca Whirlpool pool structure:\n\
                    - Token Mint A (SOL): {}\n\
                    - Token Mint B (target): {}\n\
                    - Token Vault A (SOL): {}\n\
                    - Token Vault B (target): {}\n\
                    - Liquidity: {}\n\
                    - Sqrt Price: {}",
                    token_mint_a,
                    token_mint_b,
                    token_vault_a,
                    token_vault_b,
                    liquidity,
                    sqrt_price
                )
            );
        }

        // For Orca Whirlpool, get reserves from the vault accounts
        let token_vault_a_str = token_vault_a.to_string();
        let token_vault_b_str = token_vault_b.to_string();

        let (vault_a_balance, vault_b_balance) = match
            self.get_vault_balances(&token_vault_a_str, &token_vault_b_str).await
        {
            Ok((va, vb)) => {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "WHIRLPOOL_VAULT_SUCCESS",
                        &format!(
                            "Successfully fetched Orca Whirlpool vault balances:\n\
                            - Vault A {} (SOL) balance: {}\n\
                            - Vault B {} (token) balance: {}",
                            token_vault_a_str,
                            va,
                            token_vault_b_str,
                            vb
                        )
                    );
                }
                (va, vb)
            }
            Err(e) => {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "WHIRLPOOL_VAULT_ERROR",
                        &format!("Vault balance fetch failed: {}", e)
                    );
                }
                return Err(format!("Failed to get vault balances for Orca Whirlpool pool: {}", e));
            }
        };

        // Use default decimals for now - will be corrected in price calculation with actual target token
        let token_decimals = 9; // Default fallback - should use get_cached_decimals properly
        let sol_decimals = 9; // SOL always has 9 decimals

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "WHIRLPOOL_DECODE",
                &format!(
                    "Orca Whirlpool pool {} decoded:\n\
                    - Token A (SOL): {} ({} decimals, {} reserve)\n\
                    - Token B (target): {} ({} decimals, {} reserve)\n\
                    - Token Vault A: {}\n\
                    - Token Vault B: {}\n\
                    - Liquidity: {}\n\
                    - Sqrt Price: {}",
                    pool_address,
                    token_mint_a,
                    sol_decimals,
                    vault_a_balance,
                    token_mint_b,
                    token_decimals,
                    vault_b_balance,
                    token_vault_a,
                    token_vault_b,
                    liquidity,
                    sqrt_price
                )
            );
        }

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_program_id: ORCA_WHIRLPOOL_PROGRAM_ID.to_string(),
            pool_type: get_pool_program_display_name(ORCA_WHIRLPOOL_PROGRAM_ID),
            token_0_mint: token_mint_a.to_string(), // SOL
            token_1_mint: token_mint_b.to_string(), // Target token
            token_0_vault: Some(token_vault_a.to_string()),
            token_1_vault: Some(token_vault_b.to_string()),
            token_0_reserve: vault_a_balance, // SOL reserve
            token_1_reserve: vault_b_balance, // Token reserve
            token_0_decimals: sol_decimals,
            token_1_decimals: token_decimals,
            lp_mint: None, // Whirlpool uses concentrated liquidity
            lp_supply: Some(liquidity as u64), // Use liquidity value
            creator: None,
            status: None,
            liquidity_usd: None,
            sqrt_price: Some(sqrt_price), // Store sqrt_price for concentrated liquidity calculation
        })
    }

    /// Decode Pump.fun AMM pool data structure
    ///
    /// Based on git history analysis, pump.fun pools have a specific structure
    /// that we need to decode to get the actual token vaults and reserves
    async fn decode_pump_fun_amm_pool(
        &self,
        pool_address: &str,
        account: &Account
    ) -> Result<PoolInfo, String> {
        if account.data.len() < 200 {
            return Err("Invalid Pump.fun AMM pool account data length".to_string());
        }

        let data = &account.data;
        let mut offset = 8; // Skip discriminator

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "PUMP_DEBUG",
                &format!(
                    "Pump.fun pool {} - data length: {} bytes, decoding structure...",
                    pool_address,
                    data.len()
                )
            );
        }

        // Decode PUMP.FUN AMM pool structure based on provided schema:
        // pool_bump (u8), index (u16), creator (pubkey), base_mint (pubkey), quote_mint (pubkey),
        // lp_mint (pubkey), pool_base_token_account (pubkey), pool_quote_token_account (pubkey),
        // lp_supply (u64), coin_creator (pubkey)

        let _pool_bump = data[offset]; // u8
        offset += 1;

        let _index = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap_or([0; 2])); // u16
        offset += 2;

        let _creator = Self::read_pubkey_at_offset(data, &mut offset)?; // creator pubkey
        let base_mint = Self::read_pubkey_at_offset(data, &mut offset)?; // base_mint (our token)
        let quote_mint = Self::read_pubkey_at_offset(data, &mut offset)?; // quote_mint (SOL)
        let _lp_mint = Self::read_pubkey_at_offset(data, &mut offset)?; // lp_mint
        let pool_base_token_account = Self::read_pubkey_at_offset(data, &mut offset)?; // base token vault
        let pool_quote_token_account = Self::read_pubkey_at_offset(data, &mut offset)?; // quote token vault (SOL)

        let lp_supply = if data.len() >= offset + 8 {
            u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap_or([0; 8]))
        } else {
            0
        };
        offset += 8;

        let _coin_creator = Self::read_pubkey_at_offset(data, &mut offset)?; // coin_creator

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "PUMP_EXTRACT",
                &format!(
                    "Extracted PUMP.FUN pool structure:\n\
                    - Base mint (token): {}\n\
                    - Quote mint (SOL): {}\n\
                    - Base token vault: {}\n\
                    - Quote token vault: {}\n\
                    - LP supply: {}",
                    base_mint,
                    quote_mint,
                    pool_base_token_account,
                    pool_quote_token_account,
                    lp_supply
                )
            );
        }

        // SOL is always the quote token in pump.fun
        let sol_mint = SOL_MINT.to_string();

        // Use default decimals for now - will be corrected in price calculation with actual target token
        let token_decimals = 9; // Default fallback - should use get_cached_decimals properly
        let sol_decimals = 9; // SOL always has 9 decimals

        // For PUMP.FUN pools, get reserves from the vault accounts
        let token_vault_str = pool_base_token_account.to_string();
        let sol_vault_str = pool_quote_token_account.to_string();

        let (token_reserve, sol_reserve) = match
            self.get_vault_balances(&token_vault_str, &sol_vault_str).await
        {
            Ok((tr, sr)) => {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "PUMP_VAULT_SUCCESS",
                        &format!(
                            "Successfully fetched PUMP.FUN vault balances:\n\
                            - Token vault {} balance: {}\n\
                            - SOL vault {} balance: {}",
                            token_vault_str,
                            tr,
                            sol_vault_str,
                            sr
                        )
                    );
                }
                (tr, sr)
            }
            Err(e) => {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "PUMP_VAULT_ERROR",
                        &format!("Vault balance fetch failed: {}", e)
                    );
                }
                return Err(format!("Failed to get vault balances for PUMP.FUN pool: {}", e));
            }
        };

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "PUMP_DECODE",
                &format!(
                    "Pump.fun pool {} decoded:\n\
                    - Base Token: {} ({} decimals, {} reserve)\n\
                    - Quote Token (SOL): {} ({} decimals, {} reserve)\n\
                    - Base Token Vault: {}\n\
                    - Quote Token Vault: {}\n\
                    - LP Supply: {}",
                    pool_address,
                    base_mint, // This is the actual token mint from the pool structure
                    token_decimals,
                    token_reserve,
                    quote_mint,
                    sol_decimals,
                    sol_reserve,
                    pool_base_token_account,
                    pool_quote_token_account,
                    lp_supply
                )
            );
        }

        Ok(PoolInfo {
            pool_address: pool_address.to_string(),
            pool_program_id: PUMP_FUN_AMM_PROGRAM_ID.to_string(),
            pool_type: get_pool_program_display_name(PUMP_FUN_AMM_PROGRAM_ID),
            token_0_mint: base_mint.to_string(), // Use the actual base_mint from pool structure
            token_1_mint: quote_mint.to_string(), // Use the actual quote_mint from pool structure
            token_0_vault: Some(pool_base_token_account.to_string()),
            token_1_vault: Some(pool_quote_token_account.to_string()),
            token_0_reserve: token_reserve, // Base token reserve
            token_1_reserve: sol_reserve, // Quote token (SOL) reserve
            token_0_decimals: token_decimals,
            token_1_decimals: sol_decimals,
            lp_mint: None, // Pump.fun doesn't use standard LP tokens
            lp_supply: Some(lp_supply),
            creator: None,
            status: Some(1), // Active
            liquidity_usd: None, // Will be calculated elsewhere
            sqrt_price: None, // Not applicable to bonding curve AMM
        })
    }
}

/// Helper function to extract a pubkey from raw data at a specific offset
fn extract_pubkey_at_offset(data: &[u8], offset: usize) -> Result<Pubkey, String> {
    if data.len() < offset + 32 {
        return Err(format!("Insufficient data length for pubkey at offset {}", offset));
    }

    let pubkey_bytes: [u8; 32] = data[offset..offset + 32]
        .try_into()
        .map_err(|_| "Failed to extract 32 bytes for pubkey".to_string())?;

    Ok(Pubkey::new_from_array(pubkey_bytes))
}

// =============================================================================
// UNIFIED PRICE SERVICE FUNCTIONS (REPLACING price.rs)
// =============================================================================

/// Price cache entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceCacheEntry {
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub last_updated: DateTime<Utc>,
    pub source: String, // "api", "pool"
}

impl PriceCacheEntry {
    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.last_updated;
        age > chrono::Duration::seconds(5) // 5 second TTL
    }

    pub fn from_pool_result(pool_result: &PoolPriceResult) -> Self {
        Self {
            price_sol: pool_result.price_sol,
            price_usd: pool_result.price_usd,
            liquidity_usd: Some(pool_result.liquidity_usd),
            last_updated: pool_result.calculated_at,
            source: "pool".to_string(),
        }
    }
}

/// Initialize the unified price service (now handled by pool service)
pub async fn initialize_price_service() -> Result<(), Box<dyn std::error::Error>> {
    // Pool service is already initialized via init_pool_service
    let pool_service = get_pool_service();
    log(LogTag::Pool, "INIT", "✅ Unified price service initialized (using pool service)");
    Ok(())
}

// =============================================================================
// UNIVERSAL PRICE FUNCTION
// =============================================================================

/// Universal price result structure that covers all pricing needs
#[derive(Debug, Clone)]
pub struct PriceResult {
    pub token_address: String,
    pub price_sol: Option<f64>, // Primary SOL price (pool or API)
    pub price_usd: Option<f64>, // USD price (if available)
    pub api_price_sol: Option<f64>, // API-sourced SOL price
    pub pool_price_sol: Option<f64>, // Pool-calculated SOL price
    pub pool_address: Option<String>, // Pool address (if pool source)
    pub dex_id: Option<String>, // DEX identifier (if pool source)
    pub pool_type: Option<String>, // Pool type (if pool source)
    pub volume_24h: Option<f64>, // 24h volume (if pool source)
    pub source: String, // "pool", "api", "both", or "cache"
    pub calculated_at: DateTime<Utc>, // When calculated
    pub is_cached: bool, // Whether result came from cache
    pub error: Option<String>, // Error message when price is None (explains why)
    pub reserve_sol: Option<f64>, // SOL reserves in pool (use this for liquidity checking)
    pub reserve_token: Option<f64>, // Token reserves in pool
}

impl PriceResult {
    /// Get simple SOL price
    pub fn sol_price(&self) -> Option<f64> {
        self.price_sol
    }

    /// Check if we have pool data
    pub fn has_pool_data(&self) -> bool {
        self.pool_address.is_some()
    }

    /// Check if we have API data
    pub fn has_api_data(&self) -> bool {
        self.api_price_sol.is_some()
    }
}

/// Price request options for configuring the universal get_price function
#[derive(Debug, Clone)]
pub struct PriceOptions {
    /// Include pool price calculation
    pub include_pool: bool,
    /// Include API price lookup
    pub include_api: bool,
    /// Allow cached results (respects cache TTL)
    pub allow_cache: bool,
    /// Force fresh calculation (bypass cache)
    pub force_refresh: bool,
    /// Timeout for the entire operation (seconds)
    pub timeout_secs: Option<u64>,
    /// If no fresh cache, enqueue a one-shot warm-up (batch pool calc) without adding to watchlist
    pub warm_on_miss: bool,
}

impl Default for PriceOptions {
    fn default() -> Self {
        Self {
            include_pool: true,
            include_api: true,
            allow_cache: true,
            force_refresh: false,
            timeout_secs: Some(10),
            warm_on_miss: false,
        }
    }
}

impl PriceOptions {
    /// Create options for pool only
    pub fn pool_only() -> Self {
        Self {
            include_pool: true,
            include_api: true,
            allow_cache: true,
            force_refresh: false,
            timeout_secs: Some(10),
            warm_on_miss: false,
        }
    }

    /// Create options for API only
    pub fn api_only() -> Self {
        Self {
            include_pool: true,
            include_api: true,
            allow_cache: true,
            force_refresh: false,
            timeout_secs: Some(10),
            warm_on_miss: false,
        }
    }
}

/// Universal price function - the ONLY price function you should use
///
/// This replaces all get_token_price_* and get_pool_price variants.
/// Works in both sync and async contexts via the sync parameter.
///
/// Parameters:
/// - token_address: The token mint address
/// - options: PriceOptions to configure behavior
/// - sync: If true, uses blocking execution for sync contexts
///
/// Returns PriceResult with comprehensive price information
pub async fn get_price(
    token_address: &str,
    options: Option<PriceOptions>,
    sync: bool
) -> Option<PriceResult> {
    let options = options.unwrap_or_default();

    // Handle sync execution by wrapping in block_in_place
    if sync {
        return tokio::task::block_in_place(|| {
            tokio::runtime::Handle
                ::current()
                .block_on(async { get_price_async(token_address, options).await })
        });
    }

    get_price_async(token_address, options).await
}

/// Internal async implementation of universal price function
async fn get_price_async(token_address: &str, options: PriceOptions) -> Option<PriceResult> {
    let start_time = Instant::now();
    let calculated_at = Utc::now();

    // Apply timeout if specified
    let result = if let Some(timeout_secs) = options.timeout_secs {
        match
            tokio::time::timeout(
                Duration::from_secs(timeout_secs),
                get_price_internal(token_address, &options, calculated_at)
            ).await
        {
            Ok(result) => result,
            Err(_) => {
                // Timeout occurred - return error result instead of None
                let error_msg = format!("Price calculation timed out after {}s", timeout_secs);
                Some(PriceResult {
                    token_address: token_address.to_string(),
                    price_sol: None,
                    price_usd: None,
                    api_price_sol: None,
                    pool_price_sol: None,
                    pool_address: None,
                    dex_id: None,
                    pool_type: None,
                    volume_24h: None,
                    source: "timeout".to_string(),
                    calculated_at,
                    is_cached: false,
                    error: Some(error_msg),
                    reserve_sol: None,
                    reserve_token: None,
                })
            }
        }
    } else {
        get_price_internal(token_address, &options, calculated_at).await
    };

    if is_debug_pool_prices_enabled() {
        let duration = start_time.elapsed();
        let result_info = match &result {
            Some(r) => {
                if let Some(ref err) = r.error {
                    format!("error: {}", err)
                } else {
                    format!(
                        "source={}, pool={:?}, api={:?}",
                        r.source,
                        r.pool_price_sol,
                        r.api_price_sol
                    )
                }
            }
            None => "failed".to_string(),
        };
        log(
            LogTag::Pool,
            "PRICE_RESULT",
            &format!("get_price({}) -> {} in {:?}", &token_address[..8], result_info, duration)
        );
    }

    result
}

/// Core price calculation logic - READ-ONLY from service cache
async fn get_price_internal(
    token_address: &str,
    options: &PriceOptions,
    calculated_at: DateTime<Utc>
) -> Option<PriceResult> {
    let service = get_pool_service();

    // Track request count and last accessed time for watchlist tokens
    {
        let watchlist = service.watchlist_tokens.read().await;
        if watchlist.contains(token_address) {
            drop(watchlist); // Release read lock before acquiring write locks

            // Update request count
            let mut request_counts = service.watchlist_request_counts.write().await;
            let current_count = *request_counts.get(token_address).unwrap_or(&0);
            request_counts.insert(token_address.to_string(), current_count + 1);
            drop(request_counts);

            // Update last accessed time for time-based expiry
            let mut last_accessed = service.watchlist_last_accessed.write().await;
            last_accessed.insert(token_address.to_string(), calculated_at);
        }
    }

    // Check if we have a cached price from the background service
    let cached_result = {
        let price_cache = service.price_cache.read().await;
        price_cache.get(token_address).cloned()
    };

    if let Some(pool_result) = cached_result {
        // Check if cached result is still fresh
        let age = calculated_at - pool_result.calculated_at;
        if age.num_seconds() <= PRICE_CACHE_TTL_SECONDS {
            // Convert PoolPriceResult to PriceResult
            let result = PriceResult {
                token_address: token_address.to_string(),
                price_sol: pool_result.price_sol,
                price_usd: pool_result.price_usd,
                api_price_sol: pool_result.api_price_sol,
                pool_price_sol: pool_result.price_sol, // Same as price_sol for cached results
                pool_address: Some(pool_result.pool_address),
                dex_id: Some(pool_result.dex_id),
                pool_type: pool_result.pool_type,
                volume_24h: Some(pool_result.volume_24h),
                source: pool_result.source,
                calculated_at,
                is_cached: true,
                error: pool_result.error.clone(), // Pass through error from cached result
                reserve_sol: pool_result.sol_reserve,
                reserve_token: pool_result.token_reserve,
            };

            return Some(result);
        } else {
            // Cached result is too old
            let error_msg = format!(
                "Cached price expired (age: {}s, max: {}s)",
                age.num_seconds(),
                PRICE_CACHE_TTL_SECONDS
            );

            // Optionally enqueue ad-hoc warm-up for expired cache
            if options.warm_on_miss {
                let mut ad_hoc = service.ad_hoc_refresh_tokens.write().await;
                ad_hoc.insert(token_address.to_string());
                drop(ad_hoc);
                if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "ADHOC_ENQUEUE_EXPIRED",
                        &format!(
                            "Enqueued {} for warm-up due to expired cache (age {}s)",
                            &token_address[..8],
                            age.num_seconds()
                        )
                    );
                }
            }

            let result = PriceResult {
                token_address: token_address.to_string(),
                price_sol: None,
                price_usd: None,
                api_price_sol: None,
                pool_price_sol: None,
                pool_address: None,
                dex_id: None,
                pool_type: None,
                volume_24h: None,
                source: "cache_expired".to_string(),
                calculated_at,
                is_cached: false,
                error: Some(error_msg),
                reserve_sol: None,
                reserve_token: None,
            };

            return Some(result);
        }
    }

    if is_debug_pool_prices_enabled() {
        log(
            LogTag::Pool,
            "CACHE_MISS",
            &format!(
                "No fresh cached price for {} (not in active monitoring or cache expired)",
                &token_address[..8]
            )
        );
    }

    // No cached result available - optionally enqueue one-shot warm-up
    if options.warm_on_miss {
        let mut ad_hoc = service.ad_hoc_refresh_tokens.write().await;
        ad_hoc.insert(token_address.to_string());
        drop(ad_hoc);
        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "ADHOC_ENQUEUE",
                &format!("Enqueued {} for one-shot warm-up", &token_address[..8])
            );
        }
    }

    // Return error result instead of None
    let error_msg =
        "No cached price available - not monitored; trader should schedule if needed".to_string();
    let result = PriceResult {
        token_address: token_address.to_string(),
        price_sol: None,
        price_usd: None,
        api_price_sol: None,
        pool_price_sol: None,
        pool_address: None,
        dex_id: None,
        pool_type: None,
        volume_24h: None,
        source: "no_cache".to_string(),
        calculated_at,
        is_cached: false,
        error: Some(error_msg),
        reserve_sol: None,
        reserve_token: None,
    };

    Some(result)
}

// =============================================================================
// GLOBAL WATCHLIST MANAGEMENT FUNCTIONS
// =============================================================================

/// Add token to priority monitoring (positions)
pub async fn add_priority_token(token_address: &str) {
    let service = get_pool_service();
    service.add_priority_token(token_address).await;
}

/// Remove token from priority monitoring
pub async fn remove_priority_token(token_address: &str) {
    let service = get_pool_service();
    service.remove_priority_token(token_address).await;
}

/// Add token to watchlist for random monitoring
pub async fn add_watchlist_token(token_address: &str) {
    let service = get_pool_service();
    service.add_watchlist_token(token_address).await;
}

/// Remove token from watchlist
pub async fn remove_watchlist_token(token_address: &str) {
    let service = get_pool_service();
    service.remove_watchlist_token(token_address).await;
}

/// Add multiple tokens to watchlist (batch operation)
pub async fn add_watchlist_tokens(token_addresses: &[String]) {
    let service = get_pool_service();
    service.add_watchlist_tokens(token_addresses).await;
}

/// Get current priority tokens
pub async fn get_priority_tokens() -> Vec<String> {
    let service = get_pool_service();
    service.get_priority_tokens().await
}

/// Get current watchlist tokens
pub async fn get_watchlist_tokens() -> Vec<String> {
    let service = get_pool_service();
    service.get_watchlist_tokens().await
}

/// Get watchlist status information
pub async fn get_watchlist_status() -> (usize, usize, Option<DateTime<Utc>>) {
    let service = get_pool_service();
    service.get_watchlist_status().await
}

/// Clear all priority tokens (for testing/reset)
pub async fn clear_priority_tokens() {
    let service = get_pool_service();
    service.clear_priority_tokens().await;
}

/// Clear all watchlist tokens (for testing/reset)
pub async fn clear_watchlist_tokens() {
    let service = get_pool_service();
    service.clear_watchlist_tokens().await;
}

/// Emit a one-shot consolidated pool state summary log (not gated by debug)
pub async fn log_pool_state_summary() {
    let service = get_pool_service();
    service.log_state_summary().await;
}

/// Request a one-shot warm-up for a token without adding it to watchlist
pub async fn request_price_warmup(token_address: &str) {
    let service = get_pool_service();
    let mut ad_hoc = service.ad_hoc_refresh_tokens.write().await;
    ad_hoc.insert(token_address.to_string());
}

/// Request warm-up for multiple tokens
pub async fn request_price_warmup_batch(token_addresses: &[String]) {
    let service = get_pool_service();
    let mut ad_hoc = service.ad_hoc_refresh_tokens.write().await;
    for t in token_addresses {
        ad_hoc.insert(t.clone());
    }
}

// =============================================================================
// GLOBAL INVALID POOL BLACKLIST FUNCTIONS
// =============================================================================

/// Add a token to the invalid pool blacklist
pub async fn add_to_invalid_pool_blacklist(
    token_address: &str,
    symbol: Option<&str>,
    error_reason: &str
) {
    let service = get_pool_service();
    service.add_to_invalid_pool_blacklist(token_address, symbol, error_reason).await;
}

/// Check if a token is in the invalid pool blacklist
pub async fn is_in_invalid_pool_blacklist(token_address: &str) -> bool {
    let service = get_pool_service();
    service.is_in_invalid_pool_blacklist(token_address).await
}

/// Get invalid pool blacklist stats for summary
pub async fn get_invalid_pool_blacklist_stats() -> (usize, Vec<String>) {
    let service = get_pool_service();
    service.get_invalid_pool_blacklist_stats().await
}

/// Clear the invalid pool blacklist (for testing/reset)
pub async fn clear_invalid_pool_blacklist() {
    let service = get_pool_service();
    service.clear_invalid_pool_blacklist().await
}

// =============================================================================
// DEBUG FUNCTIONS FOR POOL ANALYSIS
// =============================================================================

/// Debug function: Get pools with specific program ID from database
/// This searches the database and validates program IDs on-chain
pub async fn debug_find_pools_by_program_id(
    program_id: &str,
    max_pools: usize
) -> Result<Vec<String>, String> {
    use crate::rpc::get_rpc_client;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    // Parse target program ID
    let target_pubkey = Pubkey::from_str(program_id).map_err(|e|
        format!("Invalid program ID: {}", e)
    )?;

    // Get pool addresses from database
    let pool_addresses = crate::tokens::pool_db::get_all_pool_addresses(max_pools)?;

    log(
        LogTag::Pool,
        "DEBUG_PROGRAM_SEARCH",
        &format!("🔍 Searching {} pools for program ID: {}", pool_addresses.len(), program_id)
    );

    let mut matching_pools = Vec::new();
    let rpc_client = get_rpc_client();

    for (i, pool_addr) in pool_addresses.iter().enumerate() {
        if i % 25 == 0 && i > 0 {
            log(
                LogTag::Pool,
                "DEBUG_PROGRESS",
                &format!(
                    "🔄 Checked {}/{} pools, found {} matches",
                    i,
                    pool_addresses.len(),
                    matching_pools.len()
                )
            );
        }

        let pool_pubkey = match Pubkey::from_str(pool_addr) {
            Ok(pubkey) => pubkey,
            Err(_) => {
                continue;
            }
        };

        match rpc_client.get_account(&pool_pubkey).await {
            Ok(account) => {
                if account.owner == target_pubkey {
                    log(
                        LogTag::Pool,
                        "DEBUG_FOUND",
                        &format!("✅ Found pool with program ID {}: {}", program_id, pool_addr)
                    );
                    matching_pools.push(pool_addr.clone());
                }
            }
            Err(_) => {
                // Pool might not exist - continue searching
            }
        }
    }

    log(
        LogTag::Pool,
        "DEBUG_COMPLETE",
        &format!(
            "🎯 Found {} pools with program ID {} out of {} checked",
            matching_pools.len(),
            program_id,
            pool_addresses.len()
        )
    );

    Ok(matching_pools)
}

/// Test function to compare pool discovery between DexScreener and GeckoTerminal
/// This function is useful for debugging and validating the dual API integration
pub async fn test_dual_api_pool_discovery(token_addresses: &[String]) -> Result<(), String> {
    if token_addresses.is_empty() {
        return Err("No token addresses provided".to_string());
    }

    log(
        LogTag::Pool,
        "DUAL_API_TEST_START",
        &format!("🧪 Testing dual API pool discovery for {} tokens", token_addresses.len())
    );

    for token_address in token_addresses.iter().take(5) { // Limit to 5 tokens for testing
        log(
            LogTag::Pool,
            "DUAL_API_TEST_TOKEN",
            &format!("🔍 Testing token: {}", token_address)
        );

        // Test DexScreener
        let dexscreener_result = get_token_pairs_from_api(token_address).await;
        let dexscreener_count = match &dexscreener_result {
            Ok(pairs) => pairs.len(),
            Err(_) => 0,
        };

        // Test GeckoTerminal
        let geckoterminal_result = crate::tokens::geckoterminal::get_token_pools_from_geckoterminal(token_address).await;
        let geckoterminal_count = match &geckoterminal_result {
            Ok(pools) => pools.len(),
            Err(_) => 0,
        };

        log(
            LogTag::Pool,
            "DUAL_API_TEST_RESULT",
            &format!(
                "📊 {}: DexScreener {} pools, GeckoTerminal {} pools",
                &token_address[..8],
                dexscreener_count,
                geckoterminal_count
            )
        );

        // Log detailed results if pools found
        if dexscreener_count > 0 {
            if let Ok(pairs) = &dexscreener_result {
                for (i, pair) in pairs.iter().take(3).enumerate() {
                    log(
                        LogTag::Pool,
                        "DUAL_API_TEST_DX_POOL",
                        &format!(
                            "  🔸 DX Pool {}: {} ({}, ${:.2})",
                            i + 1,
                            pair.pair_address,
                            pair.dex_id,
                            pair.liquidity.as_ref().map(|l| l.usd).unwrap_or(0.0)
                        )
                    );
                }
            }
        }

        if geckoterminal_count > 0 {
            if let Ok(pools) = &geckoterminal_result {
                for (i, pool) in pools.iter().take(3).enumerate() {
                    log(
                        LogTag::Pool,
                        "DUAL_API_TEST_GT_POOL",
                        &format!(
                            "  🦎 GT Pool {}: {} ({}, ${:.2})",
                            i + 1,
                            pool.pool_address,
                            pool.dex_id,
                            pool.liquidity_usd
                        )
                    );
                }
            }
        }

        if dexscreener_count == 0 && geckoterminal_count == 0 {
            log(
                LogTag::Pool,
                "DUAL_API_TEST_NONE",
                &format!("  ❌ No pools found on either platform for {}", &token_address[..8])
            );
        }
    }

    log(
        LogTag::Pool,
        "DUAL_API_TEST_COMPLETE",
        "🧪 Dual API test completed"
    );

    Ok(())
}

/// Test function to compare pool discovery between DexScreener, GeckoTerminal, and Raydium
/// This function is useful for debugging and validating the triple API integration
pub async fn test_triple_api_pool_discovery(token_addresses: &[String]) -> Result<(), String> {
    if token_addresses.is_empty() {
        return Err("No token addresses provided".to_string());
    }

    log(
        LogTag::Pool,
        "TRIPLE_API_TEST_START",
        &format!("🚀 Testing triple API pool discovery for {} tokens", token_addresses.len())
    );

    for token_address in token_addresses.iter().take(5) { // Limit to 5 tokens for testing
        log(
            LogTag::Pool,
            "TRIPLE_API_TEST_TOKEN",
            &format!("🔍 Testing token: {}", token_address)
        );

        // Test DexScreener
        let dexscreener_result = get_token_pairs_from_api(token_address).await;
        let dexscreener_count = match &dexscreener_result {
            Ok(pairs) => pairs.len(),
            Err(_) => 0,
        };

        // Test GeckoTerminal
        let geckoterminal_result = crate::tokens::geckoterminal::get_token_pools_from_geckoterminal(token_address).await;
        let geckoterminal_count = match &geckoterminal_result {
            Ok(pools) => pools.len(),
            Err(_) => 0,
        };

        // Test Raydium
        let raydium_result = crate::tokens::raydium::get_token_pools_from_raydium(token_address).await;
        let raydium_count = match &raydium_result {
            Ok(pools) => pools.len(),
            Err(_) => 0,
        };

        log(
            LogTag::Pool,
            "TRIPLE_API_TEST_RESULT",
            &format!(
                "📊 {}: DexScreener {} pools, GeckoTerminal {} pools, Raydium {} pools",
                &token_address[..8], dexscreener_count, geckoterminal_count, raydium_count
            )
        );

        // Show details from each API
        if let Ok(pairs) = &dexscreener_result {
            for (i, pair) in pairs.iter().take(3).enumerate() {
                let liquidity = pair.liquidity.as_ref().map(|l| l.usd).unwrap_or(0.0);
                log(
                    LogTag::Pool,
                    "TRIPLE_API_TEST_DX_POOL",
                    &format!(
                        "   🔸 DX Pool {}: {} ({}, ${:.2})",
                        i + 1, pair.pair_address, pair.dex_id, liquidity
                    )
                );
            }
        }

        if let Ok(pools) = &geckoterminal_result {
            for (i, pool) in pools.iter().take(3).enumerate() {
                log(
                    LogTag::Pool,
                    "TRIPLE_API_TEST_GT_POOL",
                    &format!(
                        "   🦎 GT Pool {}: {} ({}, ${:.2})",
                        i + 1, pool.pool_address, pool.dex_id, pool.liquidity_usd
                    )
                );
            }
        }

        if let Ok(pools) = &raydium_result {
            for (i, pool) in pools.iter().take(3).enumerate() {
                log(
                    LogTag::Pool,
                    "TRIPLE_API_TEST_RAY_POOL",
                    &format!(
                        "   ⚡ Ray Pool {}: {} ({}, ${:.2})",
                        i + 1, pool.pool_address, pool.pool_type, pool.liquidity_usd
                    )
                );
            }
        }

        // Small delay between tokens to respect rate limits
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    log(
        LogTag::Pool,
        "TRIPLE_API_TEST_COMPLETE",
        "🚀 Triple API test completed"
    );

    Ok(())
}
