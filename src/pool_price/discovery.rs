/// Pool Discovery Module
///
/// This module handles discovering pool addresses for tokens using the DexScreener API.
/// It caches pool addresses (not pool data) for 5 minutes to reduce API calls.

use super::types::*;
use crate::logger::{ log, LogTag };

use reqwest::Client;
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use std::time::{ SystemTime, UNIX_EPOCH };
use tokio::sync::Semaphore;
use tokio::time::{ timeout, Duration };

// =============================================================================
// POOL DISCOVERY MANAGER
// =============================================================================

pub struct PoolDiscovery {
    http_client: Client,
    address_cache: Arc<Mutex<HashMap<String, PoolAddressEntry>>>,
    rate_limiter: Arc<Semaphore>,
}

impl PoolDiscovery {
    /// Create new pool discovery instance
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
            address_cache: Arc::new(Mutex::new(HashMap::new())),
            rate_limiter: Arc::new(Semaphore::new(DEXSCREENER_RATE_LIMIT_PER_MINUTE as usize)),
        }
    }

    /// Get pool addresses for a token (cached for 5 minutes)
    pub async fn get_pool_addresses(&self, mint: &str) -> PoolPriceResult<Vec<PoolAddressInfo>> {
        // Check cache first
        if let Some(cached) = self.get_cached_addresses(mint) {
            log(LogTag::Pool, "CACHE", &format!("Using cached pool addresses for {}", mint));
            return Ok(cached.pool_addresses);
        }

        // Acquire rate limit permit
        let _permit = timeout(Duration::from_secs(10), self.rate_limiter.acquire()).await
            .map_err(|_|
                PoolPriceError::RateLimit("Timeout waiting for rate limit permit".to_string())
            )?
            .map_err(|_|
                PoolPriceError::RateLimit("Failed to acquire rate limit permit".to_string())
            )?;

        log(LogTag::Pool, "API", &format!("Fetching pool addresses from DexScreener for {}", mint));

        // Fetch from DexScreener API
        let pool_addresses = self.fetch_pools_from_dexscreener(mint).await?;

        // Cache the results
        self.cache_pool_addresses(mint, &pool_addresses);

        Ok(pool_addresses)
    }

    /// Fetch pool addresses from DexScreener API
    async fn fetch_pools_from_dexscreener(
        &self,
        mint: &str
    ) -> PoolPriceResult<Vec<PoolAddressInfo>> {
        let url = format!("{}/{}", DEXSCREENER_API_BASE, mint);

        log(LogTag::Pool, "DEBUG", &format!("DexScreener API call: {}", url));

        let response = timeout(Duration::from_secs(10), self.http_client.get(&url).send()).await
            .map_err(|_| PoolPriceError::Network("Request timeout".to_string()))?
            .map_err(|e| PoolPriceError::Network(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(
                PoolPriceError::DexScreenerApi(
                    format!("API request failed with status: {}", response.status())
                )
            );
        }

        let api_response: DexScreenerResponse = response
            .json().await
            .map_err(|e| PoolPriceError::DexScreenerApi(format!("Failed to parse JSON: {}", e)))?;

        // Process and sort pools by liquidity
        let mut pool_addresses = Vec::new();

        for pair in api_response.pairs {
            // Skip pairs that don't have our target token
            let is_base_token = pair.base_token.address == mint;
            let is_quote_token = pair.quote_token.address == mint;

            if !is_base_token && !is_quote_token {
                continue;
            }

            // Extract liquidity value
            let liquidity_usd = pair.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            // Skip pools with very low liquidity
            if liquidity_usd < MIN_LIQUIDITY_USD {
                log(
                    LogTag::Pool,
                    "DEBUG",
                    &format!(
                        "Skipping low liquidity pool {} (${:.2})",
                        pair.pair_address,
                        liquidity_usd
                    )
                );
                continue;
            }

            // Classify pool by DEX and extract program ID
            let (program_id, dex_name) = self.classify_pool_from_dex_id(&pair.dex_id);

            pool_addresses.push(PoolAddressInfo {
                address: pair.pair_address.clone(),
                program_id,
                dex_name,
                liquidity_usd,
                pair_address: pair.pair_address,
            });
        }

        // Sort by liquidity (highest first)
        pool_addresses.sort_by(|a, b|
            b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
        );

        log(
            LogTag::Pool,
            "SUCCESS",
            &format!(
                "Found {} pools for {} (total liquidity: ${:.2})",
                pool_addresses.len(),
                mint,
                pool_addresses
                    .iter()
                    .map(|p| p.liquidity_usd)
                    .sum::<f64>()
            )
        );

        Ok(pool_addresses)
    }

    /// Classify pool and get program ID from DexScreener dex_id
    fn classify_pool_from_dex_id(&self, dex_id: &str) -> (String, String) {
        match dex_id.to_lowercase().as_str() {
            "raydium" => (RAYDIUM_AMM_PROGRAM_ID.to_string(), "Raydium".to_string()),
            "orca" => (ORCA_PROGRAM_ID.to_string(), "Orca".to_string()),
            "meteora" => (METEORA_DLMM_PROGRAM_ID.to_string(), "Meteora".to_string()),
            "pump" | "pumpfun" => (PUMPFUN_PROGRAM_ID.to_string(), "PumpFun".to_string()),
            "jupiter" => (JUPITER_PROGRAM_ID.to_string(), "Jupiter".to_string()),
            _ => {
                log(
                    LogTag::Pool,
                    "WARN",
                    &format!("Unknown DEX ID: {}, defaulting to Raydium", dex_id)
                );
                (RAYDIUM_AMM_PROGRAM_ID.to_string(), "Unknown".to_string())
            }
        }
    }

    /// Check cache for pool addresses
    fn get_cached_addresses(&self, mint: &str) -> Option<PoolAddressEntry> {
        let cache = self.address_cache.lock().ok()?;
        let entry = cache.get(mint)?;

        // Check if entry is still valid (TTL)
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();

        if current_time - entry.timestamp < entry.ttl_secs {
            Some(entry.clone())
        } else {
            log(LogTag::Pool, "DEBUG", &format!("Cache entry expired for {}", mint));
            None
        }
    }

    /// Cache pool addresses for a token
    fn cache_pool_addresses(&self, mint: &str, pool_addresses: &[PoolAddressInfo]) {
        if let Ok(mut cache) = self.address_cache.lock() {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let entry = PoolAddressEntry {
                mint: mint.to_string(),
                pool_addresses: pool_addresses.to_vec(),
                timestamp: current_time,
                ttl_secs: POOL_ADDRESS_CACHE_TTL_SECS,
            };

            cache.insert(mint.to_string(), entry);

            log(
                LogTag::Pool,
                "CACHE",
                &format!(
                    "Cached {} pool addresses for {} (TTL: {}s)",
                    pool_addresses.len(),
                    mint,
                    POOL_ADDRESS_CACHE_TTL_SECS
                )
            );
        }
    }

    /// Clean up expired cache entries
    pub fn cleanup_expired_cache(&self) {
        if let Ok(mut cache) = self.address_cache.lock() {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let initial_count = cache.len();
            cache.retain(|_, entry| current_time - entry.timestamp < entry.ttl_secs);
            let final_count = cache.len();

            if initial_count != final_count {
                log(
                    LogTag::Pool,
                    "CACHE",
                    &format!(
                        "Cleaned up {} expired cache entries ({} -> {})",
                        initial_count - final_count,
                        initial_count,
                        final_count
                    )
                );
            }
        }
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        if let Ok(cache) = self.address_cache.lock() {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let total_entries = cache.len();
            let valid_entries = cache
                .values()
                .filter(|entry| current_time - entry.timestamp < entry.ttl_secs)
                .count();

            (total_entries, valid_entries)
        } else {
            (0, 0)
        }
    }

    /// Preload pool addresses for multiple tokens (batch operation)
    pub async fn preload_pool_addresses(&self, mints: &[String]) -> PoolPriceResult<()> {
        log(LogTag::Pool, "INFO", &format!("Preloading pool addresses for {} tokens", mints.len()));

        for mint in mints {
            // Skip if already cached
            if self.get_cached_addresses(mint).is_some() {
                continue;
            }

            // Fetch pool addresses (this will cache them)
            match self.get_pool_addresses(mint).await {
                Ok(pools) => {
                    log(
                        LogTag::Pool,
                        "DEBUG",
                        &format!("Preloaded {} pool addresses for {}", pools.len(), mint)
                    );
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "WARN",
                        &format!("Failed to preload pool addresses for {}: {}", mint, e)
                    );
                }
            }

            // Small delay to respect rate limits
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }
}

// =============================================================================
// GLOBAL POOL DISCOVERY INSTANCE
// =============================================================================

use once_cell::sync::Lazy;

/// Global pool discovery instance
pub static POOL_DISCOVERY: Lazy<PoolDiscovery> = Lazy::new(|| PoolDiscovery::new());

/// Convenience function to get pool addresses for a token
pub async fn get_pool_addresses_for_token(mint: &str) -> PoolPriceResult<Vec<PoolAddressInfo>> {
    POOL_DISCOVERY.get_pool_addresses(mint).await
}

/// Convenience function to preload pool addresses for multiple tokens
pub async fn preload_pools_for_tokens(mints: &[String]) -> PoolPriceResult<()> {
    POOL_DISCOVERY.preload_pool_addresses(mints).await
}

/// Convenience function to cleanup expired cache entries
pub fn cleanup_pool_cache() {
    POOL_DISCOVERY.cleanup_expired_cache();
}

/// Convenience function to get cache statistics
pub fn get_pool_cache_stats() -> (usize, usize) {
    POOL_DISCOVERY.get_cache_stats()
}
