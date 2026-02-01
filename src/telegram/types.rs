//! Core types for the Telegram module
//!
//! Contains notification types, session types, and discovery types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Instant;

// ============================================================================
// NOTIFICATION TYPES
// ============================================================================

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
        ai_reasoning: Option<String>,
    },

    /// Notification when a position is closed
    PositionClosed {
        token_symbol: String,
        token_mint: String,
        pnl_sol: f64,
        pnl_percent: f64,
        exit_reason: String,
        entry_price: f64,
        exit_price: f64,
        invested: f64,
        received: f64,
        duration_secs: u64,
        ai_reasoning: Option<String>,
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
    BotCommand { command: String, response: String },

    /// Bot startup notification
    BotStarted { version: String, mode: String },

    /// Bot shutdown notification
    BotStopped { reason: String },

    /// Notification when new tokens are found by filtering
    NewTokensFound {
        session_id: String,
        new_count: usize,
    },
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
            ai_reasoning: None,
        })
    }

    /// Create a position opened notification with AI reasoning
    pub fn position_opened_with_ai(
        token_symbol: String,
        token_mint: String,
        amount_sol: f64,
        entry_price: f64,
        ai_reasoning: Option<String>,
    ) -> Self {
        Self::new(NotificationType::PositionOpened {
            token_symbol,
            token_mint,
            amount_sol,
            entry_price,
            ai_reasoning,
        })
    }

    /// Create a position closed notification
    pub fn position_closed(
        token_symbol: String,
        token_mint: String,
        pnl_sol: f64,
        pnl_percent: f64,
        exit_reason: String,
        entry_price: f64,
        exit_price: f64,
        invested: f64,
        received: f64,
        duration_secs: u64,
    ) -> Self {
        Self::new(NotificationType::PositionClosed {
            token_symbol,
            token_mint,
            pnl_sol,
            pnl_percent,
            exit_reason,
            entry_price,
            exit_price,
            invested,
            received,
            duration_secs,
            ai_reasoning: None,
        })
    }

    /// Create a position closed notification with AI reasoning
    pub fn position_closed_with_ai(
        token_symbol: String,
        token_mint: String,
        pnl_sol: f64,
        pnl_percent: f64,
        exit_reason: String,
        entry_price: f64,
        exit_price: f64,
        invested: f64,
        received: f64,
        duration_secs: u64,
        ai_reasoning: Option<String>,
    ) -> Self {
        Self::new(NotificationType::PositionClosed {
            token_symbol,
            token_mint,
            pnl_sol,
            pnl_percent,
            exit_reason,
            entry_price,
            exit_price,
            invested,
            received,
            duration_secs,
            ai_reasoning,
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

    /// Create a new tokens found notification
    pub fn new_tokens_found(session_id: String, new_count: usize) -> Self {
        Self::new(NotificationType::NewTokensFound {
            session_id,
            new_count,
        })
    }
}

// ============================================================================
// SESSION TYPES
// ============================================================================

/// Session authentication state for Telegram commands
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// Session is active - commands work
    Active,
    /// Session expired - requires /login with TOTP to reactivate (if 2FA enabled)
    Expired,
    /// Awaiting TOTP code after /login command
    AwaitingTotp,
    /// Locked due to too many failed TOTP attempts
    Locked { until: Instant },
}

impl Default for SessionState {
    fn default() -> Self {
        Self::Active // New sessions start as Active (no password required)
    }
}

/// Active Telegram session
#[derive(Debug, Clone)]
pub struct TelegramSession {
    pub user_id: i64,
    pub chat_id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_activity: Instant,
    pub created_at: Instant,
    pub state: SessionState,
    pub failed_attempts: u32,
}

impl TelegramSession {
    pub fn new(
        user_id: i64,
        chat_id: i64,
        username: Option<String>,
        first_name: Option<String>,
    ) -> Self {
        Self {
            user_id,
            chat_id,
            username,
            first_name,
            last_activity: Instant::now(),
            created_at: Instant::now(),
            state: SessionState::default(),
            failed_attempts: 0,
        }
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if session is active (authenticated)
    pub fn is_authenticated(&self) -> bool {
        matches!(self.state, SessionState::Active)
    }
}

// ============================================================================
// DISCOVERY TYPES
// ============================================================================

/// Discovered chat during chat discovery mode
#[derive(Debug, Clone)]
pub struct DiscoveredChat {
    pub chat_id: i64,
    pub user_id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub chat_type: String,
    pub message_preview: Option<String>,
    pub discovered_at: Instant,
}

// ============================================================================
// BOT TYPES
// ============================================================================

/// Telegram bot state
#[derive(Debug, Clone, PartialEq)]
pub enum BotState {
    /// No token or not started
    Disconnected,
    /// Has token, polling for chat discovery (no chat_id yet)
    Discovery,
    /// Fully operational with a configured chat
    Connected,
}

impl Default for BotState {
    fn default() -> Self {
        Self::Disconnected
    }
}
