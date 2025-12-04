use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct PoolFetcherService;

impl Default for PoolFetcherService {
  fn default() -> Self {
    Self
  }
}

#[async_trait]
impl Service for PoolFetcherService {
  fn name(&self) -> &'static str {
    "pool_fetcher"
  }

  fn priority(&self) -> i32 {
    101
  }

  fn dependencies(&self) -> Vec<&'static str> {
    vec![
      "transactions",
      "pool_helpers",
      "pool_discovery",
      "filtering",
    ]
  }

  fn is_enabled(&self) -> bool {
    crate::global::is_initialization_complete()
  }

  async fn initialize(&mut self) -> Result<(), String> {
    logger::info(
      LogTag::PoolService,
      &"Initializing pool fetcher service...".to_string(),
    );
    Ok(())
  }

  async fn start(
    &mut self,
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
  ) -> Result<Vec<JoinHandle<()>>, String> {
    logger::info(
      LogTag::PoolService,
      &"Starting pool fetcher service...".to_string(),
    );

    // Get the AccountFetcher component from global state
    let fetcher = crate::pools::get_account_fetcher()
      .ok_or("AccountFetcher component not initialized".to_string())?;

    // Spawn fetcher task
    let handle = tokio::spawn(monitor.instrument(async move {
      fetcher.start_fetcher_task(shutdown).await;
    }));

    logger::info(
      LogTag::PoolService,
 &"Pool fetcher service started (instrumented)".to_string(),
    );

    Ok(vec![handle])
  }

  async fn stop(&mut self) -> Result<(), String> {
    logger::info(
      LogTag::PoolService,
      &"Pool fetcher service stopping (via shutdown signal)".to_string(),
    );
    Ok(())
  }

  async fn health(&self) -> ServiceHealth {
    if crate::pools::get_account_fetcher().is_some() {
      ServiceHealth::Healthy
    } else {
      ServiceHealth::Unhealthy("AccountFetcher component not available".to_string())
    }
  }

  async fn metrics(&self) -> ServiceMetrics {
    let mut metrics = ServiceMetrics::default();

    // Get metrics from the component if available
    if let Some(fetcher) = crate::pools::get_account_fetcher() {
      let (operations, errors, accounts_fetched, rpc_batches) = fetcher.get_metrics();
      metrics.operations_total = operations;
      metrics.errors_total = errors;
      metrics
        .custom_metrics
        .insert("accounts_fetched".to_string(), accounts_fetched as f64);
      metrics
        .custom_metrics
        .insert("rpc_batches".to_string(), rpc_batches as f64);
      if operations > 0 {
        metrics.custom_metrics.insert(
          "avg_accounts_per_cycle".to_string(),
          accounts_fetched as f64 / operations as f64,
        );
      }
      if rpc_batches > 0 {
        metrics.custom_metrics.insert(
          "avg_accounts_per_batch".to_string(),
          accounts_fetched as f64 / rpc_batches as f64,
        );
      }
    }

    metrics
  }
}
