//! Paper-only copy decision consumer.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::services::{Service, ServiceHealth, ServiceMetrics};

pub struct CopyTradingService;

#[async_trait]
impl Service for CopyTradingService {
    fn name(&self) -> &'static str {
        "copy_trading"
    }

    fn priority(&self) -> i32 {
        // Paper decisions call the same positions/admission/filter/pool reads as a
        // future live entry, so start only after the existing trader stack.
        151
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["wallet_watch", "filtering", "positions", "wallet", "pools"]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        _monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let database = crate::trader::copy::CopyDatabase::new().map_err(|message| {
            crate::Error::Service(crate::errors::ServiceError::Start {
                service: "copy_trading".to_owned(),
                message,
            })
        })?;
        Ok(vec![tokio::spawn(crate::trader::copy::run(
            shutdown, database,
        ))])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}
