// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Transaction service bootstrap — initialization and startup of the transaction monitoring service.
//
// Startup bootstrap logic - initial transaction history loading

use futures::stream::{FuturesUnordered, StreamExt};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

use crate::chains::solana::transactions::{
    fetcher::TransactionFetcher, processor::TransactionProcessor,
};
use crate::logger::{self, LogTag};
use crate::transactions::{
    database::TransactionDatabase,
    error::Error,
    manager::TransactionsManager,
    subject::Subject,
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
) -> Result<BootstrapStats, Error> {
    // Held for the whole load: an on-demand load of older history waits for it
    // instead of moving the cursor this load is about to write.
    let _history_load = OLDER_HISTORY_LOAD.lock().await;
    let bootstrap_timer = std::time::Instant::now();
    let (subject, _debug, transaction_db) = {
        let mgr = manager_arc.lock().await;
        (
            mgr.subject().clone(),
            mgr.debug_enabled,
            mgr.transaction_database.clone(),
        )
    };
    let wallet_pubkey = crate::chains::solana::transactions::subject::try_to_pubkey(&subject)
        .map_err(|e| Error::WalletUnavailable {
            detail: e.to_string(),
        })?;
    let fetcher = TransactionFetcher::new();
    let processor = Arc::new(TransactionProcessor::for_subject(&subject).map_err(|e| {
        Error::WalletUnavailable {
            detail: e.to_string(),
        }
    })?);

    let mut stats = BootstrapStats::default();
    let (history_cap, batch_limit) = signature_window(crate::chains::access::active_has(
        crate::chains::access::Capability::FullHistory,
    ));

    // Reconcile known_signatures from already processed at start (safety)
    if let Some(db) = transaction_db.as_ref() {
        if let Ok(added) = db.reconcile_known_with_processed().await {
            if added > 0 {
                logger::info(
                    LogTag::Transactions,
                    &format!("Reconciled {added} processed->known signatures"),
                );
            }
        }
    }

    // Determine bootstrap mode using persistent bootstrap_state
    // Backfill mode: full history not completed yet → page newest→oldest from the newest signature
    // Forward incremental: once full history completed → fetch only newer than newest known
    let mut bootstrap_mode = "FULL".to_owned();
    let mut checkpoint_signature: Option<String> = None;
    if let Some(db) = transaction_db.as_ref() {
        match db.get_bootstrap_state().await {
            Ok(state) => {
                if state.full_history_completed {
                    // Forward incremental mode
                    bootstrap_mode = "INCREMENTAL".to_owned();
                    checkpoint_signature = db
                        .get_newest_known_signature(subject.clone())
                        .await
                        .unwrap_or(None);
                } else {
                    // Backfill mode not completed
                    bootstrap_mode = "FULL".to_owned();
                    // In FULL/backfill mode we do NOT stop at oldest-known; we continue until chain end.
                    checkpoint_signature = None;
                }
            }
            Err(e) => {
                logger::info(
                    LogTag::Transactions,
                    &format!("Failed to load bootstrap state: {e}"),
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
            &subject.address(),
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
        &format!("Phase 1: Collecting {phase1_label} signatures..."),
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
            .fetch_signatures_page(wallet_pubkey, batch_limit, before.as_deref(), None)
            .await
            .map_err(|e| Error::Bootstrap {
                detail: e.to_string(),
            })?;

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

        if signatures.len() < batch_limit {
            if bootstrap_mode == "FULL" {
                reached_chain_end = true;
            }
            break;
        }

        if history_cap.is_some_and(|cap| all_signatures.len() >= cap) {
            logger::info(
                LogTag::Transactions,
                &format!(
                    "Phase 1: stopped at the newest {} signatures; older history loads on demand",
                    all_signatures.len()
                ),
            );
            break;
        }
    }

    stats.total_signatures_fetched = all_signatures.len();

    // A FULL walk either reached the chain's end or stopped at the history cap; older
    // history then continues on demand before the oldest signature collected.
    if bootstrap_mode == "FULL" {
        let cursor = if reached_chain_end {
            None
        } else {
            stats.oldest_signature.as_deref()
        };
        if let Some(db) = transaction_db.as_ref() {
            if let Err(e) = db.set_history_cursor(cursor).await {
                logger::warning(
                    LogTag::Transactions,
                    &format!("Failed to record the older-history cursor: {e}"),
                );
            }
        }
    }

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
        let mut signature_is_known = is_signature_known_globally(subject.clone(), signature).await;

        if !signature_is_known {
            if let Some(db) = transaction_db.as_ref() {
                match db.is_signature_known(subject.clone(), signature).await {
                    Ok(true) => {
                        signature_is_known = true;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        logger::warning(
                            LogTag::Transactions,
                            &format!("Failed to query known status for {signature}: {e}"),
                        );
                        stats.errors += 1;
                    }
                }
            }
        }

        if signature_is_known {
            stats.known_signatures_skipped += 1;
            add_signature_to_known_globally(subject.clone(), signature.clone()).await;
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
    let mut failed_signatures: Vec<(String, crate::chains::solana::Error)> = Vec::new(); // (signature, error_reason)

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
                        Err(crate::chains::solana::Error::Execution(
                            crate::chains::ExecutionFailure::ConfirmationTimeout {
                                reference: sig.clone(),
                                waited_ms: TRANSACTION_TIMEOUT_SECS * 1000,
                            },
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
                        if let Err(e) = db.add_known_signature(subject.clone(), &signature).await {
                            logger::info(
                                LogTag::Transactions,
                                &format!("Failed to persist known signature {signature}: {e}"),
                            );
                            errors += 1;
                        }
                    }

                    add_signature_to_known_globally(subject.clone(), signature.clone()).await;
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

            let mut still_failed: Vec<(String, crate::chains::solana::Error)> = Vec::new();

            // Process failed signatures in batches
            for batch_start in (0..failed_signatures.len()).step_by(CONCURRENT_BATCH_SIZE) {
                let batch_end = (batch_start + CONCURRENT_BATCH_SIZE).min(failed_signatures.len());
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
                                Err(crate::chains::solana::Error::Execution(
                                    crate::chains::ExecutionFailure::ConfirmationTimeout {
                                        reference: sig.clone(),
                                        waited_ms: TRANSACTION_TIMEOUT_SECS * 1000,
                                    },
                                )),
                            ),
                        }
                    });
                }

                while let Some((signature, result)) = futures.next().await {
                    match result {
                        Ok(_) => {
                            if let Some(db) = transaction_db.as_ref() {
                                if let Err(e) =
                                    db.add_known_signature(subject.clone(), &signature).await
                                {
                                    logger::info(
                                        LogTag::Transactions,
                                        &format!(
                                            "Failed to persist known signature {}: {}",
                                            signature, e
                                        ),
                                    );
                                }
                            }

                            add_signature_to_known_globally(subject.clone(), signature.clone())
                                .await;
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
        match db.get_known_signatures_count(subject.clone()).await {
            Ok(count) => {
                if let Ok(mut mgr) = manager_arc.try_lock() {
                    mgr.total_transactions = count;
                }
            }
            Err(e) => {
                logger::info(
                    LogTag::Transactions,
                    &format!("Failed to refresh known signatures count: {e}"),
                );
                stats.errors += 1;
            }
        }
    }

    stats.duration_ms = bootstrap_timer.elapsed().as_millis();

    // If FULL/backfill completed to chain-end (no more pages), mark full history completed so next run uses forward-only incremental
    if bootstrap_mode == "FULL" && reached_chain_end {
        if let Some(db) = transaction_db.as_ref() {
            if let Err(e) = db.mark_full_history_completed().await {
                logger::info(
                    LogTag::Transactions,
                    &format!("Failed to mark full history completed: {e}"),
                );
            } else {
                logger::info(
                    LogTag::Transactions,
                    "Full history backfill completed. Switching to forward incremental on next start.",
                );
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

    // Own wallet only: rebuild subject_asset_deltas from the full raw_transactions
    // history once per SUBJECT_DELTAS_VERSION. The live per-transaction hook only
    // covers transactions processed from now on, so an upgrade would otherwise
    // leave this table (and everything wallet-history positions are built from)
    // permanently empty. Never fails bootstrap.
    if let Some(db) = transaction_db.as_ref() {
        match db.backfill_subject_deltas(&subject.address()).await {
            Ok(written) if written > 0 => {
                logger::info(
                    LogTag::Transactions,
                    &format!("Subject deltas backfill wrote {written} rows"),
                );
            }
            Ok(_) => {}
            Err(e) => {
                logger::warning(
                    LogTag::Transactions,
                    &format!("Subject deltas backfill failed: {e}"),
                );
            }
        }
    }

    sync_history_positions(transaction_db.as_deref(), &subject).await;

    Ok(stats)
}

/// Repairs subject-delta gaps and derives the wallet's position rounds from the delta table.
/// The full backfill runs once per delta version and cannot fix a gap that opens afterwards,
/// so the gap fill runs after every history load. The position sync is what makes a fresh
/// install show the tokens the wallet already holds as positions. Best-effort by design: a
/// failure leaves the trader and every existing position untouched, and the next load retries.
async fn sync_history_positions(db: Option<&TransactionDatabase>, subject: &Subject) {
    if let Some(db) = db {
        if let Err(e) = db.fill_subject_delta_gaps(&subject.address()).await {
            crate::transactions::database::note_subject_delta_gap();
            logger::warning(
                LogTag::Transactions,
                &format!("Subject deltas gap fill failed: {e}"),
            );
        }
    }

    match crate::positions::ledger::sync_wallet_history().await {
        Ok(summary) if summary.inserted > 0 || summary.updated > 0 => {
            logger::info(
                LogTag::Transactions,
                &format!(
                    "Wallet-history positions: {} rounds ({} new, {} updated)",
                    summary.rounds, summary.inserted, summary.updated
                ),
            );
        }
        Ok(_) => {}
        Err(e) => {
            logger::warning(
                LogTag::Transactions,
                &format!("Wallet-history position sync failed: {e}"),
            );
        }
    }
}

/// One page of older history loaded on demand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OlderHistoryPage {
    /// Signatures the page read from the chain
    pub fetched: usize,
    /// Transactions newly processed from the page
    pub processed: usize,
    /// Transactions that failed to process; the cursor stays put so the next load retries them
    pub failed: usize,
    /// Whether history older than the stored signatures can still be loaded
    pub older_history_available: bool,
}

/// Serializes the startup history load and on-demand history loads, so one page is
/// never processed twice at once and the older-history cursor has one writer.
static OLDER_HISTORY_LOAD: Mutex<()> = Mutex::const_new(());

/// Loads the page of own-wallet history that precedes the stored history cursor, processes
/// its unknown transactions and moves the cursor past it. The cursor advances only when every
/// transaction of the page was processed; reaching the chain's end marks the full history
/// completed and clears the cursor.
pub async fn load_older_history() -> Result<OlderHistoryPage, Error> {
    let _load = OLDER_HISTORY_LOAD
        .try_lock()
        .map_err(|_| Error::HistoryLoadInProgress)?;
    let manager_arc = super::lifecycle::get_global_transaction_manager()
        .await
        .ok_or(Error::NotInitialised)?;
    let subject = manager_arc.lock().await.subject().clone();
    let db = crate::transactions::database::get_transaction_database()
        .await
        .ok_or(Error::NotInitialised)?;

    let state = db.get_bootstrap_state().await?;
    let Some(before) = state
        .older_history_available()
        .then_some(state.history_before)
        .flatten()
    else {
        return Ok(OlderHistoryPage {
            fetched: 0,
            processed: 0,
            failed: 0,
            older_history_available: false,
        });
    };

    let wallet_pubkey = crate::chains::solana::transactions::subject::try_to_pubkey(&subject)
        .map_err(|e| Error::WalletUnavailable {
            detail: e.to_string(),
        })?;
    let processor =
        TransactionProcessor::for_subject(&subject).map_err(|e| Error::WalletUnavailable {
            detail: e.to_string(),
        })?;
    let (_, page_size) = signature_window(crate::chains::access::active_has(
        crate::chains::access::Capability::FullHistory,
    ));
    let signatures = TransactionFetcher::new()
        .fetch_signatures_page(wallet_pubkey, page_size, Some(before.as_str()), None)
        .await
        .map_err(|e| Error::Bootstrap {
            detail: e.to_string(),
        })?;

    let mut processed = 0;
    let mut failed = 0;
    for signature in &signatures {
        if is_signature_known_globally(subject.clone(), signature).await
            || db.is_signature_known(subject.clone(), signature).await?
        {
            continue;
        }
        let outcome = timeout(
            Duration::from_secs(TRANSACTION_TIMEOUT_SECS),
            processor.process_transaction(signature),
        )
        .await;
        match outcome {
            Ok(Ok(_)) => {
                db.add_known_signature(subject.clone(), signature).await?;
                add_signature_to_known_globally(subject.clone(), signature.clone()).await;
                processed += 1;
            }
            Ok(Err(e)) => {
                logger::warning(
                    LogTag::Transactions,
                    &format!("Older-history transaction {signature} failed to process: {e}"),
                );
                failed += 1;
            }
            Err(_) => {
                logger::warning(
                    LogTag::Transactions,
                    &format!(
                        "Older-history transaction {signature} timed out after {TRANSACTION_TIMEOUT_SECS}s"
                    ),
                );
                failed += 1;
            }
        }
    }

    let reached_chain_end = signatures.len() < page_size;
    if failed == 0 {
        if reached_chain_end {
            db.mark_full_history_completed().await?;
            db.set_history_cursor(None).await?;
        } else if let Some(oldest) = signatures.last() {
            db.set_history_cursor(Some(oldest)).await?;
        }
    }
    if processed > 0 {
        sync_history_positions(Some(&db), &subject).await;
    }

    logger::info(
        LogTag::Transactions,
        &format!(
            "Older history: fetched {} signatures, processed {processed}, failed {failed}",
            signatures.len()
        ),
    );
    Ok(OlderHistoryPage {
        fetched: signatures.len(),
        processed,
        failed,
        older_history_available: failed > 0 || !reached_chain_end,
    })
}

/// How many signatures a bootstrap collects at most, and its page size.
/// Without full history the bootstrap reads only the newest signatures, in a
/// page no larger than that cap, so a full page never reads as the end of the
/// chain; older history loads on demand.
fn signature_window(full_history: bool) -> (Option<usize>, usize) {
    if full_history {
        (None, RPC_BATCH_SIZE)
    } else {
        let cap = crate::chains::solana::access::SERVICE_HISTORY_SIGNATURES;
        (Some(cap), cap.min(RPC_BATCH_SIZE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_full_history_the_bootstrap_stops_at_the_newest_two_hundred() {
        assert_eq!(signature_window(false), (Some(200), 200));
        assert_eq!(signature_window(true), (None, RPC_BATCH_SIZE));
    }
}
