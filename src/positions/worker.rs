use super::db::initialize_positions_database;
use super::{
    apply::apply_transition,
    queue::{
        enqueue_verification,
        gc_expired_verifications,
        poll_verification_batch,
        queue_has_items_with_expiry,
        remove_verification,
        requeue_verification,
        VerificationItem,
        VerificationKind,
    },
    state::{
        reconcile_global_position_semaphore,
        MINT_TO_POSITION_INDEX,
        POSITIONS,
        SIG_TO_MINT_INDEX,
    },
    verifier::{ verify_transaction, VerificationOutcome },
};
use crate::{ arguments::is_debug_positions_enabled, logger::{ log, LogTag }, rpc::get_rpc_client };
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tokio::{ sync::Notify, time::{ sleep, Duration } };

const VERIFICATION_BATCH_SIZE: usize = 10;

/// Initialize positions system
pub async fn initialize_positions_system() -> Result<(), String> {
    log(LogTag::Positions, "STARTUP", "🚀 Initializing positions system");

    // Initialize database
    initialize_positions_database().await.map_err(|e|
        format!("Failed to initialize positions database: {}", e)
    )?;

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
                            None
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
                            None
                        );
                        enqueue_verification(item).await;
                        unverified_count += 1;
                    }
                }
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

            log(
                LogTag::Positions,
                "STARTUP",
                &format!(
                    "✅ Loaded {} positions, {} pending verification",
                    global_positions.len(),
                    unverified_count
                )
            );
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "WARNING",
                &format!("Failed to load positions from database: {}", e)
            );
        }
    }

    // Reconcile semaphore capacity with existing open positions AFTER loading state
    {
        use crate::trader::MAX_OPEN_POSITIONS;
        reconcile_global_position_semaphore(MAX_OPEN_POSITIONS).await;
    }

    log(LogTag::Positions, "STARTUP", "✅ Positions system initialized");

    Ok(())
}

/// Start positions manager service
pub async fn start_positions_manager_service(shutdown: Arc<Notify>) -> Result<(), String> {
    log(LogTag::Positions, "STARTUP", "🚀 Starting positions manager service");

    initialize_positions_system().await?;

    // Start verification worker
    tokio::spawn(verification_worker(shutdown));

    Ok(())
}

/// Verification worker loop
async fn verification_worker(shutdown: Arc<Notify>) {
    log(LogTag::Positions, "STARTUP", "🔍 Starting verification worker");

    // Wait for Transactions and Pool services to be ready before starting verification
    let mut last_log = std::time::Instant::now();
    loop {
        let tx_ready = crate::global::TRANSACTIONS_SYSTEM_READY.load(
            std::sync::atomic::Ordering::SeqCst
        );
        let pool_ready = crate::global::POOL_SERVICE_READY.load(
            std::sync::atomic::Ordering::SeqCst
        );

        if tx_ready && pool_ready {
            log(
                LogTag::Positions,
                "STARTUP",
                "✅ Dependencies ready (Transactions + Pool). Starting verification loop"
            );
            // Signal that positions system is ready now that dependencies are met
            crate::global::POSITIONS_SYSTEM_READY.store(true, std::sync::atomic::Ordering::SeqCst);
            break;
        }

        // Log only every 15 seconds
        if last_log.elapsed() >= Duration::from_secs(15) {
            log(
                LogTag::Positions,
                "STARTUP",
                &format!(
                    "⏳ Waiting for dependencies: tx_ready={} pool_ready={}",
                    tx_ready,
                    pool_ready
                )
            );
            last_log = std::time::Instant::now();
        }

        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Positions, "SHUTDOWN", "🛑 Verification worker exiting during dependency wait");
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
                log(LogTag::Positions, "SHUTDOWN", "🛑 Stopping verification worker");
                break;
            }
            _ = sleep(sleep_duration) => {
                // GUARD: Re-enqueue any missing verifications that should be queued but aren't
                let mut requeued_count = 0;
                let (queue_size_before, signatures_in_queue) = super::queue::get_queue_status().await;
                {
                    let positions = POSITIONS.read().await;
                    for position in positions.iter() {
                        // Check for missing entry verifications
                        if !position.transaction_entry_verified {
                            if let Some(entry_sig) = &position.entry_transaction_signature {
                                // Check if already in queue
                                if !signatures_in_queue.contains(entry_sig) {
                                    let item = VerificationItem::new(
                                        entry_sig.clone(),
                                        position.mint.clone(),
                                        position.id,
                                        VerificationKind::Entry,
                                        None,
                                    );
                                    enqueue_verification(item).await;
                                    requeued_count += 1;
                                }
                            }
                        }

                        // Check for missing exit verifications
                        if !position.transaction_exit_verified {
                            if let Some(exit_sig) = &position.exit_transaction_signature {
                                // Check if already in queue
                                if !signatures_in_queue.contains(exit_sig) {
                                    let item = VerificationItem::new(
                                        exit_sig.clone(),
                                        position.mint.clone(),
                                        position.id,
                                        VerificationKind::Exit,
                                        None,
                                    );
                                    enqueue_verification(item).await;
                                    requeued_count += 1;
                                }
                            }
                        }
                    }
                }

                if requeued_count > 0 {
                    log(
                        LogTag::Positions,
                        "VERIFICATION_GUARD_REQUEUE",
                        &format!(
                            "🛡️ Re-enqueued {} missing verifications (queue before: {})",
                            requeued_count,
                            queue_size_before
                        )
                    );
                }

                // Emit a periodic summary event every ~30s
                let now = chrono::Utc::now();
                if (now - last_summary).num_seconds() >= 30 {
                    let (q_size_after, _) = super::queue::get_queue_status().await;
                    crate::events::record_safe(
                        crate::events::Event::new(
                            crate::events::EventCategory::Position,
                            Some("verification_worker_summary".to_string()),
                            crate::events::Severity::Debug,
                            None,
                            None,
                            serde_json::json!({
                                "queue_size_before": queue_size_before,
                                "queue_size_after": q_size_after,
                                "requeued_count": requeued_count,
                                "batch_size": VERIFICATION_BATCH_SIZE
                            })
                        )
                    ).await;
                    last_summary = now;
                }

                // Clean up expired items - only fetch block height if needed
                let current_height = if queue_has_items_with_expiry().await {
                    get_rpc_client().get_block_height().await.ok()
                } else {
                    None
                };
                let expired_items = gc_expired_verifications(current_height).await;

                if !expired_items.is_empty() {
                    log(
                        LogTag::Positions,
                        "CLEANUP",
                        &format!("🧹 Cleaned up {} expired verifications", expired_items.len())
                    );

                    // Handle expired entry transactions by removing orphan positions
                    for item in expired_items {
                        if item.kind == VerificationKind::Entry {
                            if let Some(position_id) = item.position_id {
                                let transition = super::transitions::PositionTransition::RemoveOrphanEntry {
                                    position_id,
                                };
                                let _ = apply_transition(transition).await;
                            }
                        }
                    }
                }

                // Process verification batch
                let batch = poll_verification_batch(VERIFICATION_BATCH_SIZE).await;

                if !batch.is_empty() {
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!("🔄 Processing {} verification items", batch.len())
                        );
                    }

                    for item in batch {
                        // Emit a verification_started event and take timing baselines
                        let started_at = chrono::Utc::now();
                        let timer = Instant::now();
                        crate::events::record_safe(
                            crate::events::Event::new(
                                crate::events::EventCategory::Position,
                                Some("verification_started".to_string()),
                                crate::events::Severity::Debug,
                                Some(item.mint.clone()),
                                Some(item.signature.clone()),
                                json!({
                                    "kind": format!("{:?}", item.kind),
                                    "attempts": item.attempts,
                                    "created_at": item.created_at.to_rfc3339(),
                                    "last_attempt_at": item.last_attempt_at.map(|t| t.to_rfc3339()),
                                    "next_retry_at": item.next_retry_at.map(|t| t.to_rfc3339()),
                                    "expiry_height": item.expiry_height,
                                    "position_id": item.position_id,
                                })
                            )
                        ).await;

                        match verify_transaction(&item).await {
                            VerificationOutcome::Transition(transition) => {
                                match apply_transition(transition).await {
                                    Ok(effects) => {
                                        remove_verification(&item.signature).await;

                                        // Emit verification_finished (success/transition)
                                        crate::events::record_safe(
                                            crate::events::Event::new(
                                                crate::events::EventCategory::Position,
                                                Some("verification_finished".to_string()),
                                                crate::events::Severity::Info,
                                                Some(item.mint.clone()),
                                                Some(item.signature.clone()),
                                                json!({
                                                    "kind": format!("{:?}", item.kind),
                                                    "attempts": item.attempts,
                                                    "duration_ms": timer.elapsed().as_millis() as u64,
                                                    "started_at": started_at.to_rfc3339(),
                                                    "result": "transition",
                                                    "db_updated": effects.db_updated,
                                                    "position_closed": effects.position_closed,
                                                    "position_id": item.position_id,
                                                })
                                            )
                                        ).await;

                                        if is_debug_positions_enabled() {
                                            log(
                                                LogTag::Positions,
                                                "DEBUG",
                                                &format!(
                                                    "✅ Applied transition for {} (mint {} kind {:?}): db_updated={}, position_closed={}",
                                                    item.signature,
                                                    item.mint,
                                                    item.kind,
                                                    effects.db_updated,
                                                    effects.position_closed
                                                )
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        log(
                                            LogTag::Positions,
                                            "ERROR",
                                            &format!(
                                                "❌ Failed to apply transition for {} (mint {} kind {:?}): {}",
                                                item.signature,
                                                item.mint,
                                                item.kind,
                                                e
                                            )
                                        );
                                        // Emit verification_finished (apply_error)
                                        crate::events::record_safe(
                                            crate::events::Event::new(
                                                crate::events::EventCategory::Position,
                                                Some("verification_finished".to_string()),
                                                crate::events::Severity::Warn,
                                                Some(item.mint.clone()),
                                                Some(item.signature.clone()),
                                                json!({
                                                    "kind": format!("{:?}", item.kind),
                                                    "attempts": item.attempts,
                                                    "duration_ms": timer.elapsed().as_millis() as u64,
                                                    "started_at": started_at.to_rfc3339(),
                                                    "result": "apply_error",
                                                    "error": e,
                                                    "position_id": item.position_id
                                                })
                                            )
                                        ).await;
                                        requeue_verification(item).await;
                                    }
                                }
                            }
                            VerificationOutcome::RetryTransient(reason) => {
                                if is_debug_positions_enabled() {
                                    log(
                                        LogTag::Positions,
                                        "DEBUG",
                                        &format!(
                                            "🔄 Retrying verification for {} (mint {} kind {:?} attempts {}): {}",
                                            item.signature,
                                            item.mint,
                                            item.kind,
                                            item.attempts,
                                            reason
                                        )
                                    );
                                }
                                // Emit verification_finished (retry)
                                crate::events::record_safe(
                                    crate::events::Event::new(
                                        crate::events::EventCategory::Position,
                                        Some("verification_finished".to_string()),
                                        crate::events::Severity::Warn,
                                        Some(item.mint.clone()),
                                        Some(item.signature.clone()),
                                        json!({
                                            "kind": format!("{:?}", item.kind),
                                            "attempts": item.attempts,
                                            "duration_ms": timer.elapsed().as_millis() as u64,
                                            "started_at": started_at.to_rfc3339(),
                                            "result": "retry",
                                            "reason": reason,
                                            "position_id": item.position_id,
                                            "next_retry_at": item.next_retry_at.map(|t| t.to_rfc3339())
                                        })
                                    )
                                ).await;
                                requeue_verification(item).await;
                            }
                            VerificationOutcome::PermanentFailure(transition) => {
                                log(
                                    LogTag::Positions,
                                    "WARNING",
                                    &format!(
                                        "🚫 Permanent failure for {} (mint {} kind {:?}), applying cleanup",
                                        item.signature,
                                        item.mint,
                                        item.kind
                                    )
                                );

                                let _ = apply_transition(transition).await;
                                remove_verification(&item.signature).await;

                                // Emit verification_finished (permanent_failure)
                                crate::events::record_safe(
                                    crate::events::Event::new(
                                        crate::events::EventCategory::Position,
                                        Some("verification_finished".to_string()),
                                        crate::events::Severity::Warn,
                                        Some(item.mint.clone()),
                                        Some(item.signature.clone()),
                                        json!({
                                            "kind": format!("{:?}", item.kind),
                                            "attempts": item.attempts,
                                            "duration_ms": timer.elapsed().as_millis() as u64,
                                            "started_at": started_at.to_rfc3339(),
                                            "result": "permanent_failure",
                                            "position_id": item.position_id
                                        })
                                    )
                                ).await;
                            }
                        }
                    }
                } else if is_first_cycle {
                    log(LogTag::Positions, "VERIFICATION_QUEUE", "📋 No pending verifications");
                }
            }
        }
    }
}
