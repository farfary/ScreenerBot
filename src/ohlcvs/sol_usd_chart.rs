//! SOL/USD reference chart (bot side).
//!
//! Mirrors the data server's full SOL/USD series into an in-memory cache so the
//! bot ALWAYS has SOL's own price history (all timeframes) ready for display and
//! for converting USDC-denominated prices to SOL — prepared during runtime, never
//! computed per request. The durable multi-year history lives on the server
//! (`/v1/sol_usd`, never evicted); the bot re-pulls it at startup and refreshes it
//! periodically. Purely SOL/USD — this is the ONE deliberately USD series; token
//! candles stay SOL-denominated elsewhere.

use crate::config::with_config;
use crate::ohlcvs::types::{Candle, Timeframe};
use arc_swap::ArcSwap;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
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

/// Server timeframe string (matches the `#[serde(rename)]` on `Timeframe`).
fn tf_str(tf: Timeframe) -> &'static str {
    match tf {
        Timeframe::Minute1 => "1m",
        Timeframe::Minute5 => "5m",
        Timeframe::Minute15 => "15m",
        Timeframe::Hour1 => "1h",
        Timeframe::Hour4 => "4h",
        Timeframe::Hour12 => "12h",
        Timeframe::Day1 => "1d",
    }
}

/// How many candles to pull per timeframe. Coarse frames carry deep history, so
/// pull generously (the server caps at 10k); fine frames only reach back weeks.
fn pull_limit(tf: Timeframe) -> usize {
    match tf {
        Timeframe::Day1 | Timeframe::Hour12 | Timeframe::Hour4 => 10_000,
        Timeframe::Hour1 => 5_000,
        _ => 2_000,
    }
}

#[derive(Default)]
struct Chart {
    /// Per-timeframe candles, ascending by timestamp (chart-ready).
    series: HashMap<Timeframe, Arc<Vec<Candle>>>,
    /// Per-timeframe ts -> close, for O(1) USD→SOL conversion.
    close_maps: HashMap<Timeframe, Arc<HashMap<i64, f64>>>,
    /// Unix seconds of the last successful refresh (0 = never).
    updated_at: i64,
}

static CHART: LazyLock<ArcSwap<Chart>> = LazyLock::new(|| ArcSwap::from_pointee(Chart::default()));

/// The SOL/USD candles for a timeframe (ascending), empty until first refresh.
pub fn series(tf: Timeframe) -> Arc<Vec<Candle>> {
    CHART.load().series.get(&tf).cloned().unwrap_or_default()
}

/// SOL/USD close at a period-aligned timestamp for a timeframe, if cached.
pub fn close_at(tf: Timeframe, ts: i64) -> Option<f64> {
    CHART
        .load()
        .close_maps
        .get(&tf)
        .and_then(|m| m.get(&ts).copied())
}

/// Unix seconds since the last successful refresh (None if never refreshed).
pub fn last_updated() -> Option<i64> {
    let ts = CHART.load().updated_at;
    (ts > 0).then_some(ts)
}

/// Convert a token's USD candles (same timeframe grid) into SOL by dividing OHLC
/// by the SOL/USD close at each timestamp. Candles without SOL/USD coverage are
/// dropped — never fabricated. Empty until the chart has been fetched.
pub fn convert_usd_to_sol(tf: Timeframe, usd: &[Candle]) -> Vec<Candle> {
    let guard = CHART.load();
    let Some(map) = guard.close_maps.get(&tf) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(usd.len());
    for c in usd {
        let Some(&s) = map.get(&c.timestamp) else {
            continue;
        };
        if s <= 0.0 {
            continue;
        }
        out.push(Candle {
            timestamp: c.timestamp,
            open: c.open / s,
            high: c.high / s,
            low: c.low / s,
            close: c.close / s,
            volume: c.volume,
        });
    }
    out
}

/// The server response shape for `/v1/sol_usd`.
#[derive(serde::Deserialize)]
struct SolUsdResponse {
    candles: Vec<Candle>,
}

/// Fetch one timeframe from the data server. Returns None on any miss/error.
async fn fetch_tf(endpoint: &str, tf: Timeframe, timeout_secs: u64) -> Option<Vec<Candle>> {
    let url = format!(
        "{}/v1/sol_usd?timeframe={}&limit={}",
        endpoint.trim_end_matches('/'),
        tf_str(tf),
        pull_limit(tf)
    );
    let resp = crate::net::client()
        .get(&url)
        .timeout(Duration::from_secs(timeout_secs))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body = resp.json::<SolUsdResponse>().await.ok()?;
    Some(body.candles)
}

/// Refresh all timeframes from the server and atomically swap the cache in. Keeps
/// any timeframe the server couldn't serve this round (partial refresh is fine).
async fn refresh_once() {
    let (enabled, endpoint, timeout_secs) = with_config(|c| {
        let s = &c.ohlcv.sources.screenerbot_server;
        (s.enabled, s.endpoint.clone(), s.timeout_seconds)
    });
    if !enabled || endpoint.trim().is_empty() {
        return;
    }

    let prev = CHART.load();
    let mut series = prev.series.clone();
    let mut close_maps = prev.close_maps.clone();
    let mut any = false;

    for tf in TIMEFRAMES {
        if let Some(candles) = fetch_tf(&endpoint, tf, timeout_secs).await {
            if candles.is_empty() {
                continue;
            }
            let map: HashMap<i64, f64> = candles.iter().map(|c| (c.timestamp, c.close)).collect();
            series.insert(tf, Arc::new(candles));
            close_maps.insert(tf, Arc::new(map));
            any = true;
        }
    }

    if any {
        let total: usize = series.values().map(|v| v.len()).sum();
        CHART.store(Arc::new(Chart {
            series,
            close_maps,
            updated_at: chrono::Utc::now().timestamp(),
        }));
        crate::logger::info(
            crate::logger::LogTag::Ohlcv,
            &format!(
                "SOL/USD reference chart refreshed: {total} candles across {} timeframes",
                TIMEFRAMES.len()
            ),
        );
    }
}

/// Spawn the background refresher: pull the full chart once at startup, then keep
/// it current on a relaxed cadence (SOL history changes slowly; the recent tail is
/// what moves). Skips ticks while the network is offline. Returns the task handle.
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
