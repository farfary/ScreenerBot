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
    #[error("Pool not found: {0}")]
    PoolNotFound(String),
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

/// Result alias for the OHLCV module.
pub type OhlcvResult<T> = Result<T, OhlcvError>;
