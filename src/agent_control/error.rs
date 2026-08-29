//! Errors produced by the shared agent-control boundary: tool-argument
//! validation, tool-policy persistence, the durable pairing/approval/audit
//! store and the live-app bridge.
//!
//! This boundary deliberately does not depend on `crate::assistant` or
//! `crate::llm_analysis` error types — a transport adapter that calls `decide`
//! or a tool must not have to import a consumer's error enum.

use std::time::Duration;

use crate::errors::{DatabaseError, ErrorClass, Severity};

/// Everything that can go wrong validating a tool call, persisting the tool
/// permission policy, or working with the pairing/approval store.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// Reading or writing the tool permission policy in configuration failed.
    #[error(transparent)]
    Config(#[from] crate::config::Error),
    /// A tool argument was missing, of the wrong type, or out of range.
    #[error("invalid parameters: {detail}")]
    InvalidParameters { detail: String },
    /// The agent-control store (pairings/approvals/audit) failed.
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// A management request supplied an out-of-bounds label, agent kind or
    /// scope. The `detail` is safe to show a dashboard operator; it never
    /// echoes a secret.
    #[error("invalid pairing request: {detail}")]
    InvalidPairingRequest { detail: String },
    /// A bridge request presented a credential that is missing, malformed,
    /// unknown or revoked. The message is deliberately identical for every
    /// one of those cases so it cannot be used as an oracle.
    #[error("pairing credential rejected")]
    PairingRejected,
    /// The agent-control surface is switched off (`agent_control.enabled`).
    #[error("agent control is disabled")]
    Disabled,
    /// An approval could not be resolved because it is not pending any more —
    /// already decided, executed, expired or revoked. Resolving is
    /// exactly-once, so a lost race lands here.
    #[error("approval is no longer pending")]
    ApprovalNotPending,
    /// The referenced approval id does not exist for this client.
    #[error("approval not found")]
    ApprovalNotFound,
}

/// A raw SQLite failure in the pairing/approval/audit store surfaces as a
/// `Database` error, so every store call site can just use `?`.
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Database(DatabaseError::from(e))
    }
}

/// Result alias for the agent-control boundary.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Config(e) => e.is_retryable(),
            Error::Database(e) => e.is_retryable(),
            Error::InvalidParameters { .. }
            | Error::InvalidPairingRequest { .. }
            | Error::PairingRejected
            | Error::Disabled
            | Error::ApprovalNotPending
            | Error::ApprovalNotFound => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Config(e) => e.retry_after(),
            Error::Database(e) => e.retry_after(),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Config(e) => e.severity(),
            Error::Database(e) => e.severity(),
            Error::PairingRejected => Severity::Warning,
            Error::InvalidParameters { .. }
            | Error::InvalidPairingRequest { .. }
            | Error::Disabled
            | Error::ApprovalNotPending
            | Error::ApprovalNotFound => Severity::Info,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Config(e) => e.http_status(),
            Error::Database(e) => e.http_status(),
            Error::InvalidParameters { .. } | Error::InvalidPairingRequest { .. } => 400,
            // A rejected credential is answered as 401 with a fixed body; an
            // unknown vs revoked vs malformed credential must not be
            // distinguishable from the outside.
            Error::PairingRejected => 401,
            Error::Disabled => 403,
            Error::ApprovalNotPending => 409,
            Error::ApprovalNotFound => 404,
        }
    }
}
