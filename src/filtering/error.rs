//! Errors produced by the filtering module.

use std::time::Duration;

use crate::errors::{ErrorClass, InternalError, Severity};

/// Everything that can go wrong while filtering tokens.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A timeout or other internal failure prevented a filtering operation.
    #[error(transparent)]
    Internal(#[from] InternalError),

    /// Loading or counting a token set from storage failed. The token store's
    /// own typed error is kept as the source rather than flattened into text,
    /// so callers keep its classification and `kind` still says which set.
    #[error("could not load the {kind} token set")]
    TokenSetLoad {
        kind: &'static str,
        #[source]
        source: crate::tokens::Error,
    },
}

/// Result alias for the filtering module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Internal(e) => e.is_retryable(),
            Error::TokenSetLoad { source, .. } => source.is_retryable(),
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Internal(e) => e.retry_after(),
            Error::TokenSetLoad { source, .. } => source.retry_after(),
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Internal(e) => e.severity(),
            Error::TokenSetLoad { source, .. } => source.severity(),
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Internal(e) => e.http_status(),
            Error::TokenSetLoad { source, .. } => source.http_status(),
        }
    }
}
