/// OHLCV Data Collection and Caching System for ScreenerBot - 1-Minute Only
///
/// This module provides OHLCV (Open, High, Low, Close, Volume) data collection
/// from GeckoTerminal API with SQLite database caching and background monitoring.
/// Optimized for 1-minute timeframe only for simplicity and performance.
///
/// ## Features:
/// - **Single Timeframe**: Only 1-minute candles for consistent analysis
/// - **Database Caching**: SQLite database for efficient data storage and retrieval
///   - Database: data/ohlcvs.db
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

use crate::global::is_debug_ohlcv_enabled;
use crate::tokens::ohlcv_db::{ get_ohlcv_database, init_ohlcv_database };
use crate::logger::{ log, LogTag };
use crate::pool_service::get_pool_service;
use crate::tokens::PriceOptions;
use crate::tokens::geckoterminal::{ get_ohlcv_data_from_geckoterminal, OhlcvDataPoint };
use chrono::{ DateTime, Duration as ChronoDuration, Utc };
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::{ Notify, RwLock };

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Maximum number of cached entries in memory to prevent unbounded growth
const MAX_MEMORY_CACHE_ENTRIES: usize = 500;

/// Data retention period (6 hours - shorter since only 1m data)
const DATA_RETENTION_HOURS: i64 = 6;

/// Default limit for OHLCV data points
const DEFAULT_OHLCV_LIMIT: u32 = 200;

/// Maximum limit for OHLCV data points
const MAX_OHLCV_LIMIT: u32 = 500;

/// Background monitoring interval (30 seconds - more frequent for 1m data)
const MONITORING_INTERVAL_SECS: u64 = 30;

/// Cache file cleanup interval (15 minutes)
const CLEANUP_INTERVAL_SECS: u64 = 900;

/// Cache expiration time for 1-minute data (5 minutes)
const CACHE_EXPIRY_MINUTES: i64 = 5;

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Cached OHLCV data for a token (1-minute only) - now database-backed
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
    pub last_accessed: Option<DateTime<Utc>>, // Track when data was last requested
    pub update_count: u64,
    pub access_count: u64, // Track how often data is requested
    pub pool_address: Option<String>,
    pub pool_address_cached_at: Option<DateTime<Utc>>, // Track when pool was cached
}

// =============================================================================
// MAIN OHLCV SERVICE
// =============================================================================

/// OHLCV data collection and caching service (1-minute only)
#[derive(Clone)]
pub struct OhlcvService {
    /// In-memory cache for OHLCV data (key: mint)
    cache: Arc<RwLock<HashMap<String, CachedOhlcvData>>>,
    /// Watch list for background monitoring (key: mint)
    watch_list: Arc<RwLock<HashMap<String, OhlcvWatchEntry>>>,
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
        // Initialize database instead of file cache
        init_ohlcv_database().map_err(|e| format!("Failed to initialize OHLCV database: {}", e))?;

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "INIT_SERVICE",
                "🌐 OHLCV service initialized with database caching (rate limiting handled by GeckoTerminal module)"
            );
        }

        Ok(Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            watch_list: Arc::new(RwLock::new(HashMap::new())),
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

        log(LogTag::Ohlcv, "START", "🚀 Starting 1-minute OHLCV background monitoring service");

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "MONITOR_CONFIG",
                &format!(
                    "📋 Monitor config - Interval: {}s, Cleanup: {}s, Data retention: {}h (rate limiting handled by GeckoTerminal)",
                    MONITORING_INTERVAL_SECS,
                    CLEANUP_INTERVAL_SECS,
                    DATA_RETENTION_HOURS
                )
            );
        }

        let cache = self.cache.clone();
        let watch_list = self.watch_list.clone();
        let stats = self.stats.clone();
        let monitoring_active = self.monitoring_active.clone();

        tokio::spawn(async move {
            let mut monitoring_interval = tokio::time::interval(
                Duration::from_secs(MONITORING_INTERVAL_SECS)
            );
            let mut cleanup_interval = tokio::time::interval(
                Duration::from_secs(CLEANUP_INTERVAL_SECS)
            );
            let mut watch_cleanup_interval = tokio::time::interval(Duration::from_secs(3600)); // Cleanup watch list every hour

            loop {
                tokio::select! {
                    _ = monitoring_interval.tick() => {
                        if is_debug_ohlcv_enabled() {
                            log(LogTag::Ohlcv, "MONITOR_TICK", "⏰ 1m OHLCV monitoring tick starting");
                        }
                        if let Err(e) = Self::process_watch_list(
                            &cache,
                            &watch_list,
                            &stats
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
                    _ = watch_cleanup_interval.tick() => {
                        if is_debug_ohlcv_enabled() {
                            log(LogTag::Ohlcv, "WATCH_CLEANUP_TICK", "🧹 Watch list cleanup tick starting");
                        }
                        let temp_service = Self {
                            cache: cache.clone(),
                            watch_list: watch_list.clone(),
                            stats: stats.clone(),
                            monitoring_active: Arc::new(RwLock::new(true)),
                        };
                        temp_service.cleanup_watch_list().await;
                        if is_debug_ohlcv_enabled() {
                            log(LogTag::Ohlcv, "WATCH_CLEANUP_TICK_DONE", "✅ Watch list cleanup tick completed");
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
                    &format!("📊 Updated 1m OHLCV watch list for {}: priority={}", mint, priority)
                );
            }
        } else {
            // Add new entry
            watch_list.insert(mint.to_string(), OhlcvWatchEntry {
                mint: mint.to_string(),
                is_open_position,
                priority,
                added_at: Utc::now(),
                last_update: None,
                last_accessed: None,
                update_count: 0,
                access_count: 0,
                pool_address: None,
                pool_address_cached_at: None,
            });

            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "WATCH_ADD_DETAIL",
                    &format!(
                        "📈 Added {} to 1m OHLCV watch list (priority: {}, open_position: {})",
                        mint,
                        priority,
                        is_open_position
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
                &format!("📉 Removed {} from 1m OHLCV watch list", mint)
            );

            // Update stats
            let mut stats = self.stats.write().await;
            stats.watched_tokens = watch_list.len();
        }
    }

    /// Clean up inactive watch list entries
    pub async fn cleanup_watch_list(&self) {
        let cutoff_time = Utc::now() - ChronoDuration::hours(24); // Remove entries older than 24h with no access
        let mut removed_count = 0;

        {
            let mut watch_list = self.watch_list.write().await;
            let initial_count = watch_list.len();

            watch_list.retain(|mint, entry| {
                // Keep if:
                // 1. Open position (always keep)
                // 2. Recently accessed (within 24h)
                // 3. Recently added (within 1h, even if not accessed)
                let keep =
                    entry.is_open_position ||
                    entry.last_accessed.map_or(false, |t| t > cutoff_time) ||
                    Utc::now() - entry.added_at < ChronoDuration::hours(1);

                if !keep && is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "WATCH_CLEANUP",
                        &format!("🗑️ Removing inactive watch entry: {}", mint)
                    );
                }

                keep
            });

            removed_count = initial_count - watch_list.len();

            // Update stats
            let mut stats = self.stats.write().await;
            stats.watched_tokens = watch_list.len();
        }

        if removed_count > 0 {
            log(
                LogTag::Ohlcv,
                "WATCH_CLEANUP_COMPLETE",
                &format!("🧹 Cleaned up {} inactive watch list entries", removed_count)
            );
        }
    }

    /// Check data availability for a token
    pub async fn check_data_availability(&self, mint: &str) -> DataAvailability {
        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "AVAILABILITY_CHECK",
                &format!("🔍 Checking 1m OHLCV data availability for {}", mint)
            );
        }

        // Check in-memory cache
        let cached_data = {
            let cache = self.cache.read().await;
            cache.get(mint).cloned()
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
                        "💾 Memory cache found for {}: {} points, fresh: {}",
                        mint,
                        data.data_points.len(),
                        is_fresh
                    )
                );
            }
            (true, data.last_timestamp, data.data_points.len(), is_fresh)
        } else {
            // Check database cache
            match get_ohlcv_database() {
                Ok(db) => {
                    match db.check_data_availability(mint) {
                        Ok(metadata) => {
                            let has_data = metadata.data_points_count > 0;
                            let is_fresh = !metadata.is_expired;
                            if is_debug_ohlcv_enabled() {
                                log(
                                    LogTag::Ohlcv,
                                    "DB_CACHE_CHECK",
                                    &format!(
                                        "�️ Database cache found for {}: {} points, fresh: {}",
                                        mint,
                                        metadata.data_points_count,
                                        is_fresh
                                    )
                                );
                            }
                            (
                                has_data,
                                metadata.last_timestamp,
                                metadata.data_points_count,
                                is_fresh,
                            )
                        }
                        Err(e) => {
                            if is_debug_ohlcv_enabled() {
                                log(
                                    LogTag::Ohlcv,
                                    "DB_ERROR",
                                    &format!(
                                        "Database availability check failed for {}: {}",
                                        mint,
                                        e
                                    )
                                );
                            }
                            (false, None, 0, false)
                        }
                    }
                }
                Err(e) => {
                    if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "DB_UNAVAILABLE",
                            &format!("Database unavailable for {}: {}", mint, e)
                        );
                    }
                    (false, None, 0, false)
                }
            }
        };

        // Check if token has a pool
        let has_pool = crate::pool_service::check_token_availability(mint).await;
        let pool_address = if has_pool {
            // Get best pool address
            if
                let Some(result) = crate::tokens::get_price(mint).await
            {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "POOL_FOUND",
                        &format!(
                            "🏊 Pool found for {}: price {:.9}",
                            mint,
                            result
                        )
                    );
                }
                Some("pool_address".to_string()) // Placeholder since we only have price now
            } else {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "POOL_UNAVAILABLE",
                        &format!("⚠️ Price service returned no price for {}", mint)
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
                    "📊 1m OHLCV availability for {}: cached={}, pool={}, fresh={}, points={}",
                    mint,
                    has_cached_data,
                    has_pool,
                    is_fresh,
                    data_points_count
                )
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
        limit: Option<u32>
    ) -> Result<Vec<OhlcvDataPoint>, String> {
        let limit = limit.unwrap_or(DEFAULT_OHLCV_LIMIT).min(MAX_OHLCV_LIMIT);

        // Track access for watch list prioritization
        {
            let mut watch_list = self.watch_list.write().await;
            if let Some(entry) = watch_list.get_mut(mint) {
                entry.last_accessed = Some(Utc::now());
                entry.access_count += 1;
            }
        }

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "DATA_REQUEST",
                &format!("📊 1m OHLCV data request: {} (limit: {})", mint, limit)
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
                            &format!(
                                "✅ Memory cache hit for {}: {} points",
                                mint,
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

        // Check database cache
        match get_ohlcv_database() {
            Ok(db) => {
                match db.check_data_availability(mint) {
                    Ok(metadata) => {
                        if metadata.data_points_count > 0 && !metadata.is_expired {
                            // Load from database
                            match db.get_ohlcv_data(mint, Some(limit)) {
                                Ok(data_points) => {
                                    // Load into memory cache for faster future access
                                    let cached_data = CachedOhlcvData {
                                        mint: mint.to_string(),
                                        pool_address: metadata.pool_address.clone(),
                                        data_points: data_points.clone(),
                                        last_updated: metadata.last_updated,
                                        last_timestamp: metadata.last_timestamp,
                                    };

                                    {
                                        let mut cache = self.cache.write().await;

                                        // If cache is getting too large, remove oldest entries
                                        if cache.len() >= MAX_MEMORY_CACHE_ENTRIES {
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
                                                        &format!("🗑️ Evicted oldest cache entry: {}", key)
                                                    );
                                                }
                                            }
                                        }

                                        cache.insert(mint.to_string(), cached_data);
                                    }

                                    let mut stats = self.stats.write().await;
                                    stats.cache_hits += 1;

                                    if is_debug_ohlcv_enabled() {
                                        log(
                                            LogTag::Ohlcv,
                                            "DB_CACHE_HIT",
                                            &format!(
                                                "�️ Database cache hit for {}: {} points",
                                                mint,
                                                data_points.len()
                                            )
                                        );
                                    }

                                    return Ok(data_points);
                                }
                                Err(e) => {
                                    if is_debug_ohlcv_enabled() {
                                        log(
                                            LogTag::Ohlcv,
                                            "DB_READ_ERROR",
                                            &format!("Database read failed for {}: {}", mint, e)
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if is_debug_ohlcv_enabled() {
                            log(
                                LogTag::Ohlcv,
                                "DB_AVAILABILITY_ERROR",
                                &format!("Database availability check failed for {}: {}", mint, e)
                            );
                        }
                    }
                }
            }
            Err(e) => {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "DB_UNAVAILABLE",
                        &format!("Database unavailable for {}: {}", mint, e)
                    );
                }
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
                &format!("❌ Cache miss for {}, fetching 1m data from API", mint)
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
        match self.fetch_ohlcv_from_api(&pool_address, limit).await {
            Ok(data_points) => {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "API_SUCCESS",
                        &format!(
                            "✅ Fetched {} 1m OHLCV points for {} from API",
                            data_points.len(),
                            mint
                        )
                    );
                }

                // Cache the data in both memory and database
                let cached_data = CachedOhlcvData {
                    mint: mint.to_string(),
                    pool_address,
                    data_points: data_points.clone(),
                    last_updated: Utc::now(),
                    last_timestamp: data_points
                        .iter()
                        .map(|p| p.timestamp)
                        .max(),
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
                                    &format!("🗑️ Evicted oldest cache entry: {}", key)
                                );
                            }
                        }
                    }

                    cache.insert(mint.to_string(), cached_data.clone());
                }

                // Save to database
                if let Ok(db) = get_ohlcv_database() {
                    if
                        let Err(e) = db.store_ohlcv_data(
                            mint,
                            &cached_data.pool_address,
                            &data_points
                        )
                    {
                        log(
                            LogTag::Ohlcv,
                            "WARNING",
                            &format!("Failed to save to database: {}", e)
                        );
                    } else if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "DB_SAVE",
                            &format!(
                                "💾 Saved {} OHLCV points for {} to database",
                                data_points.len(),
                                mint
                            )
                        );
                    }
                } else {
                    log(LogTag::Ohlcv, "WARNING", "Database unavailable for saving OHLCV data");
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
                    &format!("Failed to fetch 1m OHLCV data for {}: {}", mint, e)
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

    /// Get pool address for a mint with caching
    async fn get_pool_address_for_mint(&self, mint: &str) -> Option<String> {
        // Check if we have cached pool address in watch list
        {
            let watch_list = self.watch_list.read().await;
            if let Some(entry) = watch_list.get(mint) {
                if let Some(pool_address) = &entry.pool_address {
                    if let Some(cached_at) = entry.pool_address_cached_at {
                        // Pool addresses are relatively stable, cache for 1 hour
                        if Utc::now() - cached_at < ChronoDuration::hours(1) {
                            if is_debug_ohlcv_enabled() {
                                log(
                                    LogTag::Ohlcv,
                                    "POOL_CACHE_HIT",
                                    &format!(
                                        "🏊 Using cached pool address for {}: {}",
                                        mint,
                                        pool_address
                                    )
                                );
                            }
                            return Some(pool_address.clone());
                        }
                    }
                }
            }
        }

        // Cache miss or expired - get from pool service
        if
            let Some(result) = crate::tokens::get_price(mint).await
        {
            let pool_address = "pool_address".to_string(); // Placeholder since we only have price now

            // Update watch list cache
            {
                let mut watch_list = self.watch_list.write().await;
                if let Some(entry) = watch_list.get_mut(mint) {
                    entry.pool_address = Some(pool_address.clone());
                    entry.pool_address_cached_at = Some(Utc::now());
                    if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "POOL_CACHE_UPDATE",
                            &format!("🏊 Updated pool address cache for {}: {}", mint, pool_address)
                        );
                    }
                }
            }

            Some(pool_address)
        } else {
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "POOL_LOOKUP_FAILED",
                    &format!("❌ Pool lookup failed for {}", mint)
                );
            }
            None
        }
    }

    /// Fetch 1-minute OHLCV data from GeckoTerminal API (delegates to geckoterminal module)
    async fn fetch_ohlcv_from_api(
        &self,
        pool_address: &str,
        limit: u32
    ) -> Result<Vec<OhlcvDataPoint>, String> {
        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "API_DELEGATE",
                &format!(
                    "🔄 Delegating 1m OHLCV API call to GeckoTerminal module for pool {} (limit: {})",
                    &pool_address[..8],
                    limit
                )
            );
        }

        // Update API call stats
        {
            let mut stats = self.stats.write().await;
            stats.total_api_calls += 1;
        }

        // Delegate to geckoterminal module (which handles rate limiting)
        match get_ohlcv_data_from_geckoterminal(pool_address, limit).await {
            Ok(data) => {
                // Update successful fetch stats
                {
                    let mut stats = self.stats.write().await;
                    stats.successful_fetches += 1;
                }

                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "API_SUCCESS",
                        &format!(
                            "✅ Retrieved {} OHLCV data points via GeckoTerminal module",
                            data.len()
                        )
                    );
                }

                Ok(data)
            }
            Err(e) => {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "API_ERROR",
                        &format!("❌ GeckoTerminal module returned error: {}", e)
                    );
                }
                Err(e)
            }
        }
    }

    /// Clean up old cached data (now database-based)
    async fn cleanup_old_data(
        cache: &Arc<RwLock<HashMap<String, CachedOhlcvData>>>,
        stats: &Arc<RwLock<OhlcvStats>>
    ) -> Result<(), String> {
        if is_debug_ohlcv_enabled() {
            log(LogTag::Ohlcv, "CLEANUP_START", "🧹 Starting 1m OHLCV data cleanup");
        }

        let cutoff_time = Utc::now() - ChronoDuration::hours(DATA_RETENTION_HOURS);
        let mut cleaned_memory = 0;
        let mut cleaned_db = 0;

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
                    )
                );
            }
        }

        // Clean database
        if let Ok(db) = get_ohlcv_database() {
            match db.cleanup_old_data() {
                Ok(deleted_count) => {
                    cleaned_db = deleted_count;
                    if is_debug_ohlcv_enabled() && cleaned_db > 0 {
                        log(
                            LogTag::Ohlcv,
                            "CLEANUP_DATABASE",
                            &format!("🗄️ Cleaned {} database entries", cleaned_db)
                        );
                    }
                }
                Err(e) => {
                    log(LogTag::Ohlcv, "WARNING", &format!("Database cleanup failed: {}", e));
                }
            }
        } else {
            log(LogTag::Ohlcv, "WARNING", "Database unavailable for cleanup");
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
                    "✅ Cleanup complete: {} memory entries, {} database entries removed",
                    cleaned_memory,
                    cleaned_db
                )
            );
        }

        Ok(())
    }

    /// Process watch list for background monitoring (database-backed)
    async fn process_watch_list(
        cache: &Arc<RwLock<HashMap<String, CachedOhlcvData>>>,
        watch_list: &Arc<RwLock<HashMap<String, OhlcvWatchEntry>>>,
        stats: &Arc<RwLock<OhlcvStats>>
    ) -> Result<(), String> {
        let tokens_to_update = {
            let watch_list = watch_list.read().await;
            if watch_list.is_empty() {
                return Ok(());
            }

            // Get priority tokens (open positions get priority, recently accessed get boost)
            let mut tokens: Vec<_> = watch_list.values().cloned().collect();
            tokens.sort_by(|a, b| {
                let a_recent_access = a.last_accessed.map_or(false, |t| {
                    Utc::now() - t < ChronoDuration::hours(1)
                });
                let b_recent_access = b.last_accessed.map_or(false, |t| {
                    Utc::now() - t < ChronoDuration::hours(1)
                });

                let a_effective_priority = a.priority + (if a_recent_access { 25 } else { 0 });
                let b_effective_priority = b.priority + (if b_recent_access { 25 } else { 0 });

                b_effective_priority
                    .cmp(&a_effective_priority)
                    .then_with(|| a.last_update.cmp(&b.last_update))
            });

            // Limit concurrent updates - fewer for background, more for high-priority
            let high_priority_tokens: Vec<_> = tokens
                .iter()
                .filter(|t| t.is_open_position)
                .take(5)
                .cloned()
                .collect();

            let regular_tokens: Vec<_> = tokens
                .iter()
                .filter(|t| !t.is_open_position)
                .take(3)
                .cloned()
                .collect();

            [high_priority_tokens, regular_tokens].concat()
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
                )
            );
        }

        for entry in tokens_to_update {
            let needs_update = {
                // First check memory cache
                let cache = cache.read().await;
                if let Some(cached) = cache.get(&entry.mint) {
                    let expired = cached.is_expired();
                    if is_debug_ohlcv_enabled() {
                        log(
                            LogTag::Ohlcv,
                            "CACHE_CHECK",
                            &format!(
                                "📋 Memory cache check for {}: expired={}",
                                entry.mint,
                                expired
                            )
                        );
                    }
                    expired
                } else {
                    // Check database cache
                    if let Ok(db) = get_ohlcv_database() {
                        if let Ok(metadata) = db.check_data_availability(&entry.mint) {
                            let expired = metadata.is_expired;
                            if is_debug_ohlcv_enabled() {
                                log(
                                    LogTag::Ohlcv,
                                    "DB_CACHE_CHECK",
                                    &format!(
                                        "🗄️ Database cache check for {}: expired={}",
                                        entry.mint,
                                        expired
                                    )
                                );
                            }
                            expired
                        } else {
                            if is_debug_ohlcv_enabled() {
                                log(
                                    LogTag::Ohlcv,
                                    "NO_CACHE_ENTRY",
                                    &format!("❌ No cache entry for {}", entry.mint)
                                );
                            }
                            true // No cache, definitely needs update
                        }
                    } else {
                        true // Database unavailable, needs update
                    }
                }
            };

            if needs_update {
                // Get pool address
                let pool_address = if let Some(addr) = &entry.pool_address {
                    if let Some(cached_at) = entry.pool_address_cached_at {
                        if Utc::now() - cached_at < ChronoDuration::hours(1) {
                            addr.clone()
                        } else {
                            // Pool address cache expired, refresh it
                            if
                                                    let Some(result) = crate::tokens::get_price(&entry.mint).await
                            {
                                "pool_address".to_string() // Placeholder since we only have price now
                            } else {
                                if is_debug_ohlcv_enabled() {
                                    log(
                                        LogTag::Ohlcv,
                                        "POOL_UNAVAILABLE",
                                        &format!("⚠️ No pool available for {}", entry.mint)
                                    );
                                }
                                continue;
                            }
                        }
                    } else {
                        addr.clone()
                    }
                } else {
                    if
                                            let Some(result) = crate::tokens::get_price(&entry.mint).await
                    {
                        "pool_address".to_string() // Placeholder since we only have price now
                    } else {
                        if is_debug_ohlcv_enabled() {
                            log(
                                LogTag::Ohlcv,
                                "POOL_UNAVAILABLE",
                                &format!("⚠️ No pool available for {}", entry.mint)
                            );
                        }
                        continue;
                    }
                };

                // Create temporary service instance for this update
                let temp_service = OhlcvService {
                    cache: cache.clone(),
                    watch_list: watch_list.clone(),
                    stats: stats.clone(),
                    monitoring_active: Arc::new(RwLock::new(true)),
                };

                // Fetch new data (now delegated to geckoterminal module)
                match temp_service.fetch_ohlcv_from_api(&pool_address, DEFAULT_OHLCV_LIMIT).await {
                    Ok(data_points) => {
                        // Cache the data in memory
                        let cached_data = CachedOhlcvData {
                            mint: entry.mint.clone(),
                            pool_address: pool_address.clone(),
                            data_points: data_points.clone(),
                            last_updated: Utc::now(),
                            last_timestamp: data_points
                                .iter()
                                .map(|p| p.timestamp)
                                .max(),
                        };

                        // Update memory cache
                        {
                            let mut cache = cache.write().await;
                            cache.insert(entry.mint.clone(), cached_data.clone());
                        }

                        // Save to database
                        if let Ok(db) = get_ohlcv_database() {
                            if
                                let Err(e) = db.store_ohlcv_data(
                                    &entry.mint,
                                    &pool_address,
                                    &data_points
                                )
                            {
                                log(
                                    LogTag::Ohlcv,
                                    "WARNING",
                                    &format!("Failed to save background fetch to database: {}", e)
                                );
                            }
                        }

                        // Update stats
                        {
                            let mut stats = stats.write().await;
                            stats.successful_fetches += 1;
                            stats.data_points_cached += data_points.len();
                        }

                        if is_debug_ohlcv_enabled() {
                            log(
                                LogTag::Ohlcv,
                                "BACKGROUND_UPDATE_SUCCESS",
                                &format!(
                                    "✅ Background updated {} with {} 1m points",
                                    entry.mint,
                                    data_points.len()
                                )
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Ohlcv,
                            "WARNING",
                            &format!("Background fetch failed for {}: {}", entry.mint, e)
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
                            &format!(
                                "📊 Updated watch entry for {} (count: {})",
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
}

// =============================================================================
// GLOBAL OHLCV SERVICE INSTANCE
// =============================================================================

use std::sync::LazyLock;
use tokio::sync::RwLock as TokioRwLock;

// Use LazyLock for safe global state (Rust 1.70+)
static GLOBAL_OHLCV_SERVICE: LazyLock<TokioRwLock<Option<OhlcvService>>> = LazyLock::new(||
    TokioRwLock::new(None)
);

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
            log(LogTag::Ohlcv, "ERROR", &format!("❌ Failed to initialize OHLCV service: {}", e));
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
    shutdown: Arc<Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    init_ohlcv_service().await.map_err(|e| format!("Failed to initialize OHLCV service: {}", e))?;

    // Get cloned service for async operations
    let service = get_ohlcv_service_clone().await?;

    // Start monitoring
    service.start_monitoring(shutdown.clone()).await;

    let handle = tokio::spawn(async move {
        log(LogTag::Ohlcv, "TASK_START", "🚀 1m OHLCV monitoring task started");
        shutdown.notified().await;
        log(LogTag::Ohlcv, "TASK_END", "✅ 1m OHLCV monitoring task ended");
    });

    Ok(handle)
}

/// Check if OHLCV data is available for trading decisions
pub async fn is_ohlcv_data_available(mint: &str) -> bool {
    let service = match get_ohlcv_service_clone().await {
        Ok(service) => service,
        Err(_) => {
            if is_debug_ohlcv_enabled() {
                log(LogTag::Ohlcv, "ERROR", "OHLCV service not available for availability check");
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
            &format!("📊 1m OHLCV availability check for {}: result={}", mint, is_available)
        );
    }

    is_available
}

/// Get latest 1-minute OHLCV data for analysis (convenience function)
pub async fn get_latest_ohlcv(mint: &str, limit: u32) -> Result<Vec<OhlcvDataPoint>, String> {
    if is_debug_ohlcv_enabled() {
        log(
            LogTag::Ohlcv,
            "GET_LATEST",
            &format!("📈 Getting latest 1m OHLCV data for {} (limit: {})", mint, limit)
        );
    }

    let service = get_ohlcv_service_clone().await?;
    let result = service.get_ohlcv_data(mint, Some(limit)).await;

    if is_debug_ohlcv_enabled() {
        match &result {
            Ok(data) =>
                log(
                    LogTag::Ohlcv,
                    "GET_LATEST_SUCCESS",
                    &format!("✅ Retrieved {} 1m OHLCV points for {}", data.len(), mint)
                ),
            Err(e) =>
                log(
                    LogTag::Ohlcv,
                    "GET_LATEST_ERROR",
                    &format!("❌ Failed to get 1m OHLCV data for {}: {}", mint, e)
                ),
        }
    }

    result
}
