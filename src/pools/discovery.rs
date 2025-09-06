/// Pool discovery service
/// Finds pools for tokens from external APIs (DexScreener, GeckoTerminal, Raydium)

use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{ sleep, Duration };
use tokio::sync::RwLock;
use crate::pools::cache::PoolCache;
use crate::pools::tokens::PoolTokenManager;
use crate::logger::{ log, LogTag };

/// Pool discovery result
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pool_address: String,
    pub program_id: String,
    pub token_mint: String,
    pub sol_reserve: f64,
    pub token_reserve: f64,
    pub liquidity_usd: f64,
}

/// Pool discovery service
pub struct PoolDiscovery {
    cache: Arc<PoolCache>,
    token_manager: PoolTokenManager,
    is_running: Arc<RwLock<bool>>,
}

impl PoolDiscovery {
    pub fn new(cache: Arc<PoolCache>) -> Self {
        Self {
            cache,
            token_manager: PoolTokenManager::new(),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start continuous discovery task
    pub async fn start_discovery_task(&self) {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            log(LogTag::Pool, "DISCOVERY", "Discovery task already running");
            return;
        }
        *is_running = true;
        drop(is_running);

        log(LogTag::Pool, "DISCOVERY_START", "🚀 Starting continuous pool discovery task");

        // Load top 100 tokens initially
        match self.token_manager.get_top_liquidity_tokens().await {
            Ok(tokens) => {
                log(
                    LogTag::Pool,
                    "TOKENS_LOADED",
                    &format!("📊 Loaded {} top liquidity tokens", tokens.len())
                );
                self.cache.cache_tokens(tokens).await;
            }
            Err(e) => {
                log(LogTag::Pool, "TOKENS_ERROR", &format!("Failed to load tokens: {}", e));
            }
        }

        // Clone necessary data for the background task
        let cache = self.cache.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut cycle_count = 0;

            loop {
                // Check if we should stop
                {
                    let running = is_running.read().await;
                    if !*running {
                        log(LogTag::Pool, "DISCOVERY_STOP", "🛑 Discovery task stopped");
                        break;
                    }
                }

                cycle_count += 1;
                log(
                    LogTag::Pool,
                    "DISCOVERY_CYCLE",
                    &format!("🔄 Starting discovery cycle #{}", cycle_count)
                );

                // Get tokens that don't have cached pools
                let tokens_without_pools = cache.get_tokens_without_pools().await;

                if tokens_without_pools.is_empty() {
                    log(LogTag::Pool, "DISCOVERY_SKIP", "✅ All tokens have cached pools");
                } else {
                    log(
                        LogTag::Pool,
                        "DISCOVERY_NEEDED",
                        &format!(
                            "🔍 Need to discover pools for {} tokens",
                            tokens_without_pools.len()
                        )
                    );

                    // Process tokens in batches to avoid overwhelming APIs
                    let batch_size = 10;
                    for chunk in tokens_without_pools.chunks(batch_size) {
                        for token_mint in chunk {
                            // Check if already being processed
                            if cache.mark_in_progress(token_mint).await {
                                log(
                                    LogTag::Pool,
                                    "DISCOVERY_TOKEN",
                                    &format!("🔍 Discovering pools for {}", &token_mint[..8])
                                );

                                // Discover pools from all APIs
                                match Self::discover_pools_for_token(token_mint).await {
                                    Ok(pools) => {
                                        if !pools.is_empty() {
                                            cache.cache_pools(token_mint, pools.clone()).await;
                                            log(
                                                LogTag::Pool,
                                                "DISCOVERY_SUCCESS",
                                                &format!(
                                                    "✅ Found {} pools for {}",
                                                    pools.len(),
                                                    &token_mint[..8]
                                                )
                                            );
                                        } else {
                                            log(
                                                LogTag::Pool,
                                                "DISCOVERY_EMPTY",
                                                &format!(
                                                    "❌ No pools found for {}",
                                                    &token_mint[..8]
                                                )
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        log(
                                            LogTag::Pool,
                                            "DISCOVERY_ERROR",
                                            &format!(
                                                "❌ Error discovering pools for {}: {}",
                                                &token_mint[..8],
                                                e
                                            )
                                        );
                                    }
                                }

                                cache.mark_completed(token_mint).await;
                            }
                        }

                        // Small delay between tokens to respect rate limits
                        sleep(Duration::from_millis(200)).await;
                    }
                }

                // Clean up expired cache entries
                let (pools_cleaned, accounts_cleaned, prices_cleaned) =
                    cache.cleanup_expired().await;
                if pools_cleaned > 0 || accounts_cleaned > 0 || prices_cleaned > 0 {
                    log(
                        LogTag::Pool,
                        "CACHE_CLEANUP",
                        &format!(
                            "🧹 Cleaned {} pools, {} accounts, {} prices",
                            pools_cleaned,
                            accounts_cleaned,
                            prices_cleaned
                        )
                    );
                }

                // Wait before next cycle (5 seconds)
                sleep(Duration::from_secs(5)).await;
            }
        });
    }

    /// Stop discovery task
    pub async fn stop_discovery_task(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        log(LogTag::Pool, "DISCOVERY_STOPPING", "🛑 Stopping discovery task");
    }

    /// Discover pools for a single token using all APIs
    async fn discover_pools_for_token(token_mint: &str) -> Result<Vec<PoolInfo>, String> {
        let mut all_pools = Vec::new();

        // 1. Try DexScreener API
        match Self::discover_pools_dexscreener(token_mint).await {
            Ok(mut pools) => {
                log(
                    LogTag::Pool,
                    "DEXSCREENER_SUCCESS",
                    &format!(
                        "Found {} pools from DexScreener for {}",
                        pools.len(),
                        &token_mint[..8]
                    )
                );
                all_pools.append(&mut pools);
            }
            Err(e) => {
                log(
                    LogTag::Pool,
                    "DEXSCREENER_ERROR",
                    &format!("DexScreener error for {}: {}", &token_mint[..8], e)
                );
            }
        }

        // Small delay between API calls
        sleep(Duration::from_millis(500)).await;

        // 2. Try GeckoTerminal API
        match Self::discover_pools_geckoterminal(token_mint).await {
            Ok(mut pools) => {
                log(
                    LogTag::Pool,
                    "GECKOTERMINAL_SUCCESS",
                    &format!(
                        "Found {} pools from GeckoTerminal for {}",
                        pools.len(),
                        &token_mint[..8]
                    )
                );
                all_pools.append(&mut pools);
            }
            Err(e) => {
                log(
                    LogTag::Pool,
                    "GECKOTERMINAL_ERROR",
                    &format!("GeckoTerminal error for {}: {}", &token_mint[..8], e)
                );
            }
        }

        // Small delay between API calls
        sleep(Duration::from_millis(500)).await;

        // 3. Try Raydium API
        match Self::discover_pools_raydium(token_mint).await {
            Ok(mut pools) => {
                log(
                    LogTag::Pool,
                    "RAYDIUM_SUCCESS",
                    &format!("Found {} pools from Raydium for {}", pools.len(), &token_mint[..8])
                );
                all_pools.append(&mut pools);
            }
            Err(e) => {
                log(
                    LogTag::Pool,
                    "RAYDIUM_ERROR",
                    &format!("Raydium error for {}: {}", &token_mint[..8], e)
                );
            }
        }

        // Deduplicate pools by pool_address
        Self::deduplicate_pools(all_pools)
    }

    /// Discover pools from DexScreener API
    async fn discover_pools_dexscreener(token_mint: &str) -> Result<Vec<PoolInfo>, String> {
        // Make direct HTTP request to DexScreener API
        let url = format!("https://api.dexscreener.com/token-pairs/v1/solana/{}", token_mint);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send().await
            .map_err(|e| format!("DexScreener API request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("DexScreener API error: {}", response.status()));
        }

        let response_text = response
            .text().await
            .map_err(|e| format!("Failed to read DexScreener response: {}", e))?;

        // Parse the JSON response directly
        let json_value: serde_json::Value = serde_json
            ::from_str(&response_text)
            .map_err(|e| format!("Failed to parse DexScreener JSON: {}", e))?;

        let mut pools = Vec::new();

        // Check if response has pairs array
        if let Some(pairs_array) = json_value.as_array() {
            for pair in pairs_array {
                // Extract required fields directly from JSON
                let pool_address = pair["pairAddress"].as_str().unwrap_or("").to_string();

                let dex_id = pair["dexId"].as_str().unwrap_or("").to_string();

                // Map DEX ID to program ID
                let program_id = match dex_id.as_str() {
                    "raydium" => crate::pools::constants::RAYDIUM_LEGACY_AMM_PROGRAM_ID.to_string(),
                    "raydium-cp" => crate::pools::constants::RAYDIUM_CPMM_PROGRAM_ID.to_string(),
                    "raydium-clmm" => crate::pools::constants::RAYDIUM_CLMM_PROGRAM_ID.to_string(),
                    "meteora" => crate::pools::constants::METEORA_DLMM_PROGRAM_ID.to_string(),
                    "orca" => crate::pools::constants::ORCA_WHIRLPOOL_PROGRAM_ID.to_string(),
                    "pump" => crate::pools::constants::PUMP_FUN_AMM_PROGRAM_ID.to_string(),
                    _ => {
                        continue;
                    } // Skip unknown DEXs
                };

                // Get liquidity in USD
                let liquidity_usd = pair["liquidity"]["usd"].as_f64().unwrap_or(0.0);

                // Get base and quote reserves
                let base_reserve = pair["liquidity"]["base"].as_f64().unwrap_or(0.0);

                let quote_reserve = pair["liquidity"]["quote"].as_f64().unwrap_or(0.0);

                // Determine which is SOL and which is token
                let base_token_address = pair["baseToken"]["address"].as_str().unwrap_or("");

                let quote_token_address = pair["quoteToken"]["address"].as_str().unwrap_or("");

                let (sol_reserve, token_reserve) = if
                    base_token_address == crate::pools::constants::SOL_MINT
                {
                    (base_reserve, quote_reserve)
                } else if quote_token_address == crate::pools::constants::SOL_MINT {
                    (quote_reserve, base_reserve)
                } else {
                    // Skip pools that don't have SOL
                    continue;
                };

                // Only include pools with meaningful liquidity
                if liquidity_usd < 1000.0 || pool_address.is_empty() {
                    continue;
                }

                pools.push(PoolInfo {
                    pool_address,
                    program_id,
                    token_mint: token_mint.to_string(),
                    sol_reserve,
                    token_reserve,
                    liquidity_usd,
                });
            }
        }

        Ok(pools)
    }

    /// Discover pools from GeckoTerminal API
    async fn discover_pools_geckoterminal(_token_mint: &str) -> Result<Vec<PoolInfo>, String> {
        // TODO: Implement GeckoTerminal API call
        // Use existing tokens::geckoterminal functions
        Ok(Vec::new())
    }

    /// Discover pools from Raydium API
    async fn discover_pools_raydium(_token_mint: &str) -> Result<Vec<PoolInfo>, String> {
        // TODO: Implement Raydium API call
        // Use existing tokens::raydium functions
        Ok(Vec::new())
    }

    /// Deduplicate pools by pool_address and keep the one with highest liquidity
    fn deduplicate_pools(pools: Vec<PoolInfo>) -> Result<Vec<PoolInfo>, String> {
        let mut pool_map: HashMap<String, PoolInfo> = HashMap::new();

        for pool in pools {
            let pool_address = pool.pool_address.clone();

            if let Some(existing) = pool_map.get(&pool_address) {
                // Keep the pool with higher liquidity
                if pool.liquidity_usd > existing.liquidity_usd {
                    pool_map.insert(pool_address, pool);
                }
            } else {
                pool_map.insert(pool_address, pool);
            }
        }

        Ok(pool_map.into_values().collect())
    }

    /// Discover pools for a token (legacy method for compatibility)
    pub async fn discover_pools(&self, token_address: &str) -> Result<Vec<PoolInfo>, String> {
        // Check cache first
        if let Some(cached_pools) = self.cache.get_cached_pools(token_address).await {
            return Ok(cached_pools);
        }

        // If not in cache, discover and cache
        let pools = Self::discover_pools_for_token(token_address).await?;
        self.cache.cache_pools(token_address, pools.clone()).await;

        Ok(pools)
    }

    /// Batch discover pools for multiple tokens (legacy method for compatibility)
    pub async fn batch_discover(&self, tokens: &[String]) -> HashMap<String, Vec<PoolInfo>> {
        let mut result = HashMap::new();

        for token in tokens {
            match self.discover_pools(token).await {
                Ok(pools) => {
                    result.insert(token.clone(), pools);
                }
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "BATCH_DISCOVER_ERROR",
                        &format!("Error discovering pools for {}: {}", token, e)
                    );
                    result.insert(token.clone(), Vec::new());
                }
            }
        }

        result
    }
}
