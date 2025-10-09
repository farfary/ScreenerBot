// Background monitoring service

use crate::global::is_debug_ohlcv_enabled;
use crate::logger::{log, LogTag};
use crate::ohlcvs::aggregator::OhlcvAggregator;
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::OhlcvDatabase;
use crate::ohlcvs::fetcher::OhlcvFetcher;
use crate::ohlcvs::gaps::GapManager;
use crate::ohlcvs::manager::PoolManager;
use crate::ohlcvs::priorities::{ActivityType, PriorityManager};
use crate::ohlcvs::types::{OhlcvError, OhlcvResult, Priority, Timeframe, TokenOhlcvConfig};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep, Duration, Instant};

pub struct OhlcvMonitor {
    db: Arc<OhlcvDatabase>,
    fetcher: Arc<OhlcvFetcher>,
    cache: Arc<OhlcvCache>,
    pool_manager: Arc<PoolManager>,
    gap_manager: Arc<GapManager>,
    active_tokens: Arc<RwLock<HashMap<String, TokenOhlcvConfig>>>,
    shutdown_signal: Arc<RwLock<bool>>,
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
                        if is_debug_ohlcv_enabled() {
                            log(
                                LogTag::Ohlcv,
                                "WARN",
                                &format!("Warning: Pool discovery failed for {}: {}", mint, e),
                            );
                        }
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

        MonitorStats {
            total_tokens,
            critical_tokens: by_priority.get(&Priority::Critical).copied().unwrap_or(0),
            high_tokens: by_priority.get(&Priority::High).copied().unwrap_or(0),
            medium_tokens: by_priority.get(&Priority::Medium).copied().unwrap_or(0),
            low_tokens: by_priority.get(&Priority::Low).copied().unwrap_or(0),
            cache_hit_rate: self.cache.hit_rate(),
            api_calls_per_minute: self.fetcher.calls_per_minute(),
            queue_size: self.fetcher.queue_size(),
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

            for mint in tokens {
                if let Err(e) = self.process_token(&mint).await {
                    log(
                        LogTag::Ohlcv,
                        "ERROR",
                        &format!("Error processing {}: {}", mint, e),
                    );
                }

                // Small delay between tokens
                sleep(Duration::from_millis(100)).await;
            }
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
                    // Success! Update config with discovered pools and reset failure counter
                    let updated_config = {
                        let mut active = self.active_tokens.write().await;
                        active.get_mut(mint).map(|config| {
                            config.pools = discovered.clone();
                            config.mark_pool_discovery_success();
                            config.clone()
                        })
                    };

                    if let Some(config) = updated_config {
                        self.db.upsert_monitor_config(&config)?;

                        if is_debug_ohlcv_enabled() {
                            log(
                                LogTag::Ohlcv,
                                "INFO",
                                &format!(
                                    "✅ Pool discovery succeeded for {} after {} failures",
                                    mint, config.consecutive_pool_failures
                                ),
                            );
                        }
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

                        if should_log && is_debug_ohlcv_enabled() {
                            log(
                                LogTag::Ohlcv,
                                "WARN",
                                &format!(
                                    "⚠️  Pool discovery failed for {} (attempt {}). Next retry: {}",
                                    mint,
                                    config.consecutive_pool_failures,
                                    config.get_next_retry_description()
                                ),
                            );
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
                } else {
                    // Ensure ascending timestamp order for consistency
                    let mut data_points = data_points;
                    data_points.sort_by_key(|p| p.timestamp);

                    // Store data
                    self.db.insert_1m_data(mint, &pool_address, &data_points)?;

                    // Update cache
                    self.cache.put(
                        mint,
                        Some(&pool_address),
                        Timeframe::Minute1,
                        data_points.clone(),
                    )?;

                    // Generate aggregated timeframes and cache them
                    for timeframe in &[
                        Timeframe::Minute5,
                        Timeframe::Minute15,
                        Timeframe::Hour1,
                        Timeframe::Hour4,
                        Timeframe::Hour12,
                        Timeframe::Day1,
                    ] {
                        if let Ok(aggregated) = OhlcvAggregator::aggregate(&data_points, *timeframe)
                        {
                            self.db.cache_aggregated_data(
                                mint,
                                &pool_address,
                                *timeframe,
                                &aggregated,
                            )?;
                            self.cache
                                .put(mint, Some(&pool_address), *timeframe, aggregated)?;
                        }
                    }

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
                }
            }
            Err(e) => {
                // Mark failure
                self.pool_manager.mark_failure(mint, &pool_address).await?;
                log(
                    LogTag::Ohlcv,
                    "ERROR",
                    &format!("Failed to fetch {}: {}", mint, e),
                );
            }
        }

        Ok(())
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

            for mint in tokens {
                // Auto-fill recent gaps (last 24h)
                if let Err(e) = self.gap_manager.auto_fill_recent_gaps(&mint).await {
                    log(
                        LogTag::Ohlcv,
                        "ERROR",
                        &format!("Gap fill error for {}: {}", mint, e),
                    );
                }

                sleep(Duration::from_secs(1)).await;
            }
        }
    }

    async fn sync_pool_service_tokens(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(30)); // Every 30 seconds (Pool Service updates every 5-10s)

        loop {
            tick.tick().await;

            if *self.shutdown_signal.read().await {
                break;
            }

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
                            log(
                                LogTag::Ohlcv,
                                "ERROR",
                                &format!("Failed to upgrade priority for {}: {}", mint, e),
                            );
                        } else {
                            upgraded += 1;
                        }
                    }
                } else {
                    // New token - add with appropriate priority
                    drop(active_tokens);

                    let priority = if open_positions.contains(mint) {
                        Priority::Critical
                    } else {
                        Priority::Low
                    };

                    if let Err(e) = self.add_token(mint.clone(), priority).await {
                        log(
                            LogTag::Ohlcv,
                            "ERROR",
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
                    log(
                        LogTag::Ohlcv,
                        "ERROR",
                        &format!("Failed to remove token {}: {}", mint, e),
                    );
                } else {
                    removed += 1;
                }
            }

            if added > 0 || upgraded > 0 || removed > 0 {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "SYNC",
                        &format!(
                            "Pool Service sync: {} available, {} added, {} upgraded, {} removed, {} already monitored",
                            available_mints.len(),
                            added,
                            upgraded,
                            removed,
                            already_monitored
                        )
                    );
                }
            }
        }
    }

    async fn cleanup_loop(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(3600)); // Every hour

        loop {
            tick.tick().await;

            if *self.shutdown_signal.read().await {
                break;
            }

            // Clean up old data (7 days retention)
            if let Err(e) = self.db.cleanup_old_data(7) {
                log(LogTag::Ohlcv, "ERROR", &format!("Cleanup error: {}", e));
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
                log(
                    LogTag::Ohlcv,
                    "ERROR",
                    &format!("Cache cleanup error: {}", e),
                );
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
        }
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
}
