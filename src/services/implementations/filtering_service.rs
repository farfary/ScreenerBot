//! Token filtering service — delegates to background workers for refresh and cleanup.
//!
//! This service acts as a thin wrapper that:
//! - Spawns background workers from `crate::filtering::background`
//! - Tracks metrics (operations, errors)
//! - Provides health checks via snapshot age

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use crate::filtering;
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
        vec!["tokens", "pools"]
    }

    fn is_enabled(&self) -> bool {
        // Discovery tier: filtering applies to the discovered token list and needs no
        // wallet/RPC, so it runs in full mode OR discovery-only mode. The declared
        // pools dependency is an ordering hint and is filtered out when disabled.
        crate::global::is_discovery_or_full()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        // Don't refresh during init - it blocks startup for 20+ seconds with 11k tokens
        // The background task will do the first refresh immediately after start
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<tokio::sync::Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<tokio::task::JoinHandle<()>>> {
        let operations = Arc::clone(&self.operations);
        let errors = Arc::clone(&self.errors);

        // Main filtering refresh task
        let shutdown_refresh = Arc::clone(&shutdown);
        let handle = tokio::spawn(monitor.instrument(async move {
            crate::filtering::background::run_refresh_loop(shutdown_refresh, operations, errors)
                .await;
        }));

        // Rejection history and stats cleanup task
        let shutdown_cleanup = Arc::clone(&shutdown);
        let cleanup_handle = tokio::spawn(async move {
            crate::filtering::background::run_cleanup_loop(shutdown_cleanup).await;
        });

        Ok(vec![handle, cleanup_handle])
    }

    async fn stop(&mut self) -> crate::Result<()> {
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        let max_age =
            Duration::from_secs(crate::filtering::background::snapshot_stale_limit_secs());
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
