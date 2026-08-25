//! Errors produced by the telegram module: bot lifecycle, notifications,
//! sessions, chat discovery, and command/callback handling.

use std::time::Duration;

use crate::errors::{ErrorClass, InternalError, Severity};

/// Everything that can go wrong operating the Telegram bot.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A poisoned lock on the global bot/notifier state — an invariant
    /// violation, not a data problem.
    #[error(transparent)]
    Internal(#[from] InternalError),
    /// Updating configuration failed.
    #[error(transparent)]
    Config(#[from] crate::config::Error),

    /// The bot has no usable token/chat ID configured yet, or the global
    /// bot/notifier instance has not been initialised.
    #[error("the telegram bot is not configured")]
    NotConfigured,
    /// The configured bot token was rejected by the Telegram API.
    #[error("the configured bot token is not usable: {detail}")]
    InvalidBotToken { detail: String },
    /// The configured chat ID string does not parse as a Telegram chat ID.
    #[error("chat id '{chat_id}' is invalid: {detail}")]
    InvalidChatId { chat_id: String, detail: String },
    /// A Telegram API call to send, edit, or answer a message failed.
    #[error("could not send a message to chat {chat_id}: {detail}")]
    SendFailed { chat_id: String, detail: String },
    /// No tracked session exists for this user.
    #[error("no session found for user {user_id}")]
    SessionNotFound { user_id: i64 },
    /// Too many failed login attempts; locked out for a cooldown.
    #[error("account locked, try again in {remaining_secs}s")]
    AccountLocked { remaining_secs: u64 },
    /// A TOTP code was submitted outside the awaiting-TOTP session state.
    #[error("not awaiting a TOTP verification")]
    NotAwaitingTotp,
    /// 2FA has not been set up for the shared lockscreen/telegram login flow.
    #[error("2FA is not configured; enable it in security settings first")]
    TotpNotConfigured,
    /// The shared lockscreen TOTP verifier rejected the request.
    #[error("TOTP verification failed: {detail}")]
    TotpVerificationFailed { detail: String },
    /// The discovery polling service was started while already running.
    #[error("chat discovery is already running")]
    DiscoveryAlreadyRunning,
    /// A discovered-chat lookup by ID found nothing.
    #[error("chat {chat_id} was not found in the discovered list")]
    ChatNotFound { chat_id: i64 },
}

/// Result alias for the telegram module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Internal(e) => e.is_retryable(),
            Error::Config(e) => e.is_retryable(),
            Error::SendFailed { .. } => true,
            Error::NotConfigured
            | Error::InvalidBotToken { .. }
            | Error::InvalidChatId { .. }
            | Error::SessionNotFound { .. }
            | Error::AccountLocked { .. }
            | Error::NotAwaitingTotp
            | Error::TotpNotConfigured
            | Error::TotpVerificationFailed { .. }
            | Error::DiscoveryAlreadyRunning
            | Error::ChatNotFound { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Internal(e) => e.retry_after(),
            Error::Config(e) => e.retry_after(),
            Error::SendFailed { .. } => Some(Duration::from_secs(1)),
            Error::AccountLocked { remaining_secs } => Some(Duration::from_secs(*remaining_secs)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Internal(e) => e.severity(),
            Error::Config(e) => e.severity(),
            Error::NotConfigured => Severity::Warning,
            Error::InvalidBotToken { .. } => Severity::Critical,
            Error::InvalidChatId { .. } => Severity::Error,
            Error::SendFailed { .. } => Severity::Warning,
            Error::SessionNotFound { .. } => Severity::Warning,
            Error::AccountLocked { .. } => Severity::Warning,
            Error::NotAwaitingTotp => Severity::Info,
            Error::TotpNotConfigured => Severity::Warning,
            Error::TotpVerificationFailed { .. } => Severity::Error,
            Error::DiscoveryAlreadyRunning => Severity::Info,
            Error::ChatNotFound { .. } => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Internal(e) => e.http_status(),
            Error::Config(e) => e.http_status(),
            Error::NotConfigured => 503,
            Error::InvalidBotToken { .. } => 401,
            Error::InvalidChatId { .. } => 400,
            Error::SendFailed { .. } => 502,
            Error::SessionNotFound { .. } => 404,
            Error::AccountLocked { .. } => 423,
            Error::NotAwaitingTotp => 409,
            Error::TotpNotConfigured => 412,
            Error::TotpVerificationFailed { .. } => 502,
            Error::DiscoveryAlreadyRunning => 409,
            Error::ChatNotFound { .. } => 404,
        }
    }
}
