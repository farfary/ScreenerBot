/// Pool Price System
///
/// This module provides a comprehensive pool-based price calculation system with caching,
/// background monitoring, and API fallback. It fetches pool data from DexScreener API
/// and calculates prices from pool reserves. Token selection for monitoring now relies
/// on the centralized price service priority list (no internal pool watch list).

use crate::logger::{ log, LogTag };
use crate::global::{ is_debug_pool_prices_enabled, CACHE_POOL_DIR };
use crate::tokens::dexscreener::{ get_token_pairs_from_api, TokenPair };
use crate::tokens::decimals::{ get_cached_decimals };
use crate::tokens::is_system_or_stable_token;
use crate::rpc::get_rpc_client;
use solana_sdk::{ account::Account, pubkey::Pubkey, commitment_config::CommitmentConfig };
use std::collections::{ HashMap, HashSet };
use std::str::FromStr;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;
use std::sync::Arc;
use serde::{ Deserialize, Serialize };
use chrono::{ DateTime, Utc };
use std::fs;
use std::path::Path;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Pool cache TTL (10 minutes)
/// Requirement: cache all tokens pools addresses and infos for maximum 10 minutes
const POOL_CACHE_TTL_SECONDS: i64 = 600;

/// Price cache TTL - aligned with 5s global monitoring cadence
const PRICE_CACHE_TTL_SECONDS: i64 = 5;

/// Minimum liquidity (USD) required to consider a pool usable for price calculation.
/// Lower this for testing environments if you want stats to increment sooner.
pub const MIN_POOL_LIQUIDITY_USD: f64 = 1000.0;

// Monitoring concurrency & performance budgeting
const POOL_MONITOR_CONCURRENCY: usize = 5; // Max concurrent token updates per cycle
const POOL_MONITOR_CYCLE_BUDGET_MS: u128 = 2500; // Soft per-cycle time budget
const POOL_MONITOR_PER_TOKEN_TIMEOUT_SECS: u64 = 6; // Guard per token update future

// Pool price history settings (in-memory only)
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

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get display name for pool program ID
pub fn get_pool_program_display_name(program_id: &str) -> String {
    match program_id {
        RAYDIUM_CPMM_PROGRAM_ID => "RAYDIUM CPMM".to_string(),
        RAYDIUM_LEGACY_AMM_PROGRAM_ID => "RAYDIUM LEGACY AMM".to_string(),
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
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub source: String, // "pool" or "api"
    pub calculated_at: DateTime<Utc>,
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
            // Use a small epsilon for floating point comparison (0.001% difference)
            let price_diff = (price_sol - last_entry.price_sol).abs();
            let relative_diff = if last_entry.price_sol != 0.0 {
                price_diff / last_entry.price_sol.abs()
            } else {
                price_diff
            };

            // Only record if price changed by more than 0.001% (1 in 100,000)
            if relative_diff < 0.00001 {
                return false; // Price hasn't changed significantly
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

    /// Get price history as tuples (for compatibility with existing code)
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
            .map(|entry| (
                entry.timestamp,
                entry.price_sol,
                entry.price_usd,
                entry.reserves_token,
                entry.reserves_sol,
                entry.liquidity_usd,
                entry.volume_24h,
            ))
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
    // watch_list removed; pool service delegates token selection to price service
    price_history: Arc<RwLock<HashMap<String, Vec<(DateTime<Utc>, f64)>>>>,
    // New pool-specific disk-based price history cache
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

/// Pool disk cache statistics for detailed reporting
impl PoolPriceService {
    /// Create new pool price service
    pub fn new() -> Self {
        Self {
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
        }
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
            if let Ok(fresh) = get_pool_service().calculate_pool_price(&token).await {
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

    /// Start background monitoring service
    pub async fn start_monitoring(&self) {
        let mut monitoring_active = self.monitoring_active.write().await;
        if *monitoring_active {
            log(LogTag::Pool, "WARNING", "Pool monitoring already active");
            return;
        }
        *monitoring_active = true;
        drop(monitoring_active);

        log(LogTag::Pool, "START", "Starting pool price monitoring service");

        let pool_cache = self.pool_cache.clone();
        let price_cache = self.price_cache.clone();
        // watch_list removed
        let pool_price_history = self.pool_price_history.clone();
        let monitoring_active = self.monitoring_active.clone();
        let stats_arc = self.stats.clone();
        let service_for_monitor = get_pool_service();

        // Start main monitoring loop (aligned to 5s system cadence)
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                interval.tick().await;

                // Check if monitoring should continue
                {
                    let active = monitoring_active.read().await;
                    if !*active {
                        break;
                    }
                }

                let cycle_start = Instant::now();

                // No more priority tokens - positions manager handles priority monitoring
                let tokens_to_monitor: Vec<String> = Vec::new();

                // Record last cycle token count early
                {
                    let mut stats = stats_arc.write().await;
                    stats.last_cycle_tokens = tokens_to_monitor.len() as u64;
                    stats.total_cycle_tokens += stats.last_cycle_tokens;
                    stats.monitoring_cycles += 1;
                    stats.avg_tokens_per_cycle = if stats.monitoring_cycles > 0 {
                        (stats.total_cycle_tokens as f64) / (stats.monitoring_cycles as f64)
                    } else {
                        0.0
                    };
                }

                // Monitoring is now handled by positions manager - this loop is disabled
                // Pool service still provides price calculation on-demand

                // Finish cycle timing stats
                let duration_ms = cycle_start.elapsed().as_secs_f64() * 1000.0;
                {
                    let mut stats = stats_arc.write().await;
                    stats.last_cycle_duration_ms = duration_ms;
                    stats.total_cycle_duration_ms += duration_ms;
                    stats.avg_cycle_duration_ms = if stats.monitoring_cycles > 0 {
                        stats.total_cycle_duration_ms / (stats.monitoring_cycles as f64)
                    } else {
                        0.0
                    };
                }
            }

            log(LogTag::Pool, "STOP", "Pool price monitoring service stopped");
        });
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
        let _ = self.get_pool_price(token_address, None).await;
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

    pub async fn get_recent_price_history(&self, token_address: &str) -> Vec<(DateTime<Utc>, f64)> {
        // First try disk cache for comprehensive history
        {
            let pool_cache = self.pool_price_history.read().await;
            if let Some(cache) = pool_cache.get(token_address) {
                let history = cache.get_combined_price_history();
                if !cache.pool_caches.is_empty() && !history.is_empty() {
                    if is_debug_pool_prices_enabled() && !history.is_empty() {
                        log(
                            LogTag::Pool,
                            "PRICE_HISTORY_DISK",
                            &format!(
                                "📊 Retrieved {} price history entries from disk cache for {}",
                                history.len(),
                                token_address
                            )
                        );
                    }
                    return history;
                }
            }
        }

        // Fallback to in-memory cache (limited to 10 entries)
        let history = self.price_history.read().await;
        let fallback_history = history.get(token_address).cloned().unwrap_or_default();

        if is_debug_pool_prices_enabled() && !fallback_history.is_empty() {
            log(
                LogTag::Pool,
                "PRICE_HISTORY_MEMORY",
                &format!(
                    "📈 Retrieved {} price history entries from memory cache for {}",
                    fallback_history.len(),
                    token_address
                )
            );
        }

        fallback_history
    }

    /// Get comprehensive price history for RL learning system
    pub async fn get_comprehensive_price_history(
        &self,
        token_address: &str
    ) -> Vec<(DateTime<Utc>, f64)> {
        let cache = self.pool_price_history.read().await;
        if let Some(token_cache) = cache.get(token_address) {
            let history = token_cache.get_combined_price_history();
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "PRICE_HISTORY",
                    &format!(
                        "📊 Analysis: Retrieved {} comprehensive price history entries for {} from {} pools",
                        history.len(),
                        token_address,
                        token_cache.pool_caches.len()
                    )
                );
            }

            history
        } else {
            // No cache available, return empty history
            Vec::new()
        }
    }

    /// Get detailed pool price history for a specific token
    pub async fn get_detailed_pool_price_history_for_token(
        &self,
        token_address: &str
    ) -> HashMap<
        String,
        Vec<(DateTime<Utc>, f64, Option<f64>, Option<f64>, Option<f64>, f64, Option<f64>)>
    > {
        let mut result = HashMap::new();

        let cache = self.pool_price_history.read().await;
        if let Some(token_cache) = cache.get(token_address) {
            for (pool_address, pool_cache) in &token_cache.pool_caches {
                result.insert(pool_address.clone(), pool_cache.get_detailed_price_history());
            }
        } else {
            // No cache available
        }

        result
    }

    /// Get all pool addresses that have price history for a token
    pub async fn get_pools_with_price_history_for_token(&self, token_address: &str) -> Vec<String> {
        let cache = self.pool_price_history.read().await;
        if let Some(token_cache) = cache.get(token_address) {
            token_cache.pool_caches.keys().cloned().collect()
        } else {
            // No cache available
            Vec::new()
        }
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
        // Update in-memory price history (for compatibility)
        {
            let mut history = self.price_history.write().await;
            let entry = history.entry(token_address.to_string()).or_insert_with(Vec::new);

            entry.push((Utc::now(), price_sol));

            // Keep only last 10 price points for -10% drop detection
            if entry.len() > 10 {
                entry.remove(0);
            }
        }

        // Update pool-specific disk-based price history cache
        {
            let mut pool_cache = self.pool_price_history.write().await;
            let token_cache = pool_cache
                .entry(token_address.to_string())
                .or_insert_with(||
                    TokenAggregatedPriceHistoryCache::new(token_address.to_string())
                );

            // Get or create pool-specific cache
            let pool_specific_cache = token_cache.pool_caches
                .entry(pool_address.to_string())
                .or_insert_with(||
                    PoolPriceHistoryCache::new(
                        token_address.to_string(),
                        pool_address.to_string(),
                        dex_id.to_string(),
                        pool_type.clone()
                    )
                );

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
                            "💾 Added price {:.12} SOL to pool cache for {}/{} (total entries: {})",
                            price_sol,
                            token_address,
                            pool_address,
                            pool_specific_cache.entries.len()
                        )
                    );
                }
            }
        }
    }

    /// Legacy add_price_to_history method (maintained for compatibility)
    async fn add_price_to_history(&self, token_address: &str, price: f64) {
        self.add_price_to_pool_history(
            token_address,
            "unknown",
            "unknown",
            None,
            price,
            None,
            None,
            None,
            0.0,
            None,
            "pool_legacy"
        ).await;
    }

    /// Clean up old price history entries
    async fn cleanup_price_history(&self) {
        let mut history = self.price_history.write().await;
        let cutoff = Utc::now() - chrono::Duration::hours(1); // Keep 1 hour of history

        for entry in history.values_mut() {
            entry.retain(|(timestamp, _)| *timestamp > cutoff);
        }

        // Remove empty entries
        history.retain(|_, entry| !entry.is_empty());
    }

    /// Get pool price for a token (main entry point)
    pub async fn get_pool_price(
        &self,
        token_address: &str,
        api_price_sol: Option<f64> // SOL price from API for comparison
    ) -> Option<PoolPriceResult> {
        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "PRICE_REQUEST",
                &format!(
                    "🎯 POOL PRICE REQUEST for {}: API_price={:.12} SOL",
                    token_address,
                    api_price_sol.unwrap_or(0.0)
                )
            );
        }

        let mut was_cache_hit = false;
        let mut was_blockchain = false;

        // Check price cache first
        {
            let price_cache = self.price_cache.read().await;
            if let Some(cached_price) = price_cache.get(token_address) {
                let age = Utc::now() - cached_price.calculated_at;
                if age.num_seconds() <= PRICE_CACHE_TTL_SECONDS {
                    was_cache_hit = true;
                    self.record_price_request(true, was_cache_hit, was_blockchain).await;

                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "CACHE_HIT",
                            &format!(
                                "🔄 CACHE HIT for {}: cached_price={:.12} SOL, age={}s, cache_ttl={}s, updating timestamp",
                                token_address,
                                cached_price.price_sol.unwrap_or(0.0),
                                age.num_seconds(),
                                PRICE_CACHE_TTL_SECONDS
                            )
                        );
                    }

                    // Return cached result with updated timestamp for real-time accuracy
                    let mut updated_result = cached_price.clone();
                    let old_timestamp = updated_result.calculated_at;
                    updated_result.calculated_at = Utc::now();

                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "TIMESTAMP_UPDATE",
                            &format!(
                                "⏰ TIMESTAMP UPDATE for {}: {} -> {} (cached price still valid)",
                                token_address,
                                old_timestamp.format("%H:%M:%S%.3f"),
                                updated_result.calculated_at.format("%H:%M:%S%.3f")
                            )
                        );
                    }

                    return Some(updated_result);
                } else {
                    // Stale-while-revalidate: serve stale cached price immediately and refresh in background
                    if let Some(price_sol) = cached_price.price_sol {
                        was_cache_hit = true;

                        if is_debug_pool_prices_enabled() {
                            log(
                                LogTag::Pool,
                                "CACHE_STALE_SERVE",
                                &format!(
                                    "⚡ Serving STALE cache for {}: price={:.12} SOL, age={}s (> TTL {}s). Refresh scheduled.",
                                    token_address,
                                    price_sol,
                                    age.num_seconds(),
                                    PRICE_CACHE_TTL_SECONDS
                                )
                            );
                        }

                        // Clone for return with updated timestamp
                        let mut updated_result = cached_price.clone();
                        updated_result.calculated_at = Utc::now();

                        // Fire-and-forget background refresh (deduplicated)
                        self.trigger_background_refresh(token_address).await;

                        // Record as cache-served success (not blockchain)
                        self.record_price_request(true, was_cache_hit, was_blockchain).await;
                        return Some(updated_result);
                    } else if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "CACHE_EXPIRED",
                            &format!("❌ CACHE EXPIRED for {} and contains no price; proceeding to fresh calculation", token_address)
                        );
                    }
                }
            } else if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "CACHE_MISS",
                    &format!("❓ NO CACHE for {}: first time or cleared cache, will fetch FRESH price from blockchain", token_address)
                );
            }
        }

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
        match self.calculate_pool_price(token_address).await {
            Ok(pool_result) => {
                let has_price = pool_result.price_sol.is_some();
                was_blockchain = pool_result.source == "pool";
                self.record_price_request(has_price, was_cache_hit, was_blockchain).await;

                if is_debug_pool_prices_enabled() {
                    if let Some(price_sol) = pool_result.price_sol {
                        // Log the pool price calculation
                        log(
                            LogTag::Pool,
                            "FRESH_CALC_SUCCESS",
                            &format!(
                                "✅ FRESH POOL PRICE calculated for {}: {:.12} SOL from pool {} ({})",
                                token_address,
                                price_sol,
                                pool_result.pool_address,
                                pool_result.pool_type
                                    .as_ref()
                                    .unwrap_or(&"Unknown Pool".to_string())
                            )
                        );

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
                                     📊 API={:.12} SOL vs 🏊 POOL={:.12} SOL \
                                     📈 Diff={:.12} SOL ({:+.2}%) - Pool: {} ({})",
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
                                    "🏊 POOL-ONLY PRICE for {}: {:.12} SOL (no API price for comparison)",
                                    token_address,
                                    price_sol
                                )
                            );
                        }
                    } else {
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

                // Cache the result
                {
                    let mut price_cache = self.price_cache.write().await;
                    price_cache.insert(token_address.to_string(), pool_result.clone());

                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "CACHE_STORED",
                            &format!(
                                "💾 CACHED fresh price for {}: {:.12} SOL (TTL={}s) at {}",
                                token_address,
                                pool_result.price_sol.unwrap_or(0.0),
                                PRICE_CACHE_TTL_SECONDS,
                                pool_result.calculated_at.format("%H:%M:%S%.3f")
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
                            None, // reserves_token - not available in current PoolPriceResult
                            None, // reserves_sol - not available in current PoolPriceResult
                            pool_result.liquidity_usd,
                            Some(pool_result.volume_24h),
                            &pool_result.source
                        ).await;

                        if is_debug_pool_prices_enabled() {
                            log(
                                LogTag::Pool,
                                "HISTORY_ADD",
                                &format!(
                                    "📈 Added price {:.12} SOL to history for {}",
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
                None
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
                    .max_by(|a, b|
                        a.liquidity_usd
                            .partial_cmp(&b.liquidity_usd)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    );

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

        // Check pool cache first
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
                        &format!("⏰ Pool cache EXPIRED for {}, will fetch fresh pools from API", token_address)
                    );
                }
            } else if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "FETCH_CACHE_MISS",
                    &format!("❓ No cached pools for {}, will fetch from API", token_address)
                );
            }
        }

        // Fetch from API
        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "FETCH_API_START",
                &format!("🔄 Fetching pools from DexScreener API for {}", token_address)
            );
        }

        let api_start_time = Utc::now();
        let pairs = match get_token_pairs_from_api(token_address).await {
            Ok(pairs) => pairs,
            Err(e) => {
                // Handle API timeouts gracefully - this is often normal during shutdown
                if e.contains("timeout") || e.contains("shutting down") {
                    log(
                        LogTag::Pool,
                        "INFO",
                        &format!(
                            "API timeout for {} (system may be shutting down): {}",
                            token_address,
                            e
                        )
                    );
                } else {
                    log(LogTag::Pool, "ERROR", &format!("API error for {}: {}", token_address, e));
                }
                return Err(format!("Failed to fetch pools from API: {}", e));
            }
        };
        let api_duration = Utc::now() - api_start_time;

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "FETCH_API_COMPLETE",
                &format!(
                    "✅ API fetch complete for {}: got {} pairs in {}ms",
                    token_address,
                    pairs.len(),
                    api_duration.num_milliseconds()
                )
            );
        }

        let mut cached_pools = Vec::new();
        for (index, pair) in pairs.iter().enumerate() {
            match CachedPoolInfo::from_token_pair(&pair) {
                Ok(cached_pool) => {
                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "FETCH_PARSE_SUCCESS",
                            &format!(
                                "✅ Parsed pool #{} for {}: {} ({}, liquidity: ${:.2})",
                                index + 1,
                                token_address,
                                cached_pool.pair_address,
                                cached_pool.dex_id, // Keep API dex_id for debugging pool fetching
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
                                "❌ Failed to parse pool #{} for {}: {} - Error: {}",
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

        // Sort by liquidity (highest first)
        cached_pools.sort_by(|a, b|
            b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
        );

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

        // Cache the results
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
    async fn calculate_pool_price(&self, token_address: &str) -> Result<PoolPriceResult, String> {
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
        let (price_sol, actual_pool_type) = match
            self.calculate_real_pool_price_from_reserves(
                &best_pool.pair_address,
                token_address
            ).await
        {
            Ok(Some(pool_price_info)) => {
                if is_debug_pool_prices_enabled() {
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
                (Some(pool_price_info.price_sol), Some(pool_price_info.pool_type))
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
                (None, None)
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
                (None, None)
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
            liquidity_usd: best_pool.liquidity_usd,
            volume_24h: best_pool.volume_24h,
            source: "pool".to_string(),
            calculated_at: calculation_time,
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
        token_mint: &str
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
                    liquidity_usd: 0.0, // No API data for liquidity in direct mode
                    volume_24h: 0.0, // No API data for volume in direct mode
                    source: "pool_direct".to_string(),
                    calculated_at: calculation_time,
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
        Self { consecutive_failures: 0, next_retry: Instant::now() }
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

/// Try to get global service without forcing initialization (may return None)
pub fn try_get_pool_service() -> Option<&'static PoolPriceService> {
    unsafe { GLOBAL_POOL_SERVICE.as_ref() }
}

/// Get or initialize service, logging on failure
pub fn get_pool_service() -> &'static PoolPriceService {
    init_pool_service()
}

/// Get comprehensive price history for analysis (global function)
pub async fn get_price_history_for_analysis(token_address: &str) -> Vec<(DateTime<Utc>, f64)> {
    let pool_service = get_pool_service();
    pool_service.get_comprehensive_price_history(token_address).await
}

/// Get detailed pool price history for a specific token (NEW FUNCTION)
pub async fn get_detailed_pool_price_history(
    token_address: &str
) -> HashMap<
    String,
    Vec<(DateTime<Utc>, f64, Option<f64>, Option<f64>, Option<f64>, f64, Option<f64>)>
> {
    let pool_service = get_pool_service();
    pool_service.get_detailed_pool_price_history_for_token(token_address).await
}

/// Get all pool addresses that have price history for a token (NEW FUNCTION)
pub async fn get_pools_with_price_history(token_address: &str) -> Vec<String> {
    let pool_service = get_pool_service();
    pool_service.get_pools_with_price_history_for_token(token_address).await
}

// =============================================================================
// GLOBAL HELPERS FOR POOLS INFOS CACHE
// =============================================================================

/// Get cached pools infos for a token, if any (not guaranteed fresh)
pub async fn get_cached_pools_infos_safe(token_address: &str) -> Option<Vec<CachedPoolInfo>> {
    let service = get_pool_service();
    service.get_cached_pools_infos(token_address).await
}

/// Refresh pools infos for a token (rate-limited internally) and return updated list
pub async fn refresh_pools_infos_safe(token_address: &str) -> Result<Vec<CachedPoolInfo>, String> {
    let service = get_pool_service();
    service.refresh_pools_infos(token_address).await
}

/// Get tokens which have pools infos within the last `window_seconds`
pub async fn get_tokens_with_recent_pools_infos_safe(window_seconds: i64) -> Vec<String> {
    let service = get_pool_service();
    service.get_tokens_with_recent_pools_infos(window_seconds).await
}

/// Refresh pools infos for a list of tokens (only those missing/expired). Returns count updated.
pub async fn refresh_pools_infos_for_tokens_safe(mints: &[String], max_tokens: usize) -> usize {
    let service = get_pool_service();
    service.refresh_pools_infos_for_tokens(mints, max_tokens).await
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
    // Centralized RPC client (no direct SolanaRpcClient instantiation allowed)
    rpc_client: &'static crate::rpc::RpcClient,
    pool_cache: Arc<RwLock<HashMap<String, PoolInfo>>>,
    price_cache: Arc<RwLock<HashMap<String, (f64, Instant)>>>,
    // Short TTL cache for vault (token account) balances to avoid duplicate RPCs inside monitoring cycles
    vault_balance_cache: Arc<RwLock<HashMap<String, (u64, Instant)>>>,
    stats: Arc<RwLock<PoolStats>>,
    debug_enabled: bool,
}

impl PoolPriceCalculator {
    /// Create new pool price calculator (always uses centralized RPC system).
    pub fn new() -> Self {
        let rpc_client = get_rpc_client();
        Self {
            rpc_client,
            pool_cache: Arc::new(RwLock::new(HashMap::new())),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            vault_balance_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PoolStats::new())),
            debug_enabled: false,
        }
    }
    // Removed legacy constructors (new_with_url / new_with_rpc) per centralized-systems instructions.

    /// Enable debug mode
    pub fn enable_debug(&mut self) {
        self.debug_enabled = true;
        log(LogTag::Pool, "DEBUG", "Pool calculator debug mode enabled");
    }

    /// Get pool information from on-chain data
    pub async fn get_pool_info(&self, pool_address: &str) -> Result<Option<PoolInfo>, String> {
        // Check cache first
        {
            let cache = self.pool_cache.read().await;
            if let Some(cached_pool) = cache.get(pool_address) {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "CACHE",
                        &format!("Found cached pool info for {}", pool_address)
                    );
                }
                return Ok(Some(cached_pool.clone()));
            }
        }

        let start_time = Instant::now();

        // Parse pool address
        let pool_pubkey = Pubkey::from_str(pool_address).map_err(|e|
            format!("Invalid pool address {}: {}", pool_address, e)
        )?;

        // Get account data via centralized async RPC
        let account = self.rpc_client
            .get_account(&pool_pubkey).await
            .map_err(|e| { format!("Failed to get pool account {}: {}", pool_address, e) })?;

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
            log(
                LogTag::Pool,
                "PROGRAM_ID_COMPARISON",
                &format!(
                    "Comparing:\n  Pool program: '{}'\n  Legacy AMM:   '{}'\n  CPMM:         '{}'\n  Match Legacy: {}",
                    program_id,
                    RAYDIUM_LEGACY_AMM_PROGRAM_ID,
                    RAYDIUM_CPMM_PROGRAM_ID,
                    program_id == RAYDIUM_LEGACY_AMM_PROGRAM_ID
                )
            );
        }

        // Decode based on program ID
        let pool_info = match program_id.as_str() {
            RAYDIUM_CPMM_PROGRAM_ID => {
                if self.debug_enabled {
                    log(LogTag::Pool, "DECODER_SELECT", "Using Raydium CPMM decoder");
                }
                self.decode_raydium_cpmm_pool(pool_address, &account).await?
            }
            RAYDIUM_LEGACY_AMM_PROGRAM_ID => {
                if self.debug_enabled {
                    log(LogTag::Pool, "DECODER_SELECT", "Using Raydium Legacy AMM decoder");
                }
                self.decode_raydium_legacy_amm_pool(pool_address, &account).await?
            }
            METEORA_DAMM_V2_PROGRAM_ID => {
                if self.debug_enabled {
                    log(LogTag::Pool, "DECODER_SELECT", "Using Meteora DAMM v2 decoder");
                }
                self.decode_meteora_damm_v2_pool(pool_address, &account).await?
            }
            METEORA_DLMM_PROGRAM_ID => {
                if self.debug_enabled {
                    log(LogTag::Pool, "DECODER_SELECT", "Using Meteora DLMM decoder");
                }
                self.decode_meteora_dlmm_pool(pool_address, &account).await?
            }
            ORCA_WHIRLPOOL_PROGRAM_ID => {
                if self.debug_enabled {
                    log(LogTag::Pool, "DECODER_SELECT", "Using Orca Whirlpool decoder");
                }
                self.decode_orca_whirlpool_pool(pool_address, &account).await?
            }
            PUMP_FUN_AMM_PROGRAM_ID => {
                if self.debug_enabled {
                    log(LogTag::Pool, "DECODER_SELECT", "Using Pump.fun AMM decoder");
                }
                self.decode_pump_fun_amm_pool(pool_address, &account).await?
            }
            _ => {
                // Record failure before returning
                {
                    let mut stats = self.stats.write().await;
                    stats.record_calculation(
                        false,
                        start_time.elapsed().as_millis() as f64,
                        &program_id
                    );
                }
                return Err(format!("Unsupported pool program ID: {}", program_id));
            }
        };

        // Cache the result
        {
            let mut cache = self.pool_cache.write().await;
            cache.insert(pool_address.to_string(), pool_info.clone());
        }

        // Update stats (success)
        {
            let mut stats = self.stats.write().await;
            stats.record_calculation(true, start_time.elapsed().as_millis() as f64, &program_id);
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "SUCCESS",
                &format!("Pool info decoded in {:.2}ms", start_time.elapsed().as_millis())
            );
        }

        Ok(Some(pool_info))
    }

    /// Calculate token price from pool reserves
    pub async fn calculate_token_price(
        &self,
        pool_address: &str,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        let cache_key = format!("{}_{}", pool_address, token_mint);

        // Check price cache (valid for 30 seconds)
        {
            let cache = self.price_cache.read().await;
            if let Some((price, timestamp)) = cache.get(&cache_key) {
                if timestamp.elapsed().as_secs() < 30 {
                    if self.debug_enabled {
                        log(
                            LogTag::Pool,
                            "CACHE",
                            &format!("Using cached price {:.12} SOL for {}", price, token_mint)
                        );
                    }

                    // Update cache hit stats
                    {
                        let mut stats = self.stats.write().await;
                        stats.cache_hits += 1;
                    }

                    // Return cached price with minimal pool info
                    return Ok(
                        Some(PoolPriceInfo {
                            pool_address: pool_address.to_string(),
                            pool_program_id: "cached".to_string(),
                            pool_type: "cached".to_string(),
                            token_mint: token_mint.to_string(),
                            price_sol: *price,
                            token_reserve: 0,
                            sol_reserve: 0,
                            token_decimals: 9, // Default assumption, should be fetched properly
                            sol_decimals: 9,
                        })
                    );
                }
            }
        }

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

        // Cache the price
        if let Some(ref price_info) = price_info {
            let mut cache = self.price_cache.write().await;
            cache.insert(cache_key, (price_info.price_sol, Instant::now()));
        }

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

    /// Get multiple account data in a single RPC call (for future optimization)
    pub async fn get_multiple_pool_accounts(
        &self,
        pool_addresses: &[String]
    ) -> Result<HashMap<String, Account>, String> {
        let pubkeys: Result<Vec<Pubkey>, _> = pool_addresses
            .iter()
            .map(|addr| Pubkey::from_str(addr))
            .collect();

        let pubkeys = pubkeys.map_err(|e| format!("Invalid pool address: {}", e))?;

        let accounts = self.rpc_client
            .get_multiple_accounts(&pubkeys).await
            .map_err(|e| { format!("Failed to get multiple accounts: {}", e) })?;

        let mut result = HashMap::new();
        for (i, account_opt) in accounts.into_iter().enumerate() {
            if let Some(account) = account_opt {
                result.insert(pool_addresses[i].clone(), account);
            }
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "RPC",
                &format!("Retrieved {} pool accounts in single call", result.len())
            );
        }

        Ok(result)
    }

    /// Get statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }

    /// Clear caches
    pub async fn clear_caches(&self) {
        {
            let mut pool_cache = self.pool_cache.write().await;
            pool_cache.clear();
        }
        {
            let mut price_cache = self.price_cache.write().await;
            price_cache.clear();
        }
        log(LogTag::Pool, "CACHE", "Pool and price caches cleared");
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

    /// Batch prefetch raw pool accounts and (optionally) vault token accounts for a set of pool addresses.
    /// Goal: minimize per-price call latency by front-loading a single get_multiple_accounts.
    /// NOTE: This currently only caches raw pool accounts; vault extraction will be enhanced later.
    pub async fn batch_prefetch_pools_and_vaults(
        &self,
        pool_addresses: &[String]
    ) -> Result<(), String> {
        if pool_addresses.is_empty() {
            return Ok(());
        }
        // Identify pools missing from cache (PoolInfo has no embedded timestamp here)
        let mut to_fetch: Vec<Pubkey> = Vec::new();
        let mut map: HashMap<Pubkey, String> = HashMap::new();
        {
            let cache = self.pool_cache.read().await;
            for addr in pool_addresses {
                if cache.get(addr).is_none() {
                    if let Ok(pk) = Pubkey::from_str(addr) {
                        to_fetch.push(pk);
                        map.insert(pk, addr.clone());
                    }
                }
            }
        }
        if to_fetch.is_empty() {
            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "BATCH_PREFETCH",
                    &format!("All {} pools fresh in cache", pool_addresses.len())
                );
            }
            return Ok(());
        }
        if self.debug_enabled {
            log(
                LogTag::Pool,
                "BATCH_PREFETCH",
                &format!(
                    "Fetching {} / {} pools ({} cached)",
                    to_fetch.len(),
                    pool_addresses.len(),
                    pool_addresses.len() - to_fetch.len()
                )
            );
        }
        let accounts = self.rpc_client
            .get_multiple_accounts(&to_fetch).await
            .map_err(|e| format!("Pool batch fetch failed: {}", e))?;
        let mut inserted = 0usize;
        {
            let mut cache = self.pool_cache.write().await;
            for (i, acct_opt) in accounts.into_iter().enumerate() {
                if let Some(acct) = acct_opt {
                    if let Some(addr) = map.get(&to_fetch[i]) {
                        // Decode minimally to PoolInfo so downstream users have data ready
                        let program = acct.owner.to_string();
                        let decoded = match program.as_str() {
                            id if id == RAYDIUM_CPMM_PROGRAM_ID =>
                                self.decode_raydium_cpmm_pool(addr, &acct).await.ok(),
                            id if id == RAYDIUM_LEGACY_AMM_PROGRAM_ID =>
                                self.decode_raydium_legacy_amm_pool(addr, &acct).await.ok(),
                            id if id == METEORA_DLMM_PROGRAM_ID =>
                                self.decode_meteora_dlmm_pool(addr, &acct).await.ok(),
                            id if id == METEORA_DAMM_V2_PROGRAM_ID =>
                                self.decode_meteora_damm_v2_pool(addr, &acct).await.ok(),
                            id if id == ORCA_WHIRLPOOL_PROGRAM_ID =>
                                self.decode_orca_whirlpool_pool(addr, &acct).await.ok(),
                            _ => None,
                        };
                        if let Some(info) = decoded {
                            cache.insert(addr.clone(), info);
                            inserted += 1;
                        }
                    }
                }
            }
        }
        if self.debug_enabled {
            log(
                LogTag::Pool,
                "BATCH_PREFETCH",
                &format!("Decoded & cached {} pool accounts", inserted)
            );
        }
        Ok(())
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
            GLOBAL_POOL_PRICE_CALCULATOR = Some(PoolPriceCalculator::new());
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

            // Additional validation checks
            if sol_adjusted <= 0.0 || token_adjusted <= 0.0 {
                log(
                    LogTag::Pool,
                    "CALC_WARN",
                    &format!(
                        "WARNING: Zero or negative adjusted values detected! \
                         SOL_adj: {:.12}, Token_adj: {:.12}",
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
        const VAULT_TTL_SECS: u64 = 2; // short-lived cache horizon
        let now = Instant::now();
        let mut missing: Vec<&str> = Vec::new();
        {
            let cache = self.vault_balance_cache.read().await;
            for &v in &[vault_0, vault_1] {
                match cache.get(v) {
                    Some((_, ts)) if now.duration_since(*ts).as_secs() < VAULT_TTL_SECS => {}
                    _ => missing.push(v),
                }
            }
        }
        if !missing.is_empty() {
            let mut pubkeys = Vec::new();
            for v in &missing {
                if let Ok(pk) = Pubkey::from_str(v) {
                    pubkeys.push(pk);
                }
            }
            if !pubkeys.is_empty() {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "VAULT_BATCH",
                        &format!("Fetching {} vault accounts (cache miss)", pubkeys.len())
                    );
                }
                let accounts = self.rpc_client
                    .get_multiple_accounts(&pubkeys).await
                    .map_err(|e| format!("Failed to get vault accounts: {}", e))?;
                let mut cache = self.vault_balance_cache.write().await;
                for (idx, acct_opt) in accounts.into_iter().enumerate() {
                    if let Some(acct) = acct_opt {
                        if let Ok(amount) = Self::decode_token_account_amount(&acct.data) {
                            cache.insert(missing[idx].to_string(), (amount, now));
                        }
                    }
                }
            }
        }
        let cache = self.vault_balance_cache.read().await;
        let balance_0 = cache
            .get(vault_0)
            .map(|(a, _)| *a)
            .ok_or_else(|| "Vault 0 account not found".to_string())?;
        let balance_1 = cache
            .get(vault_1)
            .map(|(a, _)| *a)
            .ok_or_else(|| "Vault 1 account not found".to_string())?;

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

        Ok((balance_0, balance_1))
    }

    /// Get DLMM token reserve balances from token accounts
    async fn get_dlmm_vault_balances(
        &self,
        reserve_0: &str,
        reserve_1: &str
    ) -> Result<(u64, u64), String> {
        const VAULT_TTL_SECS: u64 = 2;
        let now = Instant::now();
        let mut missing: Vec<&str> = Vec::new();
        {
            let cache = self.vault_balance_cache.read().await;
            for &v in &[reserve_0, reserve_1] {
                match cache.get(v) {
                    Some((_, ts)) if now.duration_since(*ts).as_secs() < VAULT_TTL_SECS => {}
                    _ => missing.push(v),
                }
            }
        }
        if !missing.is_empty() {
            let pubkeys: Vec<Pubkey> = missing
                .iter()
                .filter_map(|v| Pubkey::from_str(v).ok())
                .collect();
            if !pubkeys.is_empty() {
                if self.debug_enabled {
                    log(
                        LogTag::Pool,
                        "DLMM_VAULT_BATCH",
                        &format!("Fetching {} DLMM reserve accounts", pubkeys.len())
                    );
                }
                let accounts = self.rpc_client
                    .get_multiple_accounts(&pubkeys).await
                    .map_err(|e| format!("Failed to get DLMM reserve accounts: {}", e))?;
                let mut cache = self.vault_balance_cache.write().await;
                for (i, acct_opt) in accounts.into_iter().enumerate() {
                    if let Some(acct) = acct_opt {
                        if let Ok(amount) = Self::decode_token_account_amount(&acct.data) {
                            cache.insert(missing[i].to_string(), (amount, now));
                        }
                    }
                }
            }
        }
        let cache = self.vault_balance_cache.read().await;
        let balance_0 = cache
            .get(reserve_0)
            .map(|(a, _)| *a)
            .ok_or_else(|| format!("DLMM reserve 0 account {} not found", reserve_0))?;
        let balance_1 = cache
            .get(reserve_1)
            .map(|(a, _)| *a)
            .ok_or_else(|| format!("DLMM reserve 1 account {} not found", reserve_1))?;

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

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "LEGACY_PARSE_FIELDS",
                &format!(
                    "Using direct offset parsing: pc_mint at 0x190, coin_mint at 0x1b0, base_vault at 0x150, quote_vault at 0x160"
                )
            );

            log(
                LogTag::Pool,
                "LEGACY_VAULT_ADDRESSES",
                &format!(
                    "Parsed vault addresses (with correct mapping):\n  \
                         - Base Vault (SOL, {}): {}\n  \
                         - Quote Vault (Token, {}): {}\n  \
                         - Coin Vault (mapped to Quote): {}\n  \
                         - PC Vault (mapped to Base): {}",
                    pc_mint,
                    base_vault,
                    coin_mint,
                    quote_vault,
                    coin_vault,
                    pc_vault
                )
            );

            // Compare with expected addresses from pool data
            let expected_base_vault = "F6iWqisguZYprVwp916BgGR7d5ahP6Ev5E213k8y3MEb";
            let expected_quote_vault = "7bxbfwXi1CY7zWUXW35PBMZjhPD27SarVuHaehMzR2Fn";

            log(
                LogTag::Pool,
                "LEGACY_VAULT_COMPARISON",
                &format!(
                    "Expected vault addresses from pool data:\n  \
                         - Base Vault (SOL): {}\n  \
                         - Quote Vault (Token): {}",
                    expected_base_vault,
                    expected_quote_vault
                )
            );

            // Check if our parsed addresses match expected
            let base_vault_str = base_vault.to_string();
            let quote_vault_str = quote_vault.to_string();

            if base_vault_str == expected_base_vault && quote_vault_str == expected_quote_vault {
                log(
                    LogTag::Pool,
                    "LEGACY_VAULT_MATCH",
                    "✅ Parsed vault addresses match expected pool data EXACTLY!"
                );
            } else {
                log(
                    LogTag::Pool,
                    "LEGACY_VAULT_MISMATCH",
                    &format!(
                        "❌ Parsed vault addresses don't match expected:\n  \
                             Parsed: base={}, quote={}\n  \
                             Expected: base={}, quote={}",
                        base_vault_str,
                        quote_vault_str,
                        expected_base_vault,
                        expected_quote_vault
                    )
                );
            }
        }

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
                    Duration::from_secs(5), // Increased timeout for better reliability
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

/// Price cache entry structure for compatibility with existing code
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

/// Get token price using pool service - instant response for cached prices
pub async fn get_token_price_safe(mint: &str) -> Option<f64> {
    if is_debug_pool_prices_enabled() {
        log(LogTag::Pool, "PRICE_REQUEST", &format!("🌐 Price request for {}", mint));
    }

    let pool_service = get_pool_service();

    // Try to get pool price with fast cache lookup
    if let Some(pool_result) = pool_service.get_pool_price(mint, None).await {
        if let Some(price) = pool_result.price_sol {
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "PRICE_SUCCESS",
                    &format!(
                        "✅ Got price for {}: ${:.12} SOL from {}",
                        mint,
                        price,
                        pool_result.source
                    )
                );
            }
            return Some(price);
        }
    }

    if is_debug_pool_prices_enabled() {
        log(LogTag::Pool, "PRICE_MISS", &format!("❌ No price available for {}", mint));
    }

    None
}

/// Get token price using pool service - waits for update on cache miss (blocking version)
pub async fn get_token_price_blocking_safe(mint: &str) -> Option<f64> {
    if is_debug_pool_prices_enabled() {
        log(
            LogTag::Pool,
            "PRICE_BLOCKING_REQUEST",
            &format!("🌐 Blocking price request for {}", mint)
        );
    }

    let pool_service = get_pool_service();

    // Force availability check and calculation
    if !pool_service.check_token_availability(mint).await {
        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "PRICE_BLOCKING_UNAVAILABLE",
                &format!("❌ Token {} not available for pool pricing", mint)
            );
        }
        return None;
    }

    // Get pool price (this will calculate if needed)
    if let Some(pool_result) = pool_service.get_pool_price(mint, None).await {
        if let Some(price) = pool_result.price_sol {
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "PRICE_BLOCKING_SUCCESS",
                    &format!(
                        "✅ Got blocking price for {}: ${:.12} SOL from {}",
                        mint,
                        price,
                        pool_result.source
                    )
                );
            }
            return Some(price);
        }
    }

    if is_debug_pool_prices_enabled() {
        log(
            LogTag::Pool,
            "PRICE_BLOCKING_FAILED",
            &format!("❌ Failed to get blocking price for {}", mint)
        );
    }

    None
}

/// Force an immediate price refresh for a token (bypass cache freshness)
pub async fn force_refresh_token_price_safe(mint: &str) {
    if is_debug_pool_prices_enabled() {
        log(LogTag::Pool, "FORCE_REFRESH", &format!("🔄 Force refreshing price for {}", mint));
    }

    let pool_service = get_pool_service();

    // Force refresh by calling get_pool_price with no cache consideration
    let _ = pool_service.get_pool_price(mint, None).await;

    if is_debug_pool_prices_enabled() {
        log(
            LogTag::Pool,
            "FORCE_REFRESH_COMPLETE",
            &format!("✅ Force refresh completed for {}", mint)
        );
    }
}

/// Update multiple token prices (called from monitor) - now uses pool service
pub async fn update_tokens_prices_safe(mints: &[String]) {
    if is_debug_pool_prices_enabled() {
        log(
            LogTag::Pool,
            "BATCH_UPDATE_REQUEST",
            &format!("🔧 Batch price update for {} tokens: {:?}", mints.len(), mints)
        );
    }

    let pool_service = get_pool_service();
    let mut success_count = 0;
    let mut error_count = 0;

    for mint in mints {
        match pool_service.get_pool_price(mint, None).await {
            Some(pool_result) => {
                if pool_result.price_sol.is_some() {
                    success_count += 1;
                } else {
                    error_count += 1;
                }
            }
            None => {
                error_count += 1;
            }
        }
    }

    if is_debug_pool_prices_enabled() {
        log(
            LogTag::Pool,
            "BATCH_UPDATE_COMPLETE",
            &format!(
                "✅ Batch update complete: {}/{} successful, {} errors",
                success_count,
                mints.len(),
                error_count
            )
        );
    }
}
