//! Telegram bot configuration for notifications and bot commands

use crate::config::metadata::ConfigCategory;
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
            category: ConfigCategory::Connection,
        })]
        enabled: bool = false,

        /// Bot token from @BotFather (create your own bot)
        #[metadata(field_metadata! {
            category: ConfigCategory::Connection,
        })]
        bot_token: String = String::new(),

        /// Chat ID for sending notifications (discovered via Discovery Mode or set manually)
        #[metadata(field_metadata! {
            category: ConfigCategory::Connection,
        })]
        chat_id: String = String::new(),

        /// Message language: "app" follows the dashboard language, otherwise a registered locale code
        #[metadata(field_metadata! {
            category: ConfigCategory::Connection,
        })]
        language: String = "app".to_owned(),

        // === Authentication Section ===
        /// Session timeout in minutes (auto-logout after inactivity)
        #[metadata(field_metadata! {
            min: 5,
            max: 1440,
            step: 5,
            category: ConfigCategory::Authentication,
        })]
        session_timeout_minutes: i64 = 30,

        /// Maximum failed authentication attempts before lockout
        #[metadata(field_metadata! {
            min: 1,
            max: 10,
            step: 1,
            category: ConfigCategory::Authentication,
        })]
        max_failed_attempts: i64 = 3,

        /// Lockout duration in minutes after max failed attempts
        #[metadata(field_metadata! {
            min: 1,
            max: 60,
            step: 1,
            category: ConfigCategory::Authentication,
        })]
        lockout_minutes: i64 = 5,

        /// Whether Telegram commands require 2FA when session expires
        /// Uses the same TOTP as dashboard lockscreen (Security settings)
        #[metadata(field_metadata! {
            category: ConfigCategory::Authentication,
        })]
        commands_require_2fa: bool = true,

        // === Features Section ===
        /// Bot commands enabled (/status, /positions, /balance, /stop, /start)
        #[metadata(field_metadata! {
            category: ConfigCategory::Features,
        })]
        commands_enabled: bool = true,

        /// Enable inline action buttons on notifications
        #[metadata(field_metadata! {
            category: ConfigCategory::Features,
        })]
        inline_actions_enabled: bool = true,

        // === Notifications Section ===
        /// Notification preferences
        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_trade_alerts: bool = true,

        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_position_opened: bool = true,

        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_position_closed: bool = true,

        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_system_errors: bool = true,

        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_daily_summary: bool = false,

        /// Notify on partial exits
        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_partial_exit: bool = true,

        /// Notify on DCA executions
        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_dca_executed: bool = true,

        /// Notify on live copy-trading fills, exits and copy-task auto-pauses
        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_copy_trading: bool = true,

        /// Notify on paper copy-trading fills and exits
        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_copy_paper: bool = false,

        /// Notify on startup
        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_on_startup: bool = true,

        /// Notify on shutdown
        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_on_shutdown: bool = true,

        /// Notify when new tokens pass filtering criteria
        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        notify_filtering_alerts: bool = true,

        /// Include LLM-analysis reasoning in position notifications
        #[metadata(field_metadata! {
            category: ConfigCategory::Notifications,
        })]
        include_ai_reasoning: bool = false,

        // === Thresholds Section ===
        /// Minimum trade amount (SOL) to trigger notification
        #[metadata(field_metadata! {
            min: 0.001,
            step: 0.01,
            category: ConfigCategory::Thresholds,
        })]
        trade_alert_min_sol: f64 = 0.1,

        /// Significant P&L threshold for special alerts
        #[metadata(field_metadata! {
            min: 0.01,
            step: 0.1,
            category: ConfigCategory::Thresholds,
        })]
        significant_pnl_threshold: f64 = 0.5,
    }
}
