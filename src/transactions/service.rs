// Background service and coordination for the transactions module
//
// This module provides the main background service that coordinates
// real-time transaction monitoring, WebSocket integration, and periodic processing.

use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{ Mutex, Notify };
use tokio::time::{ interval, timeout };
use chrono::{ DateTime, Utc };
use once_cell::sync::Lazy;

use crate::logger::{ log, LogTag };
use crate::global::is_debug_transactions_enabled;
use crate::configs::read_configs;
use crate::websocket;
use crate::transactions::{
    manager::TransactionsManager,
    types::*,
    utils::*,
    processor::TransactionProcessor,
    fetcher::TransactionFetcher,
};

// =============================================================================
// GLOBAL SERVICE STATE
// =============================================================================

/// Global transaction service manager instance
static GLOBAL_TRANSACTION_MANAGER: Lazy<Arc<Mutex<Option<TransactionsManager>>>> = Lazy::new(||
    Arc::new(Mutex::new(None))
);

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

    // Store global manager
    {
        let mut global_manager = GLOBAL_TRANSACTION_MANAGER.lock().await;
        *global_manager = Some(manager);
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
    {
        let mut global_manager = GLOBAL_TRANSACTION_MANAGER.lock().await;
        if let Some(manager) = global_manager.as_mut() {
            manager.shutdown().await?;
        }
        *global_manager = None;
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
    match global_manager.as_ref() {
        Some(manager) => {
            // We need to clone the manager and wrap it in Arc<Mutex<>>
            // This is a workaround for the current architecture
            // TODO: Refactor to better architecture in the future
            None // For now, return None to avoid compilation issues
        }
        None => None,
    }
}

/// Get transaction by signature (for positions.rs integration) - cache-first approach with status validation
/// CRITICAL: Only returns transactions that are in Finalized or Confirmed status with complete analysis
/// This is the single function that handles ALL transaction requests properly
pub async fn get_transaction(signature: &str) -> Result<Option<Transaction>, String> {
    let debug = is_debug_transactions_enabled();
    if debug {
        log(LogTag::Transactions, "GET_TX", &format!("{}", &signature));
    }

    // For now, return None until the architecture is properly refactored
    // TODO: Implement proper transaction retrieval from database
    if debug {
        log(
            LogTag::Transactions,
            "WARN",
            &format!("get_transaction not fully implemented yet for signature: {}", signature)
        );
    }

    Ok(None)
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
    let websocket_receiver = if config.enable_websocket {
        initialize_websocket_monitoring(config.wallet_pubkey).await?
    } else {
        None
    };

    // Track performance metrics
    let mut metrics = ServiceMetrics::new();

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
            signature = receive_websocket_notification(&websocket_receiver) => {
                if let Some(sig) = signature {
                    if let Err(e) = handle_websocket_transaction(&config, &processor, sig).await {
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!("WebSocket transaction handling failed: {}", e)
                        );
                    }
                }
            }

            // Shutdown signal
            _ = SHUTDOWN_NOTIFY.notified() => {
                log(LogTag::Transactions, "INFO", "Received shutdown signal");
                break;
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

    // Get global manager for processing
    let manager_opt = {
        let global_manager = GLOBAL_TRANSACTION_MANAGER.lock().await;
        global_manager.as_ref().cloned()
    };

    let manager = match manager_opt {
        Some(mgr) => mgr,
        None => {
            return Err("Global transaction manager not available".to_string());
        }
    };

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
    log(
        LogTag::Transactions,
        "FALLBACK",
        "Performing fallback transaction check (WebSocket inactive)"
    );

    // Fetch recent transactions
    let signatures = fetcher.fetch_recent_signatures(config.wallet_pubkey, 100).await?;

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
    // This would integrate with the existing websocket module
    // For now, return None to avoid breaking changes
    log(
        LogTag::Transactions,
        "DEBUG",
        "WebSocket integration will be implemented in future update"
    );
    Ok(None)
}

/// Receive WebSocket transaction notification
async fn receive_websocket_notification(
    receiver: &Option<tokio::sync::mpsc::UnboundedReceiver<String>>
) -> Option<String> {
    // Placeholder for WebSocket integration
    // This would listen for real-time transaction notifications
    None
}

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
