/// Pool service supervisor - manages the lifecycle of all pool-related tasks
///
/// This module provides the main entry points for starting and stopping the pool service.
/// It coordinates all the background tasks needed for price discovery and calculation.

use crate::logger::{ log, LogTag };
use crate::global::is_debug_pool_service_enabled;
use crate::rpc::{ get_rpc_client, RpcClient };
use super::{ PoolError, cache };
use super::discovery::PoolDiscovery;
use super::analyzer::PoolAnalyzer;
use super::fetcher::AccountFetcher;
use super::calculator::PriceCalculator;
use super::types::{ MAX_WATCHED_TOKENS, POOL_REFRESH_INTERVAL_SECONDS, ProgramKind };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::{ Arc, RwLock };
use std::collections::HashMap;
use tokio::sync::Notify;
use std::time::Duration;

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

// Internal accessors (used within module graph to avoid exposing statics directly)
pub(super) fn get_account_fetcher() -> Option<Arc<AccountFetcher>> {
    unsafe { ACCOUNT_FETCHER.clone() }
}
pub(super) fn get_price_calculator() -> Option<Arc<PriceCalculator>> {
    unsafe { PRICE_CALCULATOR.clone() }
}
pub(super) fn get_pool_analyzer() -> Option<Arc<PoolAnalyzer>> {
    unsafe { POOL_ANALYZER.clone() }
}

/// Start the pool service with all background tasks
///
/// This function initializes and starts all the necessary background tasks for
/// pool discovery, price calculation, and caching.
///
/// Returns an error if the service is already running or if initialization fails.
pub async fn start_pool_service() -> Result<(), PoolError> {
    // Check if already running
    if SERVICE_RUNNING.swap(true, Ordering::SeqCst) {
        log(LogTag::PoolService, "WARN", "Pool service is already running");
        return Err(PoolError::InitializationFailed("Service already running".to_string()));
    }

    log(LogTag::PoolService, "INFO", "Starting pool service...");

    // Initialize cache system first
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
            log(LogTag::PoolService, "INFO", "Service components initialized successfully");
        }
        Err(e) => {
            SERVICE_RUNNING.store(false, Ordering::Relaxed);
            unsafe {
                GLOBAL_SHUTDOWN_HANDLE = None;
            }
            return Err(
                PoolError::InitializationFailed(format!("Component initialization failed: {}", e))
            );
        }
    }

    // Log pool monitoring mode configuration
    if ENABLE_SINGLE_POOL_MODE {
        log(
            LogTag::PoolService,
            "INFO",
            "Pool monitoring mode: SINGLE POOL (highest liquidity only)"
        );
    } else {
        log(
            LogTag::PoolService,
            "INFO",
            "Pool monitoring mode: ALL POOLS (comprehensive coverage)"
        );
    }

    // Start background tasks
    start_background_tasks(shutdown).await;

    log(LogTag::PoolService, "SUCCESS", "Pool service started successfully");
    Ok(())
}

/// Stop the pool service and all background tasks
///
/// This function gracefully shuts down all background tasks and cleans up resources.
/// It waits for tasks to complete within the specified timeout.
pub async fn stop_pool_service(timeout_seconds: u64) -> Result<(), PoolError> {
    if !SERVICE_RUNNING.load(Ordering::Relaxed) {
        log(LogTag::PoolService, "WARN", "Pool service is not running");
        return Ok(());
    }

    log(
        LogTag::PoolService,
        "INFO",
        &format!("Stopping pool service (timeout: {}s)...", timeout_seconds)
    );

    // Get shutdown handle and notify
    unsafe {
        if let Some(ref handle) = GLOBAL_SHUTDOWN_HANDLE {
            handle.notify_waiters();
        }
    }

    // Wait for shutdown with timeout
    let shutdown_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(timeout_seconds),
        async {
            // Give tasks time to shutdown gracefully
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    ).await;

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

            log(LogTag::PoolService, "SUCCESS", "✅ Pool service stopped successfully");
            Ok(())
        }
        Err(_) => {
            log(LogTag::PoolService, "WARN", "Pool service shutdown timed out");
            Err(PoolError::InitializationFailed("Shutdown timeout".to_string()))
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

/// Initialize all service components
async fn initialize_service_components() -> Result<(), String> {
    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "DEBUG", "Initializing service components...");
    }

    // Get the global RPC client and create an Arc reference
    // Note: This creates a new Arc wrapping the global client reference
    let rpc_client_ref = get_rpc_client();

    // We need to create an owned RpcClient to wrap in Arc for sharing
    // For now, we'll create a clone or work around this design issue
    let rpc_urls = rpc_client_ref.get_all_urls();
    let owned_rpc_client = Arc::new(
        crate::rpc::RpcClient
            ::new_with_urls(rpc_urls)
            .map_err(|e| format!("Failed to create owned RPC client: {}", e))?
    );

    // Initialize pool directory (shared between components)
    let pool_directory = Arc::new(RwLock::new(HashMap::new()));

    // Initialize components in dependency order
    let pool_discovery = Arc::new(PoolDiscovery::new());
    let pool_analyzer = Arc::new(
        PoolAnalyzer::new(owned_rpc_client.clone(), pool_directory.clone())
    );
    let account_fetcher = Arc::new(
        AccountFetcher::new(owned_rpc_client.clone(), pool_directory.clone())
    );
    let price_calculator = Arc::new(PriceCalculator::new(pool_directory.clone())); // Store components globally
    unsafe {
        POOL_DISCOVERY = Some(pool_discovery);
        POOL_ANALYZER = Some(pool_analyzer);
        ACCOUNT_FETCHER = Some(account_fetcher);
        PRICE_CALCULATOR = Some(price_calculator);
    }

    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "DEBUG", "Service components initialized");
    }

    Ok(())
}

/// Get the list of tokens to monitor from the database
async fn get_tokens_to_monitor() -> Result<Vec<String>, String> {
    // Check for debug override first
    if let Some(override_tokens) = get_debug_token_override() {
        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "DEBUG",
                &format!("Using debug token override: {} tokens", override_tokens.len())
            );
        }
        return Ok(override_tokens);
    }

    // Use centralized filtering function - all database access and filtering logic is now in filtering.rs
    crate::filtering::get_filtered_tokens().await
}

/// Start all background tasks for the pool service
async fn start_background_tasks(shutdown: Arc<Notify>) {
    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "DEBUG", "Starting background tasks...");
    }

    // Start discovery task (now the primary source of pools → analyzer)
    if let Some(discovery) = (unsafe { POOL_DISCOVERY.as_ref() }) {
        let shutdown_discovery = shutdown.clone();
        tokio::spawn(async move {
            discovery.start_discovery_task(shutdown_discovery).await;
        });
    }

    // Start pool monitoring supervisor task
    let shutdown_supervisor = shutdown.clone();
    tokio::spawn(async move {
        run_pool_monitoring_supervisor(shutdown_supervisor).await;
    });

    // Start price calculation pipeline
    let shutdown_pipeline = shutdown.clone();
    tokio::spawn(async move {
        run_price_calculation_pipeline(shutdown_pipeline).await;
    });

    // Start service health monitor
    let shutdown_monitor = shutdown.clone();
    tokio::spawn(async move {
        run_service_health_monitor(shutdown_monitor).await;
    });

    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "DEBUG", "Background tasks started");
    }
}

/// Main pool monitoring supervisor task
async fn run_pool_monitoring_supervisor(shutdown: Arc<Notify>) {
    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "INFO", "Starting pool monitoring supervisor");
    }

    let mut interval = tokio::time::interval(Duration::from_secs(POOL_REFRESH_INTERVAL_SECONDS));

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                if is_debug_pool_service_enabled() {
                    log(LogTag::PoolService, "INFO", "Pool monitoring supervisor shutting down");
                }
                break;
            }
            _ = interval.tick() => {
                if let Err(e) = run_monitoring_cycle().await {
                    log(LogTag::PoolService, "ERROR", &format!("Pool monitoring cycle failed: {}", e));
                }
            }
        }
    }
}

/// Run a single pool monitoring cycle
async fn run_monitoring_cycle() -> Result<(), String> {
    // Discovery is now handled by the dedicated discovery task (batch APIs → analyzer)
    // Keep this cycle lightweight for future health checks or adaptive tuning.
    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "DEBUG",
            "Supervisor tick: discovery handled asynchronously; no per-token discovery here"
        );
    }
    Ok(())
}

/// Price calculation pipeline coordinator
async fn run_price_calculation_pipeline(shutdown: Arc<Notify>) {
    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "INFO", "Starting price calculation pipeline");
    }

    // Start individual component tasks
    if let Some(analyzer) = (unsafe { POOL_ANALYZER.as_ref() }) {
        analyzer.start_analyzer_task(shutdown.clone()).await;
    }

    if let Some(fetcher) = (unsafe { ACCOUNT_FETCHER.as_ref() }) {
        fetcher.start_fetcher_task(shutdown.clone()).await;
    }

    if let Some(calculator) = (unsafe { PRICE_CALCULATOR.as_ref() }) {
        calculator.start_calculator_task(shutdown.clone()).await;
    }

    // Wait for shutdown
    shutdown.notified().await;

    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "INFO", "Price calculation pipeline shutting down");
    }
}

/// Service health monitor
async fn run_service_health_monitor(shutdown: Arc<Notify>) {
    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "INFO", "Starting service health monitor");
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
            cache_stats.total_prices,
            cache_stats.fresh_prices,
            cache_stats.history_entries
        )
    );
}
