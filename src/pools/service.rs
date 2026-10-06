// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pool service supervisor - chain-neutral lifecycle for the pool runtime
//!
//! Owns the running flag, event recording and the per-chain database and
//! history initialization every enabled chain's pool runtime needs. The
//! concrete runtime components (discovery/analyzer/fetcher/calculator) are
//! chain-owned — for Solana that is `crate::chains::solana::pools::service`.
//! The composition root (`src/services/implementations/pools_service.rs`)
//! selects that implementation and passes its
//! `initialize_components`/`clear_components` functions in here, so this
//! module never imports `crate::chains::solana`. Periodic upkeep runs in the
//! maintenance task (`super::maintenance`).

use super::types::max_watched_tokens;
use super::{cache, db, Error};
use crate::chains::{adapter_for, enabled_chains, ChainId};
use crate::config::with_config;
use crate::events::{record_safe, Event, EventCategory};
use crate::logger::{self, LogTag};

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};

// Timing constants
const FETCH_INTERVAL_MS: u64 = 500;

// Global service state
static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);

/// Initialize pool components only (no background tasks)
///
/// Opens every enabled chain's pools database and loads its open positions'
/// price history, then runs `init_chain_components` once to bring up the
/// concrete runtime the composition root selected, and finally warms each
/// chain's token pool cache for its open positions.
///
/// Returns an error if already initialized or if initialization fails.
pub async fn initialize_pool_components<F, Fut, E>(init_chain_components: F) -> Result<(), Error>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<u32, E>>,
    E: std::fmt::Display,
{
    let (single_pool_mode, dexscreener_enabled, fetch_interval_ms) = with_config(|cfg| {
        (
            cfg.pools.enable_single_pool_mode,
            cfg.pools.enable_dexscreener_discovery,
            FETCH_INTERVAL_MS,
        )
    });
    let max_tokens = max_watched_tokens();
    let refresh_interval_seconds = (fetch_interval_ms as f64) / 1000.0;

    // Record service start attempt
    record_safe(Event::info(
        EventCategory::System,
        Some("pool_service_start_attempt".to_owned()),
        None,
        None,
        serde_json::json!({
          "single_pool_mode": single_pool_mode,
          "max_watched_tokens": max_tokens,
          "refresh_interval_seconds": refresh_interval_seconds,
          "fetch_interval_ms": fetch_interval_ms,
          "dexscreener_enabled": dexscreener_enabled
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

    // Open each enabled chain's database, then load the price history of
    // that chain's open positions into its cache.
    for &chain in enabled_chains() {
        if let Err(e) = db::initialize_database(chain).await {
            logger::error(
                LogTag::PoolService,
                &format!("Failed to initialize database on {chain}: {e}"),
            );
            SERVICE_RUNNING.store(false, Ordering::Relaxed);

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
    }

    // Initialize the chain-specific runtime components
    record_safe(Event::info(
        EventCategory::System,
        Some("pool_components_init_start".to_owned()),
        None,
        None,
        serde_json::json!({
          "dexscreener_enabled": dexscreener_enabled,
          "action": "component_initialization"
        }),
    ))
    .await;

    match init_chain_components().await {
        Ok(rpc_urls_count) => {
            record_safe(Event::info(
                EventCategory::System,
                Some("pool_components_initialized".to_owned()),
                None,
                None,
                serde_json::json!({
                  "components": ["pool_discovery", "pool_analyzer", "account_fetcher", "price_calculator"],
                  "rpc_urls_count": rpc_urls_count,
                  "status": "ready"
                }),
            ))
            .await;
            logger::info(
                LogTag::PoolService,
                "Service components initialized successfully",
            );
        }
        Err(e) => {
            SERVICE_RUNNING.store(false, Ordering::Relaxed);

            record_safe(Event::error(
                EventCategory::System,
                Some("pool_service_component_init_failed".to_owned()),
                None,
                None,
                serde_json::json!({
                  "error": e.to_string(),
                  "component": "service_components",
                  "action": "initialize"
                }),
            ))
            .await;

            return Err(Error::ComponentInit {
                detail: e.to_string(),
            });
        }
    }

    // Log pool monitoring mode configuration
    if single_pool_mode {
        logger::info(
            LogTag::PoolService,
            "Pool monitoring mode: SINGLE POOL (highest liquidity only)",
        );
    } else {
        logger::info(
            LogTag::PoolService,
            "Pool monitoring mode: ALL POOLS (comprehensive coverage)",
        );
    }

    // Warm cache for open positions - ensures fresh price data at startup
    for &chain in enabled_chains() {
        warm_cache_for_open_positions(chain).await;
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
          "single_pool_mode": single_pool_mode,
          "components_ready": true
        }),
    ))
    .await;

    Ok(())
}

/// Stop the pool service
///
/// Clears the running flag and calls `clear_chain_components` to release the
/// concrete runtime the composition root selected. The maintenance task stops
/// on the service manager's shutdown signal.
pub async fn stop_pool_service<F>(clear_chain_components: F)
where
    F: FnOnce(),
{
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
    clear_chain_components();

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

/// Check if the pool service is currently running
pub fn is_pool_service_running() -> bool {
    SERVICE_RUNNING.load(Ordering::SeqCst)
}

/// Check if single pool mode is enabled
pub fn is_single_pool_mode_enabled() -> bool {
    with_config(|cfg| cfg.pools.enable_single_pool_mode)
}

/// The open positions' mints that `chain` accepts as addresses. Positions
/// carry no chain yet, so the chain's own address rule selects them.
async fn open_mints_on(chain: ChainId) -> Vec<String> {
    let adapter = adapter_for(chain);
    crate::positions::get_open_mints()
        .await
        .into_iter()
        .filter(|mint| adapter.validate_address(mint).is_ok())
        .collect()
}

/// Warm cache for open positions by prefetching their pool data
///
/// This ensures that tokens with open positions have fresh price data
/// immediately available when trading starts, rather than relying on
/// stale cached data or waiting for the first discovery tick.
async fn warm_cache_for_open_positions(chain: ChainId) {
    let open_mints = open_mints_on(chain).await;

    if open_mints.is_empty() {
        logger::debug(
            LogTag::PoolService,
            &format!("No open positions to warm cache for on {chain}"),
        );
        return;
    }

    logger::info(
        LogTag::PoolService,
        &format!(
            "Warming pool cache for {} tokens with open positions on {}",
            open_mints.len(),
            chain
        ),
    );

    // Use the tokens module prefetch to warm pool data cache
    crate::tokens::prefetch_token_pools(chain, &open_mints).await;

    logger::info(
        LogTag::PoolService,
        &format!(
            "Pool cache warming completed for {} position tokens on {}",
            open_mints.len(),
            chain
        ),
    );
}
