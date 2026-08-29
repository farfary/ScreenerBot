//! Errors produced by the LLM analysis module: model-scored filtering, entry
//! and exit decisions, the response cache, and the decision/instruction
//! SQLite persistence.

use std::time::Duration;

use crate::errors::{DataError, DatabaseError, ErrorClass, InternalError, IoError, Severity};

/// Everything that can go wrong building, scoring or persisting an LLM
/// analysis decision.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A query/prepare/transaction failure against the analysis database.
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// A task join failure, a poisoned lock, or an unfulfillable permit
    /// acquisition — invariant violations, not data problems.
    #[error(transparent)]
    Internal(#[from] InternalError),
    /// A provider response body could not be parsed against its decision
    /// schema.
    #[error(transparent)]
    Data(#[from] DataError),
    /// The data directory for the analysis database could not be created.
    #[error(transparent)]
    Io(#[from] IoError),
    /// An LLM provider client failure (model call, HTTP transport).
    #[error(transparent)]
    Apis(#[from] crate::apis::Error),

    /// LLM analysis is turned off via configuration.
    #[error("LLM analysis is disabled")]
    Disabled,
    /// The configured provider has no client/model set up.
    #[error("provider {provider} is not configured")]
    ProviderNotConfigured { provider: String },
    /// A provider rejected the request with a rate limit.
    #[error("rate limited{}", retry_after_secs.map(|s| format!(", retry after {s}s")).unwrap_or_default())]
    RateLimited { retry_after_secs: Option<u64> },
    /// An LLM call did not complete within its deadline.
    #[error("the analysis request timed out after {waited_ms}ms")]
    Timeout { waited_ms: u64 },
    /// An evaluation input failed validation before any provider work was
    /// attempted.
    #[error("invalid parameters: {detail}")]
    InvalidParameters { detail: String },
}

/// Result alias for the LLM analysis module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Database(e) => e.is_retryable(),
            Error::Internal(e) => e.is_retryable(),
            Error::Data(e) => e.is_retryable(),
            Error::Io(e) => e.is_retryable(),
            Error::Apis(e) => e.is_retryable(),
            Error::RateLimited { .. } | Error::Timeout { .. } => true,
            Error::Disabled
            | Error::ProviderNotConfigured { .. }
            | Error::InvalidParameters { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Database(e) => e.retry_after(),
            Error::Internal(e) => e.retry_after(),
            Error::Data(e) => e.retry_after(),
            Error::Io(e) => e.retry_after(),
            Error::Apis(e) => e.retry_after(),
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
            Error::Disabled => Severity::Info,
            Error::ProviderNotConfigured { .. } => Severity::Warning,
            Error::RateLimited { .. } => Severity::Warning,
            Error::Timeout { .. } => Severity::Warning,
            Error::InvalidParameters { .. } => Severity::Info,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Database(e) => e.http_status(),
            Error::Internal(e) => e.http_status(),
            Error::Data(e) => e.http_status(),
            Error::Io(e) => e.http_status(),
            Error::Apis(e) => e.http_status(),
            Error::Disabled => 409,
            Error::ProviderNotConfigured { .. } => 503,
            Error::RateLimited { .. } => 429,
            Error::Timeout { .. } => 504,
            Error::InvalidParameters { .. } => 400,
        }
    }
}
