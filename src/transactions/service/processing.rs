//! Transaction service processing — handles incoming WebSocket messages and dispatches analysis.
//
// Main service loop and periodic processing tasks

use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

use crate::logger::{self, LogTag};
use crate::transactions::{
    fetcher::TransactionFetcher,
    processor::TransactionProcessor,
    utils::{
        add_signature_to_known_globally, cleanup_expired_pending_transactions,
        is_signature_known_globally, remove_pending_transaction_globally,
    },
};

use super::config::{
    defer_transaction_retry, get_deferred_retries_count, get_deferred_retry_keys, ServiceConfig,
    DEFERRED_RETRIES, DEFERRED_RETRY_KEYS,
};
use super::health::{perform_health_check, should_perform_fallback_check, ServiceMetrics};
use super::lifecycle::{get_global_transaction_manager, SHUTDOWN_NOTIFY};
use super::websocket::{handle_websocket_transaction, initialize_websocket_monitoring};

// =============================================================================
// MAIN SERVICE LOOP
// =============================================================================

/// Main service loop that coordinates all transaction monitoring activities
pub async fn run_transaction_service(config: ServiceConfig) -> Result<(), String> {
    logger::info(
        LogTag::Transactions,
        &format!(
            "Transaction service running for wallet: {} (interval: {}s)",
            &config.wallet_pubkey.to_string(),
            config.check_interval_secs
        ),
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
        logger::info(
            LogTag::Transactions,
            "WebSocket initialized successfully - marking as active",
        );
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
                 logger::warning(
                   LogTag::Transactions,
                   &format!("Periodic check failed: {e}")
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
                     logger::warning(
                       LogTag::Transactions,
                       &format!("WebSocket transaction handling failed: {e}")
                     );
                   }
                 }
                 None => {
                   logger::info(
                     LogTag::Transactions,
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
                   logger::info(
                     LogTag::Transactions,
        "No WebSocket connection - attempting to establish..."
                   );

                   match initialize_websocket_monitoring(config.wallet_pubkey).await {
                     Ok(new_receiver) => {
                       websocket_receiver = new_receiver;
                       metrics.increment_websocket_reconnect();
                       metrics.update_websocket_activity();
                       logger::info(
                         LogTag::Transactions,
        "WebSocket connection established successfully"
                       );
                     }
                     Err(e) => {
                       logger::info(
                         LogTag::Transactions,
                         &format!("WebSocket connection failed: {e} - will retry in 15s")
                       );
                     }
                   }
                 } else {
                   // WebSocket exists - it's healthy (receiver is open means loop is running)
                   // Log debug info about connection state
                   let last_msg_ago = if let Some(last_msg) = metrics.last_websocket_message {
                     (Utc::now() - last_msg).num_seconds()
                   } else {
                     -1
                   };
                   logger::debug(
                     LogTag::Transactions,
                     &format!(
                       "WebSocket health check: HEALTHY (connection open, last tx message: {}s ago, reconnects: {})",
                       last_msg_ago,
                       metrics.websocket_reconnect_count
                     )
                   );
                 }
               }
             }

             // Shutdown signal
             _ = SHUTDOWN_NOTIFY.notified() => {
               logger::info(LogTag::Transactions, "Received shutdown signal");
               return Ok(());
             }

             // General service health check (every 5 minutes)
             _ = tokio::time::sleep(Duration::from_secs(300)) => {
               if let Err(e) = perform_health_check(&mut metrics).await {
                 logger::warning(
                   LogTag::Transactions,
                   &format!("Health check failed: {e}")
                 );
               }
             }
           }
    }
}

// =============================================================================
// PERIODIC PROCESSING
// =============================================================================

/// Perform periodic transaction checking and maintenance
async fn perform_periodic_check(
    config: &ServiceConfig,
    processor: &Arc<TransactionProcessor>,
    fetcher: &Arc<TransactionFetcher>,
    metrics: &mut ServiceMetrics,
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

    logger::debug(
        LogTag::Transactions,
        &format!(
            "Periodic check complete: {}ms, expired: {}, retries: {}, fallback: {}",
            duration.as_millis(),
            expired_count,
            retry_count,
            fallback_count
        ),
    );

    Ok(())
}

/// Process deferred retries that are ready for re-processing
/// Handles transactions that failed due to temporary issues like RPC indexing delays
async fn process_deferred_retries(
    config: &ServiceConfig,
    processor: &Arc<TransactionProcessor>,
) -> Result<usize, String> {
    let now = Utc::now();
    let mut ready_retries = Vec::new();

    // Collect retries that are ready to be processed
    {
        DEFERRED_RETRIES.run_pending_tasks();
        // Use parallel key index to iterate all deferred retry keys
        let all_keys = get_deferred_retry_keys();

        for sig in &all_keys {
            if let Some(retry) = DEFERRED_RETRIES.get(sig) {
                if retry.next_retry_at <= now {
                    ready_retries.push(retry.clone());
                    DEFERRED_RETRIES.invalidate(sig);
                    DEFERRED_RETRY_KEYS.remove(sig);
                }
            } else {
                // Key expired from moka TTL — clean up the key index
                DEFERRED_RETRY_KEYS.remove(sig);
            }
        }
    }

    if ready_retries.is_empty() {
        return Ok(0);
    }

    let mut processed = 0;
    let mut re_deferred = 0;

    for mut retry in ready_retries {
        let age_secs = (now - retry.first_seen).num_seconds();
        let signature_short = retry.signature[..8].to_string();
        let attempts = retry.attempts;

        match processor.process_transaction(&retry.signature).await {
            Ok(_) => {
                add_signature_to_known_globally(retry.signature.clone()).await;
                remove_pending_transaction_globally(&retry.signature).await;
                processed += 1;
            }
            Err(e)
                if (crate::errors::is_rpc_indexing_delay(&e)
                    || e.contains("RPC indexing delay"))
                    && retry.attempts < 3 =>
            {
                // Still having indexing issues, retry again with exponential backoff
                let current_attempts = retry.attempts;
                retry.attempts += 1;
                let delay_secs = 5 * (2_i64.pow(retry.attempts - 1));
                retry.current_delay_secs = delay_secs;
                retry.next_retry_at = now + chrono::Duration::seconds(delay_secs);
                retry.last_error = Some(e.clone());

                DEFERRED_RETRIES.insert(retry.signature.clone(), retry);
                re_deferred += 1;
            }
            Err(e) => {
                logger::info(
                    LogTag::Transactions,
                    &format!(
                        "Deferred retry failed permanently for {} after {} attempts (age: {}s): {}",
                        signature_short, attempts, age_secs, e
                    ),
                );
            }
        }
    }

    if processed > 0 || re_deferred > 0 {
        logger::info(
            LogTag::Transactions,
            &format!(
                "Deferred retries: {} processed, {} re-deferred, {} remaining in queue",
                processed,
                re_deferred,
                get_deferred_retries_count().await
            ),
        );
    }

    Ok(processed)
}

/// Perform fallback transaction check when WebSocket is not providing updates
/// Enhanced with database consistency verification
async fn perform_fallback_transaction_check(
    config: &ServiceConfig,
    fetcher: &Arc<TransactionFetcher>,
    processor: &Arc<TransactionProcessor>,
) -> Result<usize, String> {
    let pending_txs_count = crate::transactions::utils::get_pending_transactions_count().await;
    let (pending_verification_count, _) = crate::positions::queue::get_queue_status().await;

    logger::info(
    LogTag::Transactions,
    &format!(
      "Performing fallback transaction check (WebSocket inactive) - Pending txs: {}, Pending verifications: {}",
      pending_txs_count,
      pending_verification_count
    )
  );

    // Fetch recent transactions
    let signatures = fetcher
        .fetch_recent_signatures(config.wallet_pubkey, 100)
        .await?;
    let fetched_count = signatures.len();

    let mut new_count = 0;
    let mut db_persistence_failures = 0;

    // Get database reference for verification
    let transaction_db = crate::transactions::database::get_transaction_database().await;

    for signature in signatures {
        // Check if signature is already known
        if !is_signature_known_globally(&signature).await {
            // Process new transaction
            match processor.process_transaction(&signature).await {
                Ok(_) => {
                    // Add to in-memory global state
                    add_signature_to_known_globally(signature.clone()).await;

                    // CRITICAL FIX: Verify persistence to database
                    if let Some(db) = transaction_db.as_ref() {
                        // Ensure it's persisted to known_signatures table
                        if let Err(e) = db.add_known_signature(&signature).await {
                            logger::info(
                                LogTag::Transactions,
                                &format!(
                                    "Failed to persist known signature {} to database: {}",
                                    &signature[..8],
                                    e
                                ),
                            );
                            db_persistence_failures += 1;
                        } else {
                            // Double-check it's actually there
                            match db.is_signature_known(&signature).await {
                                Ok(true) => {}
                                Ok(false) => {
                                    logger::info(
                                        LogTag::Transactions,
                                        &format!(
 "Signature {} added to in-memory state but not found in database!",
                      &signature[..8]
                    ),
                                    );
                                    db_persistence_failures += 1;
                                }
                                Err(e) => {
                                    logger::info(
                                        LogTag::Transactions,
                                        &format!(
                                            "Failed to verify signature {} in database: {}",
                                            &signature[..8],
                                            e
                                        ),
                                    );
                                }
                            }
                        }
                    }

                    // Update metrics for fallback transaction
                    if let Some(manager_arc) = get_global_transaction_manager().await {
                        if let Ok(mgr) = manager_arc.try_lock() {
                            mgr.operations
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            mgr.rpc_fallback_fetched
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }

                    new_count += 1;
                }
                Err(e) => {
                    // Update error metrics
                    if let Some(manager_arc) = get_global_transaction_manager().await {
                        if let Ok(mgr) = manager_arc.try_lock() {
                            mgr.errors
                                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }

                    logger::info(
                        LogTag::Transactions,
                        &format!(
                            "Failed to process fallback transaction {}: {}",
                            &signature[..8],
                            e
                        ),
                    );
                }
            }
        }
    }

    if new_count > 0 {
        let summary = if db_persistence_failures > 0 {
            format!(
                "Fallback check found {} new transactions ({} DB persistence issues)",
                new_count, db_persistence_failures
            )
        } else {
            format!("Fallback check found {new_count} new transactions")
        };

        logger::info(LogTag::Transactions, &summary);
    }

    Ok(new_count)
}
