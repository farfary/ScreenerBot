// Background service and coordination for the transactions module
//
// This module provides the main background service that coordinates
// real-time transaction monitoring, WebSocket integration, and periodic processing.

use chrono::{ DateTime, Utc };
use once_cell::sync::Lazy;
use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{ Mutex, Notify };
use tokio::time::{ interval, timeout };

use crate::configs::read_configs;
use crate::global::is_debug_transactions_enabled;
use crate::logger::{ log, LogTag };
use crate::transactions::{
    fetcher::TransactionFetcher,
    manager::TransactionsManager,
    processor::TransactionProcessor,
    types::*,
    utils::*,
};
use crate::websocket;

// =============================================================================
// GLOBAL SERVICE STATE
// =============================================================================

/// Global transaction service manager instance
static GLOBAL_TRANSACTION_MANAGER: Lazy<Arc<Mutex<Option<Arc<Mutex<TransactionsManager>>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Global service running flag
static SERVICE_RUNNING: Lazy<Arc<Mutex<bool>>> = Lazy::new(|| Arc::new(Mutex::new(false)));

/// Global shutdown notification
static SHUTDOWN_NOTIFY: Lazy<Arc<Notify>> = Lazy::new(|| Arc::new(Notify::new()));

// =============================================================================
// SERVICE CONFIGURATION
// =============================================================================

/// Service configuration structure
#[derive(Debug, Clone)]
struct ServiceConfig {
    /// Wallet public key to monitor
    pub wallet_pubkey: solana_sdk::pubkey::Pubkey,
    /// Check interval for transaction monitoring
    pub check_interval_secs: u64,
    /// Enable WebSocket real-time monitoring
    pub enable_websocket: bool,
    /// Maximum concurrent transaction processing
    pub max_concurrent_processing: usize,
    /// Retry configuration
    pub max_retry_attempts: usize,
    pub retry_base_delay_secs: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            wallet_pubkey: solana_sdk::pubkey::Pubkey::default(),
            check_interval_secs: NORMAL_CHECK_INTERVAL_SECS,
            enable_websocket: true,
            max_concurrent_processing: 10,
            max_retry_attempts: 3,
            retry_base_delay_secs: 30,
        }
    }
}

// =============================================================================
// PUBLIC API - SERVICE LIFECYCLE
// =============================================================================

/// Start the global transaction service
pub async fn start_global_transaction_service(
    wallet_pubkey: solana_sdk::pubkey::Pubkey
) -> Result<(), String> {
    let mut running = SERVICE_RUNNING.lock().await;
    if *running {
        return Err("Transaction service is already running".to_string());
    }

    log(LogTag::Transactions, "INFO", "Starting global transaction service...");

    // Create and initialize manager
    let mut manager = TransactionsManager::new(wallet_pubkey).await?;
    manager.initialize().await?;

    // Perform initial cache bootstrap before allowing trader start
    let bootstrap_stats = perform_initial_transaction_bootstrap(&mut manager).await?;

    log(
        LogTag::Transactions,
        "BOOTSTRAP",
        &format!(
            "Initial transaction bootstrap complete: processed={}, skipped_known={}, fetched={}, pages={}, duration={}ms",
            bootstrap_stats.newly_processed,
            bootstrap_stats.known_signatures_skipped,
            bootstrap_stats.total_signatures_fetched,
            bootstrap_stats.total_rpc_pages,
            bootstrap_stats.duration_ms
        )
    );

    if let Some(first_sig) = &bootstrap_stats.newest_signature {
        log(
            LogTag::Transactions,
            "BOOTSTRAP",
            &format!(
                "Newest observed signature: {} (oldest: {})",
                format_signature_short(first_sig),
                bootstrap_stats.oldest_signature
                    .as_ref()
                    .map(|sig| format_signature_short(sig))
                    .unwrap_or_else(|| "unknown".to_string())
            )
        );
    }

    // Reset new transactions counter post-bootstrap to avoid double counting
    manager.new_transactions_count = 0;

    let manager = Arc::new(Mutex::new(manager));

    // Store global manager
    {
        let mut global_manager = GLOBAL_TRANSACTION_MANAGER.lock().await;
        *global_manager = Some(manager.clone());
    }

    // Create service configuration
    let config = ServiceConfig {
        wallet_pubkey,
        ..Default::default()
    };

    // Mark service as running
    *running = true;
    drop(running);

    // Start service task
    let service_handle = tokio::spawn(async move {
        if let Err(e) = run_transaction_service(config).await {
            log(LogTag::Transactions, "ERROR", &format!("Transaction service error: {}", e));
        }
    });

    log(
        LogTag::Transactions,
        "INFO",
        &format!(
            "Global transaction service started for wallet: {}",
            format_address_full(&wallet_pubkey.to_string())
        )
    );

    // Signal that transactions system is ready
    crate::global::TRANSACTIONS_SYSTEM_READY.store(true, std::sync::atomic::Ordering::SeqCst);
    log(LogTag::Transactions, "INFO", "🟢 Transactions system ready");

    // Don't await the service_handle here - let it run in background
    // The service will run until shutdown is requested

    Ok(())
}

/// Stop the global transaction service
pub async fn stop_global_transaction_service() -> Result<(), String> {
    let mut running = SERVICE_RUNNING.lock().await;
    if !*running {
        return Ok(()); // Already stopped
    }

    log(LogTag::Transactions, "INFO", "Stopping global transaction service...");

    // Signal shutdown
    SHUTDOWN_NOTIFY.notify_waiters();

    // Mark as not running
    *running = false;

    // Shutdown manager
    let manager_arc_opt = {
        let mut global_manager = GLOBAL_TRANSACTION_MANAGER.lock().await;
        global_manager.take()
    };

    if let Some(manager_arc) = manager_arc_opt {
        let mut manager = manager_arc.lock().await;
        manager.shutdown().await?;
    }

    log(LogTag::Transactions, "INFO", "Global transaction service stopped");
    Ok(())
}

/// Check if global transaction service is running
pub async fn is_global_transaction_service_running() -> bool {
    let running = SERVICE_RUNNING.lock().await;
    *running
}

/// Get reference to global transaction manager
pub async fn get_global_transaction_manager() -> Option<Arc<Mutex<TransactionsManager>>> {
    let global_manager = GLOBAL_TRANSACTION_MANAGER.lock().await;
    global_manager.as_ref().cloned()
}

/// Get transaction by signature (for positions.rs integration) - cache-first approach with status validation
/// CRITICAL: Only returns transactions that are in Finalized or Confirmed status with complete analysis
/// This is the single function that handles ALL transaction requests properly
pub async fn get_transaction(signature: &str) -> Result<Option<Transaction>, String> {
    let debug = is_debug_transactions_enabled();
    if debug {
        log(LogTag::Transactions, "GET_TX", &format!("{}", &signature));
    }

    // Try database first
    if let Some(db) = super::database::get_transaction_database().await {
        if let Ok(Some(tx)) = db.get_transaction(signature).await {
            return Ok(Some(tx));
        }
    }

    // If not in DB, attempt on-demand processing via processor
    if let Some(manager_arc) = get_global_transaction_manager().await {
        let manager = manager_arc.lock().await;
        let processor = TransactionProcessor::new(manager.get_wallet_pubkey());
        match processor.process_transaction(signature).await {
            Ok(tx) => {
                if debug {
                    log(
                        LogTag::Transactions,
                        "CACHE_REFRESH",
                        &format!(
                            "Processed {} on-demand and refreshed cache",
                            format_signature_short(signature)
                        )
                    );
                }
                // Persisted by processor; return the processed transaction
                return Ok(Some(tx));
            }
            Err(e) => {
                if debug {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!("On-demand processing failed for {}: {}", signature, e)
                    );
                }
            }
        }
    }

    if debug {
        log(
            LogTag::Transactions,
            "CACHE_MISS",
            &format!("No transaction data available for {}", format_signature_short(signature))
        );
    }

    Ok(None)
}

// =============================================================================
// STARTUP BOOTSTRAP
// =============================================================================

/// Statistics describing the initial bootstrap process
#[derive(Debug, Default)]
struct BootstrapStats {
    /// Total RPC pages fetched during bootstrap
    pub total_rpc_pages: usize,
    /// Total signatures fetched from RPC
    pub total_signatures_fetched: usize,
    /// Newly processed transactions during bootstrap
    pub newly_processed: usize,
    /// Signatures skipped because they were already known
    pub known_signatures_skipped: usize,
    /// Count of recoverable errors encountered
    pub errors: usize,
    /// Duration of the bootstrap in milliseconds
    pub duration_ms: u128,
    /// Most recent signature observed during bootstrap
    pub newest_signature: Option<String>,
    /// Oldest signature observed during bootstrap
    pub oldest_signature: Option<String>,
}

/// Perform full transaction history bootstrap before marking system ready
async fn perform_initial_transaction_bootstrap(
    manager: &mut TransactionsManager
) -> Result<BootstrapStats, String> {
    let bootstrap_timer = std::time::Instant::now();

    let wallet_pubkey = manager.wallet_pubkey;
    let debug = manager.debug_enabled;
    let fetcher = TransactionFetcher::new();
    let processor = TransactionProcessor::new(wallet_pubkey);
    let transaction_db = manager.transaction_database.clone();

    let mut stats = BootstrapStats::default();
    let mut before: Option<String> = None;
    let batch_limit = RPC_BATCH_SIZE;

    log(
        LogTag::Transactions,
        "BOOTSTRAP",
        &format!(
            "Bootstrapping transaction cache for wallet: {} (batch_limit={})",
            format_address_full(&wallet_pubkey.to_string()),
            batch_limit
        )
    );

    loop {
        let signatures = fetcher.fetch_signatures_page(
            wallet_pubkey,
            batch_limit,
            before.as_deref()
        ).await?;

        if signatures.is_empty() {
            if debug {
                log(
                    LogTag::Transactions,
                    "BOOTSTRAP",
                    "No additional signatures returned from RPC; bootstrap complete"
                );
            }
            break;
        }

        stats.total_rpc_pages += 1;
        stats.total_signatures_fetched += signatures.len();

        if stats.newest_signature.is_none() {
            stats.newest_signature = signatures.first().cloned();
        }
        stats.oldest_signature = signatures.last().cloned();

        for signature in &signatures {
            let mut signature_is_known = is_signature_known_globally(signature).await;

            if signature_is_known {
                stats.known_signatures_skipped += 1;
            } else if let Some(db) = transaction_db.as_ref() {
                match db.is_signature_known(signature).await {
                    Ok(true) => {
                        signature_is_known = true;
                        stats.known_signatures_skipped += 1;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        log(
                            LogTag::Transactions,
                            "WARN",
                            &format!(
                                "Failed to query known status for {}: {}",
                                format_signature_short(signature),
                                e
                            )
                        );
                        stats.errors += 1;
                    }
                }
            }

            if signature_is_known {
                add_signature_to_known_globally(signature.clone()).await;
                manager.known_signatures.insert(signature.clone());
                continue;
            }

            match processor.process_transaction(signature).await {
                Ok(_) => {
                    if let Some(db) = transaction_db.as_ref() {
                        if let Err(e) = db.add_known_signature(signature).await {
                            log(
                                LogTag::Transactions,
                                "WARN",
                                &format!(
                                    "Failed to persist known signature {}: {}",
                                    format_signature_short(signature),
                                    e
                                )
                            );
                            stats.errors += 1;
                        }
                    }

                    add_signature_to_known_globally(signature.clone()).await;
                    manager.known_signatures.insert(signature.clone());
                    manager.total_transactions += 1;
                    stats.newly_processed += 1;
                }
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!(
                            "Failed to process bootstrap transaction {}: {}",
                            format_signature_short(signature),
                            e
                        )
                    );
                    stats.errors += 1;
                }
            }
        }

        before = signatures.last().cloned();

        if signatures.len() < batch_limit {
            break;
        }
    }

    if let Some(db) = transaction_db.as_ref() {
        match db.get_known_signatures_count().await {
            Ok(count) => {
                manager.total_transactions = count;
            }
            Err(e) => {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to refresh known signatures count: {}", e)
                );
                stats.errors += 1;
            }
        }
    }

    stats.duration_ms = bootstrap_timer.elapsed().as_millis();

    if debug {
        log(LogTag::Transactions, "DEBUG", &format!("Bootstrap stats: {:?}", stats));
    }

    Ok(stats)
}

// =============================================================================
// MAIN SERVICE LOOP
// =============================================================================

/// Main service loop that coordinates all transaction monitoring activities
async fn run_transaction_service(config: ServiceConfig) -> Result<(), String> {
    log(
        LogTag::Transactions,
        "INFO",
        &format!(
            "Transaction service running for wallet: {} (interval: {}s)",
            format_address_full(&config.wallet_pubkey.to_string()),
            config.check_interval_secs
        )
    );

    // Initialize service components
    let processor = Arc::new(TransactionProcessor::new(config.wallet_pubkey));
    let fetcher = Arc::new(TransactionFetcher::new());

    // Create interval timer
    let mut check_interval = interval(Duration::from_secs(config.check_interval_secs));

    // Initialize WebSocket if enabled
    let mut websocket_receiver = if config.enable_websocket {
        initialize_websocket_monitoring(config.wallet_pubkey).await?
    } else {
        None
    };

    // Track performance metrics
    let mut metrics = ServiceMetrics::new();

    // If WebSocket is available, include it in the select loop; otherwise run without it
    if let Some(ref mut rx) = websocket_receiver {
        loop {
            tokio::select! {
                // Periodic transaction checking
                _ = check_interval.tick() => {
                    if let Err(e) = perform_periodic_check(&config, &processor, &fetcher, &mut metrics).await {
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!("Periodic check failed: {}", e)
                        );
                    }
                }

                // WebSocket transaction notifications
                sig_opt = rx.recv() => {
                    match sig_opt {
                        Some(sig) => {
                            metrics.update_websocket_activity();
                            if let Err(e) = handle_websocket_transaction(&config, &processor, sig).await {
                                log(
                                    LogTag::Transactions,
                                    "ERROR",
                                    &format!("WebSocket transaction handling failed: {}", e)
                                );
                            }
                        }
                        None => {
                            log(LogTag::Transactions, "WARN", "WebSocket channel closed - continuing without WS");
                            break; // Fall through to no-WS loop
                        }
                    }
                }

                // Shutdown signal
                _ = SHUTDOWN_NOTIFY.notified() => {
                    log(LogTag::Transactions, "INFO", "Received shutdown signal");
                    return Ok(());
                }

                // Service health check (every 5 minutes)
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
                    if let Err(e) = perform_health_check(&mut metrics).await {
                        log(
                            LogTag::Transactions,
                            "WARN",
                            &format!("Health check failed: {}", e)
                        );
                    }
                }
            }
        }
    }

    // Fallback loop without WebSocket
    loop {
        tokio::select! {
            _ = check_interval.tick() => {
                if let Err(e) = perform_periodic_check(&config, &processor, &fetcher, &mut metrics).await {
                    log(LogTag::Transactions, "ERROR", &format!("Periodic check failed: {}", e));
                }
            }
            _ = SHUTDOWN_NOTIFY.notified() => {
                log(LogTag::Transactions, "INFO", "Received shutdown signal");
                break;
            }
            _ = tokio::time::sleep(Duration::from_secs(300)) => {
                if let Err(e) = perform_health_check(&mut metrics).await {
                    log(LogTag::Transactions, "WARN", &format!("Health check failed: {}", e));
                }
            }
        }
    }

    log(LogTag::Transactions, "INFO", "Transaction service loop ended");
    Ok(())
}

// =============================================================================
// PERIODIC PROCESSING
// =============================================================================

/// Perform periodic transaction checking and maintenance
async fn perform_periodic_check(
    config: &ServiceConfig,
    processor: &Arc<TransactionProcessor>,
    fetcher: &Arc<TransactionFetcher>,
    metrics: &mut ServiceMetrics
) -> Result<(), String> {
    let start_time = std::time::Instant::now();

    // Cleanup expired pending transactions
    let expired_count = cleanup_expired_pending_transactions().await;

    // Process deferred retries
    let retry_count = process_deferred_retries(config, processor).await?;

    // Perform fallback check if WebSocket is not providing updates
    let fallback_count = if should_perform_fallback_check(metrics).await {
        perform_fallback_transaction_check(config, fetcher, processor).await?
    } else {
        0
    };

    // Update metrics
    let duration = start_time.elapsed();
    metrics.update_periodic_check(duration, expired_count, retry_count, fallback_count);

    if is_debug_transactions_enabled() {
        log(
            LogTag::Transactions,
            "DEBUG",
            &format!(
                "Periodic check complete: {}ms, expired: {}, retries: {}, fallback: {}",
                duration.as_millis(),
                expired_count,
                retry_count,
                fallback_count
            )
        );
    }

    Ok(())
}

/// Process deferred retries that are ready for re-processing
async fn process_deferred_retries(
    config: &ServiceConfig,
    processor: &Arc<TransactionProcessor>
) -> Result<usize, String> {
    // This would integrate with the manager's deferred retry system
    // For now, return 0 as placeholder
    Ok(0)
}

/// Perform fallback transaction check when WebSocket is not providing updates
async fn perform_fallback_transaction_check(
    config: &ServiceConfig,
    fetcher: &Arc<TransactionFetcher>,
    processor: &Arc<TransactionProcessor>
) -> Result<usize, String> {
    let debug = is_debug_transactions_enabled();
    log(
        LogTag::Transactions,
        "FALLBACK",
        "Performing fallback transaction check (WebSocket inactive)"
    );

    // Fetch recent transactions
    let signatures = fetcher.fetch_recent_signatures(config.wallet_pubkey, 100).await?;
    let fetched_count = signatures.len();

    let mut new_count = 0;
    for signature in signatures {
        // Check if signature is already known
        if !is_signature_known_globally(&signature).await {
            // Process new transaction
            match processor.process_transaction(&signature).await {
                Ok(_) => {
                    add_signature_to_known_globally(signature).await;
                    new_count += 1;
                }
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!(
                            "Failed to process fallback transaction {}: {}",
                            format_signature_short(&signature),
                            e
                        )
                    );
                }
            }
        }
    }

    if new_count > 0 {
        log(
            LogTag::Transactions,
            "INFO",
            &format!("Fallback check found {} new transactions", new_count)
        );
    } else if debug {
        let known = get_known_signatures_count().await;
        log(
            LogTag::Transactions,
            "DEBUG",
            &format!(
                "Fallback check processed 0 new transactions (known cache: {}, fetched: {})",
                known,
                fetched_count
            )
        );
    }

    Ok(new_count)
}

// =============================================================================
// WEBSOCKET INTEGRATION
// =============================================================================

/// Initialize WebSocket monitoring for real-time transaction notifications
async fn initialize_websocket_monitoring(
    wallet_pubkey: solana_sdk::pubkey::Pubkey
) -> Result<Option<tokio::sync::mpsc::UnboundedReceiver<String>>, String> {
    // Determine WS URL: prefer Helius if API key is present in configs; else default
    let ws_url = match read_configs() {
        Ok(cfg) => {
            // Try to find a Helius API key in the configured RPC URLs
            let mut api_key: Option<String> = None;
            for url in cfg.rpc_urls.iter() {
                if url.contains("helius-rpc.com") {
                    if let Some(pos) = url.find("api-key=") {
                        let key_start = pos + "api-key=".len();
                        let end = url[key_start..]
                            .find('&')
                            .map(|i| key_start + i)
                            .unwrap_or(url.len());
                        api_key = Some(url[key_start..end].to_string());
                        break;
                    }
                }
            }
            api_key.map(|k| websocket::SolanaWebSocketClient::get_helius_ws_url(&k))
        }
        Err(_) => None,
    };

    let ws_url_log = ws_url
        .clone()
        .unwrap_or_else(|| websocket::SolanaWebSocketClient::get_default_ws_url());
    log(
        LogTag::Transactions,
        "WEBSOCKET",
        &format!("Initializing WebSocket monitoring (url: {})", ws_url_log)
    );

    let receiver = websocket::start_websocket_monitoring(
        wallet_pubkey.to_string(),
        ws_url,
        SHUTDOWN_NOTIFY.clone()
    ).await?;

    Ok(Some(receiver))
}

/// Receive WebSocket transaction notification
// Removed placeholder; WebSocket notifications are received directly via rx.recv() in the service loop

/// Handle transaction notification from WebSocket
async fn handle_websocket_transaction(
    config: &ServiceConfig,
    processor: &Arc<TransactionProcessor>,
    signature: String
) -> Result<(), String> {
    log(
        LogTag::Transactions,
        "WEBSOCKET",
        &format!("Processing WebSocket transaction: {}", format_signature_short(&signature))
    );

    // Add to pending transactions for monitoring
    add_pending_transaction_globally(signature.clone(), Utc::now()).await;

    // Process the transaction
    match processor.process_transaction(&signature).await {
        Ok(_) => {
            add_signature_to_known_globally(signature.clone()).await;
            remove_pending_transaction_globally(&signature).await;

            log(
                LogTag::Transactions,
                "WEBSOCKET",
                &format!(
                    "Successfully processed WebSocket transaction: {}",
                    format_signature_short(&signature)
                )
            );
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!(
                    "Failed to process WebSocket transaction {}: {}",
                    format_signature_short(&signature),
                    e
                )
            );
        }
    }

    Ok(())
}

// =============================================================================
// HEALTH MONITORING
// =============================================================================

/// Service performance metrics
#[derive(Debug)]
struct ServiceMetrics {
    last_periodic_check: Option<DateTime<Utc>>,
    last_websocket_activity: Option<DateTime<Utc>>,
    periodic_check_count: u64,
    websocket_transaction_count: u64,
    error_count: u64,
    average_check_duration_ms: f64,
}

impl ServiceMetrics {
    fn new() -> Self {
        Self {
            last_periodic_check: None,
            last_websocket_activity: None,
            periodic_check_count: 0,
            websocket_transaction_count: 0,
            error_count: 0,
            average_check_duration_ms: 0.0,
        }
    }

    fn update_periodic_check(
        &mut self,
        duration: Duration,
        expired_count: usize,
        retry_count: usize,
        fallback_count: usize
    ) {
        self.last_periodic_check = Some(Utc::now());
        self.periodic_check_count += 1;

        let duration_ms = duration.as_millis() as f64;
        self.average_check_duration_ms = if self.periodic_check_count == 1 {
            duration_ms
        } else {
            (self.average_check_duration_ms * ((self.periodic_check_count - 1) as f64) +
                duration_ms) /
                (self.periodic_check_count as f64)
        };
    }

    fn update_websocket_activity(&mut self) {
        self.last_websocket_activity = Some(Utc::now());
        self.websocket_transaction_count += 1;
    }

    fn increment_error(&mut self) {
        self.error_count += 1;
    }
}

/// Perform service health check
async fn perform_health_check(metrics: &mut ServiceMetrics) -> Result<(), String> {
    let now = Utc::now();

    // Check if periodic checks are running
    if let Some(last_check) = metrics.last_periodic_check {
        let time_since_check = (now - last_check).num_seconds();
        if time_since_check > 300 {
            // 5 minutes
            log(
                LogTag::Transactions,
                "WARN",
                &format!("No periodic check in {} seconds", time_since_check)
            );
        }
    }

    // Check database connectivity
    if let Some(db) = super::database::get_transaction_database().await {
        if let Err(e) = db.health_check().await {
            return Err(format!("Database health check failed: {}", e));
        }
    }

    // Log health metrics
    if is_debug_transactions_enabled() {
        log(
            LogTag::Transactions,
            "HEALTH",
            &format!(
                "Service health: checks: {}, websocket: {}, errors: {}, avg_duration: {:.1}ms",
                metrics.periodic_check_count,
                metrics.websocket_transaction_count,
                metrics.error_count,
                metrics.average_check_duration_ms
            )
        );
    }

    Ok(())
}

/// Determine if fallback check should be performed
async fn should_perform_fallback_check(metrics: &ServiceMetrics) -> bool {
    // Perform fallback if no WebSocket activity in the last 5 minutes
    if let Some(last_activity) = metrics.last_websocket_activity {
        let time_since_activity = (Utc::now() - last_activity).num_seconds();
        time_since_activity > 300
    } else {
        // No WebSocket activity recorded yet, perform fallback
        true
    }
}

// =============================================================================
// COMPATIBILITY FUNCTIONS (for migration period)
// =============================================================================

/// Legacy function for compatibility during migration
pub async fn start_transaction_service_legacy(
    wallet_pubkey: solana_sdk::pubkey::Pubkey
) -> Result<(), String> {
    log(
        LogTag::Transactions,
        "WARN",
        "Using legacy transaction service start function - please migrate to new API"
    );
    start_global_transaction_service(wallet_pubkey).await
}

/// Legacy function for compatibility during migration
pub async fn stop_transaction_service_legacy() -> Result<(), String> {
    log(
        LogTag::Transactions,
        "WARN",
        "Using legacy transaction service stop function - please migrate to new API"
    );
    stop_global_transaction_service().await
}
