//! Internal/invariant errors.
//!
//! This is the place for:
//! - task join failures (unexpected panics)
//! - timeouts/cancellation glue errors
//! - violated invariants ("should never happen")
//!
//! Keep errors `Clone` by storing messages as strings.

#[derive(Debug, Clone)]
pub enum InternalError {
    InvariantViolation {
        message: String,
    },
    TaskJoin {
        message: String,
    },
    Timeout {
        message: String,
    },
    Generic {
        message: String,
    },
    /// A caller requested an operation this owner does not implement.
    /// Distinct from an invariant violation: the request is well-formed,
    /// the capability is simply absent (e.g. wallet-scoped swap execution
    /// on a stub router).
    UnsupportedCapability {
        capability: String,
        owner: String,
    },
}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InternalError::InvariantViolation { message } => {
                write!(f, "Invariant violation: {message}")
            }
            InternalError::TaskJoin { message } => write!(f, "Task join error: {message}"),
            InternalError::Timeout { message } => write!(f, "Timeout: {message}"),
            InternalError::Generic { message } => write!(f, "{message}"),
            InternalError::UnsupportedCapability { capability, owner } => {
                write!(f, "Unsupported capability '{capability}' on {owner}")
            }
        }
    }
}

impl std::error::Error for InternalError {}

impl From<tokio::task::JoinError> for InternalError {
    fn from(err: tokio::task::JoinError) -> Self {
        InternalError::TaskJoin {
            message: err.to_string(),
        }
    }
}

impl From<tokio::time::error::Elapsed> for InternalError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        InternalError::Timeout {
            message: err.to_string(),
        }
    }
}
