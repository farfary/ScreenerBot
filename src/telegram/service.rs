//! Telegram service integration with ServiceManager
//!
//! Provides the Service trait implementation for the Telegram module.

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use crate::telegram::discovery;
use crate::telegram::notifier::{self, send_notification};
use crate::telegram::polling;
use crate::telegram::types::{BotState, Notification};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Notify, RwLock};
use tokio::task::JoinHandle;

/// Telegram service state
pub struct TelegramService {
    /// Whether the service is initialized
    initialized: Arc<AtomicBool>,
    /// Current bot state
    state: Arc<RwLock<BotState>>,
    /// Shutdown notifier
    shutdown: Arc<Notify>,
    /// Command handler running flag
    command_handler_running: Arc<AtomicBool>,
    /// Notification queue sender
    notification_sender: Option<mpsc::Sender<Notification>>,
}

impl TelegramService {
    pub fn new() -> Self {
        Self {
            initialized: Arc::new(AtomicBool::new(false)),
            state: Arc::new(RwLock::new(BotState::Disconnected)),
            shutdown: Arc::new(Notify::new()),
            command_handler_running: Arc::new(AtomicBool::new(false)),
            notification_sender: None,
        }
    }

    /// Get current bot state
    pub async fn get_state(&self) -> BotState {
        self.state.read().await.clone()
    }

    /// Check if the service is ready to send notifications
    pub fn can_send_notifications(&self) -> bool {
        self.notification_sender.is_some()
    }
}

impl Default for TelegramService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for TelegramService {
    fn name(&self) -> &'static str {
        "telegram"
    }

    fn priority(&self) -> i32 {
        50 // After core services, before trader
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![] // No dependencies
    }

    async fn initialize(&mut self) -> Result<(), String> {
        let config = with_config(|c| c.telegram.clone());

        if !config.enabled {
            logger::info(LogTag::Telegram, "Telegram service disabled in config");
            self.initialized.store(true, Ordering::SeqCst);
            *self.state.write().await = BotState::Disconnected;
            return Ok(());
        }

        if config.bot_token.is_empty() {
            logger::info(LogTag::Telegram, "No bot token configured");
            self.initialized.store(true, Ordering::SeqCst);
            *self.state.write().await = BotState::Disconnected;
            return Ok(());
        }

        // Determine initial state based on chat_id
        let new_state = if config.chat_id.is_empty() {
            logger::info(
                LogTag::Telegram,
                "No chat ID configured, bot ready for discovery",
            );
            BotState::Discovery
        } else {
            logger::info(LogTag::Telegram, "Bot configured and ready");
            BotState::Connected
        };

        *self.state.write().await = new_state;
        self.initialized.store(true, Ordering::SeqCst);

        // Initialize the notifier if we have a chat_id
        if !config.chat_id.is_empty() {
            if let Err(e) = notifier::init_notifier() {
                logger::warning(
                    LogTag::Telegram,
                    &format!("Failed to initialize notifier: {}", e),
                );
            }
        }

        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let config = with_config(|c| c.telegram.clone());

        if !config.enabled || config.bot_token.is_empty() {
            return Ok(vec![]);
        }

        let mut handles = Vec::new();

        // Create notification queue
        let (tx, mut rx) = mpsc::channel::<Notification>(100);
        self.notification_sender = Some(tx.clone());
        notifier::set_notification_queue(tx);

        // Start notification sender worker (only if we have a chat_id)
        if !config.chat_id.is_empty() {
            let shutdown_clone = shutdown.clone();
            let sender_handle = tokio::spawn(monitor.instrument(async move {
                logger::info(LogTag::Telegram, "Notification sender worker started");

                loop {
                    tokio::select! {
                        Some(notification) = rx.recv() => {
                            send_notification(notification).await;
                        }
                        _ = shutdown_clone.notified() => {
                            logger::info(LogTag::Telegram, "Notification sender worker shutting down");
                            break;
                        }
                    }
                }
            }));
            handles.push(sender_handle);

            // Start command handler if enabled
            if config.commands_enabled {
                match polling::start_polling(shutdown.clone(), self.command_handler_running.clone())
                    .await
                {
                    Ok(handle) => handles.push(handle),
                    Err(e) => {
                        logger::warning(
                            LogTag::Telegram,
                            &format!("Failed to start command handler: {}", e),
                        );
                    }
                }
            }

            // Send startup notification
            let startup_notification = Notification::bot_started(
                crate::version::VERSION.to_string(),
                "Normal".to_string(),
            );
            send_notification(startup_notification).await;
        }

        self.shutdown = shutdown;

        Ok(handles)
    }

    async fn stop(&mut self) -> Result<(), String> {
        logger::info(LogTag::Telegram, "Telegram service shutting down");

        // Send shutdown notification if possible
        if notifier::is_enabled() {
            let shutdown_notification = Notification::bot_stopped("Graceful shutdown".to_string());
            send_notification(shutdown_notification).await;
        }

        // Signal shutdown - tasks will be awaited by ServiceManager
        self.shutdown.notify_waiters();

        logger::info(LogTag::Telegram, "Telegram service stopped");
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        let config = with_config(|c| c.telegram.clone());

        if !config.enabled {
            return ServiceHealth::Healthy; // Disabled is fine
        }

        if config.bot_token.is_empty() {
            return ServiceHealth::Healthy; // Not configured is fine
        }

        let state = self.state.read().await.clone();
        match state {
            BotState::Connected => ServiceHealth::Healthy,
            BotState::Discovery => ServiceHealth::Degraded("Discovery mode".to_string()),
            BotState::Disconnected => ServiceHealth::Unhealthy("Disconnected".to_string()),
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}

// ============================================================================
// GLOBAL SERVICE ACCESS
// ============================================================================

use once_cell::sync::Lazy;

static TELEGRAM_SERVICE: Lazy<RwLock<TelegramService>> =
    Lazy::new(|| RwLock::new(TelegramService::new()));

/// Get the global Telegram service (read-only)
pub async fn get_service() -> tokio::sync::RwLockReadGuard<'static, TelegramService> {
    TELEGRAM_SERVICE.read().await
}

/// Get the global Telegram service (mutable)
pub async fn get_service_mut() -> tokio::sync::RwLockWriteGuard<'static, TelegramService> {
    TELEGRAM_SERVICE.write().await
}

/// Check if Telegram is enabled and ready
pub async fn is_ready() -> bool {
    let service = TELEGRAM_SERVICE.read().await;
    service.initialized.load(Ordering::SeqCst)
}

/// Get current bot state
pub async fn get_bot_state() -> BotState {
    let service = TELEGRAM_SERVICE.read().await;
    service.get_state().await
}

/// Start discovery mode
pub async fn start_discovery_mode() -> Result<(), String> {
    // Update state
    let service = TELEGRAM_SERVICE.write().await;
    *service.state.write().await = BotState::Discovery;
    drop(service);

    // Start discovery
    discovery::start_discovery().await
}

/// Stop discovery mode
pub async fn stop_discovery_mode() {
    discovery::stop_discovery().await;

    // Update state based on config
    let service = TELEGRAM_SERVICE.write().await;
    let chat_id = with_config(|c| c.telegram.chat_id.clone());
    let new_state = if chat_id.is_empty() {
        BotState::Disconnected
    } else {
        BotState::Connected
    };
    *service.state.write().await = new_state;
}
