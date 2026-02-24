// Startup bootstrap logic - initial transaction history loading

use chrono::Utc;
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

use crate::logger::{self, LogTag};
use crate::transactions::{
    fetcher::TransactionFetcher,
    manager::TransactionsManager,
    processor::TransactionProcessor,
    utils::{add_signature_to_known_globally, is_signature_known_globally, RPC_BATCH_SIZE},
};

use super::config::{
    CONCURRENT_BATCH_SIZE, MAX_RETRY_ATTEMPTS, RETRY_BASE_DELAY_SECS, TRANSACTION_TIMEOUT_SECS,
};

// =============================================================================
// STARTUP BOOTSTRAP
// =============================================================================

/// Statistics describing the initial bootstrap process
#[derive(Debug, Default)]
pub struct BootstrapStats {
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
pub async fn perform_initial_transaction_bootstrap(
    manager_arc: &Arc<Mutex<TransactionsManager>>,
) -> Result<BootstrapStats, String> {
    let bootstrap_timer = std::time::Instant::now();
    let (wallet_pubkey, debug, transaction_db) = {
        let mgr = manager_arc.lock().await;
        (
            mgr.wallet_pubkey,
            mgr.debug_enabled,
            mgr.transaction_database.clone(),
        )
    };
    let fetcher = TransactionFetcher::new();
    let processor = Arc::new(TransactionProcessor::new(wallet_pubkey));

    let mut stats = BootstrapStats::default();
    let batch_limit = RPC_BATCH_SIZE;

    // Reconcile known_signatures from already processed at start (safety)
    if let Some(db) = transaction_db.as_ref() {
        if let Ok(added) = db.reconcile_known_with_processed().await {
            if added > 0 {
                logger::info(
                    LogTag::Transactions,
                    &format!("Reconciled {} processed->known signatures", added),
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
                logger::info(
                    LogTag::Transactions,
                    &format!("Failed to load bootstrap state: {}", e),
                );
                // Fallback to default behavior (FULL with checkpoint on oldest-known if any)
                checkpoint_signature = None;
            }
        }
    }

    logger::info(
        LogTag::Transactions,
        &format!(
            "Bootstrapping transaction cache for wallet: {} (mode={}, batch_limit={})",
            &wallet_pubkey.to_string(),
            &bootstrap_mode,
            batch_limit
        ),
    );

    if let Some(ref checkpoint) = checkpoint_signature {
        logger::info(
            LogTag::Transactions,
            &format!("Checkpoint: {}...", &checkpoint[..8]),
        );
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
    logger::info(
        LogTag::Transactions,
        &format!("Phase 1: Collecting {} signatures...", phase1_label),
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
        let signatures = fetcher
            .fetch_signatures_page(wallet_pubkey, batch_limit, before.as_deref())
            .await?;

        if signatures.is_empty() {
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
                    logger::info(
                        LogTag::Transactions,
                        &format!(
                            "Hit checkpoint signature - stopping at page {}",
                            stats.total_rpc_pages
                        ),
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
                if signatures.len() < batch_limit { /* done anyway */ }
                break;
            } else {
                all_signatures.extend(signatures.clone());
            }
        }

        if stats.newest_signature.is_none() {
            stats.newest_signature = signatures.first().cloned();
        }
        stats.oldest_signature = signatures.last().cloned();

        logger::info(
            LogTag::Transactions,
            &format!(
                "Fetched page {}: {} signatures | total collected: {} | elapsed: {}s",
                stats.total_rpc_pages,
                page_count,
                all_signatures.len(),
                phase1_timer.elapsed().as_secs()
            ),
        );

        before = signatures.last().cloned();

        // Persist backfill cursor every page (only in FULL/backfill mode)
        if bootstrap_mode == "FULL" {
            if let Some(db) = transaction_db.as_ref() {
                if let Err(e) = db.set_backfill_cursor(before.as_deref()).await {
                    logger::info(
                        LogTag::Transactions,
                        &format!("Failed to persist backfill cursor: {}", e),
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
            "Phase 1 complete (INCREMENTAL): collected {} signatures in {}s across {} pages",
            all_signatures.len(),
            phase1_timer.elapsed().as_secs(),
            stats.total_rpc_pages
        )
    } else if hit_checkpoint {
        format!(
 "Phase 1 complete (INCREMENTAL): collected {} NEW signatures in {}s across {} pages (stopped at checkpoint)",
      all_signatures.len(),
      phase1_timer.elapsed().as_secs(),
      stats.total_rpc_pages
    )
    } else if checkpoint_signature.is_some() {
        format!(
 "Phase 1 complete (INCREMENTAL): collected {} signatures in {}s across {} pages (checkpoint not found - fetched all)",
      all_signatures.len(),
      phase1_timer.elapsed().as_secs(),
      stats.total_rpc_pages
    )
    } else {
        format!(
            "Phase 1 complete (FULL): collected {} signatures in {}s across {} pages",
            all_signatures.len(),
            phase1_timer.elapsed().as_secs(),
            stats.total_rpc_pages
        )
    };

    logger::info(LogTag::Transactions, &phase1_summary);

    // =========================================================================
    // PHASE 2: FILTER AND PROCESS NEW SIGNATURES
    // =========================================================================
    logger::info(
        LogTag::Transactions,
        "Phase 2: Filtering and processing new transactions...",
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
                        logger::warning(
                            LogTag::Transactions,
                            &format!("Failed to query known status for {}: {}", signature, e),
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
    logger::info(
        LogTag::Transactions,
        &format!(
 "Filtering complete: {} new to process | {} already known | batch_size={} | elapsed: {}s",
      total_to_process,
      stats.known_signatures_skipped,
      CONCURRENT_BATCH_SIZE,
      phase2_timer.elapsed().as_secs()
    ),
    );

    // Process all new signatures in parallel batches with accurate progress tracking
    let mut processed_count = 0;
    let mut newly_processed = 0;
    let mut errors = 0;
    let mut failed_signatures: Vec<(String, String)> = Vec::new(); // (signature, error_reason)

    // Split into batches and process in parallel
    for batch_start in (0..signatures_to_process.len()).step_by(CONCURRENT_BATCH_SIZE) {
        let batch_end = (batch_start + CONCURRENT_BATCH_SIZE).min(signatures_to_process.len());
        let batch = &signatures_to_process[batch_start..batch_end];

        // Create futures for parallel processing with timeout
        let mut futures = FuturesUnordered::new();

        for signature in batch {
            let sig = signature.clone();
            let proc = processor.clone();
            futures.push(async move {
                let result = timeout(
                    Duration::from_secs(TRANSACTION_TIMEOUT_SECS),
                    proc.process_transaction(&sig),
                )
                .await;

                match result {
                    Ok(inner_result) => (sig.clone(), inner_result),
                    Err(_) => (
                        sig.clone(),
                        Err(format!(
                            "Transaction processing timed out after {}s",
                            TRANSACTION_TIMEOUT_SECS
                        )),
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
                            logger::info(
                                LogTag::Transactions,
                                &format!("Failed to persist known signature {}: {}", signature, e),
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
                    logger::info(
                        LogTag::Transactions,
                        &format!(
                            "Failed to process bootstrap transaction {}: {} (will retry)",
                            signature, e
                        ),
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

                logger::info(
                    LogTag::Transactions,
                    &format!(
 "Progress: {}/{} ({:.1}%) | new={} | errors={} | remaining={} | elapsed={}s",
            processed_count,
            total_to_process,
            progress_pct,
            newly_processed,
            errors,
            remaining,
            bootstrap_timer.elapsed().as_secs()
          ),
                );
            }
        }
    }

    logger::info(
        LogTag::Transactions,
        &format!(
            "Phase 2 complete: processed {}/{} new transactions | errors={} | elapsed={}s",
            newly_processed,
            total_to_process,
            errors,
            phase2_timer.elapsed().as_secs()
        ),
    );

    // =========================================================================
    // PHASE 3: RETRY FAILED TRANSACTIONS
    // =========================================================================
    let mut retry_successful = 0;
    let mut permanently_failed = 0;

    if !failed_signatures.is_empty() {
        logger::info(
            LogTag::Transactions,
            &format!(
                "Phase 3: Retrying {} failed transactions (max {} attempts per transaction)",
                failed_signatures.len(),
                MAX_RETRY_ATTEMPTS
            ),
        );

        for attempt in 1..=MAX_RETRY_ATTEMPTS {
            if failed_signatures.is_empty() {
                break;
            }

            let delay_secs = RETRY_BASE_DELAY_SECS * (2_u64).pow((attempt as u32) - 1);

            logger::info(
                LogTag::Transactions,
                &format!(
                    "Retry attempt {}/{}: {} signatures remaining | delay={}s",
                    attempt,
                    MAX_RETRY_ATTEMPTS,
                    failed_signatures.len(),
                    delay_secs
                ),
            );

            // Wait before retrying (exponential backoff)
            if attempt > 1 {
                sleep(Duration::from_secs(delay_secs)).await;
            }

            let mut still_failed: Vec<(String, String)> = Vec::new();

            // Process failed signatures in batches
            for batch_start in (0..failed_signatures.len()).step_by(CONCURRENT_BATCH_SIZE) {
                let batch_end =
                    (batch_start + CONCURRENT_BATCH_SIZE).min(failed_signatures.len());
                let batch = &failed_signatures[batch_start..batch_end];

                let mut futures = FuturesUnordered::new();

                for (signature, _) in batch {
                    let sig = signature.clone();
                    let proc = processor.clone();
                    futures.push(async move {
                        let result = timeout(
                            Duration::from_secs(TRANSACTION_TIMEOUT_SECS),
                            proc.process_transaction(&sig),
                        )
                        .await;

                        match result {
                            Ok(inner_result) => (sig.clone(), inner_result),
                            Err(_) => (
                                sig.clone(),
                                Err(format!(
                                    "Transaction processing timed out after {}s",
                                    TRANSACTION_TIMEOUT_SECS
                                )),
                            ),
                        }
                    });
                }

                while let Some((signature, result)) = futures.next().await {
                    match result {
                        Ok(_) => {
                            if let Some(db) = transaction_db.as_ref() {
                                if let Err(e) = db.add_known_signature(&signature).await {
                                    logger::info(
                                        LogTag::Transactions,
                                        &format!(
                                            "Failed to persist known signature {}: {}",
                                            signature, e
                                        ),
                                    );
                                }
                            }

                            add_signature_to_known_globally(signature.clone()).await;
                            if let Ok(mut mgr) = manager_arc.try_lock() {
                                mgr.known_signatures.insert(signature.clone());
                                mgr.total_transactions += 1;
                                // Update metrics
                                mgr.operations
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                mgr.bootstrap_fetched
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                            retry_successful += 1;
                        }
                        Err(e) => {
                            // Update error metrics
                            if let Ok(mgr) = manager_arc.try_lock() {
                                mgr.errors
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                            still_failed.push((signature.clone(), e));
                        }
                    }
                }
            }

            failed_signatures = still_failed;

            logger::info(
                LogTag::Transactions,
                &format!(
                    "Retry attempt {} complete: {} recovered | {} still failing",
                    attempt,
                    retry_successful,
                    failed_signatures.len()
                ),
            );
        }

        permanently_failed = failed_signatures.len();

        if permanently_failed > 0 {
            logger::info(
                LogTag::Transactions,
                &format!(
                    "{} transactions permanently failed after {} retry attempts",
                    permanently_failed, MAX_RETRY_ATTEMPTS
                ),
            );
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
                logger::info(
                    LogTag::Transactions,
                    &format!("Failed to refresh known signatures count: {}", e),
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
                    logger::info(
                        LogTag::Transactions,
                        &format!("Failed to mark full history completed: {}", e),
                    );
                } else {
                    // Clear cursor since we reached chain end
                    if let Err(e) = db.clear_backfill_cursor().await {
                        logger::info(
                            LogTag::Transactions,
                            &format!("Failed to clear backfill cursor: {}", e),
                        );
                    }
                    logger::info(
            LogTag::Transactions,
            "Full history backfill completed. Switching to forward incremental on next start."
          );
                }
            } else {
                // Persist last cursor already done per-page; ensure it's saved for resume
                if let Err(e) = db.set_backfill_cursor(before.as_deref()).await {
                    logger::info(
                        LogTag::Transactions,
                        &format!("Failed to persist final backfill cursor: {}", e),
                    );
                }
            }
        }
    }

    // Final summary
    let summary = if retry_successful > 0 || permanently_failed > 0 {
        format!(
            "Bootstrap complete!\n\
      ═══════════════════════════════════════════════════════════════\n\
       Total signatures found: {}\n\
       New transactions processed: {} (initial: {} + retried: {})\n\
       Already known (skipped): {}\n\
       Retry statistics: {} recovered | {} permanently failed\n\
       RPC pages fetched: {}\n\
       Total time: {:.1}s\n\
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
            "Bootstrap complete!\n\
      ═══════════════════════════════════════════════════════════════\n\
       Total signatures found: {}\n\
       New transactions processed: {}\n\
       Already known (skipped): {}\n\
       Errors: {}\n\
       RPC pages fetched: {}\n\
       Total time: {:.1}s\n\
      ═══════════════════════════════════════════════════════════════",
            stats.total_signatures_fetched,
            stats.newly_processed,
            stats.known_signatures_skipped,
            stats.errors,
            stats.total_rpc_pages,
            bootstrap_timer.elapsed().as_secs_f64()
        )
    };

    logger::info(LogTag::Transactions, &summary);

    Ok(stats)
}
