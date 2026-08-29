//! Errors produced by the in-app assistant: dashboard conversation, its
//! SQLite session/message persistence, and scheduled-conversation automation
//! (schedule parsing, task lookup, run-record bookkeeping).

use std::time::Duration;

use crate::errors::{DatabaseError, ErrorClass, InternalError, IoError, Severity};

/// Everything that can go wrong holding a conversation, persisting it, or
/// running a saved instruction on a timer.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A query/prepare/transaction failure against the assistant chat or
    /// scheduled-task databases.
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// A task join failure, a poisoned lock, or a missing singleton — invariant
    /// violations, not data problems.
    #[error(transparent)]
    Internal(#[from] InternalError),
    /// The data directory for the assistant chat database could not be created.
    #[error(transparent)]
    Io(#[from] IoError),
    /// An LLM provider client failure while running a turn.
    #[error(transparent)]
    Apis(#[from] crate::apis::Error),

    /// A caller-supplied argument (chat request, schedule value, tool
    /// permission string) failed validation before any work was attempted.
    #[error("invalid parameters: {detail}")]
    InvalidParameters { detail: String },
    /// The configured provider has no client/model set up for a chat turn.
    #[error("provider {provider} is not configured")]
    ProviderNotConfigured { provider: String },
    /// A chat turn did not complete within its deadline.
    #[error("the assistant request timed out after {waited_ms}ms")]
    Timeout { waited_ms: u64 },
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
}

/// Result alias for the assistant module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Database(e) => e.is_retryable(),
            Error::Internal(e) => e.is_retryable(),
            Error::Io(e) => e.is_retryable(),
            Error::Apis(e) => e.is_retryable(),
            Error::Timeout { .. } => true,
            Error::InvalidParameters { .. }
            | Error::ProviderNotConfigured { .. }
            | Error::UnknownScheduleType { .. }
            | Error::TaskNotFound { .. }
            | Error::RunRecord { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Database(e) => e.retry_after(),
            Error::Internal(e) => e.retry_after(),
            Error::Io(e) => e.retry_after(),
            Error::Apis(e) => e.retry_after(),
            Error::Timeout { .. } => Some(Duration::from_secs(1)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Database(e) => e.severity(),
            Error::Internal(e) => e.severity(),
            Error::Io(e) => e.severity(),
            Error::Apis(e) => e.severity(),
            Error::ProviderNotConfigured { .. } => Severity::Warning,
            Error::Timeout { .. } => Severity::Warning,
            Error::InvalidParameters { .. } | Error::UnknownScheduleType { .. } => Severity::Info,
            Error::TaskNotFound { .. } => Severity::Warning,
            Error::RunRecord { .. } => Severity::Critical,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Database(e) => e.http_status(),
            Error::Internal(e) => e.http_status(),
            Error::Io(e) => e.http_status(),
            Error::Apis(e) => e.http_status(),
            Error::ProviderNotConfigured { .. } => 503,
            Error::Timeout { .. } => 504,
            Error::InvalidParameters { .. } | Error::UnknownScheduleType { .. } => 400,
            Error::TaskNotFound { .. } => 404,
            Error::RunRecord { .. } => 500,
        }
    }
}
