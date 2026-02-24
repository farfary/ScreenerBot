// WebSocket integration for real-time transaction notifications

use chrono::Utc;
use std::sync::Arc;

use crate::logger::{self, LogTag};
use crate::transactions::{
    processor::TransactionProcessor,
    utils::{
        add_pending_transaction_globally, add_signature_to_known_globally,
        remove_pending_transaction_globally,
    },
    websocket,
};

use super::config::{defer_transaction_retry, ServiceConfig};
use super::lifecycle::{get_global_transaction_manager, SHUTDOWN_NOTIFY};

// =============================================================================
// WEBSOCKET INTEGRATION
// =============================================================================

/// Initialize WebSocket monitoring for real-time transaction notifications
pub async fn initialize_websocket_monitoring(
    wallet_pubkey: solana_sdk::pubkey::Pubkey,
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
    logger::info(
        LogTag::Transactions,
        &format!("Initializing WebSocket monitoring (url: {ws_url_log})"),
    );

    let receiver = websocket::start_websocket_monitoring(
        wallet_pubkey.to_string(),
        ws_url,
        SHUTDOWN_NOTIFY.clone(),
    )
    .await?;

    Ok(Some(receiver))
}

/// Handle transaction notification from WebSocket
/// Enhanced with deferral queue for RPC indexing delays
pub async fn handle_websocket_transaction(
    config: &ServiceConfig,
    processor: &Arc<TransactionProcessor>,
    signature: String,
) -> Result<(), String> {
    logger::info(
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
