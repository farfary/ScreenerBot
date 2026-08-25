//! Errors produced by the process module.

use std::time::Duration;

use crate::errors::{ErrorClass, IoError, Severity};

/// Everything that can go wrong while managing the process lifecycle.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A filesystem operation required by the process lifecycle failed.
    #[error(transparent)]
    Io(#[from] IoError),

    /// Another ScreenerBot instance already holds the advisory lock. Named
    /// rather than an invariant violation: it is the expected outcome of a
    /// second launch, and the boot path turns it into a specific remedy.
    #[error("another ScreenerBot instance holds the process lock at {path}")]
    LockHeld { path: String },
}

/// Result alias for the process module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Io(error) => error.is_retryable(),
            Error::LockHeld { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Io(error) => error.retry_after(),
            Error::LockHeld { .. } => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Io(error) => error.severity(),
            Error::LockHeld { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Io(error) => error.http_status(),
            Error::LockHeld { .. } => 409,
        }
    }
}
