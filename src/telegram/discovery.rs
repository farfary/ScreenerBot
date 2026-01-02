//! Chat discovery service for Telegram
//!
//! Provides independent polling for discovering chat IDs.
//! This service can run WITHOUT a configured chat_id, allowing users
//! to message the bot and have their chat ID automatically discovered.

use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::telegram::session::get_session_manager;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// Discovery service for finding chat IDs
pub struct DiscoveryService {
    /// Whether discovery is currently running
    running: Arc<AtomicBool>,
    /// Shutdown signal for the polling task
    shutdown: Arc<Notify>,
    /// Handle to the polling task
    task_handle: Option<JoinHandle<()>>,
    /// Last update offset to avoid processing duplicates
    last_update_offset: Arc<AtomicI64>,
}

impl DiscoveryService {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            shutdown: Arc::new(Notify::new()),
            task_handle: None,
            last_update_offset: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Check if discovery is currently running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Start the discovery polling service
    /// 
    /// This starts a background task that polls for Telegram updates
    /// and captures any incoming messages as discovered chats.
    pub async fn start(&mut self) -> Result<(), String> {
        if self.is_running() {
            return Err("Discovery is already running".to_string());
        }

        let bot_token = with_config(|c| c.telegram.bot_token.clone());
        if bot_token.is_empty() {
            return Err("Bot token is not configured".to_string());
        }

        // Validate the token
        let bot = Bot::new(&bot_token);
        match bot.get_me().await {
            Ok(me) => {
                logger::info(
                    LogTag::Telegram,
                    &format!(
                        "Discovery: Bot validated - @{} (ID: {})",
                        me.username.as_deref().unwrap_or("unknown"),
                        me.id
                    ),
                );
            }
            Err(e) => {
                return Err(format!("Invalid bot token: {}", e));
            }
        }

        // Set discovery active in session manager
        let manager = get_session_manager();
        manager.set_discovery_active(true);
        manager.clear_discovered_chats().await;

        self.running.store(true, Ordering::SeqCst);
        self.last_update_offset.store(0, Ordering::SeqCst);
        let running = self.running.clone();
        let shutdown = self.shutdown.clone();
        let offset = self.last_update_offset.clone();

        // Start polling task
        let handle = tokio::spawn(async move {
            logger::info(LogTag::Telegram, "Discovery polling started");

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        logger::info(LogTag::Telegram, "Discovery polling received shutdown signal");
                        break;
                    }
                    _ = discovery_poll(&bot, &offset) => {
                        // Continue polling
                    }
                }

                if !running.load(Ordering::SeqCst) {
                    break;
                }
            }

            running.store(false, Ordering::SeqCst);
            logger::info(LogTag::Telegram, "Discovery polling stopped");
        });

        self.task_handle = Some(handle);
        Ok(())
    }

    /// Stop the discovery service
    pub async fn stop(&mut self) {
        if !self.is_running() {
            return;
        }

        self.running.store(false, Ordering::SeqCst);
        self.shutdown.notify_waiters();

        // Wait for task to complete with 5 second timeout to prevent hanging
        if let Some(handle) = self.task_handle.take() {
            match tokio::time::timeout(Duration::from_secs(5), handle).await {
                Ok(_) => {}
                Err(_) => {
                    logger::warning(
                        LogTag::Telegram,
                        "Discovery task did not complete within 5s timeout",
                    );
                }
            }
        }

        // Deactivate discovery in session manager
        let manager = get_session_manager();
        manager.set_discovery_active(false);

        logger::info(LogTag::Telegram, "Discovery service stopped");
    }

    /// Select a discovered chat and save to config
    pub async fn select_chat(&self, chat_id: i64) -> Result<(), String> {
        let manager = get_session_manager();

        // Find the discovered chat
        let chat = manager
            .select_discovered_chat(chat_id)
            .await
            .ok_or("Chat not found in discovered list")?;

        // Update config with the selected chat_id
        let chat_id_str = chat_id.to_string();
        update_config_section(|cfg| {
            cfg.telegram.chat_id = chat_id_str;
        }, true)?;

        logger::info(
            LogTag::Telegram,
            &format!(
                "Selected chat: {} (ID: {}, type: {})",
                chat.first_name.as_deref().unwrap_or("Unknown"),
                chat_id,
                chat.chat_type
            ),
        );

        Ok(())
    }
}

impl Default for DiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

/// Poll for discovery updates
async fn discovery_poll(bot: &Bot, offset: &Arc<AtomicI64>) {
    // Use getUpdates with offset to avoid processing duplicates
    let current_offset = offset.load(Ordering::SeqCst);
    let mut request = bot.get_updates().timeout(10);
    if current_offset > 0 {
        request = request.offset(current_offset as i32);
    }
    match request.await {
        Ok(updates) => {
            for update in updates {
                // Update offset to next update ID to avoid reprocessing
                offset.store(update.id.0 as i64 + 1, Ordering::SeqCst);

                if let teloxide::types::UpdateKind::Message(message) = update.kind {
                    // Extract user info
                    let (user_id, username, first_name) = match &message.from {
                        Some(from) => (
                            from.id.0 as i64,
                            from.username.clone(),
                            Some(from.first_name.clone()),
                        ),
                        None => continue, // Skip messages without sender
                    };

                    let chat_id = message.chat.id;
                    let manager = get_session_manager();

                    // Get chat type
                    let chat_type = match message.chat.kind {
                        teloxide::types::ChatKind::Private(_) => "private",
                        teloxide::types::ChatKind::Public(ref p) => match p.kind {
                            teloxide::types::PublicChatKind::Group(_) => "group",
                            teloxide::types::PublicChatKind::Supergroup(_) => "supergroup",
                            teloxide::types::PublicChatKind::Channel(_) => "channel",
                        },
                    };

                    // Get message preview (first 50 chars)
                    let message_preview = message.text().map(|t| {
                        if t.len() > 50 {
                            format!("{}...", &t[..47])
                        } else {
                            t.to_string()
                        }
                    });

                    // Add to discovered chats
                    let is_new = manager
                        .add_discovered_chat(
                            chat_id.0,
                            user_id,
                            username.clone(),
                            first_name.clone(),
                            chat_type.to_string(),
                            message_preview,
                        )
                        .await;

                    // Send acknowledgment if this is a new chat
                    if is_new {
                        let chat_name = first_name.as_deref().unwrap_or("User");
                        let ack_message = format!(
                            "👋 Hello {}!\n\n\
                            ✅ <b>Chat detected!</b>\n\n\
                            Chat ID: <code>{}</code>\n\
                            Type: {}\n\n\
                            Please go to the ScreenerBot dashboard and click on this chat to select it.",
                            chat_name, chat_id.0, chat_type
                        );

                        let _ = bot
                            .send_message(chat_id, ack_message)
                            .parse_mode(ParseMode::Html)
                            .await;

                        logger::info(
                            LogTag::Telegram,
                            &format!(
                                "Discovered chat: {} ({}) - type: {}",
                                chat_name, chat_id.0, chat_type
                            ),
                        );
                    }
                }
            }
        }
        Err(e) => {
            // Log error but don't stop polling
            logger::debug(
                LogTag::Telegram,
                &format!("Discovery poll error (will retry): {}", e),
            );
            // Brief pause before retry
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}

// ============================================================================
// GLOBAL DISCOVERY SERVICE
// ============================================================================

use once_cell::sync::Lazy;
use tokio::sync::RwLock;

static DISCOVERY_SERVICE: Lazy<RwLock<DiscoveryService>> =
    Lazy::new(|| RwLock::new(DiscoveryService::new()));

/// Start the discovery service
pub async fn start_discovery() -> Result<(), String> {
    let mut service = DISCOVERY_SERVICE.write().await;
    service.start().await
}

/// Stop the discovery service
pub async fn stop_discovery() {
    let mut service = DISCOVERY_SERVICE.write().await;
    service.stop().await;
}

/// Check if discovery is running
pub async fn is_discovery_running() -> bool {
    let service = DISCOVERY_SERVICE.read().await;
    service.is_running()
}

/// Select a discovered chat and save to config
pub async fn select_discovered_chat(chat_id: i64) -> Result<(), String> {
    let service = DISCOVERY_SERVICE.read().await;
    service.select_chat(chat_id).await
}

/// Get all discovered chats
pub async fn get_discovered_chats() -> Vec<crate::telegram::types::DiscoveredChat> {
    let manager = get_session_manager();
    manager.get_discovered_chats().await
}

/// Clear all discovered chats
pub async fn clear_discovered_chats() {
    let manager = get_session_manager();
    manager.clear_discovered_chats().await;
}
