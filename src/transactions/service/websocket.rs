//! Transaction service WebSocket — establishes and maintains WebSocket subscriptions.
//!
//! Wallet log notifications are delivered through the shared `crate::rpc::subscriptions`
//! transport (one connection, multiplexed across every subscribed address). This module
//! owns nothing about the transport itself; it only turns a notification into
//! transaction handling and the side effects specific to our own wallet.

use chrono::Utc;
use std::sync::Arc;

use crate::logger::{self, LogTag};
use crate::transactions::{
    processor::TransactionProcessor,
    utils::{
        add_pending_transaction_globally, add_signature_to_known_globally,
        remove_pending_transaction_globally,
    },
};

use super::config::{defer_transaction_retry, ServiceConfig};
use super::lifecycle::get_global_transaction_manager;

// =============================================================================
// WEBSOCKET INTEGRATION
// =============================================================================

/// Subscribe to the wallet's log notifications through the shared transport.
pub async fn initialize_websocket_monitoring(
    wallet_pubkey: solana_sdk::pubkey::Pubkey,
) -> Result<Option<crate::rpc::LogsSubscription>, String> {
    logger::info(
        LogTag::Transactions,
        "Subscribing to wallet log notifications",
    );

    crate::rpc::subscribe_logs_mentions(&wallet_pubkey.to_string())
        .await
        .map(Some)
        .map_err(|e| e.to_string())
}

/// Handle one log notification for our wallet.
pub async fn handle_websocket_transaction(
    config: &ServiceConfig,
    processor: &Arc<TransactionProcessor>,
    event: crate::rpc::SubscriptionEvent,
) -> Result<(), String> {
    let signature = event.signature;

    // The wallet just changed on-chain. This subscription mentions the wallet, so it
    // sees everything -- our own swaps and anything the owner does from another app --
    // which makes it the one place that can keep the displayed worth honest. Refresh
    // the balance now rather than waiting out the snapshot interval (coalesced, so a
    // burst costs one refresh). This lives here, in the own-wallet consumer, and not
    // in the transport: a notification for some other watched address must never
    // refresh our worth.
    crate::wallet::request_balance_refresh();

    logger::debug(
        LogTag::Transactions,
        &format!("Processing WebSocket transaction: {}", &signature),
    );

    // Add to pending transactions for monitoring
    add_pending_transaction_globally(signature.clone(), Utc::now()).await;

    // Process the transaction
    match processor.process_transaction(&signature).await {
        Ok(_) => {
            add_signature_to_known_globally(signature.clone()).await;
            remove_pending_transaction_globally(&signature).await;

            // Update metrics for WebSocket transaction
            if let Some(manager_arc) = get_global_transaction_manager().await {
                if let Ok(mgr) = manager_arc.try_lock() {
                    mgr.operations
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    mgr.websocket_received
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }

            logger::info(
                LogTag::Transactions,
                &format!(
                    "Successfully processed WebSocket transaction: {}",
                    &signature
                ),
            );
        }
        Err(e) => {
            // Update error metrics
            if let Some(manager_arc) = get_global_transaction_manager().await {
                if let Ok(mgr) = manager_arc.try_lock() {
                    mgr.errors
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }

            // Check if this is an RPC indexing delay (temporary, should be retried later)
            if crate::errors::is_rpc_indexing_delay(&e) || e.contains("RPC indexing delay") {
                // Defer this transaction for retry after 5 seconds
                defer_transaction_retry(signature.clone(), 5, e.clone()).await;

                logger::info(
                    LogTag::Transactions,
                    &format!(
                        "Deferred {} for RPC indexing (will retry in 5s)",
                        &signature[..8]
                    ),
                );
            } else {
                // Log genuine errors
                logger::info(
                    LogTag::Transactions,
                    &format!(
                        "Failed to process WebSocket transaction {}: {}",
                        &signature[..8],
                        e
                    ),
                );
            }
        }
    }

    Ok(())
}
