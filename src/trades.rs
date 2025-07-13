use crate::rate_limiter::{ RateLimitedRequest, GECKOTERMINAL_LIMITER };
use serde::{ Deserialize, Serialize };
use std::collections::{ HashMap, HashSet };
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use chrono::DateTime;

// ═══════════════════════════════════════════════════════════════════════════════
// TRADES CACHE SYSTEM - BACKGROUND TASK FOR TRADE DATA COLLECTION
// ═══════════════════════════════════════════════════════════════════════════════
//
// 🎯 PURPOSE:
// • Cache 24h trade data for all watched tokens and analysis
// • Provide trade data to market analysis and monitoring functions
// • Non-blocking background task that doesn't interfere with analysis
// • Disk-based cache with automatic cleanup of old data
//
// 📁 CACHE LOCATION: .cache_trades/
// 🔄 UPDATE FREQUENCY: Every 5 minutes for active tokens
// 📊 DATA SOURCE: GeckoTerminal API trades endpoint
// 🕐 CACHE RETENTION: 24 hours maximum
// ═══════════════════════════════════════════════════════════════════════════════

const TRADES_CACHE_DIR: &str = ".cache_trades";
const TRADES_UPDATE_INTERVAL_SECONDS: u64 = 300; // 5 minutes
const TRADES_CACHE_MAX_AGE_HOURS: u64 = 24;
const TRADES_CLEANUP_INTERVAL_HOURS: u64 = 6; // Clean cache every 6 hours
const MIN_TRADE_VOLUME_USD: f64 = 0.1; // Minimum trade volume filter

// GeckoTerminal API trades response structures
#[derive(Debug, Deserialize)]
struct GeckoTradesResponse {
    data: Vec<TradeData>,
}

#[derive(Debug, Deserialize)]
struct TradeData {
    id: String,
    #[serde(rename = "type")]
    trade_type: String,
    attributes: TradeAttributes,
}

#[derive(Debug, Deserialize)]
struct TradeAttributes {
    block_number: u64,
    block_timestamp: String,
    tx_hash: String,
    tx_from_address: String,
    from_token_amount: String,
    to_token_amount: String,
    price_from_in_currency_token: String,
    price_to_in_currency_token: String,
    price_from_in_usd: String,
    price_to_in_usd: String,
    volume_in_usd: String,
    kind: String, // "buy" or "sell"
    from_token_address: String,
    to_token_address: String,
}

// Simplified trade data for caching and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub timestamp: u64,
    pub tx_hash: String,
    pub kind: String, // "buy" or "sell"
    pub from_token_amount: f64,
    pub to_token_amount: f64,
    pub price_usd: f64,
    pub volume_usd: f64,
    pub from_address: String,
    pub to_address: String,
}

// Cache structure for a token's trades (combined from multiple pools)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTradesCache {
    pub token_mint: String,
    pub symbol: String,
    pub pools: Vec<PoolTrades>, // Data from multiple pools
    pub trades: Vec<Trade>, // Combined trades from all pools
    pub last_updated: u64,
    pub total_volume_24h: f64,
    pub buy_count_24h: u64,
    pub sell_count_24h: u64,
    pub unique_traders_24h: u64,
}

// Trades data for a specific pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolTrades {
    pub pool_address: String,
    pub trades: Vec<Trade>,
    pub volume_24h: f64,
}

// Global in-memory cache
pub static TRADES_CACHE: Lazy<RwLock<HashMap<String, TokenTradesCache>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

// Track which tokens need trade data updates (token_mint -> list of pool addresses)
pub static TOKENS_TO_MONITOR: Lazy<RwLock<HashMap<String, Vec<String>>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

impl TokenTradesCache {
    pub fn new(token_mint: String, symbol: String) -> Self {
        Self {
            token_mint,
            symbol,
            pools: Vec::new(),
            trades: Vec::new(),
            last_updated: 0,
            total_volume_24h: 0.0,
            buy_count_24h: 0,
            sell_count_24h: 0,
            unique_traders_24h: 0,
        }
    }

    /// Add pool trades and recalculate combined statistics
    pub fn add_pool_trades(&mut self, pool_address: String, pool_trades: Vec<Trade>) {
        let pool_volume_24h = pool_trades
            .iter()
            .map(|t| t.volume_usd)
            .sum();

        self.pools.push(PoolTrades {
            pool_address,
            trades: pool_trades.clone(),
            volume_24h: pool_volume_24h,
        });

        // Add to combined trades (deduplicate by tx_hash)
        for trade in pool_trades {
            if !self.trades.iter().any(|t| t.tx_hash == trade.tx_hash) {
                self.trades.push(trade);
            }
        }

        // Sort combined trades by timestamp (newest first)
        self.trades.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Recalculate statistics
        self.calculate_stats();
    }

    /// Calculate trade statistics from the trades data
    pub fn calculate_stats(&mut self) {
        let now = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let twenty_four_hours_ago = now - 24 * 3600;

        // Filter trades from last 24 hours
        let recent_trades: Vec<&Trade> = self.trades
            .iter()
            .filter(|trade| trade.timestamp >= twenty_four_hours_ago)
            .collect();

        self.total_volume_24h = recent_trades
            .iter()
            .map(|trade| trade.volume_usd)
            .sum();

        self.buy_count_24h = recent_trades
            .iter()
            .filter(|trade| trade.kind == "buy")
            .count() as u64;

        self.sell_count_24h = recent_trades
            .iter()
            .filter(|trade| trade.kind == "sell")
            .count() as u64;

        // Count unique traders (from_address)
        let unique_traders: HashSet<String> = recent_trades
            .iter()
            .map(|trade| trade.from_address.clone())
            .collect();

        self.unique_traders_24h = unique_traders.len() as u64;
    }

    /// Get trades by type (buy/sell) from last N hours
    pub fn get_trades_by_type(&self, trade_type: &str, hours: u64) -> Vec<Trade> {
        let now = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cutoff_time = now - hours * 3600;

        self.trades
            .iter()
            .filter(|trade| { trade.timestamp >= cutoff_time && trade.kind == trade_type })
            .cloned()
            .collect()
    }

    /// Get large trades (whale activity) above threshold
    pub fn get_whale_trades(&self, min_usd_volume: f64, hours: u64) -> Vec<Trade> {
        let now = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let cutoff_time = now - hours * 3600;

        self.trades
            .iter()
            .filter(|trade| {
                trade.timestamp >= cutoff_time && trade.volume_usd >= min_usd_volume
            })
            .cloned()
            .collect()
    }

    /// Check if data is fresh (updated within last update interval)
    pub fn is_data_fresh(&self) -> bool {
        let now = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now - self.last_updated < TRADES_UPDATE_INTERVAL_SECONDS
    }
}

/// Start the background trades caching task
pub fn start_trades_cache_task() {
    tokio::spawn(async {
        println!("🔄 Starting trades cache background task...");

        // Create cache directory if it doesn't exist
        if let Err(e) = tokio::fs::create_dir_all(TRADES_CACHE_DIR).await {
            println!("❌ Failed to create trades cache directory: {}", e);
            return;
        }

        // Load existing cache from disk
        load_trades_cache().await;

        let mut cleanup_counter = 0;

        loop {
            if SHUTDOWN.load(Ordering::SeqCst) {
                break;
            }

            // Update trades for monitored tokens
            update_monitored_tokens_trades().await;

            // Cleanup old cache files every few cycles
            cleanup_counter += 1;
            if
                cleanup_counter >=
                (TRADES_CLEANUP_INTERVAL_HOURS * 3600) / TRADES_UPDATE_INTERVAL_SECONDS
            {
                cleanup_old_cache_files().await;
                cleanup_counter = 0;
            }

            // Save updated cache to disk
            save_trades_cache().await;

            // Wait for next update cycle
            tokio::time::sleep(Duration::from_secs(TRADES_UPDATE_INTERVAL_SECONDS)).await;
        }

        println!("🛑 Trades cache task shutting down...");
    });
}

/// Add tokens to monitoring list (for watched tokens and open positions)
pub async fn add_tokens_to_monitor(tokens: &[&Token]) {
    let mut monitor_list = TOKENS_TO_MONITOR.write().await;

    for token in tokens {
        let mut pools = vec![token.pair_address.clone()];

        // Try to get additional pools from DexScreener data using helpers.rs
        match crate::helpers::fetch_combined_pools(&token.mint).await {
            Ok(pool_infos) => {
                // Add up to 3 top pools (sorted by liquidity/volume in fetch_combined_pools)
                for pool_info in pool_infos.into_iter().take(3) {
                    if !pools.contains(&pool_info) {
                        pools.push(pool_info);
                        if pools.len() >= 3 {
                            break;
                        }
                    }
                }
                println!(
                    "📊 Found {} pools for token {} ({})",
                    pools.len(),
                    &token.symbol,
                    &token.mint[..8]
                );
            }
            Err(e) => {
                println!(
                    "⚠️ Could not fetch additional pools for {}: {} - using primary pool only",
                    token.symbol,
                    e
                );
            }
        }

        // Update monitoring list
        monitor_list.insert(token.mint.clone(), pools);
    }

    let total_pools: usize = monitor_list
        .values()
        .map(|pools| pools.len())
        .sum();
    println!("📊 Added {} tokens to trades monitoring ({} total pools)", tokens.len(), total_pools);
}

/// Remove token from monitoring list
pub async fn remove_token_from_monitor(token_mint: &str, pool_address: &str) {
    let mut monitor_list = TOKENS_TO_MONITOR.write().await;

    if let Some(pools) = monitor_list.get_mut(token_mint) {
        pools.retain(|pool| pool != pool_address);
        if pools.is_empty() {
            monitor_list.remove(token_mint);
        }
    }
}

/// Update trades data for all monitored tokens
async fn update_monitored_tokens_trades() {
    let monitor_list = TOKENS_TO_MONITOR.read().await.clone();

    if monitor_list.is_empty() {
        return;
    }

    let total_tokens = monitor_list.len();
    let total_pools: usize = monitor_list
        .values()
        .map(|pools| pools.len())
        .sum();

    println!("🔄 Updating trades for {} tokens across {} pools...", total_tokens, total_pools);

    // Process tokens in parallel but with rate limiting
    for (token_mint, pools) in monitor_list {
        // Get top 3 pools for this token (expand pools if needed)
        let expanded_pools = get_top_pools_for_token(&token_mint, &pools).await;

        // Update monitoring list with expanded pools
        {
            let mut monitor_list = TOKENS_TO_MONITOR.write().await;
            monitor_list.insert(token_mint.clone(), expanded_pools.clone());
        }

        // Process this token with all its pools
        update_token_trades_multi_pool(token_mint, expanded_pools).await;

        // Rate limiting between tokens
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// Get top pools for a token (expand to up to 3 pools)
async fn get_top_pools_for_token(token_mint: &str, current_pools: &[String]) -> Vec<String> {
    let mut pools = current_pools.to_vec();

    // If we already have 3 pools, return them
    if pools.len() >= 3 {
        pools.truncate(3);
        return pools;
    }

    // Use existing pool data from helpers.rs
    match crate::helpers::fetch_combined_pools(token_mint).await {
        Ok(pool_infos) => {
            // Pool data is already sorted by liquidity and volume in fetch_combined_pools
            // Get up to 3 top pools that aren't already in our list
            for pool_info in pool_infos.into_iter().take(3) {
                if !pools.contains(&pool_info) {
                    pools.push(pool_info);
                    if pools.len() >= 3 {
                        break;
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to fetch pools for token {}: {}", token_mint, e);
        }
    }

    // Ensure we don't exceed 3 pools
    pools.truncate(3);

    pools
}

/// Update trades data for a specific token across multiple pools
async fn update_token_trades_multi_pool(token_mint: String, pool_addresses: Vec<String>) {
    // Check if we already have fresh data
    {
        let cache = TRADES_CACHE.read().await;
        if let Some(token_cache) = cache.get(&token_mint) {
            if token_cache.is_data_fresh() {
                return; // Skip update if data is still fresh
            }
        }
    }

    // Create new cache for this token
    let mut token_cache = TokenTradesCache::new(token_mint.clone(), String::new());

    // Fetch trades from each pool
    for pool_address in pool_addresses {
        match fetch_pool_trades(&pool_address).await {
            Ok(pool_trades) => {
                token_cache.add_pool_trades(pool_address.clone(), pool_trades);
                println!(
                    "✅ Fetched trades from pool {} for token {}",
                    &pool_address[..8],
                    &token_mint[..8]
                );
            }
            Err(e) => {
                println!(
                    "❌ Failed to fetch trades from pool {} for token {}: {}",
                    &pool_address[..8],
                    &token_mint[..8],
                    e
                );
            }
        }

        // Rate limiting between pool requests
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Update timestamp
    token_cache.last_updated = std::time::SystemTime
        ::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Update in-memory cache
    {
        let mut cache = TRADES_CACHE.write().await;
        cache.insert(token_mint.clone(), token_cache.clone());
    }

    // Save to disk cache file
    save_token_trades_to_disk(&token_mint, &token_cache).await;
}

/// Fetch trades data from a specific pool via GeckoTerminal API
async fn fetch_pool_trades(pool_address: &str) -> Result<Vec<Trade>> {
    let url = format!(
        "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/trades?trade_volume_in_usd_greater_than={}",
        pool_address,
        MIN_TRADE_VOLUME_USD
    );

    let client = reqwest::Client::new();
    let response = client.get_with_rate_limit(&url, &GECKOTERMINAL_LIMITER).await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("API request failed: {}", response.status()));
    }

    let gecko_response: GeckoTradesResponse = response.json().await?;

    // Convert to our simplified trade format
    let mut trades = Vec::new();

    for trade_data in gecko_response.data {
        if let Ok(trade) = convert_gecko_trade_to_trade(trade_data) {
            trades.push(trade);
        }
    }

    // Sort trades by timestamp (newest first)
    trades.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(trades)
}

/// Convert GeckoTerminal trade data to our simplified format
fn convert_gecko_trade_to_trade(trade_data: TradeData) -> Result<Trade> {
    let attrs = trade_data.attributes;

    // Parse timestamp from ISO string
    let timestamp = DateTime::parse_from_rfc3339(&attrs.block_timestamp)?.timestamp() as u64;

    Ok(Trade {
        timestamp,
        tx_hash: attrs.tx_hash,
        kind: attrs.kind,
        from_token_amount: attrs.from_token_amount.parse().unwrap_or(0.0),
        to_token_amount: attrs.to_token_amount.parse().unwrap_or(0.0),
        price_usd: attrs.price_from_in_usd.parse().unwrap_or(0.0),
        volume_usd: attrs.volume_in_usd.parse().unwrap_or(0.0),
        from_address: attrs.tx_from_address,
        to_address: attrs.to_token_address,
    })
}

/// Load trades cache from disk
async fn load_trades_cache() {
    if let Ok(entries) = tokio::fs::read_dir(TRADES_CACHE_DIR).await {
        let mut entries = entries;
        let mut loaded_count = 0;

        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    let token_mint = file_name.replace(".json", "");

                    if let Ok(data) = tokio::fs::read(entry.path()).await {
                        if let Ok(cache) = serde_json::from_slice::<TokenTradesCache>(&data) {
                            let mut trades_cache = TRADES_CACHE.write().await;
                            trades_cache.insert(token_mint, cache);
                            loaded_count += 1;
                        }
                    }
                }
            }
        }

        if loaded_count > 0 {
            println!("📥 Loaded {} token trades from cache", loaded_count);
        }
    }
}

/// Save all trades cache to disk
async fn save_trades_cache() {
    let cache = TRADES_CACHE.read().await;

    for (token_mint, trades_cache) in cache.iter() {
        save_token_trades_to_disk(token_mint, trades_cache).await;
    }
}

/// Save specific token trades to disk
async fn save_token_trades_to_disk(token_mint: &str, trades_cache: &TokenTradesCache) {
    let file_path = format!("{}/{}.json", TRADES_CACHE_DIR, token_mint);

    if let Ok(data) = serde_json::to_vec_pretty(trades_cache) {
        if let Err(e) = tokio::fs::write(&file_path, data).await {
            println!("❌ Failed to save trades cache for {}: {}", token_mint, e);
        }
    }
}

/// Clean up old cache files (older than 24 hours)
async fn cleanup_old_cache_files() {
    let now = std::time::SystemTime
        ::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let max_age = TRADES_CACHE_MAX_AGE_HOURS * 3600;
    let mut cleaned_count = 0;

    if let Ok(entries) = tokio::fs::read_dir(TRADES_CACHE_DIR).await {
        let mut entries = entries;

        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(modified_duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                        let file_age = now - modified_duration.as_secs();

                        if file_age > max_age {
                            if let Err(e) = tokio::fs::remove_file(entry.path()).await {
                                println!("❌ Failed to remove old cache file: {}", e);
                            } else {
                                cleaned_count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    if cleaned_count > 0 {
        println!("🧹 Cleaned up {} old trade cache files", cleaned_count);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUBLIC API FUNCTIONS FOR STRATEGY MODULES
// ═══════════════════════════════════════════════════════════════════════════════

/// Get trades data for a token (used by analysis and monitoring functions)
pub async fn get_token_trades(token_mint: &str) -> Option<TokenTradesCache> {
    let cache = TRADES_CACHE.read().await;
    cache.get(token_mint).cloned()
}

/// Check if trade data is available for a token
pub async fn has_trade_data(token_mint: &str) -> bool {
    let cache = TRADES_CACHE.read().await;
    cache.contains_key(token_mint)
}

/// Get trade data availability status for multiple tokens
pub async fn get_trade_data_status(token_mints: &[String]) -> HashMap<String, bool> {
    let cache = TRADES_CACHE.read().await;
    let mut status = HashMap::new();

    for mint in token_mints {
        status.insert(mint.clone(), cache.contains_key(mint));
    }

    status
}

/// Get summary statistics for all cached trades
pub async fn get_trades_cache_summary() -> (usize, u64, u64) {
    let cache = TRADES_CACHE.read().await;
    let total_tokens = cache.len();
    let total_trades: u64 = cache
        .values()
        .map(|tc| tc.trades.len() as u64)
        .sum();

    let now = std::time::SystemTime
        ::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let fresh_tokens = cache
        .values()
        .filter(|tc| now - tc.last_updated < TRADES_UPDATE_INTERVAL_SECONDS)
        .count() as u64;

    (total_tokens, total_trades, fresh_tokens)
}
