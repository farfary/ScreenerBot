// Service lifecycle management - start/stop/status and global state

use std::sync::{Arc, LazyLock};
use tokio::sync::{Mutex, Notify};

use crate::logger::{self, LogTag};
use crate::transactions::{
    manager::TransactionsManager, processor::TransactionProcessor, types::Transaction,
    utils::is_signature_known_globally,
};

use super::bootstrap::perform_initial_transaction_bootstrap;
use super::config::ServiceConfig;
use super::processing::run_transaction_service;

// =============================================================================
// GLOBAL SERVICE STATE
// =============================================================================

/// Global transaction service manager instance
pub static GLOBAL_TRANSACTION_MANAGER: LazyLock<Arc<Mutex<Option<Arc<Mutex<TransactionsManager>>>>>>
    = LazyLock::new(|| Arc::new(Mutex::new(None)));

/// Global service running flag
static SERVICE_RUNNING: LazyLock<Arc<Mutex<bool>>> = LazyLock::new(|| Arc::new(Mutex::new(false)));

/// Global shutdown notification
pub static SHUTDOWN_NOTIFY: LazyLock<Arc<Notify>> = LazyLock::new(|| Arc::new(Notify::new()));

// =============================================================================
// PUBLIC API - SERVICE LIFECYCLE
// =============================================================================

/// Start the global transaction service
///
/// Returns JoinHandle so ServiceManager can wait for graceful shutdown.
pub async fn start_global_transaction_service(
    wallet_pubkey: solana_sdk::pubkey::Pubkey,
    monitor: tokio_metrics::TaskMonitor,
) -> Result<tokio::task::JoinHandle<()>, String> {
    let mut running = SERVICE_RUNNING.lock().await;
    if *running {
        return Err("Transaction service is already running".to_string());
    }

    logger::info(
        LogTag::Transactions,
        "Starting global transaction service...",
    );

    // Create and initialize manager
    let mut manager = TransactionsManager::new(wallet_pubkey).await?;
    manager.initialize().await?;

    // Create manager Arc and register globally BEFORE bootstrap so on-demand calls can access it
    let manager = Arc::new(Mutex::new(manager));
    {
        let mut global_manager = GLOBAL_TRANSACTION_MANAGER.lock().await;
        *global_manager = Some(manager.clone());
    }

    // Perform initial cache bootstrap before allowing trader start
    let bootstrap_stats = perform_initial_transaction_bootstrap(&manager).await?;

    logger::info(
    LogTag::Transactions,
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
        logger::info(
            LogTag::Transactions,
            &format!(
                "Newest observed signature: {} (oldest: {})",
                first_sig,
                bootstrap_stats
                    .oldest_signature
                    .as_ref()
                    .map(|sig| sig)
                    .map_or("unknown", |v| v)
            ),
        );
    }

    // Reset new transactions counter post-bootstrap to avoid double counting
    {
        let mut mgr = manager.lock().await;
        mgr.new_transactions_count = 0;
    }

    // Create service configuration
    let config = ServiceConfig {
        wallet_pubkey,
        ..Default::default()
    };

    // Mark service as running
    *running = true;
    drop(running);

    // Start service task WITH INSTRUMENTATION and return handle so ServiceManager can wait for graceful shutdown
    let service_handle = tokio::spawn(monitor.instrument(async move {
        if let Err(e) = run_transaction_service(config).await {
            logger::info(
                LogTag::Transactions,
                &format!("Transaction service error: {}", e),
            );
        }
    }));

    logger::info(
        LogTag::Transactions,
        &format!(
            "Global transaction service started for wallet: {}",
            &wallet_pubkey.to_string()
        ),
    );

    // Signal that transactions system is ready
    crate::global::TRANSACTIONS_SYSTEM_READY.store(true, std::sync::atomic::Ordering::SeqCst);
    logger::info(
        LogTag::Transactions,
        "Transactions system ready (instrumented)",
    );

    Ok(service_handle)
}

/// Stop the global transaction service
pub async fn stop_global_transaction_service() -> Result<(), String> {
    let mut running = SERVICE_RUNNING.lock().await;
    if !*running {
        return Ok(()); // Already stopped
    }

    logger::info(
        LogTag::Transactions,
        "Stopping global transaction service...",
    );

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

    logger::info(LogTag::Transactions, "Global transaction service stopped");
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
    // Try database first
    if let Some(db) = crate::transactions::database::get_transaction_database().await {
        if let Ok(Some(tx)) = db.get_transaction(signature).await {
            return Ok(Some(tx));
        }
    }

    // If not in DB, attempt on-demand processing via processor with short retries for indexing delays
    if let Some(manager_arc) = get_global_transaction_manager().await {
        let manager = manager_arc.lock().await;
        let processor = TransactionProcessor::new(manager.get_wallet_pubkey());
        drop(manager); // Avoid holding lock across RPC

        let mut attempts = 0u32;
        let max_attempts = 3u32;
        let mut delay_ms = 300u64;

        loop {
            match processor.process_transaction(signature).await {
                Ok(tx) => {
                    return Ok(Some(tx));
                }
                Err(e) => {
                    let el = e.to_lowercase();
                    let indexing_delay = el.contains("not yet indexed")
                        || el.contains("not found")
                        || el.contains("transaction not available");

                    if indexing_delay && attempts < max_attempts - 1 {
                        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                        attempts += 1;
                        delay_ms = ((delay_ms as f64) * 1.8) as u64; // mild backoff
                        continue;
                    }
                    break;
                }
            }
        }
    }

    Ok(None)
}
