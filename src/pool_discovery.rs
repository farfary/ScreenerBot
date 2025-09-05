use crate::logger::{ log, LogTag };
use crate::global::is_debug_pool_discovery_enabled;
use crate::tokens::dexscreener::{
    get_batch_token_pools_from_dexscreener,
    process_dexscreener_batch_results,
    DexScreenerBatchResult,
};
use crate::tokens::geckoterminal::{
    get_batch_token_pools_from_geckoterminal,
    process_geckoterminal_batch_results,
    GeckoTerminalBatchResult,
};
use crate::tokens::raydium::{
    get_batch_token_pools_from_raydium,
    process_raydium_batch_results,
    RaydiumBatchResult,
};
use crate::pool_interface::CachedPoolInfo;
use crate::pool_db::{ DbPoolMetadata, store_pool_metadata_batch };
use chrono::{ DateTime, Utc };
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Maximum number of tokens to process in a single batch
const MAX_BATCH_SIZE: usize = 10;

/// Rate limiting delay between batches (milliseconds)
const BATCH_DELAY_MS: u64 = 1000;

/// Cache TTL for discovered pools (seconds)
const POOL_CACHE_TTL_SECS: i64 = 300; // 5 minutes

/// SOL mint address for filtering TOKEN/SOL pairs
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Pool data for in-memory storage
#[derive(Debug, Clone)]
pub struct PoolData {
    pub pool_address: String,
    pub token_mint: String,
    pub dex_type: String,
    pub reserve_sol: f64,
    pub reserve_token: f64,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub last_updated: DateTime<Utc>,
}

/// Account data for batch fetching
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub address: String,
    pub account_type: String, // "pool", "vault", etc.
    pub token_mint: String,
    pub last_fetched: Option<DateTime<Utc>>,
}

/// Pool discovery statistics
#[derive(Debug, Clone)]
pub struct PoolDiscoveryStats {
    pub total_discoveries: u64,
    pub successful_discoveries: u64,
    pub failed_discoveries: u64,
    pub dexscreener_successful: u64,
    pub geckoterminal_successful: u64,
    pub raydium_successful: u64,
    pub combined_successful: u64,
    pub total_pools_discovered: u64,
    pub average_discovery_time_ms: f64,
    pub last_discovery: Option<DateTime<Utc>>,
}

impl Default for PoolDiscoveryStats {
    fn default() -> Self {
        Self {
            total_discoveries: 0,
            successful_discoveries: 0,
            failed_discoveries: 0,
            dexscreener_successful: 0,
            geckoterminal_successful: 0,
            raydium_successful: 0,
            combined_successful: 0,
            total_pools_discovered: 0,
            average_discovery_time_ms: 0.0,
            last_discovery: None,
        }
    }
}

impl PoolDiscoveryStats {
    pub fn get_success_rate(&self) -> f64 {
        if self.total_discoveries == 0 {
            0.0
        } else {
            (self.successful_discoveries as f64 / self.total_discoveries as f64) * 100.0
        }
    }

    pub fn record_discovery(&mut self, success: bool, time_ms: f64, pools_found: usize) {
        self.total_discoveries += 1;
        if success {
            self.successful_discoveries += 1;
            self.total_pools_discovered += pools_found as u64;
        } else {
            self.failed_discoveries += 1;
        }
        
        // Update average time
        let total_time = self.average_discovery_time_ms * (self.total_discoveries - 1) as f64 + time_ms;
        self.average_discovery_time_ms = total_time / self.total_discoveries as f64;
        
        self.last_discovery = Some(Utc::now());
    }
}

/// Pool discovery service
pub struct PoolDiscoveryService {
    stats: Arc<RwLock<PoolDiscoveryStats>>,
    discovered_pools: Arc<RwLock<HashMap<String, Vec<CachedPoolInfo>>>>,
    debug_enabled: bool,
}

// =============================================================================
// IMPLEMENTATIONS
// =============================================================================

impl PoolDiscoveryService {
    /// Create new pool discovery service
    pub fn new() -> Self {
        let debug_enabled = is_debug_pool_discovery_enabled();

        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Pool discovery service debug mode enabled");
        }

        Self {
            stats: Arc::new(RwLock::new(PoolDiscoveryStats::default())),
            discovered_pools: Arc::new(RwLock::new(HashMap::new())),
            debug_enabled,
        }
    }

    /// Enable debug mode
    pub fn enable_debug(&mut self) {
        self.debug_enabled = true;
        log(LogTag::Pool, "DEBUG", "Pool discovery service debug mode enabled (overridden)");
    }

    /// Discover pools for multiple tokens using triple API approach
    pub async fn discover_pools_batch(
        &self,
        token_addresses: &[String]
    ) -> Result<HashMap<String, Vec<CachedPoolInfo>>, String> {
        if token_addresses.is_empty() {
            return Ok(HashMap::new());
        }

        let start_time = Instant::now();

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "DISCOVERY_BATCH_START",
                &format!(
                    "🚀 Starting pool discovery for {} tokens",
                    token_addresses.len()
                )
            );
        }

        let mut dexscreener_successful = 0;
        let mut geckoterminal_successful = 0;
        let mut raydium_successful = 0;
        let mut combined_successful = 0;
        let mut total_pools_found = 0;

        // Split tokens into batches for optimal performance
        for chunk in token_addresses.chunks(MAX_BATCH_SIZE) {
            let chunk_start = Instant::now();

            // Convert chunk to Vec<String> for API calls
            let chunk_vec: Vec<String> = chunk.iter().cloned().collect();

            // Launch ALL THREE API calls concurrently for maximum speed
            let dexscreener_future = get_batch_token_pools_from_dexscreener(&chunk_vec);
            let gecko_future = get_batch_token_pools_from_geckoterminal(&chunk_vec);
            let raydium_future = get_batch_token_pools_from_raydium(&chunk_vec);

            // Wait for ALL THREE APIs to complete concurrently
            let (dexscreener_result, gecko_result, raydium_result) = tokio::join!(
                dexscreener_future,
                gecko_future,
                raydium_future
            );

            let mut dx_success = 0;
            let mut gt_success = 0;
            let mut ray_success = 0;

            // Process and combine results from all three APIs with deduplication
            let mut token_pools_map: HashMap<String, Vec<CachedPoolInfo>> = HashMap::new();

            // Process DexScreener results
            let processed_dexscreener_pools = process_dexscreener_batch_results(&dexscreener_result);
            for (token_address, cached_pools) in processed_dexscreener_pools {
                if !cached_pools.is_empty() {
                    dx_success += 1;
                    token_pools_map
                        .entry(token_address)
                        .or_insert_with(Vec::new)
                        .extend(cached_pools);
                }
            }
            dexscreener_successful += dx_success;

            // Process GeckoTerminal results
            let processed_gecko_pools = process_geckoterminal_batch_results(&gecko_result);
            for (token_address, cached_pools) in processed_gecko_pools {
                if !cached_pools.is_empty() {
                    gt_success += 1;
                    token_pools_map
                        .entry(token_address)
                        .or_insert_with(Vec::new)
                        .extend(cached_pools);
                }
            }
            geckoterminal_successful += gt_success;

            // Process Raydium results
            let processed_raydium_pools = process_raydium_batch_results(&raydium_result);
            for (token_address, cached_pools) in processed_raydium_pools {
                if !cached_pools.is_empty() {
                    ray_success += 1;
                    token_pools_map
                        .entry(token_address)
                        .or_insert_with(Vec::new)
                        .extend(cached_pools);
                }
            }
            raydium_successful += ray_success;

            // Filter for TOKEN/SOL pairs only, deduplicate, store to DB, and cache results
            {
                let mut discovered_pools = self.discovered_pools.write().await;

                for (token_address, all_pools) in token_pools_map {
                    if all_pools.is_empty() { continue; }

                    // Keep only pools where the pair includes SOL on one side
                    let filtered: Vec<CachedPoolInfo> = all_pools
                        .into_iter()
                        .filter(|p| p.base_token == token_address && p.quote_token == SOL_MINT
                            || p.quote_token == token_address && p.base_token == SOL_MINT)
                        .collect();

                    if filtered.is_empty() { continue; }

                    // Deduplicate pools (keep highest liquidity per pool address)
                    let deduplicated_pools = self.deduplicate_pools(filtered);
                    total_pools_found += deduplicated_pools.len();

                    // Store to database in a batch
                    if let Err(e) = self.store_pools_to_database(&token_address, &deduplicated_pools).await {
                        log(
                            LogTag::Pool,
                            "DISCOVERY_DB_ERROR",
                            &format!(
                                "Failed to store {} pools for {}: {}",
                                deduplicated_pools.len(),
                                &token_address[..8],
                                e
                            )
                        );
                    }

                    // Cache discovered pools in memory
                    discovered_pools.insert(token_address, deduplicated_pools);
                    combined_successful += 1;
                }
            }

            if self.debug_enabled {
                log(
                    LogTag::Pool,
                    "DISCOVERY_BATCH_CHUNK",
                    &format!(
                        "🚀 Processed {} tokens in {}ms: DX {}, GT {}, Ray {}, Combined {}",
                        chunk.len(),
                        chunk_start.elapsed().as_millis(),
                        dx_success,
                        gt_success,
                        ray_success,
                        combined_successful
                    )
                );
            }

            // Rate limiting between chunks
            tokio::time::sleep(Duration::from_millis(BATCH_DELAY_MS)).await;
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.dexscreener_successful += dexscreener_successful;
            stats.geckoterminal_successful += geckoterminal_successful;
            stats.raydium_successful += raydium_successful;
            stats.combined_successful += combined_successful;
            stats.record_discovery(
                combined_successful > 0,
                start_time.elapsed().as_millis() as f64,
                total_pools_found
            );
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "DISCOVERY_BATCH_COMPLETE",
                &format!(
                    "🚀 Pool discovery complete: DX {}/{}, GT {}/{}, Ray {}/{}, Combined {}/{}, Total pools: {}",
                    dexscreener_successful,
                    token_addresses.len(),
                    geckoterminal_successful,
                    token_addresses.len(),
                    raydium_successful,
                    token_addresses.len(),
                    combined_successful,
                    token_addresses.len(),
                    total_pools_found
                )
            );
        }

        // Return discovered pools
        Ok(self.discovered_pools.read().await.clone())
    }

    /// Discover pools for a single token
    pub async fn discover_pools_for_token(
        &self,
        token_address: &str
    ) -> Result<Vec<CachedPoolInfo>, String> {
        let result = self.discover_pools_batch(&[token_address.to_string()]).await?;
        Ok(result.get(token_address).cloned().unwrap_or_default())
    }

    /// Get discovered pools for a token (from cache)
    pub async fn get_discovered_pools(&self, token_address: &str) -> Option<Vec<CachedPoolInfo>> {
        self.discovered_pools.read().await.get(token_address).cloned()
    }

    /// Clear discovered pools for a token
    pub async fn clear_discovered_pools(&self, token_address: &str) {
        self.discovered_pools.write().await.remove(token_address);
    }

    /// Clear all discovered pools
    pub async fn clear_all_discovered_pools(&self) {
        self.discovered_pools.write().await.clear();
    }

    /// Get statistics
    pub async fn get_stats(&self) -> PoolDiscoveryStats {
        self.stats.read().await.clone()
    }

    /// Get cache size
    pub async fn get_cache_size(&self) -> usize {
        self.discovered_pools.read().await.len()
    }

    /// Discover pools and process them for pool service integration
    pub async fn discover_and_process_pools(
        &self,
        token_addresses: &[String]
    ) -> Result<(HashMap<String, PoolData>, Vec<AccountInfo>), String> {
        if token_addresses.is_empty() {
            return Ok((HashMap::new(), Vec::new()));
        }

        log(
            LogTag::Pool,
            "POOL_DISCOVERY_START",
            &format!("Starting pool discovery for {} tokens", token_addresses.len())
        );

        // Use the pool discovery service to discover pools
        let discovered_pools = self.discover_pools_batch(token_addresses).await?;

        let mut best_pools = HashMap::new();
        let mut account_queue = Vec::new();
        let mut discovered_count = 0;
        let now = Utc::now();

        // Process discovered pools and create PoolData and AccountInfo
        for (token_mint, cached_pools) in discovered_pools {
            if cached_pools.is_empty() {
                continue;
            }

            // Find the best pool (highest liquidity)
            let best_pool = cached_pools.iter()
                .max_by(|a, b| a.liquidity_usd.partial_cmp(&b.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal));

            if let Some(best_pool) = best_pool {
                // Convert CachedPoolInfo to PoolData
                let pool_data = PoolData {
                    pool_address: best_pool.pair_address.clone(),
                    token_mint: token_mint.clone(),
                    dex_type: best_pool.dex_id.clone(),
                    reserve_sol: 0.0, // Will be fetched from on-chain data
                    reserve_token: 0.0, // Will be fetched from on-chain data
                    liquidity_usd: best_pool.liquidity_usd,
                    volume_24h: best_pool.volume_24h,
                    last_updated: now,
                };

                best_pools.insert(token_mint.clone(), pool_data);

                // Add to account queue for on-chain data fetching
                let account_info = AccountInfo {
                    address: best_pool.pair_address.clone(),
                    account_type: "pool".to_string(),
                    token_mint: token_mint.clone(),
                    last_fetched: None,
                };

                account_queue.push(account_info);
                discovered_count += 1;

                log(
                    LogTag::Pool,
                    "POOL_DISCOVERED",
                    &format!(
                        "Discovered pool {} for token {} (liquidity: ${:.2})",
                        best_pool.pair_address,
                        token_mint,
                        best_pool.liquidity_usd
                    )
                );
            }
        }

        log(
            LogTag::Pool,
            "POOL_DISCOVERY_COMPLETE",
            &format!("Pool discovery completed: {} pools discovered", discovered_count)
        );

        Ok((best_pools, account_queue))
    }

    // =============================================================================
    // PRIVATE METHODS
    // =============================================================================

    /// Deduplicate pools by pool address, keeping the one with highest liquidity
    fn deduplicate_pools(&self, pools: Vec<CachedPoolInfo>) -> Vec<CachedPoolInfo> {
        use std::collections::HashMap;

        let mut by_address: HashMap<String, CachedPoolInfo> = HashMap::new();
        for pool in pools {
            match by_address.get(&pool.pair_address) {
                Some(existing) => {
                    if pool.liquidity_usd > existing.liquidity_usd {
                        by_address.insert(pool.pair_address.clone(), pool);
                    }
                }
                None => {
                    by_address.insert(pool.pair_address.clone(), pool);
                }
            }
        }

        let mut deduped: Vec<CachedPoolInfo> = by_address.into_values().collect();
        deduped.sort_by(|a, b| b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal));
        deduped
    }

    /// Store deduplicated pools to database for persistence
    async fn store_pools_to_database(&self, token_address: &str, cached_pools: &[CachedPoolInfo]) -> Result<(), String> {
        if cached_pools.is_empty() { return Ok(()); }

        // Helper to map dex_id prefix to source string
        fn source_from_dex_id(dex_id: &str) -> &'static str {
            if dex_id.starts_with("gecko_") { "geckoterminal" }
            else if dex_id.starts_with("ray_") { "raydium" }
            else { "dexscreener" }
        }

        let mut batch: Vec<DbPoolMetadata> = Vec::with_capacity(cached_pools.len());
        for pool in cached_pools {
            let source = source_from_dex_id(&pool.dex_id);
            let mut entry = DbPoolMetadata::new(
                token_address,
                &pool.pair_address,
                &pool.dex_id,
                "solana",
                source,
            );

            // Fill known fields
            entry.quote_token_address = pool.quote_token.clone();
            entry.price_native = Some(pool.price_native);
            entry.price_usd = Some(pool.price_usd);
            entry.liquidity_usd = Some(pool.liquidity_usd);
            entry.volume_24h = Some(pool.volume_24h);

            if pool.created_at > 0 {
                if let Some(dt) = chrono::DateTime::from_timestamp(pool.created_at as i64, 0) {
                    entry.pair_created_at = Some(dt.with_timezone(&Utc));
                }
            }

            batch.push(entry);
        }

        // Store in a single batch transaction
        store_pool_metadata_batch(&batch)
    }
}

// =============================================================================
// GLOBAL INSTANCE MANAGEMENT
// =============================================================================

static GLOBAL_POOL_DISCOVERY: std::sync::OnceLock<PoolDiscoveryService> = std::sync::OnceLock::new();

/// Initialize the global pool discovery service
pub fn init_pool_discovery() -> &'static PoolDiscoveryService {
    GLOBAL_POOL_DISCOVERY.get_or_init(|| {
        log(LogTag::Pool, "INIT", "Initializing global pool discovery service");
        PoolDiscoveryService::new()
    })
}

/// Get the global pool discovery service
pub fn get_pool_discovery() -> &'static PoolDiscoveryService {
    GLOBAL_POOL_DISCOVERY.get().expect("Pool discovery service not initialized")
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Discover pools for multiple tokens (convenience function)
pub async fn discover_pools_batch(token_addresses: &[String]) -> Result<HashMap<String, Vec<CachedPoolInfo>>, String> {
    get_pool_discovery().discover_pools_batch(token_addresses).await
}

/// Discover pools for a single token (convenience function)
pub async fn discover_pools_for_token(token_address: &str) -> Result<Vec<CachedPoolInfo>, String> {
    get_pool_discovery().discover_pools_for_token(token_address).await
}

/// Get discovered pools for a token (convenience function)
pub async fn get_discovered_pools(token_address: &str) -> Option<Vec<CachedPoolInfo>> {
    get_pool_discovery().get_discovered_pools(token_address).await
}

/// Clear discovered pools for a token (convenience function)
pub async fn clear_discovered_pools(token_address: &str) {
    get_pool_discovery().clear_discovered_pools(token_address).await;
}

/// Clear all discovered pools (convenience function)
pub async fn clear_all_discovered_pools() {
    get_pool_discovery().clear_all_discovered_pools().await;
}

/// Get pool discovery statistics (convenience function)
pub async fn get_pool_discovery_stats() -> PoolDiscoveryStats {
    get_pool_discovery().get_stats().await
}

/// Get pool discovery cache size (convenience function)
pub async fn get_pool_discovery_cache_size() -> usize {
    get_pool_discovery().get_cache_size().await
}

/// Discover and process pools for pool service integration (convenience function)
pub async fn discover_and_process_pools(token_addresses: &[String]) -> Result<(HashMap<String, PoolData>, Vec<AccountInfo>), String> {
    get_pool_discovery().discover_and_process_pools(token_addresses).await
}
