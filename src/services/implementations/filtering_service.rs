use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::filtering;
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};

// Timing constants
const FILTER_CACHE_TTL_SECS: u64 = 30;

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
                    _ = shutdown.notified() => break,
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

        Ok(vec![handle])
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
