// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Chain-owned candle feeds that the OHLCV fetcher tries after the Data Server.

use std::future::Future;
use std::pin::Pin;

use crate::ohlcvs::types::{Candle, Timeframe};

/// Fetches one chain-owned candle feed by token address, ascending by timestamp.
pub type CandleFeedFn =
    fn(
        String,
        Timeframe,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Candle>, crate::apis::Error>> + Send>>;

/// A candle feed a chain contributes beyond the Data Server and GeckoTerminal,
/// recorded under `label` in OHLCV events and as the stored candle source.
#[derive(Clone, Copy)]
pub struct CandleFeed {
    pub label: &'static str,
    pub fetch: CandleFeedFn,
}
