use crate::{
    positions_db::initialize_positions_database,
    logger::{ log, LogTag },
    rpc::get_rpc_client,
    arguments::is_debug_positions_enabled,
};
use super::{
    state::{ POSITIONS, SIG_TO_MINT_INDEX, MINT_TO_POSITION_INDEX },
    queue::{
        poll_verification_batch,
        requeue_verification,
        remove_verification,
        gc_expired_verifications,
        enqueue_verification,
        VerificationItem,
        VerificationKind,
    },
    verifier::{ verify_transaction, VerificationOutcome },
    apply::apply_transition,
};
use std::sync::Arc;
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
    match crate::positions_db::load_all_positions().await {
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

    let mut cycle_count = 0;

    loop {
        cycle_count += 1;
        let is_first_cycle = cycle_count == 1;

        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Positions, "SHUTDOWN", "🛑 Stopping verification worker");
                break;
            }
            _ = sleep(if is_first_cycle { Duration::from_secs(5) } else { Duration::from_secs(15) }) => {
                // GUARD: Re-enqueue any missing verifications that should be queued but aren't
                let mut requeued_count = 0;
                {
                    let positions = POSITIONS.read().await;
                    for position in positions.iter() {
                        // Check for missing entry verifications
                        if !position.transaction_entry_verified {
                            if let Some(entry_sig) = &position.entry_transaction_signature {
                                // Check if already in queue
                                let (queue_size, signatures) = super::queue::get_queue_status().await;
                                if !signatures.contains(entry_sig) {
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
                                let (queue_size, signatures) = super::queue::get_queue_status().await;
                                if !signatures.contains(exit_sig) {
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
                        &format!("🛡️ Re-enqueued {} missing verifications", requeued_count)
                    );
                }
                
                // Clean up expired items
                let current_height = get_rpc_client().get_block_height().await.ok();
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
                        match verify_transaction(&item).await {
                            VerificationOutcome::Transition(transition) => {
                                match apply_transition(transition).await {
                                    Ok(effects) => {
                                        remove_verification(&item.signature).await;
                                        
                                        if is_debug_positions_enabled() {
                                            log(
                                                LogTag::Positions,
                                                "DEBUG",
                                                &format!("✅ Applied transition for {}: db_updated={}, position_closed={}", 
                                                    item.signature, effects.db_updated, effects.position_closed)
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        log(
                                            LogTag::Positions,
                                            "ERROR",
                                            &format!("❌ Failed to apply transition for {}: {}", item.signature, e)
                                        );
                                        requeue_verification(item).await;
                                    }
                                }
                            }
                            VerificationOutcome::RetryTransient(reason) => {
                                if is_debug_positions_enabled() {
                                    log(
                                        LogTag::Positions,
                                        "DEBUG",
                                        &format!("🔄 Retrying verification for {}: {}", item.signature, reason)
                                    );
                                }
                                requeue_verification(item).await;
                            }
                            VerificationOutcome::PermanentFailure(transition) => {
                                log(
                                    LogTag::Positions,
                                    "WARNING",
                                    &format!("🚫 Permanent failure for {}, applying cleanup", item.signature)
                                );
                                
                                let _ = apply_transition(transition).await;
                                remove_verification(&item.signature).await;
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
