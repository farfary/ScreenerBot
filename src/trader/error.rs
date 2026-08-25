//! Errors produced by the trader module.

use std::time::Duration;

use crate::errors::{ErrorClass, Severity};

/// Everything that can go wrong running, controlling or copy-trading the trader.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("trader is already running")]
    AlreadyRunning,
    #[error("trader is already stopped")]
    AlreadyStopped,
    #[error("trader config update failed: {detail}")]
    ConfigUpdate { detail: String },
    #[error(transparent)]
    Database(#[from] crate::errors::DatabaseError),
    #[error(transparent)]
    Positions(#[from] crate::positions::Error),
    #[error(transparent)]
    Transactions(#[from] crate::transactions::Error),
    #[error("no copy-trading task with id {task_id}")]
    CopyTaskNotFound { task_id: i64 },
    #[error("could not decode field {field} of copy task {task_id}: {detail}")]
    CopyTaskDecode {
        task_id: i64,
        field: &'static str,
        detail: String,
    },
    #[error("could not serialize {field} for a copy task: {detail}")]
    CopySerialize { field: &'static str, detail: String },
    #[error("copy-trading reconciliation failed: {detail}")]
    CopyReconciliation { detail: String },
    /// A copy-task request violates a repository invariant (no fitting variant
    /// above: this is a request-shape rejection, not a decode/serialize/DB failure —
    /// wrong chain, wrong starting mode, an unconfirmed mode transition, etc).
    #[error("invalid copy-trading request: {detail}")]
    CopyValidation { detail: String },
    #[error("copy database task failed: {detail}")]
    CopyDatabaseUnavailable { detail: String },
    /// Registering the trade's action entry failed. The actions module's own typed
    /// error is kept as the source rather than flattened into text, so callers keep
    /// its classification.
    #[error("could not record manual trade")]
    ManualTradeRecord {
        #[source]
        source: crate::actions::Error,
    },
    #[error("no open position for token {mint}")]
    NoOpenPosition { mint: String },
    #[error("invalid trade size {amount_sol} SOL: {reason}")]
    InvalidSolAmount { amount_sol: f64, reason: String },
    /// A manual trade's `PositionManagement` is not valid for its origin (no fitting
    /// variant above: this is a request-shape validation, not a SOL-amount one).
    #[error("invalid position management {management}: {reason}")]
    InvalidManagement { management: String, reason: String },
    #[error("strategy evaluation for token {mint} failed: {detail}")]
    StrategyEvaluation { mint: String, detail: String },
    #[error("token data unavailable for {mint}")]
    TokenDataMissing { mint: String },
    #[error("no healthy endpoints available: {detail}")]
    UnhealthyEndpoints { detail: String },
    /// An upstream dependency the trader reads from (filtering, wallets, ...) has not
    /// migrated off string errors yet (no fitting variant above: this names WHICH
    /// dependency failed, distinguishable from the trader's own causes).
    #[error("{dependency} dependency failed: {detail}")]
    Dependency {
        dependency: &'static str,
        detail: String,
    },
}

/// Result alias for the trader module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::AlreadyRunning | Error::AlreadyStopped => false,
            Error::ConfigUpdate { .. } => false,
            Error::Database(e) => e.is_retryable(),
            Error::Positions(e) => e.is_retryable(),
            Error::Transactions(e) => e.is_retryable(),
            Error::CopyTaskNotFound { .. } => false,
            Error::CopyTaskDecode { .. } | Error::CopySerialize { .. } => false,
            Error::CopyReconciliation { .. } => true,
            Error::CopyValidation { .. } => false,
            Error::CopyDatabaseUnavailable { .. } => true,
            Error::ManualTradeRecord { .. } => false,
            Error::NoOpenPosition { .. } => false,
            Error::InvalidSolAmount { .. } => false,
            Error::InvalidManagement { .. } => false,
            Error::StrategyEvaluation { .. } => false,
            Error::TokenDataMissing { .. } => true,
            Error::UnhealthyEndpoints { .. } => true,
            Error::Dependency { .. } => true,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Database(e) => e.retry_after(),
            Error::Positions(e) => e.retry_after(),
            Error::Transactions(e) => e.retry_after(),
            Error::CopyReconciliation { .. } => Some(Duration::from_secs(2)),
            Error::CopyDatabaseUnavailable { .. } => Some(Duration::from_secs(1)),
            Error::TokenDataMissing { .. } => Some(Duration::from_secs(1)),
            Error::UnhealthyEndpoints { .. } => Some(Duration::from_secs(5)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::AlreadyRunning | Error::AlreadyStopped => Severity::Info,
            Error::ConfigUpdate { .. } => Severity::Error,
            Error::Database(e) => e.severity(),
            Error::Positions(e) => e.severity(),
            Error::Transactions(e) => e.severity(),
            Error::CopyTaskNotFound { .. } => Severity::Info,
            Error::CopyTaskDecode { .. } => Severity::Critical,
            Error::CopySerialize { .. } => Severity::Error,
            Error::CopyReconciliation { .. } => Severity::Error,
            Error::CopyValidation { .. } => Severity::Warning,
            Error::CopyDatabaseUnavailable { .. } => Severity::Error,
            Error::ManualTradeRecord { .. } => Severity::Critical,
            Error::NoOpenPosition { .. } => Severity::Warning,
            Error::InvalidSolAmount { .. } => Severity::Warning,
            Error::InvalidManagement { .. } => Severity::Warning,
            Error::StrategyEvaluation { .. } => Severity::Warning,
            Error::TokenDataMissing { .. } => Severity::Warning,
            Error::UnhealthyEndpoints { .. } => Severity::Error,
            Error::Dependency { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::AlreadyRunning | Error::AlreadyStopped => 400,
            Error::ConfigUpdate { .. } => 500,
            Error::Database(e) => e.http_status(),
            Error::Positions(e) => e.http_status(),
            Error::Transactions(e) => e.http_status(),
            Error::CopyTaskNotFound { .. } => 404,
            Error::CopyTaskDecode { .. }
            | Error::CopySerialize { .. }
            | Error::CopyReconciliation { .. }
            | Error::CopyDatabaseUnavailable { .. }
            | Error::ManualTradeRecord { .. } => 500,
            Error::CopyValidation { .. } => 400,
            Error::NoOpenPosition { .. } => 404,
            Error::InvalidSolAmount { .. } => 400,
            Error::InvalidManagement { .. } => 400,
            Error::StrategyEvaluation { .. } => 500,
            Error::TokenDataMissing { .. } => 503,
            Error::UnhealthyEndpoints { .. } => 503,
            Error::Dependency { .. } => 503,
        }
    }
}
