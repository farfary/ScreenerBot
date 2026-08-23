//! Errors produced by the transactions module.

use std::time::Duration;

use crate::errors::{ErrorClass, Severity};

/// Everything that can go wrong processing, persisting or verifying a transaction.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("transaction database is not initialised")]
    NotInitialised,
    #[error(transparent)]
    Database(#[from] crate::errors::DatabaseError),
    #[error("could not decode column {column} of a transaction row: {detail}")]
    RowDecode {
        column: &'static str,
        detail: String,
    },
    #[error("failed to inspect transactions schema: {detail}")]
    SchemaInspect { detail: String },
    #[error("transactions schema migration ({step}) failed: {detail}")]
    Migration { step: String, detail: String },
    #[error("failed to parse transaction {signature}: {detail}")]
    JsonParse { signature: String, detail: String },
    #[error("no transaction with signature {signature}")]
    NotFound { signature: String },
    /// A subject's chain does not match the database's own chain (no fitting
    /// variant above: this is a caller-input mismatch, not a decode, lookup or
    /// migration failure, and recurs at every subject-scoped query/write site).
    #[error("subject chain {actual} does not match database chain {expected}")]
    ChainMismatch {
        expected: crate::chains::ChainId,
        actual: crate::chains::ChainId,
    },
    #[error("subject-delta backfill failed: {detail}")]
    DeltaBackfill { detail: String },
    /// The initial full-history bootstrap (signature paging + processing) failed
    /// (no fitting variant above: this is a chain-fetch/RPC failure during startup
    /// history collection, distinct from the subject-delta reduction `DeltaBackfill`
    /// covers).
    #[error("transaction history bootstrap failed: {detail}")]
    Bootstrap { detail: String },
    /// The transaction service is already running.
    #[error("transaction service is already running")]
    ServiceAlreadyRunning,
    /// The active wallet address could not be resolved (no fitting variant above:
    /// this is a config-layer failure, not a database, decode or migration one, and
    /// recurs at several wallet-scoped query sites).
    #[error("could not resolve the active wallet address: {detail}")]
    WalletUnavailable { detail: String },
    #[error("wallet-history sync failed: {detail}")]
    WalletHistorySync { detail: String },
    #[error("verification of transaction {signature} failed: {detail}")]
    VerificationFailed { signature: String, detail: String },
    #[error("transaction {signature} is stale ({age_hours}h old)")]
    StaleTransaction { signature: String, age_hours: i64 },
}

/// Result alias for the transactions module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::NotInitialised => true,
            Error::Database(e) => e.is_retryable(),
            Error::RowDecode { .. } | Error::SchemaInspect { .. } | Error::Migration { .. } => {
                false
            }
            Error::JsonParse { .. } => false,
            Error::NotFound { .. } => false,
            Error::ChainMismatch { .. } => false,
            Error::DeltaBackfill { .. } => true,
            Error::Bootstrap { .. } => true,
            Error::ServiceAlreadyRunning => false,
            Error::WalletUnavailable { .. } => false,
            Error::WalletHistorySync { .. } => true,
            Error::VerificationFailed { .. } => false,
            Error::StaleTransaction { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::NotInitialised => Some(Duration::from_millis(500)),
            Error::Database(e) => e.retry_after(),
            Error::DeltaBackfill { .. } => Some(Duration::from_secs(2)),
            Error::WalletHistorySync { .. } => Some(Duration::from_secs(2)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::NotInitialised => Severity::Warning,
            Error::Database(e) => e.severity(),
            Error::RowDecode { .. } | Error::Migration { .. } => Severity::Critical,
            Error::SchemaInspect { .. } => Severity::Critical,
            Error::JsonParse { .. } => Severity::Warning,
            Error::NotFound { .. } => Severity::Info,
            Error::ChainMismatch { .. } => Severity::Warning,
            Error::DeltaBackfill { .. } => Severity::Error,
            Error::Bootstrap { .. } => Severity::Warning,
            Error::ServiceAlreadyRunning => Severity::Info,
            Error::WalletUnavailable { .. } => Severity::Critical,
            Error::WalletHistorySync { .. } => Severity::Error,
            Error::VerificationFailed { .. } => Severity::Critical,
            Error::StaleTransaction { .. } => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::NotInitialised => 503,
            Error::Database(e) => e.http_status(),
            Error::NotFound { .. } => 404,
            Error::ChainMismatch { .. } => 400,
            Error::JsonParse { .. } => 400,
            Error::RowDecode { .. }
            | Error::SchemaInspect { .. }
            | Error::Migration { .. }
            | Error::DeltaBackfill { .. }
            | Error::Bootstrap { .. }
            | Error::WalletUnavailable { .. }
            | Error::WalletHistorySync { .. }
            | Error::VerificationFailed { .. } => 500,
            Error::StaleTransaction { .. } => 409,
            Error::ServiceAlreadyRunning => 409,
        }
    }
}
