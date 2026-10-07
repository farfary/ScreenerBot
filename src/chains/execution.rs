// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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
    /// The transaction landed and the chain reports it failed: it moved
    /// nothing but its fee, and it can never land again.
    #[error("transaction {reference} failed on chain: {detail}")]
    Reverted { reference: String, detail: String },
    /// The transaction was never seen and the validity window it was signed
    /// for has provably closed: it can never land, on any node.
    #[error("transaction {reference} was never seen and expired after block {last_valid_block_height} (current block {current_block_height})")]
    Expired {
        reference: String,
        last_valid_block_height: u64,
        current_block_height: u64,
    },
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
            ExecutionFailure::Reverted { .. } => Severity::Warning,
            ExecutionFailure::Expired { .. } => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            ExecutionFailure::NotFound { .. } => 404,
            ExecutionFailure::ConfirmationTimeout { .. } => 504,
            ExecutionFailure::IndexingDelay { .. } => 503,
            ExecutionFailure::Reverted { .. } => 422,
            ExecutionFailure::Expired { .. } => 422,
        }
    }
}
