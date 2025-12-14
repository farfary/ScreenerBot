//! Telegram notification module for ScreenerBot
//!
//! Provides Telegram bot integration for:
//! - Sending trade alerts and position notifications
//! - Receiving bot commands (/status, /positions, /balance, /start, /stop)
//! - Daily summaries and system error alerts
//!
//! # Configuration
//!
//! Configure in `config.toml` under `[telegram]`:
//! ```toml
//! [telegram]
//! enabled = true
//! bot_token = "YOUR_BOT_TOKEN_FROM_BOTFATHER"
//! chat_id = "YOUR_CHAT_ID"
//! notify_trade_alerts = true
//! notify_position_opened = true
//! notify_position_closed = true
//! notify_system_errors = true
//! notify_daily_summary = false
//! commands_enabled = true
//! trade_alert_min_sol = 0.1
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use screenerbot::notifications::{send_notification, Notification};
//!
//! // Send a position opened notification
//! send_notification(Notification::position_opened(
//!     "BONK".to_string(),
//!     "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
//!     0.5,
//!     0.00001234,
//! )).await;
//!
//! // Queue a notification (non-blocking)
//! queue_notification(Notification::system_error(
//!     "RPC connection failed".to_string(),
//!     ErrorSeverity::Warning,
//! ));
//! ```
//!
//! # Bot Commands
//!
//! When `commands_enabled = true`, the bot responds to:
//! - `/status` - Bot uptime, version, and trading status
//! - `/positions` - List open positions with P&L
//! - `/balance` - Show wallet SOL balance
//! - `/start` - Enable trading
//! - `/stop` - Disable trading
//! - `/help` - Show available commands

mod service;
mod telegram;
mod types;

// Public exports
pub use service::{
    get_notification_service, init_notification_service, is_notification_service_enabled,
    queue_notification, send_notification, start_notification_service, NotificationService,
};
pub use telegram::TelegramNotifier;
pub use types::{ErrorSeverity, Notification, NotificationType};
