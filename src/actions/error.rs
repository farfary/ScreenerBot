//! Errors produced by the actions module.

use std::time::Duration;

use crate::errors::{DataError, DatabaseError, ErrorClass, InternalError, Severity};

/// Everything that can go wrong while tracking actions.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Data(#[from] DataError),
    #[error(transparent)]
    Internal(#[from] InternalError),
    #[error("actions database is not initialized")]
    NotInitialized,
    #[error("'{value}' is not a known action type")]
    UnknownActionType { value: String },
}

/// Result alias for the actions module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Database(error) => error.is_retryable(),
            Error::Data(error) => error.is_retryable(),
            Error::Internal(error) => error.is_retryable(),
            Error::NotInitialized | Error::UnknownActionType { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Database(error) => error.retry_after(),
            Error::Data(error) => error.retry_after(),
            Error::Internal(error) => error.retry_after(),
            Error::NotInitialized | Error::UnknownActionType { .. } => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Database(error) => error.severity(),
            Error::Data(error) => error.severity(),
            Error::Internal(error) => error.severity(),
            Error::NotInitialized => Severity::Error,
            Error::UnknownActionType { .. } => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Database(error) => error.http_status(),
            Error::Data(error) => error.http_status(),
            Error::Internal(error) => error.http_status(),
            Error::NotInitialized => 503,
            Error::UnknownActionType { .. } => 400,
        }
    }
}
