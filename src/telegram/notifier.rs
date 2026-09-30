//! Telegram notifier for sending messages and notifications
//!
//! Provides the core message sending functionality.

use crate::config::with_config;
use crate::i18n::{ids, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::telegram::formatters::{self, nested_arg, text_arg};
use crate::telegram::keyboards;
use crate::telegram::messages;
use crate::telegram::pagination::PAGINATION_MANAGER;
use crate::telegram::text::{tg, tg_id, with_icon};
use crate::telegram::types::{ErrorSeverity, Notification, NotificationType, UpdateStage};
use crate::telegram::{Error, Result};
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
    pub fn new(bot_token: &str, chat_id: &str) -> Result<Self> {
        if bot_token.is_empty() {
            return Err(Error::NotConfigured);
        }

        if chat_id.is_empty() {
            return Err(Error::NotConfigured);
        }

        let chat_id_parsed: i64 =
            chat_id
                .parse()
                .map_err(|e: std::num::ParseIntError| Error::InvalidChatId {
                    chat_id: chat_id.to_owned(),
                    detail: e.to_string(),
                })?;

        let bot = Bot::new(bot_token);

        Ok(Self {
            bot,
            chat_id: ChatId(chat_id_parsed),
        })
    }

    /// Create a notifier from config
    pub fn from_config() -> Result<Self> {
        let config = with_config(|c| c.telegram.clone());
        Self::new(&config.bot_token, &config.chat_id)
    }

    /// Send a notification
    pub async fn send(&self, notification: &Notification) -> Result<()> {
        // Handle pagination notification
        if let NotificationType::NewTokensFound { session_id, .. } = &notification.notification_type
        {
            if let Some((items, total_pages, total_items)) =
                PAGINATION_MANAGER.get_page(session_id, 0)
            {
                let text = formatters::format_tokens_page(&items, 0, total_pages, total_items);
                let keyboard = keyboards::pagination_keyboard(session_id, 0, total_pages);

                self.bot
                    .send_message(self.chat_id, text)
                    .parse_mode(ParseMode::Html)
                    .link_preview_options(teloxide::types::LinkPreviewOptions {
                        is_disabled: true,
                        url: None,
                        prefer_small_media: false,
                        prefer_large_media: false,
                        show_above_text: false,
                    })
                    .reply_markup(keyboard)
                    .await
                    .map_err(|e| Error::SendFailed {
                        chat_id: self.chat_id.0.to_string(),
                        detail: e.to_string(),
                    })?;

                logger::info(LogTag::Telegram, "Sent paginated token notification");
                return Ok(());
            }
        }

        let message = self.format_notification(notification);
        self.send_message(&message).await
    }

    /// Send a plain text message
    pub async fn send_message(&self, message: &str) -> Result<()> {
        self.bot
            .send_message(self.chat_id, message)
            .parse_mode(ParseMode::Html)
            .await
            .map_err(|e| Error::SendFailed {
                chat_id: self.chat_id.0.to_string(),
                detail: e.to_string(),
            })?;

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
    ) -> Result<()> {
        let keyboard = InlineKeyboardMarkup::new(buttons);

        self.bot
            .send_message(self.chat_id, message)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await
            .map_err(|e| Error::SendFailed {
                chat_id: self.chat_id.0.to_string(),
                detail: e.to_string(),
            })?;

        Ok(())
    }

    /// Send a message with a keyboard markup
    pub async fn send_with_keyboard(
        &self,
        message: &str,
        keyboard: InlineKeyboardMarkup,
    ) -> Result<()> {
        self.bot
            .send_message(self.chat_id, message)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await
            .map_err(|e| Error::SendFailed {
                chat_id: self.chat_id.0.to_string(),
                detail: e.to_string(),
            })?;

        Ok(())
    }

    /// Format a notification into a Telegram message, in the Telegram language
    /// at the time of sending.
    fn format_notification(&self, notification: &Notification) -> String {
        match &notification.notification_type {
            NotificationType::TradeAlert {
                token_symbol,
                token_mint,
                trade_type,
                amount_sol,
                wallet,
            } => {
                let is_buy = trade_type == "buy";
                let action = UiText::new(if is_buy {
                    ids::TELEGRAM_NOTIFY_ALERT_BOUGHT
                } else {
                    ids::TELEGRAM_NOTIFY_ALERT_SOLD
                })
                .arg("amount", text_arg(format!("{amount_sol:.4}")));
                let rows = [
                    tg(&UiText::new(ids::TELEGRAM_NOTIFY_ALERT_TOKEN)
                        .arg("symbol", text_arg(token_symbol.as_str()))),
                    tg(&UiText::new(ids::TELEGRAM_NOTIFY_ALERT_MINT)
                        .arg("mint", text_arg(token_mint.as_str()))),
                    tg(&action),
                    tg(&UiText::new(ids::TELEGRAM_NOTIFY_ALERT_WALLET).arg(
                        "wallet",
                        text_arg(formatters::shorten_middle(wallet, 6, 4, 12)),
                    )),
                ];
                format!(
                    "{}\n\n{}",
                    with_icon(
                        if is_buy { "🔵" } else { "🔴" },
                        &tg_id(ids::TELEGRAM_NOTIFY_ALERT_TITLE)
                    ),
                    rows.join("\n")
                )
            }

            NotificationType::PositionOpened {
                token_symbol,
                token_mint,
                amount_sol,
                entry_price,
                ai_reasoning,
            } => {
                let should_include_ai = with_config(|c| c.telegram.include_ai_reasoning);
                let reasoning = if should_include_ai {
                    ai_reasoning
                } else {
                    &None
                };

                messages::msg_position_opened(
                    token_symbol,
                    token_mint,
                    *amount_sol,
                    *entry_price,
                    0.0, // tokens not provided in basic notification
                    None,
                    reasoning,
                )
            }

            NotificationType::PositionClosed {
                token_symbol,
                pnl_sol,
                pnl_percent,
                exit_reason,
                entry_price,
                exit_price,
                invested,
                received,
                duration_secs,
                ai_reasoning,
                ..
            } => {
                let should_include_ai = with_config(|c| c.telegram.include_ai_reasoning);
                let reasoning = if should_include_ai {
                    ai_reasoning
                } else {
                    &None
                };

                messages::msg_position_closed(
                    token_symbol,
                    *pnl_sol,
                    *pnl_percent,
                    *entry_price,
                    *exit_price,
                    *invested,
                    *received,
                    *duration_secs,
                    exit_reason.as_deref(),
                    reasoning,
                )
            }

            NotificationType::PartialExit {
                token_symbol,
                exit_percent,
                pnl_sol,
                remaining_percent,
                ..
            } => messages::msg_partial_exit(
                token_symbol,
                *exit_percent,
                *pnl_sol,
                0.0, // pnl_pct not provided
                0.0, // received_sol not provided
                *remaining_percent,
            ),

            NotificationType::DcaExecuted {
                token_symbol,
                dca_amount_sol,
                total_invested_sol,
                dca_count,
                ..
            } => messages::msg_dca_executed(
                token_symbol,
                *dca_amount_sol,
                *total_invested_sol,
                *dca_count,
                0.0, // new_avg_price not provided
            ),

            NotificationType::SystemError { message, severity } => {
                messages::msg_system_error(severity, message)
            }

            NotificationType::CopyTrading {
                task,
                task_id,
                title,
                token_symbol,
                token_mint,
                detail,
                paper,
            } => messages::msg_copy_trading(
                task.as_deref(),
                *task_id,
                title,
                token_symbol.as_deref(),
                token_mint.as_deref(),
                detail,
                *paper,
            ),

            NotificationType::DailySummary {
                date,
                total_trades,
                winning_trades,
                losing_trades,
                total_pnl_sol,
                open_positions,
            } => messages::msg_daily_summary(
                date,
                *total_trades,
                *winning_trades,
                *losing_trades,
                *total_pnl_sol,
                *open_positions,
            ),

            NotificationType::ScheduledTaskResult {
                task_name,
                outcome,
                detail,
            } => messages::msg_scheduled_task_result(task_name, *outcome, detail),

            NotificationType::BotStarted { version, mode } => {
                messages::msg_bot_started(version, mode)
            }

            NotificationType::BotStopped { reason } => messages::msg_bot_stopped(reason),

            NotificationType::UpdateStatus {
                version,
                stage,
                silent,
                transfer_bytes,
            } => {
                let version = text_arg(version.as_str());
                let installs_silently = *silent;
                match stage {
                    UpdateStage::Available => {
                        let how = if installs_silently {
                            ids::TELEGRAM_UPDATE_HOW_CORE
                        } else {
                            ids::TELEGRAM_NOTIFY_UPDATE_HOW_INSTALLER
                        };
                        let megabytes = *transfer_bytes as f64 / (1024.0 * 1024.0);
                        with_icon(
                            "\u{2b06}\u{fe0f}",
                            &tg(&UiText::new(ids::TELEGRAM_NOTIFY_UPDATE_AVAILABLE)
                                .arg("version", version)
                                .arg("how", nested_arg(UiText::new(how)))
                                .arg("size", text_arg(format!("{megabytes:.1}")))),
                        )
                    }
                    UpdateStage::Staged => {
                        let how = if installs_silently {
                            ids::TELEGRAM_NOTIFY_UPDATE_READY_SILENT
                        } else {
                            ids::TELEGRAM_NOTIFY_UPDATE_READY_INSTALLER
                        };
                        with_icon(
                            "\u{2705}",
                            &tg(&UiText::new(ids::TELEGRAM_NOTIFY_UPDATE_READY)
                                .arg("version", version)
                                .arg("how", nested_arg(UiText::new(how)))),
                        )
                    }
                    UpdateStage::Applying => with_icon(
                        "\u{1f504}",
                        &tg(&UiText::new(ids::TELEGRAM_NOTIFY_UPDATE_APPLYING)
                            .arg("version", version)),
                    ),
                }
            }

            NotificationType::NewTokensFound { new_count, .. } => with_icon(
                "🔍",
                &tg(&UiText::new(ids::TELEGRAM_NOTIFY_NEW_TOKENS)
                    .arg("count", UiArg::Count(*new_count as i64))),
            ),
        }
    }
}

// ============================================================================
// GLOBAL NOTIFICATION FUNCTIONS
// ============================================================================

use std::sync::LazyLock;
use std::sync::RwLock;

/// Global notifier instance
static NOTIFIER: LazyLock<RwLock<Option<TelegramNotifier>>> = LazyLock::new(|| RwLock::new(None));

/// Notification queue sender
static NOTIFICATION_QUEUE: LazyLock<RwLock<Option<mpsc::Sender<Notification>>>> =
    LazyLock::new(|| RwLock::new(None));

/// Initialize the global notifier
pub fn init_notifier() -> Result<()> {
    let config = with_config(|c| c.telegram.clone());

    if !config.enabled {
        logger::info(
            LogTag::Telegram,
            "Telegram notifications disabled in config",
        );
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
                &format!("Failed to initialize notifier: {e}"),
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
    let (bot_token, chat_id) =
        with_config(|c| (c.telegram.bot_token.clone(), c.telegram.chat_id.clone()));

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
        logger::error(
            LogTag::Telegram,
            &format!("Failed to send notification: {e}"),
        );
    }
}

/// Queue a notification (non-blocking, for use from sync contexts)
pub fn queue_notification(notification: Notification) {
    if let Ok(guard) = NOTIFICATION_QUEUE.read() {
        if let Some(ref sender) = *guard {
            if sender.try_send(notification).is_err() {
                logger::warning(
                    LogTag::Telegram,
                    "Notification queue full, dropping message",
                );
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
        NotificationType::ScheduledTaskResult { .. } => true, // Gated per task by the worker
        NotificationType::BotStarted { .. } => config.notify_on_startup,
        NotificationType::BotStopped { .. } => config.notify_on_shutdown,
        NotificationType::NewTokensFound { .. } => config.notify_filtering_alerts,
        NotificationType::CopyTrading { paper, .. } => {
            if *paper {
                config.notify_copy_paper
            } else {
                config.notify_copy_trading
            }
        }
        NotificationType::UpdateStatus { .. } => with_config(|c| c.updates.notify_telegram),
    }
}

/// Send a test message to verify the connection
pub async fn send_test_message(message: &str) -> Result<()> {
    if let Ok(guard) = NOTIFIER.read() {
        if let Some(ref notifier) = *guard {
            notifier.send_message(message).await
        } else {
            Err(Error::NotConfigured)
        }
    } else {
        Err(crate::errors::InternalError::InvariantViolation {
            message: "telegram notifier lock is poisoned".to_owned(),
        }
        .into())
    }
}
