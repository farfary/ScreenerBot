//! Chain-neutral classification of on-chain execution failures.
//!
//! Every chain adapter maps its native failure vocabulary onto this set so
//! shared code (retry loops, severity reporting) never needs to know which
//! chain produced the failure.

use std::time::Duration;

use crate::errors::{ErrorClass, Severity};

/// Why an on-chain execution failed, in terms every chain can express.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ExecutionFailure {
    #[error("transaction {reference} was not found on chain")]
    NotFound { reference: String },
    #[error("confirmation for {reference} timed out after {waited_ms}ms")]
    ConfirmationTimeout { reference: String, waited_ms: u64 },
    #[error("the node has not yet indexed {reference}")]
    IndexingDelay { reference: String },
}

impl ErrorClass for ExecutionFailure {
    fn is_retryable(&self) -> bool {
        matches!(self, ExecutionFailure::IndexingDelay { .. })
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            ExecutionFailure::IndexingDelay { .. } => Some(Duration::from_secs(2)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            ExecutionFailure::NotFound { .. } => Severity::Warning,
            ExecutionFailure::ConfirmationTimeout { .. } => Severity::Warning,
            ExecutionFailure::IndexingDelay { .. } => Severity::Info,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            ExecutionFailure::NotFound { .. } => 404,
            ExecutionFailure::ConfirmationTimeout { .. } => 504,
            ExecutionFailure::IndexingDelay { .. } => 503,
        }
    }
}
