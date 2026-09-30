//! Trader service — orchestrates automated trading strategies and position management.

use crate::events::{record_trader_event, Severity};
use crate::i18n::{ids, UiText};
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth};
use crate::trader::config;
use crate::trader::monitors;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
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
            "pools",
            "tokens",
            "filtering",
        ]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        crate::trader::init_trader_system().await.map_err(|e| {
            crate::Error::Service(crate::errors::ServiceError::Initialize {
                service: "trader".to_owned(),
                message: e.to_string(),
            })
        })?;
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        logger::info(LogTag::Trader, "Starting Trader Service...");

        record_trader_event(
            "service_start",
            Severity::Info,
            None,
            None,
            crate::events::with_text(
                json!({
                    "action": "startup",
                }),
                &UiText::new(ids::EVENTS_TRADER_SERVICE_INITIALIZING),
            ),
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
                crate::events::with_text(
                    json!({
                        "enabled": false,
                    }),
                    &UiText::new(ids::EVENTS_TRADER_TRADING_DISABLED),
                ),
            )
            .await;
        } else {
            // Record enabled state event
            record_trader_event(
                "trading_enabled",
                Severity::Info,
                None,
                None,
                crate::events::with_text(
                    json!({
                        "enabled": true,
                    }),
                    &UiText::new(ids::EVENTS_TRADER_TRADING_ENABLED),
                ),
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
                logger::error(LogTag::Trader, &format!("Auto trading error: {e}"));

                record_trader_event(
                    "auto_trading_error",
                    Severity::Error,
                    None,
                    None,
                    crate::events::with_text(
                        json!({
                            "error": e.to_string(),
                        }),
                        &UiText::new(ids::EVENTS_TRADER_AUTO_TRADING_ERROR),
                    ),
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
            crate::events::with_text(
                json!({
                    "status": "running",
                }),
                &UiText::new(ids::EVENTS_TRADER_SERVICE_STARTED),
            ),
        )
        .await;

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::info(LogTag::Trader, "Stopping Trader Service...");

        record_trader_event(
            "service_stop",
            Severity::Info,
            None,
            None,
            crate::events::with_text(
                json!({
                    "action": "shutdown",
                }),
                &UiText::new(ids::EVENTS_TRADER_SERVICE_STOPPING),
            ),
        )
        .await;

        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(true);
        }

        tokio::time::sleep(Duration::from_secs(2)).await;

        logger::info(LogTag::Trader, "Trader Service stopped");

        record_trader_event(
            "service_stopped",
            Severity::Info,
            None,
            None,
            crate::events::with_text(
                json!({
                    "status": "stopped",
                }),
                &UiText::new(ids::EVENTS_TRADER_SERVICE_STOPPED),
            ),
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
