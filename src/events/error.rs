//! Errors produced by the events module.

use std::time::Duration;

use crate::errors::{DataError, DatabaseError, ErrorClass, InternalError, Severity};

/// Everything that can go wrong while recording or querying events.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// SQLite storage could not complete an event operation.
    #[error(transparent)]
    Database(#[from] DatabaseError),

    /// Event payload serialization failed.
    #[error(transparent)]
    Data(#[from] DataError),

    /// The event subsystem encountered an unexpected internal failure.
    #[error(transparent)]
    Internal(#[from] InternalError),

    /// An event operation was requested before the subsystem was initialized.
    #[error("events system is not initialized")]
    NotInitialized,

    /// A stored event row could not be decoded.
    #[error("could not decode column {column} of an event row: {detail}")]
    RowDecode {
        column: &'static str,
        detail: String,
    },
}

/// Result alias for the events module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Database(error) => error.is_retryable(),
            Error::Data(error) => error.is_retryable(),
            Error::Internal(error) => error.is_retryable(),
            Error::NotInitialized | Error::RowDecode { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Database(error) => error.retry_after(),
            Error::Data(error) => error.retry_after(),
            Error::Internal(error) => error.retry_after(),
            Error::NotInitialized | Error::RowDecode { .. } => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Database(error) => error.severity(),
            Error::Data(error) => error.severity(),
            Error::Internal(error) => error.severity(),
            Error::NotInitialized => Severity::Warning,
            Error::RowDecode { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Database(error) => error.http_status(),
            Error::Data(error) => error.http_status(),
            Error::Internal(error) => error.http_status(),
            Error::NotInitialized => 503,
            Error::RowDecode { .. } => 500,
        }
    }
}
