//! Errors produced by the ai module: LLM decisions, chat, tool execution,
//! scheduled automation, and their SQLite persistence.

use std::time::Duration;

use crate::errors::{DataError, DatabaseError, ErrorClass, InternalError, IoError, Severity};

/// Everything that can go wrong evaluating, chatting with, or scheduling the
/// AI assistant.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A query/prepare/transaction failure against the ai or ai-chat
    /// databases.
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// A task join failure, a poisoned lock, or an unfulfillable permit
    /// acquisition — invariant violations, not data problems.
    #[error(transparent)]
    Internal(#[from] InternalError),
    /// A response body could not be parsed against its expected schema.
    #[error(transparent)]
    Data(#[from] DataError),
    /// The data directory for the ai-chat database could not be created.
    #[error(transparent)]
    Io(#[from] IoError),
    /// An external API client failure (LLM provider call, HTTP transport).
    #[error(transparent)]
    Apis(#[from] crate::apis::Error),
    /// Reading or updating configuration failed.
    #[error(transparent)]
    Config(#[from] crate::config::Error),

    /// The AI module is turned off via configuration.
    #[error("the AI module is disabled")]
    Disabled,
    /// The configured provider has no client/model set up.
    #[error("provider {provider} is not configured")]
    ProviderNotConfigured { provider: String },
    /// A provider rejected the request with a rate limit.
    #[error("rate limited{}", retry_after_secs.map(|s| format!(", retry after {s}s")).unwrap_or_default())]
    RateLimited { retry_after_secs: Option<u64> },
    /// An LLM call did not complete within its deadline.
    #[error("the AI request timed out after {waited_ms}ms")]
    Timeout { waited_ms: u64 },
    /// A caller-supplied argument (chat request, config value, schedule
    /// value) failed validation before any work was attempted.
    #[error("invalid parameters: {detail}")]
    InvalidParameters { detail: String },
    /// A schedule type string did not match one of the known schedule kinds.
    #[error("'{value}' is not a known schedule type")]
    UnknownScheduleType { value: String },
    /// A scheduled task referenced by ID does not exist.
    #[error("scheduled task {task_id} not found")]
    TaskNotFound { task_id: i64 },
    /// Recording the start/completion of a scheduled run failed.
    #[error("could not record the {phase} of scheduled run {run_id}: {detail}")]
    RunRecord {
        phase: &'static str,
        run_id: String,
        detail: String,
    },
    /// A dependency this module calls into has not migrated off stringly
    /// typed errors yet; carries its rendered failure text until it does.
    #[error("{dependency} failed: {detail}")]
    Dependency { dependency: String, detail: String },
}

/// Result alias for the ai module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Database(e) => e.is_retryable(),
            Error::Internal(e) => e.is_retryable(),
            Error::Data(e) => e.is_retryable(),
            Error::Io(e) => e.is_retryable(),
            Error::Apis(e) => e.is_retryable(),
            Error::Config(e) => e.is_retryable(),
            Error::RateLimited { .. } | Error::Timeout { .. } => true,
            Error::Disabled
            | Error::ProviderNotConfigured { .. }
            | Error::InvalidParameters { .. }
            | Error::UnknownScheduleType { .. }
            | Error::TaskNotFound { .. }
            | Error::RunRecord { .. }
            | Error::Dependency { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Database(e) => e.retry_after(),
            Error::Internal(e) => e.retry_after(),
            Error::Data(e) => e.retry_after(),
            Error::Io(e) => e.retry_after(),
            Error::Apis(e) => e.retry_after(),
            Error::Config(e) => e.retry_after(),
            Error::RateLimited { retry_after_secs } => Some(
                retry_after_secs
                    .map(Duration::from_secs)
                    .unwrap_or(Duration::from_secs(1)),
            ),
            Error::Timeout { .. } => Some(Duration::from_secs(1)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Database(e) => e.severity(),
            Error::Internal(e) => e.severity(),
            Error::Data(e) => e.severity(),
            Error::Io(e) => e.severity(),
            Error::Apis(e) => e.severity(),
            Error::Config(e) => e.severity(),
            Error::Disabled => Severity::Info,
            Error::ProviderNotConfigured { .. } => Severity::Warning,
            Error::RateLimited { .. } => Severity::Warning,
            Error::Timeout { .. } => Severity::Warning,
            Error::InvalidParameters { .. } | Error::UnknownScheduleType { .. } => Severity::Info,
            Error::TaskNotFound { .. } => Severity::Warning,
            Error::RunRecord { .. } => Severity::Critical,
            Error::Dependency { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Database(e) => e.http_status(),
            Error::Internal(e) => e.http_status(),
            Error::Data(e) => e.http_status(),
            Error::Io(e) => e.http_status(),
            Error::Apis(e) => e.http_status(),
            Error::Config(e) => e.http_status(),
            Error::Disabled => 409,
            Error::ProviderNotConfigured { .. } => 503,
            Error::RateLimited { .. } => 429,
            Error::Timeout { .. } => 504,
            Error::InvalidParameters { .. } | Error::UnknownScheduleType { .. } => 400,
            Error::TaskNotFound { .. } => 404,
            Error::RunRecord { .. } => 500,
            Error::Dependency { .. } => 502,
        }
    }
}
