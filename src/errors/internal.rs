//! Internal/invariant errors.
//!
//! This is the place for:
//! - task join failures (unexpected panics)
//! - timeouts/cancellation glue errors
//! - violated invariants ("should never happen")
//!
//! Keep errors `Clone` by storing messages as strings.

#[derive(Debug, Clone, thiserror::Error)]
pub enum InternalError {
    #[error("invariant violation: {message}")]
    InvariantViolation { message: String },
    #[error("task join error: {message}")]
    TaskJoin { message: String },
    #[error("timeout: {message}")]
    Timeout { message: String },
    /// A caller requested an operation this owner does not implement.
    /// Distinct from an invariant violation: the request is well-formed,
    /// the capability is simply absent (e.g. wallet-scoped swap execution
    /// on a stub router).
    #[error("unsupported capability '{capability}' on {owner}")]
    UnsupportedCapability { capability: String, owner: String },
}

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
