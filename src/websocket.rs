use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::arguments::is_debug_websocket_enabled;
use crate::logger::{log, LogTag};

/// WebSocket client for real-time Solana transaction monitoring
///
/// Features:
/// - Real-time transaction subscription via logs monitoring
/// - Automatic heartbeat/ping mechanism (30s interval) to prevent timeouts
/// - Proper ping/pong handling for connection keep-alive
/// - Exponential backoff reconnection strategy with attempt tracking
/// - Graceful shutdown handling with proper cleanup
/// - Debug logging for connection monitoring and troubleshooting
pub struct SolanaWebSocketClient {
    wallet_address: String,
    tx_sender: mpsc::UnboundedSender<String>, // Channel to send new transaction signatures
}

/// WebSocket subscription message for account changes
#[derive(Serialize)]
struct AccountSubscribe {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Vec<serde_json::Value>,
}

/// WebSocket notification response for account changes
#[derive(Deserialize, Debug)]
struct AccountNotification {
    jsonrpc: String,
    method: Option<String>,
    params: Option<AccountNotificationParams>,
}

#[derive(Deserialize, Debug)]
struct AccountNotificationParams {
    result: Option<AccountChangeResult>,
    subscription: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct AccountChangeResult {
    context: Option<serde_json::Value>,
    value: Option<AccountChangeValue>,
}

#[derive(Deserialize, Debug)]
struct AccountChangeValue {
    account: Option<serde_json::Value>,
    pubkey: Option<String>,
}

/// WebSocket subscription for signature notifications (better for transaction monitoring)
#[derive(Serialize)]
struct SignatureSubscribe {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Vec<serde_json::Value>,
}

/// Signature notification for confirmed transactions
#[derive(Deserialize, Debug)]
struct SignatureNotification {
    jsonrpc: String,
    method: Option<String>,
    params: Option<SignatureNotificationParams>,
}

#[derive(Deserialize, Debug)]
struct SignatureNotificationParams {
    result: Option<SignatureResult>,
    subscription: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct SignatureResult {
    context: Option<serde_json::Value>,
    value: Option<serde_json::Value>,
}

impl SolanaWebSocketClient {
    /// Create new WebSocket client for monitoring wallet transactions
    pub fn new(wallet_address: String) -> (Self, mpsc::UnboundedReceiver<String>) {
        let (tx_sender, tx_receiver) = mpsc::unbounded_channel();

        let client = Self {
            wallet_address,
            tx_sender,
        };

        (client, tx_receiver)
    }

    /// Start WebSocket connection and monitor for new transactions
    pub async fn start_monitoring(
        &self,
        ws_url: &str,
        shutdown: Arc<Notify>,
    ) -> Result<(), String> {
        if is_debug_websocket_enabled() {
            log(
                LogTag::Websocket,
                "START",
                &format!(
                    "🔌 Starting WebSocket monitoring for wallet: {}",
                    &self.wallet_address
                ),
            );
        }

        // Connect to WebSocket endpoint
        let (ws_stream, _) = connect_async(ws_url)
            .await
            .map_err(|e| format!("Failed to connect to WebSocket: {}", e))?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to logs for the wallet address to catch all transactions
        // This method catches both incoming and outgoing transactions
        let subscribe_message = AccountSubscribe {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "logsSubscribe".to_string(),
            params: vec![
                serde_json::json!({
                    "mentions": [self.wallet_address]
                }),
                serde_json::json!({
                    "commitment": "confirmed"
                }),
            ],
        };

        let subscribe_text = serde_json::to_string(&subscribe_message)
            .map_err(|e| format!("Failed to serialize subscription: {}", e))?;

        if is_debug_websocket_enabled() {
            log(
                LogTag::Websocket,
                "SUBSCRIBE",
                &format!(
                    "📡 Subscribing to logs for wallet: {}",
                    &self.wallet_address
                ),
            );
        }

        // Send subscription
        ws_sender
            .send(Message::Text(subscribe_text))
            .await
            .map_err(|e| format!("Failed to send subscription: {}", e))?;

        // Create heartbeat timer (ping every 30 seconds to prevent server timeout)
        let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        heartbeat_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        if is_debug_websocket_enabled() {
            log(
                LogTag::Websocket,
                "HEARTBEAT",
                "📡 Heartbeat timer initialized (30s interval)",
            );
        }

        // Listen for messages with shutdown and heartbeat handling
        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    if is_debug_websocket_enabled() {
                        log(LogTag::Websocket, "SHUTDOWN", "WebSocket monitoring received shutdown signal");
                    }

                    // Send close message to server
                    if let Err(e) = ws_sender.send(Message::Close(None)).await {
                        if is_debug_websocket_enabled() {
                            log(LogTag::Websocket, "CLOSE_ERROR", &format!("Failed to send close message: {}", e));
                        }
                    }

                    break;
                }
                _ = heartbeat_interval.tick() => {
                    // Send periodic ping to keep connection alive
                    if let Err(e) = ws_sender.send(Message::Ping(vec![])).await {
                        if is_debug_websocket_enabled() {
                            log(LogTag::Websocket, "HEARTBEAT_ERROR", &format!("Failed to send heartbeat ping: {}", e));
                        }
                        break; // Connection failed, exit to trigger reconnect
                    } else if is_debug_websocket_enabled() {
                        log(LogTag::Websocket, "HEARTBEAT", "💓 Sent heartbeat ping to keep connection alive");
                    }
                }
                message = ws_receiver.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = self.handle_websocket_message(&text).await {
                                if is_debug_websocket_enabled() {
                                    log(
                                        LogTag::Websocket,
                                        "ERROR",
                                        &format!("Failed to handle WebSocket message: {}", e)
                                    );
                                }
                            }
                        }
                        Some(Ok(Message::Ping(payload))) => {
                            // Respond to server ping with pong to keep connection alive
                            if let Err(e) = ws_sender.send(Message::Pong(payload)).await {
                                if is_debug_websocket_enabled() {
                                    log(LogTag::Websocket, "PONG_ERROR", &format!("Failed to respond to ping: {}", e));
                                }
                                break; // Connection failed, exit to trigger reconnect
                            } else if is_debug_websocket_enabled() {
                                log(LogTag::Websocket, "PONG", "🏓 Responded to server ping with pong");
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {
                            // Server responded to our ping - connection is alive
                            if is_debug_websocket_enabled() {
                                log(LogTag::Websocket, "PONG_RECEIVED", "🏓 Received pong response - connection alive");
                            }
                        }
                        Some(Ok(Message::Close(_))) => {
                            if is_debug_websocket_enabled() {
                                log(LogTag::Websocket, "CLOSE", "WebSocket connection closed by server");
                            }
                            break;
                        }
                        Some(Ok(Message::Binary(_))) => {
                            // Ignore binary messages
                            if is_debug_websocket_enabled() {
                                log(LogTag::Websocket, "BINARY", "Received binary message (ignored)");
                            }
                        }
                        Some(Ok(Message::Frame(_))) => {
                            // Ignore raw frame messages (handled by tungstenite internally)
                            if is_debug_websocket_enabled() {
                                log(LogTag::Websocket, "FRAME", "Received raw frame message (ignored)");
                            }
                        }
                        Some(Err(e)) => {
                            if is_debug_websocket_enabled() {
                                log(LogTag::Websocket, "ERROR", &format!("WebSocket error: {}", e));
                            }
                            break;
                        }
                        None => {
                            if is_debug_websocket_enabled() {
                                log(LogTag::Websocket, "CLOSE", "WebSocket stream ended");
                            }
                            break;
                        }
                    }
                }
            }
        }

        if is_debug_websocket_enabled() {
            log(LogTag::Websocket, "STOP", "WebSocket monitoring stopped");
        }

        Ok(())
    }

    /// Handle incoming WebSocket messages and extract transaction signatures
    async fn handle_websocket_message(&self, message: &str) -> Result<(), String> {
        // Try to parse as a logs notification
        if let Ok(notification) = serde_json::from_str::<serde_json::Value>(message) {
            // Check if this is a logs notification
            if let Some(method) = notification.get("method").and_then(|v| v.as_str()) {
                if method == "logsNotification" {
                    if let Some(params) = notification.get("params") {
                        if let Some(result) = params.get("result") {
                            if let Some(value) = result.get("value") {
                                // Extract signature from the logs notification
                                if let Some(signature) =
                                    value.get("signature").and_then(|v| v.as_str())
                                {
                                    if is_debug_websocket_enabled() {
                                        log(
                                            LogTag::Websocket,
                                            "NEW_TX",
                                            &format!("🆕 New transaction detected: {}", signature),
                                        );
                                    }

                                    // Send signature to transaction processor
                                    if let Err(_) = self.tx_sender.send(signature.to_string()) {
                                        if is_debug_websocket_enabled() {
                                            log(
                                                LogTag::Websocket,
                                                "CHANNEL_ERROR",
                                                "Failed to send signature to processor - channel closed"
                                            );
                                        }
                                        return Err("Transaction channel closed".to_string());
                                    }

                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }

            // Check if this is a subscription confirmation
            if let Some(result) = notification.get("result") {
                if result.is_number() {
                    if is_debug_websocket_enabled() {
                        log(
                            LogTag::Websocket,
                            "SUBSCRIBED",
                            &format!("✅ WebSocket subscription confirmed: {}", result),
                        );
                    }
                    return Ok(());
                }
            }
        }

        // If we get here, it's likely a message we don't need to handle
        // (subscription confirmations, heartbeats, etc.)
        Ok(())
    }

    /// Get the default Solana WebSocket URL (mainnet)
    pub fn get_default_ws_url() -> String {
        "wss://api.mainnet-beta.solana.com/".to_string()
    }

    /// Get Helius WebSocket URL with API key
    pub fn get_helius_ws_url(api_key: &str) -> String {
        format!("wss://mainnet.helius-rpc.com/?api-key={}", api_key)
    }
}

/// Start WebSocket monitoring as a background task
pub async fn start_websocket_monitoring(
    wallet_address: String,
    ws_url: Option<String>,
    shutdown: Arc<Notify>,
) -> Result<mpsc::UnboundedReceiver<String>, String> {
    let (client, tx_receiver) = SolanaWebSocketClient::new(wallet_address.clone());

    let ws_url = ws_url.unwrap_or_else(|| SolanaWebSocketClient::get_default_ws_url());

    // Start monitoring in background task
    let monitoring_client = Arc::new(client);
    let ws_url_clone = ws_url.clone();
    let shutdown_clone = shutdown.clone();

    tokio::spawn(async move {
        let mut reconnect_attempts = 0u32;
        let max_reconnect_delay = 60; // Maximum delay of 60 seconds

        loop {
            // Check for shutdown before attempting connection
            tokio::select! {
                _ = shutdown_clone.notified() => {
                    if is_debug_websocket_enabled() {
                        log(LogTag::Websocket, "SHUTDOWN", "WebSocket background task received shutdown signal");
                    }
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    // Continue with connection attempt
                }
            }

            if is_debug_websocket_enabled() {
                log(
                    LogTag::Websocket,
                    "CONNECT",
                    &format!(
                        "🔄 Connecting to WebSocket: {} (attempt {})",
                        ws_url_clone,
                        reconnect_attempts + 1
                    ),
                );
            }

            // Create a shutdown signal for this connection attempt
            let connection_shutdown = Arc::new(Notify::new());
            let connection_shutdown_clone = connection_shutdown.clone();
            let main_shutdown_clone = shutdown_clone.clone();

            // Forward main shutdown to connection shutdown
            tokio::spawn(async move {
                main_shutdown_clone.notified().await;
                connection_shutdown_clone.notify_waiters();
            });

            match monitoring_client
                .start_monitoring(&ws_url_clone, connection_shutdown)
                .await
            {
                Ok(_) => {
                    // Normal exit (shutdown received) or successful long-running connection
                    reconnect_attempts = 0; // Reset attempt counter on successful connection
                    if is_debug_websocket_enabled() {
                        log(
                            LogTag::Websocket,
                            "NORMAL_EXIT",
                            "WebSocket monitoring exited normally",
                        );
                    }
                    break;
                }
                Err(e) => {
                    reconnect_attempts += 1;

                    // Exponential backoff: 2^attempt seconds, capped at max_reconnect_delay
                    let delay_seconds = std::cmp::min(
                        (2u64).pow(std::cmp::min(reconnect_attempts, 6)), // Cap at 2^6 = 64, but we'll limit to max_reconnect_delay
                        max_reconnect_delay,
                    );

                    if is_debug_websocket_enabled() {
                        log(
                            LogTag::Websocket,
                            "RECONNECT",
                            &format!(
                                "WebSocket disconnected: {} - Reconnecting in {}s (attempt {})",
                                e, delay_seconds, reconnect_attempts
                            ),
                        );
                    }

                    // Wait for calculated delay or shutdown signal
                    tokio::select! {
                        _ = shutdown_clone.notified() => {
                            if is_debug_websocket_enabled() {
                                log(LogTag::Websocket, "SHUTDOWN_DURING_WAIT", "Shutdown received during reconnection wait");
                            }
                            break;
                        }
                        _ = tokio::time::sleep(tokio::time::Duration::from_secs(delay_seconds)) => {
                            // Continue reconnection loop
                        }
                    }
                }
            }
        }

        if is_debug_websocket_enabled() {
            log(
                LogTag::Websocket,
                "TASK_EXIT",
                "WebSocket background task exiting",
            );
        }
    });

    Ok(tx_receiver)
}
