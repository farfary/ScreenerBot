//! Errors produced by the wallets module.

use std::time::Duration;

use crate::errors::{ErrorClass, Severity};

/// Everything that can go wrong storing, watching or monitoring wallets.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] crate::errors::DatabaseError),
    #[error(transparent)]
    Internal(#[from] crate::errors::InternalError),
    #[error(transparent)]
    Io(#[from] crate::errors::IoError),
    #[error(transparent)]
    Transactions(#[from] crate::transactions::Error),

    /// A database this module reads through has not been initialized yet — an
    /// invariant violation, not a not-found: initialization must run once at
    /// startup before any other wallet function is called.
    #[error("the {database} database is not initialized")]
    NotInitialized { database: &'static str },
    #[error("wallet {address} is not known")]
    WalletNotFound { address: String },
    /// The imported private key resolves to a wallet that is already stored
    /// (no fitting variant above: a duplicate import is a conflict on an
    /// existing row, not a database failure and not an unusable key).
    #[error("wallet {address} already exists")]
    WalletAlreadyExists { address: String },
    /// The wallet exists but its current role/state forbids the requested
    /// operation (no fitting variant above: this is a business-rule
    /// rejection, not a not-found or a plain database failure — archiving/
    /// deleting the main wallet, restoring a wallet that is not archived).
    #[error("wallet {id} is in an invalid state for this operation: {detail}")]
    InvalidWalletState { id: i64, detail: &'static str },
    #[error("watch target {address} is not being watched")]
    WatchTargetNotFound { address: String },
    /// Wallet watching is switched off in config, so no target may be added
    /// (no fitting variant above: the request is well formed and the address
    /// may be perfectly valid — the feature itself is disabled).
    #[error("wallet watching is disabled (wallet.watch_enabled)")]
    WatchDisabled,
    /// The address is already a watch target (no fitting variant above: a
    /// conflict with an existing target, not a malformed address).
    #[error("watch target {address} is already watched")]
    WatchTargetAlreadyWatched { address: String },
    /// The address is one of the user's own wallets, which is observed through
    /// `WatchSource::OwnWallet` rather than added as a target (no fitting
    /// variant above: a conflict with an existing wallet, not a bad address).
    #[error("{address} is one of your own wallets, not a target to watch")]
    WatchTargetIsOwnWallet { address: String },
    /// `wallet.watch_max_targets` is reached (no fitting variant above: a
    /// configured-capacity rejection, not a malformed request).
    #[error("the watch target limit of {max} is reached; remove one before adding another")]
    WatchTargetLimitReached { max: usize },
    #[error("'{value}' is not a valid watch address")]
    InvalidWatchAddress { value: String },
    /// A batch wallet-creation request is malformed or produced nothing (no
    /// fitting variant above: this is a request-shape rejection on the batch
    /// as a whole, not a single wallet's not-found/invalid-key cause).
    #[error("invalid wallet batch request: {detail}")]
    InvalidBatchRequest { detail: &'static str },
    /// A dashboard window does not map to one of the canonical cache windows
    /// (24h/7d/30d/all_time) (no fitting variant above: this is a caller-input
    /// rejection specific to dashboard windowing, not a batch-wallet request).
    #[error("{window_hours} is not a supported dashboard window")]
    InvalidWindow { window_hours: i64 },
    #[error("the supplied private key is not usable: {reason}")]
    InvalidPrivateKey { reason: String },
    #[error("could not decode persisted watch sources: {detail}")]
    WatchSourcesDecode { detail: String },
    #[error("could not inspect the {table} schema: {detail}")]
    SchemaInspect { table: &'static str, detail: String },
    #[error("wallets schema migration step {step} failed: {detail}")]
    Migration { step: String, detail: String },
    #[error("snapshot {operation} failed: {detail}")]
    SnapshotMaintenance {
        operation: &'static str,
        detail: String,
    },
    /// The dashboard cache payload could not be (de)serialized or
    /// (de)compressed (no fitting variant above: not a database, watch-source
    /// or migration failure — this is the wallet-monitor dashboard cache's
    /// own payload encoding).
    #[error("dashboard cache payload {operation} failed: {detail}")]
    DashboardPayload {
        operation: &'static str,
        detail: String,
    },
    #[error("could not update the balance for {address}: {detail}")]
    BalanceUpdate { address: String, detail: String },
    /// The injected chain-execution runtime (`WalletWatchRuntime`, registered
    /// by the composition root from `crate::chains::solana::wallets::runtime`)
    /// failed (no fitting variant above: this is the chain-execution seam
    /// boundary itself — the chains module has not migrated off string errors
    /// yet, contract §5/§9 — not a wallets-owned database or request-shape
    /// cause).
    #[error("chain runtime {operation} failed: {detail}")]
    ChainRuntime {
        operation: &'static str,
        detail: String,
    },
    /// The chain reported a classified execution outcome (for example the node
    /// has not indexed a signature yet). Carried as a typed value so callers
    /// branch on the variant instead of on a coded operation string.
    #[error(transparent)]
    ChainExecution(#[from] crate::chains::ExecutionFailure),
    /// A cross-cutting dependency this module reads from (config, chain
    /// accounts, ...) failed (no fitting variant above: this names WHICH
    /// dependency failed, distinguishable from wallets' own database or
    /// request-shape causes — mirrors `trader::Error::Dependency`).
    #[error("{dependency} dependency failed: {detail}")]
    Dependency {
        dependency: &'static str,
        detail: String,
    },
}

/// Result alias for the wallets module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Database(e) => e.is_retryable(),
            Error::Internal(e) => e.is_retryable(),
            Error::Io(e) => e.is_retryable(),
            Error::Transactions(e) => e.is_retryable(),
            Error::NotInitialized { .. } => false,
            Error::WalletNotFound { .. } => false,
            Error::WalletAlreadyExists { .. } => false,
            Error::InvalidWalletState { .. } => false,
            Error::WatchTargetNotFound { .. } => false,
            Error::WatchDisabled => false,
            Error::WatchTargetAlreadyWatched { .. } => false,
            Error::WatchTargetIsOwnWallet { .. } => false,
            Error::WatchTargetLimitReached { .. } => false,
            Error::InvalidWatchAddress { .. } => false,
            Error::InvalidBatchRequest { .. } => false,
            Error::InvalidWindow { .. } => false,
            Error::InvalidPrivateKey { .. } => false,
            Error::WatchSourcesDecode { .. } => false,
            Error::SchemaInspect { .. } => false,
            Error::Migration { .. } => false,
            Error::SnapshotMaintenance { .. } => true,
            Error::DashboardPayload { .. } => false,
            Error::BalanceUpdate { .. } => true,
            Error::ChainRuntime { .. } | Error::ChainExecution(_) => true,
            Error::Dependency { .. } => true,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Database(e) => e.retry_after(),
            Error::Internal(e) => e.retry_after(),
            Error::Io(e) => e.retry_after(),
            Error::Transactions(e) => e.retry_after(),
            Error::SnapshotMaintenance { .. } => Some(Duration::from_secs(2)),
            Error::BalanceUpdate { .. } => Some(Duration::from_millis(500)),
            Error::ChainRuntime { .. } | Error::ChainExecution(_) => {
                Some(Duration::from_millis(500))
            }
            Error::Dependency { .. } => Some(Duration::from_millis(500)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Database(e) => e.severity(),
            Error::Internal(e) => e.severity(),
            Error::Io(e) => e.severity(),
            Error::Transactions(e) => e.severity(),
            Error::NotInitialized { .. } => Severity::Critical,
            Error::WalletNotFound { .. } => Severity::Warning,
            Error::WalletAlreadyExists { .. } => Severity::Warning,
            Error::InvalidWalletState { .. } => Severity::Warning,
            Error::WatchTargetNotFound { .. } => Severity::Warning,
            Error::WatchDisabled => Severity::Warning,
            Error::WatchTargetAlreadyWatched { .. } => Severity::Warning,
            Error::WatchTargetIsOwnWallet { .. } => Severity::Warning,
            Error::WatchTargetLimitReached { .. } => Severity::Warning,
            Error::InvalidWatchAddress { .. } => Severity::Warning,
            Error::InvalidBatchRequest { .. } => Severity::Warning,
            Error::InvalidWindow { .. } => Severity::Warning,
            Error::InvalidPrivateKey { .. } => Severity::Critical,
            Error::WatchSourcesDecode { .. } => Severity::Critical,
            Error::SchemaInspect { .. } => Severity::Critical,
            Error::Migration { .. } => Severity::Critical,
            Error::SnapshotMaintenance { .. } => Severity::Warning,
            Error::DashboardPayload { .. } => Severity::Error,
            Error::BalanceUpdate { .. } => Severity::Warning,
            Error::ChainRuntime { .. } | Error::ChainExecution(_) => Severity::Warning,
            Error::Dependency { .. } => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Database(e) => e.http_status(),
            Error::Internal(e) => e.http_status(),
            Error::Io(e) => e.http_status(),
            Error::Transactions(e) => e.http_status(),
            Error::NotInitialized { .. } => 500,
            Error::WalletNotFound { .. } => 404,
            Error::WalletAlreadyExists { .. } => 409,
            Error::InvalidWalletState { .. } => 409,
            Error::WatchTargetNotFound { .. } => 404,
            Error::WatchDisabled => 400,
            Error::WatchTargetAlreadyWatched { .. } | Error::WatchTargetIsOwnWallet { .. } => 409,
            Error::WatchTargetLimitReached { .. } => 400,
            Error::InvalidWatchAddress { .. } => 400,
            Error::InvalidBatchRequest { .. } => 400,
            Error::InvalidWindow { .. } => 400,
            Error::InvalidPrivateKey { .. } => 400,
            Error::WatchSourcesDecode { .. } => 500,
            Error::SchemaInspect { .. } => 500,
            Error::Migration { .. } => 500,
            Error::SnapshotMaintenance { .. } => 500,
            Error::DashboardPayload { .. } => 500,
            Error::BalanceUpdate { .. } => 503,
            Error::ChainRuntime { .. } | Error::ChainExecution(_) => 503,
            Error::Dependency { .. } => 503,
        }
    }
}
