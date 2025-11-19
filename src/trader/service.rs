//! Trader service implementation

use crate::events::{record_trader_event, Severity};
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth};
use crate::trader::config;
use crate::trader::monitors;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use tokio::task::JoinHandle;

pub struct TraderService {
    shutdown_tx: Arc<RwLock<Option<tokio::sync::watch::Sender<bool>>>>,
}

impl TraderService {
    pub fn new() -> Self {
        Self {
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }
}

impl Default for TraderService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for TraderService {
    fn name(&self) -> &'static str {
        "trader"
    }

    fn priority(&self) -> i32 {
        150
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![
            "positions",
            "pool_discovery",
            "pool_fetcher",
            "pool_calculator",
            "token_discovery",
            "token_monitoring",
            "filtering",
        ]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> Result<(), String> {
        super::init_trader_system().await?;
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        logger::info(LogTag::Trader, "Starting Trader Service...");

        record_trader_event(
            "service_start",
            Severity::Info,
            None,
            None,
            json!({
                "action": "startup",
                "message": "Trader service initialization beginning",
            }),
        )
        .await;

        let trader_enabled = config::is_trader_enabled();
        if !trader_enabled {
            logger::info(
                LogTag::Trader,
                "Trader service started but trading is disabled in config",
            );

            // Record disabled state event
            record_trader_event(
                "trading_disabled",
                Severity::Warn,
                None,
                None,
                json!({
                    "enabled": false,
                    "message": "Trading is disabled in configuration",
                }),
            )
            .await;
        } else {
            // Record enabled state event
            record_trader_event(
                "trading_enabled",
                Severity::Info,
                None,
                None,
                json!({
                    "enabled": true,
                    "message": "Trading is enabled and active",
                }),
            )
            .await;
        }

        let (watch_tx, watch_rx) = tokio::sync::watch::channel(false);
        *self.shutdown_tx.write().await = Some(watch_tx.clone());

        let bridge_shutdown = shutdown.clone();
        tokio::spawn(async move {
            bridge_shutdown.notified().await;
            let _ = watch_tx.send(true);
        });

        let handle = tokio::spawn(monitor.instrument(async move {
            if let Err(e) = monitors::start_automated_trading(watch_rx).await {
                logger::error(LogTag::Trader, &format!("Auto trading error: {}", e));

                record_trader_event(
                    "auto_trading_error",
                    Severity::Error,
                    None,
                    None,
                    json!({
                        "error": e.to_string(),
                        "message": "Auto trading encountered an error",
                    }),
                )
                .await;
            }
        }));

        logger::info(LogTag::Trader, "Trader Service started successfully");

        record_trader_event(
            "service_started",
            Severity::Info,
            None,
            None,
            json!({
                "status": "running",
                "message": "Trader service fully initialized and running",
            }),
        )
        .await;

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        logger::info(LogTag::Trader, "Stopping Trader Service...");

        record_trader_event(
            "service_stop",
            Severity::Info,
            None,
            None,
            json!({
                "action": "shutdown",
                "message": "Trader service shutdown initiated",
            }),
        )
        .await;

        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(true);
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        logger::info(LogTag::Trader, "Trader Service stopped");

        record_trader_event(
            "service_stopped",
            Severity::Info,
            None,
            None,
            json!({
                "status": "stopped",
                "message": "Trader service gracefully stopped",
            }),
        )
        .await;

        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::global::is_initialization_complete() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
        }
    }
}
