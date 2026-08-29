//! Errors produced by the shared agent-control boundary: tool-argument
//! validation and tool-policy persistence.
//!
//! This boundary deliberately does not depend on `crate::assistant` or
//! `crate::llm_analysis` error types — a transport adapter that calls `decide`
//! or a tool must not have to import a consumer's error enum.

use std::time::Duration;

use crate::errors::{ErrorClass, Severity};

/// Everything that can go wrong validating a tool call or persisting the tool
/// permission policy.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// Reading or writing the tool permission policy in configuration failed.
    #[error(transparent)]
    Config(#[from] crate::config::Error),
    /// A tool argument was missing, of the wrong type, or out of range.
    #[error("invalid parameters: {detail}")]
    InvalidParameters { detail: String },
}

/// Result alias for the agent-control boundary.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Config(e) => e.is_retryable(),
            Error::InvalidParameters { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Config(e) => e.retry_after(),
            Error::InvalidParameters { .. } => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Config(e) => e.severity(),
            Error::InvalidParameters { .. } => Severity::Info,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Config(e) => e.http_status(),
            Error::InvalidParameters { .. } => 400,
        }
    }
}
