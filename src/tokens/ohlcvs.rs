use crate::global::{is_debug_ohlcv_enabled, CACHE_OHLCVS_DIR};
/// OHLCV Data Collection and Caching System for ScreenerBot - 1-Minute Only
///
/// This module provides OHLCV (Open, High, Low, Close, Volume) data collection
/// from GeckoTerminal API with intelligent caching and background monitoring.
/// Optimized for 1-minute timeframe only for simplicity and performance.
///
/// ## Features:
/// - **Single Timeframe**: Only 1-minute candles for consistent analysis
/// - **Smart Caching**: File-based cache organized per mint and pool
///   - Structure: CACHE_OHLCVS_DIR/<mint>/<pool_address>/1m.json
/// - **Background Monitoring**: Continuous data collection for watched tokens
/// - **Pool Integration**: Uses best pools from pool service for data fetching
/// - **Data Validation**: Handles missing intervals and validates data integrity
/// - **Cleanup System**: Automatic removal of old data beyond retention period
///
/// ## Usage:
/// ```rust
/// // Initialize OHLCV service
/// let ohlcv_service = OhlcvService::new()?;
///
/// // Add token to watch list (from trader filtering)
/// ohlcv_service.add_to_watch_list("token_mint", true).await;
///
/// // Get OHLCV data (always 1-minute)
/// let data = ohlcv_service.get_ohlcv_data("token_mint", 100).await?;
/// ```
use crate::logger::{log, LogTag};
use crate::tokens::pool::get_pool_service;
use crate::tokens::PriceOptions;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Notify, RwLock};

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// GeckoTerminal API base URL
const GECKOTERMINAL_BASE_URL: &str = "https://api.geckoterminal.com/api/v2";

/// API version header value
const API_VERSION: &str = "20230302";

/// Rate limit delay between calls (2 seconds to be safe)
const API_RATE_LIMIT_DELAY_MS: u64 = 2000;

/// Maximum number of cached entries in memory to prevent unbounded growth
const MAX_MEMORY_CACHE_ENTRIES: usize = 500;

/// Cache directory for OHLCV data
const CACHE_DIR: &str = CACHE_OHLCVS_DIR;

/// Data retention period (6 hours - shorter since only 1m data)
const DATA_RETENTION_HOURS: i64 = 6;

/// Default limit for OHLCV data points
const DEFAULT_OHLCV_LIMIT: u32 = 100;

/// Maximum limit for OHLCV data points
const MAX_OHLCV_LIMIT: u32 = 500;

/// Background monitoring interval (30 seconds - more frequent for 1m data)
const MONITORING_INTERVAL_SECS: u64 = 30;

/// Cache file cleanup interval (30 minutes)
const CLEANUP_INTERVAL_SECS: u64 = 1800;

/// Solana network identifier for GeckoTerminal
const SOLANA_NETWORK: &str = "solana";

/// Cache expiration time for 1-minute data (2 minutes)
const CACHE_EXPIRY_MINUTES: i64 = 2;

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// OHLCV data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcvDataPoint {
    /// Timestamp (Unix seconds)
    pub timestamp: i64,
    /// Open price in USD
    pub open: f64,
    /// High price in USD
    pub high: f64,
    /// Low price in USD
    pub low: f64,
    /// Close price in USD
    pub close: f64,
    /// Volume in USD
    pub volume: f64,
}

/// Cached OHLCV data for a token (1-minute only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedOhlcvData {
    pub mint: String,
    pub pool_address: String,
    pub data_points: Vec<OhlcvDataPoint>,
    pub last_updated: DateTime<Utc>,
    pub last_timestamp: Option<i64>,
}

impl CachedOhlcvData {
    /// Check if cache is expired (older than 2 minutes for 1m data)
    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.last_updated;
        age.num_minutes() > CACHE_EXPIRY_MINUTES
    }

    /// Get cache file path: CACHE_OHLCVS_DIR/<mint>/<pool_address>/1m.json
    pub fn get_cache_path(&self) -> PathBuf {
        Path::new(CACHE_DIR)
            .join(&self.mint)
            .join(&self.pool_address)
            .join("1m.json")
    }
}

/// OHLCV data availability status
#[derive(Debug, Clone)]
pub struct DataAvailability {
    pub mint: String,
    pub has_cached_data: bool,
    pub has_pool: bool,
    pub pool_address: Option<String>,
    pub last_data_timestamp: Option<i64>,
    pub data_points_count: usize,
    pub is_fresh: bool,
    pub last_checked: DateTime<Utc>,
}

/// Watch list entry for OHLCV monitoring
#[derive(Debug, Clone)]
pub struct OhlcvWatchEntry {
    pub mint: String,
    pub is_open_position: bool,
    pub priority: i32,
    pub added_at: DateTime<Utc>,
    pub last_update: Option<DateTime<Utc>>,
    pub update_count: u64,
    pub pool_address: Option<String>,
}

/// GeckoTerminal API response structures
#[derive(Debug, Deserialize)]
struct GeckoTerminalResponse {
    data: GeckoTerminalData,
}

#[derive(Debug, Deserialize)]
struct GeckoTerminalData {
    id: String,
    #[serde(rename = "type")]
    data_type: String,
    attributes: GeckoTerminalAttributes,
}

#[derive(Debug, Deserialize)]
struct GeckoTerminalAttributes {
    ohlcv_list: Vec<Vec<f64>>, // [timestamp, open, high, low, close, volume]
}

// =============================================================================
// MAIN OHLCV SERVICE
// =============================================================================

/// OHLCV data collection and caching service (1-minute only)
#[derive(Clone)]
pub struct OhlcvService {
    /// HTTP client for API requests
    client: Client,
    /// In-memory cache for OHLCV data (key: mint)
    cache: Arc<RwLock<HashMap<String, CachedOhlcvData>>>,
    /// Watch list for background monitoring (key: mint)
    watch_list: Arc<RwLock<HashMap<String, OhlcvWatchEntry>>>,
    /// Rate limiting state
    last_api_call: Arc<RwLock<Option<Instant>>>,
    /// Service statistics
    stats: Arc<RwLock<OhlcvStats>>,
    /// Monitoring active flag
    monitoring_active: Arc<RwLock<bool>>,
}

/// Service statistics
#[derive(Debug, Clone, Default)]
pub struct OhlcvStats {
    pub total_api_calls: u64,
    pub successful_fetches: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub watched_tokens: usize,
    pub last_cleanup: Option<DateTime<Utc>>,
    pub data_points_cached: usize,
}

impl OhlcvService {
    /// Create new OHLCV service
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Create cache directory structure
        let cache_dir = Path::new(CACHE_DIR);
        if !cache_dir.exists() {
            fs::create_dir_all(cache_dir)?;
            log(
                LogTag::Ohlcv,
                "INIT",
                &format!("Created OHLCV cache directory: {}", CACHE_DIR),
            );
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("ScreenerBot/1.0")
            .build()?;

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "INIT_CLIENT",
                "🌐 HTTP client configured for 1-minute OHLCV data only",
            );
        }

        Ok(Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            watch_list: Arc::new(RwLock::new(HashMap::new())),
            last_api_call: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(OhlcvStats::default())),
            monitoring_active: Arc::new(RwLock::new(false)),
        })
    }

    /// Start background monitoring service
    pub async fn start_monitoring(&self, shutdown: Arc<Notify>) {
        let mut monitoring_active = self.monitoring_active.write().await;
        if *monitoring_active {
            log(LogTag::Ohlcv, "WARNING", "OHLCV monitoring already active");
            return;
        }
        *monitoring_active = true;
        drop(monitoring_active);

        log(
            LogTag::Ohlcv,
            "START",
            "🚀 Starting 1-minute OHLCV background monitoring service",
        );

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "MONITOR_CONFIG",
                &format!(
                    "📋 Monitor config - Interval: {}s, Cleanup: {}s, Rate limit: {}ms, Data retention: {}h",
                    MONITORING_INTERVAL_SECS,
                    CLEANUP_INTERVAL_SECS,
                    API_RATE_LIMIT_DELAY_MS,
                    DATA_RETENTION_HOURS
                )
            );
        }

        let cache = self.cache.clone();
        let watch_list = self.watch_list.clone();
        let stats = self.stats.clone();
        let monitoring_active = self.monitoring_active.clone();
        let client = self.client.clone();
        let last_api_call = self.last_api_call.clone();

        tokio::spawn(async move {
            let mut monitoring_interval =
                tokio::time::interval(Duration::from_secs(MONITORING_INTERVAL_SECS));
            let mut cleanup_interval =
                tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));

            loop {
                tokio::select! {
                    _ = monitoring_interval.tick() => {
                        if is_debug_ohlcv_enabled() {
                            log(LogTag::Ohlcv, "MONITOR_TICK", "⏰ 1m OHLCV monitoring tick starting");
                        }
                        if let Err(e) = Self::process_watch_list(
                            &client,
                            &cache,
                            &watch_list,
                            &stats,
                            &last_api_call
                        ).await {
                            log(LogTag::Ohlcv, "ERROR", &format!("Watch list processing failed: {}", e));
                        }
                        if is_debug_ohlcv_enabled() {
                            log(LogTag::Ohlcv, "MONITOR_TICK_DONE", "✅ 1m OHLCV monitoring tick completed");
                        }
                    }
                    _ = cleanup_interval.tick() => {
                        if is_debug_ohlcv_enabled() {
                            log(LogTag::Ohlcv, "CLEANUP_TICK", "🧹 OHLCV cleanup tick starting");
                        }
                        if let Err(e) = Self::cleanup_old_data(&cache, &stats).await {
                            log(LogTag::Ohlcv, "ERROR", &format!("Cleanup failed: {}", e));
                        }
                        if is_debug_ohlcv_enabled() {
                            log(LogTag::Ohlcv, "CLEANUP_TICK_DONE", "✅ OHLCV cleanup tick completed");
                        }
                    }
                    _ = shutdown.notified() => {
                        log(LogTag::Ohlcv, "SHUTDOWN", "🛑 OHLCV monitoring service shutting down");
                        break;
                    }
                }

                // Check if monitoring should continue
                {
                    let active = monitoring_active.read().await;
                    if !*active {
                        break;
                    }
                }
            }

            {
                let mut monitoring_active = monitoring_active.write().await;
                *monitoring_active = false;
            }

            log(
                LogTag::Ohlcv,
                "STOPPED",
                "✅ OHLCV monitoring service stopped",
            );
        });
    }

    /// Add token to watch list for OHLCV monitoring
    pub async fn add_to_watch_list(&self, mint: &str, is_open_position: bool) {
        let priority = if is_open_position { 100 } else { 50 };

        let mut watch_list = self.watch_list.write().await;

        if let Some(existing) = watch_list.get_mut(mint) {
            // Update existing entry
            existing.is_open_position = is_open_position;
            existing.priority = priority;
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "WATCH_UPDATE",
                    &format!("📊 Updated 1m OHLCV watch list for {}: priority={}", mint, priority),
                );
            }
        } else {
            // Add new entry
            watch_list.insert(
                mint.to_string(),
                OhlcvWatchEntry {
                    mint: mint.to_string(),
                    is_open_position,
                    priority,
                    added_at: Utc::now(),
                    last_update: None,
                    update_count: 0,
                    pool_address: None,
                },
            );

            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "WATCH_ADD_DETAIL",
                    &format!(
                        "📈 Added {} to 1m OHLCV watch list (priority: {}, open_position: {})",
                        mint, priority, is_open_position
                    )
                );
            }
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.watched_tokens = watch_list.len();
        }
    }

    /// Remove token from watch list
    pub async fn remove_from_watch_list(&self, mint: &str) {
        let mut watch_list = self.watch_list.write().await;
        if watch_list.remove(mint).is_some() {
            log(
                LogTag::Ohlcv,
                "WATCH_REMOVE",
                &format!("📉 Removed {} from 1m OHLCV watch list", mint),
            );

            // Update stats
            let mut stats = self.stats.write().await;
            stats.watched_tokens = watch_list.len();
        }
    }

    /// Check data availability for a token
    pub async fn check_data_availability(&self, mint: &str) -> DataAvailability {
        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "AVAILABILITY_CHECK",
                &format!("🔍 Checking 1m OHLCV data availability for {}", mint),
            );
        }

        // Check in-memory cache
        let cached_data = {
            let cache = self.cache.read().await;
            cache.get(mint).cloned()
        };

        let (has_cached_data, last_data_timestamp, data_points_count, is_fresh) =
            if let Some(data) = &cached_data {
                let is_fresh = !data.is_expired();
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "MEMORY_CACHE_CHECK",
                        &format!(
                            "💾 Memory cache found for {}: {} points, fresh: {}",
                            mint, data.data_points.len(), is_fresh
                        ),
                    );
                }
                (true, data.last_timestamp, data.data_points.len(), is_fresh)
            } else {
                // Check file cache
                if let Ok(file_data) = self.load_from_file_cache(mint).await {
                    let is_fresh = !file_data.is_expired();
                    if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "FILE_CACHE_CHECK",
                            &format!(
                                "📁 File cache found for {}: {} points, fresh: {}",
                                mint, file_data.data_points.len(), is_fresh
                            ),
                        );
                    }
                    (true, file_data.last_timestamp, file_data.data_points.len(), is_fresh)
                } else {
                    if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "NO_CACHE",
                            &format!("❌ No cache found for {}", mint),
                        );
                    }
                    (false, None, 0, false)
                }
            };

        // Check if token has a pool
        let pool_service = get_pool_service();
        let has_pool = pool_service.check_token_availability(mint).await;
        let pool_address = if has_pool {
            // Get best pool address
            if let Some(result) = pool_service
                .get_pool_price(mint, None, &PriceOptions::default())
                .await
            {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "POOL_FOUND",
                        &format!("🏊 Pool found for {}: {}", mint, result.pool_address),
                    );
                }
                Some(result.pool_address)
            } else {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "POOL_UNAVAILABLE",
                        &format!("⚠️ Pool service returned no price for {}", mint),
                    );
                }
                None
            }
        } else {
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "NO_POOL",
                    &format!("❌ No pool available for {}", mint),
                );
            }
            None
        };

        // Log final availability status in debug mode
        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "AVAILABILITY_RESULT",
                &format!(
                    "📊 1m OHLCV availability for {}: cached={}, pool={}, fresh={}, points={}",
                    mint, has_cached_data, has_pool, is_fresh, data_points_count
                ),
            );
        }

        DataAvailability {
            mint: mint.to_string(),
            has_cached_data,
            has_pool,
            pool_address,
            last_data_timestamp,
            data_points_count,
            is_fresh,
            last_checked: Utc::now(),
        }
    }

    /// Get 1-minute OHLCV data for a token
    pub async fn get_ohlcv_data(
        &self,
        mint: &str,
        limit: Option<u32>,
    ) -> Result<Vec<OhlcvDataPoint>, String> {
        let limit = limit.unwrap_or(DEFAULT_OHLCV_LIMIT).min(MAX_OHLCV_LIMIT);

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "DATA_REQUEST",
                &format!("📊 1m OHLCV data request: {} (limit: {})", mint, limit),
            );
        }

        // Check in-memory cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached_data) = cache.get(mint) {
                if !cached_data.is_expired() {
                    let mut stats = self.stats.write().await;
                    stats.cache_hits += 1;

                    if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "CACHE_HIT",
                            &format!("✅ Memory cache hit for {}: {} points", mint, cached_data.data_points.len()),
                        );
                    }

                    // Return most recent points up to limit
                    let mut points = cached_data.data_points.clone();
                    points.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Most recent first
                    points.truncate(limit as usize);
                    return Ok(points);
                }
            }
        }

        // Check file cache
        if let Ok(file_data) = self.load_from_file_cache(mint).await {
            if !file_data.is_expired() {
                // Load into memory cache
                {
                    let mut cache = self.cache.write().await;
                    cache.insert(mint.to_string(), file_data.clone());
                }

                let mut stats = self.stats.write().await;
                stats.cache_hits += 1;

                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "FILE_CACHE_HIT",
                        &format!("📁 File cache hit for {}: {} points", mint, file_data.data_points.len()),
                    );
                }

                let mut points = file_data.data_points;
                points.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                points.truncate(limit as usize);
                return Ok(points);
            }
        }

        // Cache miss - fetch from API
        {
            let mut stats = self.stats.write().await;
            stats.cache_misses += 1;
        }

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "CACHE_MISS",
                &format!("❌ Cache miss for {}, fetching 1m data from API", mint),
            );
        }

        // Get pool address for API call
        let pool_address = if let Some(availability) = self.get_pool_address_for_mint(mint).await {
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "POOL_RESOLVED",
                    &format!("🏊 Pool resolved for {}: {}", mint, availability),
                );
            }
            availability
        } else {
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "POOL_RESOLVE_FAILED",
                    &format!("❌ Failed to resolve pool for {}", mint),
                );
            }
            return Err(format!("No pool found for token {}", mint));
        };

        // Fetch from API
        match self.fetch_ohlcv_from_api(&pool_address, limit).await {
            Ok(data_points) => {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "API_SUCCESS",
                        &format!("✅ Fetched {} 1m OHLCV points for {} from API", data_points.len(), mint),
                    );
                }

                // Cache the data
                let cached_data = CachedOhlcvData {
                    mint: mint.to_string(),
                    pool_address,
                    data_points: data_points.clone(),
                    last_updated: Utc::now(),
                    last_timestamp: data_points.iter().map(|p| p.timestamp).max(),
                };

                // Save to memory cache with size limit protection
                {
                    let mut cache = self.cache.write().await;

                    // If cache is getting too large, remove oldest entries
                    if cache.len() >= MAX_MEMORY_CACHE_ENTRIES {
                        // Find oldest entry to remove
                        let oldest_key = cache
                            .iter()
                            .min_by_key(|(_, data)| data.last_updated)
                            .map(|(k, _)| k.clone());

                        if let Some(key) = oldest_key {
                            cache.remove(&key);
                            if is_debug_ohlcv_enabled() {
                                log(
                                    LogTag::Ohlcv,
                                    "CACHE_EVICT",
                                    &format!("🗑️ Evicted oldest cache entry: {}", key),
                                );
                            }
                        }
                    }

                    cache.insert(mint.to_string(), cached_data.clone());
                }

                // Save to file cache
                if let Err(e) = self.save_to_file_cache(&cached_data).await {
                    log(
                        LogTag::Ohlcv,
                        "WARNING",
                        &format!("Failed to save to file cache: {}", e),
                    );
                }

                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.successful_fetches += 1;
                    stats.data_points_cached += data_points.len();
                }

                Ok(data_points)
            }
            Err(e) => {
                log(
                    LogTag::Ohlcv,
                    "ERROR",
                    &format!("Failed to fetch 1m OHLCV data for {}: {}", mint, e),
                );
                Err(e)
            }
        }
    }

    /// Get service statistics
    pub async fn get_stats(&self) -> OhlcvStats {
        let stats = self.stats.read().await;
        let mut stats_copy = stats.clone();

        // Update real-time stats
        let watch_list = self.watch_list.read().await;
        stats_copy.watched_tokens = watch_list.len();

        stats_copy
    }

    // Private helper methods

    /// Get pool address for a mint
    async fn get_pool_address_for_mint(&self, mint: &str) -> Option<String> {
        let pool_service = get_pool_service();
        if let Some(result) = pool_service
            .get_pool_price(mint, None, &PriceOptions::default())
            .await
        {
            Some(result.pool_address)
        } else {
            None
        }
    }

    /// Fetch 1-minute OHLCV data from GeckoTerminal API
    async fn fetch_ohlcv_from_api(
        &self,
        pool_address: &str,
        limit: u32,
    ) -> Result<Vec<OhlcvDataPoint>, String> {
        // Rate limiting
        self.enforce_rate_limit().await;

        let url = format!(
            "{}/networks/{}/pools/{}/ohlcv/minute",
            GECKOTERMINAL_BASE_URL, SOLANA_NETWORK, pool_address
        );

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "API_CALL",
                &format!("🌐 1m OHLCV API call: {} (limit: {})", url, limit),
            );
        }

        let response = self
            .client
            .get(&url)
            .header("Accept", format!("application/json;version={}", API_VERSION))
            .query(&[
                ("aggregate", "1".to_string()),
                ("limit", limit.to_string()),
                ("currency", "usd".to_string()),
            ])
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        // Update API call stats
        {
            let mut stats = self.stats.write().await;
            stats.total_api_calls += 1;
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "API_ERROR",
                    &format!("❌ API error response: {} - {}", status, error_text),
                );
            }

            // Handle specific status codes
            match status.as_u16() {
                429 => {
                    // Rate limit exceeded - wait longer before next call
                    if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "RATE_LIMIT_HIT",
                            "⏳ Rate limit exceeded, waiting 10 seconds",
                        );
                    }
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    return Err("Rate limit exceeded".to_string());
                }
                404 => {
                    return Err(format!("Pool not found: {}", pool_address));
                }
                400 => {
                    return Err(format!("Bad request - invalid parameters: {}", error_text));
                }
                500..=599 => {
                    return Err(format!("Server error ({}): {}", status, error_text));
                }
                _ => {
                    return Err(format!("API error: {} - {}", status, error_text));
                }
            }
        }

        let gecko_response: GeckoTerminalResponse = response
            .json()
            .await
            .map_err(|e| format!("JSON parsing failed: {}", e))?;

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "API_RESPONSE",
                &format!(
                    "✅ GeckoTerminal response: type={}, id={}, {} OHLCV points",
                    gecko_response.data.data_type,
                    gecko_response.data.id,
                    gecko_response.data.attributes.ohlcv_list.len()
                ),
            );
        }

        let data_points: Result<Vec<OhlcvDataPoint>, String> = gecko_response
            .data
            .attributes
            .ohlcv_list
            .into_iter()
            .map(|ohlcv| {
                if ohlcv.len() != 6 {
                    return Err(format!(
                        "Invalid OHLCV data format: expected 6 values, got {}",
                        ohlcv.len()
                    ));
                }

                let timestamp = ohlcv[0] as i64;
                let open = ohlcv[1];
                let high = ohlcv[2];
                let low = ohlcv[3];
                let close = ohlcv[4];
                let volume = ohlcv[5];

                // Validate data integrity
                if timestamp <= 0 {
                    return Err(format!("Invalid timestamp: {}", timestamp));
                }

                if open <= 0.0 || high <= 0.0 || low <= 0.0 || close <= 0.0 {
                    return Err(format!(
                        "Invalid price data: open={}, high={}, low={}, close={}",
                        open, high, low, close
                    ));
                }

                if volume < 0.0 {
                    return Err(format!("Invalid volume: {}", volume));
                }

                if high < low {
                    return Err(format!(
                        "Invalid OHLC relationship: high ({}) < low ({})",
                        high, low
                    ));
                }

                if open > high || open < low || close > high || close < low {
                    return Err(format!(
                        "OHLC values out of range: open={}, high={}, low={}, close={}",
                        open, high, low, close
                    ));
                }

                if !open.is_finite()
                    || !high.is_finite()
                    || !low.is_finite()
                    || !close.is_finite()
                    || !volume.is_finite()
                {
                    return Err("Non-finite values in OHLCV data".to_string());
                }

                Ok(OhlcvDataPoint {
                    timestamp,
                    open,
                    high,
                    low,
                    close,
                    volume,
                })
            })
            .collect();

        data_points
    }

    /// Enforce API rate limiting
    async fn enforce_rate_limit(&self) {
        let mut last_call = self.last_api_call.write().await;

        if let Some(last_time) = *last_call {
            let elapsed = last_time.elapsed();
            let required_delay = Duration::from_millis(API_RATE_LIMIT_DELAY_MS);

            if elapsed < required_delay {
                let sleep_duration = required_delay - elapsed;
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "RATE_LIMIT",
                        &format!(
                            "⏳ Rate limiting: sleeping {:?} (elapsed: {:?}, required: {:?})",
                            sleep_duration, elapsed, required_delay
                        ),
                    );
                }
                tokio::time::sleep(sleep_duration).await;
            } else if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "RATE_LIMIT_OK",
                    &format!(
                        "✅ Rate limit OK: elapsed {:?} >= required {:?}",
                        elapsed, required_delay
                    ),
                );
            }
        } else if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "RATE_LIMIT_FIRST",
                "🆕 First API call, no rate limiting needed",
            );
        }

        *last_call = Some(Instant::now());
    }

    /// Load OHLCV data from file cache
    pub async fn load_from_file_cache(&self, mint: &str) -> Result<CachedOhlcvData, String> {
        // Get pool address for this mint
        let pool_address = if let Some(addr) = self.get_pool_address_for_mint(mint).await {
            addr
        } else {
            return Err("No pool found for mint".to_string());
        };

        let cache_path = Path::new(CACHE_DIR)
            .join(mint)
            .join(&pool_address)
            .join("1m.json");

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "FILE_CACHE_LOAD",
                &format!("📁 Loading 1m cache file: {}", cache_path.display()),
            );
        }

        if !cache_path.exists() {
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "FILE_CACHE_MISSING",
                    &format!("❌ Cache file not found: {}", cache_path.display()),
                );
            }
            return Err("Cache file not found".to_string());
        }

        let content = fs::read_to_string(&cache_path)
            .map_err(|e| format!("Failed to read cache file: {}", e))?;

        let cached_data: CachedOhlcvData = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse cache file: {}", e))?;

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "FILE_CACHE_LOADED",
                &format!(
                    "✅ File cache loaded: {} points, last_updated: {}, expired: {}",
                    cached_data.data_points.len(),
                    cached_data.last_updated.format("%H:%M:%S"),
                    cached_data.is_expired()
                ),
            );
        }

        Ok(cached_data)
    }

    /// Save OHLCV data to file cache
    async fn save_to_file_cache(&self, cached_data: &CachedOhlcvData) -> Result<(), String> {
        let cache_path = cached_data.get_cache_path();

        // Ensure directory exists
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(cached_data)
            .map_err(|e| format!("Failed to serialize cache data: {}", e))?;

        let content_len = content.len();

        // Atomic write: write to temporary file first, then rename
        let temp_path = cache_path.with_extension("json.tmp");

        fs::write(&temp_path, &content)
            .map_err(|e| format!("Failed to write temporary cache file: {}", e))?;

        fs::rename(&temp_path, &cache_path).map_err(|e| {
            // Clean up temp file on failure
            let _ = fs::remove_file(&temp_path);
            format!("Failed to rename cache file: {}", e)
        })?;

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "CACHE_SAVE",
                &format!(
                    "💾 Saved 1m cache file: {} ({} points, {:.1} KB)",
                    cache_path.display(),
                    cached_data.data_points.len(),
                    (content_len as f64) / 1024.0
                ),
            );
        }

        Ok(())
    }

    /// Process watch list for background monitoring
    async fn process_watch_list(
        client: &Client,
        cache: &Arc<RwLock<HashMap<String, CachedOhlcvData>>>,
        watch_list: &Arc<RwLock<HashMap<String, OhlcvWatchEntry>>>,
        stats: &Arc<RwLock<OhlcvStats>>,
        last_api_call: &Arc<RwLock<Option<Instant>>>,
    ) -> Result<(), String> {
        let tokens_to_update = {
            let watch_list = watch_list.read().await;
            if watch_list.is_empty() {
                return Ok(());
            }

            // Get priority tokens (open positions get priority)
            let mut tokens: Vec<_> = watch_list.values().cloned().collect();
            tokens.sort_by(|a, b| {
                b.priority
                    .cmp(&a.priority)
                    .then_with(|| a.last_update.cmp(&b.last_update))
            });

            // Limit concurrent updates to avoid API overload (more aggressive for 1m)
            tokens.into_iter().take(8).collect::<Vec<_>>()
        };

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "WATCH_PROCESS",
                &format!(
                    "🔄 Processing {} watched tokens for 1m OHLCV updates (total available: {})",
                    tokens_to_update.len(),
                    {
                        let watch_list_read = watch_list.read().await;
                        watch_list_read.len()
                    }
                ),
            );
        }

        for entry in tokens_to_update {
            let needs_update = {
                let cache = cache.read().await;
                if let Some(cached) = cache.get(&entry.mint) {
                    let expired = cached.is_expired();
                    if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "CACHE_CHECK",
                            &format!(
                                "📋 Cache check for {}: expired={}",
                                entry.mint, expired
                            ),
                        );
                    }
                    expired
                } else {
                    if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "NO_CACHE_ENTRY",
                            &format!("❌ No cache entry for {}", entry.mint),
                        );
                    }
                    true // No cache, definitely needs update
                }
            };

            if needs_update {
                // Get pool address
                let pool_address = if let Some(addr) = &entry.pool_address {
                    addr.clone()
                } else {
                    let pool_service = get_pool_service();
                    if let Some(result) = pool_service
                        .get_pool_price(&entry.mint, None, &PriceOptions::default())
                        .await
                    {
                        result.pool_address
                    } else {
                        if is_debug_ohlcv_enabled() {
                            log(
                                LogTag::Ohlcv,
                                "POOL_UNAVAILABLE",
                                &format!("⚠️ No pool available for {}", entry.mint),
                            );
                        }
                        continue;
                    }
                };

                // Create temporary service instance for this update
                let temp_service = OhlcvService {
                    client: client.clone(),
                    cache: cache.clone(),
                    watch_list: watch_list.clone(),
                    last_api_call: last_api_call.clone(),
                    stats: stats.clone(),
                    monitoring_active: Arc::new(RwLock::new(true)),
                };

                // Fetch new data
                match temp_service
                    .fetch_ohlcv_from_api(&pool_address, DEFAULT_OHLCV_LIMIT)
                    .await
                {
                    Ok(data_points) => {
                        // Cache the data
                        let cached_data = CachedOhlcvData {
                            mint: entry.mint.clone(),
                            pool_address: pool_address.clone(),
                            data_points,
                            last_updated: Utc::now(),
                            last_timestamp: None, // Will be calculated if needed
                        };

                        // Update memory cache
                        {
                            let mut cache = cache.write().await;
                            cache.insert(entry.mint.clone(), cached_data.clone());
                        }

                        // Save to file cache
                        if let Err(e) = temp_service.save_to_file_cache(&cached_data).await {
                            log(
                                LogTag::Ohlcv,
                                "WARNING",
                                &format!("Failed to save background fetch to cache: {}", e),
                            );
                        }

                        // Update stats
                        {
                            let mut stats = stats.write().await;
                            stats.successful_fetches += 1;
                            stats.data_points_cached += cached_data.data_points.len();
                        }

                        if is_debug_ohlcv_enabled() {
                            log(
                                LogTag::Ohlcv,
                                "BACKGROUND_UPDATE_SUCCESS",
                                &format!(
                                    "✅ Background updated {} with {} 1m points",
                                    entry.mint,
                                    cached_data.data_points.len()
                                ),
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Ohlcv,
                            "WARNING",
                            &format!("Background fetch failed for {}: {}", entry.mint, e),
                        );
                    }
                }

                // Small delay between API calls to be nice to the API
                tokio::time::sleep(Duration::from_millis(300)).await;
            }

            // Update watch list entry
            {
                let mut watch_list = watch_list.write().await;
                if let Some(entry) = watch_list.get_mut(&entry.mint) {
                    entry.last_update = Some(Utc::now());
                    entry.update_count += 1;
                    if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "WATCH_UPDATE_COMPLETE",
                            &format!("📊 Updated watch entry for {} (count: {})", entry.mint, entry.update_count),
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Clean up old cached data
    async fn cleanup_old_data(
        cache: &Arc<RwLock<HashMap<String, CachedOhlcvData>>>,
        stats: &Arc<RwLock<OhlcvStats>>,
    ) -> Result<(), String> {
        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "CLEANUP_START",
                "🧹 Starting 1m OHLCV data cleanup",
            );
        }

        let cutoff_time = Utc::now() - ChronoDuration::hours(DATA_RETENTION_HOURS);
        let mut cleaned_memory = 0;
        let mut cleaned_files = 0;

        // Clean memory cache
        {
            let mut cache = cache.write().await;
            let initial_count = cache.len();
            cache.retain(|_, cached_data| cached_data.last_updated > cutoff_time);
            cleaned_memory = initial_count - cache.len();
            if is_debug_ohlcv_enabled() && cleaned_memory > 0 {
                log(
                    LogTag::Ohlcv,
                    "CLEANUP_MEMORY",
                    &format!(
                        "🗑️ Cleaned {} memory cache entries (kept {})",
                        cleaned_memory,
                        cache.len()
                    ),
                );
            }
        }

        // Clean file cache
        let cache_dir = Path::new(CACHE_DIR);
        if cache_dir.exists() {
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "CLEANUP_FILES_START",
                    &format!("🧹 Starting file cleanup in {}", cache_dir.display()),
                );
            }
            cleaned_files = Self::cleanup_cache_files(cache_dir, cutoff_time)?;
            if is_debug_ohlcv_enabled() && cleaned_files > 0 {
                log(
                    LogTag::Ohlcv,
                    "CLEANUP_FILES_DONE",
                    &format!("🗂️ Cleaned {} cache files", cleaned_files),
                );
            }
        }

        // Update stats
        {
            let mut stats = stats.write().await;
            stats.last_cleanup = Some(Utc::now());
        }

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "CLEANUP_COMPLETE",
                &format!(
                    "✅ Cleanup complete: {} memory entries, {} files removed",
                    cleaned_memory, cleaned_files
                ),
            );
        }

        Ok(())
    }

    /// Recursively clean cache files older than cutoff time
    fn cleanup_cache_files(dir: &Path, cutoff_time: DateTime<Utc>) -> Result<usize, String> {
        let mut cleaned_count = 0;

        for entry in fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))? {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively clean subdirectories
                cleaned_count += Self::cleanup_cache_files(&path, cutoff_time)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                // Check file modification time
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        let modified_dt: DateTime<Utc> = modified.into();
                        if modified_dt < cutoff_time {
                            if let Err(e) = fs::remove_file(&path) {
                                log(
                                    LogTag::Ohlcv,
                                    "WARNING",
                                    &format!("Failed to remove old cache file {}: {}", path.display(), e),
                                );
                            } else {
                                cleaned_count += 1;
                                if is_debug_ohlcv_enabled() {
                                    log(
                                        LogTag::Ohlcv,
                                        "FILE_REMOVED",
                                        &format!("🗑️ Removed old cache file: {}", path.display()),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(cleaned_count)
    }
}

// =============================================================================
// GLOBAL OHLCV SERVICE INSTANCE
// =============================================================================

use std::sync::LazyLock;
use tokio::sync::RwLock as TokioRwLock;

// Use LazyLock for safe global state (Rust 1.70+)
static GLOBAL_OHLCV_SERVICE: LazyLock<TokioRwLock<Option<OhlcvService>>> =
    LazyLock::new(|| TokioRwLock::new(None));

/// Initialize global OHLCV service
pub async fn init_ohlcv_service() -> Result<(), Box<dyn std::error::Error>> {
    let mut service_guard = GLOBAL_OHLCV_SERVICE.write().await;

    if service_guard.is_some() {
        // Already initialized
        return Ok(());
    }

    match OhlcvService::new() {
        Ok(service) => {
            *service_guard = Some(service);
            log(LogTag::Ohlcv, "INIT", "✅ Global 1m OHLCV service initialized");
            Ok(())
        }
        Err(e) => {
            log(
                LogTag::Ohlcv,
                "ERROR",
                &format!("❌ Failed to initialize OHLCV service: {}", e),
            );
            Err(e)
        }
    }
}

/// Get a cloned OHLCV service for async operations
pub async fn get_ohlcv_service_clone() -> Result<OhlcvService, String> {
    let service_guard = GLOBAL_OHLCV_SERVICE.read().await;
    match service_guard.as_ref() {
        Some(service) => {
            // Since OhlcvService has Arc<> fields, cloning is relatively cheap
            Ok(service.clone())
        }
        None => Err("OHLCV service not initialized - call init_ohlcv_service() first".to_string()),
    }
}

// =============================================================================
// PUBLIC HELPER FUNCTIONS
// =============================================================================

/// Start OHLCV background monitoring task
pub async fn start_ohlcv_monitoring(
    shutdown: Arc<Notify>,
) -> Result<tokio::task::JoinHandle<()>, String> {
    init_ohlcv_service()
        .await
        .map_err(|e| format!("Failed to initialize OHLCV service: {}", e))?;

    // Get cloned service for async operations
    let service = get_ohlcv_service_clone().await?;

    // Start monitoring
    service.start_monitoring(shutdown.clone()).await;

    let handle = tokio::spawn(async move {
        log(
            LogTag::Ohlcv,
            "TASK_START",
            "🚀 1m OHLCV monitoring task started",
        );
        shutdown.notified().await;
        log(LogTag::Ohlcv, "TASK_END", "✅ 1m OHLCV monitoring task ended");
    });

    Ok(handle)
}

/// Sync watch list with trader tokens (called from trader)
pub async fn sync_watch_list_with_trader(
    _shutdown: Option<std::sync::Arc<Notify>>,
) -> Result<(), String> {
    // For simplicity, we'll rely on manual addition via add_to_watch_list
    // when positions are opened/closed
    if is_debug_ohlcv_enabled() {
        log(
            LogTag::Ohlcv,
            "SYNC_SIMPLE",
            "📊 1m OHLCV watch list sync - using manual position-based management",
        );
    }
    Ok(())
}

/// Check if OHLCV data is available for trading decisions
pub async fn is_ohlcv_data_available(mint: &str) -> bool {
    let service = match get_ohlcv_service_clone().await {
        Ok(service) => service,
        Err(_) => {
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "ERROR",
                    "OHLCV service not available for availability check",
                );
            }
            return false;
        }
    };

    let availability = service.check_data_availability(mint).await;
    let is_available = availability.has_cached_data && availability.is_fresh;

    if is_debug_ohlcv_enabled() {
        log(
            LogTag::Ohlcv,
            "AVAILABILITY_CHECK",
            &format!("📊 1m OHLCV availability check for {}: result={}", mint, is_available),
        );
    }

    is_available
}

/// Get latest 1-minute OHLCV data for analysis (convenience function)
pub async fn get_latest_ohlcv(
    mint: &str,
    limit: u32,
) -> Result<Vec<OhlcvDataPoint>, String> {
    if is_debug_ohlcv_enabled() {
        log(
            LogTag::Ohlcv,
            "GET_LATEST",
            &format!("📈 Getting latest 1m OHLCV data for {} (limit: {})", mint, limit),
        );
    }

    let service = get_ohlcv_service_clone().await?;
    let result = service.get_ohlcv_data(mint, Some(limit)).await;

    if is_debug_ohlcv_enabled() {
        match &result {
            Ok(data) => log(
                LogTag::Ohlcv,
                "GET_LATEST_SUCCESS",
                &format!("✅ Retrieved {} 1m OHLCV points for {}", data.len(), mint),
            ),
            Err(e) => log(
                LogTag::Ohlcv,
                "GET_LATEST_ERROR",
                &format!("❌ Failed to get 1m OHLCV data for {}: {}", mint, e),
            ),
        }
    }

    result
}
