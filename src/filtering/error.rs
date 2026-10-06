// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Errors produced by the filtering module.

use std::time::Duration;

use crate::errors::{ErrorClass, InternalError, Severity};

/// Everything that can go wrong while filtering tokens.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A timeout or other internal failure prevented a filtering operation.
    #[error(transparent)]
    Internal(#[from] InternalError),

    /// The chain a filtering call named could not be served.
    #[error(transparent)]
    Chain(#[from] crate::chains::Error),

    /// The chain has no installed runtime, so its filter profile is unavailable.
    #[error("no chain runtime is installed for {chain}")]
    RuntimeUnavailable { chain: crate::chains::ChainId },

    /// Loading or counting a token set from storage failed. The token store's
    /// own typed error is kept as the source rather than flattened into text,
    /// so callers keep its classification and `kind` still says which set.
    #[error("could not load the {kind} token set")]
    TokenSetLoad {
        kind: &'static str,
        #[source]
        source: crate::tokens::Error,
    },
}

/// Result alias for the filtering module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Internal(e) => e.is_retryable(),
            Error::Chain(e) => e.is_retryable(),
            Error::RuntimeUnavailable { .. } => true,
            Error::TokenSetLoad { source, .. } => source.is_retryable(),
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Internal(e) => e.retry_after(),
            Error::Chain(e) => e.retry_after(),
            Error::RuntimeUnavailable { .. } => None,
            Error::TokenSetLoad { source, .. } => source.retry_after(),
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Internal(e) => e.severity(),
            Error::Chain(e) => e.severity(),
            Error::RuntimeUnavailable { .. } => Severity::Warning,
            Error::TokenSetLoad { source, .. } => source.severity(),
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Internal(e) => e.http_status(),
            Error::Chain(e) => e.http_status(),
            Error::RuntimeUnavailable { .. } => 503,
            Error::TokenSetLoad { source, .. } => source.http_status(),
        }
    }
}
