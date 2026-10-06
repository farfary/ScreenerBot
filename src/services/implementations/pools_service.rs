// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pools service — initializes the pool components of every enabled chain and runs the pools
//! maintenance task. The pricing stages run in the pool pricing service.

use crate::i18n::{ids, UiText};
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct PoolsService;

#[async_trait]
impl Service for PoolsService {
    fn name(&self) -> &'static str {
        "pools"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn dependencies(&self) -> Vec<&'static str> {
        // Initialization loads price history and warms the cache for the mints of
        // open positions, which the positions service loads.
        vec!["positions"]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        logger::info(LogTag::PoolService, "Initializing pool components...");

        // Open each enabled chain's database and history, and initialize its
        // pricing driver through the chain runtime.
        crate::pools::initialize_pool_components()
            .await
            .map_err(|e| {
                crate::Error::Service(crate::errors::ServiceError::Initialize {
                    service: "pools".to_owned(),
                    message: format!("Failed to initialize pool components: {e}"),
                })
            })?;

        logger::info(LogTag::PoolService, "Pool components initialized");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        logger::info(LogTag::PoolService, "Starting pool maintenance task...");

        // Periodic cache and database upkeep for every chain runs in one task.
        // The pricing stage loops are started by the pool pricing service.
        let handle = crate::pools::start_maintenance_task(shutdown, monitor);

        logger::info(
            LogTag::PoolService,
            "Pool maintenance task started (1 instrumented handle)",
        );

        // Return the handle so ServiceManager can wait for graceful shutdown
        Ok(vec![handle])
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::info(LogTag::PoolService, "Stopping pool service...");

        // Stop the pool service, releasing every chain's pricing components.
        crate::pools::stop_pool_service().await;

        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::pools::is_pool_service_running() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy(UiText::new(ids::SERVICES_HEALTH_POOLS_NOT_RUNNING))
        }
    }
}
