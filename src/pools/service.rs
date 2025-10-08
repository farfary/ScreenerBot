use super::analyzer::PoolAnalyzer;
use super::calculator::PriceCalculator;
use super::discovery::{PoolDiscovery, ENABLE_DEXSCREENER_DISCOVERY};
use super::fetcher::AccountFetcher;
use super::types::{ProgramKind, MAX_WATCHED_TOKENS, POOL_REFRESH_INTERVAL_SECONDS};
use super::{cache, db, PoolError};
use crate::arguments::is_debug_pool_cache_enabled;
use crate::events::{record_safe, Event, EventCategory, Severity};
use crate::global::{is_debug_pool_cleanup_enabled, is_debug_pool_service_enabled};
/// Pool service supervisor - manages the lifecycle of all pool-related tasks
///
/// This module provides the main entry points for starting and stopping the pool service.
/// It coordinates all the background tasks needed for price discovery and calculation.
use crate::logger::{log, LogTag};
use crate::rpc::{get_rpc_client, RpcClient};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::Notify;

// Global service state
static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);
static mut GLOBAL_SHUTDOWN_HANDLE: Option<Arc<Notify>> = None;

// =============================================================================
// POOL MONITORING CONFIGURATION
// =============================================================================

/// Enable single pool mode - only monitor the highest liquidity pool per token
/// When true: Only the biggest pool by liquidity is monitored (optimized performance)
/// When false: All pools are monitored (comprehensive coverage - current behavior)
pub const ENABLE_SINGLE_POOL_MODE: bool = true;

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
    // Record service start attempt
    record_safe(Event::info(
        EventCategory::System,
        Some("pool_service_start_attempt".to_string()),
        None,
        None,
        serde_json::json!({
            "single_pool_mode": ENABLE_SINGLE_POOL_MODE,
            "max_watched_tokens": MAX_WATCHED_TOKENS,
            "refresh_interval_seconds": POOL_REFRESH_INTERVAL_SECONDS,
            "dexscreener_enabled": ENABLE_DEXSCREENER_DISCOVERY
        }),
    ))
    .await;

    // Check if already running
    if SERVICE_RUNNING.swap(true, Ordering::SeqCst) {
        log(
            LogTag::PoolService,
            "WARN",
            "Pool service is already running",
        );

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

    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "INFO", "Starting pool service...");
    }

    // Initialize database first
    if let Err(e) = db::initialize_database().await {
        log(
            LogTag::PoolService,
            "ERROR",
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
            if is_debug_pool_service_enabled() {
                log(
                    LogTag::PoolService,
                    "INFO",
                    "Service components initialized successfully",
                );
            }
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
    if ENABLE_SINGLE_POOL_MODE {
        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "INFO",
                "Pool monitoring mode: SINGLE POOL (highest liquidity only)",
            );
        }
    } else {
        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "INFO",
                "Pool monitoring mode: ALL POOLS (comprehensive coverage)",
            );
        }
    }

    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "SUCCESS",
            "Pool components initialized successfully",
        );
    }

    record_safe(Event::info(
        EventCategory::System,
        Some("pool_components_initialized".to_string()),
        None,
        None,
        serde_json::json!({
            "status": "initialized",
            "single_pool_mode": ENABLE_SINGLE_POOL_MODE,
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
        log(LogTag::PoolService, "WARN", "Pool service is not running");

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

    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "INFO",
            &format!("Stopping pool service (timeout: {}s)...", timeout_seconds),
        );
    }

    // Get shutdown handle and notify
    unsafe {
        if let Some(ref handle) = GLOBAL_SHUTDOWN_HANDLE {
            handle.notify_waiters();
        }
    }

    // Wait for shutdown with timeout
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

            if is_debug_pool_service_enabled() {
                log(
                    LogTag::PoolService,
                    "SUCCESS",
                    "✅ Pool service stopped successfully",
                );
            }

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
            log(
                LogTag::PoolService,
                "WARN",
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
    ENABLE_SINGLE_POOL_MODE
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
    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "SUCCESS",
            "Pool helper tasks started (3 handles returned)",
        );
    }

    handles
}

/// Initialize all service components
async fn initialize_service_components() -> Result<(), String> {
    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "DEBUG",
            "Initializing service components...",
        );
    }

    record_safe(Event::info(
        EventCategory::System,
        Some("pool_components_init_start".to_string()),
        None,
        None,
        serde_json::json!({
            "dexscreener_enabled": ENABLE_DEXSCREENER_DISCOVERY,
            "action": "component_initialization"
        }),
    ))
    .await;

    // Initialize external APIs required by discovery before starting background tasks
    if ENABLE_DEXSCREENER_DISCOVERY {
        if let Err(e) = crate::tokens::init_dexscreener_api().await {
            // Fail fast because discovery depends on this API when enabled

            record_safe(Event::error(
                EventCategory::System,
                Some("dexscreener_api_init_failed".to_string()),
                None,
                None,
                serde_json::json!({
                    "error": e,
                    "component": "dexscreener_api",
                    "required": true
                }),
            ))
            .await;

            return Err(format!("Failed to initialize DexScreener API: {}", e));
        }
        // Verify global handle is available
        match crate::tokens::get_global_dexscreener_api().await {
            Ok(_) => {
                if is_debug_pool_service_enabled() {
                    log(
                        LogTag::PoolService,
                        "DEBUG",
                        "DexScreener API initialized and global handle acquired",
                    );
                }

                record_safe(Event::info(
                    EventCategory::System,
                    Some("dexscreener_api_initialized".to_string()),
                    None,
                    None,
                    serde_json::json!({
                        "component": "dexscreener_api",
                        "status": "ready"
                    }),
                ))
                .await;
            }
            Err(e) => {
                record_safe(Event::error(
                    EventCategory::System,
                    Some("dexscreener_api_handle_unavailable".to_string()),
                    None,
                    None,
                    serde_json::json!({
                        "error": e,
                        "component": "dexscreener_api",
                        "stage": "handle_verification"
                    }),
                ))
                .await;

                return Err(format!(
                    "DexScreener API global handle unavailable after init: {}",
                    e
                ));
            }
        }
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

    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "DEBUG",
            "Service components initialized",
        );
    }

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
    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "INFO",
            "Starting service health monitor",
        );
    }

    let mut interval = tokio::time::interval(Duration::from_secs(30)); // Health check every 30s

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                if is_debug_pool_service_enabled() {
                    log(LogTag::PoolService, "INFO", "Service health monitor shutting down");
                }
                break;
            }
            _ = interval.tick() => {
                if is_debug_pool_service_enabled() {
                    emit_service_health_stats().await;
                }
            }
        }
    }
}

/// Emit service health statistics
async fn emit_service_health_stats() {
    let cache_stats = cache::get_cache_stats();

    log(
        LogTag::PoolService,
        "HEALTH",
        &format!(
            "Pool service health: {} total prices, {} fresh prices, {} history entries",
            cache_stats.total_prices, cache_stats.fresh_prices, cache_stats.history_entries
        ),
    );
}

/// Database cleanup task - runs periodically to clean old entries
async fn run_database_cleanup_task(shutdown: Arc<Notify>) {
    if is_debug_pool_cleanup_enabled() {
        log(
            LogTag::PoolService,
            "INFO",
            "Starting database cleanup task",
        );
    }

    // Run cleanup every 6 hours
    let mut interval = tokio::time::interval(Duration::from_secs(6 * 60 * 60));

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                if is_debug_pool_cleanup_enabled() {
                    log(LogTag::PoolService, "INFO", "Database cleanup task shutting down");
                }
                break;
            }
            _ = interval.tick() => {
                if let Err(e) = db::cleanup_old_entries().await {
                    log(LogTag::PoolService, "ERROR", &format!("Database cleanup failed: {}", e));
                } else if is_debug_pool_cleanup_enabled() {
                    log(LogTag::PoolService, "INFO", "Database cleanup completed successfully");
                }
            }
        }
    }
}

/// Gap cleanup task - runs periodically to remove gapped price data
async fn run_gap_cleanup_task(shutdown: Arc<Notify>) {
    if is_debug_pool_cleanup_enabled() {
        log(LogTag::PoolService, "INFO", "Starting gap cleanup task");
    }

    // Run gap cleanup every 30 minutes
    let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                if is_debug_pool_cleanup_enabled() {
                    log(LogTag::PoolService, "INFO", "Gap cleanup task shutting down");
                }
                break;
            }
            _ = interval.tick() => {
                // Clean up gapped data from memory
                cleanup_memory_gaps().await;

                // Clean up gapped data from database
                match db::cleanup_all_gapped_data().await {
                    Ok(deleted) => {
                        if deleted > 0 {
                            if is_debug_pool_cleanup_enabled() {
                                log(
                                    LogTag::PoolService,
                                    "GAP_CLEANUP",
                                    &format!("Gap cleanup completed: removed {} gapped entries", deleted)
                                );
                            }
                        } else if is_debug_pool_cleanup_enabled() {
                            log(LogTag::PoolService, "INFO", "Gap cleanup completed: no gapped data found");
                        }
                    }
                    Err(e) => {
                        log(LogTag::PoolService, "ERROR", &format!("Gap cleanup failed: {}", e));
                    }
                }
            }
        }
    }
}

/// Clean up gapped data from in-memory cache
async fn cleanup_memory_gaps() {
    let (total_removed, tokens_cleaned) = cache::cleanup_all_memory_gaps().await;

    if total_removed > 0 && is_debug_pool_cache_enabled() {
        log(
            LogTag::PoolCache,
            "GAP_CLEANUP",
            &format!(
                "Cleaned {} gapped entries from memory across {} tokens",
                total_removed, tokens_cleaned
            ),
        );
    }
}
