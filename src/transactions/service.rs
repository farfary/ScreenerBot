// Background service and coordination for the transactions module
//
// This module provides the main background service that coordinates
// real-time transaction monitoring, WebSocket integration, and periodic processing.

use chrono::{ DateTime, Utc };
use futures::stream::{ StreamExt, FuturesUnordered };
use once_cell::sync::Lazy;
use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{ Mutex, Notify };
use tokio::time::{ interval, timeout, sleep };

use crate::global::is_debug_transactions_enabled;
use crate::logger::{ log, LogTag };
use crate::transactions::{
    fetcher::TransactionFetcher,
    manager::TransactionsManager,
    processor::TransactionProcessor,
    types::*,
    utils::*,
    websocket,
};

// =============================================================================
// BOOTSTRAP CONFIGURATION
// =============================================================================

/// Number of transactions to process concurrently during bootstrap
/// Change this value to adjust parallel processing batch size
const CONCURRENT_BATCH_SIZE: usize = 10;

/// Timeout for individual transaction processing during bootstrap (in seconds)
/// Transactions taking longer than this will be timed out and retried later
const TRANSACTION_TIMEOUT_SECS: u64 = 15;

/// Maximum retry attempts for failed/timed-out transactions
const MAX_RETRY_ATTEMPTS: usize = 3;

/// Base delay between retry attempts (increases exponentially)
const RETRY_BASE_DELAY_SECS: u64 = 2;

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
///
/// Returns JoinHandle so ServiceManager can wait for graceful shutdown.
pub async fn start_global_transaction_service(
    wallet_pubkey: solana_sdk::pubkey::Pubkey
) -> Result<tokio::task::JoinHandle<()>, String> {
    let mut running = SERVICE_RUNNING.lock().await;
    if *running {
        return Err("Transaction service is already running".to_string());
    }

    log(LogTag::Transactions, "INFO", "Starting global transaction service...");

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
                first_sig,
                bootstrap_stats.oldest_signature
                    .as_ref()
                    .map(|sig| sig)
                    .map_or("unknown", |v| v)
            )
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

    // Start service task and return handle so ServiceManager can wait for graceful shutdown
    let service_handle = tokio::spawn(async move {
        if let Err(e) = run_transaction_service(config).await {
            log(LogTag::Transactions, "ERROR", &format!("Transaction service error: {}", e));
        }
    });

    log(
        LogTag::Transactions,
        "INFO",
        &format!("Global transaction service started for wallet: {}", &wallet_pubkey.to_string())
    );

    // Signal that transactions system is ready
    crate::global::TRANSACTIONS_SYSTEM_READY.store(true, std::sync::atomic::Ordering::SeqCst);
    log(LogTag::Transactions, "INFO", "🟢 Transactions system ready");

    Ok(service_handle)
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
                    if debug {
                        log(
                            LogTag::Transactions,
                            "CACHE_REFRESH",
                            &format!("Processed {} on-demand and refreshed cache", signature)
                        );
                    }
                    return Ok(Some(tx));
                }
                Err(e) => {
                    let el = e.to_lowercase();
                    let indexing_delay =
                        el.contains("not yet indexed") ||
                        el.contains("not found") ||
                        el.contains("transaction not available");

                    if debug {
                        log(
                            LogTag::Transactions,
                            "WARN",
                            &format!(
                                "On-demand processing failed for {} (attempt {}): {}",
                                signature,
                                attempts + 1,
                                e
                            )
                        );
                    }

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

    if debug {
        log(
            LogTag::Transactions,
            "CACHE_MISS",
            &format!("No transaction data available for {}", signature)
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
    manager_arc: &Arc<Mutex<TransactionsManager>>
) -> Result<BootstrapStats, String> {
    let bootstrap_timer = std::time::Instant::now();
    let (wallet_pubkey, debug, transaction_db) = {
        let mgr = manager_arc.lock().await;
        (mgr.wallet_pubkey, mgr.debug_enabled, mgr.transaction_database.clone())
    };
    let fetcher = TransactionFetcher::new();
    let processor = Arc::new(TransactionProcessor::new(wallet_pubkey));

    let mut stats = BootstrapStats::default();
    let batch_limit = RPC_BATCH_SIZE;

    // Reconcile known_signatures from already processed at start (safety)
    if let Some(db) = transaction_db.as_ref() {
        if let Ok(added) = db.reconcile_known_with_processed().await {
            if added > 0 {
                log(
                    LogTag::Transactions,
                    "BOOTSTRAP",
                    &format!("Reconciled {} processed->known signatures", added)
                );
            }
        }
    }

    // Determine bootstrap mode using persistent bootstrap_state
    // Backfill mode: full history not completed yet → page newest→oldest using backfill_before_cursor
    // Forward incremental: once full history completed → fetch only newer than newest known
    let mut bootstrap_mode = "FULL".to_string();
    let mut backfill_cursor: Option<String> = None;
    let mut checkpoint_signature: Option<String> = None;
    if let Some(db) = transaction_db.as_ref() {
        match db.get_bootstrap_state().await {
            Ok(state) => {
                if state.full_history_completed {
                    // Forward incremental mode
                    bootstrap_mode = "INCREMENTAL".to_string();
                    checkpoint_signature = db.get_newest_known_signature().await.unwrap_or(None);
                } else {
                    // Backfill mode not completed
                    bootstrap_mode = "FULL".to_string();
                    backfill_cursor = state.backfill_before_cursor;
                    // In FULL/backfill mode we do NOT stop at oldest-known; we continue until chain end.
                    checkpoint_signature = None;
                }
            }
            Err(e) => {
                log(
                    LogTag::Transactions,
                    "WARN",
                    &format!("Failed to load bootstrap state: {}", e)
                );
                // Fallback to default behavior (FULL with checkpoint on oldest-known if any)
                checkpoint_signature = None;
            }
        }
    }

    log(
        LogTag::Transactions,
        "BOOTSTRAP",
        &format!(
            "Bootstrapping transaction cache for wallet: {} (mode={}, batch_limit={})",
            &wallet_pubkey.to_string(),
            &bootstrap_mode,
            batch_limit
        )
    );

    if let Some(ref checkpoint) = checkpoint_signature {
        log(LogTag::Transactions, "BOOTSTRAP", &format!("📌 Checkpoint: {}...", &checkpoint[..8]));
    }

    // =========================================================================
    // PHASE 1: COLLECT SIGNATURES (incremental from checkpoint or full history)
    // =========================================================================
    let phase1_label = match (bootstrap_mode.as_str(), checkpoint_signature.is_some()) {
        ("INCREMENTAL", true) => "NEWER than newest-known",
        ("INCREMENTAL", false) => "RECENT (no checkpoint)",
        ("FULL", true) => "MISSING (until oldest-known)",
        ("FULL", false) => "ALL",
        _ => "ALL",
    };
    log(
        LogTag::Transactions,
        "BOOTSTRAP_PHASE1",
        &format!("📥 Phase 1: Collecting {} signatures...", phase1_label)
    );

    let mut all_signatures: Vec<String> = Vec::new();
    // Start collection:
    // - FULL/backfill: always start from latest (before=None) to re-collect the full window; we'll page until chain end
    // - INCREMENTAL: start from latest as well; we will stop at checkpoint (newest-known)
    let mut before: Option<String> = None;
    let phase1_timer = std::time::Instant::now();
    let mut hit_checkpoint = false;
    let mut reached_chain_end = false;

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
                    "BOOTSTRAP_PHASE1",
                    "No additional signatures returned from RPC"
                );
            }
            reached_chain_end = true;
            break;
        }

        stats.total_rpc_pages += 1;
        let page_count = signatures.len();

        // Check if we hit the checkpoint
        if let Some(ref checkpoint) = checkpoint_signature {
            let mut signatures_to_add = Vec::new();

            for sig in &signatures {
                if sig == checkpoint {
                    hit_checkpoint = true;
                    log(
                        LogTag::Transactions,
                        "BOOTSTRAP_PHASE1",
                        &format!(
                            "✅ Hit checkpoint signature - stopping at page {}",
                            stats.total_rpc_pages
                        )
                    );
                    break;
                }
                signatures_to_add.push(sig.clone());
            }

            all_signatures.extend(signatures_to_add);

            if hit_checkpoint {
                break;
            }
        } else {
            // No checkpoint: in FULL mode collect all; in INCREMENTAL collect just this page
            if bootstrap_mode == "INCREMENTAL" {
                all_signatures.extend(signatures.clone());
                // Only one page needed in forward incremental
                if signatures.len() < batch_limit {/* done anyway */}
                break;
            } else {
                all_signatures.extend(signatures.clone());
            }
        }

        if stats.newest_signature.is_none() {
            stats.newest_signature = signatures.first().cloned();
        }
        stats.oldest_signature = signatures.last().cloned();

        log(
            LogTag::Transactions,
            "BOOTSTRAP_PHASE1",
            &format!(
                "📄 Fetched page {}: {} signatures | total collected: {} | elapsed: {}s",
                stats.total_rpc_pages,
                page_count,
                all_signatures.len(),
                phase1_timer.elapsed().as_secs()
            )
        );

        before = signatures.last().cloned();

        // Persist backfill cursor every page (only in FULL/backfill mode)
        if bootstrap_mode == "FULL" {
            if let Some(db) = transaction_db.as_ref() {
                if let Err(e) = db.set_backfill_cursor(before.as_deref()).await {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!("Failed to persist backfill cursor: {}", e)
                    );
                }
            }
        }

        if signatures.len() < batch_limit {
            if bootstrap_mode == "FULL" {
                reached_chain_end = true;
            }
            break;
        }
    }

    stats.total_signatures_fetched = all_signatures.len();

    let phase1_summary = if bootstrap_mode == "INCREMENTAL" {
        format!(
            "✅ Phase 1 complete (INCREMENTAL): collected {} signatures in {}s across {} pages",
            all_signatures.len(),
            phase1_timer.elapsed().as_secs(),
            stats.total_rpc_pages
        )
    } else if hit_checkpoint {
        format!(
            "✅ Phase 1 complete (INCREMENTAL): collected {} NEW signatures in {}s across {} pages (stopped at checkpoint)",
            all_signatures.len(),
            phase1_timer.elapsed().as_secs(),
            stats.total_rpc_pages
        )
    } else if checkpoint_signature.is_some() {
        format!(
            "✅ Phase 1 complete (INCREMENTAL): collected {} signatures in {}s across {} pages (checkpoint not found - fetched all)",
            all_signatures.len(),
            phase1_timer.elapsed().as_secs(),
            stats.total_rpc_pages
        )
    } else {
        format!(
            "✅ Phase 1 complete (FULL): collected {} signatures in {}s across {} pages",
            all_signatures.len(),
            phase1_timer.elapsed().as_secs(),
            stats.total_rpc_pages
        )
    };

    log(LogTag::Transactions, "BOOTSTRAP_PHASE1", &phase1_summary);

    // =========================================================================
    // PHASE 2: FILTER AND PROCESS NEW SIGNATURES
    // =========================================================================
    log(
        LogTag::Transactions,
        "BOOTSTRAP_PHASE2",
        "⚙️  Phase 2: Filtering and processing new transactions..."
    );

    let phase2_timer = std::time::Instant::now();
    let mut signatures_to_process: Vec<String> = Vec::new();

    // Filter out already known signatures
    for signature in &all_signatures {
        let mut signature_is_known = is_signature_known_globally(signature).await;

        if !signature_is_known {
            if let Some(db) = transaction_db.as_ref() {
                match db.is_signature_known(signature).await {
                    Ok(true) => {
                        signature_is_known = true;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        log(
                            LogTag::Transactions,
                            "WARN",
                            &format!("Failed to query known status for {}: {}", signature, e)
                        );
                        stats.errors += 1;
                    }
                }
            }
        }

        if signature_is_known {
            stats.known_signatures_skipped += 1;
            add_signature_to_known_globally(signature.clone()).await;
            if let Ok(mut mgr) = manager_arc.try_lock() {
                mgr.known_signatures.insert(signature.clone());
            }
        } else {
            signatures_to_process.push(signature.clone());
        }
    }

    let total_to_process = signatures_to_process.len();
    log(
        LogTag::Transactions,
        "BOOTSTRAP_PHASE2",
        &format!(
            "📊 Filtering complete: {} new to process | {} already known | batch_size={} | elapsed: {}s",
            total_to_process,
            stats.known_signatures_skipped,
            CONCURRENT_BATCH_SIZE,
            phase2_timer.elapsed().as_secs()
        )
    );

    // Process all new signatures in parallel batches with accurate progress tracking
    let mut processed_count = 0;
    let mut newly_processed = 0;
    let mut errors = 0;
    let mut failed_signatures: Vec<(String, String)> = Vec::new(); // (signature, error_reason)

    // Split into batches and process in parallel
    for batch_start in (0..signatures_to_process.len()).step_by(CONCURRENT_BATCH_SIZE) {
        let batch_end = std::cmp::min(
            batch_start + CONCURRENT_BATCH_SIZE,
            signatures_to_process.len()
        );
        let batch = &signatures_to_process[batch_start..batch_end];

        // Create futures for parallel processing with timeout
        let mut futures = FuturesUnordered::new();

        for signature in batch {
            let sig = signature.clone();
            let proc = processor.clone();
            futures.push(async move {
                let result = timeout(
                    Duration::from_secs(TRANSACTION_TIMEOUT_SECS),
                    proc.process_transaction(&sig)
                ).await;

                match result {
                    Ok(inner_result) => (sig.clone(), inner_result),
                    Err(_) =>
                        (
                            sig.clone(),
                            Err(
                                format!("Transaction processing timed out after {}s", TRANSACTION_TIMEOUT_SECS)
                            ),
                        ),
                }
            });
        }

        // Process batch in parallel
        while let Some((signature, result)) = futures.next().await {
            match result {
                Ok(_) => {
                    if let Some(db) = transaction_db.as_ref() {
                        if let Err(e) = db.add_known_signature(&signature).await {
                            log(
                                LogTag::Transactions,
                                "WARN",
                                &format!("Failed to persist known signature {}: {}", signature, e)
                            );
                            errors += 1;
                        }
                    }

                    add_signature_to_known_globally(signature.clone()).await;
                    if let Ok(mut mgr) = manager_arc.try_lock() {
                        mgr.known_signatures.insert(signature.clone());
                        mgr.total_transactions += 1;
                    }
                    newly_processed += 1;
                }
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!(
                            "Failed to process bootstrap transaction {}: {} (will retry)",
                            signature,
                            e
                        )
                    );
                    errors += 1;
                    failed_signatures.push((signature.clone(), e));
                }
            }

            processed_count += 1;

            // Show progress summary every 10 transactions or at the end
            if processed_count % 10 == 0 || processed_count == total_to_process {
                let remaining = total_to_process - processed_count;
                let progress_pct = ((processed_count as f64) / (total_to_process as f64)) * 100.0;

                log(
                    LogTag::Transactions,
                    "BOOTSTRAP_PROGRESS",
                    &format!(
                        "📊 Progress: {}/{} ({:.1}%) | new={} | errors={} | remaining={} | elapsed={}s",
                        processed_count,
                        total_to_process,
                        progress_pct,
                        newly_processed,
                        errors,
                        remaining,
                        bootstrap_timer.elapsed().as_secs()
                    )
                );
            }
        }
    }

    log(
        LogTag::Transactions,
        "BOOTSTRAP_PHASE2",
        &format!(
            "✅ Phase 2 complete: processed {}/{} new transactions | errors={} | elapsed={}s",
            newly_processed,
            total_to_process,
            errors,
            phase2_timer.elapsed().as_secs()
        )
    );

    // =========================================================================
    // PHASE 3: RETRY FAILED TRANSACTIONS
    // =========================================================================
    let mut retry_successful = 0;
    let mut permanently_failed = 0;

    if !failed_signatures.is_empty() {
        log(
            LogTag::Transactions,
            "BOOTSTRAP_RETRY",
            &format!(
                "🔄 Phase 3: Retrying {} failed transactions (max {} attempts per transaction)",
                failed_signatures.len(),
                MAX_RETRY_ATTEMPTS
            )
        );

        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            if failed_signatures.is_empty() {
                break;
            }

            let delay_secs = RETRY_BASE_DELAY_SECS * (2_u64).pow((attempt as u32) - 1);

            log(
                LogTag::Transactions,
                "BOOTSTRAP_RETRY",
                &format!(
                    "🔄 Retry attempt {}/{}: {} signatures remaining | delay={}s",
                    attempt,
                    MAX_RETRY_ATTEMPTS,
                    failed_signatures.len(),
                    delay_secs
                )
            );

            // Wait before retrying (exponential backoff)
            if attempt > 1 {
                sleep(Duration::from_secs(delay_secs)).await;
            }

            let mut still_failed: Vec<(String, String)> = Vec::new();

            // Process failed signatures in batches
            for batch_start in (0..failed_signatures.len()).step_by(CONCURRENT_BATCH_SIZE) {
                let batch_end = std::cmp::min(
                    batch_start + CONCURRENT_BATCH_SIZE,
                    failed_signatures.len()
                );
                let batch = &failed_signatures[batch_start..batch_end];

                let mut futures = FuturesUnordered::new();

                for (signature, _) in batch {
                    let sig = signature.clone();
                    let proc = processor.clone();
                    futures.push(async move {
                        let result = timeout(
                            Duration::from_secs(TRANSACTION_TIMEOUT_SECS),
                            proc.process_transaction(&sig)
                        ).await;

                        match result {
                            Ok(inner_result) => (sig.clone(), inner_result),
                            Err(_) =>
                                (
                                    sig.clone(),
                                    Err(
                                        format!("Transaction processing timed out after {}s", TRANSACTION_TIMEOUT_SECS)
                                    ),
                                ),
                        }
                    });
                }

                while let Some((signature, result)) = futures.next().await {
                    match result {
                        Ok(_) => {
                            if let Some(db) = transaction_db.as_ref() {
                                if let Err(e) = db.add_known_signature(&signature).await {
                                    log(
                                        LogTag::Transactions,
                                        "WARN",
                                        &format!(
                                            "Failed to persist known signature {}: {}",
                                            signature,
                                            e
                                        )
                                    );
                                }
                            }

                            add_signature_to_known_globally(signature.clone()).await;
                            if let Ok(mut mgr) = manager_arc.try_lock() {
                                mgr.known_signatures.insert(signature.clone());
                                mgr.total_transactions += 1;
                            }
                            retry_successful += 1;
                        }
                        Err(e) => {
                            still_failed.push((signature.clone(), e));
                        }
                    }
                }
            }

            failed_signatures = still_failed;

            log(
                LogTag::Transactions,
                "BOOTSTRAP_RETRY",
                &format!(
                    "✅ Retry attempt {} complete: {} recovered | {} still failing",
                    attempt,
                    retry_successful,
                    failed_signatures.len()
                )
            );
        }

        permanently_failed = failed_signatures.len();

        if permanently_failed > 0 {
            log(
                LogTag::Transactions,
                "BOOTSTRAP_RETRY",
                &format!(
                    "⚠️  {} transactions permanently failed after {} retry attempts",
                    permanently_failed,
                    MAX_RETRY_ATTEMPTS
                )
            );

            if debug {
                log(
                    LogTag::Transactions,
                    "DEBUG",
                    &format!(
                        "Permanently failed signatures: {:?}",
                        failed_signatures
                            .iter()
                            .map(|(sig, _)| sig)
                            .take(10)
                            .collect::<Vec<_>>()
                    )
                );
            }
        }
    }

    // Update stats with final counts
    stats.newly_processed = newly_processed + retry_successful;
    stats.errors = permanently_failed;

    // Update manager with final count from database
    if let Some(db) = transaction_db.as_ref() {
        match db.get_known_signatures_count().await {
            Ok(count) => {
                if let Ok(mut mgr) = manager_arc.try_lock() {
                    mgr.total_transactions = count;
                }
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

    // If FULL/backfill completed to chain-end (no more pages), mark full history completed so next run uses forward-only incremental
    if bootstrap_mode == "FULL" {
        if let Some(db) = transaction_db.as_ref() {
            if reached_chain_end {
                if let Err(e) = db.mark_full_history_completed().await {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!("Failed to mark full history completed: {}", e)
                    );
                } else {
                    // Clear cursor since we reached chain end
                    if let Err(e) = db.clear_backfill_cursor().await {
                        log(
                            LogTag::Transactions,
                            "WARN",
                            &format!("Failed to clear backfill cursor: {}", e)
                        );
                    }
                    log(
                        LogTag::Transactions,
                        "BOOTSTRAP",
                        "Full history backfill completed. Switching to forward incremental on next start."
                    );
                }
            } else {
                // Persist last cursor already done per-page; ensure it's saved for resume
                if let Err(e) = db.set_backfill_cursor(before.as_deref()).await {
                    log(
                        LogTag::Transactions,
                        "WARN",
                        &format!("Failed to persist final backfill cursor: {}", e)
                    );
                }
            }
        }
    }

    // Final summary
    let summary = if retry_successful > 0 || permanently_failed > 0 {
        format!(
            "✨ Bootstrap complete!\n\
            ═══════════════════════════════════════════════════════════════\n\
            📊 Total signatures found: {}\n\
            ✅ New transactions processed: {} (initial: {} + retried: {})\n\
            ⏭️  Already known (skipped): {}\n\
            🔄 Retry statistics: {} recovered | {} permanently failed\n\
            📄 RPC pages fetched: {}\n\
            ⏱️  Total time: {:.1}s\n\
            ═══════════════════════════════════════════════════════════════",
            stats.total_signatures_fetched,
            stats.newly_processed,
            newly_processed,
            retry_successful,
            stats.known_signatures_skipped,
            retry_successful,
            permanently_failed,
            stats.total_rpc_pages,
            bootstrap_timer.elapsed().as_secs_f64()
        )
    } else {
        format!(
            "✨ Bootstrap complete!\n\
            ═══════════════════════════════════════════════════════════════\n\
            📊 Total signatures found: {}\n\
            ✅ New transactions processed: {}\n\
            ⏭️  Already known (skipped): {}\n\
            ❌ Errors: {}\n\
            📄 RPC pages fetched: {}\n\
            ⏱️  Total time: {:.1}s\n\
            ═══════════════════════════════════════════════════════════════",
            stats.total_signatures_fetched,
            stats.newly_processed,
            stats.known_signatures_skipped,
            stats.errors,
            stats.total_rpc_pages,
            bootstrap_timer.elapsed().as_secs_f64()
        )
    };

    log(LogTag::Transactions, "BOOTSTRAP_COMPLETE", &summary);

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
            &config.wallet_pubkey.to_string(),
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

    // If WebSocket was successfully initialized, mark it as active immediately
    // This prevents unnecessary fallback checks during the initial period when
    // no transactions have occurred yet
    if websocket_receiver.is_some() {
        metrics.update_websocket_activity();
        log(LogTag::Transactions, "INFO", "WebSocket initialized successfully - marking as active");
    }

    // Create WebSocket health check interval (every 15 seconds)
    let mut ws_health_check_interval = interval(Duration::from_secs(15));
    ws_health_check_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Main service loop with WebSocket health monitoring and reconnection
    loop {
        tokio::select! {
            // Periodic transaction checking (every 3 seconds by default)
            _ = check_interval.tick() => {
                if let Err(e) = perform_periodic_check(&config, &processor, &fetcher, &mut metrics).await {
                    log(
                        LogTag::Transactions,
                        "ERROR",
                        &format!("Periodic check failed: {}", e)
                    );
                }
            }

            // WebSocket transaction notifications (if WebSocket is active)
            sig_opt = async {
                if let Some(ref mut rx) = websocket_receiver {
                    rx.recv().await
                } else {
                    // No WebSocket - return pending (never resolves)
                    std::future::pending::<Option<String>>().await
                }
            } => {
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
                        log(
                            LogTag::Transactions,
                            "WARN",
                            "WebSocket channel closed - will attempt reconnection on next health check"
                        );
                        websocket_receiver = None;
                    }
                }
            }

            // WebSocket health check (every 15 seconds)
            _ = ws_health_check_interval.tick() => {
                metrics.update_websocket_health_check();
                
                if config.enable_websocket {
                    let ws_exists = websocket_receiver.is_some();
                    
                    if !ws_exists {
                        // WebSocket doesn't exist - try to establish it
                        // This is the ONLY case where we reconnect
                        log(
                            LogTag::Transactions,
                            "INFO",
                            "🔌 No WebSocket connection - attempting to establish..."
                        );
                        
                        match initialize_websocket_monitoring(config.wallet_pubkey).await {
                            Ok(new_receiver) => {
                                websocket_receiver = new_receiver;
                                metrics.increment_websocket_reconnect();
                                metrics.update_websocket_activity();
                                log(
                                    LogTag::Transactions,
                                    "INFO",
                                    "✅ WebSocket connection established successfully"
                                );
                            }
                            Err(e) => {
                                log(
                                    LogTag::Transactions,
                                    "WARN",
                                    &format!("WebSocket connection failed: {} - will retry in 15s", e)
                                );
                            }
                        }
                    } else {
                        // WebSocket exists - it's healthy (receiver is open means loop is running)
                        // Log debug info about connection state
                        if is_debug_transactions_enabled() {
                            let last_msg_ago = if let Some(last_msg) = metrics.last_websocket_message {
                                (Utc::now() - last_msg).num_seconds()
                            } else {
                                -1
                            };
                            log(
                                LogTag::Transactions,
                                "DEBUG",
                                &format!(
                                    "WebSocket health check: HEALTHY (connection open, last tx message: {}s ago, reconnects: {})",
                                    last_msg_ago,
                                    metrics.websocket_reconnect_count
                                )
                            );
                        }
                    }
                }
            }

            // Shutdown signal
            _ = SHUTDOWN_NOTIFY.notified() => {
                log(LogTag::Transactions, "INFO", "Received shutdown signal");
                return Ok(());
            }

            // General service health check (every 5 minutes)
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
    let pending_txs_count = get_pending_transactions_count().await;
    let (pending_verification_count, _) = crate::positions::queue::get_queue_status().await;

    log(
        LogTag::Transactions,
        "FALLBACK",
        &format!(
            "Performing fallback transaction check (WebSocket inactive) - Pending txs: {}, Pending verifications: {}",
            pending_txs_count,
            pending_verification_count
        )
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
                        &format!("Failed to process fallback transaction {}: {}", &signature, e)
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
    // Determine WS URL: prefer Helius if API key is present in config; else default
    let ws_url = {
        let rpc_urls = crate::config::with_config(|cfg| cfg.rpc.urls.clone());

        // Try to find a Helius API key in the configured RPC URLs
        let mut api_key: Option<String> = None;
        for url in rpc_urls.iter() {
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
        &format!("Processing WebSocket transaction: {}", &signature)
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
                &format!("Successfully processed WebSocket transaction: {}", &signature)
            );
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to process WebSocket transaction {}: {}", &signature, e)
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
    service_start_time: Option<DateTime<Utc>>,
    last_periodic_check: Option<DateTime<Utc>>,
    last_websocket_activity: Option<DateTime<Utc>>,
    last_websocket_message: Option<DateTime<Utc>>, // Track ANY WebSocket message (pings, pongs, etc)
    last_websocket_health_check: Option<DateTime<Utc>>,
    websocket_reconnect_count: u64,
    periodic_check_count: u64,
    websocket_transaction_count: u64,
    error_count: u64,
    average_check_duration_ms: f64,
}

impl ServiceMetrics {
    fn new() -> Self {
        Self {
            service_start_time: Some(Utc::now()),
            last_periodic_check: None,
            last_websocket_activity: None,
            last_websocket_message: None,
            last_websocket_health_check: None,
            websocket_reconnect_count: 0,
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
        let now = Utc::now();
        self.last_websocket_activity = Some(now);
        self.last_websocket_message = Some(now);
        self.websocket_transaction_count += 1;
    }

    fn update_websocket_message(&mut self) {
        self.last_websocket_message = Some(Utc::now());
    }

    fn update_websocket_health_check(&mut self) {
        self.last_websocket_health_check = Some(Utc::now());
    }

    fn increment_websocket_reconnect(&mut self) {
        self.websocket_reconnect_count += 1;
    }

    fn is_websocket_healthy(&self) -> bool {
        // WebSocket health is determined by whether the receiver channel is still open
        // If the WebSocket loop crashes or connection fails, the channel closes
        // This method is called only when receiver exists, so if we get here, it's healthy
        //
        // We use a conservative timeout (5 minutes) for transaction message silence
        // to avoid false positives during idle periods when no transactions occur
        if let Some(last_msg) = self.last_websocket_message {
            let silence_duration = (Utc::now() - last_msg).num_seconds();
            silence_duration < 300 // 5 minutes instead of 30 seconds
        } else {
            // No transaction messages yet - connection is healthy if recently initialized
            if let Some(activity) = self.last_websocket_activity {
                let since_activity = (Utc::now() - activity).num_seconds();
                since_activity < 300 // 5 minutes grace period
            } else {
                // No activity at all yet - assume healthy
                true
            }
        }
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
///
/// Fallback is only performed when:
/// 1. There are pending transactions OR pending position verifications to monitor
/// 2. WebSocket has been silent for >10 seconds (or never connected with >10s uptime)
async fn should_perform_fallback_check(metrics: &ServiceMetrics) -> bool {
    // Step 1: Check if there's anything to monitor
    let has_pending_txs = get_pending_transactions_count().await > 0;

    // Check positions verification queue for pending work
    let (pending_verification_count, _) = crate::positions::queue::get_queue_status().await;
    let has_pending_verifications = pending_verification_count > 0;

    if !has_pending_txs && !has_pending_verifications {
        // Idle state - no need for fallback
        if is_debug_transactions_enabled() {
            log(
                LogTag::Transactions,
                "DEBUG",
                "Skipping fallback: no pending transactions or verifications (idle state)"
            );
        }
        return false;
    }

    // Step 2: Check WebSocket activity with grace period
    if let Some(last_activity) = metrics.last_websocket_activity {
        let silence_duration = (Utc::now() - last_activity).num_seconds();

        // Only fallback if WS has been silent for >10 seconds
        if silence_duration > 10 {
            if is_debug_transactions_enabled() {
                log(
                    LogTag::Transactions,
                    "DEBUG",
                    &format!(
                        "Fallback triggered: WebSocket silent for {}s (pending txs: {}, pending verifications: {})",
                        silence_duration,
                        has_pending_txs,
                        has_pending_verifications
                    )
                );
            }
            true
        } else {
            false
        }
    } else {
        // WebSocket never connected or failed
        // Only fallback if we've been running for >10 seconds (startup grace)
        if let Some(service_start) = metrics.service_start_time {
            let uptime = (Utc::now() - service_start).num_seconds();
            if uptime > 10 {
                if is_debug_transactions_enabled() {
                    log(
                        LogTag::Transactions,
                        "DEBUG",
                        &format!(
                            "Fallback triggered: WebSocket never connected, uptime {}s (pending txs: {}, pending verifications: {})",
                            uptime,
                            has_pending_txs,
                            has_pending_verifications
                        )
                    );
                }
                true
            } else {
                if is_debug_transactions_enabled() {
                    log(
                        LogTag::Transactions,
                        "DEBUG",
                        &format!("Waiting for WebSocket to connect (uptime: {}s)", uptime)
                    );
                }
                false
            }
        } else {
            // Just started, give WS time to connect
            false
        }
    }
}
