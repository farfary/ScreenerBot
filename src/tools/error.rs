//! Errors produced by the tools module.

use std::time::Duration;

use crate::errors::{ErrorClass, Severity};

/// Everything that can go wrong running a tool: ATA cleanup, multi-wallet
/// sessions, favorites, trade watching.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] crate::errors::DatabaseError),
    #[error(transparent)]
    Wallets(#[from] crate::wallets::Error),

    #[error("could not decode column {column}: {detail}")]
    RowDecode {
        column: &'static str,
        detail: String,
    },
    #[error("tools schema migration step {step} failed: {detail}")]
    Migration { step: String, detail: String },
    /// A tool config (sizing, multi-wallet, watched-token) fails its own
    /// validation rules (no fitting variant above: this is a request-shape
    /// rejection on caller-supplied config, not a session or database cause).
    #[error("invalid tool configuration: {detail}")]
    InvalidConfig { detail: String },
    #[error("the main wallet is unavailable: {detail}")]
    MainWalletUnavailable { detail: String },
    #[error("{provider} search failed: {detail}")]
    Search {
        provider: &'static str,
        detail: String,
    },
    #[error("the failed-ATA cache could not be {operation}: {detail}")]
    AtaCache {
        operation: &'static str,
        detail: String,
    },
    /// A cross-cutting dependency this module reads from (chains, config,
    /// ...) failed (no fitting variant above: this names WHICH dependency
    /// failed — mirrors `wallets::Error::Dependency`).
    #[error("{dependency} dependency failed: {detail}")]
    Dependency {
        dependency: &'static str,
        detail: String,
    },
}

/// Result alias for the tools module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Database(e) => e.is_retryable(),
            Error::Wallets(e) => e.is_retryable(),
            Error::RowDecode { .. } => false,
            Error::Migration { .. } => false,
            Error::InvalidConfig { .. } => false,
            Error::MainWalletUnavailable { .. } => true,
            Error::Search { .. } => true,
            Error::AtaCache { .. } => true,
            Error::Dependency { .. } => true,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Database(e) => e.retry_after(),
            Error::Wallets(e) => e.retry_after(),
            Error::MainWalletUnavailable { .. } => Some(Duration::from_secs(1)),
            Error::Search { .. } => Some(Duration::from_millis(500)),
            Error::AtaCache { .. } => Some(Duration::from_secs(1)),
            Error::Dependency { .. } => Some(Duration::from_millis(500)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Database(e) => e.severity(),
            Error::Wallets(e) => e.severity(),
            Error::RowDecode { .. } => Severity::Critical,
            Error::Migration { .. } => Severity::Critical,
            Error::InvalidConfig { .. } => Severity::Warning,
            Error::MainWalletUnavailable { .. } => Severity::Warning,
            Error::Search { .. } => Severity::Warning,
            Error::AtaCache { .. } => Severity::Warning,
            Error::Dependency { .. } => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Database(e) => e.http_status(),
            Error::Wallets(e) => e.http_status(),
            Error::RowDecode { .. } => 500,
            Error::Migration { .. } => 500,
            Error::InvalidConfig { .. } => 400,
            Error::MainWalletUnavailable { .. } => 503,
            Error::Search { .. } => 503,
            Error::AtaCache { .. } => 500,
            Error::Dependency { .. } => 503,
        }
    }
}
