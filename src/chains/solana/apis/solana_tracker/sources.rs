// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Neutral-facing OHLCV fallback over the Solana-only SolanaTracker client.
//!
//! This provider moved under the Solana adapter; the neutral OHLCV
//! fetcher reaches it through fn seams installed by
//! `crate::run::services::register_all_services`. This file is deleted
//! when OHLCV sources become per-chain.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock};

use super::{
    SolanaTrackerClient, RATE_LIMIT_PER_MINUTE as ST_RATE_LIMIT, TIMEOUT_SECS as ST_TIMEOUT,
};
use crate::config::get_config_clone;

/// Client singleton (construction moved from `apis::manager.rs`; identical
/// config logic, identical warning text).
static CLIENT: LazyLock<Arc<SolanaTrackerClient>> = LazyLock::new(|| {
    let cfg = get_config_clone();
    let st_cfg = &cfg.ohlcv.sources.solana_tracker;
    let st_enabled = st_cfg.enabled && !st_cfg.api_key.is_empty();
    let st_rate_limit = if st_cfg.rate_limit_per_minute == 0 {
        ST_RATE_LIMIT
    } else {
        st_cfg.rate_limit_per_minute as usize
    };
    let st_timeout = if st_cfg.timeout_seconds == 0 {
        ST_TIMEOUT
    } else {
        st_cfg.timeout_seconds
    };
    Arc::new(
        SolanaTrackerClient::with_base_url(
            st_enabled,
            st_cfg.api_key.clone(),
            st_rate_limit,
            st_timeout,
            st_cfg.endpoint.clone(),
        )
        .unwrap_or_else(|e| {
            crate::logger::warning(
                crate::logger::LogTag::Api,
                &format!(
                    "Failed to initialize SolanaTracker client: {} - using disabled client",
                    e
                ),
            );
            SolanaTrackerClient::new(false, String::new(), ST_RATE_LIMIT, ST_TIMEOUT)
                .expect("Failed to create disabled SolanaTracker client")
        }),
    )
});

/// The process-wide SolanaTracker client (one rate limiter, one stats tracker).
pub fn client() -> Arc<SolanaTrackerClient> {
    Arc::clone(&CLIENT)
}

/// Whether the SolanaTracker OHLCV fallback is enabled with a key.
pub fn enabled() -> bool {
    client().is_enabled()
}

/// The provider call + candle mapping + ascending sort from
/// `ohlcvs/fetcher.rs::fetch_from_solana_tracker`, verbatim except the client
/// source. Returns the provider's own typed error so the fetcher's
/// `err.to_string()` logging stays byte-identical.
pub fn fetch_candles(
    mint: String,
    interval: String,
) -> Pin<Box<dyn Future<Output = Result<Vec<crate::ohlcvs::Candle>, crate::apis::Error>> + Send>> {
    Box::pin(fetch(mint, interval))
}

async fn fetch(
    mint: String,
    interval: String,
) -> Result<Vec<crate::ohlcvs::Candle>, crate::apis::Error> {
    let response = client()
        .fetch_ohlcv(&mint, &interval, "sol", None, None)
        .await;
    match response {
        Ok(ohlcv) => {
            let mut data_points: Vec<crate::ohlcvs::Candle> = ohlcv
                .candles
                .into_iter()
                .map(|c| crate::ohlcvs::Candle {
                    timestamp: c.time,
                    open: c.open,
                    high: c.high,
                    low: c.low,
                    close: c.close,
                    volume: c.volume,
                })
                .collect();
            // SolanaTracker returns newest first, sort by timestamp ascending
            data_points.sort_by_key(|c| c.timestamp);
            Ok(data_points)
        }
        Err(err) => Err(err),
    }
}
