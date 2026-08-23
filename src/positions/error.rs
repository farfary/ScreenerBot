//! Errors produced by the positions module.

use std::time::Duration;

use crate::errors::{ErrorClass, Severity};

/// Everything that can go wrong managing a position.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    // --- persistence lifecycle ---
    #[error("the positions database is not initialised")]
    NotInitialised,
    #[error(transparent)]
    Database(#[from] crate::errors::DatabaseError),
    #[error("could not decode column {column} of a position row: {detail}")]
    RowDecode {
        column: &'static str,
        detail: String,
    },
    #[error("positions schema migration failed: {detail}")]
    SchemaMigration { detail: String },
    #[error("positions database {operation} failed: {detail}")]
    Maintenance {
        operation: &'static str,
        detail: String,
    },

    // --- lookup ---
    #[error("no position for token {mint}")]
    NotFound { mint: String },
    #[error("no position with id {position_id}")]
    NotFoundById { position_id: i64 },
    #[error("no position with signature {signature}")]
    NotFoundBySignature { signature: String },
    #[error("token {mint} is not in the token store")]
    TokenNotFound { mint: String },

    // --- state conflicts ---
    #[error("an open position already exists for token {mint}")]
    AlreadyOpen { mint: String },
    #[error("persisted position {field} has unknown value '{value}'")]
    UnknownPersistedValue { field: &'static str, value: String },

    // --- validation ---
    #[error("invalid price {price} for token {mint}")]
    InvalidPrice { mint: String, price: f64 },
    #[error("invalid trade size {amount_sol} SOL: {reason}")]
    InvalidTradeSize { amount_sol: f64, reason: String },
    #[error("invalid exit percentage {percent}: {reason}")]
    InvalidExitPercentage { percent: f64, reason: String },
    #[error("calculated exit amount for token {mint} is zero")]
    ZeroExitAmount { mint: String },
    #[error("DCA is disabled in configuration")]
    DcaDisabled,

    // --- transition and execution ---
    #[error("could not apply the {transition} transition to {mint}: {detail}")]
    TransitionFailed {
        transition: &'static str,
        mint: String,
        detail: String,
    },
    #[error("swap for token {mint} failed: {detail}")]
    SwapFailed { mint: String, detail: String },
    #[error("wallet-history sync failed: {detail}")]
    WalletHistorySync { detail: String },

    /// The active wallet address could not be resolved (no fitting variant above:
    /// this is a config-layer failure, not a database, lookup or transition one,
    /// and it recurs at ~20 wallet-scoped query/write sites).
    #[error("could not resolve the active wallet address: {detail}")]
    WalletUnavailable { detail: String },

    /// The global position-capacity semaphore has no free permit (no fitting variant
    /// above: this is neither a per-mint conflict nor a validation failure).
    #[error("no free position slot ({remaining} remaining)")]
    SlotUnavailable { remaining: usize },
}

/// Result alias for the positions module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            // Startup race — the caller may succeed if it waits for init.
            Error::NotInitialised => true,
            Error::Database(e) => e.is_retryable(),
            // Lookups are a final verdict at the time of the call; nothing
            // about repeating the same call changes the answer.
            Error::NotFound { .. }
            | Error::NotFoundById { .. }
            | Error::NotFoundBySignature { .. }
            | Error::TokenNotFound { .. } => false,
            // State conflicts describe the world as it is right now.
            Error::AlreadyOpen { .. } | Error::UnknownPersistedValue { .. } => false,
            // Validation failures are a property of the input, not the attempt.
            Error::InvalidPrice { .. }
            | Error::InvalidTradeSize { .. }
            | Error::InvalidExitPercentage { .. }
            | Error::ZeroExitAmount { .. }
            | Error::DcaDisabled => false,
            // Money-state failures are not safely retried without a human
            // or the verifier reconciling what actually happened on chain.
            Error::RowDecode { .. }
            | Error::SchemaMigration { .. }
            | Error::TransitionFailed { .. }
            | Error::SwapFailed { .. } => false,
            Error::Maintenance { .. } => false,
            Error::WalletHistorySync { .. } => true,
            Error::WalletUnavailable { .. } => false,
            Error::SlotUnavailable { .. } => true,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::NotInitialised => Some(Duration::from_millis(500)),
            Error::Database(e) => e.retry_after(),
            Error::WalletHistorySync { .. } => Some(Duration::from_secs(2)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::NotInitialised => Severity::Warning,
            Error::Database(e) => e.severity(),
            Error::RowDecode { .. } | Error::SchemaMigration { .. } => Severity::Critical,
            Error::Maintenance { .. } => Severity::Error,
            Error::NotFound { .. }
            | Error::NotFoundById { .. }
            | Error::NotFoundBySignature { .. }
            | Error::TokenNotFound { .. } => Severity::Info,
            Error::AlreadyOpen { .. } | Error::UnknownPersistedValue { .. } => Severity::Warning,
            Error::InvalidPrice { .. }
            | Error::InvalidTradeSize { .. }
            | Error::InvalidExitPercentage { .. }
            | Error::ZeroExitAmount { .. }
            | Error::DcaDisabled => Severity::Warning,
            Error::TransitionFailed { .. } | Error::SwapFailed { .. } => Severity::Critical,
            Error::WalletHistorySync { .. } => Severity::Error,
            Error::WalletUnavailable { .. } => Severity::Critical,
            Error::SlotUnavailable { .. } => Severity::Info,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::NotInitialised => 503,
            Error::Database(e) => e.http_status(),
            Error::NotFound { .. }
            | Error::NotFoundById { .. }
            | Error::NotFoundBySignature { .. }
            | Error::TokenNotFound { .. } => 404,
            Error::AlreadyOpen { .. } | Error::ZeroExitAmount { .. } => 409,
            Error::UnknownPersistedValue { .. }
            | Error::InvalidPrice { .. }
            | Error::InvalidTradeSize { .. }
            | Error::InvalidExitPercentage { .. }
            | Error::DcaDisabled => 400,
            Error::RowDecode { .. }
            | Error::SchemaMigration { .. }
            | Error::Maintenance { .. }
            | Error::TransitionFailed { .. }
            | Error::SwapFailed { .. }
            | Error::WalletHistorySync { .. }
            | Error::WalletUnavailable { .. } => 500,
            Error::SlotUnavailable { .. } => 503,
        }
    }
}
