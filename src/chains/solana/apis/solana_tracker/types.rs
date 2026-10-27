// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! SolanaTracker API response types.

use serde::Deserialize;

/// OHLCV response — note: API returns key "oclhv" (their typo, not "ohlcv")
#[derive(Debug, Clone, Deserialize)]
pub struct OhlcvResponse {
    /// Candle data (API key is "oclhv" — a typo in their API)
    #[serde(alias = "oclhv", alias = "ohlcv", default)]
    pub candles: Vec<OhlcvCandle>,
}

/// Single OHLCV candle from SolanaTracker
#[derive(Debug, Clone, Deserialize)]
pub struct OhlcvCandle {
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
    /// Unix timestamp in seconds
    pub time: i64,
}

/// Credits response
#[derive(Debug, Clone, Deserialize)]
pub struct CreditsResponse {
    pub credits: i64,
}

/// Search result
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub mint: String,
    #[serde(default)]
    pub image: Option<String>,
}
