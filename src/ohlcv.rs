use crate::prelude::*;
use serde::{ Deserialize, Serialize };
use std::collections::{ HashMap, HashSet };
use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use chrono::{ DateTime, Utc };

// ═══════════════════════════════════════════════════════════════════════════════
// OHLCV CACHE SYSTEM - BACKGROUND TASK FOR PRICE/VOLUME DATA COLLECTION
// ═══════════════════════════════════════════════════════════════════════════════
//
// 🎯 PURPOSE:
// • Cache 24h OHLCV data for all watched tokens, open positions, and closed positions
// • Provide dataframe-like interface to should_buy, should_sell, should_dca functions
// • Non-blocking background task that doesn't interfere with trading
// • Disk-based cache with automatic cleanup of old data
// • Trader can operate even when data is not available
//
// 📁 CACHE LOCATION: .cache_ohlcv/
// 🔄 UPDATE FREQUENCY: Every 2 minutes for active tokens
// 📊 DATA SOURCE: GeckoTerminal API OHLCV endpoint
// 🕐 CACHE RETENTION: 24 hours maximum
// 💾 STORAGE: Unlimited tokens can be cached, but only last 24h data
// ═══════════════════════════════════════════════════════════════════════════════

const OHLCV_CACHE_DIR: &str = ".cache_ohlcv";
const OHLCV_UPDATE_INTERVAL_SECONDS: u64 = 120; // 2 minutes
const OHLCV_CACHE_MAX_AGE_HOURS: u64 = 24;
const OHLCV_CLEANUP_INTERVAL_HOURS: u64 = 6; // Clean cache every 6 hours
const OHLCV_REQUEST_LIMIT: u64 = 1000; // Max candles per request
const OHLCV_RATE_LIMIT_MS: u64 = 200; // Rate limiting between requests

// GeckoTerminal API OHLCV response structures
#[derive(Debug, Deserialize)]
struct OhlcvResponse {
    data: OhlcvData,
}

#[derive(Debug, Deserialize)]
struct OhlcvData {
    id: String,
    #[serde(rename = "type")]
    data_type: String,
    attributes: OhlcvAttributes,
}

#[derive(Debug, Deserialize)]
struct OhlcvAttributes {
    ohlcv_list: Vec<Vec<f64>>, // [timestamp, open, high, low, close, volume]
}

// Single OHLCV candle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcvCandle {
    pub timestamp: u64, // Unix timestamp
    pub open: f64, // Opening price (USD)
    pub high: f64, // Highest price (USD)
    pub low: f64, // Lowest price (USD)
    pub close: f64, // Closing price (USD)
    pub volume: f64, // Volume (USD)
}

// DataFrame-like structure for OHLCV data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcvDataFrame {
    pub token_mint: String,
    pub symbol: String,
    pub pool_address: String,
    pub candles: Vec<OhlcvCandle>,
    pub last_updated: u64,
    pub timeframe: String, // "1m", "5m", "15m", "1h", "4h", "1d"
}

// Cache structure for a token's OHLCV data across multiple timeframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenOhlcvCache {
    pub token_mint: String,
    pub symbol: String,
    pub pool_address: String,
    pub minute_1: OhlcvDataFrame, // 1-minute candles
    pub minute_5: OhlcvDataFrame, // 5-minute candles
    pub minute_15: OhlcvDataFrame, // 15-minute candles
    pub hour_1: OhlcvDataFrame, // 1-hour candles
    pub hour_4: OhlcvDataFrame, // 4-hour candles
    pub day_1: OhlcvDataFrame, // 1-day candles
    pub last_updated: u64,
}

// Global in-memory OHLCV cache
pub static OHLCV_CACHE: Lazy<RwLock<HashMap<String, TokenOhlcvCache>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

// Track which tokens need OHLCV data updates
pub static TOKENS_TO_MONITOR_OHLCV: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

// Priority tokens (watched tokens, open positions) - updated more frequently
pub static PRIORITY_TOKENS: Lazy<RwLock<HashSet<String>>> = Lazy::new(||
    RwLock::new(HashSet::new())
);

impl OhlcvCandle {
    pub fn new(timestamp: u64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        Self {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        }
    }

    /// Get the typical price (HLC/3)
    pub fn typical_price(&self) -> f64 {
        (self.high + self.low + self.close) / 3.0
    }

    /// Get the price change from open to close
    pub fn price_change(&self) -> f64 {
        self.close - self.open
    }

    /// Get the price change percentage
    pub fn price_change_percent(&self) -> f64 {
        if self.open == 0.0 { 0.0 } else { ((self.close - self.open) / self.open) * 100.0 }
    }

    /// Check if this is a green/bullish candle
    pub fn is_green(&self) -> bool {
        self.close > self.open
    }

    /// Get the body size (absolute difference between open and close)
    pub fn body_size(&self) -> f64 {
        (self.close - self.open).abs()
    }

    /// Get the upper wick size
    pub fn upper_wick(&self) -> f64 {
        self.high - self.open.max(self.close)
    }

    /// Get the lower wick size
    pub fn lower_wick(&self) -> f64 {
        self.open.min(self.close) - self.low
    }
}

impl OhlcvDataFrame {
    pub fn new(
        token_mint: String,
        symbol: String,
        pool_address: String,
        timeframe: String
    ) -> Self {
        Self {
            token_mint,
            symbol,
            pool_address,
            candles: Vec::new(),
            last_updated: 0,
            timeframe,
        }
    }

    /// Add new candles and maintain 24h limit
    pub fn add_candles(&mut self, new_candles: Vec<OhlcvCandle>) {
        // Add new candles
        for candle in new_candles {
            // Check if candle already exists (by timestamp)
            if !self.candles.iter().any(|c| c.timestamp == candle.timestamp) {
                self.candles.push(candle);
            }
        }

        // Sort by timestamp (newest first)
        self.candles.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Remove candles older than 24 hours
        let twenty_four_hours_ago =
            std::time::SystemTime
                ::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() -
            24 * 3600;

        self.candles.retain(|candle| candle.timestamp >= twenty_four_hours_ago);

        self.last_updated = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Get the latest candle
    pub fn latest(&self) -> Option<&OhlcvCandle> {
        self.candles.first()
    }

    /// Get candles from the last N minutes/hours
    pub fn get_recent_candles(&self, minutes: u64) -> Vec<&OhlcvCandle> {
        let cutoff_time =
            std::time::SystemTime
                ::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() -
            minutes * 60;

        self.candles
            .iter()
            .filter(|candle| candle.timestamp >= cutoff_time)
            .collect()
    }

    /// Get the current price (latest close)
    pub fn current_price(&self) -> Option<f64> {
        self.latest().map(|candle| candle.close)
    }

    /// Calculate price change over last N candles
    pub fn price_change_over_period(&self, candles_count: usize) -> Option<f64> {
        if self.candles.len() < candles_count {
            return None;
        }

        let latest = self.candles.first()?.close;
        let previous = self.candles.get(candles_count - 1)?.close;

        Some(((latest - previous) / previous) * 100.0)
    }

    /// Calculate average volume over last N candles
    pub fn average_volume(&self, candles_count: usize) -> Option<f64> {
        if self.candles.is_empty() {
            return None;
        }

        let candles_to_check = self.candles.iter().take(candles_count);
        let total_volume: f64 = candles_to_check.map(|c| c.volume).sum();
        let count = self.candles.len().min(candles_count);

        Some(total_volume / (count as f64))
    }

    /// Get volume weighted average price (VWAP) over last N candles
    pub fn vwap(&self, candles_count: usize) -> Option<f64> {
        if self.candles.is_empty() {
            return None;
        }

        let candles_to_check = self.candles.iter().take(candles_count);
        let mut total_price_volume = 0.0;
        let mut total_volume = 0.0;

        for candle in candles_to_check {
            let typical_price = candle.typical_price();
            total_price_volume += typical_price * candle.volume;
            total_volume += candle.volume;
        }

        if total_volume > 0.0 {
            Some(total_price_volume / total_volume)
        } else {
            None
        }
    }

    /// Check if data is fresh (updated within last update interval)
    pub fn is_data_fresh(&self) -> bool {
        let now = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now - self.last_updated < OHLCV_UPDATE_INTERVAL_SECONDS
    }

    /// Get volatility (standard deviation of price changes) over last N candles
    pub fn volatility(&self, candles_count: usize) -> Option<f64> {
        if self.candles.len() < 2 {
            return None;
        }

        let recent_candles: Vec<&OhlcvCandle> = self.candles.iter().take(candles_count).collect();
        if recent_candles.len() < 2 {
            return None;
        }

        let price_changes: Vec<f64> = recent_candles
            .iter()
            .map(|c| c.price_change_percent())
            .collect();

        let mean = price_changes.iter().sum::<f64>() / (price_changes.len() as f64);
        let variance =
            price_changes
                .iter()
                .map(|change| (change - mean).powi(2))
                .sum::<f64>() / (price_changes.len() as f64);

        Some(variance.sqrt())
    }
}

impl TokenOhlcvCache {
    pub fn new(token_mint: String, symbol: String, pool_address: String) -> Self {
        Self {
            token_mint: token_mint.clone(),
            symbol: symbol.clone(),
            pool_address: pool_address.clone(),
            minute_1: OhlcvDataFrame::new(
                token_mint.clone(),
                symbol.clone(),
                pool_address.clone(),
                "1m".to_string()
            ),
            minute_5: OhlcvDataFrame::new(
                token_mint.clone(),
                symbol.clone(),
                pool_address.clone(),
                "5m".to_string()
            ),
            minute_15: OhlcvDataFrame::new(
                token_mint.clone(),
                symbol.clone(),
                pool_address.clone(),
                "15m".to_string()
            ),
            hour_1: OhlcvDataFrame::new(
                token_mint.clone(),
                symbol.clone(),
                pool_address.clone(),
                "1h".to_string()
            ),
            hour_4: OhlcvDataFrame::new(
                token_mint.clone(),
                symbol.clone(),
                pool_address.clone(),
                "4h".to_string()
            ),
            day_1: OhlcvDataFrame::new(token_mint, symbol, pool_address, "1d".to_string()),
            last_updated: 0,
        }
    }

    /// Update the cache timestamp
    pub fn update_timestamp(&mut self) {
        self.last_updated = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Check if any timeframe data is fresh
    pub fn has_fresh_data(&self) -> bool {
        self.minute_1.is_data_fresh() ||
            self.minute_5.is_data_fresh() ||
            self.hour_1.is_data_fresh()
    }

    /// Get the most appropriate timeframe for analysis
    pub fn get_primary_timeframe(&self) -> &OhlcvDataFrame {
        // Prefer 5-minute data for most analysis
        if !self.minute_5.candles.is_empty() {
            &self.minute_5
        } else if !self.minute_1.candles.is_empty() {
            &self.minute_1
        } else if !self.hour_1.candles.is_empty() {
            &self.hour_1
        } else {
            &self.day_1
        }
    }
}

/// Start the background OHLCV caching task
pub fn start_ohlcv_cache_task() {
    tokio::spawn(async {
        println!("🔄 Starting OHLCV cache background task...");

        // Create cache directory if it doesn't exist
        if let Err(e) = tokio::fs::create_dir_all(OHLCV_CACHE_DIR).await {
            println!("❌ Failed to create OHLCV cache directory: {}", e);
            return;
        }

        // Load existing cache from disk
        load_ohlcv_cache().await;

        let mut cleanup_counter = 0;
        let mut priority_update_counter = 0;

        loop {
            if SHUTDOWN.load(Ordering::SeqCst) {
                break;
            }

            // Update priority tokens more frequently (every cycle)
            update_priority_tokens_ohlcv().await;
            priority_update_counter += 1;

            // Update all monitored tokens every 3 cycles
            if priority_update_counter >= 3 {
                update_all_monitored_tokens_ohlcv().await;
                priority_update_counter = 0;
            }

            // Cleanup old cache files every few cycles
            cleanup_counter += 1;
            if
                cleanup_counter >=
                (OHLCV_CLEANUP_INTERVAL_HOURS * 3600) / OHLCV_UPDATE_INTERVAL_SECONDS
            {
                cleanup_old_ohlcv_cache_files().await;
                cleanup_counter = 0;
            }

            // Save updated cache to disk
            save_ohlcv_cache().await;

            // Wait for next update cycle
            tokio::time::sleep(Duration::from_secs(OHLCV_UPDATE_INTERVAL_SECONDS)).await;
        }

        println!("🛑 OHLCV cache task shutting down...");
    });
}

/// Add tokens to OHLCV monitoring list
pub async fn add_tokens_to_ohlcv_monitor(tokens: &[&Token]) {
    let mut monitor_list = TOKENS_TO_MONITOR_OHLCV.write().await;

    for token in tokens {
        monitor_list.insert(token.mint.clone(), token.pair_address.clone());
    }

    println!("📊 Added {} tokens to OHLCV monitoring", tokens.len());
}

/// Add token to priority monitoring (watched tokens, open positions)
pub async fn add_priority_token(token_mint: &str) {
    let mut priority_tokens = PRIORITY_TOKENS.write().await;
    priority_tokens.insert(token_mint.to_string());
    println!("⭐ Added {} to priority OHLCV monitoring", &token_mint[..8]);
}

/// Remove token from priority monitoring
pub async fn remove_priority_token(token_mint: &str) {
    let mut priority_tokens = PRIORITY_TOKENS.write().await;
    priority_tokens.remove(token_mint);
}

/// Update OHLCV data for priority tokens (watched tokens, open positions)
async fn update_priority_tokens_ohlcv() {
    let priority_tokens = PRIORITY_TOKENS.read().await.clone();
    let monitor_list = TOKENS_TO_MONITOR_OHLCV.read().await;

    for token_mint in priority_tokens {
        if let Some(pool_address) = monitor_list.get(&token_mint) {
            // Check if we already have fresh data
            {
                let cache = OHLCV_CACHE.read().await;
                if let Some(token_cache) = cache.get(&token_mint) {
                    if token_cache.has_fresh_data() {
                        continue; // Skip if data is still fresh
                    }
                }
            }

            update_token_ohlcv(&token_mint, pool_address).await;

            // Rate limiting between priority tokens
            tokio::time::sleep(Duration::from_millis(OHLCV_RATE_LIMIT_MS)).await;
        }
    }
}

/// Update OHLCV data for all monitored tokens
async fn update_all_monitored_tokens_ohlcv() {
    let monitor_list = TOKENS_TO_MONITOR_OHLCV.read().await.clone();
    let priority_tokens = PRIORITY_TOKENS.read().await.clone();

    if monitor_list.is_empty() {
        return;
    }

    println!("🔄 Updating OHLCV for {} monitored tokens...", monitor_list.len());

    for (token_mint, pool_address) in monitor_list {
        // Skip priority tokens (already updated)
        if priority_tokens.contains(&token_mint) {
            continue;
        }

        // Check if we already have fresh data
        {
            let cache = OHLCV_CACHE.read().await;
            if let Some(token_cache) = cache.get(&token_mint) {
                if token_cache.has_fresh_data() {
                    continue; // Skip if data is still fresh
                }
            }
        }

        update_token_ohlcv(&token_mint, &pool_address).await;

        // Rate limiting between tokens
        tokio::time::sleep(Duration::from_millis(OHLCV_RATE_LIMIT_MS * 2)).await;
    }
}

/// Update OHLCV data for a specific token
async fn update_token_ohlcv(token_mint: &str, pool_address: &str) {
    // Get or create token cache
    let mut token_cache = {
        let cache = OHLCV_CACHE.read().await;
        cache
            .get(token_mint)
            .cloned()
            .unwrap_or_else(|| {
                TokenOhlcvCache::new(
                    token_mint.to_string(),
                    String::new(),
                    pool_address.to_string()
                )
            })
    };

    // Update different timeframes
    let timeframes = [
        ("minute", "1", &mut token_cache.minute_1),
        ("minute", "5", &mut token_cache.minute_5),
        ("minute", "15", &mut token_cache.minute_15),
        ("hour", "1", &mut token_cache.hour_1),
        ("hour", "4", &mut token_cache.hour_4),
        ("day", "1", &mut token_cache.day_1),
    ];

    for (timeframe, aggregate, dataframe) in timeframes {
        match fetch_ohlcv_data(pool_address, timeframe, aggregate).await {
            Ok(candles) => {
                dataframe.add_candles(candles);
                println!("✅ Updated {}:{} OHLCV for {}", timeframe, aggregate, &token_mint[..8]);
            }
            Err(e) => {
                println!(
                    "❌ Failed to fetch {}:{} OHLCV for {}: {}",
                    timeframe,
                    aggregate,
                    &token_mint[..8],
                    e
                );
            }
        }

        // Rate limiting between timeframe requests
        tokio::time::sleep(Duration::from_millis(OHLCV_RATE_LIMIT_MS / 2)).await;
    }

    // Update cache timestamp
    token_cache.update_timestamp();

    // Update in-memory cache
    {
        let mut cache = OHLCV_CACHE.write().await;
        cache.insert(token_mint.to_string(), token_cache.clone());
    }

    // Save to disk cache file
    save_token_ohlcv_to_disk(token_mint, &token_cache).await;
}

/// Fetch OHLCV data from GeckoTerminal API
async fn fetch_ohlcv_data(
    pool_address: &str,
    timeframe: &str,
    aggregate: &str
) -> Result<Vec<OhlcvCandle>> {
    let url = format!(
        "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/{}?aggregate={}&limit={}",
        pool_address,
        timeframe,
        aggregate,
        OHLCV_REQUEST_LIMIT
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).header("accept", "application/json").send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("API request failed: {}", response.status()));
    }

    let ohlcv_response: OhlcvResponse = response.json().await?;

    // Convert to our OhlcvCandle format
    let mut candles = Vec::new();
    for ohlcv_data in ohlcv_response.data.attributes.ohlcv_list {
        if ohlcv_data.len() >= 6 {
            let candle = OhlcvCandle::new(
                ohlcv_data[0] as u64, // timestamp
                ohlcv_data[1], // open
                ohlcv_data[2], // high
                ohlcv_data[3], // low
                ohlcv_data[4], // close
                ohlcv_data[5] // volume
            );
            candles.push(candle);
        }
    }

    Ok(candles)
}

/// Load OHLCV cache from disk
async fn load_ohlcv_cache() {
    if let Ok(entries) = tokio::fs::read_dir(OHLCV_CACHE_DIR).await {
        let mut entries = entries;
        let mut loaded_count = 0;

        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".json") {
                    let token_mint = file_name.replace(".json", "");

                    if let Ok(data) = tokio::fs::read(entry.path()).await {
                        if let Ok(cache) = serde_json::from_slice::<TokenOhlcvCache>(&data) {
                            let mut ohlcv_cache = OHLCV_CACHE.write().await;
                            ohlcv_cache.insert(token_mint, cache);
                            loaded_count += 1;
                        }
                    }
                }
            }
        }

        if loaded_count > 0 {
            println!("📥 Loaded {} token OHLCV caches from disk", loaded_count);
        }
    }
}

/// Save all OHLCV cache to disk
async fn save_ohlcv_cache() {
    let cache = OHLCV_CACHE.read().await;

    for (token_mint, ohlcv_cache) in cache.iter() {
        save_token_ohlcv_to_disk(token_mint, ohlcv_cache).await;
    }
}

/// Save specific token OHLCV to disk
async fn save_token_ohlcv_to_disk(token_mint: &str, ohlcv_cache: &TokenOhlcvCache) {
    // Ensure cache directory exists
    if let Err(e) = tokio::fs::create_dir_all(OHLCV_CACHE_DIR).await {
        println!("❌ Failed to create OHLCV cache directory: {}", e);
        return;
    }

    let file_path = format!("{}/{}.json", OHLCV_CACHE_DIR, token_mint);

    if let Ok(data) = serde_json::to_vec_pretty(ohlcv_cache) {
        if let Err(e) = tokio::fs::write(&file_path, data).await {
            println!("❌ Failed to save OHLCV cache for {}: {}", token_mint, e);
        }
    }
}

/// Clean up old cache files (older than 24 hours)
async fn cleanup_old_ohlcv_cache_files() {
    let now = std::time::SystemTime
        ::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let max_age = OHLCV_CACHE_MAX_AGE_HOURS * 3600;
    let mut cleaned_count = 0;

    if let Ok(entries) = tokio::fs::read_dir(OHLCV_CACHE_DIR).await {
        let mut entries = entries;

        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(modified_duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                        let file_age = now - modified_duration.as_secs();

                        if file_age > max_age {
                            if let Err(e) = tokio::fs::remove_file(entry.path()).await {
                                println!("❌ Failed to remove old OHLCV cache file: {}", e);
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
        println!("🧹 Cleaned up {} old OHLCV cache files", cleaned_count);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUBLIC API FUNCTIONS FOR STRATEGY MODULES
// ═══════════════════════════════════════════════════════════════════════════════

/// Get OHLCV dataframe for a token (used by should_buy, should_sell, should_dca)
pub async fn get_token_ohlcv_dataframe(token_mint: &str) -> Option<TokenOhlcvCache> {
    let cache = OHLCV_CACHE.read().await;
    cache.get(token_mint).cloned()
}

/// Check if OHLCV data is available for a token
pub async fn has_ohlcv_data(token_mint: &str) -> bool {
    let cache = OHLCV_CACHE.read().await;
    if let Some(token_cache) = cache.get(token_mint) {
        // Check if any timeframe has data
        !token_cache.minute_1.candles.is_empty() ||
            !token_cache.minute_5.candles.is_empty() ||
            !token_cache.hour_1.candles.is_empty() ||
            !token_cache.day_1.candles.is_empty()
    } else {
        false
    }
}

/// Get current price for a token (from latest OHLCV data)
pub async fn get_current_price_from_ohlcv(token_mint: &str) -> Option<f64> {
    let cache = OHLCV_CACHE.read().await;
    if let Some(token_cache) = cache.get(token_mint) {
        let primary_timeframe = token_cache.get_primary_timeframe();
        primary_timeframe.current_price()
    } else {
        None
    }
}

/// Get OHLCV data availability status for multiple tokens
pub async fn get_ohlcv_data_status(token_mints: &[String]) -> HashMap<String, bool> {
    let cache = OHLCV_CACHE.read().await;
    let mut status = HashMap::new();

    for mint in token_mints {
        let has_data = if let Some(token_cache) = cache.get(mint) {
            !token_cache.minute_1.candles.is_empty() ||
                !token_cache.minute_5.candles.is_empty() ||
                !token_cache.hour_1.candles.is_empty() ||
                !token_cache.day_1.candles.is_empty()
        } else {
            false
        };
        status.insert(mint.clone(), has_data);
    }

    status
}

/// Get summary statistics for all cached OHLCV data
pub async fn get_ohlcv_cache_summary() -> (usize, u64, u64) {
    let cache = OHLCV_CACHE.read().await;
    let total_tokens = cache.len();
    let total_candles: u64 = cache
        .values()
        .map(|tc| {
            (tc.minute_1.candles.len() as u64) +
                (tc.minute_5.candles.len() as u64) +
                (tc.hour_1.candles.len() as u64) +
                (tc.day_1.candles.len() as u64)
        })
        .sum();

    let now = std::time::SystemTime
        ::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let fresh_tokens = cache
        .values()
        .filter(|tc| { now - tc.last_updated < OHLCV_UPDATE_INTERVAL_SECONDS })
        .count() as u64;

    (total_tokens, total_candles, fresh_tokens)
}

/// Force update OHLCV data for a specific token (for immediate use)
pub async fn force_update_token_ohlcv(token_mint: &str, pool_address: &str) {
    update_token_ohlcv(token_mint, pool_address).await;
}

/// Add any token to monitoring (even if not watched/traded)
pub async fn add_token_to_ohlcv_cache(token_mint: &str, pool_address: &str) {
    let mut monitor_list = TOKENS_TO_MONITOR_OHLCV.write().await;
    monitor_list.insert(token_mint.to_string(), pool_address.to_string());
    println!("📊 Added {} to OHLCV cache monitoring", &token_mint[..8]);
}
