//! Errors produced by the tokens module: metadata, discovery, security and
//! market-data persistence.

use std::time::Duration;

use crate::errors::{DatabaseError, ErrorClass, InternalError, Severity};

/// Everything that can go wrong maintaining token metadata, discovery,
/// security data and market-data persistence.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A query/prepare/transaction failure against the tokens database.
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// A task join failure, a poisoned lock, or an unfulfillable permit
    /// acquisition — invariant violations, not data problems.
    #[error(transparent)]
    Internal(#[from] InternalError),

    /// A Solana-side read (mint account fetch, decimals) failed. Carried
    /// typed so callers keep the chain's classification instead of a rendered
    /// string.
    #[error(transparent)]
    Chain(#[from] crate::chains::solana::Error),

    /// A search request was not usable as submitted.
    #[error("search query is not usable: {reason}")]
    InvalidSearchQuery { reason: String },
    /// The global token database is scoped to a different chain than the
    /// caller asked about.
    #[error("token database is scoped to {expected}, not {actual}")]
    ChainMismatch { expected: String, actual: String },

    /// An external data source (DexScreener, GeckoTerminal, Rugcheck, ...)
    /// returned an error.
    #[error("{provider} error: {message}")]
    Api { provider: String, message: String },
    /// A rate limit was hit, or a rate-limited slot could not be acquired
    /// in time.
    #[error("{provider} rate limited: {message}")]
    RateLimit { provider: String, message: String },
    /// A supplied mint address is not usable.
    #[error("'{value}' is not a valid mint address")]
    InvalidMint { value: String },
    /// A resource this call depends on has not been initialized yet (the
    /// global database, the rate-limit coordinator, ...).
    #[error("{resource} is not initialized")]
    NotInitialized { resource: String },
    /// A SQLite row could not be decoded into its expected shape.
    #[error("could not decode row: {detail}")]
    RowDecode { detail: String },
    /// A caller-supplied priority is not one of the valid tiers.
    #[error("invalid priority value {value}: must be one of 10, 25, 40, 55, 60, 75, 100")]
    InvalidPriority { value: i32 },
    /// The token still has no market data after an on-demand fetch attempt.
    #[error("token {mint} has no market data available (cannot price/swap)")]
    NoMarketData { mint: String },
    /// A schema migration's own integrity check failed (row-count mismatch
    /// or a foreign-key violation).
    #[error("tokens schema migration integrity check failed for {table}: {detail}")]
    MigrationIntegrity { table: String, detail: String },
}

/// Result alias for the tokens module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Database(e) => e.is_retryable(),
            Error::Internal(e) => e.is_retryable(),
            Error::Chain(e) => e.is_retryable(),
            Error::Api { .. } | Error::RateLimit { .. } => true,
            Error::InvalidMint { .. }
            | Error::InvalidSearchQuery { .. }
            | Error::ChainMismatch { .. }
            | Error::NotInitialized { .. }
            | Error::RowDecode { .. }
            | Error::InvalidPriority { .. }
            | Error::NoMarketData { .. }
            | Error::MigrationIntegrity { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Database(e) => e.retry_after(),
            Error::Internal(e) => e.retry_after(),
            Error::Chain(e) => e.retry_after(),
            Error::Api { .. } => Some(Duration::from_millis(500)),
            Error::RateLimit { .. } => Some(Duration::from_secs(1)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Database(e) => e.severity(),
            Error::Internal(e) => e.severity(),
            Error::Chain(e) => e.severity(),
            Error::Api { .. } | Error::RateLimit { .. } => Severity::Warning,
            Error::InvalidMint { .. } | Error::InvalidSearchQuery { .. } => Severity::Info,
            Error::ChainMismatch { .. } => Severity::Critical,
            Error::NotInitialized { .. } => Severity::Error,
            Error::RowDecode { .. } => Severity::Warning,
            Error::InvalidPriority { .. } => Severity::Warning,
            Error::NoMarketData { .. } => Severity::Info,
            Error::MigrationIntegrity { .. } => Severity::Critical,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Database(e) => e.http_status(),
            Error::Internal(e) => e.http_status(),
            Error::Chain(e) => e.http_status(),
            Error::Api { .. } => 502,
            Error::RateLimit { .. } => 429,
            Error::InvalidMint { .. } | Error::InvalidSearchQuery { .. } => 400,
            Error::ChainMismatch { .. } => 500,
            Error::NotInitialized { .. } => 503,
            Error::RowDecode { .. } => 500,
            Error::InvalidPriority { .. } => 400,
            Error::NoMarketData { .. } => 404,
            Error::MigrationIntegrity { .. } => 500,
        }
    }
}
