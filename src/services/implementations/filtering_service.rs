use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::filtering;
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use crate::tokens::cleanup_rejection_history_async;

// Timing constants
const FILTER_CACHE_TTL_SECS: u64 = 30;
// Cleanup rejection history every 10 minutes to prevent unbounded growth
const REJECTION_HISTORY_CLEANUP_INTERVAL_SECS: u64 = 600;
// Keep only 24 hours of rejection history (data grows ~5GB/day otherwise)
const REJECTION_HISTORY_HOURS_TO_KEEP: i64 = 24;

pub struct FilteringService {
    operations: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
}

impl FilteringService {
    pub fn new() -> Self {
        Self {
            operations: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
        }
    }

    fn refresh_interval_secs() -> u64 {
        FILTER_CACHE_TTL_SECS
    }

    fn snapshot_stale_limit_secs() -> u64 {
        Self::refresh_interval_secs() * 4
    }
}

impl Default for FilteringService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for FilteringService {
    fn name(&self) -> &'static str {
        "filtering"
    }

    fn priority(&self) -> i32 {
        90
    }

    fn dependencies(&self) -> Vec<&'static str> {
        // Note: tokens service handles all token data including store, discovery, and security
        vec!["tokens", "pool_helpers"]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> Result<(), String> {
        // Don't refresh during init - it blocks startup for 20+ seconds with 11k tokens
        // The background task will do the first refresh immediately after start
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<tokio::sync::Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<tokio::task::JoinHandle<()>>, String> {
        let operations = Arc::clone(&self.operations);
        let errors = Arc::clone(&self.errors);

        // Main filtering refresh task
        let shutdown_refresh = Arc::clone(&shutdown);
        let handle = tokio::spawn(monitor.instrument(async move {
            // Do first refresh immediately on start (async, doesn't block other services)
            logger::info(LogTag::Filtering, "Starting initial filtering refresh...");
            match filtering::refresh().await {
                Ok(_) => {
                    operations.fetch_add(1, Ordering::Relaxed);
                    logger::info(LogTag::Filtering, "Initial filtering refresh complete");
                }
                Err(err) => {
                    errors.fetch_add(1, Ordering::Relaxed);
                    logger::warning(
                        LogTag::Filtering,
                        &format!("Initial refresh failed: {}", err),
                    );
                }
            }

            // Then continue with periodic refresh loop
            loop {
                let interval_secs = FilteringService::refresh_interval_secs();
                let sleep_duration = Duration::from_secs(interval_secs);

                tokio::select! {
                    _ = shutdown_refresh.notified() => break,
                    _ = tokio::time::sleep(sleep_duration) => {}
                }

                match filtering::refresh().await {
                    Ok(_) => {
                        operations.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(err) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                        logger::warning(LogTag::Filtering, &err);
                        tokio::time::sleep(Duration::from_secs(3)).await;
                    }
                }
            }
        }));

        // Rejection history cleanup task - prevents unbounded database growth
        let shutdown_cleanup = Arc::clone(&shutdown);
        let cleanup_handle = tokio::spawn(async move {
            // Do initial cleanup immediately on start
            logger::info(LogTag::Filtering, "Running initial rejection history cleanup...");
            match cleanup_rejection_history_async(REJECTION_HISTORY_HOURS_TO_KEEP).await {
                Ok(deleted) => {
                    if deleted > 0 {
                        logger::info(
                            LogTag::Filtering,
                            &format!("Initial cleanup: removed {} old rejection history entries", deleted),
                        );
                    }
                }
                Err(e) => {
                    logger::warning(
                        LogTag::Filtering,
                        &format!("Initial rejection history cleanup failed: {}", e),
                    );
                }
            }

            // Then continue with periodic cleanup loop
            loop {
                tokio::select! {
                    _ = shutdown_cleanup.notified() => break,
                    _ = tokio::time::sleep(Duration::from_secs(REJECTION_HISTORY_CLEANUP_INTERVAL_SECS)) => {}
                }

                match cleanup_rejection_history_async(REJECTION_HISTORY_HOURS_TO_KEEP).await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            logger::debug(
                                LogTag::Filtering,
                                &format!("Rejection history cleanup: removed {} entries older than {}h", deleted, REJECTION_HISTORY_HOURS_TO_KEEP),
                            );
                        }
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::Filtering,
                            &format!("Rejection history cleanup failed: {}", e),
                        );
                    }
                }
            }
        });

        Ok(vec![handle, cleanup_handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        let max_age = Duration::from_secs(Self::snapshot_stale_limit_secs());
        let store = filtering::global_store();

        match store.snapshot_age().await {
            Some(age) if age <= max_age => ServiceHealth::Healthy,
            Some(age) => ServiceHealth::Degraded(format!("snapshot_age_secs={}", age.as_secs())),
            None => ServiceHealth::Starting,
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        let mut metrics = ServiceMetrics::default();
        metrics.operations_total = self.operations.load(Ordering::Relaxed);
        metrics.errors_total = self.errors.load(Ordering::Relaxed);
        metrics
    }
}
