// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pool service supervisor - chain-neutral lifecycle for the pool runtime
//!
//! Owns the running flag, event recording and, for every enabled chain, the
//! database and history initialization plus the chain's pricing driver
//! (`ChainRuntime::pricing_driver`), which brings up the chain-owned
//! discovery/analysis/fetch/calculation components. The stage loops are
//! started by the pool pricing service; periodic upkeep runs in the maintenance
//! task (`super::maintenance`).

use super::driver::{PricingDriver, PricingStage, PricingStageMetrics};
use super::types::max_watched_tokens;
use super::{cache, db, Error};
use crate::chains::{adapter_for, enabled_chains, runtime_for, ChainId};
use crate::events::{record_safe, Event, EventCategory};
use crate::logger::{self, LogTag};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

// Timing constants
const FETCH_INTERVAL_MS: u64 = 500;

// Global service state
static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);

/// The pricing driver of an enabled `chain`; a missing runtime is an
/// invariant error, because every enabled chain installs one at boot.
pub(super) fn pricing_driver(chain: ChainId) -> Result<Arc<dyn PricingDriver>, Error> {
    runtime_for(chain)
        .map(|runtime| runtime.pricing_driver())
        .ok_or(Error::RuntimeUnavailable { chain })
}

/// Initialize pool components only (no background tasks)
///
/// For each enabled chain, in order: opens its pools database, loads its
/// open positions' price history, initializes its pricing driver, and warms
/// its token pool cache for those positions.
///
/// Returns an error if already initialized or if initialization fails.
pub async fn initialize_pool_components() -> Result<(), Error> {
    let fetch_interval_ms = FETCH_INTERVAL_MS;
    let max_tokens = max_watched_tokens();
    let refresh_interval_seconds = (fetch_interval_ms as f64) / 1000.0;

    // Record service start attempt
    record_safe(Event::info(
        EventCategory::System,
        Some("pool_service_start_attempt".to_owned()),
        None,
        None,
        serde_json::json!({
          "max_watched_tokens": max_tokens,
          "refresh_interval_seconds": refresh_interval_seconds,
          "fetch_interval_ms": fetch_interval_ms
        }),
    ))
    .await;

    // Check if already running
    if SERVICE_RUNNING.swap(true, Ordering::SeqCst) {
        logger::warning(LogTag::PoolService, "Pool service is already running");

        record_safe(Event::warn(
            EventCategory::System,
            Some("pool_service_already_running".to_owned()),
            None,
            None,
            serde_json::json!({
              "error": "Service already running",
              "action": "start_rejected"
            }),
        ))
        .await;

        return Err(Error::AlreadyRunning);
    }

    logger::info(LogTag::PoolService, "Starting pool service...");

    for &chain in enabled_chains() {
        if let Err(e) = initialize_chain(chain).await {
            SERVICE_RUNNING.store(false, Ordering::Relaxed);
            return Err(e);
        }
    }

    logger::info(
        LogTag::PoolService,
        "Pool components initialized successfully",
    );

    record_safe(Event::info(
        EventCategory::System,
        Some("pool_components_initialized".to_owned()),
        None,
        None,
        serde_json::json!({
          "status": "initialized",
          "components_ready": true
        }),
    ))
    .await;

    Ok(())
}

/// Bring up one chain: open its database, load its open positions' price
/// history and initialize its pricing driver.
async fn initialize_chain(chain: ChainId) -> Result<(), Error> {
    if let Err(e) = db::initialize_database(chain).await {
        logger::error(
            LogTag::PoolService,
            &format!("Failed to initialize database on {chain}: {e}"),
        );

        record_safe(Event::error(
            EventCategory::System,
            Some("pool_service_db_init_failed".to_owned()),
            None,
            None,
            serde_json::json!({
              "error": e.to_string(),
              "component": "database",
              "action": "initialize"
            }),
        ))
        .await;

        return Err(e);
    }

    let open_mints = open_mints_on(chain).await;
    cache::load_history(chain, &open_mints).await;

    // Initialize the chain's pricing pipeline components
    record_safe(Event::info(
        EventCategory::System,
        Some("pool_components_init_start".to_owned()),
        None,
        None,
        serde_json::json!({
          "action": "component_initialization"
        }),
    ))
    .await;

    let initialized = match pricing_driver(chain) {
        Ok(driver) => driver.initialize().await,
        Err(e) => Err(e),
    };
    match initialized {
        Ok(init) => {
            record_safe(Event::info(
                EventCategory::System,
                Some("pool_components_initialized".to_owned()),
                None,
                None,
                serde_json::json!({
                  "chain": chain,
                  "components": init.components,
                  "rpc_urls_count": init.rpc_urls_count,
                  "status": "ready"
                }),
            ))
            .await;
            logger::info(
                LogTag::PoolService,
                &format!("Service components initialized successfully on {chain}"),
            );
        }
        Err(e) => {
            record_safe(Event::error(
                EventCategory::System,
                Some("pool_service_component_init_failed".to_owned()),
                None,
                None,
                serde_json::json!({
                  "chain": chain,
                  "error": e.to_string(),
                  "component": "service_components",
                  "action": "initialize"
                }),
            ))
            .await;

            return Err(e);
        }
    }

    Ok(())
}

/// Stop the pool service
///
/// Clears the running flag and releases every enabled chain's pricing
/// components. The maintenance task and the stage loops stop on the service
/// manager's shutdown signal.
pub async fn stop_pool_service() {
    record_safe(Event::info(
        EventCategory::System,
        Some("pool_service_stop_attempt".to_owned()),
        None,
        None,
        serde_json::json!({
          "action": "stop_requested"
        }),
    ))
    .await;

    if !SERVICE_RUNNING.load(Ordering::Relaxed) {
        logger::warning(LogTag::PoolService, "Pool service is not running");

        record_safe(Event::warn(
            EventCategory::System,
            Some("pool_service_not_running".to_owned()),
            None,
            None,
            serde_json::json!({
              "warning": "Service not running",
              "action": "stop_skipped"
            }),
        ))
        .await;

        return;
    }

    SERVICE_RUNNING.store(false, Ordering::Relaxed);
    for &chain in enabled_chains() {
        match pricing_driver(chain) {
            Ok(driver) => driver.clear(),
            Err(e) => logger::warning(
                LogTag::PoolService,
                &format!("Pricing components not cleared on {chain}: {e}"),
            ),
        }
    }

    logger::info(LogTag::PoolService, "Pool service stopped successfully");

    record_safe(Event::info(
        EventCategory::System,
        Some("pool_service_stopped".to_owned()),
        None,
        None,
        serde_json::json!({
          "status": "stopped",
          "clean_shutdown": true
        }),
    ))
    .await;
}

/// Start `stage`'s loop on every enabled chain under the service manager's
/// `shutdown` and `monitor`, returning every loop handle. Never awaits a loop.
/// On a failure, the loops already started are aborted and the error is
/// returned.
pub fn start_pricing_stage(
    stage: PricingStage,
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> Result<Vec<JoinHandle<()>>, Error> {
    let mut handles = Vec::new();
    for &chain in enabled_chains() {
        let started = pricing_driver(chain)
            .and_then(|driver| driver.start_stage(stage, shutdown.clone(), monitor.clone()));
        match started {
            Ok(Some(handle)) => handles.push(handle),
            Ok(None) => {}
            Err(e) => {
                for handle in &handles {
                    handle.abort();
                }
                return Err(e);
            }
        }
    }
    Ok(handles)
}

/// Whether `stage`'s component is initialized on every enabled chain.
pub fn pricing_stage_ready(stage: PricingStage) -> bool {
    enabled_chains().iter().all(|&chain| {
        pricing_driver(chain)
            .map(|driver| driver.stage_ready(stage))
            .unwrap_or(false)
    })
}

/// `stage`'s counters summed by key over every enabled chain; `None` while
/// no chain has the stage initialized.
pub fn pricing_stage_metrics(stage: PricingStage) -> Option<PricingStageMetrics> {
    let mut sum: Option<PricingStageMetrics> = None;
    for &chain in enabled_chains() {
        let Some(metrics) = pricing_driver(chain)
            .ok()
            .and_then(|driver| driver.stage_metrics(stage))
        else {
            continue;
        };
        let sum = sum.get_or_insert_with(PricingStageMetrics::default);
        sum.operations += metrics.operations;
        sum.errors += metrics.errors;
        for (key, value) in metrics.counters {
            match sum.counters.iter_mut().find(|(k, _)| *k == key) {
                Some((_, total)) => *total += value,
                None => sum.counters.push((key, value)),
            }
        }
    }
    sum
}

/// Check if the pool service is currently running
pub fn is_pool_service_running() -> bool {
    SERVICE_RUNNING.load(Ordering::SeqCst)
}

/// The held mints of the live and paper books that `chain` accepts as addresses.
/// Positions carry no chain yet, so the chain's own address rule selects them.
async fn open_mints_on(chain: ChainId) -> Vec<String> {
    let adapter = adapter_for(chain);
    crate::positions::books::held_mints()
        .await
        .into_iter()
        .filter(|mint| adapter.validate_address(mint).is_ok())
        .collect()
}

/// Start the task that warms every chain's token pool cache for the held tokens, so
/// they have fresh pool data as soon as pricing starts instead of at the first discovery
/// tick. The cache refresh reads the token database, which the tokens service publishes
/// after this service starts, so the task waits for the tokens system to be ready rather
/// than relying on the start order; it ends on shutdown if that never happens.
pub fn start_open_position_warm_up(
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> JoinHandle<()> {
    tokio::spawn(monitor.instrument(async move {
        tokio::select! {
            _ = crate::global::wait_for_tokens_system_ready() => {}
            _ = shutdown.notified() => return,
        }
        for &chain in enabled_chains() {
            warm_cache_for_open_positions(chain).await;
        }
    }))
}

/// Queue a pool refresh of `chain`'s held tokens on the token pool cache workers.
async fn warm_cache_for_open_positions(chain: ChainId) {
    let open_mints = open_mints_on(chain).await;

    if open_mints.is_empty() {
        logger::debug(
            LogTag::PoolService,
            &format!("No open positions to warm cache for on {chain}"),
        );
        return;
    }

    crate::tokens::prefetch_token_pools(chain, &open_mints).await;

    logger::info(
        LogTag::PoolService,
        &format!(
            "Queued a pool refresh for {} held tokens on {}",
            open_mints.len(),
            chain
        ),
    );
}

#[cfg(test)]
mod warm_up_tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn the_held_token_warm_up_waits_for_the_tokens_system_and_then_runs() {
        let shutdown = Arc::new(Notify::new());
        let warm_up = start_open_position_warm_up(shutdown, tokio_metrics::TaskMonitor::new());
        tokio::task::yield_now().await;
        assert!(
            !warm_up.is_finished(),
            "the warm-up must not read the token database before it is published"
        );

        crate::global::mark_tokens_system_ready();
        tokio::time::timeout(Duration::from_secs(5), warm_up)
            .await
            .expect("a ready tokens system must start the warm-up")
            .expect("warm-up task");
    }

    #[tokio::test]
    async fn the_held_token_warm_up_ends_on_shutdown_while_it_waits() {
        let shutdown = Arc::new(Notify::new());
        let warm_up =
            start_open_position_warm_up(shutdown.clone(), tokio_metrics::TaskMonitor::new());
        tokio::task::yield_now().await;

        shutdown.notify_waiters();
        tokio::time::timeout(Duration::from_secs(5), warm_up)
            .await
            .expect("shutdown must end a waiting warm-up")
            .expect("warm-up task");
    }
}
