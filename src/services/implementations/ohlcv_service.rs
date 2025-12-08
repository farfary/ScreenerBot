use crate::logger::{self, LogTag};
use crate::ohlcvs::{ActivityType, Priority};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

/// OHLCV (Open, High, Low, Close, Volume) data collection service
///
/// Manages multi-timeframe OHLCV data with priority-based monitoring.
/// Provides intelligent caching, gap detection, and multi-pool support.
pub struct OhlcvService;

impl Default for OhlcvService {
  fn default() -> Self {
    Self
  }
}

#[async_trait]
impl Service for OhlcvService {
  fn name(&self) -> &'static str {
    "ohlcv"
  }

  fn priority(&self) -> i32 {
    45
  }

  fn dependencies(&self) -> Vec<&'static str> {
    vec!["tokens", "positions"]
  }

  fn is_enabled(&self) -> bool {
    crate::global::is_initialization_complete()
  }

  async fn initialize(&mut self) -> Result<(), String> {
    crate::ohlcvs::OhlcvService::initialize()
      .await
      .map_err(|e| format!("Failed to initialize OHLCV service: {}", e))?;
    Ok(())
  }

  async fn start(
    &mut self,
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
  ) -> Result<Vec<JoinHandle<()>>, String> {
    let mut handles = crate::ohlcvs::OhlcvService::start(shutdown.clone(), monitor.clone())
      .await
      .map_err(|e| format!("Failed to start OHLCV runtime: {}", e))?;

    let autop_monitor = monitor.clone();
    let autop_shutdown = shutdown.clone();
    let autop_handle = tokio::spawn(
      autop_monitor.instrument(async move {
        use tokio::time::{ Duration, sleep };

        tokio::select! {
          _ = autop_shutdown.notified() => {
            logger::info(LogTag::Ohlcv, &"AUTO_POPULATE_EXIT: Shutdown received before OHLCV auto-populate".to_string());
            return;
          }
          _ = sleep(Duration::from_secs(5)) => {}
        }
        logger::debug(LogTag::Ohlcv, &"AUTO_POPULATE: Adding open positions to OHLCV monitoring...".to_string());

        let open_positions = crate::positions::get_open_positions().await;
        for position in &open_positions {
          if let Err(e) = crate::ohlcvs::add_token_monitoring(&position.mint, Priority::Critical).await {
            logger::error(LogTag::Ohlcv, &format!("Failed to add {} to monitoring: {}", position.mint, e));
            continue;
          }

          if let Err(e) = crate::ohlcvs::record_activity(&position.mint, ActivityType::PositionOpened).await {
            logger::error(LogTag::Ohlcv, &format!("Failed to record activity for {}: {}", position.mint, e));
          }
        }

        logger::debug(
          LogTag::Ohlcv,
          &format!("AUTO_POPULATE_DONE: Added {} open positions to OHLCV monitoring", open_positions.len()),
        );
      })
    );

    handles.push(autop_handle);

    Ok(handles)
  }

  async fn health(&self) -> ServiceHealth {
    // Check if OHLCV service is operational
    let metrics = crate::ohlcvs::get_metrics().await;
    if metrics.tokens_monitored > 0 || metrics.data_points_stored > 0 {
      ServiceHealth::Healthy
    } else {
      ServiceHealth::Starting
    }
  }

  async fn metrics(&self) -> ServiceMetrics {
    let ohlcv_metrics = crate::ohlcvs::get_metrics().await;

    // OHLCV doesn't track operations/errors in the traditional sense,
    // but we can use data points stored and gaps filled as proxies
    let mut service_metrics = ServiceMetrics::default();
    service_metrics.operations_total = ohlcv_metrics.data_points_stored as u64;

    // Map OHLCV metrics to custom metrics
    service_metrics.custom_metrics.insert(
      "tokens_monitored".to_string(),
      ohlcv_metrics.tokens_monitored as f64,
    );
    service_metrics.custom_metrics.insert(
      "pools_tracked".to_string(),
      ohlcv_metrics.pools_tracked as f64,
    );
    service_metrics.custom_metrics.insert(
      "api_calls_per_minute".to_string(),
      ohlcv_metrics.api_calls_per_minute,
    );
    service_metrics
      .custom_metrics
      .insert("cache_hit_rate".to_string(), ohlcv_metrics.cache_hit_rate);
    service_metrics.custom_metrics.insert(
      "average_fetch_latency_ms".to_string(),
      ohlcv_metrics.average_fetch_latency_ms,
    );
    service_metrics.custom_metrics.insert(
      "gaps_detected".to_string(),
      ohlcv_metrics.gaps_detected as f64,
    );
    service_metrics
      .custom_metrics
      .insert("gaps_filled".to_string(), ohlcv_metrics.gaps_filled as f64);
    service_metrics.custom_metrics.insert(
      "data_points_stored".to_string(),
      ohlcv_metrics.data_points_stored as f64,
    );
    service_metrics.custom_metrics.insert(
      "database_size_mb".to_string(),
      ohlcv_metrics.database_size_mb,
    );

    service_metrics
  }
}
