//! Errors produced by the reset module.

use std::time::Duration;

use crate::errors::{DatabaseError, ErrorClass, IoError, Severity};

/// Everything that can go wrong while clearing runtime state.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// One or more requested reset targets could not be removed.
    #[error("reset completed with {count} errors")]
    CompletedWithErrors { count: usize },

    /// Opening or preparing the positions database failed.
    #[error(transparent)]
    Database(#[from] DatabaseError),

    /// Reading confirmation input or removing a runtime target failed.
    #[error(transparent)]
    Io(#[from] IoError),
}

/// Result alias for the reset module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::CompletedWithErrors { .. } => false,
            Error::Database(error) => error.is_retryable(),
            Error::Io(error) => error.is_retryable(),
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::CompletedWithErrors { .. } => None,
            Error::Database(error) => error.retry_after(),
            Error::Io(error) => error.retry_after(),
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::CompletedWithErrors { .. } => Severity::Error,
            Error::Database(error) => error.severity(),
            Error::Io(error) => error.severity(),
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::CompletedWithErrors { .. } => 500,
            Error::Database(error) => error.http_status(),
            Error::Io(error) => error.http_status(),
        }
    }
}
