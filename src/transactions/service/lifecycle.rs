//! Transaction service lifecycle — manages service start, stop, and graceful shutdown.
//
// Service lifecycle management - start/stop/status and global state

use std::sync::{Arc, LazyLock};
use std::time::Duration;
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
pub static GLOBAL_TRANSACTION_MANAGER: LazyLock<
    Arc<Mutex<Option<Arc<Mutex<TransactionsManager>>>>>,
> = LazyLock::new(|| Arc::new(Mutex::new(None)));

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
        return Err("Transaction service is already running".to_owned());
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

    // Perform initial cache bootstrap before allowing trader start.
    //
    // A network/RPC outage here must NOT crash the whole app: the bootstrap is
    // only a cache pre-fill optimization, and trading entries are already paused
    // by the connectivity service while RPC/internet are unhealthy. So a network
    // error is downgraded to a degraded boot — the service still starts and the
    // periodic check loop catches up automatically once connectivity returns; a
    // restart resumes any unfinished history backfill from the persisted cursor.
    // Non-network errors (DB corruption, etc.) stay fatal.
    match perform_initial_transaction_bootstrap(&manager).await {
        Ok(bootstrap_stats) => {
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
                            .map_or("unknown", |v| v)
                    ),
                );
            }
        }
        Err(e) if is_network_bootstrap_error(&e) => {
            logger::warning(
                LogTag::Transactions,
                &format!(
                    "Initial transaction bootstrap skipped - network/RPC unavailable: {e}. \
                     Starting in degraded mode; the cache will catch up automatically once \
                     connectivity is restored."
                ),
            );
        }
        Err(e) => {
            // Non-network failure is fatal: roll back the global registration we
            // installed above so a later start attempt sees a clean slate.
            {
                let mut global_manager = GLOBAL_TRANSACTION_MANAGER.lock().await;
                *global_manager = None;
            }
            return Err(e);
        }
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
                &format!("Transaction service error: {e}"),
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

/// Classify a bootstrap error string as a transient network/RPC failure.
///
/// Bootstrap errors are stringly-typed (they bubble up from the RPC fetcher as
/// `String`), so we match on the well-known connectivity markers. A match means
/// the failure is an outage we can recover from, not a permanent fault.
fn is_network_bootstrap_error(msg: &str) -> bool {
    let m = msg.to_lowercase();
    [
        "no providers available",
        "network timeout",
        "operation timed out",
        "timed out",
        "error sending request",
        "error trying to connect",
        "connection refused",
        "connection reset",
        "dns error",
        "failed to lookup address",
        "tcp connect error",
        "request timeout",
        "network error",
    ]
    .iter()
    .any(|needle| m.contains(needle))
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
/// Force a full re-process of a transaction from RPC (fetch + analyze + persist),
/// bypassing the DB cache. Used when a cached row exists but lacks swap analysis
/// (e.g. it was stored at submit time before the tx landed), so the verifier can
/// obtain real proceeds and finalize the position.
pub async fn reprocess_transaction(signature: &str) -> Result<Option<Transaction>, String> {
    if let Some(manager_arc) = get_global_transaction_manager().await {
        let manager = manager_arc.lock().await;
        let processor = TransactionProcessor::new(manager.get_wallet_pubkey());
        drop(manager); // Avoid holding lock across RPC

        let mut attempts = 0u32;
        let max_attempts = 3u32;
        let mut delay_ms = 300u64;
        loop {
            match processor.process_transaction(signature).await {
                Ok(tx) => return Ok(Some(tx)),
                Err(e) => {
                    let el = e.to_lowercase();
                    let indexing_delay = el.contains("not yet indexed")
                        || el.contains("not found")
                        || el.contains("transaction not available");
                    if indexing_delay && attempts < max_attempts - 1 {
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        attempts += 1;
                        delay_ms = ((delay_ms as f64) * 1.8) as u64;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }
    Ok(None)
}

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
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
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
