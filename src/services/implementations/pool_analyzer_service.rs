use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct PoolAnalyzerService;

impl Default for PoolAnalyzerService {
  fn default() -> Self {
    Self
  }
}

#[async_trait]
impl Service for PoolAnalyzerService {
  fn name(&self) -> &'static str {
    "pool_analyzer"
  }

  fn priority(&self) -> i32 {
    103
  }

  fn dependencies(&self) -> Vec<&'static str> {
    vec!["pool_helpers", "pool_fetcher", "filtering"]
  }

  fn is_enabled(&self) -> bool {
    crate::global::is_initialization_complete()
  }

  async fn initialize(&mut self) -> Result<(), String> {
    logger::info(
      LogTag::PoolService,
      &"Initializing pool analyzer service...".to_string(),
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
      &"Starting pool analyzer service...".to_string(),
    );

    // Get the PoolAnalyzer component from global state
    let analyzer = crate::pools::get_pool_analyzer()
      .ok_or("PoolAnalyzer component not initialized".to_string())?;

    // Spawn analyzer task
    let handle = tokio::spawn(monitor.instrument(async move {
      analyzer.start_analyzer_task(shutdown).await;
    }));

    logger::info(
      LogTag::PoolService,
 &"Pool analyzer service started (instrumented)".to_string(),
    );

    Ok(vec![handle])
  }

  async fn stop(&mut self) -> Result<(), String> {
    logger::info(
      LogTag::PoolService,
      &"Pool analyzer service stopping (via shutdown signal)".to_string(),
    );
    Ok(())
  }

  async fn health(&self) -> ServiceHealth {
    if crate::pools::get_pool_analyzer().is_some() {
      ServiceHealth::Healthy
    } else {
      ServiceHealth::Unhealthy("PoolAnalyzer component not available".to_string())
    }
  }

  async fn metrics(&self) -> ServiceMetrics {
    let mut metrics = ServiceMetrics::default();

    // Get metrics from the component if available
    if let Some(analyzer) = crate::pools::get_pool_analyzer() {
      let (operations, errors, pools_analyzed) = analyzer.get_metrics();
      metrics.operations_total = operations;
      metrics.errors_total = errors;
      metrics
        .custom_metrics
        .insert("pools_analyzed".to_string(), pools_analyzed as f64);
      if operations > 0 {
        metrics.custom_metrics.insert(
          "success_rate".to_string(),
          (pools_analyzed as f64 / operations as f64) * 100.0,
        );
      }
    }

    metrics
  }
}
