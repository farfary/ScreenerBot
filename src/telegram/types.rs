// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Core types for the Telegram module
//!
//! Contains notification types, session types, and discovery types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::events::ScheduledTaskOutcome;
use crate::i18n::{ids, UiText};
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
        amount_native: f64,
        wallet: String, // external wallet that traded
    },

    /// Notification when a new position is opened
    PositionOpened {
        token_symbol: String,
        token_mint: String,
        amount_native: f64,
        entry_price: f64,
        ai_reasoning: Option<String>,
    },

    /// Notification when a position is closed
    PositionClosed {
        token_symbol: String,
        token_mint: String,
        pnl_native: f64,
        pnl_percent: f64,
        exit_reason: Option<String>,
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
        pnl_native: f64,
        remaining_percent: f64,
    },

    /// Notification when DCA is executed
    DcaExecuted {
        token_symbol: String,
        token_mint: String,
        dca_amount_native: f64,
        total_invested_native: f64,
        dca_count: u32,
    },

    /// A swap confirmed on chain that its position does not hold yet; it is
    /// verified again until it is booked
    SwapUnbooked {
        token_symbol: String,
        token_mint: String,
        signature: String,
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
        total_pnl_native: f64,
        open_positions: u32,
    },

    /// Result of a scheduled assistant task run. `detail` is the agent's own
    /// output for a completed run and the failure text otherwise.
    ScheduledTaskResult {
        task_name: String,
        outcome: ScheduledTaskOutcome,
        detail: String,
    },

    /// Bot startup notification
    BotStarted { version: String, mode: StartMode },

    /// Bot shutdown notification
    BotStopped { reason: StopReason },

    /// Progress of an application update
    UpdateStatus {
        version: String,
        stage: UpdateStage,
        /// Whether the release installs without any operating-system dialog.
        silent: bool,
        /// Bytes that have to be transferred to apply it.
        transfer_bytes: u64,
    },

    /// A copy-trading fill, exit, failure or auto-pause
    CopyTrading {
        /// Task name; `None` shows the localized "Task #id".
        task: Option<String>,
        task_id: i64,
        title: UiText,
        token_symbol: Option<String>,
        token_mint: Option<String>,
        detail: UiText,
        /// Paper decisions are gated by their own preference.
        paper: bool,
    },

    /// Notification when new tokens are found by filtering
    NewTokensFound {
        session_id: String,
        new_count: usize,
    },
}

/// How the bot was started.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StartMode {
    Normal,
}

impl StartMode {
    pub fn ui_text(self) -> UiText {
        match self {
            Self::Normal => UiText::new(ids::TELEGRAM_NOTIFY_START_MODE_NORMAL),
        }
    }
}

/// Why the bot stopped.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StopReason {
    Graceful,
}

impl StopReason {
    pub fn ui_text(self) -> UiText {
        match self {
            Self::Graceful => UiText::new(ids::TELEGRAM_NOTIFY_STOP_REASON_GRACEFUL),
        }
    }
}

/// Where an update has reached in its lifecycle.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateStage {
    /// A newer release was found.
    Available,
    /// Its artifact is downloaded and verified, waiting to be applied.
    Staged,
    /// The backend is restarting onto it.
    Applying,
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
        amount_native: f64,
        wallet: String,
    ) -> Self {
        Self::new(NotificationType::TradeAlert {
            token_symbol,
            token_mint,
            trade_type: trade_type.to_string(),
            amount_native,
            wallet,
        })
    }

    /// Create an update-status notification
    pub fn update_status(
        version: String,
        stage: UpdateStage,
        silent: bool,
        transfer_bytes: u64,
    ) -> Self {
        Self::new(NotificationType::UpdateStatus {
            version,
            stage,
            silent,
            transfer_bytes,
        })
    }

    /// Create a position opened notification
    pub fn position_opened(
        token_symbol: String,
        token_mint: String,
        amount_native: f64,
        entry_price: f64,
    ) -> Self {
        Self::new(NotificationType::PositionOpened {
            token_symbol,
            token_mint,
            amount_native,
            entry_price,
            ai_reasoning: None,
        })
    }

    /// Create a position opened notification with AI reasoning
    pub fn position_opened_with_ai(
        token_symbol: String,
        token_mint: String,
        amount_native: f64,
        entry_price: f64,
        ai_reasoning: Option<String>,
    ) -> Self {
        Self::new(NotificationType::PositionOpened {
            token_symbol,
            token_mint,
            amount_native,
            entry_price,
            ai_reasoning,
        })
    }

    /// Create a position closed notification
    pub fn position_closed(
        token_symbol: String,
        token_mint: String,
        pnl_native: f64,
        pnl_percent: f64,
        exit_reason: Option<String>,
        entry_price: f64,
        exit_price: f64,
        invested: f64,
        received: f64,
        duration_secs: u64,
    ) -> Self {
        Self::new(NotificationType::PositionClosed {
            token_symbol,
            token_mint,
            pnl_native,
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
        pnl_native: f64,
        pnl_percent: f64,
        exit_reason: Option<String>,
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
            pnl_native,
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
        pnl_native: f64,
        remaining_percent: f64,
    ) -> Self {
        Self::new(NotificationType::PartialExit {
            token_symbol,
            token_mint,
            exit_percent,
            pnl_native,
            remaining_percent,
        })
    }

    /// Create a DCA executed notification
    pub fn dca_executed(
        token_symbol: String,
        token_mint: String,
        dca_amount_native: f64,
        total_invested_native: f64,
        dca_count: u32,
    ) -> Self {
        Self::new(NotificationType::DcaExecuted {
            token_symbol,
            token_mint,
            dca_amount_native,
            total_invested_native,
            dca_count,
        })
    }

    /// Create a notification for a confirmed swap its position does not hold yet
    pub fn swap_unbooked(token_symbol: String, token_mint: String, signature: String) -> Self {
        Self::new(NotificationType::SwapUnbooked {
            token_symbol,
            token_mint,
            signature,
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
        total_pnl_native: f64,
        open_positions: u32,
    ) -> Self {
        Self::new(NotificationType::DailySummary {
            date,
            total_trades,
            winning_trades,
            losing_trades,
            total_pnl_native,
            open_positions,
        })
    }

    /// Create a scheduled task result notification
    pub fn scheduled_task_result(
        task_name: String,
        outcome: ScheduledTaskOutcome,
        detail: String,
    ) -> Self {
        Self::new(NotificationType::ScheduledTaskResult {
            task_name,
            outcome,
            detail,
        })
    }

    /// Create a bot started notification
    pub fn bot_started(version: String, mode: StartMode) -> Self {
        Self::new(NotificationType::BotStarted { version, mode })
    }

    /// Create a bot stopped notification
    pub fn bot_stopped(reason: StopReason) -> Self {
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
