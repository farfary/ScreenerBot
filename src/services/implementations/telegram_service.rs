// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Telegram service — manages Telegram bot lifecycle, notifications, and command polling.

use crate::config::with_config;
use crate::i18n::{ids, UiText};
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use crate::telegram::notifier::{
    self, send_notification, send_notification_with, TelegramNotifier,
};
use crate::telegram::polling;
use crate::telegram::types::{BotState, Notification};
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch, Notify, RwLock};
use tokio::task::JoinHandle;

/// The longest the sender worker waits to announce the service's start or stop, so a
/// slow Telegram never holds a stop past the service manager's stop grace.
const ANNOUNCEMENT_TIMEOUT: Duration = Duration::from_secs(3);

/// Telegram service state
pub struct TelegramService {
    /// Whether the service is initialized
    initialized: Arc<AtomicBool>,
    /// Current bot state
    state: Arc<RwLock<BotState>>,
    /// This service's own stop signal. Its tasks end on it alone, so the service can stop
    /// while every other service keeps running; level-triggered, so a task busy with a
    /// request when the stop arrives still sees it.
    stop_signal: watch::Sender<bool>,
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
            stop_signal: watch::Sender::new(false),
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

    /// Off when the config turns Telegram off, so the service manager reports it
    /// `Disabled` like every other service its config disables, instead of a healthy
    /// service that never runs.
    fn is_enabled(&self) -> bool {
        with_config(|c| c.telegram.enabled)
    }

    /// Turning Telegram off in Settings stops the service at once, and new credentials
    /// restart it; its tasks end on the service's own stop signal.
    fn stops_alone(&self) -> bool {
        true
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        let config = with_config(|c| c.telegram.clone());

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
                    &format!("Failed to initialize notifier: {e}"),
                );
            }
        }

        Ok(())
    }

    async fn start(
        &mut self,
        _shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let config = with_config(|c| c.telegram.clone());

        if config.bot_token.is_empty() {
            return Ok(vec![]);
        }

        self.stop_signal = watch::Sender::new(false);

        let mut handles = Vec::new();

        // Create notification queue
        let (tx, mut rx) = mpsc::channel::<Notification>(100);
        self.notification_sender = Some(tx.clone());
        notifier::set_notification_queue(tx);

        // Start notification sender worker (only if we have a chat_id)
        if !config.chat_id.is_empty() {
            // The worker announces this start and, once stopped, the stop through the bot
            // and chat it started with; both sends run outside the service manager's lock.
            let announcer = TelegramNotifier::new(&config.bot_token, &config.chat_id).ok();
            let mut stop = self.stop_signal.subscribe();
            let sender_handle = tokio::spawn(monitor.instrument(async move {
                logger::info(LogTag::Telegram, "Notification sender worker started");
                if let Some(announcer) = &announcer {
                    let started = Notification::bot_started(
                        crate::version::VERSION.to_string(),
                        crate::telegram::types::StartMode::Normal,
                    );
                    let _ = tokio::time::timeout(
                        ANNOUNCEMENT_TIMEOUT,
                        send_notification_with(announcer, started),
                    )
                    .await;
                }

                loop {
                    tokio::select! {
                        Some(notification) = rx.recv() => {
                            send_notification(notification).await;
                        }
                        // The guard `wait_for` returns is not `Send`; drop it inside the
                        // branch future so the other branch can await.
                        _ = async { stop.wait_for(|stopped| *stopped).await.map(|_| ()) } => {
                            logger::info(LogTag::Telegram, "Notification sender worker shutting down");
                            break;
                        }
                    }
                }

                if let Some(announcer) = &announcer {
                    let stopped =
                        Notification::bot_stopped(crate::telegram::types::StopReason::Graceful);
                    let _ = tokio::time::timeout(
                        ANNOUNCEMENT_TIMEOUT,
                        send_notification_with(announcer, stopped),
                    )
                    .await;
                }
            }));
            handles.push(sender_handle);

            let stop = self.stop_signal.subscribe();
            let alert_monitor = monitor.clone();
            handles.push(tokio::spawn(alert_monitor.instrument(async move {
                crate::telegram::wallet_alerts::run(stop).await;
            })));

            // Start command handler if enabled
            if config.commands_enabled {
                match polling::start_polling(
                    self.stop_signal.subscribe(),
                    self.command_handler_running.clone(),
                )
                .await
                {
                    Ok(handle) => handles.push(handle),
                    Err(e) => {
                        logger::warning(
                            LogTag::Telegram,
                            &format!("Failed to start command handler: {e}"),
                        );
                    }
                }
            }
        }

        Ok(handles)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::info(LogTag::Telegram, "Telegram service shutting down");

        // End this service's tasks; the ServiceManager awaits them, and the sender worker
        // announces the stop on its way out. The shared notifier goes with them, so
        // nothing else is sent while the service is stopped.
        self.stop_signal.send_replace(true);
        notifier::clear_notifier();
        self.notification_sender = None;
        *self.state.write().await = BotState::Disconnected;

        logger::info(LogTag::Telegram, "Telegram service stopped");
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        let config = with_config(|c| c.telegram.clone());

        if config.bot_token.is_empty() {
            return ServiceHealth::Healthy; // Not configured is fine
        }

        let state = self.state.read().await.clone();
        match state {
            BotState::Connected => ServiceHealth::Healthy,
            BotState::Discovery => {
                ServiceHealth::Degraded(UiText::new(ids::SERVICES_HEALTH_TELEGRAM_DISCOVERY))
            }
            BotState::Disconnected => {
                ServiceHealth::Unhealthy(UiText::new(ids::SERVICES_HEALTH_TELEGRAM_DISCONNECTED))
            }
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}
