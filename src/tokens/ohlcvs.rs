/// OHLCV Data Collection and Caching System for ScreenerBot
///
/// This module provides comprehensive OHLCV (Open, High, Low, Close, Volume) data collection
/// from GeckoTerminal API with intelligent caching and background monitoring.
///
/// ## Features:
/// - **Multi-timeframe Support**: minute(1,5,15), hour(1,4,12), day(1) aggregations
/// - **Smart Caching**: File-based cache organized by mint/timeframe in .cache_ohlcvs/
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
/// // Check data availability
/// let availability = ohlcv_service.check_data_availability("token_mint", &Timeframe::Hour1).await;
///
/// // Get OHLCV data
/// let data = ohlcv_service.get_ohlcv_data("token_mint", &Timeframe::Hour1, 100).await?;
/// ```

use crate::logger::{ log, LogTag };
use crate::global::is_debug_ohlcv_enabled;
use crate::tokens::pool::{ get_pool_service };
use crate::tokens::price_service::{ get_priority_tokens_safe };
use tokio::sync::{ RwLock, Notify };
use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use std::path::{ Path, PathBuf };
use std::fs;
use std::time::{ Duration, Instant };
use chrono::{ DateTime, Utc, Duration as ChronoDuration };
use serde::{ Serialize, Deserialize };
use reqwest::Client;

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// GeckoTerminal API base URL
const GECKOTERMINAL_BASE_URL: &str = "https://api.geckoterminal.com/api/v2";

/// API version header value
const API_VERSION: &str = "20230302";

/// Rate limit: 30 calls per minute
const API_RATE_LIMIT_PER_MINUTE: u32 = 30;

/// Rate limit delay between calls (2 seconds to be safe)
const API_RATE_LIMIT_DELAY_MS: u64 = 2000;

/// Cache directory for OHLCV data
const CACHE_DIR: &str = ".cache_ohlcvs";

/// Data retention period (24 hours)
const DATA_RETENTION_HOURS: i64 = 24;

/// Default limit for OHLCV data points
const DEFAULT_OHLCV_LIMIT: u32 = 100;

/// Maximum limit for OHLCV data points
const MAX_OHLCV_LIMIT: u32 = 1000;

/// Background monitoring interval (1 minute)
const MONITORING_INTERVAL_SECS: u64 = 60;

/// Cache file cleanup interval (1 hour)
const CLEANUP_INTERVAL_SECS: u64 = 3600;

/// Solana network identifier for GeckoTerminal
const SOLANA_NETWORK: &str = "solana";

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Supported timeframes with aggregation values
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Timeframe {
    /// 1-minute candles
    Minute1,
    /// 5-minute candles
    Minute5,
    /// 15-minute candles
    Minute15,
    /// 1-hour candles
    Hour1,
    /// 4-hour candles
    Hour4,
    /// 12-hour candles
    Hour12,
    /// 1-day candles
    Day1,
}

impl Timeframe {
    /// Get the GeckoTerminal API timeframe and aggregate parameters
    pub fn get_api_params(&self) -> (&'static str, u32) {
        match self {
            Timeframe::Minute1 => ("minute", 1),
            Timeframe::Minute5 => ("minute", 5),
            Timeframe::Minute15 => ("minute", 15),
            Timeframe::Hour1 => ("hour", 1),
            Timeframe::Hour4 => ("hour", 4),
            Timeframe::Hour12 => ("hour", 12),
            Timeframe::Day1 => ("day", 1),
        }
    }

    /// Get cache subdirectory name
    pub fn get_cache_dir(&self) -> &'static str {
        match self {
            Timeframe::Minute1 => "1m",
            Timeframe::Minute5 => "5m",
            Timeframe::Minute15 => "15m",
            Timeframe::Hour1 => "1h",
            Timeframe::Hour4 => "4h",
            Timeframe::Hour12 => "12h",
            Timeframe::Day1 => "1d",
        }
    }

    /// Get all available timeframes
    pub fn all() -> Vec<Timeframe> {
        vec![
            Timeframe::Minute1,
            Timeframe::Minute5,
            Timeframe::Minute15,
            Timeframe::Hour1,
            Timeframe::Hour4,
            Timeframe::Hour12,
            Timeframe::Day1
        ]
    }
}

impl std::fmt::Display for Timeframe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_cache_dir())
    }
}

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

/// Cached OHLCV data for a token/timeframe combination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedOhlcvData {
    pub mint: String,
    pub timeframe: Timeframe,
    pub pool_address: String,
    pub data_points: Vec<OhlcvDataPoint>,
    pub last_updated: DateTime<Utc>,
    pub last_timestamp: Option<i64>,
}

impl CachedOhlcvData {
    /// Check if cache is expired (older than 5 minutes for real-time timeframes)
    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.last_updated;
        match self.timeframe {
            Timeframe::Minute1 | Timeframe::Minute5 => age.num_minutes() > 5,
            Timeframe::Minute15 | Timeframe::Hour1 => age.num_minutes() > 15,
            _ => age.num_hours() > 1,
        }
    }

    /// Get cache file path
    pub fn get_cache_path(&self) -> PathBuf {
        // Safe mint prefix extraction (first 8 chars or entire mint if shorter)
        let mint_prefix = if self.mint.len() >= 8 { &self.mint[..8] } else { &self.mint };

        let cache_dir = Path::new(CACHE_DIR)
            .join(mint_prefix) // First 8 chars of mint for organization
            .join(self.timeframe.get_cache_dir());

        cache_dir.join(format!("{}.json", self.mint))
    }
}

/// OHLCV data availability status
#[derive(Debug, Clone)]
pub struct DataAvailability {
    pub mint: String,
    pub timeframe: Timeframe,
    pub has_cached_data: bool,
    pub has_pool: bool,
    pub pool_address: Option<String>,
    pub last_data_timestamp: Option<i64>,
    pub data_points_count: usize,
    pub is_fresh: bool, // Data is less than expected interval old
    pub last_checked: DateTime<Utc>,
}

/// Watch list entry for OHLCV monitoring
#[derive(Debug, Clone)]
pub struct OhlcvWatchEntry {
    pub mint: String,
    pub is_open_position: bool,
    pub priority: i32,
    pub timeframes: HashSet<Timeframe>,
    pub added_at: DateTime<Utc>,
    pub last_update: Option<DateTime<Utc>>,
    pub update_count: u64,
    pub pool_address: Option<String>,
}

/// GeckoTerminal API response structures
#[derive(Debug, Deserialize)]
struct GeckoTerminalResponse {
    data: GeckoTerminalData,
    meta: Option<GeckoTerminalMeta>,
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

#[derive(Debug, Deserialize)]
struct GeckoTerminalMeta {
    base: Option<GeckoTerminalTokenInfo>,
    quote: Option<GeckoTerminalTokenInfo>,
}

#[derive(Debug, Deserialize)]
struct GeckoTerminalTokenInfo {
    address: String,
    name: String,
    symbol: String,
    coingecko_coin_id: Option<String>,
}

// =============================================================================
// MAIN OHLCV SERVICE
// =============================================================================

/// OHLCV data collection and caching service
pub struct OhlcvService {
    /// HTTP client for API requests
    client: Client,
    /// In-memory cache for OHLCV data
    cache: Arc<RwLock<HashMap<String, CachedOhlcvData>>>, // key: mint_timeframe
    /// Watch list for background monitoring
    watch_list: Arc<RwLock<HashMap<String, OhlcvWatchEntry>>>, // key: mint
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
    pub cached_timeframes: usize,
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
            log(LogTag::Ohlcv, "INIT", &format!("Created OHLCV cache directory: {}", CACHE_DIR));
        }

        // Create subdirectories for each timeframe
        for timeframe in Timeframe::all() {
            let timeframe_dir = cache_dir.join(timeframe.get_cache_dir());
            if !timeframe_dir.exists() {
                fs::create_dir_all(&timeframe_dir)?;
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "INIT_DIR",
                        &format!("📁 Created timeframe directory: {}", timeframe_dir.display())
                    );
                }
            }
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("ScreenerBot/1.0")
            .build()?;

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "INIT_CLIENT",
                "🌐 HTTP client configured with 30s timeout and custom user-agent"
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

        log(LogTag::Ohlcv, "START", "🚀 Starting OHLCV background monitoring service");

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
            let mut monitoring_interval = tokio::time::interval(
                Duration::from_secs(MONITORING_INTERVAL_SECS)
            );
            let mut cleanup_interval = tokio::time::interval(
                Duration::from_secs(CLEANUP_INTERVAL_SECS)
            );

            loop {
                tokio::select! {
                    _ = monitoring_interval.tick() => {
                        if is_debug_ohlcv_enabled() {
                            log(LogTag::Ohlcv, "MONITOR_TICK", "⏰ Background monitoring tick starting");
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
                            log(LogTag::Ohlcv, "MONITOR_TICK_DONE", "✅ Background monitoring tick completed");
                        }
                    }
                    _ = cleanup_interval.tick() => {
                        if is_debug_ohlcv_enabled() {
                            log(LogTag::Ohlcv, "CLEANUP_TICK", "🧹 Background cleanup tick starting");
                        }
                        if let Err(e) = Self::cleanup_old_data(&cache, &stats).await {
                            log(LogTag::Ohlcv, "ERROR", &format!("Cleanup failed: {}", e));
                        }
                        if is_debug_ohlcv_enabled() {
                            log(LogTag::Ohlcv, "CLEANUP_TICK_DONE", "✅ Background cleanup tick completed");
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

            log(LogTag::Ohlcv, "STOPPED", "✅ OHLCV monitoring service stopped");
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
                    &format!("📊 Updated watch list for {}: priority={}", mint, priority)
                );
            }
        } else {
            // Add new entry
            let timeframes = if is_open_position {
                // Open positions get all timeframes
                Timeframe::all().into_iter().collect()
            } else {
                // Regular tokens get essential timeframes
                vec![Timeframe::Minute15, Timeframe::Hour1, Timeframe::Hour4].into_iter().collect()
            };

            watch_list.insert(mint.to_string(), OhlcvWatchEntry {
                mint: mint.to_string(),
                is_open_position,
                priority,
                timeframes,
                added_at: Utc::now(),
                last_update: None,
                update_count: 0,
                pool_address: None,
            });

            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "WATCH_ADD_DETAIL",
                    &format!(
                        "📈 Added {} to OHLCV watch list (priority: {}, open_position: {}, timeframes: {})",
                        mint,
                        priority,
                        is_open_position,
                        if is_open_position {
                            "ALL"
                        } else {
                            "Essential (15m, 1h, 4h)"
                        }
                    )
                );
            }

            log(
                LogTag::Ohlcv,
                "WATCH_ADD",
                &format!(
                    "📈 Added {} to OHLCV watch list (priority: {}, open_position: {})",
                    mint,
                    priority,
                    is_open_position
                )
            );
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
                &format!("📉 Removed {} from OHLCV watch list", mint)
            );

            // Update stats
            let mut stats = self.stats.write().await;
            stats.watched_tokens = watch_list.len();
        }
    }

    /// Check data availability for a token/timeframe
    pub async fn check_data_availability(
        &self,
        mint: &str,
        timeframe: &Timeframe
    ) -> DataAvailability {
        let cache_key = format!("{}_{}", mint, timeframe.get_cache_dir());

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "AVAILABILITY_CHECK",
                &format!("🔍 Checking data availability for {} {}", mint, timeframe)
            );
        }

        // Check in-memory cache
        let cached_data = {
            let cache = self.cache.read().await;
            cache.get(&cache_key).cloned()
        };

        let (has_cached_data, last_data_timestamp, data_points_count, is_fresh) = if
            let Some(data) = &cached_data
        {
            let is_fresh = !data.is_expired();
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "MEMORY_CACHE_CHECK",
                    &format!(
                        "💾 Memory cache found for {} {}: {} points, fresh: {}",
                        mint,
                        timeframe,
                        data.data_points.len(),
                        is_fresh
                    )
                );
            }
            (true, data.last_timestamp, data.data_points.len(), is_fresh)
        } else {
            // Check file cache
            if let Ok(file_data) = self.load_from_file_cache(mint, timeframe).await {
                let is_fresh = !file_data.is_expired();
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "FILE_CACHE_CHECK",
                        &format!(
                            "📁 File cache found for {} {}: {} points, fresh: {}",
                            mint,
                            timeframe,
                            file_data.data_points.len(),
                            is_fresh
                        )
                    );
                }
                (true, file_data.last_timestamp, file_data.data_points.len(), is_fresh)
            } else {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "NO_CACHE",
                        &format!("❌ No cache found for {} {}", mint, timeframe)
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
            if let Some(result) = pool_service.get_pool_price(mint, None).await {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "POOL_FOUND",
                        &format!("🏊 Pool found for {}: {}", mint, result.pool_address)
                    );
                }
                Some(result.pool_address)
            } else {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "POOL_UNAVAILABLE",
                        &format!("⚠️ Pool service returned no price for {}", mint)
                    );
                }
                None
            }
        } else {
            if is_debug_ohlcv_enabled() {
                log(LogTag::Ohlcv, "NO_POOL", &format!("❌ No pool available for {}", mint));
            }
            None
        };

        // Log final availability status in debug mode
        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "AVAILABILITY_RESULT",
                &format!(
                    "📊 Availability result for {} {}: cached={}, pool={}, fresh={}, points={}",
                    mint,
                    timeframe,
                    has_cached_data,
                    has_pool,
                    is_fresh,
                    data_points_count
                )
            );
        }

        DataAvailability {
            mint: mint.to_string(),
            timeframe: timeframe.clone(),
            has_cached_data,
            has_pool,
            pool_address,
            last_data_timestamp,
            data_points_count,
            is_fresh,
            last_checked: Utc::now(),
        }
    }

    /// Get OHLCV data for a token/timeframe
    pub async fn get_ohlcv_data(
        &self,
        mint: &str,
        timeframe: &Timeframe,
        limit: Option<u32>
    ) -> Result<Vec<OhlcvDataPoint>, String> {
        let limit = limit.unwrap_or(DEFAULT_OHLCV_LIMIT).min(MAX_OHLCV_LIMIT);
        let cache_key = format!("{}_{}", mint, timeframe.get_cache_dir());

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "DATA_REQUEST",
                &format!("📊 OHLCV data request: {} {} (limit: {})", mint, timeframe, limit)
            );
        }

        // Check in-memory cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached_data) = cache.get(&cache_key) {
                if !cached_data.is_expired() {
                    let mut stats = self.stats.write().await;
                    stats.cache_hits += 1;

                    if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "CACHE_HIT",
                            &format!(
                                "💾 Cache hit for {} {}: {} points",
                                mint,
                                timeframe,
                                cached_data.data_points.len()
                            )
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
        if let Ok(file_data) = self.load_from_file_cache(mint, timeframe).await {
            if !file_data.is_expired() {
                // Load into memory cache
                {
                    let mut cache = self.cache.write().await;
                    cache.insert(cache_key.clone(), file_data.clone());
                }

                let mut stats = self.stats.write().await;
                stats.cache_hits += 1;

                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "FILE_CACHE_HIT",
                        &format!(
                            "📁 File cache hit for {} {}: {} points",
                            mint,
                            timeframe,
                            file_data.data_points.len()
                        )
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
                &format!("❌ Cache miss for {} {}, fetching from API", mint, timeframe)
            );
        }

        // Get pool address for API call
        let pool_address = if let Some(availability) = self.get_pool_address_for_mint(mint).await {
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "POOL_RESOLVED",
                    &format!("🏊 Pool resolved for {}: {}", mint, availability)
                );
            }
            availability
        } else {
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "POOL_RESOLVE_FAILED",
                    &format!("❌ Failed to resolve pool for {}", mint)
                );
            }
            return Err(format!("No pool found for token {}", mint));
        };

        // Fetch from API
        match self.fetch_ohlcv_from_api(&pool_address, timeframe, limit).await {
            Ok(data_points) => {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "API_SUCCESS",
                        &format!(
                            "✅ Fetched {} OHLCV points for {} {} from API",
                            data_points.len(),
                            mint,
                            timeframe
                        )
                    );
                }

                // Cache the data
                let cached_data = CachedOhlcvData {
                    mint: mint.to_string(),
                    timeframe: timeframe.clone(),
                    pool_address,
                    data_points: data_points.clone(),
                    last_updated: Utc::now(),
                    last_timestamp: data_points
                        .iter()
                        .map(|p| p.timestamp)
                        .max(),
                };

                // Save to memory cache
                {
                    let mut cache = self.cache.write().await;
                    cache.insert(cache_key, cached_data.clone());
                }

                // Save to file cache
                if let Err(e) = self.save_to_file_cache(&cached_data).await {
                    log(LogTag::Ohlcv, "WARNING", &format!("Failed to save to file cache: {}", e));
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
                    &format!("Failed to fetch OHLCV data for {} {}: {}", mint, timeframe, e)
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

        let cache = self.cache.read().await;
        stats_copy.cached_timeframes = cache.len();

        stats_copy
    }

    // Private helper methods

    /// Get pool address for a mint
    async fn get_pool_address_for_mint(&self, mint: &str) -> Option<String> {
        let pool_service = get_pool_service();
        if let Some(result) = pool_service.get_pool_price(mint, None).await {
            Some(result.pool_address)
        } else {
            None
        }
    }

    /// Fetch OHLCV data from GeckoTerminal API
    async fn fetch_ohlcv_from_api(
        &self,
        pool_address: &str,
        timeframe: &Timeframe,
        limit: u32
    ) -> Result<Vec<OhlcvDataPoint>, String> {
        // Rate limiting
        self.enforce_rate_limit().await;

        let (timeframe_str, aggregate) = timeframe.get_api_params();

        let url = format!(
            "{}/networks/{}/pools/{}/ohlcv/{}",
            GECKOTERMINAL_BASE_URL,
            SOLANA_NETWORK,
            pool_address,
            timeframe_str
        );

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "API_CALL",
                &format!("🌐 API call: {} (aggregate: {}, limit: {})", url, aggregate, limit)
            );
        }

        let response = self.client
            .get(&url)
            .header("Accept", format!("application/json;version={}", API_VERSION))
            .query(
                &[
                    ("aggregate", aggregate.to_string()),
                    ("limit", limit.to_string()),
                    ("currency", "usd".to_string()),
                ]
            )
            .send().await
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
                    &format!("❌ API error response: {} - {}", status, error_text)
                );
            }
            return Err(format!("API error: {} - {}", status, error_text));
        }

        let gecko_response: GeckoTerminalResponse = response
            .json().await
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
                )
            );
        }

        let data_points: Result<
            Vec<OhlcvDataPoint>,
            String
        > = gecko_response.data.attributes.ohlcv_list
            .into_iter()
            .map(|ohlcv| {
                if ohlcv.len() != 6 {
                    return Err(
                        format!("Invalid OHLCV data format: expected 6 values, got {}", ohlcv.len())
                    );
                }

                Ok(OhlcvDataPoint {
                    timestamp: ohlcv[0] as i64,
                    open: ohlcv[1],
                    high: ohlcv[2],
                    low: ohlcv[3],
                    close: ohlcv[4],
                    volume: ohlcv[5],
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
                            "⏳ Rate limiting: sleeping for {:?} (elapsed: {:?}, required: {:?})",
                            sleep_duration,
                            elapsed,
                            required_delay
                        )
                    );
                }
                tokio::time::sleep(sleep_duration).await;
            } else if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "RATE_LIMIT_OK",
                    &format!(
                        "✅ Rate limit OK: elapsed {:?} >= required {:?}",
                        elapsed,
                        required_delay
                    )
                );
            }
        } else if is_debug_ohlcv_enabled() {
            log(LogTag::Ohlcv, "RATE_LIMIT_FIRST", "🆕 First API call, no rate limiting needed");
        }

        *last_call = Some(Instant::now());
    }

    /// Load OHLCV data from file cache (public for testing)
    pub async fn load_from_file_cache(
        &self,
        mint: &str,
        timeframe: &Timeframe
    ) -> Result<CachedOhlcvData, String> {
        // Safe mint prefix extraction (first 8 chars or entire mint if shorter)
        let mint_prefix = if mint.len() >= 8 { &mint[..8] } else { mint };

        let cache_path = Path::new(CACHE_DIR)
            .join(mint_prefix)
            .join(timeframe.get_cache_dir())
            .join(format!("{}.json", mint));

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "FILE_CACHE_LOAD",
                &format!("📁 Attempting to load file cache: {}", cache_path.display())
            );
        }

        if !cache_path.exists() {
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "FILE_CACHE_MISSING",
                    &format!("❌ Cache file not found: {}", cache_path.display())
                );
            }
            return Err("Cache file not found".to_string());
        }

        let content = fs
            ::read_to_string(&cache_path)
            .map_err(|e| format!("Failed to read cache file: {}", e))?;

        let cached_data: CachedOhlcvData = serde_json
            ::from_str(&content)
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
                )
            );
        }

        Ok(cached_data)
    }

    /// Save OHLCV data to file cache
    async fn save_to_file_cache(&self, cached_data: &CachedOhlcvData) -> Result<(), String> {
        let cache_path = cached_data.get_cache_path();

        // Ensure directory exists
        if let Some(parent) = cache_path.parent() {
            fs
                ::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        }

        let content = serde_json
            ::to_string_pretty(cached_data)
            .map_err(|e| format!("Failed to serialize cache data: {}", e))?;

        let content_len = content.len();

        fs::write(&cache_path, content).map_err(|e| format!("Failed to write cache file: {}", e))?;

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "CACHE_SAVE",
                &format!(
                    "💾 Saved cache file: {} ({} points, {:.1} KB)",
                    cache_path.display(),
                    cached_data.data_points.len(),
                    (content_len as f64) / 1024.0
                )
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
        last_api_call: &Arc<RwLock<Option<Instant>>>
    ) -> Result<(), String> {
        let tokens_to_update = {
            let watch_list = watch_list.read().await;
            if watch_list.is_empty() {
                return Ok(());
            }

            // Get priority tokens (open positions get priority)
            let mut tokens: Vec<_> = watch_list.values().cloned().collect();
            tokens.sort_by(|a, b| {
                b.priority.cmp(&a.priority).then_with(|| a.last_update.cmp(&b.last_update))
            });

            // Limit concurrent updates to avoid API overload
            tokens.into_iter().take(5).collect::<Vec<_>>()
        };

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "WATCH_PROCESS",
                &format!(
                    "🔄 Processing {} watched tokens for OHLCV updates (total available: {})",
                    tokens_to_update.len(),
                    {
                        let watch_list_read = watch_list.read().await;
                        watch_list_read.len()
                    }
                )
            );
        }

        for entry in tokens_to_update {
            // Check each timeframe for this token
            for timeframe in &entry.timeframes {
                let cache_key = format!("{}_{}", entry.mint, timeframe.get_cache_dir());

                let needs_update = {
                    let cache = cache.read().await;
                    if let Some(cached) = cache.get(&cache_key) {
                        let expired = cached.is_expired();
                        if is_debug_ohlcv_enabled() {
                            log(
                                LogTag::Ohlcv,
                                "CACHE_CHECK",
                                &format!(
                                    "📊 Cache check for {} {}: expired={}, last_updated={}",
                                    entry.mint,
                                    timeframe,
                                    expired,
                                    cached.last_updated.format("%H:%M:%S")
                                )
                            );
                        }
                        expired
                    } else {
                        if is_debug_ohlcv_enabled() {
                            log(
                                LogTag::Ohlcv,
                                "NO_CACHE_ENTRY",
                                &format!(
                                    "❌ No cache entry for {} {}, update needed",
                                    entry.mint,
                                    timeframe
                                )
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
                        if let Some(result) = pool_service.get_pool_price(&entry.mint, None).await {
                            result.pool_address
                        } else {
                            if is_debug_ohlcv_enabled() {
                                log(
                                    LogTag::Ohlcv,
                                    "NO_POOL",
                                    &format!(
                                        "⚠️ No pool found for {}, skipping OHLCV update",
                                        entry.mint
                                    )
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
                    match
                        temp_service.fetch_ohlcv_from_api(
                            &pool_address,
                            timeframe,
                            DEFAULT_OHLCV_LIMIT
                        ).await
                    {
                        Ok(data_points) => {
                            if is_debug_ohlcv_enabled() {
                                log(
                                    LogTag::Ohlcv,
                                    "BACKGROUND_UPDATE",
                                    &format!(
                                        "📈 Background update: {} {} - {} points (pool: {})",
                                        entry.mint,
                                        timeframe,
                                        data_points.len(),
                                        &pool_address[..8]
                                    )
                                );
                            }

                            // Cache the data
                            let cached_data = CachedOhlcvData {
                                mint: entry.mint.clone(),
                                timeframe: timeframe.clone(),
                                pool_address: pool_address.clone(),
                                data_points,
                                last_updated: Utc::now(),
                                last_timestamp: None,
                            };

                            // Update memory cache
                            {
                                let mut cache = cache.write().await;
                                cache.insert(cache_key, cached_data.clone());
                            }

                            // Save to file cache
                            if let Err(e) = temp_service.save_to_file_cache(&cached_data).await {
                                log(
                                    LogTag::Ohlcv,
                                    "WARNING",
                                    &format!("Background save failed: {}", e)
                                );
                            }

                            // Update stats
                            {
                                let mut stats = stats.write().await;
                                stats.successful_fetches += 1;
                            }
                        }
                        Err(e) => {
                            log(
                                LogTag::Ohlcv,
                                "ERROR",
                                &format!(
                                    "Background fetch failed for {} {}: {}",
                                    entry.mint,
                                    timeframe,
                                    e
                                )
                            );
                        }
                    }

                    // Small delay between API calls to be nice to the API
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
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
                            "WATCH_ENTRY_UPDATE",
                            &format!(
                                "📝 Updated watch entry for {}: count={}",
                                entry.mint,
                                entry.update_count
                            )
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
        stats: &Arc<RwLock<OhlcvStats>>
    ) -> Result<(), String> {
        if is_debug_ohlcv_enabled() {
            log(LogTag::Ohlcv, "CLEANUP_START", "🧹 Starting OHLCV data cleanup");
        }

        let cutoff_time = Utc::now() - ChronoDuration::hours(DATA_RETENTION_HOURS);
        let mut cleaned_memory = 0;
        let mut cleaned_files = 0;

        // Clean memory cache
        {
            let mut cache = cache.write().await;
            let initial_count = cache.len();
            cache.retain(|_, cached_data| { cached_data.last_updated > cutoff_time });
            cleaned_memory = initial_count - cache.len();
            if is_debug_ohlcv_enabled() && cleaned_memory > 0 {
                log(
                    LogTag::Ohlcv,
                    "CLEANUP_MEMORY",
                    &format!(
                        "🗑️ Cleaned {} memory cache entries (kept {})",
                        cleaned_memory,
                        cache.len()
                    )
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
                    &format!("🧹 Starting file cleanup in {}", cache_dir.display())
                );
            }
            cleaned_files = Self::cleanup_cache_files(cache_dir, cutoff_time)?;
            if is_debug_ohlcv_enabled() && cleaned_files > 0 {
                log(
                    LogTag::Ohlcv,
                    "CLEANUP_FILES_DONE",
                    &format!("🗂️ Cleaned {} cache files", cleaned_files)
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
                    cleaned_memory,
                    cleaned_files
                )
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
                            if fs::remove_file(&path).is_ok() {
                                cleaned_count += 1;
                                if is_debug_ohlcv_enabled() {
                                    log(
                                        LogTag::Ohlcv,
                                        "FILE_DELETED",
                                        &format!("🗑️ Deleted old cache file: {}", path.display())
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

use std::sync::{ Once };

static mut GLOBAL_OHLCV_SERVICE: Option<OhlcvService> = None;
static OHLCV_INIT: Once = Once::new();

/// Initialize global OHLCV service
pub fn init_ohlcv_service() -> Result<&'static OhlcvService, Box<dyn std::error::Error>> {
    unsafe {
        OHLCV_INIT.call_once(|| {
            match OhlcvService::new() {
                Ok(service) => {
                    GLOBAL_OHLCV_SERVICE = Some(service);
                    log(LogTag::Ohlcv, "INIT", "✅ Global OHLCV service initialized");
                }
                Err(e) => {
                    log(
                        LogTag::Ohlcv,
                        "ERROR",
                        &format!("❌ Failed to initialize OHLCV service: {}", e)
                    );
                }
            }
        });

        GLOBAL_OHLCV_SERVICE.as_ref().ok_or_else(|| "OHLCV service initialization failed".into())
    }
}

/// Get global OHLCV service
pub fn get_ohlcv_service() -> &'static OhlcvService {
    unsafe {
        GLOBAL_OHLCV_SERVICE.as_ref().expect(
            "OHLCV service not initialized - call init_ohlcv_service() first"
        )
    }
}

// =============================================================================
// PUBLIC HELPER FUNCTIONS
// =============================================================================

/// Start OHLCV background monitoring task
pub async fn start_ohlcv_monitoring(
    shutdown: Arc<Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    let service = init_ohlcv_service().map_err(|e|
        format!("Failed to initialize OHLCV service: {}", e)
    )?;

    service.start_monitoring(shutdown.clone()).await;

    let handle = tokio::spawn(async move {
        log(LogTag::Ohlcv, "TASK_START", "🚀 OHLCV monitoring task started");
        shutdown.notified().await;
        log(LogTag::Ohlcv, "TASK_END", "✅ OHLCV monitoring task ended");
    });

    Ok(handle)
}

/// Sync watch list with price service priority tokens (called from trader)
pub async fn sync_watch_list_with_trader() -> Result<(), String> {
    let service = get_ohlcv_service();

    // Get priority tokens from price service (these are the ones we're actively monitoring)
    let priority_tokens = get_priority_tokens_safe().await;

    if is_debug_ohlcv_enabled() {
        log(
            LogTag::Ohlcv,
            "SYNC_START",
            &format!("🔄 Syncing OHLCV watch list with {} priority tokens", priority_tokens.len())
        );
    }

    for token_mint in priority_tokens {
        // Check if it's an open position (higher priority)
        let is_open_position = crate::positions::is_open_position(&token_mint);
        service.add_to_watch_list(&token_mint, is_open_position).await;

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "SYNC_TOKEN",
                &format!(
                    "🔄 Synced token to watch list: {} (open_position: {})",
                    token_mint,
                    is_open_position
                )
            );
        }
    }

    if is_debug_ohlcv_enabled() {
        let stats = service.get_stats().await;
        log(
            LogTag::Ohlcv,
            "SYNC_COMPLETE",
            &format!("✅ OHLCV watch list synced: {} tokens being monitored", stats.watched_tokens)
        );
    }

    Ok(())
}

/// Check if OHLCV data is available for trading decisions
pub async fn is_ohlcv_data_available(mint: &str, timeframe: &Timeframe) -> bool {
    let service = get_ohlcv_service();
    let availability = service.check_data_availability(mint, timeframe).await;
    let result = availability.has_cached_data && availability.is_fresh;

    if is_debug_ohlcv_enabled() {
        log(
            LogTag::Ohlcv,
            "AVAILABILITY_CHECK",
            &format!(
                "📊 OHLCV availability check for {} {}: cached={}, fresh={}, result={}",
                mint,
                timeframe,
                availability.has_cached_data,
                availability.is_fresh,
                result
            )
        );
    }

    result
}

/// Get latest OHLCV data for analysis (convenience function)
pub async fn get_latest_ohlcv(
    mint: &str,
    timeframe: &Timeframe,
    limit: u32
) -> Result<Vec<OhlcvDataPoint>, String> {
    if is_debug_ohlcv_enabled() {
        log(
            LogTag::Ohlcv,
            "GET_LATEST",
            &format!("📈 Getting latest OHLCV data for {} {} (limit: {})", mint, timeframe, limit)
        );
    }

    let service = get_ohlcv_service();
    let result = service.get_ohlcv_data(mint, timeframe, Some(limit)).await;

    if is_debug_ohlcv_enabled() {
        match &result {
            Ok(data) =>
                log(
                    LogTag::Ohlcv,
                    "GET_LATEST_SUCCESS",
                    &format!("✅ Retrieved {} OHLCV points for {} {}", data.len(), mint, timeframe)
                ),
            Err(e) =>
                log(
                    LogTag::Ohlcv,
                    "GET_LATEST_ERROR",
                    &format!("❌ Failed to get OHLCV data for {} {}: {}", mint, timeframe, e)
                ),
        }
    }

    result
}
