//! Telegram notifier for sending messages and notifications
//!
//! Provides the core message sending functionality.

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::telegram::formatters;
use crate::telegram::types::{ErrorSeverity, Notification, NotificationType};
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};
use tokio::sync::mpsc;

/// Telegram notifier for sending messages
pub struct TelegramNotifier {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramNotifier {
    /// Create a new Telegram notifier
    ///
    /// # Arguments
    /// * `bot_token` - Telegram bot token from @BotFather
    /// * `chat_id` - Chat ID to send notifications to
    ///
    /// # Returns
    /// * `Ok(Self)` - Successfully created notifier
    /// * `Err(String)` - Failed to create notifier
    pub fn new(bot_token: &str, chat_id: &str) -> Result<Self, String> {
        if bot_token.is_empty() {
            return Err("Bot token is empty".to_string());
        }

        if chat_id.is_empty() {
            return Err("Chat ID is empty".to_string());
        }

        let chat_id_parsed: i64 = chat_id
            .parse()
            .map_err(|e| format!("Invalid chat ID '{}': {}", chat_id, e))?;

        let bot = Bot::new(bot_token);

        Ok(Self {
            bot,
            chat_id: ChatId(chat_id_parsed),
        })
    }

    /// Create a notifier from config
    pub fn from_config() -> Result<Self, String> {
        let config = with_config(|c| c.telegram.clone());
        Self::new(&config.bot_token, &config.chat_id)
    }

    /// Send a notification
    pub async fn send(&self, notification: &Notification) -> Result<(), String> {
        let message = self.format_notification(notification);
        self.send_message(&message).await
    }

    /// Send a plain text message
    pub async fn send_message(&self, message: &str) -> Result<(), String> {
        self.bot
            .send_message(self.chat_id, message)
            .parse_mode(ParseMode::Html)
            .await
            .map_err(|e| format!("Failed to send Telegram message: {}", e))?;

        logger::debug(
            LogTag::Telegram,
            &format!("Sent Telegram notification (length={})", message.len()),
        );

        Ok(())
    }

    /// Send a message with inline keyboard buttons
    pub async fn send_with_buttons(
        &self,
        message: &str,
        buttons: Vec<Vec<InlineKeyboardButton>>,
    ) -> Result<(), String> {
        let keyboard = InlineKeyboardMarkup::new(buttons);

        self.bot
            .send_message(self.chat_id, message)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await
            .map_err(|e| format!("Failed to send Telegram message with buttons: {}", e))?;

        Ok(())
    }

    /// Send a message with a keyboard markup
    pub async fn send_with_keyboard(
        &self,
        message: &str,
        keyboard: InlineKeyboardMarkup,
    ) -> Result<(), String> {
        self.bot
            .send_message(self.chat_id, message)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await
            .map_err(|e| format!("Failed to send Telegram message with keyboard: {}", e))?;

        Ok(())
    }

    /// Format a notification into a Telegram message
    fn format_notification(&self, notification: &Notification) -> String {
        match &notification.notification_type {
            NotificationType::TradeAlert {
                token_symbol,
                token_mint,
                trade_type,
                amount_sol,
                wallet,
            } => {
                let emoji = if trade_type == "buy" { "🔵" } else { "🔴" };
                let action = if trade_type == "buy" {
                    "bought"
                } else {
                    "sold"
                };
                format!(
                    "{} <b>Trade Alert</b>\n\n\
                     Token: <code>${}</code>\n\
                     Mint: <code>{}</code>\n\
                     Action: {} {:.4} SOL\n\
                     Wallet: <code>{}</code>",
                    emoji,
                    token_symbol,
                    token_mint,
                    action,
                    amount_sol,
                    Self::truncate_address(wallet)
                )
            }

            NotificationType::PositionOpened {
                token_symbol,
                token_mint,
                amount_sol,
                entry_price,
            } => formatters::msg_position_opened(
                token_symbol,
                token_mint,
                *amount_sol,
                *entry_price,
                0.0, // tokens not provided in basic notification
                "Unknown",
            ),

            NotificationType::PositionClosed {
                token_symbol,
                token_mint,
                pnl_sol,
                pnl_percent,
                exit_reason,
            } => formatters::msg_position_closed(
                token_symbol,
                token_mint,
                *pnl_sol,
                *pnl_percent,
                0.0, // entry_price not provided
                0.0, // exit_price not provided
                0.0, // invested not provided
                0.0, // received not provided
                0,   // duration not provided
                exit_reason,
            ),

            NotificationType::PartialExit {
                token_symbol,
                token_mint,
                exit_percent,
                pnl_sol,
                remaining_percent,
            } => formatters::msg_partial_exit(
                token_symbol,
                token_mint,
                *exit_percent,
                *pnl_sol,
                0.0, // pnl_pct not provided
                0.0, // received_sol not provided
                *remaining_percent,
            ),

            NotificationType::DcaExecuted {
                token_symbol,
                token_mint,
                dca_amount_sol,
                total_invested_sol,
                dca_count,
            } => formatters::msg_dca_executed(
                token_symbol,
                token_mint,
                *dca_amount_sol,
                *total_invested_sol,
                *dca_count,
                0.0, // new_avg_price not provided
            ),

            NotificationType::SystemError { message, severity } => {
                formatters::msg_system_error(&severity.to_string(), message)
            }

            NotificationType::DailySummary {
                date,
                total_trades,
                winning_trades,
                losing_trades,
                total_pnl_sol,
                open_positions,
            } => formatters::msg_daily_summary(
                date,
                *total_trades,
                *winning_trades,
                *losing_trades,
                *total_pnl_sol,
                *open_positions,
            ),

            NotificationType::BotCommand { command, response } => {
                format!("📟 <b>Command:</b> /{}\n\n{}", command, response)
            }

            NotificationType::BotStarted { version, mode } => {
                formatters::msg_bot_started(version, mode, "", 0.0)
            }

            NotificationType::BotStopped { reason } => {
                formatters::msg_bot_stopped(reason, 0, 0, 0.0)
            }
        }
    }

    /// Truncate an address for display
    fn truncate_address(address: &str) -> String {
        if address.len() > 12 {
            format!("{}...{}", &address[..6], &address[address.len() - 4..])
        } else {
            address.to_string()
        }
    }
}

// ============================================================================
// GLOBAL NOTIFICATION FUNCTIONS
// ============================================================================

use once_cell::sync::Lazy;
use std::sync::RwLock;

/// Global notifier instance
static NOTIFIER: Lazy<RwLock<Option<TelegramNotifier>>> = Lazy::new(|| RwLock::new(None));

/// Notification queue sender
static NOTIFICATION_QUEUE: Lazy<RwLock<Option<mpsc::Sender<Notification>>>> =
    Lazy::new(|| RwLock::new(None));

/// Initialize the global notifier
pub fn init_notifier() -> Result<(), String> {
    let config = with_config(|c| c.telegram.clone());

    if !config.enabled {
        logger::info(LogTag::Telegram, "Telegram notifications disabled in config");
        return Ok(());
    }

    match TelegramNotifier::new(&config.bot_token, &config.chat_id) {
        Ok(notifier) => {
            if let Ok(mut guard) = NOTIFIER.write() {
                *guard = Some(notifier);
            }
            logger::info(LogTag::Telegram, "Telegram notifier initialized");
            Ok(())
        }
        Err(e) => {
            logger::warning(
                LogTag::Telegram,
                &format!("Failed to initialize notifier: {}", e),
            );
            // Don't return error - allow bot to run without notifications
            Ok(())
        }
    }
}

/// Check if notifications are enabled
pub fn is_enabled() -> bool {
    if let Ok(guard) = NOTIFIER.read() {
        guard.is_some()
    } else {
        false
    }
}

/// Send a notification (async)
pub async fn send_notification(notification: Notification) {
    // Check notification preferences first (no lock needed)
    if !should_send_notification(&notification) {
        logger::debug(LogTag::Telegram, "Notification filtered by preferences");
        return;
    }

    // Get bot token and chat_id from config - this avoids holding lock across await
    let (bot_token, chat_id) = with_config(|c| {
        (c.telegram.bot_token.clone(), c.telegram.chat_id.clone())
    });

    if bot_token.is_empty() || chat_id.is_empty() {
        return;
    }

    // Check if notifier is configured (quick check, no await)
    let is_configured = if let Ok(guard) = NOTIFIER.read() {
        guard.is_some()
    } else {
        false
    };

    if !is_configured {
        return;
    }

    // Create a temporary notifier for this send operation
    let notifier = match TelegramNotifier::new(&bot_token, &chat_id) {
        Ok(n) => n,
        Err(_) => return,
    };

    if let Err(e) = notifier.send(&notification).await {
        logger::error(LogTag::Telegram, &format!("Failed to send notification: {}", e));
    }
}

/// Queue a notification (non-blocking, for use from sync contexts)
pub fn queue_notification(notification: Notification) {
    if let Ok(guard) = NOTIFICATION_QUEUE.read() {
        if let Some(ref sender) = *guard {
            if sender.try_send(notification).is_err() {
                logger::warning(LogTag::Telegram, "Notification queue full, dropping message");
            }
        }
    }
}

/// Set the notification queue sender
pub fn set_notification_queue(sender: mpsc::Sender<Notification>) {
    if let Ok(mut guard) = NOTIFICATION_QUEUE.write() {
        *guard = Some(sender);
    }
}

/// Check if a notification should be sent based on config preferences
fn should_send_notification(notification: &Notification) -> bool {
    let config = with_config(|c| c.telegram.clone());

    match &notification.notification_type {
        NotificationType::TradeAlert { amount_sol, .. } => {
            config.notify_trade_alerts && *amount_sol >= config.trade_alert_min_sol
        }
        NotificationType::PositionOpened { .. } => config.notify_position_opened,
        NotificationType::PositionClosed { .. } => config.notify_position_closed,
        NotificationType::PartialExit { .. } => config.notify_partial_exit,
        NotificationType::DcaExecuted { .. } => config.notify_dca_executed,
        NotificationType::SystemError { severity, .. } => match severity {
            ErrorSeverity::Critical | ErrorSeverity::Error => config.notify_system_errors,
            ErrorSeverity::Warning => config.notify_system_errors,
            ErrorSeverity::Info => false, // Don't send info level unless explicitly enabled
        },
        NotificationType::DailySummary { .. } => config.notify_daily_summary,
        NotificationType::BotCommand { .. } => true, // Always send command responses
        NotificationType::BotStarted { .. } => config.notify_on_startup,
        NotificationType::BotStopped { .. } => config.notify_on_shutdown,
    }
}

/// Send a test message to verify the connection
pub async fn send_test_message(message: &str) -> Result<(), String> {
    if let Ok(guard) = NOTIFIER.read() {
        if let Some(ref notifier) = *guard {
            notifier.send_message(message).await
        } else {
            Err("Notifier not initialized".to_string())
        }
    } else {
        Err("Failed to acquire notifier lock".to_string())
    }
}
