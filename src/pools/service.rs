//! Pool service supervisor - chain-neutral lifecycle for the pool runtime
//!
//! Owns the running flag, shutdown protocol, event recording and the shared
//! db/cache initialization that every chain's pool runtime needs. The concrete
//! runtime components (discovery/analyzer/fetcher/calculator) are chain-owned —
//! for Solana that is `crate::chains::solana::pools::service`. The composition
//! root (`src/services/implementations/pools_service.rs`) selects that
//! implementation and passes its `initialize_components`/`clear_components`
//! functions in here, so this module never imports `crate::chains::solana`.

use super::types::max_watched_tokens;
use super::{cache, db, Error};
use crate::chains::active_chain;
use crate::config::with_config;
use crate::events::{record_safe, Event, EventCategory};
use crate::logger::{self, LogTag};

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::Notify;

// Timing constants
const FETCH_INTERVAL_MS: u64 = 500;

// Global service state
static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);

// Thread-safe global state using Lazy + RwLock pattern
static GLOBAL_SHUTDOWN_HANDLE: LazyLock<RwLock<Option<Arc<Notify>>>> =
    LazyLock::new(|| RwLock::new(None));

// Debug override for token monitoring (used by debug tools)
static DEBUG_TOKEN_OVERRIDE: LazyLock<RwLock<Option<Vec<String>>>> =
    LazyLock::new(|| RwLock::new(None));

/// Initialize pool components only (no background tasks)
///
/// Initializes the chain-neutral pool service state (database, cache, shutdown
/// handle) and then runs `init_chain_components` to bring up the concrete
/// runtime for whichever chain the composition root selected.
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

    // Initialize database first
    if let Err(e) = db::initialize_database(active_chain()).await {
        logger::error(
            LogTag::PoolService,
            &format!("Failed to initialize database: {e}"),
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

    // Initialize cache system after database
    cache::initialize_cache().await;

    // Create shutdown notification
    let shutdown = Arc::new(Notify::new());

    // Store shutdown handle globally
    if let Ok(mut handle) = GLOBAL_SHUTDOWN_HANDLE.write() {
        *handle = Some(shutdown.clone());
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
            if let Ok(mut handle) = GLOBAL_SHUTDOWN_HANDLE.write() {
                *handle = None;
            }

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
    warm_cache_for_open_positions().await;

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

/// Stop the pool service and all background tasks
///
/// Gracefully shuts down all background tasks and clears the chain-neutral
/// state, calling `clear_chain_components` to release the concrete runtime
/// the composition root selected.
pub async fn stop_pool_service<F>(
    timeout_seconds: u64,
    clear_chain_components: F,
) -> Result<(), Error>
where
    F: FnOnce(),
{
    record_safe(Event::info(
        EventCategory::System,
        Some("pool_service_stop_attempt".to_owned()),
        None,
        None,
        serde_json::json!({
          "timeout_seconds": timeout_seconds,
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

        return Ok(());
    }

    logger::info(
        LogTag::PoolService,
        &format!("Stopping pool service (timeout: {timeout_seconds}s)..."),
    );

    // Get shutdown handle and notify
    if let Ok(handle) = GLOBAL_SHUTDOWN_HANDLE.read() {
        if let Some(ref notify) = *handle {
            notify.notify_waiters();
        }
    } // Wait for shutdown with timeout
    let shutdown_result = tokio::time::timeout(Duration::from_secs(timeout_seconds), async {
        // Give tasks time to shutdown gracefully
        tokio::time::sleep(Duration::from_millis(500)).await;
    })
    .await;

    match shutdown_result {
        Ok(_) => {
            SERVICE_RUNNING.store(false, Ordering::Relaxed);

            // Clean up global components
            if let Ok(mut handle) = GLOBAL_SHUTDOWN_HANDLE.write() {
                *handle = None;
            }
            clear_chain_components();

            logger::info(LogTag::PoolService, "Pool service stopped successfully");

            record_safe(Event::info(
                EventCategory::System,
                Some("pool_service_stopped".to_owned()),
                None,
                None,
                serde_json::json!({
                  "status": "stopped",
                  "clean_shutdown": true,
                  "timeout_seconds": timeout_seconds
                }),
            ))
            .await;

            Ok(())
        }
        Err(_) => {
            logger::warning(LogTag::PoolService, "Pool service shutdown timed out");

            record_safe(Event::error(
                EventCategory::System,
                Some("pool_service_stop_timeout".to_owned()),
                None,
                None,
                serde_json::json!({
                  "error": "Shutdown timeout",
                  "timeout_seconds": timeout_seconds,
                  "forced_cleanup": true
                }),
            ))
            .await;

            Err(Error::ShutdownTimeout { timeout_seconds })
        }
    }
}

/// Check if the pool service is currently running
pub fn is_pool_service_running() -> bool {
    SERVICE_RUNNING.load(Ordering::SeqCst)
}

/// Check if single pool mode is enabled
pub fn is_single_pool_mode_enabled() -> bool {
    with_config(|cfg| cfg.pools.enable_single_pool_mode)
}

/// Set debug token override for monitoring only specific tokens (debug use only)
///
/// When set, the pool service will monitor only these tokens instead of
/// discovering tokens from the database. Use None to disable override.
pub fn set_debug_token_override(tokens: Option<Vec<String>>) {
    if let Ok(mut override_guard) = DEBUG_TOKEN_OVERRIDE.write() {
        *override_guard = tokens;
    }
}

/// Get current debug token override
pub fn get_debug_token_override() -> Option<Vec<String>> {
    DEBUG_TOKEN_OVERRIDE.read().ok()?.clone()
}

/// Start helper background tasks (health monitor, database cleanup, gap cleanup)
///
/// These are utility tasks that don't need to be separate services.
/// Called by PoolsService after all pool sub-services are started.
///
/// Returns JoinHandles so ServiceManager can wait for graceful shutdown.
pub async fn start_helper_tasks(
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();

    // Start service health monitor
    let shutdown_monitor = shutdown.clone();
    let monitor_1 = monitor.clone();
    handles.push(tokio::spawn(monitor_1.instrument(async move {
        run_service_health_monitor(shutdown_monitor).await;
    })));

    // Start database cleanup task
    let shutdown_cleanup = shutdown.clone();
    let monitor_2 = monitor.clone();
    handles.push(tokio::spawn(monitor_2.instrument(async move {
        run_database_cleanup_task(shutdown_cleanup).await;
    })));

    // Start gap detection and cleanup task
    let shutdown_gap_cleanup = shutdown.clone();
    handles.push(tokio::spawn(monitor.instrument(async move {
        run_gap_cleanup_task(shutdown_gap_cleanup).await;
    })));

    // Set readiness flag
    crate::global::POOL_SERVICE_READY.store(true, std::sync::atomic::Ordering::SeqCst);
    logger::info(
        LogTag::PoolService,
        "Pool helper tasks started (3 handles returned)",
    );

    handles
}

/// Service health monitor
async fn run_service_health_monitor(shutdown: Arc<Notify>) {
    logger::info(LogTag::PoolService, "Starting service health monitor");

    let mut interval = tokio::time::interval(Duration::from_secs(30)); // Health check every 30s

    loop {
        tokio::select! {
          _ = shutdown.notified() => {
            logger::info(LogTag::PoolService, "Service health monitor shutting down");
            break;
          }
          _ = interval.tick() => {
            // Emit health stats unconditionally; internal function performs its own level control
            emit_service_health_stats().await;
          }
        }
    }
}

/// Emit service health statistics
async fn emit_service_health_stats() {
    let cache_stats = cache::get_cache_stats();

    logger::debug(
        LogTag::PoolService,
        &format!(
            "Pool service health: {} total prices, {} fresh prices, {} history entries",
            cache_stats.total_prices, cache_stats.fresh_prices, cache_stats.history_entries
        ),
    );
}

/// Database cleanup task - runs periodically to clean old entries
async fn run_database_cleanup_task(shutdown: Arc<Notify>) {
    logger::info(LogTag::PoolService, "Starting database cleanup task");

    // Run cleanup every 6 hours
    let mut interval = tokio::time::interval(Duration::from_secs(6 * 60 * 60));

    loop {
        tokio::select! {
          _ = shutdown.notified() => {
              logger::info(LogTag::PoolService, "Database cleanup task shutting down");
            break;
          }
          _ = interval.tick() => {
            if let Err(e) = db::cleanup_old_entries(active_chain()).await {
              logger::error(LogTag::PoolService, &format!("Database cleanup failed: {e}"));
            } else {
              logger::info(LogTag::PoolService, "Database cleanup completed successfully");
            }
          }
        }
    }
}

/// Gap cleanup task - runs periodically to remove gapped price data
async fn run_gap_cleanup_task(shutdown: Arc<Notify>) {
    logger::info(LogTag::PoolService, "Starting gap cleanup task");

    // Run gap cleanup every 30 minutes
    let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));

    loop {
        tokio::select! {
          _ = shutdown.notified() => {
              logger::info(LogTag::PoolService, "Gap cleanup task shutting down");
            break;
          }
          _ = interval.tick() => {
            // Clean up gapped data from memory
            cleanup_memory_gaps().await;

            // Clean up gapped data from database
            match db::cleanup_all_gapped_data(active_chain()).await {
              Ok(deleted) => {
                if deleted > 0 {
                  logger::info(
                    LogTag::PoolService,
                    &format!("Gap cleanup completed: removed {deleted} gapped entries"),
                  );
                } else {
                  logger::info(LogTag::PoolService, "Gap cleanup completed: no gapped data found");
                }
              }
              Err(e) => {
                logger::error(LogTag::PoolService, &format!("Gap cleanup failed: {e}"));
              }
            }
          }
        }
    }
}

/// Clean up gapped data from in-memory cache
async fn cleanup_memory_gaps() {
    let (total_removed, tokens_cleaned) = cache::cleanup_all_memory_gaps().await;

    if total_removed > 0 {
        logger::info(
            LogTag::PoolCache,
            &format!(
                "Cleaned {} gapped entries from memory across {} tokens",
                total_removed, tokens_cleaned
            ),
        );
    }
}

/// Warm cache for open positions by prefetching their pool data
///
/// This ensures that tokens with open positions have fresh price data
/// immediately available when trading starts, rather than relying on
/// stale cached data or waiting for the first discovery tick.
async fn warm_cache_for_open_positions() {
    let open_mints = crate::positions::get_open_mints().await;

    if open_mints.is_empty() {
        logger::debug(LogTag::PoolService, "No open positions to warm cache for");
        return;
    }

    logger::info(
        LogTag::PoolService,
        &format!(
            "Warming pool cache for {} tokens with open positions",
            open_mints.len()
        ),
    );

    // Use the tokens module prefetch to warm pool data cache
    crate::tokens::prefetch_token_pools(active_chain(), &open_mints).await;

    logger::info(
        LogTag::PoolService,
        &format!(
            "Pool cache warming completed for {} position tokens",
            open_mints.len()
        ),
    );
}
