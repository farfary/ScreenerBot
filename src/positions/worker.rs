// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position worker — background task that processes position lifecycle events.

use super::db::initialize_positions_database;
use super::{
    apply::apply_transition,
    queue::{
        abandon_verification, assign_expiry_bound, clear_not_landed, defer_settlement,
        enqueue_verification, mark_swap_confirmed, poll_verification_batch, remove_verification,
        requeue_verification, settlement_candidates, unbounded_signatures, VerificationItem,
    },
    settle::{self, Disposition},
    state::{
        reconcile_global_position_semaphore, rehydrate_pending_dca_swaps, MINT_TO_POSITION_INDEX,
        POSITIONS, SIG_TO_MINT_INDEX,
    },
    types::{ApplyFailureDisposition, GiveUpReason, VerificationKind, VerificationOutcome},
    verifier::verify_transaction,
};
use crate::chains::SignatureVerdict;
use crate::errors::ErrorClass;
use crate::logger::{self, LogTag};
use crate::positions::{Error, Result};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::sync::Arc;
use tokio::{
    sync::Notify,
    time::{sleep, Duration},
};

const VERIFICATION_BATCH_SIZE: usize = 10;

/// How long a settlement waits for the mint's position lock. A DCA or partial exit holds the
/// lock across its swap and its pending registration, so a settlement that cannot take it
/// in time is deferred to a later read instead of stalling the worker.
const SETTLEMENT_LOCK_WAIT: Duration = Duration::from_millis(500);

/// Initialize positions system
pub async fn initialize_positions_system() -> Result<()> {
    logger::info(LogTag::Positions, "Initializing positions system");

    // Initialize database
    initialize_positions_database().await?;

    // Load existing positions from database
    match crate::positions::load_all_positions().await {
        Ok(positions) => {
            let mut global_positions = POSITIONS.write().await;
            let mut sig_to_mint_index = SIG_TO_MINT_INDEX.write().await;
            let mut mint_to_position_index = MINT_TO_POSITION_INDEX.write().await;

            let mut unverified_count = 0;

            // Process each position
            for position in &positions {
                // Add unverified entry transactions to queue
                if !position.transaction_entry_verified {
                    if let Some(entry_sig) = &position.entry_transaction_signature {
                        let item = VerificationItem::new(
                            entry_sig.clone(),
                            position.mint.clone(),
                            position.id,
                            VerificationKind::Entry,
                            None,
                        );
                        enqueue_verification(item).await;
                        unverified_count += 1;
                    }
                }

                // Add unverified exit transactions to queue
                if !position.transaction_exit_verified {
                    if let Some(exit_sig) = &position.exit_transaction_signature {
                        let item = VerificationItem::new(
                            exit_sig.clone(),
                            position.mint.clone(),
                            position.id,
                            VerificationKind::Exit,
                            None,
                        );
                        enqueue_verification(item).await;
                        unverified_count += 1;
                    }
                }

                // NOTE: pending PARTIAL exits are NOT recovered from the position here.
                // They are rehydrated below from the durable PENDING_PARTIAL_EXIT_DETAILS
                // metadata, which rebuilds both the counter and the verification items.
                //
                // Inferring them from `exit_transaction_signature` (as this used to) was
                // only possible because partial_close wrongly stamped its signature there,
                // and it made the block above enqueue that same partial signature as a
                // FULL-exit verification — closing the position and releasing its permit
                // while the remaining tokens were still in the wallet, on every restart.
            }

            // Populate state
            *global_positions = positions;

            // Rebuild indexes
            sig_to_mint_index.clear();
            mint_to_position_index.clear();

            for (index, position) in global_positions.iter().enumerate() {
                // Signature indexes
                if let Some(ref entry_sig) = position.entry_transaction_signature {
                    sig_to_mint_index.insert(entry_sig.clone(), position.mint.clone());
                }
                if let Some(ref exit_sig) = position.exit_transaction_signature {
                    sig_to_mint_index.insert(exit_sig.clone(), position.mint.clone());
                }

                // Position index
                mint_to_position_index.insert(position.mint.clone(), index);
            }

            logger::info(
                LogTag::Positions,
                &format!(
                    "Loaded {} positions, {} pending verification",
                    global_positions.len(),
                    unverified_count
                ),
            );
        }
        Err(e) => {
            logger::warning(
                LogTag::Positions,
                &format!("Failed to load positions from database: {e}"),
            );
        }
    }

    match rehydrate_pending_dca_swaps().await {
        Ok(pending) => {
            if !pending.is_empty() {
                let mut restored = 0;
                for entry in pending {
                    let item = VerificationItem::new_dca(
                        entry.signature.clone(),
                        entry.mint.clone(),
                        Some(entry.position_id),
                        entry.expiry_height,
                    );
                    enqueue_verification(item).await;
                    restored += 1;
                }

                logger::info(
                    LogTag::Positions,
                    &format!(
                        "Restored {} pending DCA verifications from metadata",
                        restored
                    ),
                );
            }
        }
        Err(err) => {
            logger::error(
                LogTag::Positions,
                &format!("Failed to rehydrate pending DCA swaps: {err}"),
            );
        }
    }

    match super::state::rehydrate_pending_partial_exits().await {
        Ok(pending) => {
            if !pending.is_empty() {
                let mut restored = 0;
                for entry in pending {
                    let item = VerificationItem::new_partial_exit(
                        entry.signature.clone(),
                        entry.mint.clone(),
                        Some(entry.position_id),
                        entry.expected_exit_amount,
                        entry.requested_exit_percentage,
                        entry.expiry_height,
                    );
                    enqueue_verification(item).await;
                    restored += 1;
                }

                logger::info(
                    LogTag::Positions,
                    &format!(
                        "Restored {} pending partial exit verifications from metadata",
                        restored
                    ),
                );
            }
        }
        Err(err) => {
            logger::error(
                LogTag::Positions,
                &format!("Failed to rehydrate pending partial exits: {err}"),
            );
        }
    }

    // Initialize global position semaphore and reconcile with existing open positions
    {
        let max_open_positions = crate::config::with_config(|cfg| cfg.trader.max_open_positions);

        // Initialize the semaphore with configured capacity
        crate::positions::init_global_position_semaphore(max_open_positions);

        // Reconcile semaphore capacity with existing open positions
        reconcile_global_position_semaphore(max_open_positions).await;

        // The slot registry makes releases idempotent, so it must start out matching the
        // positions that actually hold a slot.
        super::state::rebuild_position_slot_holders().await;
    }

    logger::info(LogTag::Positions, "Positions system initialized");

    Ok(())
}

/// Start positions manager service
///
/// Returns JoinHandle so ServiceManager can wait for graceful shutdown.
pub async fn start_positions_manager_service(
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> Result<tokio::task::JoinHandle<()>> {
    logger::info(
        LogTag::Positions,
        "Starting positions manager service (instrumented)",
    );

    initialize_positions_system().await?;

    // Create shutdown watch channel for price updater
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Start price updater task
    let _price_updater_handle =
        tokio::spawn(super::price_updater::start_price_updater(shutdown_rx));

    // Start verification worker
    let verification_handle = tokio::spawn(monitor.instrument(async move {
        verification_worker(shutdown).await;
        // Signal price updater to shutdown when verification worker exits
        let _ = shutdown_tx.send(true);
    }));

    // Return verification handle (price updater will be cleaned up automatically)
    Ok(verification_handle)
}

/// Wait for the next verification cycle: whichever comes first, the adaptive nap elapsing or NEW
/// work being enqueued.
///
/// The nap alone was dead time bolted onto the front of every trade. It is computed BEFORE the
/// queue is read (5s while the queue is empty), so a swap enqueued a moment after the worker
/// dozed off simply waited it out — even though a fresh item is immediately due, and the swap,
/// enqueued right after submission, often confirms within seconds. Nothing was pending but
/// the worker's alarm clock. On a DCA that is
/// exactly the gap the user sees: the notification says the buy is done, but the SOL and tokens
/// only reach the position when `DcaVerified` is applied by this loop.
async fn wait_for_next_cycle(nap: Duration) {
    tokio::select! {
        _ = sleep(nap) => {}
        _ = super::queue::wait_for_new_work() => {}
    }
}

/// Verification worker loop
async fn verification_worker(shutdown: Arc<Notify>) {
    logger::info(LogTag::Positions, "Starting verification worker");

    // Wait for Transactions and Pool services to be ready before starting verification
    let mut last_log = std::time::Instant::now();
    loop {
        let tx_ready =
            crate::global::TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst);
        let pool_ready =
            crate::global::POOL_SERVICE_READY.load(std::sync::atomic::Ordering::SeqCst);

        if tx_ready && pool_ready {
            logger::info(
                LogTag::Positions,
                "Dependencies ready (Transactions + Pool). Starting verification loop",
            );
            // Signal that positions system is ready now that dependencies are met
            crate::global::POSITIONS_SYSTEM_READY.store(true, std::sync::atomic::Ordering::SeqCst);
            break;
        }

        // Log only every 15 seconds
        if last_log.elapsed() >= Duration::from_secs(15) {
            logger::info(
                LogTag::Positions,
                &format!(
                    "Waiting for dependencies: tx_ready={} pool_ready={}",
                    tx_ready, pool_ready
                ),
            );
            last_log = std::time::Instant::now();
        }

        tokio::select! {
             _ = shutdown.notified() => {
        logger::info(LogTag::Positions, "Verification worker exiting during dependency wait");
               return;
             }
             _ = tokio::time::sleep(Duration::from_secs(1)) => {}
           }
    }

    let mut cycle_count = 0;
    let mut last_summary = chrono::Utc::now();

    loop {
        cycle_count += 1;
        let is_first_cycle = cycle_count == 1;

        // Compute adaptive sleep duration before select to avoid awaiting inside select arm
        let sleep_duration = {
            let (q_size, _) = super::queue::get_queue_status().await;
            if is_first_cycle {
                Duration::from_secs(3)
            } else if q_size > 50 {
                Duration::from_millis(500)
            } else if q_size > 0 {
                Duration::from_secs(2)
            } else {
                Duration::from_secs(5)
            }
        };

        tokio::select! {
            _ = shutdown.notified() => {
                logger::info(LogTag::Positions, "Stopping verification worker");
                break;
            }
            _ = wait_for_next_cycle(sleep_duration) => {
                run_verification_cycle(is_first_cycle, &mut last_summary).await;
            }
        }
    }
}

/// One pass of the verification worker: restore lost queue items, settle queued signatures
/// by their verdicts, then verify one batch of due items.
async fn run_verification_cycle(is_first_cycle: bool, last_summary: &mut DateTime<Utc>) {
    let (queue_size_before, requeued_count) = reenqueue_missing_verifications().await;
    if requeued_count > 0 {
        logger::info(
            LogTag::Positions,
            &format!(
                "Re-enqueued {} missing verifications (queue before: {})",
                requeued_count, queue_size_before
            ),
        );
    }

    // Emit a periodic summary event every ~30s
    let now = chrono::Utc::now();
    if (now - *last_summary).num_seconds() >= 30 {
        let (q_size_after, _) = super::queue::get_queue_status().await;
        crate::events::record_position_event_flexible(
            "verification_worker_summary",
            crate::events::Severity::Debug,
            None,
            None,
            serde_json::json!({
                "queue_size_before": queue_size_before,
                "queue_size_after": q_size_after,
                "requeued_count": requeued_count,
                "batch_size": VERIFICATION_BATCH_SIZE
            }),
        )
        .await;
        *last_summary = now;
    }

    settle_queued_signatures().await;

    let batch = poll_verification_batch(VERIFICATION_BATCH_SIZE).await;
    if batch.is_empty() {
        if is_first_cycle {
            logger::info(LogTag::Positions, "No pending verifications");
        }
        return;
    }

    logger::debug(
        LogTag::Positions,
        &format!("Processing {} verification items", batch.len()),
    );

    let mut unresolved = Vec::new();
    for item in batch {
        let started_at = chrono::Utc::now();
        crate::events::record_position_event_flexible(
            "verification_started",
            crate::events::Severity::Debug,
            Some(&item.mint),
            Some(&item.signature),
            json!({
                "kind": format!("{:?}", item.kind),
                "attempts": item.attempts,
                "created_at": item.created_at.to_rfc3339(),
                "last_attempt_at": item.last_attempt_at.map(|t| t.to_rfc3339()),
                "next_retry_at": item.next_retry_at.map(|t| t.to_rfc3339()),
                "expiry_height": item.expiry_height,
                "position_id": item.position_id,
            }),
        )
        .await;

        let outcome = verify_transaction(&item).await;
        if let Some(item) = process_verification_item(item, outcome, started_at).await {
            unresolved.push(item);
        }
    }
    settle_unresolved(unresolved).await;
}

/// Re-enqueues the verification of every unverified entry or exit signature of a position
/// that is missing from the queue. Returns the queue size before and the number re-enqueued.
/// The items carry no expiry bound; [`settle_queued_signatures`] assigns one.
async fn reenqueue_missing_verifications() -> (usize, usize) {
    let (queue_size_before, signatures_in_queue) = super::queue::get_queue_status().await;
    let mut requeued_count = 0;
    let positions = POSITIONS.read().await;
    for position in positions.iter() {
        if !position.transaction_entry_verified {
            if let Some(entry_sig) = &position.entry_transaction_signature {
                if !signatures_in_queue.contains(entry_sig) {
                    let item = VerificationItem::new(
                        entry_sig.clone(),
                        position.mint.clone(),
                        position.id,
                        VerificationKind::Entry,
                        None,
                    );
                    if enqueue_verification(item).await {
                        requeued_count += 1;
                    }
                }
            }
        }

        if !position.transaction_exit_verified {
            if let Some(exit_sig) = &position.exit_transaction_signature {
                if !signatures_in_queue.contains(exit_sig) {
                    let item = if let Some(pending) =
                        super::state::get_pending_partial_exit(exit_sig).await
                    {
                        VerificationItem::new_partial_exit(
                            exit_sig.clone(),
                            position.mint.clone(),
                            position.id,
                            pending.expected_exit_amount,
                            pending.requested_exit_percentage,
                            pending.expiry_height,
                        )
                    } else {
                        VerificationItem::new(
                            exit_sig.clone(),
                            position.mint.clone(),
                            position.id,
                            VerificationKind::Exit,
                            None,
                        )
                    };
                    if enqueue_verification(item).await {
                        requeued_count += 1;
                    }
                }
            }
        }
    }
    (queue_size_before, requeued_count)
}

/// Settles the due queued signatures the chain has not confirmed yet, from one batched
/// verdict read. Unconfirmed items without an expiry bound get the current one first, but
/// only the items queued before it was read: a transaction submitted before the read can no
/// longer land after the bound. A landed swap stays queued as confirmed; a swap that failed
/// on chain or did not land leaves the queue with its transition applied; a not-landed read
/// that cannot be acted on yet defers the item's next read.
async fn settle_queued_signatures() {
    let unbounded = unbounded_signatures().await;
    if !unbounded.is_empty() {
        if let Some(bound) = settle::submission_expiry_bound().await {
            assign_expiry_bound(bound, &unbounded).await;
        }
    }

    let candidates = settlement_candidates().await;
    if candidates.is_empty() {
        return;
    }
    let verdicts = match settle::signature_verdicts(&candidates).await {
        Ok(verdicts) => verdicts,
        Err(error) => {
            logger::debug(
                LogTag::Positions,
                &format!(
                    "Signature verdicts for {} queued items unavailable: {error}",
                    candidates.len()
                ),
            );
            return;
        }
    };

    for (item, verdict) in candidates.into_iter().zip(verdicts) {
        match settle::disposition(&item, verdict, attributable_dust(&item, verdict).await) {
            Disposition::Requeue {
                swap_confirmed: true,
            } => {
                mark_swap_confirmed(&item.signature).await;
            }
            Disposition::Requeue {
                swap_confirmed: false,
            } => {
                if item.not_landed_seen() {
                    clear_not_landed(&item.signature).await;
                }
            }
            Disposition::Defer => {
                if item.deferral_parks_entry() {
                    record_parked_entry(&item).await;
                }
                defer_settlement(&item.signature).await;
            }
            Disposition::Apply(transition) => {
                let Some(_lock) = settlement_lock(&item).await else {
                    defer_settlement(&item.signature).await;
                    continue;
                };
                if remove_verification(&item.signature).await.is_some() {
                    apply_settlement(&item, verdict, transition).await;
                }
            }
        }
    }
}

/// Settles items the verifier gave up on, from one batched verdict read. An item the
/// verdict leaves undecided is renewed rather than dropped: its swap may still land, and
/// dropping it would strand the position. A not-landed read that cannot be acted on yet
/// renews the item deferred.
async fn settle_unresolved(items: Vec<VerificationItem>) {
    if items.is_empty() {
        return;
    }
    let verdicts = settle::signature_verdicts(&items)
        .await
        .unwrap_or_else(|error| {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "Signature verdicts for {} unresolved items unavailable: {error}",
                    items.len()
                ),
            );
            vec![SignatureVerdict::Pending; items.len()]
        });

    for (item, verdict) in items.into_iter().zip(verdicts) {
        match settle::disposition(&item, verdict, attributable_dust(&item, verdict).await) {
            Disposition::Requeue { swap_confirmed } => {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Verification of {} (mint {}) is unresolved ({verdict:?}) - renewing it",
                        item.signature, item.mint
                    ),
                );
                let mut renewed = item.renewed();
                renewed.swap_confirmed |= swap_confirmed;
                renewed.not_landed_reads = 0;
                enqueue_verification(renewed).await;
            }
            Disposition::Defer => {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Verification of {} (mint {}) did not land but cannot be settled yet - renewing it deferred",
                        item.signature, item.mint
                    ),
                );
                if item.deferral_parks_entry() {
                    record_parked_entry(&item).await;
                }
                enqueue_verification(item.renewed().deferred()).await;
            }
            Disposition::Apply(transition) => {
                let Some(_lock) = settlement_lock(&item).await else {
                    enqueue_verification(item.renewed().deferred()).await;
                    continue;
                };
                apply_settlement(&item.renewed(), verdict, transition).await;
            }
        }
    }
}

/// The mint's position lock for applying a settlement, or `None` when it is not free within
/// [`SETTLEMENT_LOCK_WAIT`]. Held across the transition, it orders the settlement against a
/// DCA or partial exit of the mint, which registers its pending swap under the same lock.
async fn settlement_lock(item: &VerificationItem) -> Option<super::PositionLockGuard> {
    let lock = tokio::time::timeout(
        SETTLEMENT_LOCK_WAIT,
        super::state::acquire_position_lock(&item.mint),
    )
    .await
    .ok();
    if lock.is_none() {
        logger::debug(
            LogTag::Positions,
            &format!(
                "Settlement of {} deferred: the position lock of {} is held",
                item.signature, item.mint
            ),
        );
    }
    lock
}

/// Reports an entry its second not-landed read parks: the row and its slot stay held while
/// the wallet holds more of the mint than dust attributable to it, or that holding is unknown.
async fn record_parked_entry(item: &VerificationItem) {
    logger::warning(
        LogTag::Positions,
        &format!(
            "Entry {} (mint {}, position {:?}) did not land on two reads but its holding is not known dust - keeping the position until a later read decides",
            item.signature, item.mint, item.position_id
        ),
    );
    crate::events::record_position_event_flexible(
        "entry_settlement_parked",
        crate::events::Severity::Warn,
        Some(&item.mint),
        Some(&item.signature),
        json!({
            "position_id": item.position_id,
            "not_landed_reads": item.not_landed_reads.saturating_add(1),
            "expiry_height": item.expiry_height,
        }),
    )
    .await;
}

/// The attributable-dust reading an entry needs when its signature did not land on a
/// second consecutive read; `None` for every other item and verdict.
async fn attributable_dust(item: &VerificationItem, verdict: SignatureVerdict) -> Option<bool> {
    if item.kind == VerificationKind::Entry
        && !item.is_dca
        && verdict == SignatureVerdict::NotLanded
        && item.not_landed_seen()
    {
        settle::entry_attributable_is_dust(item).await
    } else {
        None
    }
}

/// What becomes of a settled item whose transition failed to apply.
#[derive(Debug)]
enum RefusedSettlement {
    /// The refusal will not change on a retry: stop verifying the signature for the session.
    Abandon(GiveUpReason),
    /// The refusal may pass later: read the signature again after a deferral.
    Retry(VerificationItem),
}

/// The follow-up to a settlement of `item` that `error` refused. A retry keeps the item's
/// confirmation as it was: only a verified transaction confirms a swap, and a refused
/// settlement verified nothing.
fn refused_settlement(item: &VerificationItem, error: &Error) -> RefusedSettlement {
    match item.apply_failure_disposition(error) {
        ApplyFailureDisposition::Drop(reason) => RefusedSettlement::Abandon(reason),
        ApplyFailureDisposition::Requeue => RefusedSettlement::Retry(item.deferred()),
    }
}

/// Applies the transition a signature verdict decided and settles the trade action waiting
/// on the signature. A refusal that a retry cannot change abandons the signature for the
/// session, so the next cycle does not re-enqueue and refuse it again; any other refusal
/// reads the signature again after a deferral.
async fn apply_settlement(
    item: &VerificationItem,
    verdict: SignatureVerdict,
    transition: super::transitions::PositionTransition,
) {
    let action_verdict = match verdict {
        SignatureVerdict::NotLanded => Err(crate::actions::ActionFailure::new(
            crate::i18n::ids::ACTIONS_FAILURE_VERIFICATION_EXPIRED,
        )),
        _ => transition.action_verdict().unwrap_or_else(|| {
            Err(crate::actions::ActionFailure::new(
                crate::i18n::ids::ACTIONS_FAILURE_TRANSACTION_FAILED,
            ))
        }),
    };
    match apply_transition(transition).await {
        Ok(_) => {
            logger::info(
                LogTag::Positions,
                &format!(
                    "Settled {} (mint {} kind {:?}) by its verdict {verdict:?}",
                    item.signature, item.mint, item.kind
                ),
            );
            crate::actions::settle_verification(&item.signature, action_verdict).await;
        }
        Err(error) => {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Failed to apply the {verdict:?} settlement of {} (mint {} kind {:?}): {error}",
                    item.signature, item.mint, item.kind
                ),
            );
            match refused_settlement(item, &error) {
                RefusedSettlement::Abandon(reason) => {
                    abandon_after_apply_failure(item, reason, &error).await;
                }
                RefusedSettlement::Retry(item) => {
                    enqueue_verification(item).await;
                }
            }
        }
    }
}

/// Books the outcome of one verification attempt: applies its transition, requeues it, or,
/// when the verifier gives up on it, returns the item for settlement by its signature
/// verdict. `started_at` is when the attempt began.
pub(super) async fn process_verification_item(
    item: VerificationItem,
    outcome: VerificationOutcome,
    started_at: DateTime<Utc>,
) -> Option<VerificationItem> {
    let duration_ms = || (chrono::Utc::now() - started_at).num_milliseconds().max(0) as u64;
    match outcome {
        VerificationOutcome::Transition(transition) => {
            let verdict = transition.action_verdict();
            match apply_transition(transition).await {
                Ok(effects) => {
                    remove_verification(&item.signature).await;
                    if let Some(verdict) = verdict {
                        crate::actions::settle_verification(&item.signature, verdict).await;
                    }

                    {
                        use crate::positions::metrics::VERIFICATION_METRICS;
                        use std::sync::atomic::Ordering;

                        VERIFICATION_METRICS
                            .operations
                            .fetch_add(1, Ordering::Relaxed);

                        if item.is_dca {
                            VERIFICATION_METRICS
                                .dca_verified
                                .fetch_add(1, Ordering::Relaxed);
                        } else if item.is_partial_exit {
                            VERIFICATION_METRICS
                                .partial_exit_verified
                                .fetch_add(1, Ordering::Relaxed);
                        } else {
                            match item.kind {
                                VerificationKind::Entry => {
                                    VERIFICATION_METRICS
                                        .entry_verified
                                        .fetch_add(1, Ordering::Relaxed);
                                }
                                VerificationKind::Exit => {
                                    VERIFICATION_METRICS
                                        .exit_verified
                                        .fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                    }

                    crate::events::record_position_event_flexible(
                        "verification_finished",
                        crate::events::Severity::Info,
                        Some(&item.mint),
                        Some(&item.signature),
                        json!({
                            "kind": format!("{:?}", item.kind),
                            "attempts": item.attempts,
                            "duration_ms": duration_ms(),
                            "started_at": started_at.to_rfc3339(),
                            "result": "transition",
                            "db_updated": effects.db_updated,
                            "position_closed": effects.position_closed,
                            "position_id": item.position_id,
                        }),
                    )
                    .await;

                    logger::debug(
                        LogTag::Positions,
                        &format!(
                            "Applied transition for {} (mint {} kind {:?}): db_updated={}, position_closed={}",
                            item.signature,
                            item.mint,
                            item.kind,
                            effects.db_updated,
                            effects.position_closed
                        ),
                    );
                }
                Err(e) => {
                    crate::positions::metrics::VERIFICATION_METRICS
                        .errors
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    logger::error(
                        LogTag::Positions,
                        &format!(
                            "Failed to apply transition for {} (mint {} kind {:?}): {}",
                            item.signature, item.mint, item.kind, e
                        ),
                    );
                    crate::events::record_position_event_flexible(
                        "verification_finished",
                        crate::events::Severity::Warn,
                        Some(&item.mint),
                        Some(&item.signature),
                        json!({
                            "kind": format!("{:?}", item.kind),
                            "attempts": item.attempts,
                            "duration_ms": duration_ms(),
                            "started_at": started_at.to_rfc3339(),
                            "result": "apply_error",
                            "error": e.to_string(),
                            "position_id": item.position_id
                        }),
                    )
                    .await;
                    match item.apply_failure_disposition(&e) {
                        ApplyFailureDisposition::Requeue => {
                            let mut confirmed = item;
                            confirmed.swap_confirmed = true;
                            requeue_verification(confirmed).await;
                        }
                        ApplyFailureDisposition::Drop(reason) => {
                            abandon_after_apply_failure(&item, reason, &e).await;
                        }
                    }
                }
            }
        }
        VerificationOutcome::RetryTransient(reason) => {
            if let Some(give_up_reason) = item.should_give_up() {
                {
                    use crate::positions::metrics::VERIFICATION_METRICS;
                    use std::sync::atomic::Ordering;

                    VERIFICATION_METRICS
                        .abandoned
                        .fetch_add(1, Ordering::Relaxed);
                    VERIFICATION_METRICS.errors.fetch_add(1, Ordering::Relaxed);
                }

                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Verification of {} (mint={}, kind={:?}) gave up: {:?} - last error: {} - settling it by its signature verdict",
                        item.signature, item.mint, item.kind, give_up_reason, reason
                    ),
                );

                crate::events::record_position_event_flexible(
                    "verification_abandoned",
                    crate::events::Severity::Error,
                    Some(&item.mint),
                    Some(&item.signature),
                    serde_json::json!({
                        "give_up_reason": give_up_reason,
                        "last_error": reason,
                        "attempts": item.attempts,
                        "age_hours": (chrono::Utc::now() - item.created_at).num_hours(),
                        "kind": format!("{:?}", item.kind),
                        "position_id": item.position_id,
                        "created_at": item.created_at.to_rfc3339()
                    }),
                )
                .await;

                return Some(item);
            }

            crate::positions::metrics::VERIFICATION_METRICS
                .retries
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            logger::debug(
                LogTag::Positions,
                &format!(
                    "Retrying verification for {} (mint {} kind {:?} attempts {}): {}",
                    item.signature, item.mint, item.kind, item.attempts, reason
                ),
            );
            crate::events::record_position_event_flexible(
                "verification_finished",
                crate::events::Severity::Warn,
                Some(&item.mint),
                Some(&item.signature),
                json!({
                    "kind": format!("{:?}", item.kind),
                    "attempts": item.attempts,
                    "duration_ms": duration_ms(),
                    "started_at": started_at.to_rfc3339(),
                    "result": "retry",
                    "reason": reason,
                    "position_id": item.position_id,
                    "next_retry_at": item.next_retry_at.map(|t| t.to_rfc3339())
                }),
            )
            .await;
            requeue_verification(item).await;
        }
    }
    None
}

/// Stops verifying an item whose transition failed to apply and will not be retried, and
/// settles the trade action waiting on its signature as given up.
///
/// The position is left exactly as stored: no kind-specific abandonment transition runs,
/// because the swap may be confirmed and the row only lacks its booking. The signature is
/// refused by every later enqueue for the rest of the session; a restart re-enqueues the
/// still-pending state from storage.
async fn abandon_after_apply_failure(item: &VerificationItem, reason: GiveUpReason, error: &Error) {
    crate::positions::metrics::VERIFICATION_METRICS
        .abandoned
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    logger::error(
        LogTag::Positions,
        &format!(
            "Abandoning verification for {} (mint={}, kind={:?}) after apply failure: {:?} - last error: {}",
            item.signature, item.mint, item.kind, reason, error
        ),
    );

    crate::events::record_position_event_flexible(
        "verification_abandoned",
        crate::events::Severity::Error,
        Some(&item.mint),
        Some(&item.signature),
        json!({
            "give_up_reason": reason,
            "last_error": error.to_string(),
            "attempts": item.attempts,
            "age_hours": (chrono::Utc::now() - item.created_at).num_hours(),
            "kind": format!("{:?}", item.kind),
            "position_id": item.position_id,
            "created_at": item.created_at.to_rfc3339(),
            "stage": "apply",
            "error_retryable": error.is_retryable(),
        }),
    )
    .await;

    abandon_verification(item.signature.clone()).await;
    crate::actions::settle_verification(
        &item.signature,
        Err(crate::actions::ActionFailure::with_details(
            crate::i18n::ids::ACTIONS_FAILURE_VERIFICATION_GAVE_UP,
            format!("{reason:?}"),
        )),
    )
    .await;
}

#[cfg(test)]
mod refused_settlement_tests {
    use super::*;
    use crate::errors::DatabaseError;

    fn entry() -> VerificationItem {
        VerificationItem::new(
            "entry-sig".to_owned(),
            "mint".to_owned(),
            Some(7),
            VerificationKind::Entry,
            Some(100),
        )
    }

    #[test]
    fn a_refused_orphan_removal_is_abandoned() {
        let refusal = Error::EntryLanded {
            position_id: 7,
            signature: "entry-sig".to_owned(),
        };
        for item in [entry(), entry().deferred()] {
            assert!(matches!(
                refused_settlement(&item, &refusal),
                RefusedSettlement::Abandon(GiveUpReason::ApplyRejected { .. })
            ));
        }
    }

    #[test]
    fn a_retryable_refusal_is_read_again_later_without_confirming_the_swap() {
        let busy = Error::Database(DatabaseError::Busy {
            operation: "commit_booking".to_owned(),
            message: "database is locked".to_owned(),
        });
        let item = entry();
        match refused_settlement(&item, &busy) {
            RefusedSettlement::Retry(retry) => {
                assert!(
                    !retry.swap_confirmed,
                    "a refused settlement verified nothing"
                );
                assert_eq!(retry.signature, item.signature);
                assert_eq!(retry.attempts, item.attempts);
                assert!(retry.next_retry_at.is_some_and(|at| at > Utc::now()));
            }
            other => panic!("expected a retry, got {other:?}"),
        }
    }
}
