// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pool pricing service — runs the pool pricing pipeline (discovery, account
//! fetch, price calculation, analysis) on every enabled chain.

use crate::i18n::{ids, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::pools::{PricingStage, PricingStageMetrics};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// The pricing stages in start order, each with the component label its
/// health message names.
const STAGES: [(PricingStage, &str); 4] = [
    (PricingStage::Discovery, "PoolDiscovery"),
    (PricingStage::Fetch, "AccountFetcher"),
    (PricingStage::Calculation, "PriceCalculator"),
    (PricingStage::Analysis, "PoolAnalyzer"),
];

pub struct PoolPricingService;

#[async_trait]
impl Service for PoolPricingService {
    fn name(&self) -> &'static str {
        "pool_pricing"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["transactions", "pools", "filtering"]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        logger::debug(LogTag::PoolService, "Initializing pool pricing service...");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        logger::debug(LogTag::PoolService, "Starting pool pricing service...");

        // One loop per stage and enabled chain, each spawned under this
        // service's shutdown and monitor; the handles are the loops
        // themselves. A stage that fails to start aborts the loops of the
        // stages already started.
        let mut handles = Vec::new();
        for (stage, _) in STAGES {
            match crate::pools::start_pricing_stage(stage, shutdown.clone(), monitor.clone()) {
                Ok(started) => handles.extend(started),
                Err(e) => {
                    for handle in &handles {
                        handle.abort();
                    }
                    return Err(e.into());
                }
            }
        }

        logger::info(
            LogTag::PoolService,
            &format!(
                "Pool pricing service started ({} instrumented handles)",
                handles.len()
            ),
        );

        Ok(handles)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::debug(
            LogTag::PoolService,
            "Pool pricing service stopping (via shutdown signal)",
        );
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        // Every stage is either ready or unavailable, so the worst stage is
        // the first one that is not ready.
        match STAGES
            .iter()
            .find(|(stage, _)| !crate::pools::pricing_stage_ready(*stage))
        {
            None => ServiceHealth::Healthy,
            Some((_, component)) => ServiceHealth::Unhealthy(
                UiText::new(ids::SERVICES_HEALTH_COMPONENT_UNAVAILABLE)
                    .arg("component", UiArg::Text((*component).to_owned())),
            ),
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        let mut metrics = ServiceMetrics::default();

        // Counters summed over every enabled chain; ratios are derived from
        // the sums. Keys every stage reports carry the stage prefix.
        for (stage, _) in STAGES {
            let Some(stage_metrics) = crate::pools::pricing_stage_metrics(stage) else {
                continue;
            };
            metrics.operations_total += stage_metrics.operations;
            metrics.errors_total += stage_metrics.errors;
            let prefix = stage.as_str();
            metrics.custom_metrics.insert(
                format!("{prefix}_operations"),
                stage_metrics.operations as f64,
            );
            metrics
                .custom_metrics
                .insert(format!("{prefix}_errors"), stage_metrics.errors as f64);
            insert_stage_metrics(&mut metrics, stage, &stage_metrics);
        }

        metrics
    }
}

/// Insert `stage`'s own counters and the ratios derived from them.
fn insert_stage_metrics(
    metrics: &mut ServiceMetrics,
    stage: PricingStage,
    stage_metrics: &PricingStageMetrics,
) {
    let counter = |key: &str| {
        stage_metrics
            .counters
            .iter()
            .find(|(k, _)| *k == key)
            .map_or(0, |(_, value)| *value)
    };
    let operations = stage_metrics.operations;
    let custom = &mut metrics.custom_metrics;
    match stage {
        PricingStage::Discovery => {
            let pools_discovered = counter("pools_discovered");
            custom.insert("pools_discovered".to_owned(), pools_discovered as f64);
            if operations > 0 {
                custom.insert(
                    "avg_pools_per_cycle".to_owned(),
                    pools_discovered as f64 / operations as f64,
                );
            }
        }
        PricingStage::Fetch => {
            let accounts_fetched = counter("accounts_fetched");
            let rpc_batches = counter("rpc_batches");
            custom.insert("accounts_fetched".to_owned(), accounts_fetched as f64);
            custom.insert("rpc_batches".to_owned(), rpc_batches as f64);
            if operations > 0 {
                custom.insert(
                    "avg_accounts_per_cycle".to_owned(),
                    accounts_fetched as f64 / operations as f64,
                );
            }
            if rpc_batches > 0 {
                custom.insert(
                    "avg_accounts_per_batch".to_owned(),
                    accounts_fetched as f64 / rpc_batches as f64,
                );
            }
        }
        PricingStage::Calculation => {
            let prices_calculated = counter("prices_calculated");
            custom.insert("prices_calculated".to_owned(), prices_calculated as f64);
            if operations > 0 {
                custom.insert(
                    "calculation_success_rate".to_owned(),
                    (prices_calculated as f64 / operations as f64) * 100.0,
                );
            }
        }
        PricingStage::Analysis => {
            let pools_analyzed = counter("pools_analyzed");
            custom.insert("pools_analyzed".to_owned(), pools_analyzed as f64);
            if operations > 0 {
                custom.insert(
                    "analysis_success_rate".to_owned(),
                    (pools_analyzed as f64 / operations as f64) * 100.0,
                );
            }
        }
    }
}
