/// Pool service supervisor - manages the lifecycle of all pool-related tasks
///
/// This module provides the main entry points for starting and stopping the pool service.
/// It coordinates all the background tasks needed for price discovery and calculation.

use crate::logger::{ log, LogTag };
use crate::global::is_debug_pool_service_enabled;
use crate::tokens::cache::TokenDatabase;
use crate::rpc::{ get_rpc_client, RpcClient };
use super::{ PoolError, cache };
use super::discovery::PoolDiscovery;
use super::analyzer::PoolAnalyzer;
use super::fetcher::AccountFetcher;
use super::calculator::PriceCalculator;
use super::types::{ MAX_WATCHED_TOKENS, POOL_REFRESH_INTERVAL_SECONDS };
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

// Service components (will be initialized when service starts)
static mut POOL_DISCOVERY: Option<Arc<PoolDiscovery>> = None;
static mut POOL_ANALYZER: Option<Arc<PoolAnalyzer>> = None;
static mut ACCOUNT_FETCHER: Option<Arc<AccountFetcher>> = None;
static mut PRICE_CALCULATOR: Option<Arc<PriceCalculator>> = None;

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
    let pool_analyzer = Arc::new(PoolAnalyzer::new(owned_rpc_client.clone()));
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
    let database = TokenDatabase::new().map_err(|e|
        format!("Failed to create token database: {}", e)
    )?;

    // Get tokens ordered by liquidity (highest first)
    let all_tokens = database
        .get_all_tokens_with_update_time().await
        .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

    // Take up to MAX_WATCHED_TOKENS with minimum liquidity threshold
    const MIN_LIQUIDITY_USD: f64 = 100.0; // Only monitor tokens with > $100 liquidity

    let monitored_tokens: Vec<String> = all_tokens
        .into_iter()
        .filter(|(_, _, _, liquidity)| *liquidity >= MIN_LIQUIDITY_USD)
        .take(MAX_WATCHED_TOKENS)
        .map(|(mint, _, _, _)| mint)
        .collect();

    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "DEBUG",
            &format!("Selected {} tokens for monitoring from database", monitored_tokens.len())
        );
    }

    Ok(monitored_tokens)
}

/// Start all background tasks for the pool service
async fn start_background_tasks(shutdown: Arc<Notify>) {
    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "DEBUG", "Starting background tasks...");
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
    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "DEBUG", "Starting pool monitoring cycle");
    }

    // Get tokens to monitor
    let tokens_to_monitor = get_tokens_to_monitor().await?;

    if tokens_to_monitor.is_empty() {
        if is_debug_pool_service_enabled() {
            log(LogTag::PoolService, "DEBUG", "No tokens to monitor, skipping cycle");
        }
        return Ok(());
    }

    // Discover pools for monitored tokens
    let pool_discovery = (unsafe { POOL_DISCOVERY.as_ref() }).ok_or(
        "Pool discovery not initialized"
    )?;

    let mut total_pools_discovered = 0;

    for token_mint in tokens_to_monitor.iter() {
        let pools = pool_discovery.discover_pools_for_token(token_mint).await;
        total_pools_discovered += pools.len();

        // Send pools to analyzer for processing
        if let Some(analyzer) = (unsafe { POOL_ANALYZER.as_ref() }) {
            for pool in pools {
                let program_id = match Pubkey::from_str(pool.program_kind.program_id()) {
                    Ok(id) => id,
                    Err(_) => {
                        log(
                            LogTag::PoolService,
                            "WARN",
                            &format!("Invalid program ID for pool: {}", pool.pool_id)
                        );
                        continue;
                    }
                };

                if
                    let Err(e) = analyzer
                        .get_sender()
                        .send(super::analyzer::AnalyzerMessage::AnalyzePool {
                            pool_id: pool.pool_id,
                            program_id,
                            base_mint: pool.base_mint,
                            quote_mint: pool.quote_mint,
                            liquidity_usd: pool.liquidity_usd,
                        })
                {
                    log(
                        LogTag::PoolService,
                        "WARN",
                        &format!("Failed to send pool to analyzer: {}", e)
                    );
                }
            }
        }
    }

    if is_debug_pool_service_enabled() {
        log(
            LogTag::PoolService,
            "DEBUG",
            &format!(
                "Monitoring cycle completed: {} tokens, {} pools discovered",
                tokens_to_monitor.len(),
                total_pools_discovered
            )
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
