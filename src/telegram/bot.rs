//! Telegram bot instance management
//!
//! Handles bot creation, connection, and lifecycle management.

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::telegram::types::BotState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};
use tokio::sync::{Notify, RwLock};

/// Telegram bot wrapper with state management
pub struct TelegramBot {
    /// The underlying teloxide Bot instance
    bot: Option<Bot>,
    /// Current bot state
    state: RwLock<BotState>,
    /// Whether the bot is currently polling for updates
    polling_active: AtomicBool,
    /// Shutdown signal
    shutdown: Arc<Notify>,
}

impl TelegramBot {
    /// Create a new TelegramBot instance
    pub fn new() -> Self {
        Self {
            bot: None,
            state: RwLock::new(BotState::Disconnected),
            polling_active: AtomicBool::new(false),
            shutdown: Arc::new(Notify::new()),
        }
    }

    /// Initialize the bot with the configured token
    pub async fn initialize(&mut self) -> Result<(), String> {
        let token = with_config(|c| c.telegram.bot_token.clone());

        if token.is_empty() {
            logger::info(LogTag::Telegram, "No bot token configured, bot disabled");
            return Ok(());
        }

        // Validate token by calling getMe
        let bot = Bot::new(&token);
        match bot.get_me().await {
            Ok(me) => {
                logger::info(
                    LogTag::Telegram,
                    &format!(
                        "Bot initialized: @{} (ID: {})",
                        me.username.as_deref().unwrap_or("unknown"),
                        me.id
                    ),
                );
                self.bot = Some(bot);

                // Determine initial state based on chat_id config
                let chat_id = with_config(|c| c.telegram.chat_id.clone());
                let new_state = if chat_id.is_empty() {
                    BotState::Discovery
                } else {
                    BotState::Connected
                };
                *self.state.write().await = new_state;

                Ok(())
            }
            Err(e) => {
                logger::error(
                    LogTag::Telegram,
                    &format!("Failed to validate bot token: {}", e),
                );
                Err(format!("Invalid bot token: {}", e))
            }
        }
    }

    /// Get the current bot state
    pub async fn get_state(&self) -> BotState {
        self.state.read().await.clone()
    }

    /// Set the bot state
    pub async fn set_state(&self, state: BotState) {
        let state_debug = format!("{:?}", state);
        *self.state.write().await = state;
        logger::debug(
            LogTag::Telegram,
            &format!("Bot state changed to: {}", state_debug),
        );
    }

    /// Check if the bot is initialized and has a valid token
    pub fn is_initialized(&self) -> bool {
        self.bot.is_some()
    }

    /// Check if polling is currently active
    pub fn is_polling(&self) -> bool {
        self.polling_active.load(Ordering::SeqCst)
    }

    /// Get the underlying Bot instance
    pub fn get_bot(&self) -> Option<&Bot> {
        self.bot.as_ref()
    }

    /// Get the shutdown notifier
    pub fn get_shutdown(&self) -> Arc<Notify> {
        self.shutdown.clone()
    }

    /// Signal shutdown
    pub fn shutdown(&self) {
        self.shutdown.notify_waiters();
        self.polling_active.store(false, Ordering::SeqCst);
    }

    /// Send a message to a specific chat
    pub async fn send_message(&self, chat_id: i64, message: &str) -> Result<(), String> {
        let bot = self.bot.as_ref().ok_or("Bot not initialized")?;

        bot.send_message(ChatId(chat_id), message)
            .parse_mode(ParseMode::Html)
            .await
            .map_err(|e| format!("Failed to send message: {}", e))?;

        Ok(())
    }

    /// Send a message to the configured chat
    pub async fn send_to_configured_chat(&self, message: &str) -> Result<(), String> {
        let chat_id_str = with_config(|c| c.telegram.chat_id.clone());
        if chat_id_str.is_empty() {
            return Err("No chat ID configured".to_string());
        }

        let chat_id: i64 = chat_id_str
            .parse()
            .map_err(|e| format!("Invalid chat ID: {}", e))?;

        self.send_message(chat_id, message).await
    }

    /// Set polling active state
    pub fn set_polling_active(&self, active: bool) {
        self.polling_active.store(active, Ordering::SeqCst);
    }
}

impl Default for TelegramBot {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GLOBAL BOT INSTANCE
// ============================================================================

use once_cell::sync::Lazy;
use std::sync::Mutex;

static TELEGRAM_BOT: Lazy<Mutex<Option<TelegramBot>>> = Lazy::new(|| Mutex::new(None));

/// Initialize the global bot instance
pub async fn init_bot() -> Result<(), String> {
    let mut bot = TelegramBot::new();
    bot.initialize().await?;

    if let Ok(mut guard) = TELEGRAM_BOT.lock() {
        *guard = Some(bot);
    }

    Ok(())
}

/// Get the global bot instance
pub fn get_bot() -> Option<std::sync::MutexGuard<'static, Option<TelegramBot>>> {
    TELEGRAM_BOT.lock().ok()
}

/// Check if the bot is initialized
pub fn is_bot_initialized() -> bool {
    if let Ok(guard) = TELEGRAM_BOT.lock() {
        guard.as_ref().map(|b| b.is_initialized()).unwrap_or(false)
    } else {
        false
    }
}

/// Send a message using the global bot instance
pub async fn send_message(chat_id: i64, message: &str) -> Result<(), String> {
    let token = with_config(|c| c.telegram.bot_token.clone());
    if token.is_empty() {
        return Err("Bot not configured".to_string());
    }

    let bot = Bot::new(&token);
    bot.send_message(ChatId(chat_id), message)
        .parse_mode(ParseMode::Html)
        .await
        .map_err(|e| format!("Failed to send message: {}", e))?;

    Ok(())
}
