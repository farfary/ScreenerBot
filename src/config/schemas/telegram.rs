//! Telegram bot configuration for notifications and bot commands

use crate::config_struct;
use crate::field_metadata;

// ============================================================================
// TELEGRAM BOT CONFIGURATION
// ============================================================================

config_struct! {
    /// Telegram bot configuration for notifications and commands
    pub struct TelegramConfig {
        // === Connection Section ===
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

        /// Chat ID for sending notifications (get from @userinfobot or discovery flow)
        #[metadata(field_metadata! {
            label: "Chat ID",
            hint: "Your Telegram chat ID for receiving notifications. Get it by messaging @userinfobot or use the discovery flow.",
            placeholder: "123456789",
            category: "Connection",
        })]
        chat_id: String = String::new(),

        // === Features Section ===
        /// Bot commands enabled (/status, /positions, /balance, /stop, /start)
        #[metadata(field_metadata! {
            label: "Enable Commands",
            hint: "Allow controlling the bot via Telegram commands: /status, /positions, /balance, /stop, /start",
            category: "Features",
        })]
        commands_enabled: bool = true,

        /// Enable inline action buttons on notifications
        #[metadata(field_metadata! {
            label: "Inline Actions",
            hint: "Show action buttons (sell, close, blacklist) on position notifications",
            category: "Features",
        })]
        inline_actions_enabled: bool = true,

        // === Notifications Section ===
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

        /// Notify on partial exits
        #[metadata(field_metadata! {
            label: "Partial Exits",
            hint: "Notify when a partial exit (sell percentage) is executed",
            category: "Notifications",
        })]
        notify_partial_exit: bool = true,

        /// Notify on DCA executions
        #[metadata(field_metadata! {
            label: "DCA Executed",
            hint: "Notify when a DCA (dollar-cost averaging) buy is executed",
            category: "Notifications",
        })]
        notify_dca_executed: bool = true,

        /// Notify on startup
        #[metadata(field_metadata! {
            label: "Bot Startup",
            hint: "Send notification when ScreenerBot starts",
            category: "Notifications",
        })]
        notify_on_startup: bool = true,

        /// Notify on shutdown
        #[metadata(field_metadata! {
            label: "Bot Shutdown",
            hint: "Send notification when ScreenerBot stops",
            category: "Notifications",
        })]
        notify_on_shutdown: bool = true,

        // === Thresholds Section ===
        /// Minimum trade amount (SOL) to trigger notification
        #[metadata(field_metadata! {
            label: "Min Trade Alert",
            hint: "Minimum trade size in SOL to trigger a trade alert notification",
            unit: "SOL",
            min: 0.001,
            step: 0.01,
            category: "Thresholds",
        })]
        trade_alert_min_sol: f64 = 0.1,

        /// Significant P&L threshold for special alerts
        #[metadata(field_metadata! {
            label: "Significant P&L",
            hint: "SOL amount threshold for highlighting significant P&L in notifications",
            unit: "SOL",
            min: 0.01,
            step: 0.1,
            category: "Thresholds",
        })]
        significant_pnl_threshold: f64 = 0.5,
    }
}
