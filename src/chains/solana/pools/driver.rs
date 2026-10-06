// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The Solana pricing driver: the neutral `PricingDriver` contract over this
//! module's discovery, analysis, fetch and calculation components
//! (`super::service`). Reached only through `SolanaRuntime::pricing_driver`.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Notify;
use tokio::task::JoinHandle;

use super::service;
use crate::chains::ChainId;
use crate::pools::types::PoolDescriptor;
use crate::pools::{
    sort_protocol_counts, AccountFetchStatus, Error, PoolDirectoryStatus, PricingDriver,
    PricingInit, PricingStage, PricingStageMetrics, PricingStatus, Result,
};

/// The components `initialize` brings up, in pipeline order.
const COMPONENTS: &[&str] = &[
    "pool_discovery",
    "pool_analyzer",
    "account_fetcher",
    "price_calculator",
];

/// The Solana pricing driver. Stateless: the components live in
/// `super::service`.
pub struct SolanaPricingDriver;

const NOT_INITIALIZED: &str = "component not initialized";
const ALREADY_STARTED: &str = "loop already started since the last initialization";

#[async_trait::async_trait]
impl PricingDriver for SolanaPricingDriver {
    async fn initialize(&self) -> Result<PricingInit> {
        let rpc_urls_count =
            service::initialize_components()
                .await
                .map_err(|e| Error::ComponentInit {
                    detail: e.to_string(),
                })?;
        Ok(PricingInit {
            components: COMPONENTS,
            rpc_urls_count,
        })
    }

    fn start_stage(
        &self,
        stage: PricingStage,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Option<JoinHandle<()>>> {
        let started = match stage {
            PricingStage::Discovery => match service::get_pool_discovery() {
                None => Err(NOT_INITIALIZED),
                Some(discovery) if !discovery.claim_loop() => Err(ALREADY_STARTED),
                Some(discovery) => Ok(tokio::spawn(
                    monitor.instrument(discovery.run_discovery_loop(shutdown)),
                )),
            },
            PricingStage::Analysis => match service::get_pool_analyzer() {
                None => Err(NOT_INITIALIZED),
                Some(analyzer) => match analyzer.take_receiver() {
                    None => Err(ALREADY_STARTED),
                    Some(rx) => Ok(tokio::spawn(
                        monitor.instrument(analyzer.run_analyzer_loop(rx, shutdown)),
                    )),
                },
            },
            PricingStage::Fetch => match service::get_account_fetcher() {
                None => Err(NOT_INITIALIZED),
                Some(fetcher) => match fetcher.take_receiver() {
                    None => Err(ALREADY_STARTED),
                    Some(rx) => Ok(tokio::spawn(
                        monitor.instrument(fetcher.run_fetcher_loop(rx, shutdown)),
                    )),
                },
            },
            PricingStage::Calculation => match service::get_price_calculator() {
                None => Err(NOT_INITIALIZED),
                Some(calculator) => match calculator.take_receiver() {
                    None => Err(ALREADY_STARTED),
                    Some(rx) => Ok(tokio::spawn(
                        monitor.instrument(calculator.run_calculator_loop(rx, shutdown)),
                    )),
                },
            },
        };
        started.map(Some).map_err(|detail| Error::StageUnavailable {
            chain: ChainId::Solana,
            stage,
            detail: detail.to_owned(),
        })
    }

    fn stage_ready(&self, stage: PricingStage) -> bool {
        match stage {
            PricingStage::Discovery => service::get_pool_discovery().is_some(),
            PricingStage::Analysis => service::get_pool_analyzer().is_some(),
            PricingStage::Fetch => service::get_account_fetcher().is_some(),
            PricingStage::Calculation => service::get_price_calculator().is_some(),
        }
    }

    fn stage_metrics(&self, stage: PricingStage) -> Option<PricingStageMetrics> {
        match stage {
            PricingStage::Discovery => service::get_pool_discovery().map(|discovery| {
                let (operations, errors, pools_discovered) = discovery.get_metrics();
                PricingStageMetrics {
                    operations,
                    errors,
                    counters: vec![("pools_discovered", pools_discovered)],
                }
            }),
            PricingStage::Analysis => service::get_pool_analyzer().map(|analyzer| {
                let (operations, errors, pools_analyzed) = analyzer.get_metrics();
                PricingStageMetrics {
                    operations,
                    errors,
                    counters: vec![("pools_analyzed", pools_analyzed)],
                }
            }),
            PricingStage::Fetch => service::get_account_fetcher().map(|fetcher| {
                let (operations, errors, accounts_fetched, rpc_batches) = fetcher.get_metrics();
                PricingStageMetrics {
                    operations,
                    errors,
                    counters: vec![
                        ("accounts_fetched", accounts_fetched),
                        ("rpc_batches", rpc_batches),
                    ],
                }
            }),
            PricingStage::Calculation => service::get_price_calculator().map(|calculator| {
                let (operations, errors, prices_calculated) = calculator.get_metrics();
                PricingStageMetrics {
                    operations,
                    errors,
                    counters: vec![("prices_calculated", prices_calculated)],
                }
            }),
        }
    }

    fn clear(&self) {
        service::clear_components();
    }

    fn token_pools(&self, mint: &str) -> Vec<PoolDescriptor> {
        service::get_token_pools(mint)
    }

    fn pool_protocol(&self, mint: &str, pool: &str) -> Option<&'static str> {
        service::get_pool_program(mint, pool)
    }

    fn status(&self) -> PricingStatus {
        let directory = service::get_pool_analyzer().and_then(|analyzer| {
            let directory = analyzer.get_pool_directory();
            let guard = directory.read().ok()?;

            let mut counts: HashMap<String, usize> = HashMap::new();
            for descriptor in guard.values() {
                *counts
                    .entry(descriptor.program_kind.as_str().to_string())
                    .or_default() += 1;
            }

            Some(PoolDirectoryStatus {
                total_pools: guard.len(),
                pools_by_protocol: sort_protocol_counts(counts.into_iter().collect()),
            })
        });

        let fetch = service::get_account_fetcher().map(|fetcher| {
            let stats = fetcher.get_fetch_stats();
            AccountFetchStatus {
                total_bundles: stats.total_bundles,
                bundles_with_data: stats.bundles_with_data,
                total_accounts_tracked: stats.total_accounts_tracked,
            }
        });

        PricingStatus { directory, fetch }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Signal shutdown until `handle` finishes, as the service manager does,
    /// and fail when it takes longer than `limit`.
    async fn stop_within(shutdown: &Notify, handle: JoinHandle<()>, limit: Duration) {
        let mut handle = handle;
        let finished = tokio::time::timeout(limit, async {
            loop {
                shutdown.notify_waiters();
                tokio::select! {
                    joined = &mut handle => return joined,
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {}
                }
            }
        })
        .await;
        assert!(
            matches!(finished, Ok(Ok(()))),
            "stage loop did not stop within {limit:?}"
        );
    }

    fn assert_already_started(result: Result<Option<JoinHandle<()>>>, stage: PricingStage) {
        match result {
            Err(Error::StageUnavailable {
                chain: ChainId::Solana,
                stage: failed,
                ..
            }) => assert_eq!(failed, stage),
            Err(other) => panic!("expected StageUnavailable for {stage}, got {other}"),
            Ok(_) => panic!("a second start of {stage} must fail"),
        }
    }

    /// The component statics are process-wide, so every lifecycle case runs
    /// in this one test.
    #[tokio::test]
    async fn stage_lifecycle_is_single_start_non_blocking_and_stops_on_shutdown() {
        let driver = SolanaPricingDriver;
        let monitor = tokio_metrics::TaskMonitor::new();

        service::clear_components();
        for stage in [PricingStage::Fetch, PricingStage::Calculation] {
            assert!(!driver.stage_ready(stage));
            match driver.start_stage(stage, Arc::new(Notify::new()), monitor.clone()) {
                Err(Error::StageUnavailable { stage: failed, .. }) => assert_eq!(failed, stage),
                _ => panic!("starting {stage} before initialization must fail"),
            }
        }

        service::install_components();
        for stage in [PricingStage::Fetch, PricingStage::Calculation] {
            assert!(driver.stage_ready(stage));
            let shutdown = Arc::new(Notify::new());

            // Start returns the loop's own handle without awaiting the loop.
            let handle = driver
                .start_stage(stage, shutdown.clone(), monitor.clone())
                .expect("first start succeeds")
                .expect("Solana runs every stage");
            tokio::time::sleep(Duration::from_millis(100)).await;
            assert!(!handle.is_finished(), "{stage} loop must keep running");

            // A second start without a new initialization fails with a typed error.
            assert_already_started(
                driver.start_stage(stage, shutdown.clone(), monitor.clone()),
                stage,
            );

            stop_within(&shutdown, handle, Duration::from_secs(1)).await;
        }

        // A new initialization installs fresh components that can start again.
        service::install_components();
        let shutdown = Arc::new(Notify::new());
        let handle = driver
            .start_stage(PricingStage::Fetch, shutdown.clone(), monitor.clone())
            .expect("start after reinitialization succeeds")
            .expect("Solana runs every stage");
        stop_within(&shutdown, handle, Duration::from_secs(1)).await;

        service::clear_components();
    }

    #[test]
    fn a_second_discovery_loop_claim_is_refused() {
        let discovery = super::super::discovery::PoolDiscovery::new();
        assert!(discovery.claim_loop());
        assert!(!discovery.claim_loop());
    }
}
