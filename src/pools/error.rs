// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Errors produced by the pools module: pricing, persistence and blacklist
//! state.

use std::time::Duration;

use crate::errors::{DatabaseError, ErrorClass, InternalError, Severity};

/// Everything that can go wrong maintaining chain-neutral pool prices,
/// history and blacklist state.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A query/prepare/read-row failure against the pools database.
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// A task join failure or a poisoned lock — invariant violations, not
    /// data problems.
    #[error(transparent)]
    Internal(#[from] InternalError),

    /// The pools database has not been opened yet (or its connection was
    /// dropped) when an operation needed it.
    #[error("pools database is not initialized")]
    NotInitialized,
    /// The background price-history write queue could not accept a price.
    #[error("price-history queue is unavailable: {detail}")]
    QueueUnavailable { detail: String },
    /// Raw account bytes could not be decoded into the expected field.
    #[error("could not decode {field}: {detail}")]
    Decode { field: &'static str, detail: String },
    /// A pool's mint/vault pairing does not qualify as a SOL pair.
    #[error("pool is not a valid SOL pair: {reason}")]
    InvalidPool { reason: String },
    /// The legacy-to-chain-scoped schema migration's own integrity check
    /// failed (row-count mismatch or a foreign-key violation).
    #[error("pools schema migration integrity check failed for {table}: {detail}")]
    MigrationIntegrity { table: String, detail: String },
    /// `initialize_pool_components` was called while the service was
    /// already running.
    #[error("pool service is already running")]
    AlreadyRunning,
    /// The chain-specific runtime components (discovery/analyzer/fetcher/
    /// calculator) failed to initialize. Carries the concrete chain
    /// adapter's own error rendered to text — the pools module is
    /// chain-neutral and must not name a chain adapter's error type.
    #[error("pool component initialization failed: {detail}")]
    ComponentInit { detail: String },
}

/// Result alias for the pools module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Database(e) => e.is_retryable(),
            Error::Internal(e) => e.is_retryable(),
            Error::ComponentInit { .. } => true,
            Error::NotInitialized
            | Error::QueueUnavailable { .. }
            | Error::Decode { .. }
            | Error::InvalidPool { .. }
            | Error::MigrationIntegrity { .. }
            | Error::AlreadyRunning => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Database(e) => e.retry_after(),
            Error::Internal(e) => e.retry_after(),
            Error::ComponentInit { .. } => Some(Duration::from_secs(2)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Database(e) => e.severity(),
            Error::Internal(e) => e.severity(),
            Error::NotInitialized | Error::QueueUnavailable { .. } => Severity::Error,
            Error::MigrationIntegrity { .. } => Severity::Critical,
            Error::Decode { .. } => Severity::Warning,
            Error::InvalidPool { .. } => Severity::Info,
            Error::AlreadyRunning => Severity::Warning,
            Error::ComponentInit { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Database(e) => e.http_status(),
            Error::Internal(e) => e.http_status(),
            Error::NotInitialized | Error::QueueUnavailable { .. } => 503,
            Error::MigrationIntegrity { .. } => 500,
            Error::Decode { .. } | Error::InvalidPool { .. } => 422,
            Error::AlreadyRunning => 409,
            Error::ComponentInit { .. } => 503,
        }
    }
}
