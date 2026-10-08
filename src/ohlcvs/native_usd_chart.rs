// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Native/USD reference chart per enabled chain (bot side).
//!
//! Mirrors the Data Server's native/USD series of each enabled chain, every
//! timeframe, into an in-memory slot per chain, so the native asset's own price
//! history is ready for display without a per-request computation. The durable
//! multi-year history lives on the Data Server; the bot pulls it at startup and
//! refreshes it periodically. This is the one deliberately USD-denominated series;
//! token candles stay native-denominated elsewhere.

use crate::chains::{ChainId, ChainScope, PerChain};
use crate::ohlcvs::types::{Candle, Timeframe};
use arc_swap::ArcSwap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// Every timeframe we mirror.
const TIMEFRAMES: [Timeframe; 7] = [
    Timeframe::Minute1,
    Timeframe::Minute5,
    Timeframe::Minute15,
    Timeframe::Hour1,
    Timeframe::Hour4,
    Timeframe::Hour12,
    Timeframe::Day1,
];

/// How many candles to pull per timeframe. Coarse frames carry deep history, so
/// pull generously (the server caps at 10k); fine frames only reach back weeks.
fn pull_limit(tf: Timeframe) -> usize {
    match tf {
        // Coarse frames carry the deep multi-year history (back to 2020); pull the
        // server's max so the bot mirror is as deep as one call allows.
        Timeframe::Day1 | Timeframe::Hour12 | Timeframe::Hour4 | Timeframe::Hour1 => 10_000,
        _ => 2_000,
    }
}

#[derive(Default)]
struct Chart {
    /// Per-timeframe candles, ascending by timestamp (chart-ready).
    series: HashMap<Timeframe, Arc<Vec<Candle>>>,
    /// Unix seconds of the last successful refresh (0 = never).
    updated_at: i64,
}

static CHART: PerChain<ArcSwap<Chart>> = PerChain::new(empty_chart);

fn empty_chart(_chain: ChainId) -> ArcSwap<Chart> {
    ArcSwap::from_pointee(Chart::default())
}

/// The chain's native/USD candles for a timeframe (ascending), empty until its
/// first refresh.
pub fn series(chain: ChainId, tf: Timeframe) -> Arc<Vec<Candle>> {
    CHART
        .get(chain)
        .load()
        .series
        .get(&tf)
        .cloned()
        .unwrap_or_default()
}

/// Unix seconds of the chain's last successful refresh (None if never refreshed).
pub fn last_updated(chain: ChainId) -> Option<i64> {
    let ts = CHART.get(chain).load().updated_at;
    (ts > 0).then_some(ts)
}

/// The chain's native/USD 24h price change as a percentage, from its 1h series.
/// None until enough 1h history is cached.
pub fn change_24h_percent(chain: ChainId) -> Option<f64> {
    change_over_24_hourly(&series(chain, Timeframe::Hour1))
}

/// Percentage change from the close 24 hourly candles before the last one to the
/// last close. None with fewer than 25 candles or a non-positive earlier close.
fn change_over_24_hourly(series: &[Candle]) -> Option<f64> {
    let n = series.len();
    if n < 25 {
        return None;
    }
    let now = series[n - 1].close;
    let prev = series[n - 25].close;
    (prev > 0.0).then(|| (now - prev) / prev * 100.0)
}

/// The Data Server response shape for the native/USD series.
#[derive(serde::Deserialize)]
struct NativeUsdResponse {
    candles: Vec<Candle>,
}

/// Fetch one timeframe of the chain's series from the data server. Returns None
/// on any miss/error.
async fn fetch_tf(chain: ChainId, tf: Timeframe) -> Option<Vec<Candle>> {
    let body = crate::data_server::get_json::<NativeUsdResponse>(
        crate::data_server::Surface::Ohlcv,
        chain,
        "/v1/native_usd",
        &[
            ("timeframe", tf.as_str().to_string()),
            ("limit", pull_limit(tf).to_string()),
        ],
    )
    .await?;
    Some(body.candles)
}

/// Refresh every enabled chain's timeframes from the Data Server and swap each
/// chain's slot in atomically. A timeframe the server could not serve this round
/// keeps its previous series (a partial refresh is fine).
async fn refresh_once() {
    // Seven timeframes per chain: ask once whether the source is usable at all
    // rather than discovering it per request. `is_usable` is optimistic on an
    // unknown or transient state, so a first run and a blip both still try.
    if !crate::data_server::is_usable(crate::data_server::Surface::Ohlcv) {
        return;
    }
    for chain in ChainScope::All.chains() {
        refresh_chain(chain).await;
    }
}

async fn refresh_chain(chain: ChainId) {
    let slot = CHART.get(chain);
    let mut series = slot.load().series.clone();
    let mut any = false;

    for tf in TIMEFRAMES {
        if let Some(candles) = fetch_tf(chain, tf).await {
            if candles.is_empty() {
                continue;
            }
            series.insert(tf, Arc::new(candles));
            any = true;
        }
    }

    if any {
        let total: usize = series.values().map(|v| v.len()).sum();
        slot.store(Arc::new(Chart {
            series,
            updated_at: chrono::Utc::now().timestamp(),
        }));
        crate::logger::info(
            crate::logger::LogTag::Ohlcv,
            &format!(
                "{chain} native/USD reference chart refreshed: {total} candles across {} timeframes",
                TIMEFRAMES.len()
            ),
        );
    }
}

/// Spawn the background refresher: pull the full chart once at startup, then keep
/// it current on a relaxed cadence (the history changes slowly; the recent tail is
/// what moves). One task serves every enabled chain. Skips ticks while the network is offline. Returns the task handle.
pub fn start(shutdown: Arc<Notify>, monitor: tokio_metrics::TaskMonitor) -> JoinHandle<()> {
    tokio::spawn(monitor.instrument(async move {
        // Prime immediately so the chart is available as early as possible.
        if !crate::connectivity::is_network_offline() {
            refresh_once().await;
        }
        let mut tick = tokio::time::interval(Duration::from_secs(300));
        tick.tick().await; // consume the immediate first tick
        loop {
            tokio::select! {
                _ = shutdown.notified() => break,
                _ = tick.tick() => {
                    if !crate::connectivity::is_network_offline() {
                        refresh_once().await;
                    }
                }
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hourly(closes: &[f64]) -> Vec<Candle> {
        closes
            .iter()
            .enumerate()
            .map(|(i, &close)| Candle::new(i as i64 * 3600, close, close, close, close, 1.0))
            .collect()
    }

    #[test]
    fn change_needs_twenty_five_hourly_candles() {
        assert_eq!(change_over_24_hourly(&hourly(&[1.0; 24])), None);
        assert_eq!(change_over_24_hourly(&[]), None);
    }

    #[test]
    fn change_refuses_a_non_positive_earlier_close() {
        let mut closes = vec![1.0; 25];
        closes[0] = 0.0;
        assert_eq!(change_over_24_hourly(&hourly(&closes)), None);
    }

    #[test]
    fn change_compares_the_last_close_with_the_close_24_candles_earlier() {
        let mut closes = vec![7.0; 30];
        closes[5] = 100.0;
        closes[29] = 110.0;
        let change = change_over_24_hourly(&hourly(&closes)).unwrap();
        assert!((change - 10.0).abs() < 1e-9, "{change}");
    }

    #[test]
    fn a_chart_stored_in_a_chain_slot_is_read_back_for_that_chain() {
        let candles = hourly(&[1.0, 2.0]);
        let mut series_map = HashMap::new();
        series_map.insert(Timeframe::Hour1, Arc::new(candles));
        CHART.get(ChainId::Solana).store(Arc::new(Chart {
            series: series_map,
            updated_at: 1_700_000_000,
        }));
        assert_eq!(series(ChainId::Solana, Timeframe::Hour1).len(), 2);
        assert!(series(ChainId::Solana, Timeframe::Day1).is_empty());
        assert_eq!(last_updated(ChainId::Solana), Some(1_700_000_000));
    }
}
