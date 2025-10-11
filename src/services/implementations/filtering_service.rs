use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::config::with_config;
use crate::filtering;
use crate::logger::{log, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};

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
        with_config(|cfg| cfg.filtering.filter_cache_ttl_secs.max(5))
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
        vec!["pool_helpers", "token_discovery", "security"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        filtering::refresh().await
    }

    async fn start(
        &mut self,
        shutdown: Arc<tokio::sync::Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<tokio::task::JoinHandle<()>>, String> {
        let operations = Arc::clone(&self.operations);
        let errors = Arc::clone(&self.errors);

        let handle = tokio::spawn(monitor.instrument(async move {
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
                        log(LogTag::Filtering, "REFRESH_ERROR", &err);
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
