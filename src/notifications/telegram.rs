//! Telegram bot integration for ScreenerBot
//!
//! Provides Telegram bot functionality for sending notifications and handling commands.
//! Uses the teloxide crate for Telegram Bot API integration.

use super::types::{ErrorSeverity, Notification, NotificationType};
use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::positions;
use crate::utils::get_sol_balance;
use crate::version::VERSION;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};
use tokio::sync::Notify;

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
            LogTag::Notifications,
            &format!(
                "Sent Telegram notification (length={})",
                message.len()
            ),
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
                let action = if trade_type == "buy" { "bought" } else { "sold" };
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
            } => {
                format!(
                    "🟢 <b>Position Opened</b>\n\n\
                     Token: <code>${}</code>\n\
                     Mint: <code>{}</code>\n\
                     Amount: {:.4} SOL\n\
                     Entry Price: {}",
                    token_symbol,
                    token_mint,
                    amount_sol,
                    Self::format_price(*entry_price)
                )
            }

            NotificationType::PositionClosed {
                token_symbol,
                token_mint,
                pnl_sol,
                pnl_percent,
                exit_reason,
            } => {
                let emoji = if *pnl_sol >= 0.0 { "🟢" } else { "🔴" };
                let pnl_sign = if *pnl_sol >= 0.0 { "+" } else { "" };
                format!(
                    "{} <b>Position Closed</b>\n\n\
                     Token: <code>${}</code>\n\
                     Mint: <code>{}</code>\n\
                     P&L: {}{:.4} SOL ({}{:.2}%)\n\
                     Reason: {}",
                    emoji,
                    token_symbol,
                    token_mint,
                    pnl_sign,
                    pnl_sol,
                    pnl_sign,
                    pnl_percent,
                    exit_reason
                )
            }

            NotificationType::PartialExit {
                token_symbol,
                token_mint,
                exit_percent,
                pnl_sol,
                remaining_percent,
            } => {
                let emoji = if *pnl_sol >= 0.0 { "🟡" } else { "🟠" };
                let pnl_sign = if *pnl_sol >= 0.0 { "+" } else { "" };
                format!(
                    "{} <b>Partial Exit</b>\n\n\
                     Token: <code>${}</code>\n\
                     Mint: <code>{}</code>\n\
                     Exited: {:.1}%\n\
                     P&L: {}{:.4} SOL\n\
                     Remaining: {:.1}%",
                    emoji,
                    token_symbol,
                    token_mint,
                    exit_percent,
                    pnl_sign,
                    pnl_sol,
                    remaining_percent
                )
            }

            NotificationType::DcaExecuted {
                token_symbol,
                token_mint,
                dca_amount_sol,
                total_invested_sol,
                dca_count,
            } => {
                format!(
                    "📈 <b>DCA Executed</b>\n\n\
                     Token: <code>${}</code>\n\
                     Mint: <code>{}</code>\n\
                     DCA Amount: {:.4} SOL\n\
                     Total Invested: {:.4} SOL\n\
                     DCA Count: {}",
                    token_symbol, token_mint, dca_amount_sol, total_invested_sol, dca_count
                )
            }

            NotificationType::SystemError { message, severity } => {
                let emoji = match severity {
                    ErrorSeverity::Info => "ℹ️",
                    ErrorSeverity::Warning => "⚠️",
                    ErrorSeverity::Error => "❌",
                    ErrorSeverity::Critical => "🚨",
                };
                format!(
                    "{} <b>System {}</b>\n\n{}",
                    emoji,
                    Self::capitalize(&severity.to_string()),
                    message
                )
            }

            NotificationType::DailySummary {
                date,
                total_trades,
                winning_trades,
                losing_trades,
                total_pnl_sol,
                open_positions,
            } => {
                let emoji = if *total_pnl_sol >= 0.0 { "📊" } else { "📉" };
                let pnl_sign = if *total_pnl_sol >= 0.0 { "+" } else { "" };
                let win_rate = if *total_trades > 0 {
                    (*winning_trades as f64 / *total_trades as f64) * 100.0
                } else {
                    0.0
                };
                format!(
                    "{} <b>Daily Summary - {}</b>\n\n\
                     Total Trades: {}\n\
                     Wins: {} | Losses: {}\n\
                     Win Rate: {:.1}%\n\
                     Total P&L: {}{:.4} SOL\n\
                     Open Positions: {}",
                    emoji,
                    date,
                    total_trades,
                    winning_trades,
                    losing_trades,
                    win_rate,
                    pnl_sign,
                    total_pnl_sol,
                    open_positions
                )
            }

            NotificationType::BotCommand { command, response } => {
                format!("🤖 <b>/{}</b>\n\n{}", command, response)
            }

            NotificationType::BotStarted { version, mode } => {
                format!(
                    "🚀 <b>ScreenerBot Started</b>\n\n\
                     Version: {}\n\
                     Mode: {}",
                    version, mode
                )
            }

            NotificationType::BotStopped { reason } => {
                format!("🛑 <b>ScreenerBot Stopped</b>\n\nReason: {}", reason)
            }
        }
    }

    /// Format a price with appropriate precision
    fn format_price(price: f64) -> String {
        if price < 0.000001 {
            format!("{:.2e} SOL", price)
        } else if price < 0.001 {
            format!("{:.9} SOL", price)
        } else if price < 1.0 {
            format!("{:.6} SOL", price)
        } else {
            format!("{:.4} SOL", price)
        }
    }

    /// Truncate a wallet address for display
    fn truncate_address(address: &str) -> String {
        if address.len() > 12 {
            format!("{}...{}", &address[..6], &address[address.len() - 4..])
        } else {
            address.to_string()
        }
    }

    /// Capitalize the first letter of a string
    fn capitalize(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }
}

/// Start the Telegram command handler
///
/// This runs in the background and listens for incoming Telegram commands.
/// Returns a shutdown flag that can be used to stop the handler.
pub async fn start_command_handler(
    bot_token: String,
    chat_id: String,
    shutdown: Arc<Notify>,
    running: Arc<AtomicBool>,
) -> Result<tokio::task::JoinHandle<()>, String> {
    if bot_token.is_empty() {
        return Err("Bot token is empty".to_string());
    }

    let chat_id_parsed: i64 = chat_id
        .parse()
        .map_err(|e| format!("Invalid chat ID: {}", e))?;

    let bot = Bot::new(bot_token.clone());
    let allowed_chat_id = ChatId(chat_id_parsed);

    running.store(true, Ordering::SeqCst);

    let handle = tokio::spawn(async move {
        logger::info(LogTag::Notifications, "Telegram command handler started");

        // Use polling with timeout so we can check for shutdown
        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    logger::info(LogTag::Notifications, "Telegram command handler received shutdown signal");
                    break;
                }
                _ = handle_updates(&bot, allowed_chat_id) => {
                    // Continue polling
                }
            }
        }

        running.store(false, Ordering::SeqCst);
        logger::info(LogTag::Notifications, "Telegram command handler stopped");
    });

    Ok(handle)
}

/// Handle a single batch of Telegram updates
async fn handle_updates(bot: &Bot, allowed_chat_id: ChatId) {
    // Use getUpdates with a short timeout for manual polling
    match bot.get_updates().timeout(10).await {
        Ok(updates) => {
            for update in updates {
                // Use the UpdateKind enum to extract messages
                if let teloxide::types::UpdateKind::Message(message) = update.kind {
                    // Security: Only respond to messages from the configured chat
                    if message.chat.id != allowed_chat_id {
                        logger::warning(
                            LogTag::Notifications,
                            &format!(
                                "Ignoring message from unauthorized chat: {}",
                                message.chat.id
                            ),
                        );
                        continue;
                    }

                    // Handle text commands
                    if let Some(text) = message.text() {
                        if let Err(e) = handle_command(bot, allowed_chat_id, text).await {
                            logger::error(
                                LogTag::Notifications,
                                &format!("Error handling command '{}': {}", text, e),
                            );
                        }
                    }
                }
            }
        }
        Err(e) => {
            // Log error but don't spam - connection issues are normal
            logger::debug(
                LogTag::Notifications,
                &format!("Error fetching Telegram updates: {}", e),
            );
        }
    }
}

/// Handle a single command
async fn handle_command(bot: &Bot, chat_id: ChatId, text: &str) -> Result<(), String> {
    let text = text.trim();

    // Check if it's a command
    if !text.starts_with('/') {
        return Ok(());
    }

    let command = text.split_whitespace().next().unwrap_or("");
    let response = match command {
        "/start" => handle_start_command().await,
        "/stop" => handle_stop_command().await,
        "/status" => handle_status_command().await,
        "/positions" => handle_positions_command().await,
        "/balance" => handle_balance_command().await,
        "/help" => handle_help_command(),
        _ => format!("❓ Unknown command: {}\n\nUse /help to see available commands.", command),
    };

    bot.send_message(chat_id, &response)
        .parse_mode(ParseMode::Html)
        .await
        .map_err(|e| format!("Failed to send response: {}", e))?;

    logger::info(
        LogTag::Notifications,
        &format!("Handled Telegram command: {}", command),
    );

    Ok(())
}

/// Check if trader is enabled from config
fn is_trader_enabled() -> bool {
    with_config(|cfg| cfg.trader.enabled)
}

/// Handle /start command - Enable trading
async fn handle_start_command() -> String {
    let currently_enabled = is_trader_enabled();

    if currently_enabled {
        return "✅ <b>Trading is already enabled</b>".to_string();
    }

    // Enable trading via config
    match update_config_section(
        |cfg| {
            cfg.trader.enabled = true;
        },
        true,
    ) {
        Ok(()) => {
            logger::info(LogTag::Notifications, "Trading enabled via Telegram command");
            "✅ <b>Trading Enabled</b>\n\nThe bot will now monitor for entry signals and execute trades."
                .to_string()
        }
        Err(e) => {
            format!("❌ <b>Failed to enable trading</b>\n\nError: {}", e)
        }
    }
}

/// Handle /stop command - Disable trading
async fn handle_stop_command() -> String {
    let currently_enabled = is_trader_enabled();

    if !currently_enabled {
        return "✅ <b>Trading is already disabled</b>".to_string();
    }

    // Disable trading via config
    match update_config_section(
        |cfg| {
            cfg.trader.enabled = false;
        },
        true,
    ) {
        Ok(()) => {
            logger::info(
                LogTag::Notifications,
                "Trading disabled via Telegram command",
            );
            "🛑 <b>Trading Disabled</b>\n\nThe bot will stop entering new positions.\nExisting positions will continue to be monitored."
                .to_string()
        }
        Err(e) => {
            format!("❌ <b>Failed to disable trading</b>\n\nError: {}", e)
        }
    }
}

/// Handle /status command
async fn handle_status_command() -> String {
    let trading_enabled = is_trader_enabled();
    let open_positions = positions::get_open_positions_count().await;
    let uptime = (chrono::Utc::now() - *crate::global::STARTUP_TIME).num_seconds() as u64;
    let services_ready = crate::global::are_core_services_ready();

    let uptime_str = format_duration(uptime);
    let status_emoji = if trading_enabled && services_ready {
        "🟢"
    } else if services_ready {
        "🟡"
    } else {
        "🔴"
    };
    let trading_status = if trading_enabled { "Enabled" } else { "Disabled" };

    format!(
        "{} <b>ScreenerBot Status</b>\n\n\
         Version: {}\n\
         Uptime: {}\n\
         Trading: {}\n\
         Open Positions: {}\n\
         Services Ready: {}",
        status_emoji,
        VERSION,
        uptime_str,
        trading_status,
        open_positions,
        if services_ready { "Yes ✅" } else { "No ❌" }
    )
}

/// Handle /positions command
async fn handle_positions_command() -> String {
    let positions = positions::get_open_positions().await;

    if positions.is_empty() {
        return "📊 <b>No Open Positions</b>\n\nYou have no active positions.".to_string();
    }

    let mut response = format!("📊 <b>Open Positions ({})</b>\n\n", positions.len());

    for (i, pos) in positions.iter().take(10).enumerate() {
        let pnl_emoji = if pos.unrealized_pnl.unwrap_or(0.0) >= 0.0 {
            "🟢"
        } else {
            "🔴"
        };
        let pnl_pct = pos.unrealized_pnl_percent.unwrap_or(0.0);
        let pnl_sign = if pnl_pct >= 0.0 { "+" } else { "" };

        response.push_str(&format!(
            "{}. <code>${}</code> {}\n   Size: {:.4} SOL | P&L: {}{:.2}%\n\n",
            i + 1,
            pos.symbol,
            pnl_emoji,
            pos.total_size_sol,
            pnl_sign,
            pnl_pct
        ));
    }

    if positions.len() > 10 {
        response.push_str(&format!("... and {} more", positions.len() - 10));
    }

    response
}

/// Handle /balance command
async fn handle_balance_command() -> String {
    let wallet_address = match crate::utils::get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => return format!("❌ <b>Error</b>\n\nFailed to get wallet address: {}", e),
    };

    let sol_balance = match get_sol_balance(&wallet_address).await {
        Ok(balance) => balance,
        Err(e) => return format!("❌ <b>Error</b>\n\nFailed to get SOL balance: {}", e),
    };

    // Get SOL price for USD value
    let sol_price_usd = crate::sol_price::get_sol_price();
    let usd_value = sol_balance * sol_price_usd;

    format!(
        "💰 <b>Wallet Balance</b>\n\n\
         Address: <code>{}</code>\n\
         SOL: {:.4}\n\
         USD: ${:.2}",
        TelegramNotifier::truncate_address(&wallet_address),
        sol_balance,
        usd_value
    )
}

/// Handle /help command
fn handle_help_command() -> String {
    "🤖 <b>ScreenerBot Commands</b>\n\n\
     /status - Bot status, uptime, and trading state\n\
     /positions - List open positions with P&L\n\
     /balance - Show wallet SOL balance\n\
     /start - Enable trading\n\
     /stop - Disable trading (keeps monitoring)\n\
     /help - Show this help message\n\n\
     <i>Note: Commands only work from the configured chat ID.</i>"
        .to_string()
}

/// Format seconds into a human-readable duration
fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}
