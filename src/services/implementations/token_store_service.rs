use crate::logger::{log, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use crate::tokens::database::TokenDatabase;
use crate::tokens::store::get_global_token_store;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio_metrics::TaskMonitor;

// Timing constants
const CACHE_REFRESH_INTERVAL_MINUTES: u64 = 5;

/// Background service responsible for keeping the token store hot.
#[derive(Default)]
pub struct TokenStoreService {
    initialized: bool,
}

impl TokenStoreService {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    fn refresh_interval() -> Duration {
        Duration::from_secs(CACHE_REFRESH_INTERVAL_MINUTES * 60)
    }
}

#[async_trait]
impl Service for TokenStoreService {
    fn name(&self) -> &'static str {
        "tokens_store"
    }

    fn priority(&self) -> i32 {
        15
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["events"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        let store = get_global_token_store();
        if !store.is_database_configured() {
            let db =
                TokenDatabase::new().map_err(|e| format!("Token store DB init failed: {}", e))?;
            store.configure_database(db);
            log(
                LogTag::Tokens,
                "INIT",
                "Token store database configured by TokenStoreService",
            );
        } else {
            log(
                LogTag::Tokens,
                "INIT",
                "Token store database already configured; reusing existing handle",
            );
        }
        self.initialized = true;
        log(LogTag::Tokens, "INIT", "Token store service initialized");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<tokio::sync::Notify>,
        monitor: TaskMonitor,
    ) -> Result<Vec<tokio::task::JoinHandle<()>>, String> {
        if !self.initialized {
            return Err("Token store not initialized".into());
        }

        let store = get_global_token_store();
        let monitor_refresh = monitor.clone();
        let handle = tokio::spawn(monitor_refresh.instrument(async move {
            if let Err(err) = store.refresh_all_from_database().await {
                log(
                    LogTag::Cache,
                    "ERROR",
                    &format!("Initial token store refresh failed: {}", err),
                );
            } else {
                log(LogTag::Cache, "INFO", "Token store cache hydrated from database");
            }

            loop {
                let refresh_duration = TokenStoreService::refresh_interval();
                tokio::select! {
                    _ = shutdown.notified() => {
                        log(LogTag::Cache, "INFO", "Token store service shutting down");
                        break;
                    }
                    _ = sleep(refresh_duration) => {
                        if let Err(err) = store.refresh_all_from_database().await {
                            log(LogTag::Cache, "WARN", &format!("Token store refresh failed: {}", err));
                        }
                    }
                }
            }
        }));

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        if get_global_token_store().len() == 0 {
            ServiceHealth::Starting
        } else {
            ServiceHealth::Healthy
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        let store = get_global_token_store();
        let metrics = store.metrics_snapshot();
        let mut service_metrics = ServiceMetrics::default();
        service_metrics
            .custom_metrics
            .insert("tokens_total".into(), store.len() as f64);
        service_metrics.custom_metrics.insert(
            "token_store_last_full_refresh".into(),
            metrics.last_full_refresh_unix as f64,
        );
        service_metrics.custom_metrics.insert(
            "token_store_last_delta_refresh".into(),
            metrics.last_delta_refresh_unix as f64,
        );
        service_metrics
            .custom_metrics
            .insert("token_store_updates".into(), metrics.total_updates as f64);
        service_metrics
            .custom_metrics
            .insert("token_store_inserts".into(), metrics.total_inserts as f64);
        service_metrics
            .custom_metrics
            .insert("token_store_removals".into(), metrics.total_removals as f64);
        service_metrics
    }
}
