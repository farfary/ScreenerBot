// transactions/websocket.rs - Real-time transaction sync via WebSocket
use super::types::*;
use super::cache::TransactionDatabase;
use super::fetcher::{ TransactionFetcher, get_transactions_with_cache_and_fallback };
use crate::logger::{ log, LogTag };
use serde_json;
use tokio_tungstenite::{ connect_async, tungstenite::protocol::Message };
use futures_util::{ SinkExt, StreamExt };
use std::error::Error;
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::Mutex;

/// WebSocket client for real-time transaction monitoring
pub struct TransactionWebSocket {
    db: Arc<Mutex<TransactionDatabase>>,
    subscription_id: Option<u64>,
    wallet_addresses: Vec<String>,
}

impl TransactionWebSocket {
    /// Create a new WebSocket client
    pub fn new(wallet_addresses: Vec<String>) -> Result<Self, String> {
        let db = Arc::new(Mutex::new(TransactionDatabase::new().map_err(|e| e.to_string())?));

        Ok(Self {
            db,
            subscription_id: None,
            wallet_addresses,
        })
    }

    /// Start WebSocket connection and monitor transactions
    pub async fn start_monitoring(&mut self, rpc_ws_url: &str) -> Result<(), String> {
        log(LogTag::System, "INFO", &format!("Connecting to WebSocket: {}", rpc_ws_url));

        let (ws_stream, _) = connect_async(rpc_ws_url).await.map_err(|e| e.to_string())?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to account notifications for each wallet
        for wallet_address in &self.wallet_addresses {
            let subscription_request = self.create_account_subscription(wallet_address);
            ws_sender.send(Message::Text(subscription_request)).await.map_err(|e| e.to_string())?;
            log(
                LogTag::System,
                "INFO",
                &format!("Subscribed to account notifications for: {}", wallet_address)
            );

            // Also subscribe to logs to catch transaction signatures
            let logs_subscription = self.create_logs_subscription(wallet_address);
            ws_sender.send(Message::Text(logs_subscription)).await.map_err(|e| e.to_string())?;
            log(
                LogTag::System,
                "INFO",
                &format!("Subscribed to logs notifications for: {}", wallet_address)
            );
        }

        // Create channel for handling messages
        let (tx, mut rx) = mpsc::channel(100);
        let db_clone = self.db.clone();

        // Spawn message handler task
        let handler_task = tokio::spawn(async move {
            while let Some(ws_message) = rx.recv().await {
                if let Err(e) = Self::handle_websocket_message(&db_clone, ws_message).await {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Error handling WebSocket message: {}", e)
                    );
                }
            }
        });

        // Process WebSocket messages
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = tx.send(text).await {
                        log(
                            LogTag::System,
                            "ERROR",
                            &format!("Failed to send message to handler: {}", e)
                        );
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    log(LogTag::System, "INFO", "WebSocket connection closed");
                    break;
                }
                Err(e) => {
                    log(LogTag::System, "ERROR", &format!("WebSocket error: {}", e));
                    break;
                }
                _ => {}
            }
        }

        // Wait for handler to finish
        handler_task.abort();
        Ok(())
    }

    /// Create account subscription message
    pub fn create_account_subscription(&self, wallet_address: &str) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "accountSubscribe",
            "params": [
                wallet_address,
                {
                    "encoding": "jsonParsed",
                    "commitment": "confirmed"
                }
            ]
        }).to_string()
    }

    /// Create signature subscription message
    pub fn create_signature_subscription(&self, wallet_address: &str) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "signatureSubscribe",
            "params": [
                wallet_address,
                {
                    "commitment": "confirmed"
                }
            ]
        }).to_string()
    }

    /// Create logs subscription to monitor all transactions involving the wallet
    pub fn create_logs_subscription(&self, wallet_address: &str) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "logsSubscribe",
            "params": [
                {
                    "mentions": [wallet_address]
                },
                {
                    "commitment": "confirmed"
                }
            ]
        }).to_string()
    }

    /// Handle incoming WebSocket messages
    async fn handle_websocket_message(
        db: &Arc<Mutex<TransactionDatabase>>,
        message_text: String
    ) -> Result<(), Box<dyn Error>> {
        let ws_message: WebSocketMessage = serde_json::from_str(&message_text)?;

        match ws_message.method.as_deref() {
            Some("accountNotification") => {
                Self::handle_account_notification(db, ws_message).await?;
            }
            Some("signatureNotification") => {
                Self::handle_signature_notification(db, ws_message).await?;
            }
            Some("logsNotification") => {
                Self::handle_logs_notification(db, ws_message).await?;
            }
            _ => {
                // Handle subscription responses
                if let Some(result) = ws_message.result {
                    log(LogTag::System, "INFO", &format!("Subscription result: {:?}", result));
                }
            }
        }

        Ok(())
    }

    /// Handle account notification (balance changes)
    async fn handle_account_notification(
        db: &Arc<Mutex<TransactionDatabase>>,
        message: WebSocketMessage
    ) -> Result<(), Box<dyn Error>> {
        if let Some(params) = message.params {
            if let Some(notification) = params.get("result") {
                log(LogTag::System, "INFO", &format!("Account notification: {:?}", notification));

                // Extract account information and trigger transaction fetch
                if let Some(context) = notification.get("context") {
                    if let Some(slot) = context.get("slot").and_then(|s| s.as_u64()) {
                        // Update last seen slot for this account
                        let db_lock = db.lock().await;
                        // Note: We'd need to extend the database schema to track account-specific slots
                        log(LogTag::System, "INFO", &format!("Account updated at slot: {}", slot));
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle signature notification (new transactions)
    async fn handle_signature_notification(
        db: &Arc<Mutex<TransactionDatabase>>,
        message: WebSocketMessage
    ) -> Result<(), Box<dyn Error>> {
        if let Some(params) = message.params {
            if let Some(result) = params.get("result") {
                if let Some(signature) = result.get("signature").and_then(|s| s.as_str()) {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!("🎯 NEW TRANSACTION DETECTED via WebSocket: {}", signature)
                    );

                    // Increment notification counter (for debugging)
                    static NOTIFICATION_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(
                        0
                    );
                    let count =
                        NOTIFICATION_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;

                    log(
                        LogTag::System,
                        "INFO",
                        &format!(
                            "📊 WebSocket notification #{}: {} ready for database storage",
                            count,
                            signature
                        )
                    );
                }
            }
        }
        Ok(())
    }

    /// Handle logs notification (transaction logs with signatures)
    async fn handle_logs_notification(
        _db: &Arc<Mutex<TransactionDatabase>>,
        message: WebSocketMessage
    ) -> Result<(), Box<dyn Error>> {
        if let Some(params) = message.params {
            if let Some(result) = params.get("result") {
                if let Some(value) = result.get("value") {
                    if let Some(signature) = value.get("signature").and_then(|s| s.as_str()) {
                        log(
                            LogTag::System,
                            "SUCCESS",
                            &format!("🎯 TRANSACTION SIGNATURE from logs: {}", signature)
                        );

                        // Increment notification counter (for debugging)
                        static LOGS_NOTIFICATION_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(
                            0
                        );
                        let count =
                            LOGS_NOTIFICATION_COUNT.fetch_add(
                                1,
                                std::sync::atomic::Ordering::Relaxed
                            ) + 1;

                        log(
                            LogTag::System,
                            "INFO",
                            &format!("📊 Logs notification #{}: {} detected", count, signature)
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// Start background WebSocket monitoring task
    pub async fn start_background_monitoring(
        wallet_addresses: Vec<String>,
        rpc_ws_url: String
    ) -> Result<tokio::task::JoinHandle<()>, String> {
        let task = tokio::spawn(async move {
            loop {
                match TransactionWebSocket::new(wallet_addresses.clone()) {
                    Ok(mut ws_client) => {
                        log(LogTag::System, "INFO", "Starting WebSocket transaction monitoring...");

                        if let Err(e) = ws_client.start_monitoring(&rpc_ws_url).await {
                            log(
                                LogTag::System,
                                "ERROR",
                                &format!("WebSocket monitoring error: {}", e)
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::System,
                            "ERROR",
                            &format!("Failed to create WebSocket client: {}", e)
                        );
                    }
                }

                // Wait before reconnecting
                log(LogTag::System, "INFO", "Reconnecting WebSocket in 30 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            }
        });

        Ok(task)
    }

    /// Enhanced WebSocket client with signature monitoring
    pub async fn start_enhanced_monitoring(
        &mut self,
        rpc_ws_url: &str
    ) -> Result<(), Box<dyn Error>> {
        log(
            LogTag::System,
            "INFO",
            &format!("Starting enhanced WebSocket monitoring: {}", rpc_ws_url)
        );

        let (ws_stream, _) = connect_async(rpc_ws_url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to both account and signature notifications
        for wallet_address in &self.wallet_addresses {
            // Account subscription for balance changes
            let account_sub = self.create_account_subscription(wallet_address);
            ws_sender.send(Message::Text(account_sub)).await?;

            // Note: Signature subscription requires transaction signature, not wallet address
            // We'd need to modify this to subscribe to specific signatures or use logs subscription
            log(
                LogTag::System,
                "INFO",
                &format!("Enhanced subscription for wallet: {}", wallet_address)
            );
        }

        // Create enhanced message handler
        let (tx, mut rx) = mpsc::channel(1000);
        let db_clone = self.db.clone();
        let wallet_addresses_clone = self.wallet_addresses.clone();

        let handler_task = tokio::spawn(async move {
            while let Some(ws_message) = rx.recv().await {
                if
                    let Err(e) = Self::handle_enhanced_message(
                        &db_clone,
                        &wallet_addresses_clone,
                        ws_message
                    ).await
                {
                    log(LogTag::System, "ERROR", &format!("Enhanced handler error: {}", e));
                }
            }
        });

        // Process messages with enhanced handling
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = tx.send(text).await {
                        log(
                            LogTag::System,
                            "ERROR",
                            &format!("Failed to send to enhanced handler: {}", e)
                        );
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    log(LogTag::System, "INFO", "Enhanced WebSocket connection closed");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    // Respond to ping with pong
                    if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                        log(LogTag::System, "ERROR", &format!("Failed to send pong: {}", e));
                    }
                }
                Err(e) => {
                    log(LogTag::System, "ERROR", &format!("Enhanced WebSocket error: {}", e));
                    break;
                }
                _ => {}
            }
        }

        handler_task.abort();
        Ok(())
    }

    /// Enhanced message handler with better transaction detection
    async fn handle_enhanced_message(
        db: &Arc<Mutex<TransactionDatabase>>,
        wallet_addresses: &[String],
        message_text: String
    ) -> Result<(), Box<dyn Error>> {
        let ws_message: WebSocketMessage = serde_json::from_str(&message_text)?;

        match ws_message.method.as_deref() {
            Some("accountNotification") => {
                // Account balance changed - likely new transaction
                Self::handle_account_change_notification(db, wallet_addresses, ws_message).await?;
            }
            Some("logsNotification") => {
                // Transaction logs - can detect specific program interactions
                Self::handle_logs_notification(db, ws_message).await?;
            }
            _ => {
                if let Some(result) = ws_message.result {
                    if let Some(subscription_id) = result.as_u64() {
                        log(
                            LogTag::System,
                            "INFO",
                            &format!("Subscription ID: {}", subscription_id)
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle account balance change notifications
    async fn handle_account_change_notification(
        db: &Arc<Mutex<TransactionDatabase>>,
        wallet_addresses: &[String],
        message: WebSocketMessage
    ) -> Result<(), Box<dyn Error>> {
        if let Some(params) = message.params {
            if let Some(result) = params.get("result") {
                if let Some(context) = result.get("context") {
                    if let Some(slot) = context.get("slot").and_then(|s| s.as_u64()) {
                        log(
                            LogTag::System,
                            "INFO",
                            &format!("Account balance changed at slot: {}", slot)
                        );

                        // Trigger incremental sync for affected wallets
                        // This could be implemented by adding a "sync_queue" table
                        for wallet_address in wallet_addresses {
                            log(
                                LogTag::System,
                                "INFO",
                                &format!("Queuing sync for wallet: {}", wallet_address)
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// WebSocket configuration
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub rpc_ws_url: String,
    pub reconnect_delay_secs: u64,
    pub ping_interval_secs: u64,
    pub max_reconnect_attempts: u32,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            rpc_ws_url: "wss://api.mainnet-beta.solana.com".to_string(),
            reconnect_delay_secs: 30,
            ping_interval_secs: 30,
            max_reconnect_attempts: 10,
        }
    }
}

/// WebSocket manager for multiple connections
pub struct WebSocketManager {
    config: WebSocketConfig,
    active_connections: Vec<tokio::task::JoinHandle<()>>,
}

impl WebSocketManager {
    /// Create a new WebSocket manager
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            config,
            active_connections: Vec::new(),
        }
    }

    /// Start monitoring multiple wallets
    pub async fn start_monitoring_wallets(
        &mut self,
        wallet_addresses: Vec<String>
    ) -> Result<(), Box<dyn Error>> {
        log(
            LogTag::System,
            "INFO",
            &format!("Starting WebSocket monitoring for {} wallets", wallet_addresses.len())
        );

        let task = TransactionWebSocket::start_background_monitoring(
            wallet_addresses,
            self.config.rpc_ws_url.clone()
        ).await?;

        self.active_connections.push(task);
        Ok(())
    }

    /// Stop all active connections
    pub async fn stop_all(&mut self) {
        log(LogTag::System, "INFO", "Stopping all WebSocket connections...");

        for task in self.active_connections.drain(..) {
            task.abort();
        }

        log(LogTag::System, "INFO", "All WebSocket connections stopped");
    }

    /// Get connection status
    pub fn get_connection_count(&self) -> usize {
        self.active_connections.len()
    }
}
