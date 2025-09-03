use serde::{ Deserialize, Serialize };
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_tungstenite::{ connect_async, tungstenite::Message };
use futures_util::{ SinkExt, StreamExt };

use crate::logger::{ log, LogTag };
use crate::arguments::is_debug_websocket_enabled;

/// WebSocket client for real-time Solana transaction monitoring
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
    pub async fn start_monitoring(&self, ws_url: &str) -> Result<(), String> {
        if is_debug_websocket_enabled() {
            log(
                LogTag::Websocket,
                "START",
                &format!(
                    "🔌 Starting WebSocket monitoring for wallet: {}",
                    &self.wallet_address[..8]
                )
            );
        }

        // Connect to WebSocket endpoint
        let (ws_stream, _) = connect_async(ws_url).await.map_err(|e|
            format!("Failed to connect to WebSocket: {}", e)
        )?;

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
                })
            ],
        };

        let subscribe_text = serde_json
            ::to_string(&subscribe_message)
            .map_err(|e| format!("Failed to serialize subscription: {}", e))?;

        if is_debug_websocket_enabled() {
            log(
                LogTag::Websocket,
                "SUBSCRIBE",
                &format!("📡 Subscribing to logs for wallet: {}", &self.wallet_address[..8])
            );
        }

        // Send subscription
        ws_sender
            .send(Message::Text(subscribe_text)).await
            .map_err(|e| format!("Failed to send subscription: {}", e))?;

        // Listen for messages
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
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
                Ok(Message::Close(_)) => {
                    if is_debug_websocket_enabled() {
                        log(LogTag::Websocket, "CLOSE", "WebSocket connection closed by server");
                    }
                    break;
                }
                Ok(_) => {
                    // Ignore other message types (binary, ping, pong)
                }
                Err(e) => {
                    if is_debug_websocket_enabled() {
                        log(LogTag::Websocket, "ERROR", &format!("WebSocket error: {}", e));
                    }
                    break;
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
                                if
                                    let Some(signature) = value
                                        .get("signature")
                                        .and_then(|v| v.as_str())
                                {
                                    if is_debug_websocket_enabled() {
                                        log(
                                            LogTag::Websocket,
                                            "NEW_TX",
                                            &format!(
                                                "🆕 New transaction detected: {}",
                                                &signature[..8]
                                            )
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
                            &format!("✅ WebSocket subscription confirmed: {}", result)
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
    ws_url: Option<String>
) -> Result<mpsc::UnboundedReceiver<String>, String> {
    let (client, tx_receiver) = SolanaWebSocketClient::new(wallet_address.clone());

    let ws_url = ws_url.unwrap_or_else(|| SolanaWebSocketClient::get_default_ws_url());

    // Start monitoring in background task
    let monitoring_client = Arc::new(client);
    let ws_url_clone = ws_url.clone();

    tokio::spawn(async move {
        loop {
            if is_debug_websocket_enabled() {
                log(
                    LogTag::Websocket,
                    "CONNECT",
                    &format!("🔄 Connecting to WebSocket: {}", ws_url_clone)
                );
            }

            if let Err(e) = monitoring_client.start_monitoring(&ws_url_clone).await {
                if is_debug_websocket_enabled() {
                    log(
                        LogTag::Websocket,
                        "RECONNECT",
                        &format!("WebSocket disconnected: {} - Reconnecting in 5 seconds", e)
                    );
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                continue;
            }

            // If we exit normally, wait before reconnecting
            if is_debug_websocket_enabled() {
                log(LogTag::Websocket, "RESTART_DELAY", "Reconnecting in 2 seconds (normal exit)");
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    });

    Ok(tx_receiver)
}
