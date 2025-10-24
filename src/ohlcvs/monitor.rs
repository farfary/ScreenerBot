// Background monitoring service

use crate::config::with_config;
use crate::events::{record_ohlcv_event, Severity};
use crate::logger::{self, LogTag};
use crate::ohlcvs::aggregator::OhlcvAggregator;
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::OhlcvDatabase;
use crate::ohlcvs::fetcher::OhlcvFetcher;
use crate::ohlcvs::gaps::GapManager;
use crate::ohlcvs::manager::PoolManager;
use crate::ohlcvs::priorities::{ActivityType, PriorityManager};
use crate::ohlcvs::types::{
    MintGapAggregate, OhlcvDataPoint, OhlcvError, OhlcvResult, OhlcvTokenStatus, Priority,
    Timeframe, TokenOhlcvConfig,
};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tokio::task::spawn_blocking;
use tokio::time::{interval, sleep, Duration, Instant};

const AGGREGATED_TIMEFRAMES: [Timeframe; 6] = [
    Timeframe::Minute5,
    Timeframe::Minute15,
    Timeframe::Hour1,
    Timeframe::Hour4,
    Timeframe::Hour12,
    Timeframe::Day1,
];

const GAP_SUMMARY_LIMIT: usize = 5;

#[derive(Debug, Default, Clone)]
struct MonitorTelemetry {
    monitor_cycle_started_at: Option<DateTime<Utc>>,
    monitor_cycle_completed_at: Option<DateTime<Utc>>,
    monitor_cycle_duration_ms: Option<u64>,
    monitor_cycle_tokens_processed: usize,
    monitor_cycle_total: u64,
    gap_cycle_started_at: Option<DateTime<Utc>>,
    gap_cycle_completed_at: Option<DateTime<Utc>>,
    gap_cycle_duration_ms: Option<u64>,
    gap_cycle_tokens_processed: usize,
    gap_cycle_total: u64,
    last_rate_limit_at: Option<DateTime<Utc>>,
    rate_limit_events: u64,
    total_backfills_scheduled: u64,
    total_backfills_completed: u64,
    total_backfills_failed: u64,
    last_backfill_started_at: Option<DateTime<Utc>>,
    last_backfill_completed_at: Option<DateTime<Utc>>,
    last_backfill_duration_ms: Option<u64>,
    last_backfill_points: Option<usize>,
    last_backfill_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MonitorTelemetrySnapshot {
    pub monitor_cycle_started_at: Option<DateTime<Utc>>,
    pub monitor_cycle_completed_at: Option<DateTime<Utc>>,
    pub monitor_cycle_duration_ms: Option<u64>,
    pub monitor_cycle_tokens_processed: usize,
    pub monitor_cycle_total: u64,
    pub gap_cycle_started_at: Option<DateTime<Utc>>,
    pub gap_cycle_completed_at: Option<DateTime<Utc>>,
    pub gap_cycle_duration_ms: Option<u64>,
    pub gap_cycle_tokens_processed: usize,
    pub gap_cycle_total: u64,
    pub last_rate_limit_at: Option<DateTime<Utc>>,
    pub rate_limit_events: u64,
    pub total_backfills_scheduled: u64,
    pub total_backfills_completed: u64,
    pub total_backfills_failed: u64,
    pub last_backfill_started_at: Option<DateTime<Utc>>,
    pub last_backfill_completed_at: Option<DateTime<Utc>>,
    pub last_backfill_duration_ms: Option<u64>,
    pub last_backfill_points: Option<usize>,
    pub last_backfill_error: Option<String>,
}

impl From<&MonitorTelemetry> for MonitorTelemetrySnapshot {
    fn from(value: &MonitorTelemetry) -> Self {
        Self {
            monitor_cycle_started_at: value.monitor_cycle_started_at.clone(),
            monitor_cycle_completed_at: value.monitor_cycle_completed_at.clone(),
            monitor_cycle_duration_ms: value.monitor_cycle_duration_ms,
            monitor_cycle_tokens_processed: value.monitor_cycle_tokens_processed,
            monitor_cycle_total: value.monitor_cycle_total,
            gap_cycle_started_at: value.gap_cycle_started_at.clone(),
            gap_cycle_completed_at: value.gap_cycle_completed_at.clone(),
            gap_cycle_duration_ms: value.gap_cycle_duration_ms,
            gap_cycle_tokens_processed: value.gap_cycle_tokens_processed,
            gap_cycle_total: value.gap_cycle_total,
            last_rate_limit_at: value.last_rate_limit_at.clone(),
            rate_limit_events: value.rate_limit_events,
            total_backfills_scheduled: value.total_backfills_scheduled,
            total_backfills_completed: value.total_backfills_completed,
            total_backfills_failed: value.total_backfills_failed,
            last_backfill_started_at: value.last_backfill_started_at.clone(),
            last_backfill_completed_at: value.last_backfill_completed_at.clone(),
            last_backfill_duration_ms: value.last_backfill_duration_ms,
            last_backfill_points: value.last_backfill_points,
            last_backfill_error: value.last_backfill_error.clone(),
        }
    }
}

pub struct OhlcvMonitor {
    db: Arc<OhlcvDatabase>,
    fetcher: Arc<OhlcvFetcher>,
    cache: Arc<OhlcvCache>,
    pool_manager: Arc<PoolManager>,
    gap_manager: Arc<GapManager>,
    active_tokens: Arc<RwLock<HashMap<String, TokenOhlcvConfig>>>,
    shutdown_signal: Arc<RwLock<bool>>,
    backfill_in_progress: Arc<Mutex<HashSet<String>>>,
    telemetry: Arc<RwLock<MonitorTelemetry>>,
}

impl OhlcvMonitor {
    pub fn new(
        db: Arc<OhlcvDatabase>,
        fetcher: Arc<OhlcvFetcher>,
        cache: Arc<OhlcvCache>,
        pool_manager: Arc<PoolManager>,
        gap_manager: Arc<GapManager>,
    ) -> Self {
        Self {
            db,
            fetcher,
            cache,
            pool_manager,
            gap_manager,
            active_tokens: Arc::new(RwLock::new(HashMap::new())),
            shutdown_signal: Arc::new(RwLock::new(false)),
            backfill_in_progress: Arc::new(Mutex::new(HashSet::new())),
            telemetry: Arc::new(RwLock::new(MonitorTelemetry::default())),
        }
    }

    /// Start monitoring all active tokens
    pub async fn start(self: Arc<Self>) -> OhlcvResult<()> {
        // Load active tokens from database
        self.load_active_tokens().await?;

        // Start background tasks
        tokio::spawn(self.clone().monitor_loop());
        tokio::spawn(self.clone().gap_fill_loop());
        tokio::spawn(self.clone().cleanup_loop());
        tokio::spawn(self.clone().cache_maintenance_loop());
        tokio::spawn(self.clone().sync_pool_service_tokens());

        Ok(())
    }

    /// Stop monitoring
    pub async fn stop(&self) {
        let mut shutdown = self.shutdown_signal.write().await;
        *shutdown = true;
    }

    /// Add a token to monitoring
    pub async fn add_token(&self, mint: String, priority: Priority) -> OhlcvResult<()> {
        let mut config = TokenOhlcvConfig::new(mint.clone(), priority);

        // Try to load existing pools from database
        let pools = match self.pool_manager.get_pools(&mint).await {
            Ok(pools) if !pools.is_empty() => pools,
            _ => {
                // No pools in database, try to discover them from Pool Service
                match self.pool_manager.discover_pools(&mint).await {
                    Ok(discovered) => discovered,
                    Err(e) => {
                        // If discovery fails, still allow monitoring but with empty pools
                        // The monitor loop will retry discovery later
                        logger::warning(LogTag::Ohlcv, &format!("Warning: Pool discovery failed for {}: {}", mint, e));
                        Vec::new()
                    }
                }
            }
        };

        config.pools = pools;

        // Store in database
        self.db.upsert_monitor_config(&config)?;

        // Add to active tokens
        let mut active = self.active_tokens.write().await;
        active.insert(mint, config);

        Ok(())
    }

    /// Remove a token from monitoring
    pub async fn remove_token(&self, mint: &str) -> OhlcvResult<()> {
        let removed_config = {
            let mut active = self.active_tokens.write().await;
            active.remove(mint).map(|mut config| {
                config.is_active = false;
                config
            })
        };

        if let Some(config) = removed_config {
            self.db.upsert_monitor_config(&config)?;
        }

        Ok(())
    }

    /// Update token priority
    pub async fn update_priority(&self, mint: &str, priority: Priority) -> OhlcvResult<()> {
        let updated_config = {
            let mut active = self.active_tokens.write().await;
            active.get_mut(mint).map(|config| {
                config.priority = priority;
                config.fetch_frequency = priority.base_interval();
                config.clone()
            })
        };

        if let Some(config) = updated_config {
            self.db.upsert_monitor_config(&config)?;
        }

        Ok(())
    }

    /// Record activity for a token
    pub async fn record_activity(
        &self,
        mint: &str,
        activity_type: ActivityType,
    ) -> OhlcvResult<()> {
        let (updated_config, should_trigger_fetch) = {
            let mut active = self.active_tokens.write().await;
            active.get_mut(mint).map_or((None, false), |config| {
                config.mark_activity();

                // Update priority based on activity
                let new_priority =
                    PriorityManager::update_priority_on_activity(config.priority, activity_type);
                config.priority = new_priority;
                config.fetch_frequency = new_priority.base_interval();

                let trigger = matches!(
                    activity_type,
                    ActivityType::PositionOpened | ActivityType::DataRequested
                );

                (Some(config.clone()), trigger)
            })
        };

        if let Some(config) = updated_config {
            self.db.upsert_monitor_config(&config)?;

            if should_trigger_fetch {
                self.fetch_token_data(mint).await?;
            }
        }

        Ok(())
    }

    /// Force refresh for a token
    pub async fn force_refresh(&self, mint: &str) -> OhlcvResult<()> {
        self.fetch_token_data(mint).await
    }

    /// Get monitoring statistics
    pub async fn get_stats(&self) -> MonitorStats {
        let active = self.active_tokens.read().await;
        let total_tokens = active.len();
        let by_priority = count_by_priority(&active);
        drop(active);

        let telemetry_snapshot = {
            let telemetry = self.telemetry.read().await;
            MonitorTelemetrySnapshot::from(&*telemetry)
        };

        let backfills_in_progress = self
            .backfill_in_progress
            .lock()
            .map(|set| set.len())
            .unwrap_or(0);

        let db = Arc::clone(&self.db);

        let (open_gap_tokens, open_gap_total, top_open_gaps) = match spawn_blocking(move || {
            let (token_count, gap_count) = db.get_gap_aggregate()?;
            let top = db.get_top_open_gaps(GAP_SUMMARY_LIMIT)?;
            Ok::<_, OhlcvError>((token_count, gap_count, top))
        })
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(err)) => {
                logger::warning(
                    LogTag::Ohlcv,
                    &format!("Failed to collect gap stats: {}", err),
                );
                (0, 0, Vec::new())
            }
            Err(join_err) => {
                logger::warning(
                    LogTag::Ohlcv,
                    &format!("Gap stats join error: {}", join_err),
                );
                (0, 0, Vec::new())
            }
        };

        MonitorStats {
            total_tokens,
            critical_tokens: by_priority.get(&Priority::Critical).copied().unwrap_or(0),
            high_tokens: by_priority.get(&Priority::High).copied().unwrap_or(0),
            medium_tokens: by_priority.get(&Priority::Medium).copied().unwrap_or(0),
            low_tokens: by_priority.get(&Priority::Low).copied().unwrap_or(0),
            cache_hit_rate: self.cache.hit_rate(),
            api_calls_per_minute: self.fetcher.calls_per_minute(),
            queue_size: self.fetcher.queue_size(),
            telemetry: telemetry_snapshot,
            backfills_in_progress,
            open_gap_tokens,
            open_gap_total,
            top_open_gaps,
        }
    }

    // ==================== Private Methods ====================

    async fn load_active_tokens(&self) -> OhlcvResult<()> {
        let configs = self.db.get_all_active_configs()?;

        let mut active = self.active_tokens.write().await;
        for config in configs {
            // Load pools for each token
            let pools = self.pool_manager.get_pools(&config.mint).await?;
            let mut full_config = config;
            full_config.pools = pools;

            active.insert(full_config.mint.clone(), full_config);
        }

        Ok(())
    }

    async fn monitor_loop(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(5)); // Check every 5 seconds

        // Calculate delay based on configured rate limit to respect API limits
        // Formula: (60,000ms / rate_limit_per_minute) + buffer
        use crate::apis::geckoterminal;
        let rate_limit = geckoterminal::RATE_LIMIT_PER_MINUTE;
        let delay_ms: u64 = if rate_limit > 0 {
            ((60_000 / rate_limit) + 100) as u64 // Add 100ms buffer for safety
        } else {
            2_000 // Fallback to 2 seconds if config invalid
        };

        // Always log this critical info
        logger::info(
            LogTag::Ohlcv,
            &format!(
                "OHLCV monitor starting: rate_limit={}/min, delay={}ms between tokens",
                rate_limit, delay_ms
            ),
        );

        loop {
            tick.tick().await;

            // Check shutdown signal
            if *self.shutdown_signal.read().await {
                break;
            }

            // Process each active token
            let tokens: Vec<String> = {
                let active = self.active_tokens.read().await;
                active.keys().cloned().collect()
            };

            let processed_count = tokens.len();
            let cycle_start = Instant::now();
            self.record_monitor_cycle_start(processed_count).await;

            for mint in tokens {
                match self.process_token(&mint).await {
                    Ok(_) => {}
                    Err(OhlcvError::NotFound(_)) => {
                        logger::debug(
                            LogTag::Ohlcv,
                            &format!("Token {} disappeared during processing; skipping", mint),
                        );
                        record_ohlcv_event(
                            "token_missing",
                            Severity::Debug,
                            Some(mint.as_str()),
                            None,
                            json!({
                                "message": format!("Token {} was missing during processing", mint),
                                "action": "skip_cycle",
                            }),
                        )
                        .await;
                    }
                    Err(OhlcvError::PoolNotFound(_)) => {
                        logger::warning(
                            LogTag::Ohlcv,
                            &format!("No healthy pools available for {}; deferring", mint),
                        );
                        record_ohlcv_event(
                            "pool_unavailable",
                            Severity::Warn,
                            Some(mint.as_str()),
                            None,
                            json!({
                                "message": format!(
                                    "No healthy pools available for {}; deferring",
                                    mint
                                ),
                                "reason": "pool_health",
                            }),
                        )
                        .await;
                    }
                    Err(OhlcvError::RateLimitExceeded) => {
                        self.record_rate_limit_event().await;
                        record_ohlcv_event(
                            "rate_limit_hit",
                            Severity::Warn,
                            Some(mint.as_str()),
                            None,
                            json!({
                                "message": format!(
                                    "Rate limit triggered while processing {}",
                                    mint
                                ),
                                "rate_limit_per_minute": rate_limit,
                                "delay_ms": delay_ms,
                                "tokens_in_cycle": processed_count,
                            }),
                        )
                        .await;
                        logger::warning(
                            LogTag::Ohlcv,
                            &format!(
                                "Rate limit hit while processing {}; backing off briefly",
                                mint
                            ),
                        );
                        sleep(Duration::from_secs(2)).await;
                    }
                    Err(e) => {
                        let (kind, severity) = classify_ohlcv_error(&e);
                        record_ohlcv_event(
                            "process_token_error",
                            severity,
                            Some(mint.as_str()),
                            None,
                            json!({
                                "message": format!("Error processing {}: {}", mint, e),
                                "error_kind": kind,
                            }),
                        )
                        .await;
                        logger::error(
                            LogTag::Ohlcv,
                            &format!("Error processing {}: {}", mint, e),
                        );
                    }
                }

                // Rate-limit-aware delay between tokens
                sleep(Duration::from_millis(delay_ms)).await;
            }

            self.record_monitor_cycle_end(cycle_start, processed_count)
                .await;
        }
    }

    async fn process_token(&self, mint: &str) -> OhlcvResult<()> {
        let action = {
            let active = self.active_tokens.read().await;
            let config = active
                .get(mint)
                .ok_or_else(|| OhlcvError::NotFound(mint.to_string()))?;

            // Get recommended action based on priority and activity
            PriorityManager::get_recommended_action(config)
        };

        match action {
            crate::ohlcvs::priorities::RecommendedAction::FetchNow => {
                self.fetch_token_data(mint).await?;
            }
            crate::ohlcvs::priorities::RecommendedAction::Throttle(_duration) => {
                // Skip this cycle, will fetch on next check based on timing
            }
            crate::ohlcvs::priorities::RecommendedAction::Pause => {
                // Token is paused, skip
            }
        }

        Ok(())
    }

    async fn fetch_token_data(&self, mint: &str) -> OhlcvResult<()> {
        // Check if we should try pool discovery (using backoff logic)
        let (has_pools, should_retry_discovery) = {
            let active = self.active_tokens.read().await;
            let config = active
                .get(mint)
                .ok_or_else(|| OhlcvError::NotFound(mint.to_string()))?;
            let has = config.get_best_pool().is_some();
            let should_retry = !has && config.should_retry_pool_discovery();
            (has, should_retry)
        };

        // If no pools and backoff period has elapsed, try to discover them
        if !has_pools && should_retry_discovery {
            match self.pool_manager.discover_pools(mint).await {
                Ok(discovered) if !discovered.is_empty() => {
                    let first_pool_address = discovered.first().map(|p| p.address.clone());

                    // Success! Update config with discovered pools and reset failure counter
                    let updated_config = {
                        let mut active = self.active_tokens.write().await;
                        active.get_mut(mint).map(|config| {
                            let previous_failures = config.consecutive_pool_failures;
                            config.pools = discovered.clone();
                            config.mark_pool_discovery_success();
                            (previous_failures, config.clone())
                        })
                    };

                    if let Some((previous_failures, config)) = updated_config {
                        self.db.upsert_monitor_config(&config)?;

                        logger::info(
                            LogTag::Ohlcv,
                            &format!(
                                "✅ Pool discovery succeeded for {} after {} failures",
                                mint, previous_failures
                            ),
                        );

                        record_ohlcv_event(
                            "pool_discovery_success",
                            Severity::Info,
                            Some(mint),
                            first_pool_address.as_deref(),
                            json!({
                                "message": format!(
                                    "Discovered pools for {}",
                                    mint
                                ),
                                "discovered_pools": discovered,
                                "previous_failures": previous_failures,
                            }),
                        )
                        .await;
                    }
                }
                Ok(_) | Err(_) => {
                    // Failed - update failure counter and backoff timestamp
                    let update_result = {
                        let mut active = self.active_tokens.write().await;
                        active.get_mut(mint).map(|config| {
                            let was_failure_count = config.consecutive_pool_failures;
                            config.mark_pool_discovery_failure();
                            let updated = config.clone();
                            (was_failure_count, updated)
                        })
                    };

                    if let Some((was_failure_count, config)) = update_result {
                        self.db.upsert_monitor_config(&config)?;

                        // Log with appropriate frequency (only first 3 attempts, then once at 5, 10, etc.)
                        let should_log = was_failure_count < 3 || was_failure_count % 5 == 0;

                        if should_log {
                            logger::warning(
                                LogTag::Ohlcv,
                                &format!(
                                    "⚠️  Pool discovery failed for {} (attempt {}). Next retry: {}",
                                    mint,
                                    config.consecutive_pool_failures,
                                    config.get_next_retry_description()
                                ),
                            );
                        }

                        if should_log {
                            record_ohlcv_event(
                                "pool_discovery_failed",
                                Severity::Warn,
                                Some(mint),
                                None,
                                json!({
                                    "message": format!(
                                        "Pool discovery failed for {}",
                                        mint
                                    ),
                                    "attempt": config.consecutive_pool_failures,
                                    "next_retry": config.get_next_retry_description(),
                                }),
                            )
                            .await;
                        }
                    }

                    return Err(OhlcvError::PoolNotFound(format!(
                        "No pools available for token: {}",
                        mint
                    )));
                }
            }
        } else if !has_pools {
            // In backoff period - skip silently
            return Err(OhlcvError::PoolNotFound(format!(
                "Token {} in discovery backoff period",
                mint
            )));
        }

        // Get token config with pools
        let (pool_address, priority, batch_size) = {
            let active = self.active_tokens.read().await;
            let config = active
                .get(mint)
                .ok_or_else(|| OhlcvError::NotFound(mint.to_string()))?;

            // Get best pool
            let pool = config
                .get_best_pool()
                .ok_or_else(|| OhlcvError::PoolNotFound(mint.to_string()))?;

            // Calculate batch size based on priority
            let batch_size = PriorityManager::calculate_batch_size(config.priority);

            (pool.address.clone(), config.priority, batch_size)
        };

        // Fetch 1-minute data (base timeframe) with priority-based batch size
        let data = self
            .fetcher
            .fetch_immediate(&pool_address, Timeframe::Minute1, None, batch_size)
            .await;

        match data {
            Ok(data_points) => {
                if data_points.is_empty() {
                    // Mark empty fetch
                    let updated_config = {
                        let mut active = self.active_tokens.write().await;
                        active.get_mut(mint).map(|config| {
                            config.mark_empty_fetch();
                            config.clone()
                        })
                    };

                    if let Some(config) = updated_config {
                        self.db.upsert_monitor_config(&config)?;
                    }

                    record_ohlcv_event(
                        "empty_fetch",
                        Severity::Debug,
                        Some(mint),
                        Some(pool_address.as_str()),
                        json!({
                            "message": format!(
                                "Empty OHLCV fetch for {} via {}",
                                mint, pool_address
                            ),
                            "batch_size": batch_size,
                            "priority": priority.to_string(),
                        }),
                    )
                    .await;
                } else {
                    let stored_points = self.persist_chunk(mint, &pool_address, data_points)?;

                    // Mark success
                    self.pool_manager.mark_success(mint, &pool_address).await?;

                    let updated_config = {
                        let mut active = self.active_tokens.write().await;
                        active.get_mut(mint).map(|config| {
                            config.consecutive_empty_fetches = 0;
                            config.mark_activity();
                            config.clone()
                        })
                    };

                    if let Some(config) = updated_config {
                        self.db.upsert_monitor_config(&config)?;
                    }

                    // Ensure retention window remains populated (best-effort for gap coverage)
                    if let Err(e) = self.ensure_retention_window(mint, &pool_address).await {
                        logger::warning(
                            LogTag::Ohlcv,
                            &format!(
                                "Retention backfill failed for {} via {}: {}",
                                mint, pool_address, e
                            ),
                        );
                        record_ohlcv_event(
                            "retention_backfill_failed",
                            Severity::Warn,
                            Some(mint),
                            Some(pool_address.as_str()),
                            json!({
                                "message": format!(
                                    "Retention backfill failed for {} via {}",
                                    mint, pool_address
                                ),
                                "error": e.to_string(),
                            }),
                        )
                        .await;
                    }

                    if !stored_points.is_empty() {
                        let earliest = stored_points.first().map(|p| p.timestamp);
                        let latest = stored_points.last().map(|p| p.timestamp);

                        record_ohlcv_event(
                            "fetch_success",
                            Severity::Info,
                            Some(mint),
                            Some(pool_address.as_str()),
                            json!({
                                "message": format!(
                                    "Stored {} OHLCV points for {}",
                                    stored_points.len(), mint
                                ),
                                "inserted_points": stored_points.len(),
                                "earliest_timestamp": earliest,
                                "latest_timestamp": latest,
                                "priority": priority.to_string(),
                                "batch_size": batch_size,
                            }),
                        )
                        .await;
                    }

                    // Detect and register gaps for recent data (best-effort)
                    if !stored_points.is_empty() {
                        if let Err(e) = self
                            .gap_manager
                            .detect_gaps(mint, &pool_address, Timeframe::Minute1)
                            .await
                        {
                            logger::warning(
                                LogTag::Ohlcv,
                                &format!(
                                    "Gap detection failed for {} via {}: {}",
                                    mint, pool_address, e
                                ),
                            );
                            record_ohlcv_event(
                                "gap_detection_failed",
                                Severity::Warn,
                                Some(mint),
                                Some(pool_address.as_str()),
                                json!({
                                    "message": format!(
                                        "Gap detection failed for {} via {}",
                                        mint, pool_address
                                    ),
                                    "error": e.to_string(),
                                }),
                            )
                            .await;
                        }
                    }
                }
            }
            Err(e) => {
                if !matches!(e, OhlcvError::RateLimitExceeded) {
                    // Only penalize pool health for non-rate-limit failures
                    self.pool_manager.mark_failure(mint, &pool_address).await?;
                }

                let level = if matches!(e, OhlcvError::RateLimitExceeded) {
                    "WARN"
                } else {
                    "ERROR"
                };

                if !matches!(e, OhlcvError::RateLimitExceeded) {
                    let (kind, severity) = classify_ohlcv_error(&e);
                    record_ohlcv_event(
                        "fetch_failed",
                        severity,
                        Some(mint),
                        Some(pool_address.as_str()),
                        json!({
                            "message": format!(
                                "Failed to fetch OHLCV for {} via {}: {}",
                                mint, pool_address, e
                            ),
                            "error_kind": kind,
                            "batch_size": batch_size,
                            "priority": priority.to_string(),
                        }),
                    )
                    .await;
                }

                if level == "WARN" {
                    logger::warning(
                        LogTag::Ohlcv,
                        &format!("Failed to fetch {} using pool {}: {}", mint, pool_address, e),
                    );
                } else {
                    logger::error(
                        LogTag::Ohlcv,
                        &format!("Failed to fetch {} using pool {}: {}", mint, pool_address, e),
                    );
                }
            }
        }

        Ok(())
    }

    async fn record_monitor_cycle_start(&self, token_count: usize) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.monitor_cycle_started_at = Some(Utc::now());
        telemetry.monitor_cycle_tokens_processed = token_count;
    }

    async fn record_monitor_cycle_end(&self, start: Instant, token_count: usize) {
        let duration_ms = start.elapsed().as_millis() as u64;
        let mut telemetry = self.telemetry.write().await;
        telemetry.monitor_cycle_completed_at = Some(Utc::now());
        telemetry.monitor_cycle_duration_ms = Some(duration_ms);
        telemetry.monitor_cycle_tokens_processed = token_count;
        telemetry.monitor_cycle_total = telemetry.monitor_cycle_total.saturating_add(1);
    }

    async fn record_gap_cycle_start(&self, token_count: usize) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.gap_cycle_started_at = Some(Utc::now());
        telemetry.gap_cycle_tokens_processed = token_count;
    }

    async fn record_gap_cycle_end(&self, start: Instant, token_count: usize) {
        let duration_ms = start.elapsed().as_millis() as u64;
        let mut telemetry = self.telemetry.write().await;
        telemetry.gap_cycle_completed_at = Some(Utc::now());
        telemetry.gap_cycle_duration_ms = Some(duration_ms);
        telemetry.gap_cycle_tokens_processed = token_count;
        telemetry.gap_cycle_total = telemetry.gap_cycle_total.saturating_add(1);
    }

    async fn record_rate_limit_event(&self) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.last_rate_limit_at = Some(Utc::now());
        telemetry.rate_limit_events = telemetry.rate_limit_events.saturating_add(1);
    }

    async fn record_backfill_scheduled(&self) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.total_backfills_scheduled = telemetry.total_backfills_scheduled.saturating_add(1);
        telemetry.last_backfill_started_at = Some(Utc::now());
        telemetry.last_backfill_error = None;
    }

    async fn record_backfill_completed(&self, duration_ms: u64, points: usize) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.total_backfills_completed = telemetry.total_backfills_completed.saturating_add(1);
        telemetry.last_backfill_completed_at = Some(Utc::now());
        telemetry.last_backfill_duration_ms = Some(duration_ms);
        telemetry.last_backfill_points = Some(points);
        telemetry.last_backfill_error = None;
    }

    async fn record_backfill_failed(&self, duration_ms: u64, message: String) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.total_backfills_failed = telemetry.total_backfills_failed.saturating_add(1);
        telemetry.last_backfill_completed_at = Some(Utc::now());
        telemetry.last_backfill_duration_ms = Some(duration_ms);
        telemetry.last_backfill_error = Some(message);
    }

    fn persist_chunk(
        &self,
        mint: &str,
        pool_address: &str,
        mut data_points: Vec<OhlcvDataPoint>,
    ) -> OhlcvResult<Vec<OhlcvDataPoint>> {
        if data_points.is_empty() {
            return Ok(Vec::new());
        }

        data_points.sort_by_key(|p| p.timestamp);
        data_points.dedup_by_key(|p| p.timestamp);

        self.db.insert_1m_data(mint, pool_address, &data_points)?;

        self.cache.put(
            mint,
            Some(pool_address),
            Timeframe::Minute1,
            data_points.clone(),
        )?;

        for timeframe in AGGREGATED_TIMEFRAMES.iter().copied() {
            let aggregated = OhlcvAggregator::aggregate(&data_points, timeframe)?;
            if aggregated.is_empty() {
                continue;
            }

            self.db
                .cache_aggregated_data(mint, pool_address, timeframe, &aggregated)?;
            self.cache
                .put(mint, Some(pool_address), timeframe, aggregated)?;
        }

        Ok(data_points)
    }

    async fn ensure_retention_window(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        let retention_days = with_config(|cfg| cfg.ohlcv.retention_days);
        if retention_days <= 0 {
            return Ok(());
        }

        let retention_seconds = retention_days * 86_400;
        let target_start = (Utc::now().timestamp() - retention_seconds).max(0);
        let maybe_range = match self.db.get_time_bounds(mint, pool_address)? {
            Some((oldest, _latest)) if oldest <= target_start => None,
            Some((oldest, _latest)) => Some((target_start, oldest)),
            None => {
                let now_ts = Utc::now().timestamp();
                let end = now_ts.max(target_start + 60);
                Some((target_start, end))
            }
        };

        let Some((from_ts, to_ts)) = maybe_range else {
            return Ok(());
        };

        if !self.try_start_backfill(mint) {
            return Ok(());
        }

        self.record_backfill_scheduled().await;

        record_ohlcv_event(
            "backfill_scheduled",
            Severity::Info,
            Some(mint),
            Some(pool_address),
            json!({
                "message": format!(
                    "Scheduled retention backfill for {} via {}",
                    mint, pool_address
                ),
                "from_timestamp": from_ts,
                "to_timestamp": to_ts,
                "range_seconds": to_ts.saturating_sub(from_ts),
            }),
        )
        .await;

        let runner = self.clone();
        let mint_owned = mint.to_string();
        let pool_owned = pool_address.to_string();

        logger::info(
            LogTag::Ohlcv,
            &format!(
                "Scheduling retention backfill for {} via {} (target start: {}, current oldest requested: {})",
                mint_owned,
                pool_owned,
                from_ts,
                to_ts
            ),
        );

        tokio::spawn(async move {
            match runner
                .backfill_range(&mint_owned, &pool_owned, from_ts, to_ts)
                .await
            {
                Ok(points) => {
                    logger::info(
                        LogTag::Ohlcv,
                        &format!(
                            "Retention backfill for {} via {} completed (points inserted: {})",
                            mint_owned, pool_owned, points
                        ),
                    );
                }
                Err(e) => {
                    logger::warning(
                        LogTag::Ohlcv,
                        &format!(
                            "Retention backfill failed for {} via {}: {}",
                            mint_owned, pool_owned, e
                        ),
                    );
                }
            }

            runner.finish_backfill(&mint_owned);
        });

        Ok(())
    }

    fn try_start_backfill(&self, mint: &str) -> bool {
        match self.backfill_in_progress.lock() {
            Ok(mut set) => {
                if set.contains(mint) {
                    false
                } else {
                    set.insert(mint.to_string());
                    true
                }
            }
            Err(_) => false,
        }
    }

    fn finish_backfill(&self, mint: &str) {
        if let Ok(mut set) = self.backfill_in_progress.lock() {
            set.remove(mint);
        }
    }

    async fn backfill_range(
        &self,
        mint: &str,
        pool_address: &str,
        from_timestamp: i64,
        to_timestamp: i64,
    ) -> OhlcvResult<usize> {
        if from_timestamp >= to_timestamp {
            return Ok(0);
        }

        let start = Instant::now();

        let result: OhlcvResult<usize> = async {
            let mut cursor_end = to_timestamp;
            let mut total_inserted = 0usize;

            loop {
                let chunk = self
                    .fetcher
                    .fetch_historical(pool_address, Timeframe::Minute1, from_timestamp, cursor_end)
                    .await?;

                if chunk.is_empty() {
                    break;
                }

                let earliest = chunk.first().map(|p| p.timestamp).unwrap_or(cursor_end);

                let stored = self.persist_chunk(mint, pool_address, chunk)?;
                if !stored.is_empty() {
                    total_inserted += stored.len();
                }

                if earliest <= from_timestamp {
                    break;
                }

                cursor_end = earliest.saturating_sub(Timeframe::Minute1.to_seconds());

                if cursor_end <= from_timestamp {
                    break;
                }
            }

            if total_inserted > 0 {
                self.gap_manager
                    .detect_gaps(mint, pool_address, Timeframe::Minute1)
                    .await?;
            }

            Ok(total_inserted)
        }
        .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(points) => {
                self.record_backfill_completed(duration_ms, points).await;
                record_ohlcv_event(
                    "backfill_completed",
                    Severity::Info,
                    Some(mint),
                    Some(pool_address),
                    json!({
                        "message": format!(
                            "Backfill completed for {} via {}",
                            mint, pool_address
                        ),
                        "inserted_points": points,
                        "duration_ms": duration_ms,
                        "from_timestamp": from_timestamp,
                        "to_timestamp": to_timestamp,
                        "range_seconds": to_timestamp.saturating_sub(from_timestamp),
                    }),
                )
                .await;
                Ok(points)
            }
            Err(err) => {
                let error_message = err.to_string();
                self.record_backfill_failed(duration_ms, error_message.clone())
                    .await;
                record_ohlcv_event(
                    "backfill_failed",
                    Severity::Error,
                    Some(mint),
                    Some(pool_address),
                    json!({
                        "message": format!(
                            "Backfill failed for {} via {}",
                            mint, pool_address
                        ),
                        "error": error_message,
                        "duration_ms": duration_ms,
                        "from_timestamp": from_timestamp,
                        "to_timestamp": to_timestamp,
                        "range_seconds": to_timestamp.saturating_sub(from_timestamp),
                    }),
                )
                .await;
                Err(err)
            }
        }
    }

    async fn gap_fill_loop(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(300)); // Check every 5 minutes

        loop {
            tick.tick().await;

            if *self.shutdown_signal.read().await {
                break;
            }

            // Process gap filling for active tokens
            let tokens: Vec<String> = {
                let active = self.active_tokens.read().await;
                active.keys().cloned().collect()
            };

            let processed_count = tokens.len();
            let cycle_start = Instant::now();
            self.record_gap_cycle_start(processed_count).await;

            for mint in tokens {
                // Auto-fill recent gaps (last 24h)
                if let Err(e) = self.gap_manager.auto_fill_recent_gaps(&mint).await {
                    logger::error(
                        LogTag::Ohlcv,
                        &format!("Gap fill error for {}: {}", mint, e),
                    );
                    record_ohlcv_event(
                        "gap_fill_failed",
                        Severity::Error,
                        Some(mint.as_str()),
                        None,
                        json!({
                            "message": format!(
                                "Gap fill error for {}",
                                mint
                            ),
                            "error": e.to_string(),
                        }),
                    )
                    .await;
                }

                sleep(Duration::from_secs(1)).await;
            }

            self.record_gap_cycle_end(cycle_start, processed_count)
                .await;
        }
    }

    async fn sync_pool_service_tokens(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(30)); // Every 30 seconds (Pool Service updates every 5-10s)

        loop {
            tick.tick().await;

            if *self.shutdown_signal.read().await {
                break;
            }

            // Respect configured maximum monitored tokens (0 = unlimited for backward compatibility)
            let configured_limit = with_config(|cfg| cfg.ohlcv.max_monitored_tokens);
            let max_tokens = if configured_limit == 0 {
                usize::MAX
            } else {
                configured_limit
            };

            // Get all tokens with available prices from Pool Service (same list Trader monitors)
            let available_mints = crate::pools::get_available_tokens();

            // Get open positions to determine priority
            let open_positions = match crate::positions::state::get_open_positions().await {
                positions if !positions.is_empty() => positions
                    .into_iter()
                    .map(|p| p.mint)
                    .collect::<std::collections::HashSet<_>>(),
                _ => std::collections::HashSet::new(),
            };

            let mut added = 0;
            let mut upgraded = 0;
            let mut already_monitored = 0;

            // Add or upgrade tokens from Pool Service
            for mint in &available_mints {
                let active_tokens = self.active_tokens.read().await;

                if let Some(config) = active_tokens.get(mint) {
                    // Token already monitored
                    already_monitored += 1;

                    // Upgrade to Critical if has open position
                    if open_positions.contains(mint) && config.priority != Priority::Critical {
                        drop(active_tokens);
                        if let Err(e) = self.update_priority(mint, Priority::Critical).await {
                            logger::error(
                                LogTag::Ohlcv,
                                &format!("Failed to upgrade priority for {}: {}", mint, e),
                            );
                        } else {
                            upgraded += 1;
                        }
                    }
                } else {
                    // New token - add with appropriate priority
                    drop(active_tokens);

                    let is_open_position = open_positions.contains(mint);

                    if !is_open_position && max_tokens != usize::MAX {
                        let current_len = {
                            let snapshot = self.active_tokens.read().await;
                            snapshot.len()
                        };

                        if current_len >= max_tokens {
                            continue;
                        }
                    }

                    let priority = if is_open_position {
                        Priority::Critical
                    } else {
                        Priority::Low
                    };

                    if let Err(e) = self.add_token(mint.clone(), priority).await {
                        logger::error(
                            LogTag::Ohlcv,
                            &format!("Failed to add token {}: {}", mint, e),
                        );
                    } else {
                        added += 1;
                    }
                }
            }

            // Optional: Remove tokens no longer in Pool Service
            // (Keep tokens that were manually added or have positions)
            let available_set: std::collections::HashSet<_> =
                available_mints.iter().cloned().collect();
            let mut removed = 0;

            let active_tokens = self.active_tokens.read().await;
            let tokens_to_check: Vec<String> = active_tokens.keys().cloned().collect();
            drop(active_tokens);

            for mint in tokens_to_check {
                // Don't remove if has open position
                if open_positions.contains(&mint) {
                    continue;
                }

                // Don't remove if still in Pool Service
                if available_set.contains(&mint) {
                    continue;
                }

                // Remove stale token
                if let Err(e) = self.remove_token(&mint).await {
                    logger::error(
                        LogTag::Ohlcv,
                        &format!("Failed to remove token {}: {}", mint, e),
                    );
                } else {
                    removed += 1;
                }
            }

            let mut trimmed = 0;
            if max_tokens != usize::MAX {
                let tokens_to_trim = self
                    .determine_tokens_to_trim(max_tokens, &open_positions)
                    .await;

                for mint in tokens_to_trim {
                    if let Err(e) = self.remove_token(&mint).await {
                        logger::error(
                            LogTag::Ohlcv,
                            &format!("Failed to trim token {}: {}", mint, e),
                        );
                    } else {
                        trimmed += 1;
                    }
                }
            }

            if added > 0 || upgraded > 0 || removed > 0 || trimmed > 0 {
                logger::debug(
                    LogTag::Ohlcv,
                    &format!(
                        "Pool Service sync: {} available, {} added, {} upgraded, {} removed, {} trimmed, {} already monitored",
                        available_mints.len(),
                        added,
                        upgraded,
                        removed,
                        trimmed,
                        already_monitored
                    ),
                );
            }
        }
    }

    async fn determine_tokens_to_trim(
        &self,
        max_tokens: usize,
        open_positions: &std::collections::HashSet<String>,
    ) -> Vec<String> {
        if max_tokens == usize::MAX {
            return Vec::new();
        }

        let active = self.active_tokens.read().await;
        let active_len = active.len();

        if active_len <= max_tokens {
            return Vec::new();
        }

        let mut candidates: Vec<(String, Priority, chrono::DateTime<Utc>, u32)> = active
            .iter()
            .filter(|(mint, _)| !open_positions.contains(*mint))
            .map(|(mint, config)| {
                (
                    mint.clone(),
                    config.priority,
                    config.last_activity,
                    config.consecutive_empty_fetches,
                )
            })
            .collect();

        let mut current_size = active_len;
        drop(active);

        candidates.sort_by(|a, b| {
            a.1.cmp(&b.1)
                .then_with(|| a.2.cmp(&b.2))
                .then_with(|| b.3.cmp(&a.3))
        });

        let mut to_remove = Vec::new();
        for (mint, _, _, _) in candidates {
            if current_size <= max_tokens {
                break;
            }
            to_remove.push(mint);
            current_size -= 1;
        }

        to_remove
    }

    async fn cleanup_loop(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(3600)); // Every hour

        loop {
            tick.tick().await;

            if *self.shutdown_signal.read().await {
                break;
            }

            let retention_days = with_config(|cfg| cfg.ohlcv.retention_days);

            if retention_days > 0 {
                if let Err(e) = self.db.cleanup_old_data(retention_days) {
                    logger::error(LogTag::Ohlcv, &format!("Cleanup error: {}", e));
                    record_ohlcv_event(
                        "cleanup_failed",
                        Severity::Error,
                        None,
                        None,
                        json!({
                            "message": "Failed to cleanup OHLCV database",
                            "error": e.to_string(),
                            "retention_days": retention_days,
                        }),
                    )
                    .await;
                }
            }
        }
    }

    async fn cache_maintenance_loop(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(600)); // Every 10 minutes

        loop {
            tick.tick().await;

            if *self.shutdown_signal.read().await {
                break;
            }

            // Clean up expired cache entries
            if let Err(e) = self.cache.cleanup_expired() {
                logger::error(
                    LogTag::Ohlcv,
                    &format!("Cache cleanup error: {}", e),
                );
                record_ohlcv_event(
                    "cache_cleanup_failed",
                    Severity::Error,
                    None,
                    None,
                    json!({
                        "message": "Failed to cleanup OHLCV cache",
                        "error": e.to_string(),
                    }),
                )
                .await;
            }
        }
    }
}

impl Clone for OhlcvMonitor {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            fetcher: Arc::clone(&self.fetcher),
            cache: Arc::clone(&self.cache),
            pool_manager: Arc::clone(&self.pool_manager),
            gap_manager: Arc::clone(&self.gap_manager),
            active_tokens: Arc::clone(&self.active_tokens),
            shutdown_signal: Arc::clone(&self.shutdown_signal),
            backfill_in_progress: Arc::clone(&self.backfill_in_progress),
            telemetry: Arc::clone(&self.telemetry),
        }
    }
}

fn classify_ohlcv_error(error: &OhlcvError) -> (&'static str, Severity) {
    match error {
        OhlcvError::DatabaseError(_) => ("database_error", Severity::Error),
        OhlcvError::ApiError(_) => ("api_error", Severity::Error),
        OhlcvError::RateLimitExceeded => ("rate_limit", Severity::Warn),
        OhlcvError::PoolNotFound(_) => ("pool_not_found", Severity::Warn),
        OhlcvError::InvalidTimeframe(_) => ("invalid_timeframe", Severity::Error),
        OhlcvError::DataGap { .. } => ("data_gap", Severity::Warn),
        OhlcvError::CacheError(_) => ("cache_error", Severity::Error),
        OhlcvError::NotFound(_) => ("not_found", Severity::Warn),
    }
}

fn count_by_priority(configs: &HashMap<String, TokenOhlcvConfig>) -> HashMap<Priority, usize> {
    let mut counts = HashMap::new();
    for config in configs.values() {
        *counts.entry(config.priority).or_insert(0) += 1;
    }
    counts
}

#[derive(Debug, Clone)]
pub struct MonitorStats {
    pub total_tokens: usize,
    pub critical_tokens: usize,
    pub high_tokens: usize,
    pub medium_tokens: usize,
    pub low_tokens: usize,
    pub cache_hit_rate: f64,
    pub api_calls_per_minute: f64,
    pub queue_size: usize,
    pub telemetry: MonitorTelemetrySnapshot,
    pub backfills_in_progress: usize,
    pub open_gap_tokens: usize,
    pub open_gap_total: usize,
    pub top_open_gaps: Vec<MintGapAggregate>,
}

impl Default for MonitorStats {
    fn default() -> Self {
        Self {
            total_tokens: 0,
            critical_tokens: 0,
            high_tokens: 0,
            medium_tokens: 0,
            low_tokens: 0,
            cache_hit_rate: 0.0,
            api_calls_per_minute: 0.0,
            queue_size: 0,
            telemetry: MonitorTelemetrySnapshot::default(),
            backfills_in_progress: 0,
            open_gap_tokens: 0,
            open_gap_total: 0,
            top_open_gaps: Vec::new(),
        }
    }
}
