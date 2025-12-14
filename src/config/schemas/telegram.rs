//! Telegram bot configuration for notifications and bot commands

use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// TELEGRAM BOT CONFIGURATION
// ============================================================================

config_struct! {
    /// Telegram bot configuration for notifications and commands
    pub struct TelegramConfig {
        /// Enable Telegram notifications and commands
        #[metadata(field_metadata! {
            label: "Enable Telegram",
            hint: "Enable Telegram bot integration for notifications and commands",
            category: "Connection",
        })]
        enabled: bool = false,

        /// Bot token from @BotFather (create your own bot)
        #[metadata(field_metadata! {
            label: "Bot Token",
            hint: "Get this from @BotFather on Telegram: 1) Search for @BotFather 2) Send /newbot 3) Follow instructions 4) Copy the token",
            placeholder: "123456789:ABCdefGHIjklMNOpqrsTUVwxyz",
            category: "Connection",
        })]
        bot_token: String = String::new(),

        /// Your chat ID to receive notifications
        #[metadata(field_metadata! {
            label: "Chat ID",
            hint: "Get this by messaging @userinfobot or @getidsbot on Telegram - they will reply with your chat ID",
            placeholder: "123456789",
            category: "Connection",
        })]
        chat_id: String = String::new(),

        /// Notification preferences
        #[metadata(field_metadata! {
            label: "Trade Alerts",
            hint: "Notify when significant trades are detected on watched tokens",
            category: "Notifications",
        })]
        notify_trade_alerts: bool = true,

        #[metadata(field_metadata! {
            label: "Position Opened",
            hint: "Notify when a new trading position is opened",
            category: "Notifications",
        })]
        notify_position_opened: bool = true,

        #[metadata(field_metadata! {
            label: "Position Closed",
            hint: "Notify when a trading position is closed (with P&L)",
            category: "Notifications",
        })]
        notify_position_closed: bool = true,

        #[metadata(field_metadata! {
            label: "System Errors",
            hint: "Notify about critical system errors requiring attention",
            category: "Notifications",
        })]
        notify_system_errors: bool = true,

        #[metadata(field_metadata! {
            label: "Daily Summary",
            hint: "Send a daily summary of trading activity and P&L",
            category: "Notifications",
        })]
        notify_daily_summary: bool = false,

        /// Bot commands enabled (/status, /positions, /balance, /stop, /start)
        #[metadata(field_metadata! {
            label: "Enable Commands",
            hint: "Allow controlling the bot via Telegram commands: /status, /positions, /balance, /stop, /start",
            category: "Commands",
        })]
        commands_enabled: bool = true,

        /// Minimum trade amount (SOL) to trigger notification
        #[metadata(field_metadata! {
            label: "Min Trade Alert",
            hint: "Minimum trade size in SOL to trigger a trade alert notification",
            unit: "SOL",
            min: 0.001,
            step: 0.01,
            category: "Notifications",
        })]
        trade_alert_min_sol: f64 = 0.1,
    }
}
