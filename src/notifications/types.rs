//! Notification types for Telegram integration
//!
//! Defines the notification types that can be sent via Telegram.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Types of notifications that can be sent
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NotificationType {
    /// Alert when a tracked wallet makes a trade
    TradeAlert {
        token_symbol: String,
        token_mint: String,
        trade_type: String, // "buy" or "sell"
        amount_sol: f64,
        wallet: String, // external wallet that traded
    },

    /// Notification when a new position is opened
    PositionOpened {
        token_symbol: String,
        token_mint: String,
        amount_sol: f64,
        entry_price: f64,
    },

    /// Notification when a position is closed
    PositionClosed {
        token_symbol: String,
        token_mint: String,
        pnl_sol: f64,
        pnl_percent: f64,
        exit_reason: String,
    },

    /// Notification when a partial exit is executed
    PartialExit {
        token_symbol: String,
        token_mint: String,
        exit_percent: f64,
        pnl_sol: f64,
        remaining_percent: f64,
    },

    /// Notification when DCA is executed
    DcaExecuted {
        token_symbol: String,
        token_mint: String,
        dca_amount_sol: f64,
        total_invested_sol: f64,
        dca_count: u32,
    },

    /// System error or warning notification
    SystemError {
        message: String,
        severity: ErrorSeverity,
    },

    /// Daily summary of trading activity
    DailySummary {
        date: String,
        total_trades: u32,
        winning_trades: u32,
        losing_trades: u32,
        total_pnl_sol: f64,
        open_positions: u32,
    },

    /// Response to a bot command
    BotCommand {
        command: String,
        response: String,
    },

    /// Bot startup notification
    BotStarted { version: String, mode: String },

    /// Bot shutdown notification
    BotStopped { reason: String },
}

/// Severity levels for system errors
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Info => write!(f, "info"),
            ErrorSeverity::Warning => write!(f, "warning"),
            ErrorSeverity::Error => write!(f, "error"),
            ErrorSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// A notification with timestamp
#[derive(Clone, Debug)]
pub struct Notification {
    pub notification_type: NotificationType,
    pub timestamp: DateTime<Utc>,
}

impl Notification {
    /// Create a new notification with current timestamp
    pub fn new(notification_type: NotificationType) -> Self {
        Self {
            notification_type,
            timestamp: Utc::now(),
        }
    }

    /// Create a trade alert notification
    pub fn trade_alert(
        token_symbol: String,
        token_mint: String,
        trade_type: &str,
        amount_sol: f64,
        wallet: String,
    ) -> Self {
        Self::new(NotificationType::TradeAlert {
            token_symbol,
            token_mint,
            trade_type: trade_type.to_string(),
            amount_sol,
            wallet,
        })
    }

    /// Create a position opened notification
    pub fn position_opened(
        token_symbol: String,
        token_mint: String,
        amount_sol: f64,
        entry_price: f64,
    ) -> Self {
        Self::new(NotificationType::PositionOpened {
            token_symbol,
            token_mint,
            amount_sol,
            entry_price,
        })
    }

    /// Create a position closed notification
    pub fn position_closed(
        token_symbol: String,
        token_mint: String,
        pnl_sol: f64,
        pnl_percent: f64,
        exit_reason: String,
    ) -> Self {
        Self::new(NotificationType::PositionClosed {
            token_symbol,
            token_mint,
            pnl_sol,
            pnl_percent,
            exit_reason,
        })
    }

    /// Create a partial exit notification
    pub fn partial_exit(
        token_symbol: String,
        token_mint: String,
        exit_percent: f64,
        pnl_sol: f64,
        remaining_percent: f64,
    ) -> Self {
        Self::new(NotificationType::PartialExit {
            token_symbol,
            token_mint,
            exit_percent,
            pnl_sol,
            remaining_percent,
        })
    }

    /// Create a DCA executed notification
    pub fn dca_executed(
        token_symbol: String,
        token_mint: String,
        dca_amount_sol: f64,
        total_invested_sol: f64,
        dca_count: u32,
    ) -> Self {
        Self::new(NotificationType::DcaExecuted {
            token_symbol,
            token_mint,
            dca_amount_sol,
            total_invested_sol,
            dca_count,
        })
    }

    /// Create a system error notification
    pub fn system_error(message: String, severity: ErrorSeverity) -> Self {
        Self::new(NotificationType::SystemError { message, severity })
    }

    /// Create a daily summary notification
    pub fn daily_summary(
        date: String,
        total_trades: u32,
        winning_trades: u32,
        losing_trades: u32,
        total_pnl_sol: f64,
        open_positions: u32,
    ) -> Self {
        Self::new(NotificationType::DailySummary {
            date,
            total_trades,
            winning_trades,
            losing_trades,
            total_pnl_sol,
            open_positions,
        })
    }

    /// Create a bot command response notification
    pub fn bot_command(command: String, response: String) -> Self {
        Self::new(NotificationType::BotCommand { command, response })
    }

    /// Create a bot started notification
    pub fn bot_started(version: String, mode: String) -> Self {
        Self::new(NotificationType::BotStarted { version, mode })
    }

    /// Create a bot stopped notification
    pub fn bot_stopped(reason: String) -> Self {
        Self::new(NotificationType::BotStopped { reason })
    }
}
