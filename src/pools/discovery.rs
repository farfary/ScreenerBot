/// Pool discovery service
/// Finds pools for tokens from external APIs (DexScreener, GeckoTerminal, Raydium)

use chrono::{ DateTime, Utc };
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{ sleep, Duration };
use tokio::sync::RwLock;
use crate::pools::cache::PoolCache;
use crate::pools::types::PoolInfo;
use crate::pools::tokens::PoolTokenManager;
use crate::pools::constants::{
    INITIAL_TOKEN_LOAD_COUNT,
    DISCOVERY_BATCH_SIZE,
    DISCOVERY_BATCH_DELAY_MS,
    DISCOVERY_CYCLE_DELAY_SECS,
    DEXSCREENER_REQUEST_DELAY_MS,
    SOL_MINT,
};
use crate::logger::{ log, LogTag };

/// Universal pool discovery result from all APIs (DexScreener, GeckoTerminal, Raydium)
#[derive(Debug, Clone)]
pub struct DiscoveryPoolResult {
    /// Pool address
    pub pool_address: String,
    /// Token mint address
    pub token_mint: String,
    /// DEX name (raydium, meteora, orca, pump.fun, etc.)
    pub dex_name: String,
    /// Pool name or identifier
    pub pool_name: Option<String>,
    /// Base token mint address
    pub base_token_mint: String,
    /// Quote token mint address
    pub quote_token_mint: String,
    /// Price in native quote token
    pub price_native: f64,
    /// Price in USD (if available)
    pub price_usd: Option<f64>,
    /// Liquidity in USD
    pub liquidity_usd: f64,
    /// Base token reserves (raw amount)
    pub base_reserve: f64,
    /// Quote token reserves (raw amount)
    pub quote_reserve: f64,
    /// SOL reserves in the pool
    pub sol_reserves: f64,
    /// Token reserves in the pool
    pub token_reserves: f64,
    /// 24h volume in USD
    pub volume_24h: f64,
    /// Price change percentage (24h)
    pub price_change_24h: Option<f64>,
    /// Number of transactions (24h)
    pub transactions_24h: Option<u64>,
    /// Pool creation timestamp
    pub created_at: Option<u64>,
    /// When this data was discovered
    pub discovered_at: DateTime<Utc>,
    /// API source (dexscreener, geckoterminal, raydium)
    pub source: String,
}

impl DiscoveryPoolResult {
    /// Create a new discovery pool result
    pub fn new(
        pool_address: String,
        token_mint: String,
        dex_name: String,
        base_token_mint: String,
        quote_token_mint: String,
        price_native: f64,
        liquidity_usd: f64,
        base_reserve: f64,
        quote_reserve: f64,
        source: String
    ) -> Self {
        // Determine SOL and token reserves
        let sol_mint = crate::pools::constants::SOL_MINT;
        let (sol_reserves, token_reserves) = if base_token_mint == sol_mint {
            (base_reserve, quote_reserve)
        } else if quote_token_mint == sol_mint {
            (quote_reserve, base_reserve)
        } else {
            // If neither is SOL, set both to 0
            (0.0, 0.0)
        };

        Self {
            pool_address,
            token_mint,
            dex_name,
            pool_name: None,
            base_token_mint,
            quote_token_mint,
            price_native,
            price_usd: None,
            liquidity_usd,
            base_reserve,
            quote_reserve,
            sol_reserves,
            token_reserves,
            volume_24h: 0.0,
            price_change_24h: None,
            transactions_24h: None,
            created_at: None,
            discovered_at: Utc::now(),
            source,
        }
    }

    /// Check if this pool contains SOL
    pub fn has_sol_pair(&self) -> bool {
        let sol_mint = crate::pools::constants::SOL_MINT;
        self.base_token_mint == sol_mint || self.quote_token_mint == sol_mint
    }

    /// Get the other token mint (paired with the target token)
    pub fn get_other_token_mint(&self, target_token: &str) -> Option<&str> {
        if self.base_token_mint == target_token {
            Some(&self.quote_token_mint)
        } else if self.quote_token_mint == target_token {
            Some(&self.base_token_mint)
        } else {
            None
        }
    }

    /// Calculate price in SOL if possible
    pub fn get_price_in_sol(&self) -> Option<f64> {
        let sol_mint = crate::pools::constants::SOL_MINT;

        if self.base_token_mint == self.token_mint && self.quote_token_mint == sol_mint {
            // Token/SOL pair, price is in SOL
            Some(self.price_native)
        } else if self.quote_token_mint == self.token_mint && self.base_token_mint == sol_mint {
            // SOL/Token pair, need to invert
            if self.price_native > 0.0 {
                Some(1.0 / self.price_native)
            } else {
                None
            }
        } else {
            None
        }
    }
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
                    for chunk in tokens_without_pools.chunks(DISCOVERY_BATCH_SIZE) {
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
                                            cache.cache_discovery_pools(
                                                token_mint,
                                                pools.clone()
                                            ).await;
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
                        sleep(Duration::from_millis(DISCOVERY_BATCH_DELAY_MS)).await;
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

                // Wait before next cycle
                sleep(Duration::from_secs(DISCOVERY_CYCLE_DELAY_SECS)).await;
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
    async fn discover_pools_for_token(
        token_mint: &str
    ) -> Result<Vec<DiscoveryPoolResult>, String> {
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
        sleep(Duration::from_millis(DEXSCREENER_REQUEST_DELAY_MS)).await;

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
        Self::deduplicate_discovery_pools(all_pools)
    }

    /// Discover pools from DexScreener API
    async fn discover_pools_dexscreener(
        token_mint: &str
    ) -> Result<Vec<DiscoveryPoolResult>, String> {
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

                // Get liquidity in USD
                let liquidity_usd = pair["liquidity"]["usd"].as_f64().unwrap_or(0.0);

                // Get base and quote reserves
                let base_reserve = pair["liquidity"]["base"].as_f64().unwrap_or(0.0);
                let quote_reserve = pair["liquidity"]["quote"].as_f64().unwrap_or(0.0);

                // Get base and quote token addresses
                let base_token_address = pair["baseToken"]["address"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let quote_token_address = pair["quoteToken"]["address"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                // Only include pools with meaningful liquidity and valid addresses
                if
                    liquidity_usd < 1000.0 ||
                    pool_address.is_empty() ||
                    base_token_address.is_empty() ||
                    quote_token_address.is_empty()
                {
                    continue;
                }

                // Only include pools that contain the target token
                if base_token_address != token_mint && quote_token_address != token_mint {
                    continue;
                }

                pools.push(
                    DiscoveryPoolResult::new(
                        pool_address,
                        token_mint.to_string(),
                        "DexScreener".to_string(),
                        base_token_address,
                        quote_token_address,
                        1.0, // price_native - will be calculated later
                        liquidity_usd,
                        base_reserve,
                        quote_reserve,
                        "DexScreener API".to_string()
                    )
                );
            }
        }

        Ok(pools)
    }

    /// Discover pools from GeckoTerminal API
    async fn discover_pools_geckoterminal(
        token_mint: &str
    ) -> Result<Vec<DiscoveryPoolResult>, String> {
        // Use the existing GeckoTerminal API function
        let gecko_pools =
            crate::tokens::geckoterminal::get_token_pools_from_geckoterminal(token_mint).await?;

        let mut pools = Vec::new();

        for gecko_pool in gecko_pools {
            // Convert GeckoTerminal pool to DiscoveryPoolResult
            // Determine base and quote tokens from the pool structure
            let (base_token_address, quote_token_address) = if gecko_pool.base_token == token_mint {
                (gecko_pool.base_token.clone(), gecko_pool.quote_token.clone())
            } else {
                (gecko_pool.quote_token.clone(), gecko_pool.base_token.clone())
            };

            // GeckoTerminal doesn't always provide exact reserves, so we'll use price and liquidity to estimate
            let total_liquidity = gecko_pool.liquidity_usd;

            // Estimate reserves based on 50/50 split assumption
            let base_reserve =
                total_liquidity /
                2.0 /
                (if gecko_pool.price_usd > 0.0 { gecko_pool.price_usd } else { 1.0 });
            let quote_reserve = total_liquidity / 2.0;

            // Only include pools with meaningful liquidity
            if total_liquidity < 1000.0 || gecko_pool.pool_address.is_empty() {
                continue;
            }

            pools.push(
                DiscoveryPoolResult::new(
                    gecko_pool.pool_address,
                    token_mint.to_string(),
                    "GeckoTerminal".to_string(),
                    base_token_address,
                    quote_token_address,
                    1.0, // price_native - will be calculated later
                    total_liquidity,
                    base_reserve,
                    quote_reserve,
                    "GeckoTerminal API".to_string()
                )
            );
        }

        Ok(pools)
    }

    /// Discover pools from Raydium API
    async fn discover_pools_raydium(token_mint: &str) -> Result<Vec<DiscoveryPoolResult>, String> {
        // Use the existing Raydium API function
        let raydium_pools = crate::tokens::raydium::get_token_pools_from_raydium(token_mint).await?;

        let mut pools = Vec::new();

        for raydium_pool in raydium_pools {
            // Convert Raydium pool to DiscoveryPoolResult
            // Raydium pools have base_token and quote_token fields
            let base_token_address = raydium_pool.base_token;
            let quote_token_address = raydium_pool.quote_token;

            // Estimate reserves from liquidity and price
            // Raydium provides price_usd and liquidity_usd
            let total_liquidity = raydium_pool.liquidity_usd;

            // Estimate reserves based on 50/50 split assumption
            let base_reserve = if raydium_pool.price_usd > 0.0 {
                total_liquidity / 2.0 / raydium_pool.price_usd
            } else {
                total_liquidity / 2.0
            };
            let quote_reserve = total_liquidity / 2.0;

            // Only include pools with meaningful liquidity
            if total_liquidity < 1000.0 || raydium_pool.pool_address.is_empty() {
                continue;
            }

            pools.push(
                DiscoveryPoolResult::new(
                    raydium_pool.pool_address,
                    token_mint.to_string(),
                    "Raydium".to_string(),
                    base_token_address,
                    quote_token_address,
                    1.0, // price_native - will be calculated later
                    total_liquidity,
                    base_reserve,
                    quote_reserve,
                    "Raydium API".to_string()
                )
            );
        }

        Ok(pools)
    }

    /// Deduplicate pools by pool_address and keep the one with highest liquidity
    fn deduplicate_discovery_pools(
        pools: Vec<DiscoveryPoolResult>
    ) -> Result<Vec<DiscoveryPoolResult>, String> {
        let mut pool_map: HashMap<String, DiscoveryPoolResult> = HashMap::new();

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

        // Convert from new discovery to legacy format for backward compatibility
        let discovery_pools = Self::discover_pools_for_token(token_address).await?;
        let legacy_pools = Self::convert_discovery_to_legacy(discovery_pools);

        // Cache in legacy format too
        self.cache.cache_pools(token_address, legacy_pools.clone()).await;

        Ok(legacy_pools)
    }

    /// Convert DiscoveryPoolResult to PoolInfo for legacy compatibility
    fn convert_discovery_to_legacy(discovery_pools: Vec<DiscoveryPoolResult>) -> Vec<PoolInfo> {
        discovery_pools
            .into_iter()
            .map(|pool| {
                PoolInfo::new(
                    pool.pool_address,
                    "".to_string(), // program_id not available in discovery
                    pool.dex_name,
                    pool.base_token_mint,
                    pool.quote_token_mint,
                    pool.base_reserve as u64,
                    pool.quote_reserve as u64,
                    9, // default decimals
                    9, // default decimals
                    Some(pool.liquidity_usd)
                )
            })
            .collect()
    }
}
