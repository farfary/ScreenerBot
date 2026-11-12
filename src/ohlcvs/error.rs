// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Errors produced by the OHLCV module: candle storage, fetching, caching and series-pool state.

use crate::errors::{DatabaseError, InternalError};

/// Everything that can go wrong fetching, storing and serving candles.
#[derive(Debug, Clone, thiserror::Error)]
pub enum OhlcvError {
    /// A classified storage failure of the OHLCV store, kept typed so startup and
    /// recovery can tell a schema conflict or a damaged file from a transient failure.
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    /// A task join failure or another invariant violation, not a data problem.
    #[error("Internal error: {0}")]
    Internal(#[from] InternalError),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    /// The provider refuses this client as a whole (a bot-challenge page instead of an answer),
    /// so the request was not made or was refused for that reason alone.
    #[error("{provider} is blocking requests")]
    ProviderBlocked { provider: &'static str },
    #[error("Pool not found: {0}")]
    PoolNotFound(String),
    /// Another task is building this mint's strategy bundle; its result lands in the
    /// bundle cache, so the caller reads the cache on its next cycle instead.
    #[error("Bundle build already in progress for {0}")]
    BundleBuildInProgress(String),
    #[error("Invalid timeframe: {0}")]
    InvalidTimeframe(String),
    #[error("Data gap detected: {start} to {end}")]
    DataGap { start: i64, end: i64 },
    #[error("Cache error: {0}")]
    CacheError(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Chain: {0}")]
    Chain(#[from] crate::chains::Error),
    /// A candle or backfill-flag write for a pool that is no longer the token's series pool.
    #[error("Pool {pool} is no longer the series pool of {mint}")]
    SeriesPoolMoved { mint: String, pool: String },
}

impl OhlcvError {
    /// Whether the request was not served because its source cannot be asked now (rate limited,
    /// or blocking this client as a whole). Such a failure says nothing about the pool or the
    /// gap span it asked for, so it counts against neither.
    pub(crate) fn is_backpressure(&self) -> bool {
        matches!(self, Self::RateLimitExceeded | Self::ProviderBlocked { .. })
    }
}

/// Result alias for the OHLCV module.
pub type OhlcvResult<T> = Result<T, OhlcvError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_source_that_cannot_be_asked_now_is_backpressure() {
        let cases = [
            ("rate limited", OhlcvError::RateLimitExceeded, true),
            (
                "provider blocked",
                OhlcvError::ProviderBlocked {
                    provider: "Provider",
                },
                true,
            ),
            (
                "provider error",
                OhlcvError::ApiError("HTTP 500".to_string()),
                false,
            ),
            (
                "unknown pool",
                OhlcvError::PoolNotFound("pool".to_string()),
                false,
            ),
            ("no data", OhlcvError::NotFound("mint".to_string()), false),
        ];
        for (case, error, backpressure) in cases {
            assert_eq!(error.is_backpressure(), backpressure, "{case}");
        }
    }
}
