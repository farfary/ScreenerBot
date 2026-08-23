//! Transaction service processing — own-wallet consumer of the shared wallet watch
//! service, plus periodic housekeeping.
//!
//! Detection (WebSocket `logsNotification` + poll fallback + gap-fill) is owned
//! entirely by `wallets::watch` now: one funnel, three triggers, shared by every
//! subject including this one (§6.5 of the wallet-observation plan). This service
//! no longer creates a WebSocket of its own -- it subscribes to `wallets::watch::
//! subscribe_activity()`, filters to its own address, and reacts. What genuinely
//! stays own-wallet-specific is periodic expired-pending cleanup. RPC indexing
//! delays are retained and retried durably by the wallet-watch funnel itself.

use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::interval;

use crate::logger::{self, LogTag};
use crate::transactions::utils::{
    add_signature_to_known_globally, cleanup_expired_pending_transactions,
};
use crate::wallets::watch::{subscribe_activity, WalletActivity};

use super::config::ServiceConfig;
use super::health::{perform_health_check, ServiceMetrics};
use super::lifecycle::{get_global_transaction_manager, SHUTDOWN_NOTIFY};

// =============================================================================
// MAIN SERVICE LOOP
// =============================================================================

/// Main service loop: react to own-wallet activity, run periodic housekeeping.
pub async fn run_transaction_service(config: ServiceConfig) -> crate::transactions::Result<()> {
    logger::info(
        LogTag::Transactions,
        &format!(
            "Transaction service running for wallet: {} (housekeeping; \
             detection via wallets::watch, interval: {}s)",
            config.subject.address(),
            config.check_interval_secs
        ),
    );

    let own_address = config.subject.address();

    let mut check_interval = interval(Duration::from_secs(config.check_interval_secs));
    let mut activity_rx = subscribe_activity();
    let mut metrics = ServiceMetrics::new();

    loop {
        tokio::select! {
            // Periodic housekeeping (every few seconds by default)
            _ = check_interval.tick() => {
                if let Err(e) = perform_periodic_check(&mut metrics).await {
                    logger::warning(
                        LogTag::Transactions,
                        &format!("Periodic check failed: {e}")
                    );
                }
            }

            // The own wallet's slice of the shared wallet-watch activity feed
            activity = activity_rx.recv() => {
                match activity {
                    Ok(activity) if activity.subject == own_address => {
                        handle_own_wallet_activity(&config, &mut metrics, &activity).await;
                    }
                    Ok(_) => {
                        // Some other subject's activity -- not ours, ignore.
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        // Bounded broadcast channel: this consumer fell behind and
                        // missed `skipped` messages rather than stalling the
                        // pipeline for everyone else. The database/known-signatures
                        // bookkeeping those messages would have driven is still
                        // caught by the next periodic deferred-retry pass and by
                        // `is_signature_known_globally` falling through to the
                        // database, so nothing is lost -- just delayed.
                        metrics.increment_error();
                        logger::warning(
                            LogTag::Transactions,
                            &format!("Own-wallet activity consumer lagged by {skipped} messages")
                        );
                    }
                    Err(RecvError::Closed) => {
                        // The wallet watch service is a registered dependency and
                        // should outlive this loop; seeing its channel close means
                        // it is shutting down too. Wait for our own shutdown signal
                        // rather than busy-looping on a closed channel.
                        logger::warning(LogTag::Transactions, "Wallet watch activity channel closed");
                        tokio::time::sleep(Duration::from_secs(5)).await;
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

/// React to one piece of confirmed activity for our own wallet: update the
/// bookkeeping the dashboard's transaction stats read. Recording (the processed row,
/// the events feed) and dedupe already happened inside the wallet-watch funnel
/// before this was broadcast -- this is purely local counters.
async fn handle_own_wallet_activity(
    config: &ServiceConfig,
    metrics: &mut ServiceMetrics,
    activity: &WalletActivity,
) {
    let subject = config.subject.clone();
    add_signature_to_known_globally(subject, activity.signature.clone()).await;
    metrics.update_activity();

    if let Some(manager_arc) = get_global_transaction_manager().await {
        if let Ok(mgr) = manager_arc.try_lock() {
            mgr.operations
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            mgr.websocket_received
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    logger::debug(
        LogTag::Transactions,
        &format!(
            "Own-wallet activity: {} ({:?})",
            activity.signature, activity.kind
        ),
    );

    // The wallet just moved, so what it holds may no longer match what the positions
    // table says: a token sold in another wallet app has to close its position here too.
    // Debounced and single-flight inside the ledger; this call never blocks.
    crate::positions::ledger::schedule_resync();
}

// =============================================================================
// PERIODIC PROCESSING
// =============================================================================

/// Perform periodic housekeeping: expired-pending cleanup and deferred-retry
/// processing. No detection work happens here any more -- see the module doc.
async fn perform_periodic_check(metrics: &mut ServiceMetrics) -> crate::transactions::Result<()> {
    let start_time = std::time::Instant::now();

    let expired_count = cleanup_expired_pending_transactions().await;
    let duration = start_time.elapsed();
    metrics.update_periodic_check(duration, expired_count, 0);

    Ok(())
}
