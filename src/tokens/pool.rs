/// Pool Price System
///
/// This module provides a comprehensive pool-based price calculation system with caching,
/// background monitoring, and API fallback. It fetches pool data from DexScreener API,
/// calculates prices from pool reserves, and maintains a watch list for continuous monitoring.

use crate::logger::{ log, LogTag };
use crate::global::is_debug_pool_prices_enabled;
use crate::tokens::api::{ get_token_pairs_from_api, TokenPair };
use crate::tokens::decimals::{ get_token_decimals_from_chain, get_cached_decimals };
use solana_client::rpc_client::RpcClient;
use solana_sdk::{ account::Account, pubkey::Pubkey, commitment_config::CommitmentConfig };
use std::collections::HashMap;
use std::str::FromStr;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;
use std::sync::Arc;
use serde::{ Deserialize, Serialize };
use chrono::{ DateTime, Utc };

// =============================================================================
// CONSTANTS
// =============================================================================

/// Pool cache TTL (5 minutes)
const POOL_CACHE_TTL_SECONDS: i64 = 300;

/// Price cache TTL (1 second for real-time monitoring)
const PRICE_CACHE_TTL_SECONDS: i64 = 1;

/// SOL mint address
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Raydium CPMM Program ID
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

/// Meteora DAMM v2 Program ID
pub const METEORA_DAMM_V2_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";

/// Meteora DLMM Program ID
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get display name for pool program ID
pub fn get_pool_program_display_name(program_id: &str) -> String {
    match program_id {
        RAYDIUM_CPMM_PROGRAM_ID => "RAYDIUM CPMM".to_string(),
        METEORA_DAMM_V2_PROGRAM_ID => "METEORA DAMM v2".to_string(),
        METEORA_DLMM_PROGRAM_ID => "METEORA DLMM".to_string(),
        _ => format!("UNKNOWN ({})", &program_id[..8]), // Show first 8 chars for unknown programs
    }
}

/// Check if a token is a stable/system token that should be excluded from watch lists
fn is_system_or_stable_token(mint: &str) -> bool {
    let system_tokens = [
        "So11111111111111111111111111111111111111112", // SOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", // USDT
        "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y7ARj", // stSOL
        "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So", // mSOL
        "11111111111111111111111111111111", // System Program (invalid token)
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", // Token Program
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb", // Token-2022 Program
    ];

    system_tokens.contains(&mint)
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

/// Watch list entry for background monitoring
#[derive(Debug, Clone)]
pub struct WatchListEntry {
    pub token_address: String,
    pub added_at: DateTime<Utc>,
    pub priority: i32,
    pub last_price_check: Option<DateTime<Utc>>,
}

// =============================================================================
// MAIN POOL PRICE SERVICE
// =============================================================================

pub struct PoolPriceService {
    pool_cache: Arc<RwLock<HashMap<String, Vec<CachedPoolInfo>>>>,
    price_cache: Arc<RwLock<HashMap<String, PoolPriceResult>>>,
    availability_cache: Arc<RwLock<HashMap<String, TokenAvailability>>>,
    watch_list: Arc<RwLock<HashMap<String, WatchListEntry>>>,
    monitoring_active: Arc<RwLock<bool>>,
}

impl PoolPriceService {
    /// Create new pool price service
    pub fn new() -> Self {
        Self {
            pool_cache: Arc::new(RwLock::new(HashMap::new())),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            availability_cache: Arc::new(RwLock::new(HashMap::new())),
            watch_list: Arc::new(RwLock::new(HashMap::new())),
            monitoring_active: Arc::new(RwLock::new(false)),
        }
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
        let watch_list = self.watch_list.clone();
        let monitoring_active = self.monitoring_active.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3));

            loop {
                interval.tick().await;

                // Check if monitoring should continue
                {
                    let active = monitoring_active.read().await;
                    if !*active {
                        break;
                    }
                }

                // Process watch list
                let tokens_to_monitor = {
                    let watch_list = watch_list.read().await;
                    watch_list.keys().cloned().collect::<Vec<_>>()
                };

                if !tokens_to_monitor.is_empty() {
                    if is_debug_pool_prices_enabled() {
                        log(
                            LogTag::Pool,
                            "MONITOR",
                            &format!("Monitoring {} tokens", tokens_to_monitor.len())
                        );
                    }

                    // Update prices for watched tokens
                    for token_address in tokens_to_monitor {
                        if
                            let Err(e) = Self::update_token_price_internal(
                                &pool_cache,
                                &price_cache,
                                &token_address
                            ).await
                        {
                            if is_debug_pool_prices_enabled() {
                                log(
                                    LogTag::Pool,
                                    "ERROR",
                                    &format!("Failed to update price for {}: {}", token_address, e)
                                );
                            }
                        }
                    }
                }
            }

            log(LogTag::Pool, "STOP", "Pool price monitoring service stopped");
        });
    }

    /// Stop background monitoring service
    pub async fn stop_monitoring(&self) {
        let mut monitoring_active = self.monitoring_active.write().await;
        *monitoring_active = false;
        log(LogTag::Pool, "STOP", "Stopping pool price monitoring service");
    }

    /// Add token to watch list
    pub async fn add_to_watch_list(&self, token_address: &str, priority: i32) {
        // Skip system/stable tokens from watch list
        if is_system_or_stable_token(token_address) {
            if is_debug_pool_prices_enabled() {
                log(
                    LogTag::Pool,
                    "SKIP_SYSTEM",
                    &format!("Skipping system/stable token from watch list: {}", token_address)
                );
            }
            return;
        }

        let mut watch_list = self.watch_list.write().await;
        watch_list.insert(token_address.to_string(), WatchListEntry {
            token_address: token_address.to_string(),
            added_at: Utc::now(),
            priority,
            last_price_check: None,
        });

        if is_debug_pool_prices_enabled() {
            log(
                LogTag::Pool,
                "WATCH_ADD",
                &format!("Added {} to watch list (priority: {})", token_address, priority)
            );
        }
    }

    /// Remove token from watch list
    pub async fn remove_from_watch_list(&self, token_address: &str) {
        let mut watch_list = self.watch_list.write().await;
        if watch_list.remove(token_address).is_some() {
            log(LogTag::Pool, "UNWATCH", &format!("Removed {} from watch list", token_address));
        }
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

        // Check price cache first
        {
            let price_cache = self.price_cache.read().await;
            if let Some(cached_price) = price_cache.get(token_address) {
                let age = Utc::now() - cached_price.calculated_at;
                if age.num_seconds() <= PRICE_CACHE_TTL_SECONDS {
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
                } else if is_debug_pool_prices_enabled() {
                    log(
                        LogTag::Pool,
                        "CACHE_EXPIRED",
                        &format!(
                            "❌ CACHE EXPIRED for {}: age={}s > max={}s, will fetch FRESH price from blockchain",
                            token_address,
                            age.num_seconds(),
                            PRICE_CACHE_TTL_SECONDS
                        )
                    );
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

                Some(pool_result)
            }
            Err(e) => {
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
                    best_pool.map(|p| p.liquidity_usd > 1000.0).unwrap_or(false),
                    last_checked: Utc::now(),
                };

                {
                    let mut availability_cache = self.availability_cache.write().await;
                    availability_cache.insert(token_address.to_string(), availability.clone());
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
        let pairs = get_token_pairs_from_api(token_address).await?;
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

        // Create PoolPriceCalculator to get real-time reserves
        let mut calculator = PoolPriceCalculator::new().map_err(|e|
            format!("Failed to create pool calculator: {}", e)
        )?;

        // Calculate price from actual blockchain reserves
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
    ) -> Result<(), String> {
        // This is a simplified version for background updates
        // In a full implementation, this would calculate prices from on-chain data

        // For now, just check if we have cached pools and update timestamp
        let has_cached_pools = {
            let pool_cache = pool_cache.read().await;
            pool_cache.contains_key(token_address)
        };

        if has_cached_pools {
            // Update last check time in watch list entry would go here
            // This is a placeholder for the actual price calculation logic
        }

        Ok(())
    }

    /// Get current watch list
    pub async fn get_watch_list(&self) -> Vec<WatchListEntry> {
        let watch_list = self.watch_list.read().await;
        watch_list.values().cloned().collect()
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize, usize) {
        let pool_cache = self.pool_cache.read().await;
        let price_cache = self.price_cache.read().await;
        let availability_cache = self.availability_cache.read().await;

        (pool_cache.len(), price_cache.len(), availability_cache.len())
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

// =============================================================================
// GLOBAL POOL PRICE SERVICE
// =============================================================================

static mut GLOBAL_POOL_SERVICE: Option<PoolPriceService> = None;
static POOL_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global pool price service
pub fn init_pool_service() -> &'static PoolPriceService {
    unsafe {
        POOL_INIT.call_once(|| {
            GLOBAL_POOL_SERVICE = Some(PoolPriceService::new());
        });
        GLOBAL_POOL_SERVICE.as_ref().unwrap()
    }
}

/// Get global pool price service
pub fn get_pool_service() -> &'static PoolPriceService {
    unsafe {
        if GLOBAL_POOL_SERVICE.is_none() {
            return init_pool_service();
        }
        GLOBAL_POOL_SERVICE.as_ref().unwrap()
    }
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
    rpc_client: Arc<RpcClient>,
    pool_cache: Arc<RwLock<HashMap<String, PoolInfo>>>,
    price_cache: Arc<RwLock<HashMap<String, (f64, Instant)>>>,
    stats: Arc<RwLock<PoolStats>>,
    debug_enabled: bool,
}

impl PoolPriceCalculator {
    /// Create new pool price calculator with default RPC
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Use primary RPC from configs
        let rpc_url = Self::get_rpc_url()?;
        Self::new_with_url(&rpc_url)
    }

    /// Create new pool price calculator with custom RPC URL
    pub fn new_with_url(rpc_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let rpc_client = Arc::new(
            RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed())
        );

        Ok(Self {
            rpc_client,
            pool_cache: Arc::new(RwLock::new(HashMap::new())),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(PoolStats::new())),
            debug_enabled: false,
        })
    }

    /// Create with optional custom RPC URL (for tool usage)
    pub async fn new_with_rpc(
        rpc_url: Option<&String>
    ) -> Result<Self, Box<dyn std::error::Error>> {
        match rpc_url {
            Some(url) => Self::new_with_url(url),
            None => Self::new(),
        }
    }

    /// Get RPC URL from configs
    fn get_rpc_url() -> Result<String, Box<dyn std::error::Error>> {
        // Try to read from configs.json
        if let Ok(config_content) = std::fs::read_to_string("configs.json") {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&config_content) {
                if let Some(rpc_url) = config.get("solana_rpc_url").and_then(|v| v.as_str()) {
                    return Ok(rpc_url.to_string());
                }
            }
        }

        // Fallback to environment variable or default
        Ok(
            std::env
                ::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string())
        )
    }

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

        // Get account data
        let account = self.rpc_client
            .get_account(&pool_pubkey)
            .map_err(|e| format!("Failed to get pool account {}: {}", pool_address, e))?;

        // Determine pool type by owner (program ID)
        let program_id = account.owner.to_string();

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "INFO",
                &format!("Pool {} owned by program {}", pool_address, program_id)
            );
        }

        // Decode based on program ID
        let pool_info = match program_id.as_str() {
            RAYDIUM_CPMM_PROGRAM_ID => {
                self.decode_raydium_cpmm_pool(pool_address, &account).await?
            }
            METEORA_DAMM_V2_PROGRAM_ID => {
                self.decode_meteora_damm_v2_pool(pool_address, &account).await?
            }
            METEORA_DLMM_PROGRAM_ID => {
                self.decode_meteora_dlmm_pool(pool_address, &account).await?
            }
            _ => {
                return Err(format!("Unsupported pool program ID: {}", program_id));
            }
        };

        // Cache the result
        {
            let mut cache = self.pool_cache.write().await;
            cache.insert(pool_address.to_string(), pool_info.clone());
        }

        // Update stats
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
                            token_decimals: 6, // Default assumption
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
            METEORA_DAMM_V2_PROGRAM_ID => {
                self.calculate_meteora_damm_v2_price(&pool_info, token_mint).await?
            }
            METEORA_DLMM_PROGRAM_ID => {
                self.calculate_meteora_dlmm_price(&pool_info, token_mint).await?
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
            .get_multiple_accounts(&pubkeys)
            .map_err(|e| format!("Failed to get multiple accounts: {}", e))?;

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

        match self.rpc_client.get_account(&pool_pubkey) {
            Ok(account) => Ok(Some(account.data)),
            Err(e) => {
                if e.to_string().contains("not found") {
                    Ok(None)
                } else {
                    Err(format!("Failed to fetch account data: {}", e))
                }
            }
        }
    }
}

// =============================================================================
// RAYDIUM CPMM POOL DECODER
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
    async fn get_vault_balances(&self, vault_0: &str, vault_1: &str) -> Result<(u64, u64), String> {
        let vault_0_pubkey = Pubkey::from_str(vault_0).map_err(|e|
            format!("Invalid vault 0 address {}: {}", vault_0, e)
        )?;
        let vault_1_pubkey = Pubkey::from_str(vault_1).map_err(|e|
            format!("Invalid vault 1 address {}: {}", vault_1, e)
        )?;

        let accounts = self.rpc_client
            .get_multiple_accounts(&[vault_0_pubkey, vault_1_pubkey])
            .map_err(|e| format!("Failed to get vault accounts: {}", e))?;

        let vault_0_account = accounts[0]
            .as_ref()
            .ok_or_else(|| "Vault 0 account not found".to_string())?;
        let vault_1_account = accounts[1]
            .as_ref()
            .ok_or_else(|| "Vault 1 account not found".to_string())?;

        let balance_0 = Self::decode_token_account_amount(&vault_0_account.data)?;
        let balance_1 = Self::decode_token_account_amount(&vault_1_account.data)?;

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
        let reserve_0_pubkey = Pubkey::from_str(reserve_0).map_err(|e|
            format!("Invalid reserve 0 address {}: {}", reserve_0, e)
        )?;
        let reserve_1_pubkey = Pubkey::from_str(reserve_1).map_err(|e|
            format!("Invalid reserve 1 address {}: {}", reserve_1, e)
        )?;

        let accounts = self.rpc_client
            .get_multiple_accounts(&[reserve_0_pubkey, reserve_1_pubkey])
            .map_err(|e| format!("Failed to get DLMM reserve accounts: {}", e))?;

        let reserve_0_account = accounts[0]
            .as_ref()
            .ok_or_else(|| format!("DLMM reserve 0 account {} not found", reserve_0))?;
        let reserve_1_account = accounts[1]
            .as_ref()
            .ok_or_else(|| format!("DLMM reserve 1 account {} not found", reserve_1))?;

        // For DLMM, reserves are stored in SPL Token accounts
        let balance_0 = Self::decode_token_account_amount(&reserve_0_account.data)?;
        let balance_1 = Self::decode_token_account_amount(&reserve_1_account.data)?;

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
        let token_x_mint = extract_pubkey_at_offset(data, 88)?;
        let token_y_mint = extract_pubkey_at_offset(data, 120)?;
        let reserve_x = extract_pubkey_at_offset(data, 152)?;
        let reserve_y = extract_pubkey_at_offset(data, 184)?;

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
