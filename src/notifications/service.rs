//! Notification service for ScreenerBot
//!
//! Provides a background service for sending Telegram notifications.
//! Handles message queueing, rate limiting, and graceful shutdown.

use super::telegram::{start_command_handler, TelegramNotifier};
use super::types::Notification;
use crate::config::with_config;
use crate::logger::{self, LogTag};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Notify, RwLock};

/// Global notification service instance
static NOTIFICATION_SERVICE: std::sync::LazyLock<RwLock<Option<NotificationService>>> =
    std::sync::LazyLock::new(|| RwLock::new(None));

/// Notification service that manages Telegram integration
pub struct NotificationService {
    notifier: Option<TelegramNotifier>,
    sender: Option<mpsc::Sender<Notification>>,
    command_handler_running: Arc<AtomicBool>,
}

impl NotificationService {
    /// Create a new notification service
    pub async fn new() -> Self {
        let config = with_config(|cfg| cfg.telegram.clone());

        if !config.enabled {
            logger::info(
                LogTag::Notifications,
                "Telegram notifications disabled in config",
            );
            return Self {
                notifier: None,
                sender: None,
                command_handler_running: Arc::new(AtomicBool::new(false)),
            };
        }

        // Create the notifier
        let notifier = match TelegramNotifier::new(&config.bot_token, &config.chat_id) {
            Ok(n) => {
                logger::info(LogTag::Notifications, "Telegram notifier initialized");
                Some(n)
            }
            Err(e) => {
                logger::error(
                    LogTag::Notifications,
                    &format!("Failed to create Telegram notifier: {}", e),
                );
                None
            }
        };

        Self {
            notifier,
            sender: None,
            command_handler_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if notifications are enabled
    pub fn is_enabled(&self) -> bool {
        self.notifier.is_some()
    }

    /// Send a notification
    pub async fn send(&self, notification: Notification) {
        if let Some(ref notifier) = self.notifier {
            // Check notification preferences
            if !self.should_send_notification(&notification) {
                logger::debug(
                    LogTag::Notifications,
                    "Notification filtered by preferences",
                );
                return;
            }

            if let Err(e) = notifier.send(&notification).await {
                logger::error(
                    LogTag::Notifications,
                    &format!("Failed to send notification: {}", e),
                );
            }
        }
    }

    /// Send a notification via the queue (for use from sync contexts)
    pub fn queue(&self, notification: Notification) {
        if let Some(ref sender) = self.sender {
            if sender.try_send(notification).is_err() {
                logger::warning(
                    LogTag::Notifications,
                    "Notification queue full, dropping message",
                );
            }
        }
    }

    /// Check if a notification should be sent based on config preferences
    fn should_send_notification(&self, notification: &Notification) -> bool {
        let config = with_config(|cfg| cfg.telegram.clone());

        match &notification.notification_type {
            super::types::NotificationType::TradeAlert { amount_sol, .. } => {
                config.notify_trade_alerts && *amount_sol >= config.trade_alert_min_sol
            }
            super::types::NotificationType::PositionOpened { .. } => config.notify_position_opened,
            super::types::NotificationType::PositionClosed { .. } => config.notify_position_closed,
            super::types::NotificationType::PartialExit { .. } => config.notify_position_closed,
            super::types::NotificationType::DcaExecuted { .. } => config.notify_position_opened,
            super::types::NotificationType::SystemError { severity, .. } => {
                match severity {
                    super::types::ErrorSeverity::Critical | super::types::ErrorSeverity::Error => {
                        config.notify_system_errors
                    }
                    super::types::ErrorSeverity::Warning => config.notify_system_errors,
                    super::types::ErrorSeverity::Info => false, // Don't send info level unless explicitly enabled
                }
            }
            super::types::NotificationType::DailySummary { .. } => config.notify_daily_summary,
            super::types::NotificationType::BotCommand { .. } => true, // Always send command responses
            super::types::NotificationType::BotStarted { .. } => config.notify_system_errors,
            super::types::NotificationType::BotStopped { .. } => config.notify_system_errors,
        }
    }

    /// Start the background notification worker
    pub async fn start_worker(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<tokio::task::JoinHandle<()>>, String> {
        let mut handles = Vec::new();

        if !self.is_enabled() {
            return Ok(handles);
        }

        // Create notification queue
        let (tx, mut rx) = mpsc::channel::<Notification>(100);
        self.sender = Some(tx);

        // Clone notifier for the worker
        let config = with_config(|cfg| cfg.telegram.clone());
        let notifier = TelegramNotifier::new(&config.bot_token, &config.chat_id)
            .map_err(|e| format!("Failed to create notifier for worker: {}", e))?;

        // Start notification sender worker
        let shutdown_clone = shutdown.clone();
        let sender_handle = tokio::spawn(monitor.instrument(async move {
            logger::info(LogTag::Notifications, "Notification sender worker started");

            loop {
                tokio::select! {
                    Some(notification) = rx.recv() => {
                        if let Err(e) = notifier.send(&notification).await {
                            logger::error(
                                LogTag::Notifications,
                                &format!("Failed to send queued notification: {}", e),
                            );
                        }
                    }
                    _ = shutdown_clone.notified() => {
                        logger::info(LogTag::Notifications, "Notification sender worker shutting down");
                        break;
                    }
                }
            }
        }));
        handles.push(sender_handle);

        // Start command handler if enabled
        if config.commands_enabled {
            let command_handle = start_command_handler(
                config.bot_token.clone(),
                config.chat_id.clone(),
                shutdown.clone(),
                self.command_handler_running.clone(),
            )
            .await?;
            handles.push(command_handle);
        }

        Ok(handles)
    }
}

/// Initialize the global notification service
pub async fn init_notification_service() {
    let service = NotificationService::new().await;
    let mut guard = NOTIFICATION_SERVICE.write().await;
    *guard = Some(service);
    logger::info(LogTag::Notifications, "Notification service initialized");
}

/// Start the notification service background workers
pub async fn start_notification_service(
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> Result<Vec<tokio::task::JoinHandle<()>>, String> {
    let mut guard = NOTIFICATION_SERVICE.write().await;
    if let Some(ref mut service) = *guard {
        service.start_worker(shutdown, monitor).await
    } else {
        Err("Notification service not initialized".to_string())
    }
}

/// Send a notification via the global service
pub async fn send_notification(notification: Notification) {
    let guard = NOTIFICATION_SERVICE.read().await;
    if let Some(ref service) = *guard {
        service.send(notification).await;
    }
}

/// Queue a notification (non-blocking, for use from sync contexts)
pub fn queue_notification(notification: Notification) {
    // Use tokio's try_write to avoid blocking
    if let Ok(guard) = NOTIFICATION_SERVICE.try_read() {
        if let Some(ref service) = *guard {
            service.queue(notification);
        }
    }
}

/// Check if the notification service is enabled
pub async fn is_notification_service_enabled() -> bool {
    let guard = NOTIFICATION_SERVICE.read().await;
    if let Some(ref service) = *guard {
        service.is_enabled()
    } else {
        false
    }
}

/// Get the notification service (for direct access if needed)
pub async fn get_notification_service() -> Option<tokio::sync::RwLockReadGuard<'static, Option<NotificationService>>> {
    let guard = NOTIFICATION_SERVICE.read().await;
    if guard.is_some() {
        Some(guard)
    } else {
        None
    }
}
