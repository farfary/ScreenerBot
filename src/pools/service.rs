use super::analyzer::PoolAnalyzer;
use super::calculator::PriceCalculator;
use super::discovery::{is_dexscreener_discovery_enabled, PoolDiscovery};
use super::fetcher::AccountFetcher;
use super::types::max_watched_tokens;
use super::{cache, db, PoolError};
use crate::config::with_config;
use crate::events::{record_safe, Event, EventCategory, Severity};
/// Pool service supervisor - manages the lifecycle of all pool-related tasks
///
/// This module provides the main entry points for starting and stopping the pool service.
/// It coordinates all the background tasks needed for price discovery and calculation.
use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClient};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::Notify;

// Timing constants
const FETCH_INTERVAL_MS: u64 = 500;

// Global service state
static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);
static mut GLOBAL_SHUTDOWN_HANDLE: Option<Arc<Notify>> = None;

// =============================================================================
// POOL MONITORING CONFIGURATION
// =============================================================================

// Debug override for token monitoring (used by debug tools)
static mut DEBUG_TOKEN_OVERRIDE: Option<Vec<String>> = None;

// Service components (will be initialized when service starts)
static mut POOL_DISCOVERY: Option<Arc<PoolDiscovery>> = None;
static mut POOL_ANALYZER: Option<Arc<PoolAnalyzer>> = None;
static mut ACCOUNT_FETCHER: Option<Arc<AccountFetcher>> = None;
static mut PRICE_CALCULATOR: Option<Arc<PriceCalculator>> = None;

// Public accessors for service manager (used by individual service implementations)
pub fn get_pool_discovery() -> Option<Arc<PoolDiscovery>> {
    unsafe { POOL_DISCOVERY.clone() }
}

pub fn get_account_fetcher() -> Option<Arc<AccountFetcher>> {
    unsafe { ACCOUNT_FETCHER.clone() }
}

pub fn get_price_calculator() -> Option<Arc<PriceCalculator>> {
    unsafe { PRICE_CALCULATOR.clone() }
}

pub fn get_pool_analyzer() -> Option<Arc<PoolAnalyzer>> {
    unsafe { POOL_ANALYZER.clone() }
}

/// Initialize pool components only (no background tasks)
///
/// This function initializes the pool service components (database, cache, RPC client, components)
/// without starting any background tasks. Background tasks are now managed by separate services.
///
/// Returns an error if already initialized or if initialization fails.
pub async fn initialize_pool_components() -> Result<(), PoolError> {
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
        Some("pool_service_start_attempt".to_string()),
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
            Some("pool_service_already_running".to_string()),
            None,
            None,
            serde_json::json!({
                "error": "Service already running",
                "action": "start_rejected"
            }),
        ))
        .await;

        return Err(PoolError::InitializationFailed(
            "Service already running".to_string(),
        ));
    }

    logger::info(LogTag::PoolService, "Starting pool service...");

    // Initialize database first
    if let Err(e) = db::initialize_database().await {
        logger::error(
            LogTag::PoolService,
            &format!("Failed to initialize database: {}", e),
        );
        SERVICE_RUNNING.store(false, Ordering::Relaxed);

        record_safe(Event::error(
            EventCategory::System,
            Some("pool_service_db_init_failed".to_string()),
            None,
            None,
            serde_json::json!({
                "error": e,
                "component": "database",
                "action": "initialize"
            }),
        ))
        .await;

        return Err(PoolError::InitializationFailed(format!(
            "Database initialization failed: {}",
            e
        )));
    }

    // Initialize cache system after database
    cache::initialize_cache().await;

    // Create shutdown notification
    let shutdown = Arc::new(Notify::new());

    // Store shutdown handle globally
    unsafe {
        GLOBAL_SHUTDOWN_HANDLE = Some(shutdown.clone());
    }

    // Initialize service components
    match initialize_service_components().await {
        Ok(_) => {
            logger::info(
                LogTag::PoolService,
                "Service components initialized successfully",
            );
        }
        Err(e) => {
            SERVICE_RUNNING.store(false, Ordering::Relaxed);
            unsafe {
                GLOBAL_SHUTDOWN_HANDLE = None;
            }

            record_safe(Event::error(
                EventCategory::System,
                Some("pool_service_component_init_failed".to_string()),
                None,
                None,
                serde_json::json!({
                    "error": e,
                    "component": "service_components",
                    "action": "initialize"
                }),
            ))
            .await;

            return Err(PoolError::InitializationFailed(format!(
                "Component initialization failed: {}",
                e
            )));
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

    logger::info(
        LogTag::PoolService,
        "Pool components initialized successfully",
    );

    record_safe(Event::info(
        EventCategory::System,
        Some("pool_components_initialized".to_string()),
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
/// This function gracefully shuts down all background tasks and cleans up resources.
/// It waits for tasks to complete within the specified timeout.
pub async fn stop_pool_service(timeout_seconds: u64) -> Result<(), PoolError> {
    record_safe(Event::info(
        EventCategory::System,
        Some("pool_service_stop_attempt".to_string()),
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
            Some("pool_service_not_running".to_string()),
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
        &format!("Stopping pool service (timeout: {}s)...", timeout_seconds),
    );

    // Get shutdown handle and notify
    unsafe {
        if let Some(ref handle) = GLOBAL_SHUTDOWN_HANDLE {
            handle.notify_waiters();
        }
}    // Wait for shutdown with timeout
    let shutdown_result =
        tokio::time::timeout(tokio::time::Duration::from_secs(timeout_seconds), async {
            // Give tasks time to shutdown gracefully
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        })
        .await;

    match shutdown_result {
        Ok(_) => {
            SERVICE_RUNNING.store(false, Ordering::Relaxed);

            // Clean up global components
            unsafe {
                GLOBAL_SHUTDOWN_HANDLE = None;
                POOL_DISCOVERY = None;
                POOL_ANALYZER = None;
                ACCOUNT_FETCHER = None;
                PRICE_CALCULATOR = None;
            }

            logger::info(
                LogTag::PoolService,
                "✅ Pool service stopped successfully",
            );

            record_safe(Event::info(
                EventCategory::System,
                Some("pool_service_stopped".to_string()),
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
            logger::warning(
                LogTag::PoolService,
                "Pool service shutdown timed out",
            );

            record_safe(Event::error(
                EventCategory::System,
                Some("pool_service_stop_timeout".to_string()),
                None,
                None,
                serde_json::json!({
                    "error": "Shutdown timeout",
                    "timeout_seconds": timeout_seconds,
                    "forced_cleanup": true
                }),
            ))
            .await;

            Err(PoolError::InitializationFailed(
                "Shutdown timeout".to_string(),
            ))
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
    unsafe {
        DEBUG_TOKEN_OVERRIDE = tokens;
    }
}

/// Get current debug token override
pub fn get_debug_token_override() -> Option<Vec<String>> {
    unsafe { DEBUG_TOKEN_OVERRIDE.clone() }
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

/// Initialize all service components
async fn initialize_service_components() -> Result<(), String> {
    let dexscreener_enabled = is_dexscreener_discovery_enabled();
    logger::debug(LogTag::PoolService, "Initializing service components...");

    record_safe(Event::info(
        EventCategory::System,
        Some("pool_components_init_start".to_string()),
        None,
        None,
        serde_json::json!({
            "dexscreener_enabled": dexscreener_enabled,
            "action": "component_initialization"
        }),
    ))
    .await;

    // DexScreener global init removed; discovery now uses internal API clients via tokens subsystem
    if dexscreener_enabled {
        logger::debug(
            LogTag::PoolService,
            "DexScreener discovery enabled (no global init required)",
        );
    }

    // Get the global RPC client and clone it for sharing with pool components
    // This ensures all RPC stats are aggregated in the global instance
    let rpc_client_ref = get_rpc_client();
    let shared_rpc_client = Arc::new(rpc_client_ref.clone());

    let rpc_urls_count = rpc_client_ref.get_all_urls().len();

    // Initialize pool directory (shared between components)
    let pool_directory = Arc::new(RwLock::new(HashMap::new()));

    // Initialize components in dependency order
    let pool_discovery = Arc::new(PoolDiscovery::new());
    let pool_analyzer = Arc::new(PoolAnalyzer::new(
        shared_rpc_client.clone(),
        pool_directory.clone(),
    ));
    let account_fetcher = Arc::new(AccountFetcher::new(
        shared_rpc_client.clone(),
        pool_directory.clone(),
    ));
    let price_calculator = Arc::new(PriceCalculator::new(pool_directory.clone()));

    // Store components globally
    unsafe {
        POOL_DISCOVERY = Some(pool_discovery);
        POOL_ANALYZER = Some(pool_analyzer);
        ACCOUNT_FETCHER = Some(account_fetcher);
        PRICE_CALCULATOR = Some(price_calculator);
    }

    logger::debug(LogTag::PoolService, "Service components initialized");

    record_safe(
        Event::info(
            EventCategory::System,
            Some("pool_components_initialized".to_string()),
            None,
            None,
            serde_json::json!({
                "components": ["pool_discovery", "pool_analyzer", "account_fetcher", "price_calculator"],
                "rpc_urls_count": rpc_urls_count,
                "status": "ready"
            })
        )
    ).await;
    Ok(())
}

// REMOVED: start_background_tasks() - background tasks are now managed by separate services
// REMOVED: run_pool_monitoring_supervisor() - no longer needed (empty supervisor)
// REMOVED: run_monitoring_cycle() - no longer needed
// REMOVED: run_price_calculation_pipeline() - components now started by separate services
// REMOVED: wait_for_transactions_ready() - handled by service dependencies

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

    logger::info(
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
                if let Err(e) = db::cleanup_old_entries().await {
                    logger::error(LogTag::PoolService, &format!("Database cleanup failed: {}", e));
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
                match db::cleanup_all_gapped_data().await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            logger::info(
                                LogTag::PoolService,
                                &format!("Gap cleanup completed: removed {} gapped entries", deleted),
                            );
                        } else {
                            logger::info(LogTag::PoolService, "Gap cleanup completed: no gapped data found");
                        }
                    }
                    Err(e) => {
                        logger::error(LogTag::PoolService, &format!("Gap cleanup failed: {}", e));
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
