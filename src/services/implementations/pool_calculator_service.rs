// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pool calculator service — computes token prices from pool reserves.

use crate::i18n::{ids, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::pools::PricingStage;
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// The pricing stage this service runs on every enabled chain.
const STAGE: PricingStage = PricingStage::Calculation;

pub struct PoolCalculatorService;

#[async_trait]
impl Service for PoolCalculatorService {
    fn name(&self) -> &'static str {
        "pool_calculator"
    }

    fn priority(&self) -> i32 {
        102
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["pools", "pool_fetcher", "filtering"]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        logger::info(
            LogTag::PoolService,
            &"Initializing pool calculator service...".to_owned(),
        );
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        logger::info(
            LogTag::PoolService,
            &"Starting pool calculator service...".to_owned(),
        );

        // One loop per enabled chain, each spawned under this service's
        // shutdown and monitor; the handles are the loops themselves.
        let handles = crate::pools::start_pricing_stage(STAGE, shutdown, monitor)?;

        logger::info(
            LogTag::PoolService,
            &"Pool calculator service started (instrumented)".to_owned(),
        );

        Ok(handles)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::info(
            LogTag::PoolService,
            &"Pool calculator service stopping (via shutdown signal)".to_owned(),
        );
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::pools::pricing_stage_ready(STAGE) {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy(
                UiText::new(ids::SERVICES_HEALTH_COMPONENT_UNAVAILABLE)
                    .arg("component", UiArg::Text("PriceCalculator".to_owned())),
            )
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        let mut metrics = ServiceMetrics::default();

        // Counters summed over every enabled chain; ratios are derived from
        // the sums.
        if let Some(stage) = crate::pools::pricing_stage_metrics(STAGE) {
            let counter = |key: &str| {
                stage
                    .counters
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map_or(0, |(_, value)| *value)
            };
            let operations = stage.operations;
            metrics.operations_total = operations;
            metrics.errors_total = stage.errors;
            let prices_calculated = counter("prices_calculated");
            metrics
                .custom_metrics
                .insert("prices_calculated".to_owned(), prices_calculated as f64);
            if operations > 0 {
                metrics.custom_metrics.insert(
                    "success_rate".to_owned(),
                    (prices_calculated as f64 / operations as f64) * 100.0,
                );
            }
        }

        metrics
    }
}
