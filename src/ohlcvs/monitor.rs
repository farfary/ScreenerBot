// Background monitoring service

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

        // Load pools for this token
        let pools = self.pool_manager.get_pools(&mint).await?;
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
        let mut active = self.active_tokens.write().await;
        if let Some(mut config) = active.remove(mint) {
            config.is_active = false;
            self.db.upsert_monitor_config(&config)?;
        }

        Ok(())
    }

    /// Update token priority
    pub async fn update_priority(&self, mint: &str, priority: Priority) -> OhlcvResult<()> {
        let mut active = self.active_tokens.write().await;
        if let Some(config) = active.get_mut(mint) {
            config.priority = priority;
            config.fetch_frequency = priority.base_interval();
            self.db.upsert_monitor_config(config)?;
        }

        Ok(())
    }

    /// Record activity for a token
    pub async fn record_activity(
        &self,
        mint: &str,
        activity_type: ActivityType,
    ) -> OhlcvResult<()> {
        let mut active = self.active_tokens.write().await;
        if let Some(config) = active.get_mut(mint) {
            config.mark_activity();

            // Update priority based on activity
            let new_priority =
                PriorityManager::update_priority_on_activity(config.priority, activity_type);
            config.priority = new_priority;
            config.fetch_frequency = new_priority.base_interval();

            self.db.upsert_monitor_config(config)?;

            // Trigger immediate fetch for high-priority activities
            if matches!(
                activity_type,
                ActivityType::PositionOpened | ActivityType::DataRequested
            ) {
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
                    eprintln!("[OHLCV Monitor] Error processing {}: {}", mint, e);
                }

                // Small delay between tokens
                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    async fn process_token(&self, mint: &str) -> OhlcvResult<()> {
        let should_fetch = {
            let active = self.active_tokens.read().await;
            let config = active
                .get(mint)
                .ok_or_else(|| OhlcvError::NotFound(mint.to_string()))?;

            // Check if it's time to fetch
            let action = PriorityManager::get_recommended_action(config);
            matches!(
                action,
                crate::ohlcvs::priorities::RecommendedAction::FetchNow
            )
        };

        if should_fetch {
            self.fetch_token_data(mint).await?;
        }

        Ok(())
    }

    async fn fetch_token_data(&self, mint: &str) -> OhlcvResult<()> {
        // Get token config
        let (pool_address, priority) = {
            let active = self.active_tokens.read().await;
            let config = active
                .get(mint)
                .ok_or_else(|| OhlcvError::NotFound(mint.to_string()))?;

            // Get best pool
            let pool = config
                .get_best_pool()
                .ok_or_else(|| OhlcvError::PoolNotFound(mint.to_string()))?;

            (pool.address.clone(), config.priority)
        };

        // Fetch 1-minute data (base timeframe)
        let data = self
            .fetcher
            .fetch_immediate(&pool_address, Timeframe::Minute1, None, 1000)
            .await;

        match data {
            Ok(data_points) => {
                if data_points.is_empty() {
                    // Mark empty fetch
                    let mut active = self.active_tokens.write().await;
                    if let Some(config) = active.get_mut(mint) {
                        config.mark_empty_fetch();
                        self.db.upsert_monitor_config(config)?;
                    }
                } else {
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

                    let mut active = self.active_tokens.write().await;
                    if let Some(config) = active.get_mut(mint) {
                        config.consecutive_empty_fetches = 0;
                        self.db.upsert_monitor_config(config)?;
                    }
                }
            }
            Err(e) => {
                // Mark failure
                self.pool_manager.mark_failure(mint, &pool_address).await?;
                eprintln!("[OHLCV Monitor] Failed to fetch {}: {}", mint, e);
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
                    eprintln!("[OHLCV Monitor] Gap fill error for {}: {}", mint, e);
                }

                sleep(Duration::from_secs(1)).await;
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
                eprintln!("[OHLCV Monitor] Cleanup error: {}", e);
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
                eprintln!("[OHLCV Monitor] Cache cleanup error: {}", e);
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
