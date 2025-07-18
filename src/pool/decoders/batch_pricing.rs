use anyhow::{ anyhow, Result };
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::rpc::RpcManager;
use crate::pool::decoders::legacy_pools::UniversalPoolDecoder;

/// Batch pricing configuration
#[derive(Debug, Clone)]
pub struct BatchPricingConfig {
    pub batch_size: usize,
    pub max_concurrent_batches: usize,
    pub cache_duration_seconds: u64,
    pub use_fast_tier: bool,
    pub use_discovery_tier: bool,
}

impl Default for BatchPricingConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            max_concurrent_batches: 4,
            cache_duration_seconds: 30,
            use_fast_tier: true,
            use_discovery_tier: true,
        }
    }
}

/// Pool price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPrice {
    pub pool_address: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub base_reserves: u64,
    pub quote_reserves: u64,
    pub price: f64,
    pub timestamp: DateTime<Utc>,
    pub slot: u64,
}

/// Cached pool price with expiration
#[derive(Debug, Clone)]
struct CachedPoolPrice {
    price: PoolPrice,
    expires_at: DateTime<Utc>,
}

/// Pool account information for batch processing
#[derive(Debug, Clone)]
pub struct PoolAccount {
    pub pool_address: String,
    pub program_id: Pubkey,
    pub account_data: Vec<u8>,
    pub slot: u64,
}

/// Batch pricing result
#[derive(Debug, Clone)]
pub struct BatchPricingResult {
    pub successful_prices: Vec<PoolPrice>,
    pub failed_pools: Vec<String>,
    pub total_processed: usize,
    pub processing_time_ms: u64,
}

/// Advanced batch pricing system that combines old and new approaches
pub struct BatchPricingManager {
    rpc_manager: Arc<RpcManager>,
    universal_decoder: UniversalPoolDecoder,
    price_cache: Arc<RwLock<HashMap<String, CachedPoolPrice>>>,
    config: BatchPricingConfig,
}

impl BatchPricingManager {
    pub fn new(rpc_manager: Arc<RpcManager>, config: BatchPricingConfig) -> Self {
        let universal_decoder = UniversalPoolDecoder::new(rpc_manager.clone());

        Self {
            rpc_manager,
            universal_decoder,
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Get prices for multiple pools using batch processing
    pub async fn get_batch_prices(&self, pool_addresses: &[String]) -> Result<BatchPricingResult> {
        let start_time = std::time::Instant::now();

        // Check cache first
        let mut cached_prices = Vec::new();
        let mut uncached_pools = Vec::new();

        {
            let cache = self.price_cache.read().await;
            let now = Utc::now();

            for pool_address in pool_addresses {
                if let Some(cached) = cache.get(pool_address) {
                    if cached.expires_at > now {
                        cached_prices.push(cached.price.clone());
                        continue;
                    }
                }
                uncached_pools.push(pool_address.clone());
            }
        }

        // Fetch fresh prices for uncached pools
        let mut fresh_prices = Vec::new();
        let mut failed_pools = Vec::new();

        if !uncached_pools.is_empty() {
            // Use dual-tier approach from old system
            if self.config.use_fast_tier {
                match self.batch_prices_fast_tier(&uncached_pools).await {
                    Ok(fast_results) => {
                        fresh_prices.extend(fast_results.successful_prices);
                        failed_pools.extend(fast_results.failed_pools);
                    }
                    Err(e) => {
                        println!("Fast tier failed: {}", e);
                        failed_pools.extend(uncached_pools.clone());
                    }
                }
            }

            // Use discovery tier for failed pools if enabled
            if self.config.use_discovery_tier && !failed_pools.is_empty() {
                let discovery_pools = failed_pools.clone();
                failed_pools.clear();

                match self.batch_prices_discovery_tier(&discovery_pools).await {
                    Ok(discovery_results) => {
                        fresh_prices.extend(discovery_results.successful_prices);
                        failed_pools.extend(discovery_results.failed_pools);
                    }
                    Err(e) => {
                        println!("Discovery tier failed: {}", e);
                        failed_pools.extend(discovery_pools);
                    }
                }
            }
        }

        // Update cache with fresh prices
        if !fresh_prices.is_empty() {
            let mut cache = self.price_cache.write().await;
            let expires_at =
                Utc::now() + chrono::Duration::seconds(self.config.cache_duration_seconds as i64);

            for price in &fresh_prices {
                cache.insert(price.pool_address.clone(), CachedPoolPrice {
                    price: price.clone(),
                    expires_at,
                });
            }
        }

        // Combine results
        let mut all_prices = cached_prices;
        all_prices.extend(fresh_prices);

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(BatchPricingResult {
            successful_prices: all_prices,
            failed_pools,
            total_processed: pool_addresses.len(),
            processing_time_ms,
        })
    }

    /// Fast tier: Use getMultipleAccounts and parallel processing
    async fn batch_prices_fast_tier(
        &self,
        pool_addresses: &[String]
    ) -> Result<BatchPricingResult> {
        let start_time = std::time::Instant::now();

        // Convert addresses to Pubkeys
        let pubkeys: Result<Vec<Pubkey>, _> = pool_addresses
            .iter()
            .map(|addr| addr.parse::<Pubkey>())
            .collect();
        let pubkeys = pubkeys?;

        // Get multiple accounts in batches
        let mut successful_prices = Vec::new();
        let mut failed_pools = Vec::new();

        for chunk in pubkeys.chunks(self.config.batch_size) {
            match self.rpc_manager.get_multiple_accounts(chunk).await {
                Ok(accounts) => {
                    // Process accounts in parallel
                    let tasks: Vec<_> = accounts
                        .into_iter()
                        .zip(chunk.iter())
                        .zip(pool_addresses.iter())
                        .map(|((account_opt, pubkey), pool_address)| {
                            let pool_address = pool_address.clone();
                            let decoder = &self.universal_decoder;

                            async move {
                                match account_opt {
                                    Some(account) => {
                                        // Create a dummy owner for now - in a real implementation,
                                        // we'd need to get the account owner from the account
                                        let owner = account.owner;

                                        match
                                            decoder.decode_any_pool_price(
                                                &account.data,
                                                &owner,
                                                pubkey
                                            )
                                        {
                                            Ok((base_reserves, quote_reserves, price)) => {
                                                // Get mints for full price info
                                                match
                                                    decoder.decode_any_pool(
                                                        &account.data,
                                                        &owner,
                                                        pubkey
                                                    )
                                                {
                                                    Ok((_, _, base_mint, quote_mint)) => {
                                                        Some(PoolPrice {
                                                            pool_address,
                                                            base_mint: base_mint.to_string(),
                                                            quote_mint: quote_mint.to_string(),
                                                            base_reserves,
                                                            quote_reserves,
                                                            price,
                                                            timestamp: Utc::now(),
                                                            slot: 0, // We don't have slot info in current Account structure
                                                        })
                                                    }
                                                    Err(_) => None,
                                                }
                                            }
                                            Err(_) => None,
                                        }
                                    }
                                    None => None,
                                }
                            }
                        })
                        .collect();

                    // Execute tasks concurrently
                    let results = futures::future::join_all(tasks).await;

                    for (i, result) in results.into_iter().enumerate() {
                        let pool_idx = successful_prices.len() + failed_pools.len();
                        match result {
                            Some(price) => successful_prices.push(price),
                            None => failed_pools.push(pool_addresses[pool_idx].clone()),
                        }
                    }
                }
                Err(_) => {
                    // If batch fails, mark all as failed
                    let start_idx = successful_prices.len() + failed_pools.len();
                    failed_pools.extend(
                        chunk
                            .iter()
                            .enumerate()
                            .map(|(i, _)| pool_addresses[start_idx + i].clone())
                    );
                }
            }
        }

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(BatchPricingResult {
            successful_prices,
            failed_pools,
            total_processed: pool_addresses.len(),
            processing_time_ms,
        })
    }

    /// Discovery tier: Individual account fetching with retry logic
    async fn batch_prices_discovery_tier(
        &self,
        pool_addresses: &[String]
    ) -> Result<BatchPricingResult> {
        let start_time = std::time::Instant::now();

        let mut successful_prices = Vec::new();
        let mut failed_pools = Vec::new();

        // Process pools individually with retry logic
        for pool_address in pool_addresses {
            match self.get_single_pool_price(pool_address).await {
                Ok(price) => successful_prices.push(price),
                Err(_) => failed_pools.push(pool_address.clone()),
            }
        }

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(BatchPricingResult {
            successful_prices,
            failed_pools,
            total_processed: pool_addresses.len(),
            processing_time_ms,
        })
    }

    /// Get price for a single pool with retry logic
    async fn get_single_pool_price(&self, pool_address: &str) -> Result<PoolPrice> {
        let pubkey = pool_address.parse::<Pubkey>()?;

        // Try to get account data with retry
        let account = self.rpc_manager.get_account(&pubkey).await?;

        // Decode using universal decoder
        let (base_reserves, quote_reserves, price) = self.universal_decoder.decode_any_pool_price(
            &account.data,
            &account.owner,
            &pubkey
        )?;

        // Get mints for full price info
        let (_, _, base_mint, quote_mint) = self.universal_decoder.decode_any_pool(
            &account.data,
            &account.owner,
            &pubkey
        )?;

        Ok(PoolPrice {
            pool_address: pool_address.to_string(),
            base_mint: base_mint.to_string(),
            quote_mint: quote_mint.to_string(),
            base_reserves,
            quote_reserves,
            price,
            timestamp: Utc::now(),
            slot: 0, // We don't have slot info in current Account structure
        })
    }

    /// Clear expired cache entries
    pub async fn cleanup_cache(&self) {
        let mut cache = self.price_cache.write().await;
        let now = Utc::now();

        cache.retain(|_, cached| cached.expires_at > now);
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> HashMap<String, u64> {
        let cache = self.price_cache.read().await;
        let now = Utc::now();

        let total_entries = cache.len() as u64;
        let expired_entries = cache
            .values()
            .filter(|cached| cached.expires_at <= now)
            .count() as u64;

        HashMap::from([
            ("total_entries".to_string(), total_entries),
            ("expired_entries".to_string(), expired_entries),
            ("valid_entries".to_string(), total_entries - expired_entries),
        ])
    }
}
