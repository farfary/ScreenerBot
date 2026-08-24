//! Errors returned while constructing or parsing chain identities.

use std::time::Duration;

use crate::chains::{ChainId, ExecutionFailure};
use crate::errors::{ErrorClass, Severity};

/// A validation error for a chain-neutral identity, or an execution failure
/// bubbled up from a chain adapter.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// The supplied chain name is not supported by this build.
    #[error("unsupported chain: {value}")]
    UnsupportedChain { value: String },
    /// An identity value was empty after whitespace was removed.
    #[error("{kind} cannot be empty")]
    EmptyIdentifier { kind: &'static str },
    /// The identity belongs to a different chain than the conversion requested.
    #[error("expected {expected} account, got {actual}")]
    WrongChain { expected: ChainId, actual: ChainId },
    /// The account address is not valid for the identity's chain.
    #[error("invalid {chain} account '{value}'")]
    InvalidAccount { chain: ChainId, value: String },
    /// An on-chain execution attempt failed.
    #[error(transparent)]
    Execution(#[from] ExecutionFailure),
}

/// Result alias for the chains module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Execution(e) => e.is_retryable(),
            Error::UnsupportedChain { .. }
            | Error::EmptyIdentifier { .. }
            | Error::WrongChain { .. }
            | Error::InvalidAccount { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Execution(e) => e.retry_after(),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Execution(e) => e.severity(),
            Error::UnsupportedChain { .. }
            | Error::EmptyIdentifier { .. }
            | Error::WrongChain { .. }
            | Error::InvalidAccount { .. } => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Execution(e) => e.http_status(),
            Error::UnsupportedChain { .. }
            | Error::EmptyIdentifier { .. }
            | Error::WrongChain { .. }
            | Error::InvalidAccount { .. } => 400,
        }
    }
}
