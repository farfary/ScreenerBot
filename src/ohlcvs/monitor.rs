// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV monitor service — background worker that fetches candles on schedule.

use crate::config::with_config;
use crate::events::{record_ohlcv_event, Severity};
use crate::i18n::{ids, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::ohlcvs::aggregator::OhlcvAggregator;
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::{OhlcvDatabase, StoredBucket};
use crate::ohlcvs::feeds::CandleFeed;
use crate::ohlcvs::fetcher::{CandleSource, OhlcvFetcher, MAX_CANDLES_PER_REQUEST};
use crate::ohlcvs::gaps::{GapManager, GAP_FILL_REQUESTS_PER_CYCLE};
use crate::ohlcvs::manager::PoolManager;
use crate::ohlcvs::priorities::{ActivityType, PriorityManager};
use crate::ohlcvs::types::{
    Candle, MonitorStats, MonitorTelemetrySnapshot, OhlcvError, OhlcvResult, PoolConfig, Priority,
    Timeframe, TokenOhlcvConfig,
};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tokio::task::spawn_blocking;
use tokio::time::{interval, sleep, Duration, Instant};

const AGGREGATED_TIMEFRAMES: [Timeframe; 6] = [
    Timeframe::Minute5,
    Timeframe::Minute15,
    Timeframe::Hour1,
    Timeframe::Hour4,
    Timeframe::Hour12,
    Timeframe::Day1,
];

pub(super) const GAP_SUMMARY_LIMIT: usize = 5;

/// Smallest native refresh request: the forming bucket, the one before it and a margin.
const MIN_REFRESH_CANDLES: usize = 3;

/// Buckets requested beyond the measured lag, so a bucket that closes while the
/// request is in flight is still covered.
pub(super) const CATCH_UP_MARGIN: usize = 2;

/// Minimum spacing of two native fetches that count toward the no-newer rule.
const NATIVE_RETRY_DELAY_SECS: i64 = 15;

/// Delay of the first re-fetch after the Data Server reports it is refreshing
/// a series. Each further re-fetch doubles the delay up to
/// `REFRESHING_REFETCH_MAX_DELAY_SECS`.
const REFRESHING_REFETCH_BASE_DELAY_SECS: i64 = 15;

/// Longest delay between two re-fetches of a refreshing series.
const REFRESHING_REFETCH_MAX_DELAY_SECS: i64 = 300;

/// Re-fetches of one timeframe scheduled while the Data Server keeps reporting
/// that it is refreshing the series (about 33 minutes in total). The server's
/// upstream read can land tens of minutes after its first answer when its
/// egress is rate-limited, so a short window would keep a partial closed
/// bucket in place.
const MAX_REFRESHING_REFETCHES: u32 = 10;

/// Delay of the next re-fetch of a series the Data Server reports as
/// refreshing, after `refetches_so_far` re-fetches have been scheduled:
/// 15, 30, 60, 120, 240 s, then 300 s. `None` once the budget is spent.
fn refreshing_refetch_delay_secs(refetches_so_far: u32) -> Option<i64> {
    if refetches_so_far >= MAX_REFRESHING_REFETCHES {
        return None;
    }
    let delay = REFRESHING_REFETCH_BASE_DELAY_SECS
        .checked_shl(refetches_so_far)
        .unwrap_or(REFRESHING_REFETCH_MAX_DELAY_SECS);
    Some(delay.min(REFRESHING_REFETCH_MAX_DELAY_SECS))
}

/// Delay before a timeframe whose stored native values came from a fallback
/// provider is fetched again while the Data Server is usable. A fallback answer
/// differs from the Data Server's series, and waiting the full interval (hours
/// at low priority) would keep those values in the chart and the strategies.
const FALLBACK_RETRY_DELAY_SECS: i64 = 300;

/// Consecutive fallback-served pages after which the short fallback retry stops
/// (about 30 minutes at `FALLBACK_RETRY_DELAY_SECS`). Past it the timeframe
/// follows its ordinary interval until a Data Server page replaces the values.
const MAX_FALLBACK_RETRIES: u32 = 6;

/// Margin past the Data Server's freshness window before a closed bucket is
/// re-read, so the read lands after the server's own re-fetch of that bucket.
const SETTLE_MARGIN_SECS: i64 = 60;

/// Bounds of a settle delay. The upper bound keeps a closed coarse bucket from
/// carrying a provisional value for hours on any priority.
const MIN_SETTLE_SECS: i64 = 60;
const MAX_SETTLE_SECS: i64 = 1_200;

/// Settle delay of a closed bucket: how long after a bucket closes its native
/// row is read once more, so it carries the source's final value rather than
/// the provisional one a fetch right after the close can return.
///
/// Derived from the Data Server, which re-reads a requested series from its
/// upstream once the newest stored candle is older than a per-timeframe
/// freshness window (1m 90 s, 5m 4 min, 15m 12 min, 1h 45 min, 4h 3 h,
/// 12h 9 h, 1d 18 h). The delay is that window plus `SETTLE_MARGIN_SECS`,
/// clamped to `[MIN_SETTLE_SECS, MAX_SETTLE_SECS]`: 5m 300 s, 15m 780 s, 1h and
/// coarser 1200 s. For 1h and coarser the clamp is shorter than the server's
/// window, so the settle read returns a final value only when the server's
/// first read after the close ran after its upstream finalized the bucket.
/// The chain's candle feeds and GeckoTerminal finalize within these delays, so
/// the same rule holds when they are the source. Independent of priority.
fn settle_delay_secs(timeframe: Timeframe) -> i64 {
    let server_freshness_window = match timeframe {
        Timeframe::Minute1 => 90,
        Timeframe::Minute5 => 240,
        Timeframe::Minute15 => 720,
        Timeframe::Hour1 => 2_700,
        Timeframe::Hour4 => 10_800,
        Timeframe::Hour12 => 32_400,
        Timeframe::Day1 => 64_800,
    };
    (server_freshness_window + SETTLE_MARGIN_SECS).clamp(MIN_SETTLE_SECS, MAX_SETTLE_SECS)
}

/// Unix secs at which the bucket starting at `bucket` is settled.
fn settle_point(bucket: i64, timeframe: Timeframe) -> i64 {
    bucket + timeframe.to_seconds() + settle_delay_secs(timeframe)
}

/// Start of the newest closed bucket whose settle point has passed at `now`.
/// Once the settle delay of the bucket that closed last has elapsed this is
/// that bucket; before then it is the one before it.
fn settle_target(now: i64, timeframe: Timeframe) -> i64 {
    OhlcvDatabase::bucket_start(now - settle_delay_secs(timeframe), timeframe)
        - timeframe.to_seconds()
}

/// Pair each candle aggregated from `minute_candles` (sorted ascending) into
/// `timeframe` with the start of the newest 1m candle inside its bucket.
fn with_newest_minute(
    minute_candles: &[Candle],
    aggregated: Vec<Candle>,
    timeframe: Timeframe,
) -> Vec<(Candle, i64)> {
    let bucket = timeframe.to_seconds();
    aggregated
        .into_iter()
        .filter_map(|candle| {
            let end = minute_candles.partition_point(|m| m.timestamp < candle.timestamp + bucket);
            let newest = minute_candles[..end].last()?.timestamp;
            (newest >= candle.timestamp).then_some((candle, newest))
        })
        .collect()
}

/// In-memory coverage state of one native (mint, timeframe) series.
#[derive(Debug, Default, Clone, PartialEq)]
struct NativeSeriesState {
    /// Unix secs of the last native fetch attempt, successful or not.
    last_fetch_at: Option<i64>,
    /// The last page came from a Data Server refreshing behind its answer.
    last_page_refreshing: bool,
    /// Stored native newest when the current no-newer streak began, and when.
    no_newer_since: Option<(Option<i64>, i64)>,
    /// A later no-newer fetch, at least the retry delay after the first,
    /// confirmed the streak.
    no_newer_confirmed: bool,
    /// When the scheduled re-fetch after a `refreshing` page is due.
    refetch_at: Option<i64>,
    /// Re-fetches scheduled since the last page that was not refreshing.
    refetches: u32,
    /// The last page that carried storable candles was served by a fallback
    /// provider, so the stored native values may differ from the Data Server's.
    fallback_values: bool,
    /// Consecutive pages with candles served by a fallback provider since the
    /// last Data Server page.
    fallback_pages: u32,
    /// Newest bucket a completed fetch read at or after its settle point. A
    /// fetch that returns identical values leaves the row's `fetched_at`
    /// untouched, so this is what stops the settle rule from firing again.
    settled_bucket: Option<i64>,
}

impl NativeSeriesState {
    fn record_failure(&mut self, now: i64) {
        self.last_fetch_at = Some(now);
    }

    /// Record a native page. `stored_newest` is the native newest before the page
    /// was written; `fetched_newest` is the newest storable bucket in the page.
    fn record_page(
        &mut self,
        now: i64,
        stored_newest: Option<i64>,
        fetched_newest: Option<i64>,
        server_refreshing: bool,
    ) {
        self.last_fetch_at = Some(now);
        self.last_page_refreshing = server_refreshing;

        let newer = match (fetched_newest, stored_newest) {
            (Some(fetched), Some(stored)) => fetched > stored,
            (Some(_), None) => true,
            (None, _) => false,
        };
        if newer {
            self.no_newer_since = None;
            self.no_newer_confirmed = false;
        } else {
            match self.no_newer_since {
                Some((anchor, since)) if anchor == stored_newest => {
                    if now - since >= NATIVE_RETRY_DELAY_SECS {
                        self.no_newer_confirmed = true;
                    }
                }
                _ => {
                    self.no_newer_since = Some((stored_newest, now));
                    self.no_newer_confirmed = false;
                }
            }
        }

        if !server_refreshing {
            self.refetches = 0;
            self.refetch_at = None;
        } else if let Some(delay) = refreshing_refetch_delay_secs(self.refetches) {
            self.refetches += 1;
            self.refetch_at = Some(now + delay);
        } else {
            self.refetch_at = None;
        }
    }

    /// Record which upstream served a page. A Data Server page replaces any
    /// fallback values (native rows are last-write-wins); a fallback page counts
    /// only when it carried storable candles, since an empty one wrote nothing.
    fn record_source(&mut self, source: Option<CandleSource>, carried_candles: bool) {
        match source {
            Some(CandleSource::DataServer) => {
                self.fallback_values = false;
                self.fallback_pages = 0;
            }
            Some(source) if source.is_fallback() && carried_candles => {
                self.fallback_values = true;
                self.fallback_pages = self.fallback_pages.saturating_add(1);
            }
            _ => {}
        }
    }

    /// Record that a completed fetch at `now` read every bucket whose settle point
    /// has passed. A Data Server page that announced a refresh still pending
    /// does not count while a scheduled re-fetch remains.
    fn record_settled(&mut self, now: i64, timeframe: Timeframe) {
        if self.refetch_at.is_some() {
            return;
        }
        let target = settle_target(now, timeframe);
        self.settled_bucket = Some(self.settled_bucket.map_or(target, |b| b.max(target)));
    }

    /// The no-newer rule holds for the series as it is stored now.
    fn no_newer_holds(&self, newest: Option<i64>) -> bool {
        self.no_newer_confirmed
            && self
                .no_newer_since
                .is_some_and(|(anchor, _)| anchor == newest)
    }

    fn is_caught_up(&self, newest: Option<i64>, now: i64, timeframe: Timeframe) -> bool {
        series_caught_up(
            newest,
            now,
            timeframe,
            self.last_page_refreshing,
            self.no_newer_holds(newest),
        )
    }
}

/// Buckets between the bucket holding `newest` and the current bucket.
fn buckets_behind(newest: i64, now: i64, timeframe: Timeframe) -> i64 {
    let current = OhlcvDatabase::bucket_start(now, timeframe);
    let stored = OhlcvDatabase::bucket_start(newest, timeframe);
    ((current - stored) / timeframe.to_seconds()).max(0)
}

/// Coverage rule of a native series. It is caught up when its newest stored
/// bucket is at most one bucket behind the current one (and the page that says
/// so was not a stale Data Server answer), or when the no-newer rule confirmed
/// that the source has nothing newer (an illiquid token with no trades).
fn series_caught_up(
    newest: Option<i64>,
    now: i64,
    timeframe: Timeframe,
    last_page_refreshing: bool,
    no_newer_confirmed: bool,
) -> bool {
    let recent = newest.is_some_and(|ts| buckets_behind(ts, now, timeframe) <= 1);
    (recent && !last_page_refreshing) || no_newer_confirmed
}

/// Candles to request for a newest-N native fetch: the lag since the newest
/// stored bucket plus a margin, clamped to `[MIN_REFRESH_CANDLES,
/// MAX_CANDLES_PER_REQUEST]`. An empty series asks for the backfill size.
fn catch_up_limit(newest: Option<i64>, now: i64, timeframe: Timeframe) -> usize {
    let Some(newest) = newest else {
        return timeframe
            .max_backfill_candles()
            .min(MAX_CANDLES_PER_REQUEST);
    };
    let behind = usize::try_from(buckets_behind(newest, now, timeframe)).unwrap_or(usize::MAX);
    behind
        .saturating_add(CATCH_UP_MARGIN)
        .clamp(MIN_REFRESH_CANDLES, MAX_CANDLES_PER_REQUEST)
}

/// Native refresh interval of a timeframe, scaled by priority the way
/// `Priority::base_interval` scales the 1m cadence, relative to `High` (the
/// priority of a viewed chart) and never below the base.
fn native_refresh_interval_secs(timeframe: Timeframe, priority: Priority) -> i64 {
    let base = match timeframe {
        Timeframe::Minute1 => timeframe.to_seconds(),
        Timeframe::Minute5 => 300,
        Timeframe::Minute15 => 900,
        Timeframe::Hour1 => 1_800,
        Timeframe::Hour4 | Timeframe::Hour12 | Timeframe::Day1 => 3_600,
    };
    let scale = (priority.base_interval().as_secs() / Priority::High.base_interval().as_secs())
        .max(1) as i64;
    base * scale
}

/// Whether a coarse timeframe's native refresh is due: it was never fetched
/// since start, a scheduled re-fetch after a `refreshing` page came due, its
/// priority-scaled interval elapsed, or it is behind (not caught up) and the
/// last attempt is at least the retry delay old.
///
/// The coverage state is in memory only, so the first cycle after start
/// refreshes every timeframe once: a stored newest bucket that was written
/// while still forming looks caught up (one bucket behind) but holds a partial
/// candle until the source is read again.
fn native_refresh_due(
    state: &NativeSeriesState,
    newest: Option<i64>,
    now: i64,
    timeframe: Timeframe,
    priority: Priority,
) -> bool {
    if state.refetch_at.is_some_and(|at| now >= at) {
        return true;
    }
    let Some(last) = state.last_fetch_at else {
        return true;
    };
    let behind = !state.is_caught_up(newest, now, timeframe);
    let elapsed = now - last;
    elapsed >= native_refresh_interval_secs(timeframe, priority)
        || (behind && elapsed >= NATIVE_RETRY_DELAY_SECS)
}

/// Whether a timeframe holding fallback values is due for another attempt at the
/// Data Server: the Data Server is usable, fewer than `MAX_FALLBACK_RETRIES`
/// re-fetches have been answered by a fallback in a row, and the last fetch is at
/// least `FALLBACK_RETRY_DELAY_SECS` old. When the Data Server is disabled or
/// unavailable the fallback is the primary source and only the ordinary rules
/// apply.
fn fallback_refresh_due(state: &NativeSeriesState, now: i64, data_server_usable: bool) -> bool {
    data_server_usable
        && state.fallback_values
        && state.fallback_pages <= MAX_FALLBACK_RETRIES
        && state
            .last_fetch_at
            .is_some_and(|last| now - last >= FALLBACK_RETRY_DELAY_SECS)
}

/// The bucket a settle refresh would read: the settle target, unless the series
/// is empty (the ordinary rules fetch it), the last attempt is younger than the
/// retry delay, or a fetch already read the target at or after its settle point.
fn settle_candidate(
    state: &NativeSeriesState,
    newest: Option<i64>,
    now: i64,
    timeframe: Timeframe,
) -> Option<i64> {
    newest?;
    if state
        .last_fetch_at
        .is_some_and(|last| now - last < NATIVE_RETRY_DELAY_SECS)
    {
        return None;
    }
    let target = settle_target(now, timeframe);
    if state
        .settled_bucket
        .is_some_and(|settled| settled >= target)
    {
        return None;
    }
    Some(target)
}

/// Whether a coarse timeframe is due to settle its newest closed bucket (see
/// `settle_delay_secs`). Due once the bucket's settle point has passed while
/// its stored row is unsettled: a native row whose values were last written
/// before the settle point, or a local aggregate only. A missing row in a
/// series with native rows is unsettled unless the no-newer rule already
/// confirmed the source has nothing newer. After one fetch at or past the
/// settle point the bucket is settled whatever that fetch returned, so a bucket
/// without trades is never re-read for settling.
fn settle_refresh_due(
    state: &NativeSeriesState,
    newest: Option<i64>,
    row: Option<StoredBucket>,
    now: i64,
    timeframe: Timeframe,
) -> bool {
    let Some(target) = settle_candidate(state, newest, now, timeframe) else {
        return false;
    };
    match row {
        Some(StoredBucket {
            native: true,
            fetched_at,
        }) => fetched_at.is_none_or(|at| at < settle_point(target, timeframe)),
        Some(StoredBucket { native: false, .. }) => true,
        None => !state.no_newer_holds(newest),
    }
}

/// A coarse timeframe selected for this refresh pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DueTimeframe {
    timeframe: Timeframe,
    newest: Option<i64>,
    last_fetch_at: Option<i64>,
}

/// Fetch order of a pass: most overdue first, measured as the time since the
/// last fetch relative to the timeframe's refresh interval, with a timeframe
/// never fetched since start ahead of all others. Ties keep the incoming
/// (fine-first) order. A pass stops at the first rate limit, so a timeframe it
/// did not reach leads the next pass instead of starving behind finer ones.
fn order_by_overdue(due: &mut [DueTimeframe], now: i64, priority: Priority) {
    due.sort_by(|a, b| match (a.last_fetch_at, b.last_fetch_at) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(last_a), Some(last_b)) => {
            // elapsed_a / interval_a vs elapsed_b / interval_b, descending,
            // cross-multiplied to stay in integers.
            let overdue_a = i128::from((now - last_a).max(0))
                * i128::from(native_refresh_interval_secs(b.timeframe, priority));
            let overdue_b = i128::from((now - last_b).max(0))
                * i128::from(native_refresh_interval_secs(a.timeframe, priority));
            overdue_b.cmp(&overdue_a)
        }
    });
}

/// The Data Server can answer OHLCV requests now: configured, online, signed
/// in, not refused, and not in a transport outage.
fn data_server_usable() -> bool {
    crate::data_server::is_usable(crate::data_server::Surface::Ohlcv)
        && !matches!(
            crate::data_server::access::current(),
            crate::data_server::DataAccess::Unreachable
        )
}

/// Newest canonical bucket in a page among the candles that are stored.
fn newest_storable_bucket(candles: &[Candle], timeframe: Timeframe) -> Option<i64> {
    candles
        .iter()
        .filter(|candle| OhlcvDatabase::is_storable(candle))
        .map(|candle| OhlcvDatabase::bucket_start(candle.timestamp, timeframe))
        .max()
}

/// Pause between two sequential native fetches of one token. Keeps
/// GeckoTerminal, the fallback when the Data Server misses, under its burst
/// limit.
fn inter_fetch_delay(priority: Priority) -> Duration {
    Duration::from_millis(match priority {
        Priority::Critical => 100,
        Priority::High => 200,
        Priority::Medium => 400,
        Priority::Low => 800,
    })
}

#[derive(Debug, Default, Clone)]
struct MonitorTelemetry {
    monitor_cycle_started_at: Option<DateTime<Utc>>,
    monitor_cycle_completed_at: Option<DateTime<Utc>>,
    monitor_cycle_duration_ms: Option<u64>,
    monitor_cycle_tokens_processed: usize,
    monitor_cycle_total: u64,
    gap_cycle_started_at: Option<DateTime<Utc>>,
    gap_cycle_completed_at: Option<DateTime<Utc>>,
    gap_cycle_duration_ms: Option<u64>,
    gap_cycle_tokens_processed: usize,
    gap_cycle_total: u64,
    last_rate_limit_at: Option<DateTime<Utc>>,
    rate_limit_events: u64,
    total_backfills_scheduled: u64,
    total_backfills_completed: u64,
    total_backfills_failed: u64,
    last_backfill_started_at: Option<DateTime<Utc>>,
    last_backfill_completed_at: Option<DateTime<Utc>>,
    last_backfill_duration_ms: Option<u64>,
    last_backfill_points: Option<usize>,
    last_backfill_error: Option<String>,
}

impl From<&MonitorTelemetry> for MonitorTelemetrySnapshot {
    fn from(value: &MonitorTelemetry) -> Self {
        Self {
            monitor_cycle_started_at: value.monitor_cycle_started_at.clone(),
            monitor_cycle_completed_at: value.monitor_cycle_completed_at.clone(),
            monitor_cycle_duration_ms: value.monitor_cycle_duration_ms,
            monitor_cycle_tokens_processed: value.monitor_cycle_tokens_processed,
            monitor_cycle_total: value.monitor_cycle_total,
            gap_cycle_started_at: value.gap_cycle_started_at.clone(),
            gap_cycle_completed_at: value.gap_cycle_completed_at.clone(),
            gap_cycle_duration_ms: value.gap_cycle_duration_ms,
            gap_cycle_tokens_processed: value.gap_cycle_tokens_processed,
            gap_cycle_total: value.gap_cycle_total,
            last_rate_limit_at: value.last_rate_limit_at.clone(),
            rate_limit_events: value.rate_limit_events,
            total_backfills_scheduled: value.total_backfills_scheduled,
            total_backfills_completed: value.total_backfills_completed,
            total_backfills_failed: value.total_backfills_failed,
            last_backfill_started_at: value.last_backfill_started_at.clone(),
            last_backfill_completed_at: value.last_backfill_completed_at.clone(),
            last_backfill_duration_ms: value.last_backfill_duration_ms,
            last_backfill_points: value.last_backfill_points,
            last_backfill_error: value.last_backfill_error.clone(),
        }
    }
}

pub struct OhlcvMonitor {
    db: Arc<OhlcvDatabase>,
    fetcher: Arc<OhlcvFetcher>,
    cache: Arc<OhlcvCache>,
    pool_manager: Arc<PoolManager>,
    gap_manager: Arc<GapManager>,
    active_tokens: Arc<RwLock<HashMap<String, TokenOhlcvConfig>>>,
    shutdown_signal: Arc<RwLock<bool>>,
    backfill_in_progress: Arc<Mutex<HashSet<String>>>,
    discovery_in_progress: Arc<Mutex<HashSet<String>>>,
    native_series: Arc<Mutex<HashMap<(String, Timeframe), NativeSeriesState>>>,
    telemetry: Arc<RwLock<MonitorTelemetry>>,
}

impl OhlcvMonitor {
    pub fn new(
        db: Arc<OhlcvDatabase>,
        fetcher: Arc<OhlcvFetcher>,
        cache: Arc<OhlcvCache>,
        pool_manager: Arc<PoolManager>,
        gap_manager: Arc<GapManager>,
    ) -> Self {
        Self {
            db,
            fetcher,
            cache,
            pool_manager,
            gap_manager,
            active_tokens: Arc::new(RwLock::new(HashMap::new())),
            shutdown_signal: Arc::new(RwLock::new(false)),
            backfill_in_progress: Arc::new(Mutex::new(HashSet::new())),
            discovery_in_progress: Arc::new(Mutex::new(HashSet::new())),
            native_series: Arc::new(Mutex::new(HashMap::new())),
            telemetry: Arc::new(RwLock::new(MonitorTelemetry::default())),
        }
    }

    /// Start monitoring all active tokens
    pub async fn start(self: Arc<Self>) -> OhlcvResult<()> {
        let enabled = crate::config::with_config(|cfg| cfg.ohlcv.enabled);
        if !enabled {
            logger::info(
                LogTag::Ohlcv,
                &"OHLCV monitor is disabled via config, skipping start".to_owned(),
            );
            return Ok(());
        }

        // Load active tokens from database
        self.load_active_tokens().await?;

        // Start background tasks
        tokio::spawn(self.clone().monitor_loop());
        tokio::spawn(self.clone().gap_fill_loop());
        tokio::spawn(self.clone().cleanup_loop());
        tokio::spawn(self.clone().cache_maintenance_loop());
        tokio::spawn(self.clone().sync_pool_service_tokens());

        Ok(())
    }

    /// Stop monitoring
    pub async fn stop(&self) {
        let mut shutdown = self.shutdown_signal.write().await;
        *shutdown = true;
    }

    /// Add a token to monitoring
    pub async fn add_token(&self, mint: String, priority: Priority) -> OhlcvResult<()> {
        let mut config = TokenOhlcvConfig::new(mint.clone(), priority);

        // Load existing pools from the database (fast, local). If none are stored
        // yet, do NOT block this call on network pool discovery: `discover_pools`
        // performs a rate-limited DexScreener/GeckoTerminal fetch that can stall
        // for tens of seconds when the provider budget is exhausted. Because
        // `add_token` is awaited inline by the `GET /tokens/:mint/ohlcv` handler
        // (every chart-dialog open, polled ~1s), a blocking discovery here wedged
        // the chart endpoint AND — via shared OHLCV DB contention — the token
        // detail endpoint's `has_data` check, leaving the dialog stuck on a
        // loading spinner. Instead, proceed immediately with whatever pools exist
        // and kick discovery off in the background; the discovered pools are
        // persisted to the DB, so the next poll (or the monitor loop) picks them
        // up and backfill starts then. Mirrors the pre-existing "discovery failed
        // -> empty pools -> retry later" path, just without the stall.
        let pools = match self.pool_manager.get_pools(&mint).await {
            Ok(pools) if !pools.is_empty() => pools,
            _ => {
                self.spawn_pool_discovery(&mint);
                Vec::new()
            }
        };

        let has_stored_pools = !pools.is_empty();
        config.pools = pools;

        // Store in database
        self.db.upsert_monitor_config(&config)?;

        // Add to active tokens
        let newly_active = {
            let mut active = self.active_tokens.write().await;
            active.insert(mint.clone(), config.clone()).is_none()
        };

        // Stored pool rows (liquidity, the canonical default) are a snapshot from the last
        // discovery, and discovery otherwise only runs for a token with NO pools — so a default
        // chosen by a since-fixed ranking, or a market that moved, never healed. Re-resolve in the
        // background when a token starts being monitored; the series pool follows on the next read.
        if newly_active && has_stored_pools {
            self.spawn_pool_discovery(&mint);
        }

        // Trigger multi-timeframe backfill for new token.
        // Guard with try_start_backfill so repeated add_token calls for the same
        // mint (every token-detail dialog open calls add_token_monitoring, plus
        // the monitor/sync loops) do NOT spawn concurrent full backfills of all 7
        // timeframes — that previously hammered GeckoTerminal far past its limit
        // and produced bursts of "Rate limit exceeded" warnings for the same
        // mint+timeframe within the same second.
        if let Some(pool) = config.series_pool() {
            if self.try_start_backfill(&mint) {
                let runner = self.clone();
                let mint_owned = mint.clone();
                let pool_owned = pool.address.clone();
                let priority_owned = priority;

                tokio::spawn(async move {
                    logger::debug(
                        LogTag::Ohlcv,
                        &format!(
                            "Triggering 30-day backfill for new token mint={} pool={} priority={:?}",
                            mint_owned, pool_owned, priority_owned
                        ),
                    );

                    match runner
                        .backfill_all_timeframes(&mint_owned, &pool_owned, priority_owned)
                        .await
                    {
                        Ok(total) => {
                            logger::debug(
                                LogTag::Ohlcv,
                                &format!(
                                    "Backfill completed for mint={} total_candles={}",
                                    mint_owned, total
                                ),
                            );
                        }
                        Err(e) => {
                            logger::warning(
                                LogTag::Ohlcv,
                                &format!("Backfill failed for mint={mint_owned}: {e}"),
                            );
                        }
                    }

                    runner.finish_backfill(&mint_owned);
                });
            }
        } else {
            logger::warning(
                LogTag::Ohlcv,
                &format!(
                    "No pool available for mint={}, backfill deferred until pool discovered",
                    mint
                ),
            );
        }

        Ok(())
    }

    /// Remove a token from monitoring
    pub async fn remove_token(&self, mint: &str) -> OhlcvResult<()> {
        let removed_config = {
            let mut active = self.active_tokens.write().await;
            active.remove(mint).map(|mut config| {
                config.is_active = false;
                config
            })
        };

        if let Some(config) = removed_config {
            self.db.upsert_monitor_config(&config)?;
        }

        if let Ok(mut series) = self.native_series.lock() {
            series.retain(|(series_mint, _), _| series_mint != mint);
        }

        Ok(())
    }

    /// Update token priority
    pub async fn update_priority(&self, mint: &str, priority: Priority) -> OhlcvResult<()> {
        let updated_config = {
            let mut active = self.active_tokens.write().await;
            active.get_mut(mint).map(|config| {
                config.priority = priority;
                config.fetch_frequency = priority.base_interval();
                config.clone()
            })
        };

        if let Some(config) = updated_config {
            self.db.upsert_monitor_config(&config)?;
        }

        Ok(())
    }

    /// Record activity for a token. Activities that need data now
    /// (`PositionOpened`, `DataRequested`) fetch the token immediately.
    pub async fn record_activity(
        &self,
        mint: &str,
        activity_type: ActivityType,
    ) -> OhlcvResult<()> {
        if self.mark_activity(mint, activity_type).await? {
            self.fetch_token_data(mint).await?;
        }
        Ok(())
    }

    /// Apply an activity's priority update to a monitored token and persist it,
    /// without fetching. Returns whether the activity calls for an immediate
    /// fetch; it is always false for a token outside the monitored set.
    pub async fn mark_activity(
        &self,
        mint: &str,
        activity_type: ActivityType,
    ) -> OhlcvResult<bool> {
        let (updated_config, should_trigger_fetch) = {
            let mut active = self.active_tokens.write().await;
            active.get_mut(mint).map_or((None, false), |config| {
                config.mark_activity();

                // Update priority based on activity
                let new_priority =
                    PriorityManager::update_priority_on_activity(config.priority, activity_type);
                config.priority = new_priority;
                config.fetch_frequency = new_priority.base_interval();

                let trigger = matches!(
                    activity_type,
                    ActivityType::PositionOpened | ActivityType::DataRequested
                );

                (Some(config.clone()), trigger)
            })
        };

        let Some(config) = updated_config else {
            return Ok(false);
        };
        self.db.upsert_monitor_config(&config)?;
        Ok(should_trigger_fetch)
    }

    /// Force refresh for a token. Pools are re-resolved in the background only when the token
    /// has none registered or its pool snapshot is past its TTL: a timeframe switch refreshes
    /// candles, and re-discovering on every one moved the series pool under the chart.
    pub async fn force_refresh(&self, mint: &str) -> OhlcvResult<()> {
        let snapshot_fresh = crate::tokens::has_fresh_token_pools_snapshot(self.db.chain(), mint);
        if self.pool_rediscovery_due(mint, snapshot_fresh).await? {
            self.spawn_pool_discovery(mint);
        }
        self.fetch_token_data(mint).await
    }

    /// Whether an explicit refresh re-resolves the token's pools: only with no registered pool
    /// or a pool snapshot past its TTL.
    async fn pool_rediscovery_due(&self, mint: &str, snapshot_fresh: bool) -> OhlcvResult<bool> {
        let has_pools = !self.pool_manager.get_pools(mint).await?.is_empty();
        Ok(!has_pools || !snapshot_fresh)
    }

    /// Whether the given mint is currently in the active monitoring set.
    pub async fn is_monitored(&self, mint: &str) -> bool {
        self.active_tokens.read().await.contains_key(mint)
    }

    /// Get monitoring statistics
    pub async fn get_stats(&self) -> MonitorStats {
        let active = self.active_tokens.read().await;
        let total_tokens = active.len();
        let by_priority = count_by_priority(&active);
        drop(active);

        let telemetry_snapshot = {
            let telemetry = self.telemetry.read().await;
            MonitorTelemetrySnapshot::from(&*telemetry)
        };

        let backfills_in_progress = self
            .backfill_in_progress
            .lock()
            .map(|set| set.len())
            .unwrap_or_default();

        let db = Arc::clone(&self.db);

        let (open_gap_tokens, open_gap_total, top_open_gaps) = match spawn_blocking(move || {
            let (token_count, gap_count) = db.get_gap_aggregate()?;
            let top = db.get_top_open_gaps(GAP_SUMMARY_LIMIT)?;
            Ok::<_, OhlcvError>((token_count, gap_count, top))
        })
        .await
        {
            Ok(Ok(result)) => result,
            Ok(Err(err)) => {
                logger::warning(
                    LogTag::Ohlcv,
                    &format!("Failed to collect gap stats: {err}"),
                );
                (0, 0, Vec::new())
            }
            Err(join_err) => {
                logger::warning(LogTag::Ohlcv, &format!("Gap stats join error: {join_err}"));
                (0, 0, Vec::new())
            }
        };

        MonitorStats {
            total_tokens,
            critical_tokens: by_priority
                .get(&Priority::Critical)
                .copied()
                .unwrap_or_default(),
            high_tokens: by_priority
                .get(&Priority::High)
                .copied()
                .unwrap_or_default(),
            medium_tokens: by_priority
                .get(&Priority::Medium)
                .copied()
                .unwrap_or_default(),
            low_tokens: by_priority.get(&Priority::Low).copied().unwrap_or_default(),
            cache_hit_rate: self.cache.hit_rate(),
            api_calls_per_minute: self.fetcher.calls_per_minute(),
            queue_size: self.fetcher.queue_size(),
            telemetry: telemetry_snapshot,
            backfills_in_progress,
            open_gap_tokens,
            open_gap_total,
            top_open_gaps,
        }
    }

    // ==================== Private Methods ====================

    async fn load_active_tokens(&self) -> OhlcvResult<()> {
        let configs = self.db.get_all_active_configs()?;

        let mut active = self.active_tokens.write().await;
        for config in configs {
            // Load pools for each token
            let pools = self.pool_manager.get_pools(&config.mint).await?;
            let mut full_config = config;
            full_config.pools = pools;

            active.insert(full_config.mint.clone(), full_config);
        }

        Ok(())
    }

    async fn monitor_loop(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(5)); // Check every 5 seconds

        // Calculate delay based on configured rate limit to respect API limits
        // Formula: (60_000ms / rate_limit_per_minute) + buffer
        let rate_limit: usize = with_config(|cfg| {
            if !cfg.ohlcv.sources.geckoterminal.enabled {
                0
            } else {
                let configured = cfg.ohlcv.sources.geckoterminal.rate_limit_per_minute as usize;
                if configured == 0 {
                    crate::apis::geckoterminal::RATE_LIMIT_PER_MINUTE
                } else {
                    configured
                }
            }
        });
        let delay_ms: u64 = if rate_limit > 0 {
            (60_000u64 / rate_limit as u64).saturating_add(100) // Add 100ms buffer for safety
        } else {
            2_000 // Fallback to 2 seconds if config disabled or invalid
        };

        // Always log this critical info
        logger::info(
            LogTag::Ohlcv,
            &format!(
                "OHLCV monitor starting: rate_limit={}/min, delay={}ms between tokens",
                rate_limit, delay_ms
            ),
        );

        loop {
            tick.tick().await;

            // Check shutdown signal
            if *self.shutdown_signal.read().await {
                break;
            }

            // Skip the cycle while the internet is confirmed offline — candle
            // fetches go to GeckoTerminal and would only time out. Resumes
            // automatically on reconnect; never triggers at startup.
            if crate::connectivity::is_network_offline() {
                continue;
            }

            // Process each active token
            let tokens: Vec<String> = {
                let active = self.active_tokens.read().await;
                active.keys().cloned().collect()
            };

            let processed_count = tokens.len();
            let cycle_start = Instant::now();
            self.record_monitor_cycle_start(processed_count).await;

            for mint in tokens {
                match self.process_token(&mint).await {
                    Ok(_) => {}
                    Err(OhlcvError::NotFound(_)) => {
                        logger::debug(
                            LogTag::Ohlcv,
                            &format!("Token {mint} disappeared during processing; skipping"),
                        );
                        record_ohlcv_event(
                            "token_missing",
                            Severity::Debug,
                            Some(mint.as_str()),
                            None,
                            crate::events::with_text(
                                json!({
                                  "action": "skip_cycle",
                                }),
                                &UiText::new(ids::EVENTS_OHLCV_TOKEN_MISSING)
                                    .arg("mint", UiArg::Text(mint.to_string())),
                            ),
                        )
                        .await;
                    }
                    Err(OhlcvError::PoolNotFound(_)) => {
                        logger::warning(
                            LogTag::Ohlcv,
                            &format!("No healthy pools available for {mint}; deferring"),
                        );
                        record_ohlcv_event(
                            "pool_unavailable",
                            Severity::Warn,
                            Some(mint.as_str()),
                            None,
                            crate::events::with_text(
                                json!({
                                  "reason": "pool_health",
                                }),
                                &UiText::new(ids::EVENTS_OHLCV_POOL_UNAVAILABLE)
                                    .arg("mint", UiArg::Text(mint.to_string())),
                            ),
                        )
                        .await;
                    }
                    Err(OhlcvError::RateLimitExceeded) => {
                        self.record_rate_limit_event().await;
                        record_ohlcv_event(
                            "rate_limit_hit",
                            Severity::Warn,
                            Some(mint.as_str()),
                            None,
                            crate::events::with_text(
                                json!({
                                  "rate_limit_per_minute": rate_limit,
                                  "delay_ms": delay_ms,
                                  "tokens_in_cycle": processed_count,
                                }),
                                &UiText::new(ids::EVENTS_OHLCV_RATE_LIMIT_HIT)
                                    .arg("mint", UiArg::Text(mint.to_string())),
                            ),
                        )
                        .await;
                        logger::warning(
                            LogTag::Ohlcv,
                            &format!(
                                "Rate limit hit while processing {}; backing off briefly",
                                mint
                            ),
                        );
                        sleep(Duration::from_secs(2)).await;
                    }
                    Err(e) => {
                        let (kind, severity) = classify_ohlcv_error(&e);
                        record_ohlcv_event(
                            "process_token_error",
                            severity,
                            Some(mint.as_str()),
                            None,
                            crate::events::with_text(
                                json!({
                                  "error_kind": kind,
                                }),
                                &UiText::new(ids::EVENTS_OHLCV_PROCESS_TOKEN_ERROR)
                                    .arg("mint", UiArg::Text(mint.to_string()))
                                    .arg("error", UiArg::Text(e.to_string())),
                            ),
                        )
                        .await;
                        logger::error(LogTag::Ohlcv, &format!("Error processing {mint}: {e}"));
                    }
                }

                // Rate-limit-aware delay between tokens
                sleep(Duration::from_millis(delay_ms)).await;
            }

            self.record_monitor_cycle_end(cycle_start, processed_count)
                .await;
        }
    }

    async fn process_token(&self, mint: &str) -> OhlcvResult<()> {
        let action = {
            let active = self.active_tokens.read().await;
            let config = active
                .get(mint)
                .ok_or_else(|| OhlcvError::NotFound(mint.to_string()))?;

            // Get recommended action based on priority and activity
            PriorityManager::get_recommended_action(config)
        };

        match action {
            crate::ohlcvs::priorities::RecommendedAction::FetchNow => {
                self.fetch_token_data(mint).await?;
            }
            crate::ohlcvs::priorities::RecommendedAction::Throttle(_duration) => {
                // Skip this cycle, will fetch on next check based on timing
            }
            crate::ohlcvs::priorities::RecommendedAction::Pause => {
                // Token is paused, skip
            }
        }

        Ok(())
    }

    async fn fetch_token_data(&self, mint: &str) -> OhlcvResult<()> {
        // Check if we should try pool discovery (using backoff logic)
        let (has_pools, should_retry_discovery) = {
            let active = self.active_tokens.read().await;
            let config = active
                .get(mint)
                .ok_or_else(|| OhlcvError::NotFound(mint.to_string()))?;
            let has = config.series_pool().is_some();
            let should_retry = !has && config.should_retry_pool_discovery();
            (has, should_retry)
        };

        // If no pools and backoff period has elapsed, try to discover them
        if !has_pools && should_retry_discovery {
            match self.pool_manager.discover_pools(mint).await {
                Ok(discovered) if !discovered.is_empty() => {
                    let first_pool_address = discovered.first().map(|p| p.address.clone());

                    // Success! Update config with discovered pools and reset failure counter
                    let updated_config = {
                        let mut active = self.active_tokens.write().await;
                        active.get_mut(mint).map(|config| {
                            let previous_failures = config.consecutive_pool_failures;
                            config.pools = discovered.clone();
                            config.mark_pool_discovery_success();
                            (previous_failures, config.clone())
                        })
                    };

                    if let Some((previous_failures, config)) = updated_config {
                        self.db.upsert_monitor_config(&config)?;

                        logger::debug(
                            LogTag::Ohlcv,
                            &format!(
                                "Pool discovery succeeded for {} after {} failures",
                                mint, previous_failures
                            ),
                        );

                        record_ohlcv_event(
                            "pool_discovery_success",
                            Severity::Info,
                            Some(mint),
                            first_pool_address.as_deref(),
                            crate::events::with_text(
                                json!({
                                  "discovered_pools": discovered,
                                  "previous_failures": previous_failures,
                                }),
                                &UiText::new(ids::EVENTS_OHLCV_POOL_DISCOVERY_SUCCESS)
                                    .arg("mint", UiArg::Text(mint.to_string())),
                            ),
                        )
                        .await;
                    }
                }
                Ok(_) | Err(_) => {
                    // Failed - update failure counter and backoff timestamp
                    let update_result = {
                        let mut active = self.active_tokens.write().await;
                        active.get_mut(mint).map(|config| {
                            let was_failure_count = config.consecutive_pool_failures;
                            config.mark_pool_discovery_failure();
                            let updated = config.clone();
                            (was_failure_count, updated)
                        })
                    };

                    if let Some((was_failure_count, config)) = update_result {
                        self.db.upsert_monitor_config(&config)?;

                        // Log with appropriate frequency (only first 3 attempts, then once at 5, 10, etc.)
                        let should_log = was_failure_count < 3 || was_failure_count % 5 == 0;

                        if should_log {
                            logger::warning(
                                LogTag::Ohlcv,
                                &format!(
                                    "Pool discovery failed for {} (attempt {}). Next retry: {}",
                                    mint,
                                    config.consecutive_pool_failures,
                                    config.get_next_retry_description()
                                ),
                            );
                        }

                        if should_log {
                            record_ohlcv_event(
                                "pool_discovery_failed",
                                Severity::Warn,
                                Some(mint),
                                None,
                                crate::events::with_text(
                                    json!({
                                      "attempt": config.consecutive_pool_failures,
                                      "next_retry": config.get_next_retry_description(),
                                    }),
                                    &UiText::new(ids::EVENTS_OHLCV_POOL_DISCOVERY_FAILED)
                                        .arg("mint", UiArg::Text(mint.to_string())),
                                ),
                            )
                            .await;
                        }
                    }

                    // Pool discovery failed — try the chain's candle feeds (no pool needed)
                    let feeds = self.fetcher.candle_feeds();
                    if !feeds.is_empty() {
                        return self.fetch_via_feeds(mint, &feeds).await;
                    }

                    return Err(OhlcvError::PoolNotFound(format!(
                        "No pools available for token: {}",
                        mint
                    )));
                }
            }
        } else if !has_pools {
            // In backoff period — try the chain's candle feeds (no pool needed)
            let feeds = self.fetcher.candle_feeds();
            if !feeds.is_empty() {
                return self.fetch_via_feeds(mint, &feeds).await;
            }
            return Err(OhlcvError::PoolNotFound(format!(
                "Token {} in discovery backoff period",
                mint
            )));
        }

        let priority = {
            let active = self.active_tokens.read().await;
            active
                .get(mint)
                .ok_or_else(|| OhlcvError::NotFound(mint.to_string()))?
                .priority
        };

        // Resolve the pool from the stored pool rows — the same source and rule the chart and
        // status read — so a default the pool manager moved after failures is the pool written.
        let pool = self
            .pool_manager
            .series_pool(mint)
            .await?
            .ok_or_else(|| OhlcvError::PoolNotFound(mint.to_string()))?;
        let (pool_address, pool_is_native) = (pool.address, pool.is_native_pair);

        // Fetch 1-minute data (base timeframe) with multi-source fallback, sized to
        // reach back to the newest stored 1m candle.
        let minute_newest =
            self.db
                .get_latest_native_timestamp(mint, &pool_address, Timeframe::Minute1)?;
        let batch_size = catch_up_limit(minute_newest, Utc::now().timestamp(), Timeframe::Minute1);
        let data = self
            .fetcher
            .fetch_multi_source(
                mint,
                &pool_address,
                "minute",
                1,
                batch_size,
                pool_is_native,
                None,
            )
            .await;
        let fetch_succeeded = data.is_ok();

        match data {
            Ok(response) => {
                let fetched_newest = newest_storable_bucket(&response.candles, Timeframe::Minute1);
                let now = Utc::now().timestamp();
                self.update_native_series(mint, Timeframe::Minute1, |state| {
                    state.record_page(
                        now,
                        minute_newest,
                        fetched_newest,
                        response.server_refreshing,
                    )
                });
                let data_points = response.candles;
                if data_points.is_empty() {
                    // Mark empty fetch
                    let updated_config = {
                        let mut active = self.active_tokens.write().await;
                        active.get_mut(mint).map(|config| {
                            config.mark_fetch();
                            config.mark_empty_fetch();
                            config.clone()
                        })
                    };

                    if let Some(config) = updated_config {
                        self.db.upsert_monitor_config(&config)?;
                    }

                    record_ohlcv_event(
                        "empty_fetch",
                        Severity::Debug,
                        Some(mint),
                        Some(pool_address.as_str()),
                        crate::events::with_text(
                            json!({
                              "batch_size": batch_size,
                              "priority": priority.to_string(),
                            }),
                            &UiText::new(ids::EVENTS_OHLCV_EMPTY_FETCH)
                                .arg("mint", UiArg::Text(mint.to_string()))
                                .arg("pool", UiArg::Text(pool_address.to_string())),
                        ),
                    )
                    .await;
                } else {
                    let stored_points = self.persist_chunk(mint, &pool_address, data_points)?;

                    // Mark success
                    self.pool_manager.mark_success(mint, &pool_address).await?;

                    let updated_config = {
                        let mut active = self.active_tokens.write().await;
                        active.get_mut(mint).map(|config| {
                            config.consecutive_empty_fetches = 0;
                            config.mark_fetch();
                            config.mark_activity();
                            config.clone()
                        })
                    };

                    if let Some(config) = updated_config {
                        self.db.upsert_monitor_config(&config)?;
                    }

                    // Ensure retention window remains populated (best-effort for gap coverage)
                    if let Err(e) = self.ensure_retention_window(mint, &pool_address).await {
                        logger::warning(
                            LogTag::Ohlcv,
                            &format!(
                                "Retention backfill failed for {} via {}: {}",
                                mint, pool_address, e
                            ),
                        );
                        record_ohlcv_event(
                            "retention_backfill_failed",
                            Severity::Warn,
                            Some(mint),
                            Some(pool_address.as_str()),
                            crate::events::with_text(
                                json!({
                                  "error": e.to_string(),
                                }),
                                &UiText::new(ids::EVENTS_OHLCV_RETENTION_BACKFILL_FAILED)
                                    .arg("mint", UiArg::Text(mint.to_string()))
                                    .arg("pool", UiArg::Text(pool_address.to_string())),
                            ),
                        )
                        .await;
                    }

                    if !stored_points.is_empty() {
                        match self.refresh_derived_timeframes_from_1m(mint, &pool_address) {
                            Ok(derived_points) if derived_points > 0 => {
                                logger::debug(
                                    LogTag::Ohlcv,
                                    &format!(
                                        "Derived {} higher-timeframe OHLCV points for {} via {}",
                                        derived_points, mint, pool_address
                                    ),
                                );
                            }
                            Ok(_) => {}
                            Err(e) => {
                                logger::warning(
                                    LogTag::Ohlcv,
                                    &format!(
                                        "Higher-timeframe derivation failed for {} via {}: {}",
                                        mint, pool_address, e
                                    ),
                                );
                            }
                        }
                    }

                    if !stored_points.is_empty() {
                        let earliest = stored_points.first().map(|p| p.timestamp);
                        let latest = stored_points.last().map(|p| p.timestamp);

                        record_ohlcv_event(
                            "fetch_success",
                            Severity::Info,
                            Some(mint),
                            Some(pool_address.as_str()),
                            crate::events::with_text(
                                json!({
                                  "inserted_points": stored_points.len(),
                                  "earliest_timestamp": earliest,
                                  "latest_timestamp": latest,
                                  "priority": priority.to_string(),
                                  "batch_size": batch_size,
                                }),
                                &UiText::new(ids::EVENTS_OHLCV_FETCH_SUCCESS)
                                    .arg("count", UiArg::Text(stored_points.len().to_string()))
                                    .arg("mint", UiArg::Text(mint.to_string())),
                            ),
                        )
                        .await;
                    }

                    // Detect and register gaps for recent data (best-effort)
                    if !stored_points.is_empty() {
                        if let Err(e) = self
                            .gap_manager
                            .detect_gaps(mint, &pool_address, Timeframe::Minute1)
                            .await
                        {
                            logger::warning(
                                LogTag::Ohlcv,
                                &format!(
                                    "Gap detection failed for {} via {}: {}",
                                    mint, pool_address, e
                                ),
                            );
                            record_ohlcv_event(
                                "gap_detection_failed",
                                Severity::Warn,
                                Some(mint),
                                Some(pool_address.as_str()),
                                crate::events::with_text(
                                    json!({
                                      "error": e.to_string(),
                                    }),
                                    &UiText::new(ids::EVENTS_OHLCV_GAP_DETECTION_FAILED)
                                        .arg("mint", UiArg::Text(mint.to_string()))
                                        .arg("pool", UiArg::Text(pool_address.to_string())),
                                ),
                            )
                            .await;
                        }
                    }
                }
            }
            Err(e) => {
                let now = Utc::now().timestamp();
                self.update_native_series(mint, Timeframe::Minute1, |state| {
                    state.record_failure(now)
                });
                if !matches!(e, OhlcvError::RateLimitExceeded) {
                    // Only penalize pool health for non-rate-limit failures
                    self.pool_manager.mark_failure(mint, &pool_address).await?;
                }

                let level = if matches!(e, OhlcvError::RateLimitExceeded) {
                    "WARN"
                } else {
                    "ERROR"
                };

                if !matches!(e, OhlcvError::RateLimitExceeded) {
                    let (kind, severity) = classify_ohlcv_error(&e);
                    record_ohlcv_event(
                        "fetch_failed",
                        severity,
                        Some(mint),
                        Some(pool_address.as_str()),
                        crate::events::with_text(
                            json!({
                              "error_kind": kind,
                              "batch_size": batch_size,
                              "priority": priority.to_string(),
                            }),
                            &UiText::new(ids::EVENTS_OHLCV_FETCH_FAILED)
                                .arg("mint", UiArg::Text(mint.to_string()))
                                .arg("pool", UiArg::Text(pool_address.to_string()))
                                .arg("error", UiArg::Text(e.to_string())),
                        ),
                    )
                    .await;
                }

                if level == "WARN" {
                    logger::warning(
                        LogTag::Ohlcv,
                        &format!(
                            "Failed to fetch {} using pool {}: {}",
                            mint, pool_address, e
                        ),
                    );
                } else {
                    logger::error(
                        LogTag::Ohlcv,
                        &format!(
                            "Failed to fetch {} using pool {}: {}",
                            mint, pool_address, e
                        ),
                    );
                }
            }
        }

        if fetch_succeeded {
            self.refresh_native_timeframes(mint, &pool_address, priority)
                .await;
        }

        Ok(())
    }

    /// Fetch 1m candles by token address from the chain's candle feeds, in order,
    /// when the token has no usable pool. The first feed that answers with
    /// candles is stored under its label, with the mint as the pool placeholder.
    /// A failing feed is recorded and the next one is tried; the last failure is
    /// returned when no feed answered.
    async fn fetch_via_feeds(&self, mint: &str, feeds: &[CandleFeed]) -> OhlcvResult<()> {
        let batch_size = {
            let active = self.active_tokens.read().await;
            let config = active
                .get(mint)
                .ok_or_else(|| OhlcvError::NotFound(mint.to_string()))?;
            PriorityManager::calculate_batch_size(config.priority)
        };

        let mut last_error = None;
        for feed in feeds {
            let data = self
                .fetcher
                .fetch_from_feed(feed, mint, Timeframe::Minute1, batch_size)
                .await;
            match data {
                Ok(candles) if !candles.is_empty() => {
                    return self.store_feed_candles(feed, mint, &candles).await;
                }
                Ok(_) => {}
                Err(e) => {
                    record_ohlcv_event(
                        &format!("{}_fetch_failed", feed.label),
                        Severity::Warn,
                        Some(mint),
                        None,
                        json!({
                            "error": e.to_string(),
                            "source": feed.label,
                        }),
                    )
                    .await;
                    last_error = Some(e);
                }
            }
        }

        if let Some(e) = last_error {
            return Err(e);
        }
        // Every feed answered empty
        let mut active = self.active_tokens.write().await;
        if let Some(config) = active.get_mut(mint) {
            config.mark_empty_fetch();
        }
        Ok(())
    }

    /// Store 1m candles a feed returned for a pool-less token and derive the
    /// coarser timeframes from them.
    async fn store_feed_candles(
        &self,
        feed: &CandleFeed,
        mint: &str,
        candles: &[Candle],
    ) -> OhlcvResult<()> {
        // Store in DB (use mint as pool placeholder since we don't have one)
        let stored_count = self.db.insert_candles_batch(
            mint,
            mint, // Use mint as pool_address for feed-sourced data
            Timeframe::Minute1,
            candles,
            feed.label,
        )?;

        if stored_count > 0 {
            if let Err(e) = self.refresh_derived_timeframes_from_1m(mint, mint) {
                logger::warning(
                    LogTag::Ohlcv,
                    &format!(
                        "Higher-timeframe derivation failed for {} via {}: {}",
                        mint, feed.label, e
                    ),
                );
            }

            // Partial batch fetch; the DB holds the full series.
            // Invalidate rather than cache the slice (see persist_chunk).
            self.cache
                .invalidate(mint, None, Some(Timeframe::Minute1))?;

            // Mark successful fetch
            {
                let mut active = self.active_tokens.write().await;
                if let Some(config) = active.get_mut(mint) {
                    config.mark_fetch();
                    config.mark_activity();
                }
            }

            record_ohlcv_event(
                &format!("{}_fetch_success", feed.label),
                Severity::Info,
                Some(mint),
                None,
                json!({
                    "source": feed.label,
                    "candles_fetched": candles.len(),
                    "candles_stored": stored_count,
                }),
            )
            .await;

            logger::info(
                LogTag::Ohlcv,
                &format!(
                    "{} OHLCV: {} candles for {} (no pool needed)",
                    feed.label,
                    stored_count,
                    &mint[..mint.len().min(12)]
                ),
            );
        }

        Ok(())
    }

    async fn record_monitor_cycle_start(&self, token_count: usize) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.monitor_cycle_started_at = Some(Utc::now());
        telemetry.monitor_cycle_tokens_processed = token_count;
    }

    async fn record_monitor_cycle_end(&self, start: Instant, token_count: usize) {
        let duration_ms = start.elapsed().as_millis() as u64;
        let mut telemetry = self.telemetry.write().await;
        telemetry.monitor_cycle_completed_at = Some(Utc::now());
        telemetry.monitor_cycle_duration_ms = Some(duration_ms);
        telemetry.monitor_cycle_tokens_processed = token_count;
        telemetry.monitor_cycle_total = telemetry.monitor_cycle_total.saturating_add(1);
    }

    async fn record_gap_cycle_start(&self, token_count: usize) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.gap_cycle_started_at = Some(Utc::now());
        telemetry.gap_cycle_tokens_processed = token_count;
    }

    async fn record_gap_cycle_end(&self, start: Instant, token_count: usize) {
        let duration_ms = start.elapsed().as_millis() as u64;
        let mut telemetry = self.telemetry.write().await;
        telemetry.gap_cycle_completed_at = Some(Utc::now());
        telemetry.gap_cycle_duration_ms = Some(duration_ms);
        telemetry.gap_cycle_tokens_processed = token_count;
        telemetry.gap_cycle_total = telemetry.gap_cycle_total.saturating_add(1);
    }

    async fn record_rate_limit_event(&self) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.last_rate_limit_at = Some(Utc::now());
        telemetry.rate_limit_events = telemetry.rate_limit_events.saturating_add(1);
    }

    async fn record_backfill_scheduled(&self) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.total_backfills_scheduled = telemetry.total_backfills_scheduled.saturating_add(1);
        telemetry.last_backfill_started_at = Some(Utc::now());
        telemetry.last_backfill_error = None;
    }

    async fn record_backfill_completed(&self, duration_ms: u64, points: usize) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.total_backfills_completed = telemetry.total_backfills_completed.saturating_add(1);
        telemetry.last_backfill_completed_at = Some(Utc::now());
        telemetry.last_backfill_duration_ms = Some(duration_ms);
        telemetry.last_backfill_points = Some(points);
        telemetry.last_backfill_error = None;
    }

    async fn record_backfill_failed(&self, duration_ms: u64, message: String) {
        let mut telemetry = self.telemetry.write().await;
        telemetry.total_backfills_failed = telemetry.total_backfills_failed.saturating_add(1);
        telemetry.last_backfill_completed_at = Some(Utc::now());
        telemetry.last_backfill_duration_ms = Some(duration_ms);
        telemetry.last_backfill_error = Some(message);
    }

    fn persist_chunk(
        &self,
        mint: &str,
        pool_address: &str,
        mut data_points: Vec<Candle>,
    ) -> OhlcvResult<Vec<Candle>> {
        if data_points.is_empty() {
            return Ok(Vec::new());
        }

        data_points.sort_by_key(|p| p.timestamp);
        data_points.dedup_by_key(|p| p.timestamp);

        // Store in unified candles table with timeframe=Minute1
        self.db.insert_candles_batch(
            mint,
            pool_address,
            Timeframe::Minute1,
            &data_points,
            "monitor",
        )?;

        // This chunk is only the freshly-fetched 1m window; the DB now holds the
        // merged full series. Invalidate rather than cache the partial slice so
        // the chart's full-history read repopulates the cache from the DB.
        // (Caching the window here made the chart show only the recent candles
        // while the status popup — which reads the DB — showed the full count.)
        self.cache
            .invalidate(mint, Some(pool_address), Some(Timeframe::Minute1))?;

        Ok(data_points)
    }

    /// Recompute the aggregated timeframes of the last two days from stored 1m.
    fn refresh_derived_timeframes_from_1m(
        &self,
        mint: &str,
        pool_address: &str,
    ) -> OhlcvResult<usize> {
        let now = Utc::now().timestamp();
        self.refresh_derived_timeframes_between(
            mint,
            pool_address,
            now - 2 * Timeframe::Day1.to_seconds(),
            now,
        )
    }

    /// Recompute every aggregated bucket overlapping `[from_ts, to_ts]` from the
    /// stored 1m rows. Written as `monitor_aggregate` with the start of the
    /// newest 1m row of each bucket, so closed native buckets keep their source
    /// values and a forming native bucket changes only when newer 1m data
    /// exists (see `OhlcvDatabase::upsert_aggregate_candles`).
    fn refresh_derived_timeframes_between(
        &self,
        mint: &str,
        pool_address: &str,
        from_ts: i64,
        to_ts: i64,
    ) -> OhlcvResult<usize> {
        let now = Utc::now().timestamp();
        let day = Timeframe::Day1.to_seconds();
        // Whole days: every aggregated timeframe divides a day, so each bucket is
        // recomputed from all of its stored 1m rows, never a slice of them.
        let from_ts = OhlcvDatabase::bucket_start(from_ts, Timeframe::Day1);
        let to_ts = (OhlcvDatabase::bucket_start(to_ts, Timeframe::Day1) + day - 1).min(now);
        let minute_candles = self.db.get_candles(
            mint,
            Some(pool_address),
            Timeframe::Minute1,
            Some(from_ts),
            Some(to_ts),
            None,
        )?;

        if minute_candles.is_empty() {
            return Ok(0);
        }

        let mut inserted_total = 0;

        for timeframe in AGGREGATED_TIMEFRAMES {
            let aggregated =
                OhlcvAggregator::aggregate(&minute_candles, Timeframe::Minute1, timeframe)?;

            if aggregated.is_empty() {
                continue;
            }

            let with_newest_minute = with_newest_minute(&minute_candles, aggregated, timeframe);
            let inserted = self.db.upsert_aggregate_candles(
                mint,
                pool_address,
                timeframe,
                &with_newest_minute,
                now,
            )?;

            inserted_total += inserted;
            // Only this window was aggregated here; the DB retains the full
            // series for this timeframe. Invalidate so the chart re-reads the
            // complete history instead of this short window (see persist_chunk).
            self.cache
                .invalidate(mint, Some(pool_address), Some(timeframe))?;
        }

        Ok(inserted_total)
    }

    fn is_timeframe_backfill_ready(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<bool> {
        let flag_complete = self.db.is_backfill_complete(mint, timeframe)?;
        let has_candles = self
            .db
            .get_time_bounds(mint, pool_address, timeframe)?
            .is_some();

        if flag_complete && !has_candles {
            self.db.mark_backfill_incomplete(mint, timeframe)?;
            logger::warning(
                LogTag::Ohlcv,
                &format!(
                    "Corrected stale backfill flag for mint={} timeframe={} (no candles stored)",
                    mint,
                    timeframe.as_str()
                ),
            );
        }

        Ok(flag_complete && has_candles)
    }

    fn are_all_timeframes_backfill_ready(
        &self,
        mint: &str,
        pool_address: &str,
    ) -> OhlcvResult<bool> {
        for timeframe in Timeframe::all() {
            if !self.is_timeframe_backfill_ready(mint, pool_address, timeframe)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn ensure_retention_window(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        let retention_days = with_config(|cfg| cfg.ohlcv.retention_days);
        if retention_days <= 0 {
            return Ok(());
        }

        if self.are_all_timeframes_backfill_ready(mint, pool_address)? {
            return Ok(());
        }

        // Get token priority for rate limiting
        let priority = {
            let active = self.active_tokens.read().await;
            active
                .get(mint)
                .map(|config| config.priority)
                .unwrap_or(Priority::Medium)
        };

        if !self.try_start_backfill(mint) {
            return Ok(());
        }

        self.record_backfill_scheduled().await;

        record_ohlcv_event(
            "backfill_scheduled",
            Severity::Info,
            Some(mint),
            Some(pool_address),
            crate::events::with_text(
                json!({
                  "retention_days": retention_days,
                  "timeframes": ["1d", "12h", "4h", "1h", "15m", "5m", "1m"],
                }),
                &UiText::new(ids::EVENTS_OHLCV_BACKFILL_SCHEDULED)
                    .arg("mint", UiArg::Text(mint.to_string()))
                    .arg("pool", UiArg::Text(pool_address.to_string())),
            ),
        )
        .await;

        let runner = self.clone();
        let mint_owned = mint.to_string();
        let pool_owned = pool_address.to_string();

        logger::info(
            LogTag::Ohlcv,
            &format!(
                "Scheduling multi-timeframe backfill for {} via {} (retention: {} days)",
                mint_owned, pool_owned, retention_days
            ),
        );

        tokio::spawn(async move {
            match runner
                .backfill_all_timeframes(&mint_owned, &pool_owned, priority)
                .await
            {
                Ok(points) => {
                    logger::info(
                        LogTag::Ohlcv,
                        &format!(
                            "Multi-timeframe backfill for {} via {} completed (total candles: {})",
                            mint_owned, pool_owned, points
                        ),
                    );
                }
                Err(e) => {
                    logger::warning(
                        LogTag::Ohlcv,
                        &format!(
                            "Multi-timeframe backfill failed for {} via {}: {}",
                            mint_owned, pool_owned, e
                        ),
                    );
                }
            }

            runner.finish_backfill(&mint_owned);
        });

        Ok(())
    }

    /// Kick off network pool discovery for `mint` in the background so callers
    /// (notably the chart endpoint) never block on the rate-limited provider
    /// fetch. Deduped per-mint via `discovery_in_progress` so the ~1s chart poll
    /// can't spawn a discovery every tick. Discovered pools are persisted by
    /// `discover_pools`, so the next poll/monitor cycle finds them and starts
    /// backfill.
    fn spawn_pool_discovery(&self, mint: &str) {
        if !self.try_start_discovery(mint) {
            return;
        }
        let runner = self.clone();
        let mint_owned = mint.to_string();
        tokio::spawn(async move {
            if let Err(e) = runner.pool_manager.discover_pools(&mint_owned).await {
                logger::warning(
                    LogTag::Ohlcv,
                    &format!("Warning: background pool discovery failed for {mint_owned}: {e}"),
                );
            }
            runner.finish_discovery(&mint_owned);
        });
    }

    fn try_start_discovery(&self, mint: &str) -> bool {
        match self.discovery_in_progress.lock() {
            Ok(mut set) => {
                if set.contains(mint) {
                    false
                } else {
                    set.insert(mint.to_string());
                    true
                }
            }
            Err(_) => false,
        }
    }

    fn finish_discovery(&self, mint: &str) {
        if let Ok(mut set) = self.discovery_in_progress.lock() {
            set.remove(mint);
        }
    }

    fn try_start_backfill(&self, mint: &str) -> bool {
        match self.backfill_in_progress.lock() {
            Ok(mut set) => {
                if set.contains(mint) {
                    false
                } else {
                    set.insert(mint.to_string());
                    true
                }
            }
            Err(_) => false,
        }
    }

    fn finish_backfill(&self, mint: &str) {
        if let Ok(mut set) = self.backfill_in_progress.lock() {
            set.remove(mint);
        }
    }

    /// Every five minutes, fill due gap spans of the active tokens within the
    /// retention window: sequential, throttled, and capped at
    /// `GAP_FILL_REQUESTS_PER_CYCLE` requests per scan.
    async fn gap_fill_loop(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(300));

        loop {
            tick.tick().await;

            if *self.shutdown_signal.read().await {
                break;
            }

            let tokens: Vec<(String, Priority)> = {
                let active = self.active_tokens.read().await;
                active
                    .iter()
                    .map(|(mint, config)| (mint.clone(), config.priority))
                    .collect()
            };

            let cycle_start = Instant::now();
            self.record_gap_cycle_start(tokens.len()).await;

            let mut budget = GAP_FILL_REQUESTS_PER_CYCLE;
            let mut processed = 0;
            for (mint, priority) in tokens {
                if budget == 0 {
                    break;
                }
                processed += 1;
                match self.fill_token_gaps(&mint, priority, &mut budget).await {
                    Ok(()) => {}
                    Err(e) => {
                        logger::error(LogTag::Ohlcv, &format!("Gap fill error for {mint}: {e}"));
                        record_ohlcv_event(
                            "gap_fill_failed",
                            Severity::Error,
                            Some(mint.as_str()),
                            None,
                            crate::events::with_text(
                                json!({
                                  "error": e.to_string(),
                                }),
                                &UiText::new(ids::EVENTS_OHLCV_GAP_FILL_FAILED)
                                    .arg("mint", UiArg::Text(mint.to_string())),
                            ),
                        )
                        .await;
                        if matches!(e, OhlcvError::RateLimitExceeded) {
                            self.record_rate_limit_event().await;
                            break;
                        }
                    }
                }
            }

            self.record_gap_cycle_end(cycle_start, processed).await;
        }
    }

    /// Fill the due gap spans of a token's series pool, spending one unit of
    /// `budget` per request. Holds the token's backfill slot so gap filling and
    /// backfill never fetch the same token concurrently. Aggregated buckets
    /// over the 1m rows it wrote are recomputed afterwards.
    async fn fill_token_gaps(
        &self,
        mint: &str,
        priority: Priority,
        budget: &mut usize,
    ) -> OhlcvResult<()> {
        let Some(pool) = self.pool_manager.series_pool(mint).await? else {
            return Ok(());
        };
        let spans =
            self.gap_manager
                .due_spans(mint, &pool.address, Utc::now().timestamp(), *budget)?;
        if spans.is_empty() || !self.try_start_backfill(mint) {
            return Ok(());
        }

        let mut minute_range: Option<(i64, i64)> = None;
        let mut result = Ok(());
        for span in spans {
            if *budget == 0 {
                break;
            }
            *budget -= 1;
            sleep(inter_fetch_delay(priority)).await;
            match self
                .gap_manager
                .fill_span(mint, &pool.address, pool.is_native_pair, &span)
                .await
            {
                Ok(fill) => {
                    if span.timeframe == Timeframe::Minute1 {
                        if let Some((lo, hi)) = fill.written_range {
                            minute_range = Some(match minute_range {
                                Some((from, to)) => (from.min(lo), to.max(hi)),
                                None => (lo, hi),
                            });
                        }
                    }
                }
                Err(OhlcvError::RateLimitExceeded) => {
                    result = Err(OhlcvError::RateLimitExceeded);
                    break;
                }
                Err(e) => {
                    logger::warning(
                        LogTag::Ohlcv,
                        &format!(
                            "Gap span fill failed for mint={} timeframe={} span={}..{}: {}",
                            mint,
                            span.timeframe.as_str(),
                            span.start,
                            span.end,
                            e
                        ),
                    );
                }
            }
        }
        self.finish_backfill(mint);

        if let Some((from, to)) = minute_range {
            if let Err(e) = self.refresh_derived_timeframes_between(mint, &pool.address, from, to) {
                logger::warning(
                    LogTag::Ohlcv,
                    &format!(
                        "Higher-timeframe derivation after gap fill failed for {} via {}: {}",
                        mint, pool.address, e
                    ),
                );
            }
        }

        result
    }

    async fn sync_pool_service_tokens(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(30)); // Every 30 seconds (Pool Service updates every 5-10s)

        loop {
            tick.tick().await;

            if *self.shutdown_signal.read().await {
                break;
            }

            // Respect configured maximum monitored tokens (0 = unlimited for backward compatibility)
            let configured_limit = with_config(|cfg| cfg.ohlcv.max_monitored_tokens);
            let max_tokens = if configured_limit == 0 {
                usize::MAX
            } else {
                configured_limit
            };

            // Get all tokens with available prices from Pool Service (same list Trader monitors).
            let chain = self.db.chain();
            let available_mints: Vec<String> = crate::pools::get_available_tokens(chain);

            // Open positions raise priority only when the position store holds this
            // monitor's chain.
            let open_positions = match crate::positions::db::get_store_chain().await {
                Ok(store_chain) if store_chain == chain => {
                    crate::positions::state::get_open_positions()
                        .await
                        .into_iter()
                        .map(|p| p.mint)
                        .collect::<std::collections::HashSet<_>>()
                }
                Ok(_) => std::collections::HashSet::new(),
                Err(e) => {
                    logger::debug(
                        LogTag::Ohlcv,
                        &format!("Open positions unavailable for {chain} pool sync: {e}"),
                    );
                    std::collections::HashSet::new()
                }
            };

            let mut added = 0;
            let mut upgraded = 0;
            let mut already_monitored = 0;

            // Add or upgrade tokens from Pool Service
            for mint in &available_mints {
                let active_tokens = self.active_tokens.read().await;

                if let Some(config) = active_tokens.get(mint) {
                    // Token already monitored
                    already_monitored += 1;

                    // Upgrade to Critical if has open position
                    if open_positions.contains(mint) && config.priority != Priority::Critical {
                        drop(active_tokens);
                        if let Err(e) = self.update_priority(mint, Priority::Critical).await {
                            logger::error(
                                LogTag::Ohlcv,
                                &format!("Failed to upgrade priority for {mint}: {e}"),
                            );
                        } else {
                            upgraded += 1;
                        }
                    }
                } else {
                    // New token - add with appropriate priority
                    drop(active_tokens);

                    let is_open_position = open_positions.contains(mint);

                    if !is_open_position && max_tokens != usize::MAX {
                        let current_len = {
                            let snapshot = self.active_tokens.read().await;
                            snapshot.len()
                        };

                        if current_len >= max_tokens {
                            continue;
                        }
                    }

                    let priority = if is_open_position {
                        Priority::Critical
                    } else {
                        Priority::Low
                    };

                    if let Err(e) = self.add_token(mint.clone(), priority).await {
                        logger::error(LogTag::Ohlcv, &format!("Failed to add token {mint}: {e}"));
                    } else {
                        added += 1;
                    }
                }
            }

            // Remove tokens no longer in Pool Service — but KEEP anything the user
            // is actively interested in, so charts a user opens/trades stay ready
            // (DexScreener-style) instead of being dropped the moment the trader's
            // watchlist rotates. A token survives if it has an open position, is
            // still in Pool Service, is High/Critical priority (viewed/requested/
            // traded — see PriorityManager::update_priority_on_activity), or was
            // active within the cache-retention grace window.
            let available_set: std::collections::HashSet<_> =
                available_mints.iter().cloned().collect();
            let grace_hours = with_config(|cfg| cfg.ohlcv.cache_retention_hours).max(0);
            let activity_cutoff = Utc::now() - chrono::Duration::hours(grace_hours);
            let mut removed = 0;

            let active_tokens = self.active_tokens.read().await;
            let tokens_to_check: Vec<(String, Priority, DateTime<Utc>)> = active_tokens
                .iter()
                .map(|(mint, cfg)| (mint.clone(), cfg.priority, cfg.last_activity))
                .collect();
            drop(active_tokens);

            for (mint, priority, last_activity) in tokens_to_check {
                // Keep: open position, still in Pool Service, user-interest
                // priority, or recently active within the grace window.
                if open_positions.contains(&mint)
                    || available_set.contains(&mint)
                    || matches!(priority, Priority::Critical | Priority::High)
                    || last_activity >= activity_cutoff
                {
                    continue;
                }

                // Remove stale token
                if let Err(e) = self.remove_token(&mint).await {
                    logger::error(
                        LogTag::Ohlcv,
                        &format!("Failed to remove token {mint}: {e}"),
                    );
                } else {
                    removed += 1;
                }
            }

            let mut trimmed = 0;
            if max_tokens != usize::MAX {
                let tokens_to_trim = self
                    .determine_tokens_to_trim(max_tokens, &open_positions)
                    .await;

                for mint in tokens_to_trim {
                    if let Err(e) = self.remove_token(&mint).await {
                        logger::error(LogTag::Ohlcv, &format!("Failed to trim token {mint}: {e}"));
                    } else {
                        trimmed += 1;
                    }
                }
            }

            if added > 0 || upgraded > 0 || removed > 0 || trimmed > 0 {
                logger::debug(
          LogTag::Ohlcv,
          &format!(
            "Pool Service sync: {} available, {} added, {} upgraded, {} removed, {} trimmed, {} already monitored",
            available_mints.len(),
            added,
            upgraded,
            removed,
            trimmed,
            already_monitored
          ),
        );
            }
        }
    }

    async fn determine_tokens_to_trim(
        &self,
        max_tokens: usize,
        open_positions: &std::collections::HashSet<String>,
    ) -> Vec<String> {
        if max_tokens == usize::MAX {
            return Vec::new();
        }

        let active = self.active_tokens.read().await;
        let active_len = active.len();

        if active_len <= max_tokens {
            return Vec::new();
        }

        let mut candidates: Vec<(String, Priority, chrono::DateTime<Utc>, u32)> = active
            .iter()
            .filter(|(mint, _)| !open_positions.contains(*mint))
            .map(|(mint, config)| {
                (
                    mint.clone(),
                    config.priority,
                    config.last_activity,
                    config.consecutive_empty_fetches,
                )
            })
            .collect();

        let mut current_size = active_len;
        drop(active);

        candidates.sort_by(|a, b| {
            a.1.cmp(&b.1)
                .then_with(|| a.2.cmp(&b.2))
                .then_with(|| b.3.cmp(&a.3))
        });

        let mut to_remove = Vec::new();
        for (mint, _, _, _) in candidates {
            if current_size <= max_tokens {
                break;
            }
            to_remove.push(mint);
            current_size -= 1;
        }

        to_remove
    }

    // ==================== Multi-Timeframe Backfill ====================

    /// Backfill all timeframes for a token as deep as the source provides.
    /// Fetches timeframes in priority order (1d → 12h → 4h → 1h → 15m → 5m → 1m);
    /// each call requests `max_backfill_candles` (server returns all it holds).
    async fn backfill_all_timeframes(
        &self,
        mint: &str,
        pool_address: &str,
        priority: Priority,
    ) -> OhlcvResult<usize> {
        let mut total_fetched = 0;

        // FINE-FIRST order (1m→1d). Backfill is sequential + throttled on purpose:
        // fetch_multi_source falls back to GeckoTerminal when the shared data
        // server misses (new/illiquid tokens), and Gecko 429s on a concurrent
        // burst — so we must NOT parallelize. But the old order was coarse-first
        // (1d first, 1m last), so the sub-4h frames the user is actually looking
        // at (the chart defaults to a fine timeframe) appeared LAST, ~8-10s in,
        // and looked "not loading" while 4h/12h/1d were already there. Fetching
        // fine-first makes 1m/5m/15m/1h land in the first few seconds; the coarse
        // frames (few candles, one quick fetch each) fill right after.
        let timeframes = [
            Timeframe::Minute1,
            Timeframe::Minute5,
            Timeframe::Minute15,
            Timeframe::Hour1,
            Timeframe::Hour4,
            Timeframe::Hour12,
            Timeframe::Day1,
        ];

        logger::debug(
            LogTag::Ohlcv,
            &format!(
                "Starting full-depth backfill for mint={} pool={} priority={:?}",
                mint, pool_address, priority
            ),
        );

        for timeframe in timeframes {
            // A series move during the backfill resets the token onto another pool; the
            // backfill of this pool stops instead of fetching rows the write guard refuses.
            let still_series = self
                .pool_manager
                .series_pool(mint)
                .await?
                .is_some_and(|pool| pool.address == pool_address);
            if !still_series {
                logger::debug(
                    LogTag::Ohlcv,
                    &format!(
                        "Stopping backfill for mint={} pool={}: no longer the series pool",
                        mint, pool_address
                    ),
                );
                return Ok(total_fetched);
            }

            // Check if already complete and backed by stored candles.
            if self.is_timeframe_backfill_ready(mint, pool_address, timeframe)? {
                logger::debug(
                    LogTag::Ohlcv,
                    &format!(
                        "Skipping {} backfill for mint={} (already complete)",
                        timeframe.as_str(),
                        mint
                    ),
                );
                continue;
            }

            // The first native page of any series is full size (`catch_up_limit`
            // of an empty series is the backfill size), so stored native candles
            // mean the depth is fetched and only coverage decides completion.
            let (newest, caught_up) = self.native_coverage(mint, pool_address, timeframe)?;
            if newest.is_some() && caught_up {
                self.db
                    .mark_backfill_complete(mint, pool_address, timeframe)?;
                continue;
            }

            // An incomplete timeframe is re-requested at most once per retry delay,
            // which is also the spacing the no-newer rule needs.
            let last_fetch_at = self.native_series_state(mint, timeframe).last_fetch_at;
            if last_fetch_at.is_some_and(|at| Utc::now().timestamp() - at < NATIVE_RETRY_DELAY_SECS)
            {
                continue;
            }

            match self
                .fetch_native_timeframe(
                    mint,
                    pool_address,
                    timeframe,
                    timeframe.max_backfill_candles(),
                )
                .await
            {
                Ok(count) => {
                    total_fetched += count;
                    let (newest, caught_up) =
                        self.native_coverage(mint, pool_address, timeframe)?;
                    if newest.is_some() && caught_up {
                        self.db
                            .mark_backfill_complete(mint, pool_address, timeframe)?;
                        logger::debug(
                            LogTag::Ohlcv,
                            &format!(
                                "Backfill complete for mint={} timeframe={} candles={}",
                                mint,
                                timeframe.as_str(),
                                count
                            ),
                        );
                    } else if newest.is_none() {
                        self.db.mark_backfill_incomplete(mint, timeframe)?;
                        logger::warning(
                            LogTag::Ohlcv,
                            &format!(
                                "Backfill produced no candles for mint={} timeframe={}; leaving incomplete",
                                mint,
                                timeframe.as_str()
                            ),
                        );
                    } else {
                        logger::debug(
                            LogTag::Ohlcv,
                            &format!(
                                "Backfill page for mint={} timeframe={} is not caught up (newest={:?}); leaving incomplete",
                                mint,
                                timeframe.as_str(),
                                newest
                            ),
                        );
                    }
                }
                Err(e @ OhlcvError::SeriesPoolMoved { .. }) => return Err(e),
                Err(e) => {
                    self.db.mark_backfill_incomplete(mint, timeframe)?;
                    logger::warning(
                        LogTag::Ohlcv,
                        &format!(
                            "Backfill failed for mint={} timeframe={}: {}",
                            mint,
                            timeframe.as_str(),
                            e
                        ),
                    );
                    // Continue to next timeframe
                }
            }

            // Rate limiting based on priority (prevents Gecko 429 on the fallback
            // path when the shared server misses).
            sleep(inter_fetch_delay(priority)).await;
        }

        if self.are_all_timeframes_backfill_ready(mint, pool_address)? {
            self.db.mark_all_backfills_complete(mint, pool_address)?;
        }

        logger::debug(
            LogTag::Ohlcv,
            &format!(
                "Completed full-depth backfill for mint={} pool={} total_candles={}",
                mint, pool_address, total_fetched
            ),
        );

        Ok(total_fetched)
    }

    /// Refresh each coarse timeframe natively when it is due (see
    /// `native_refresh_due`, `fallback_refresh_due` and `settle_refresh_due`),
    /// most overdue first (`order_by_overdue`) and sized by `catch_up_limit`.
    /// Sequential and throttled like backfill, it stops at the first rate limit,
    /// and it holds the token's backfill slot so the two never fetch the same
    /// token concurrently. The local 1m aggregation only fills the
    /// live edge; this is what makes closed coarse buckets match the source and
    /// heals a series the app missed while it was not running.
    async fn refresh_native_timeframes(&self, mint: &str, pool_address: &str, priority: Priority) {
        if !self.try_start_backfill(mint) {
            return;
        }

        let data_server_usable = data_server_usable();
        let now = Utc::now().timestamp();
        let mut due = Vec::with_capacity(AGGREGATED_TIMEFRAMES.len());
        for timeframe in AGGREGATED_TIMEFRAMES {
            let newest = match self
                .db
                .get_latest_native_timestamp(mint, pool_address, timeframe)
            {
                Ok(newest) => newest,
                Err(e) => {
                    logger::warning(
                        LogTag::Ohlcv,
                        &format!(
                            "Native refresh skipped for mint={} timeframe={}: {}",
                            mint,
                            timeframe.as_str(),
                            e
                        ),
                    );
                    continue;
                }
            };
            let state = self.native_series_state(mint, timeframe);
            if native_refresh_due(&state, newest, now, timeframe, priority)
                || fallback_refresh_due(&state, now, data_server_usable)
                || self.settle_due(mint, pool_address, timeframe, &state, newest, now)
            {
                due.push(DueTimeframe {
                    timeframe,
                    newest,
                    last_fetch_at: state.last_fetch_at,
                });
            }
        }
        order_by_overdue(&mut due, now, priority);

        for DueTimeframe {
            timeframe, newest, ..
        } in due
        {
            let now = Utc::now().timestamp();
            let limit = catch_up_limit(newest, now, timeframe);
            sleep(inter_fetch_delay(priority)).await;
            match self
                .fetch_native_timeframe(mint, pool_address, timeframe, limit)
                .await
            {
                Ok(changed) => {
                    logger::debug(
                        LogTag::Ohlcv,
                        &format!(
                            "Native refresh mint={} timeframe={} limit={} changed={}",
                            mint,
                            timeframe.as_str(),
                            limit,
                            changed
                        ),
                    );
                }
                Err(OhlcvError::RateLimitExceeded) => {
                    logger::warning(
                        LogTag::Ohlcv,
                        &format!(
                            "Native refresh for mint={} stopped at timeframe={}: rate limit exceeded",
                            mint,
                            timeframe.as_str()
                        ),
                    );
                    break;
                }
                Err(e) => {
                    logger::warning(
                        LogTag::Ohlcv,
                        &format!(
                            "Native refresh failed for mint={} timeframe={}: {}",
                            mint,
                            timeframe.as_str(),
                            e
                        ),
                    );
                }
            }
        }

        self.finish_backfill(mint);
    }

    /// Fetch the newest `limit` native candles of one timeframe, store them under
    /// `OhlcvDatabase::NATIVE_SOURCE`, drop the timeframe's hot-cache entry and
    /// record the page in the series coverage state. Returns rows inserted or
    /// changed.
    async fn fetch_native_timeframe(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        limit: usize,
    ) -> OhlcvResult<usize> {
        let (api_endpoint, aggregate) = timeframe.to_api_params();

        logger::debug(
            LogTag::Ohlcv,
            &format!(
                "Fetching timeframe={} mint={} endpoint={} aggregate={} limit={}",
                timeframe.as_str(),
                mint,
                api_endpoint,
                aggregate,
                limit
            ),
        );

        let pool_is_native = self.pool_is_native(mint, pool_address);

        let stored_newest = self
            .db
            .get_latest_native_timestamp(mint, pool_address, timeframe)?;

        // Fetch using multi-source fallback
        let response = match self
            .fetcher
            .fetch_multi_source(
                mint,
                pool_address,
                api_endpoint,
                aggregate,
                limit,
                pool_is_native,
                None,
            )
            .await
        {
            Ok(response) => response,
            Err(e) => {
                let now = Utc::now().timestamp();
                self.update_native_series(mint, timeframe, |state| state.record_failure(now));
                return Err(e);
            }
        };

        let changed = if response.candles.is_empty() {
            0
        } else {
            let changed = self.db.insert_candles_batch(
                mint,
                pool_address,
                timeframe,
                &response.candles,
                OhlcvDatabase::NATIVE_SOURCE,
            )?;
            // A fetch returns only one window; the DB now holds the merged full
            // series. Invalidate so the chart reads the complete history rather
            // than this single page (see persist_chunk).
            self.cache
                .invalidate(mint, Some(pool_address), Some(timeframe))?;
            changed
        };

        let fetched_newest = newest_storable_bucket(&response.candles, timeframe);
        let now = Utc::now().timestamp();
        self.update_native_series(mint, timeframe, |state| {
            state.record_page(
                now,
                stored_newest,
                fetched_newest,
                response.server_refreshing,
            );
            state.record_source(response.source, fetched_newest.is_some());
            state.record_settled(now, timeframe);
        });

        Ok(changed)
    }

    /// Whether a timeframe is due to settle its newest closed bucket (see
    /// `settle_refresh_due`). Reads the bucket's row only when the in-memory state
    /// leaves a settle refresh possible.
    fn settle_due(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        state: &NativeSeriesState,
        newest: Option<i64>,
        now: i64,
    ) -> bool {
        let Some(target) = settle_candidate(state, newest, now, timeframe) else {
            return false;
        };
        match self
            .db
            .get_stored_bucket(mint, pool_address, timeframe, target)
        {
            Ok(row) => settle_refresh_due(state, newest, row, now, timeframe),
            Err(e) => {
                logger::warning(
                    LogTag::Ohlcv,
                    &format!(
                        "Settle check skipped for mint={} timeframe={} bucket={}: {}",
                        mint,
                        timeframe.as_str(),
                        target,
                        e
                    ),
                );
                false
            }
        }
    }

    /// The pool's denomination, so a non-native pool is served by the data server only
    /// (see `fetch_multi_source`). See `registered_pool_is_native`.
    fn pool_is_native(&self, mint: &str, pool_address: &str) -> bool {
        self.db
            .get_pools(mint)
            .map(|pools| registered_pool_is_native(&pools, pool_address))
            .unwrap_or(false)
    }

    /// Newest stored native bucket of a series and whether it is caught up.
    fn native_coverage(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<(Option<i64>, bool)> {
        let newest = self
            .db
            .get_latest_native_timestamp(mint, pool_address, timeframe)?;
        let caught_up = self.native_series_state(mint, timeframe).is_caught_up(
            newest,
            Utc::now().timestamp(),
            timeframe,
        );
        Ok((newest, caught_up))
    }

    fn native_series_state(&self, mint: &str, timeframe: Timeframe) -> NativeSeriesState {
        self.native_series
            .lock()
            .ok()
            .and_then(|series| series.get(&(mint.to_string(), timeframe)).cloned())
            .unwrap_or_default()
    }

    fn update_native_series(
        &self,
        mint: &str,
        timeframe: Timeframe,
        update: impl FnOnce(&mut NativeSeriesState),
    ) {
        if let Ok(mut series) = self.native_series.lock() {
            update(series.entry((mint.to_string(), timeframe)).or_default());
        }
    }

    async fn cleanup_loop(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(3600)); // Every hour

        loop {
            tick.tick().await;

            if *self.shutdown_signal.read().await {
                break;
            }

            let retention_days = with_config(|cfg| cfg.ohlcv.retention_days);

            // Note: OHLCV candle data is preserved forever for historical analysis.
            // Only cleanup filled gap tracking records to manage database size.
            // Unfilled gaps are kept for retry purposes.
            if retention_days > 0 {
                match self.db.cleanup_filled_gaps(retention_days) {
                    Ok(deleted) => {
                        if deleted > 0 {
                            logger::debug(
                                LogTag::Ohlcv,
                                &format!(
                  "Cleaned up {} filled gap records older than {} days (candle data preserved)",
                  deleted, retention_days
                ),
                            );
                        }
                    }
                    Err(e) => {
                        logger::error(LogTag::Ohlcv, &format!("Gap cleanup error: {e}"));
                        record_ohlcv_event(
                            "gap_cleanup_failed",
                            Severity::Error,
                            None,
                            None,
                            crate::events::with_text(
                                json!({
                                  "error": e.to_string(),
                                  "retention_days": retention_days,
                                }),
                                &UiText::new(ids::EVENTS_OHLCV_GAP_CLEANUP_FAILED),
                            ),
                        )
                        .await;
                    }
                }
            }
        }
    }

    async fn cache_maintenance_loop(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(600)); // Every 10 minutes

        loop {
            tick.tick().await;

            if *self.shutdown_signal.read().await {
                break;
            }

            // Clean up expired cache entries
            if let Err(e) = self.cache.cleanup_expired() {
                logger::error(LogTag::Ohlcv, &format!("Cache cleanup error: {e}"));
                record_ohlcv_event(
                    "cache_cleanup_failed",
                    Severity::Error,
                    None,
                    None,
                    crate::events::with_text(
                        json!({
                          "error": e.to_string(),
                        }),
                        &UiText::new(ids::EVENTS_OHLCV_CACHE_CLEANUP_FAILED),
                    ),
                )
                .await;
            }
        }
    }
}

impl Clone for OhlcvMonitor {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            fetcher: Arc::clone(&self.fetcher),
            cache: Arc::clone(&self.cache),
            pool_manager: Arc::clone(&self.pool_manager),
            gap_manager: Arc::clone(&self.gap_manager),
            active_tokens: Arc::clone(&self.active_tokens),
            shutdown_signal: Arc::clone(&self.shutdown_signal),
            backfill_in_progress: Arc::clone(&self.backfill_in_progress),
            discovery_in_progress: Arc::clone(&self.discovery_in_progress),
            native_series: Arc::clone(&self.native_series),
            telemetry: Arc::clone(&self.telemetry),
        }
    }
}

fn classify_ohlcv_error(error: &OhlcvError) -> (&'static str, Severity) {
    match error {
        OhlcvError::DatabaseError(_) => ("database_error", Severity::Error),
        OhlcvError::ApiError(_) => ("api_error", Severity::Error),
        OhlcvError::RateLimitExceeded => ("rate_limit", Severity::Warn),
        OhlcvError::PoolNotFound(_) => ("pool_not_found", Severity::Warn),
        OhlcvError::InvalidTimeframe(_) => ("invalid_timeframe", Severity::Error),
        OhlcvError::DataGap { .. } => ("data_gap", Severity::Warn),
        OhlcvError::CacheError(_) => ("cache_error", Severity::Error),
        OhlcvError::NotFound(_) => ("not_found", Severity::Warn),
        OhlcvError::Chain(_) => ("chain_error", Severity::Error),
        OhlcvError::SeriesPoolMoved { .. } => ("series_pool_moved", Severity::Debug),
    }
}

fn count_by_priority(configs: &HashMap<String, TokenOhlcvConfig>) -> HashMap<Priority, usize> {
    let mut counts = HashMap::new();
    for config in configs.values() {
        *counts.entry(config.priority).or_default() += 1;
    }
    counts
}

/// Whether `pool_address` is a registered native-quoted pool. An unknown pool or quote is not
/// native, so it never reaches a source that answers in the pool's quote token.
fn registered_pool_is_native(pools: &[PoolConfig], pool_address: &str) -> bool {
    pools
        .iter()
        .find(|p| p.address == pool_address)
        .is_some_and(|p| p.is_native_pair)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 3_600;
    /// 30 minutes into an hour bucket.
    const NOW: i64 = 1_700_000_000 - 1_700_000_000 % HOUR + 1_800;
    const CURRENT: i64 = NOW - NOW % HOUR;

    fn candle(ts: i64, volume: f64) -> Candle {
        Candle::new(ts, 1.0, 1.0, 1.0, 1.0, volume)
    }

    #[test]
    fn catch_up_limit_covers_the_lag_plus_margin_within_bounds() {
        // Empty series: the backfill size.
        assert_eq!(catch_up_limit(None, NOW, Timeframe::Hour1), 1000);
        // Newest is the forming bucket or the one before it: the minimum.
        assert_eq!(catch_up_limit(Some(CURRENT), NOW, Timeframe::Hour1), 3);
        assert_eq!(
            catch_up_limit(Some(CURRENT - HOUR), NOW, Timeframe::Hour1),
            3
        );
        // A 21-bucket hole: 21 + 2.
        assert_eq!(
            catch_up_limit(Some(CURRENT - 21 * HOUR), NOW, Timeframe::Hour1),
            23
        );
        // A mid-bucket stored timestamp counts from its bucket start.
        assert_eq!(
            catch_up_limit(Some(CURRENT - 10 * HOUR + 59), NOW, Timeframe::Hour1),
            12
        );
        // Longer than one request can carry: the request cap.
        assert_eq!(
            catch_up_limit(Some(CURRENT - 5_000 * 60), NOW, Timeframe::Minute1),
            MAX_CANDLES_PER_REQUEST
        );
        // A clock behind the stored newest never asks for fewer than the minimum.
        assert_eq!(catch_up_limit(Some(NOW + HOUR), NOW, Timeframe::Hour1), 3);
    }

    #[test]
    fn refresh_interval_scales_by_priority_and_never_drops_below_base() {
        assert_eq!(
            native_refresh_interval_secs(Timeframe::Minute5, Priority::Critical),
            300
        );
        assert_eq!(
            native_refresh_interval_secs(Timeframe::Minute5, Priority::High),
            300
        );
        assert_eq!(
            native_refresh_interval_secs(Timeframe::Minute15, Priority::High),
            900
        );
        assert_eq!(
            native_refresh_interval_secs(Timeframe::Hour1, Priority::High),
            1_800
        );
        for tf in [Timeframe::Hour4, Timeframe::Hour12, Timeframe::Day1] {
            assert_eq!(native_refresh_interval_secs(tf, Priority::High), 3_600);
        }
        assert_eq!(
            native_refresh_interval_secs(Timeframe::Hour1, Priority::Medium),
            5 * 1_800
        );
        assert_eq!(
            native_refresh_interval_secs(Timeframe::Hour1, Priority::Low),
            15 * 1_800
        );
    }

    #[test]
    fn refresh_is_due_on_interval_catch_up_or_scheduled_refetch() {
        let tf = Timeframe::Hour1;
        let caught_up = Some(CURRENT - HOUR);
        let behind = Some(CURRENT - 3 * HOUR);

        // Never fetched since start: always due, even when the stored newest
        // looks caught up, because that bucket may have been stored while forming.
        let fresh = NativeSeriesState::default();
        for priority in [Priority::Critical, Priority::Low] {
            assert!(native_refresh_due(&fresh, caught_up, NOW, tf, priority));
            assert!(native_refresh_due(&fresh, Some(CURRENT), NOW, tf, priority));
            assert!(native_refresh_due(&fresh, behind, NOW, tf, priority));
            assert!(native_refresh_due(&fresh, None, NOW, tf, priority));
        }

        let fetched = |ago: i64| NativeSeriesState {
            last_fetch_at: Some(NOW - ago),
            ..NativeSeriesState::default()
        };
        // Caught up: only the priority-scaled interval makes it due.
        assert!(!native_refresh_due(
            &fetched(1_799),
            caught_up,
            NOW,
            tf,
            Priority::High
        ));
        assert!(native_refresh_due(
            &fetched(1_800),
            caught_up,
            NOW,
            tf,
            Priority::High
        ));
        assert!(!native_refresh_due(
            &fetched(1_800),
            caught_up,
            NOW,
            tf,
            Priority::Low
        ));
        // Behind: due once the retry delay has passed since the last attempt.
        assert!(!native_refresh_due(
            &fetched(10),
            behind,
            NOW,
            tf,
            Priority::Low
        ));
        assert!(native_refresh_due(
            &fetched(15),
            behind,
            NOW,
            tf,
            Priority::Low
        ));

        // A scheduled re-fetch after a refreshing page is due on time, not before.
        let mut refreshing = NativeSeriesState::default();
        refreshing.record_page(NOW, caught_up, caught_up, true);
        assert!(!native_refresh_due(
            &refreshing,
            caught_up,
            NOW + 14,
            tf,
            Priority::Low
        ));
        assert!(native_refresh_due(
            &refreshing,
            caught_up,
            NOW + 15,
            tf,
            Priority::Low
        ));

        // Behind but confirmed by the no-newer rule: not a catch-up.
        let mut illiquid = NativeSeriesState::default();
        illiquid.record_page(NOW - 40, behind, behind, false);
        illiquid.record_page(NOW - 20, behind, behind, false);
        assert!(!native_refresh_due(
            &illiquid,
            behind,
            NOW,
            tf,
            Priority::High
        ));
    }

    #[test]
    fn caught_up_by_recency_unless_the_page_was_a_stale_server_answer() {
        let tf = Timeframe::Hour1;
        assert!(series_caught_up(Some(CURRENT), NOW, tf, false, false));
        assert!(series_caught_up(
            Some(CURRENT - HOUR),
            NOW,
            tf,
            false,
            false
        ));
        assert!(!series_caught_up(
            Some(CURRENT - 2 * HOUR),
            NOW,
            tf,
            false,
            false
        ));
        assert!(!series_caught_up(None, NOW, tf, false, false));
        // A refreshing page that looks recent is not enough on its own.
        assert!(!series_caught_up(Some(CURRENT), NOW, tf, true, false));
        // The no-newer rule accepts an old series.
        assert!(series_caught_up(
            Some(CURRENT - 50 * HOUR),
            NOW,
            tf,
            true,
            true
        ));
    }

    #[test]
    fn no_newer_rule_needs_two_fetches_the_retry_delay_apart() {
        let tf = Timeframe::Hour1;
        let old = Some(CURRENT - 50 * HOUR);
        let mut state = NativeSeriesState::default();

        state.record_page(NOW, old, old, false);
        assert!(!state.is_caught_up(old, NOW, tf));
        // Too soon: does not confirm, and does not restart the streak.
        state.record_page(NOW + 5, old, old, false);
        assert!(!state.is_caught_up(old, NOW + 5, tf));
        state.record_page(NOW + NATIVE_RETRY_DELAY_SECS, old, old, false);
        assert!(state.is_caught_up(old, NOW + NATIVE_RETRY_DELAY_SECS, tf));

        // Confirmation belongs to the newest it was measured against.
        let newer = Some(CURRENT - 40 * HOUR);
        assert!(!state.is_caught_up(newer, NOW + 20, tf));

        // A page with a newer candle resets the streak.
        state.record_page(NOW + 30, old, newer, false);
        assert!(!state.is_caught_up(newer, NOW + 30, tf));
        state.record_page(NOW + 40, newer, newer, false);
        state.record_page(NOW + 60, newer, newer, false);
        assert!(state.is_caught_up(newer, NOW + 60, tf));

        // A source with nothing at all is confirmed empty too.
        let mut empty = NativeSeriesState::default();
        empty.record_page(NOW, None, None, false);
        empty.record_page(NOW + NATIVE_RETRY_DELAY_SECS, None, None, false);
        assert!(empty.no_newer_holds(None));
    }

    #[test]
    fn aggregated_candles_carry_the_newest_minute_of_their_bucket() {
        let minute = |ts: i64| Candle::new(ts, 1.0, 1.0, 1.0, 1.0, 1.0);
        let base = 1_700_000_000 - 1_700_000_000 % 3_600;
        let minutes = vec![
            minute(base),
            minute(base + 60),
            minute(base + 1_740),
            minute(base + 3_600),
            minute(base + 3_660),
        ];
        let aggregated =
            OhlcvAggregator::aggregate(&minutes, Timeframe::Minute1, Timeframe::Hour1).unwrap();
        let paired: Vec<(i64, i64)> = with_newest_minute(&minutes, aggregated, Timeframe::Hour1)
            .into_iter()
            .map(|(candle, newest)| (candle.timestamp, newest))
            .collect();
        assert_eq!(
            paired,
            vec![(base, base + 1_740), (base + 3_600, base + 3_660)]
        );
    }

    #[test]
    fn refreshing_refetch_delays_back_off_to_five_minutes_then_stop() {
        let delays: Vec<Option<i64>> = (0..12).map(refreshing_refetch_delay_secs).collect();
        assert_eq!(
            delays,
            vec![
                Some(15),
                Some(30),
                Some(60),
                Some(120),
                Some(240),
                Some(300),
                Some(300),
                Some(300),
                Some(300),
                Some(300),
                None,
                None,
            ]
        );
        let total: i64 = delays.iter().flatten().sum();
        assert_eq!(total, 1_965);
        assert_eq!(refreshing_refetch_delay_secs(u32::MAX), None);
    }

    #[test]
    fn refreshing_pages_schedule_backed_off_refetches_until_the_budget_is_spent() {
        let recent = Some(CURRENT);
        let mut state = NativeSeriesState::default();
        let mut at = NOW;
        for attempt in 0..MAX_REFRESHING_REFETCHES {
            state.record_page(at, recent, recent, true);
            let delay = refreshing_refetch_delay_secs(attempt).unwrap();
            assert_eq!(state.refetch_at, Some(at + delay));
            assert_eq!(state.refetches, attempt + 1);
            at += delay;
        }
        state.record_page(at, recent, recent, true);
        assert_eq!(state.refetch_at, None);
        assert_eq!(state.refetches, MAX_REFRESHING_REFETCHES);

        // A ready page clears the schedule and the budget.
        let ready_at = at + 100;
        state.record_page(ready_at, recent, recent, false);
        assert_eq!(state.refetch_at, None);
        assert_eq!(state.refetches, 0);
        assert!(state.is_caught_up(recent, ready_at, Timeframe::Hour1));

        // A refreshing page after a ready one starts the schedule over.
        let mut restarted = state.clone();
        restarted.record_page(ready_at + 10, recent, recent, true);
        assert_eq!(restarted.refetch_at, Some(ready_at + 10 + 15));
        assert_eq!(restarted.refetches, 1);

        // A failed fetch only moves the attempt clock.
        let before = state.clone();
        state.record_failure(ready_at + 100);
        assert_eq!(state.last_fetch_at, Some(ready_at + 100));
        assert_eq!(
            NativeSeriesState {
                last_fetch_at: before.last_fetch_at,
                ..state
            },
            before
        );
    }

    #[test]
    fn fallback_values_are_retried_soon_only_while_the_data_server_is_usable() {
        let mut state = NativeSeriesState {
            last_fetch_at: Some(NOW),
            ..NativeSeriesState::default()
        };
        // An empty fallback answer wrote nothing.
        state.record_source(Some(CandleSource::GeckoTerminal), false);
        assert!(!fallback_refresh_due(
            &state,
            NOW + FALLBACK_RETRY_DELAY_SECS,
            true
        ));

        state.record_source(Some(CandleSource::GeckoTerminal), true);
        assert!(!fallback_refresh_due(
            &state,
            NOW + FALLBACK_RETRY_DELAY_SECS - 1,
            true
        ));
        assert!(fallback_refresh_due(
            &state,
            NOW + FALLBACK_RETRY_DELAY_SECS,
            true
        ));
        // Data Server disabled or unavailable: the fallback is primary.
        assert!(!fallback_refresh_due(
            &state,
            NOW + FALLBACK_RETRY_DELAY_SECS,
            false
        ));
        // An empty answer does not clear values an earlier page stored.
        state.record_source(None, false);
        assert!(state.fallback_values);

        state.record_source(Some(CandleSource::Feed("feed")), true);
        assert!(state.fallback_values);
        // A Data Server page replaces the fallback values.
        state.record_source(Some(CandleSource::DataServer), true);
        assert!(!fallback_refresh_due(
            &state,
            NOW + FALLBACK_RETRY_DELAY_SECS,
            true
        ));
    }

    #[test]
    fn fallback_retries_stop_after_the_cap_until_a_data_server_page() {
        let mut state = NativeSeriesState {
            last_fetch_at: Some(NOW),
            ..NativeSeriesState::default()
        };
        let due = |state: &NativeSeriesState| {
            fallback_refresh_due(state, NOW + FALLBACK_RETRY_DELAY_SECS, true)
        };

        // The first fallback page, then one short retry per further fallback page.
        state.record_source(Some(CandleSource::GeckoTerminal), true);
        for _ in 0..MAX_FALLBACK_RETRIES {
            assert!(due(&state));
            state.record_source(Some(CandleSource::GeckoTerminal), true);
        }
        assert_eq!(state.fallback_pages, MAX_FALLBACK_RETRIES + 1);
        assert!(state.fallback_values);
        assert!(!due(&state));

        // Empty answers neither extend nor reset the streak.
        state.record_source(Some(CandleSource::GeckoTerminal), false);
        state.record_source(None, false);
        assert!(!due(&state));

        // A Data Server page clears the streak; a later fallback page retries again.
        state.record_source(Some(CandleSource::DataServer), true);
        assert_eq!(state.fallback_pages, 0);
        state.record_source(Some(CandleSource::Feed("feed")), true);
        assert!(due(&state));
    }

    #[test]
    fn a_pass_fetches_the_most_overdue_timeframe_first() {
        let due = |timeframe, ago: Option<i64>| DueTimeframe {
            timeframe,
            newest: None,
            last_fetch_at: ago.map(|ago| NOW - ago),
        };
        // High priority intervals: 5m 300 s, 15m 900 s, 1h 1800 s, 4h+ 3600 s.
        let mut pass = vec![
            due(Timeframe::Minute5, Some(10)),
            due(Timeframe::Minute15, Some(450)),
            due(Timeframe::Hour1, Some(1_800)),
            due(Timeframe::Hour4, Some(3_600)),
            due(Timeframe::Hour12, None),
            due(Timeframe::Day1, Some(7_200)),
        ];
        order_by_overdue(&mut pass, NOW, Priority::High);
        let order: Vec<Timeframe> = pass.iter().map(|d| d.timeframe).collect();
        assert_eq!(
            order,
            vec![
                // Never fetched since start.
                Timeframe::Hour12,
                // Two intervals overdue.
                Timeframe::Day1,
                // One interval each: fine first.
                Timeframe::Hour1,
                Timeframe::Hour4,
                Timeframe::Minute15,
                // Rate limited a moment ago.
                Timeframe::Minute5,
            ]
        );

        // Several never fetched: fine first.
        let mut fresh = vec![
            due(Timeframe::Minute5, None),
            due(Timeframe::Hour1, None),
            due(Timeframe::Minute15, Some(5_000)),
        ];
        order_by_overdue(&mut fresh, NOW, Priority::Low);
        let order: Vec<Timeframe> = fresh.iter().map(|d| d.timeframe).collect();
        assert_eq!(
            order,
            vec![Timeframe::Minute5, Timeframe::Hour1, Timeframe::Minute15]
        );
    }

    #[test]
    fn settle_delay_follows_the_data_server_window_within_bounds() {
        assert_eq!(settle_delay_secs(Timeframe::Minute5), 300);
        assert_eq!(settle_delay_secs(Timeframe::Minute15), 780);
        for tf in [
            Timeframe::Hour1,
            Timeframe::Hour4,
            Timeframe::Hour12,
            Timeframe::Day1,
        ] {
            assert_eq!(settle_delay_secs(tf), MAX_SETTLE_SECS);
        }
        for tf in AGGREGATED_TIMEFRAMES {
            let delay = settle_delay_secs(tf);
            assert!((MIN_SETTLE_SECS..=MAX_SETTLE_SECS).contains(&delay));
        }
        // The target is the bucket that closed last once its delay has passed,
        // the one before it until then.
        let tf = Timeframe::Hour1;
        let delay = settle_delay_secs(tf);
        assert_eq!(settle_target(CURRENT + delay, tf), CURRENT - HOUR);
        assert_eq!(settle_target(CURRENT + delay - 1, tf), CURRENT - 2 * HOUR);
        assert_eq!(settle_point(CURRENT - HOUR, tf), CURRENT + delay);
        // A delay as long as the bucket still targets a closed bucket.
        let five = Timeframe::Minute5;
        let start = CURRENT + 5 * 300;
        assert_eq!(settle_target(start, five), start - 2 * 300);
        assert_eq!(settle_point(start - 2 * 300, five), start);
    }

    #[test]
    fn a_closed_bucket_is_settled_once_after_its_settle_point() {
        let tf = Timeframe::Hour1;
        let delay = settle_delay_secs(tf);
        let previous = CURRENT - HOUR;
        let settle_at = CURRENT + delay;
        let newest = Some(previous);
        let fetched = |at: i64| NativeSeriesState {
            last_fetch_at: Some(at),
            settled_bucket: Some(previous - HOUR),
            ..NativeSeriesState::default()
        };
        let native = |at: i64| {
            Some(StoredBucket {
                native: true,
                fetched_at: Some(at),
            })
        };

        // Written just after the close: unsettled, due at the settle point and
        // not before.
        let state = fetched(CURRENT + 30);
        let row = native(CURRENT + 30);
        assert!(!settle_refresh_due(&state, newest, row, settle_at - 1, tf));
        assert!(settle_refresh_due(&state, newest, row, settle_at, tf));
        // Still due later in the bucket until a fetch settles it.
        assert!(settle_refresh_due(
            &state,
            newest,
            row,
            CURRENT + HOUR - 1,
            tf
        ));

        // Written at or after the settle point: settled.
        assert!(!settle_refresh_due(
            &fetched(settle_at),
            newest,
            native(settle_at),
            settle_at + 60,
            tf
        ));

        // A local aggregate only: unsettled.
        let aggregate = Some(StoredBucket {
            native: false,
            fetched_at: Some(settle_at + 10),
        });
        assert!(settle_refresh_due(
            &state,
            newest,
            aggregate,
            settle_at + 60,
            tf
        ));

        // One fetch past the settle point settles the bucket even when it
        // returned identical values and the row's write time did not move.
        let mut after = state.clone();
        after.record_page(settle_at + 5, newest, newest, false);
        after.record_settled(settle_at + 5, tf);
        assert_eq!(after.settled_bucket, Some(previous));
        assert!(!settle_refresh_due(&after, newest, row, settle_at + 60, tf));
        assert!(!settle_refresh_due(
            &after,
            newest,
            row,
            CURRENT + HOUR + delay - 1,
            tf
        ));
        // The next bucket settles on its own schedule.
        assert!(settle_refresh_due(
            &after,
            Some(CURRENT),
            native(CURRENT + HOUR + 5),
            CURRENT + HOUR + delay,
            tf
        ));

        // A refreshing Data Server page with a re-fetch scheduled does not settle.
        let mut refreshing = state.clone();
        refreshing.record_page(settle_at + 5, newest, newest, true);
        refreshing.record_settled(settle_at + 5, tf);
        assert_eq!(refreshing.settled_bucket, Some(previous - HOUR));

        // Retry spacing and an empty series.
        assert!(!settle_refresh_due(
            &fetched(settle_at - 5),
            newest,
            row,
            settle_at,
            tf
        ));
        assert!(!settle_refresh_due(
            &NativeSeriesState::default(),
            None,
            None,
            settle_at,
            tf
        ));
    }

    #[test]
    fn a_bucket_without_trades_is_not_refetched_for_settling() {
        let tf = Timeframe::Hour1;
        let delay = settle_delay_secs(tf);
        let settle_at = CURRENT + delay;
        let newest = Some(CURRENT - 2 * HOUR);

        // Missing in a series with native rows: one settle fetch.
        let mut state = NativeSeriesState {
            last_fetch_at: Some(CURRENT + 30),
            settled_bucket: Some(CURRENT - 2 * HOUR),
            ..NativeSeriesState::default()
        };
        assert!(settle_refresh_due(&state, newest, None, settle_at, tf));

        // The source returned nothing for it: never due again for that bucket.
        state.record_page(settle_at, newest, newest, false);
        state.record_settled(settle_at, tf);
        for later in [settle_at + 15, settle_at + 600, CURRENT + HOUR + delay - 1] {
            assert!(!settle_refresh_due(&state, newest, None, later, tf));
        }

        // Confirmed by the no-newer rule: a missing bucket is not a settle
        // candidate at all.
        let mut idle = NativeSeriesState::default();
        idle.record_page(CURRENT + 20, newest, newest, false);
        idle.record_page(CURRENT + 40, newest, newest, false);
        assert!(idle.no_newer_holds(newest));
        assert!(!settle_refresh_due(&idle, newest, None, settle_at, tf));
    }

    #[test]
    fn newest_storable_bucket_ignores_empty_candles_and_snaps_to_the_bucket() {
        let tf = Timeframe::Hour1;
        assert_eq!(newest_storable_bucket(&[], tf), None);
        assert_eq!(
            newest_storable_bucket(
                &[candle(CURRENT, 0.0), candle(CURRENT - HOUR + 90, 2.0)],
                tf
            ),
            Some(CURRENT - HOUR)
        );
        assert_eq!(
            newest_storable_bucket(&[candle(CURRENT, f64::NAN)], tf),
            None
        );
    }

    #[tokio::test]
    async fn an_explicit_refresh_rediscovers_only_without_pools_or_a_fresh_snapshot() {
        use crate::chains::ChainId;
        let _ = crate::config::utils::CONFIG
            .get_or_init(|| std::sync::RwLock::new(crate::config::Config::default()));
        let path = std::env::temp_dir().join(format!(
            "screenerbot-ohlcv-monitor-rediscovery-{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let db = Arc::new(OhlcvDatabase::new(&path, ChainId::Solana).unwrap());
        let fetcher = Arc::new(OhlcvFetcher::new(ChainId::Solana));
        let cache = Arc::new(OhlcvCache::new(ChainId::Solana));
        let pool_manager = Arc::new(PoolManager::new(Arc::clone(&db), Arc::clone(&cache)));
        let gap_manager = Arc::new(GapManager::new(
            Arc::clone(&db),
            Arc::clone(&fetcher),
            Arc::clone(&cache),
        ));
        let monitor = OhlcvMonitor::new(Arc::clone(&db), fetcher, cache, pool_manager, gap_manager);

        // The production freshness input: a mint the token cache never saw is not fresh.
        assert!(!crate::tokens::has_fresh_token_pools_snapshot(
            ChainId::Solana,
            "never-seen-mint"
        ));
        assert!(monitor.pool_rediscovery_due("mint", true).await.unwrap());
        assert!(monitor.pool_rediscovery_due("mint", false).await.unwrap());
        db.upsert_pool(
            "mint",
            &PoolConfig::new("pool".to_string(), "dex".to_string(), 1.0),
        )
        .unwrap();
        assert!(!monitor.pool_rediscovery_due("mint", true).await.unwrap());
        assert!(monitor.pool_rediscovery_due("mint", false).await.unwrap());

        drop(monitor);
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn an_unknown_pool_or_quote_is_not_native() {
        let mut sol = PoolConfig::new("sol".to_owned(), "dex".to_owned(), 1.0);
        sol.is_native_pair = true;
        let mut usd = PoolConfig::new("usd".to_owned(), "dex".to_owned(), 1.0);
        usd.is_native_pair = false;
        let pools = [sol, usd];
        assert!(registered_pool_is_native(&pools, "sol"));
        assert!(!registered_pool_is_native(&pools, "usd"));
        assert!(!registered_pool_is_native(&pools, "unregistered"));
        assert!(!registered_pool_is_native(&[], "sol"));
    }
}
