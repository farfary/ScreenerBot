// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV gap detection and filling — tracks missing candle ranges and fills
//! them from the sources, recording every attempt.

use crate::config::with_config;
use crate::events::{record_ohlcv_event, Severity};
use crate::ohlcvs::aggregator::OhlcvAggregator;
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::{GapRecord, OhlcvDatabase};
use crate::ohlcvs::fetcher::{FetchResponse, OhlcvFetcher, MAX_CANDLES_PER_REQUEST};
use crate::ohlcvs::manager::PoolManager;
use crate::ohlcvs::monitor::CATCH_UP_MARGIN;
use crate::ohlcvs::types::{Candle, OhlcvError, OhlcvResult, Timeframe};
use chrono::Utc;
use serde_json::json;
use std::sync::Arc;
use tokio::time::Duration;

/// Gap-fill requests one scan may spend across all tokens. The scan runs every
/// five minutes next to the live monitor, so this keeps it to a few requests
/// per minute on the shared provider budgets.
pub(crate) const GAP_FILL_REQUESTS_PER_CYCLE: usize = 24;

/// Attempts without progress after which a gap span is no longer retried.
pub(crate) const MAX_GAP_ATTEMPTS: u32 = 6;

/// Retry delay after the first inconclusive attempt, doubled on each further
/// one. Just under the five-minute scan interval, so the first retry lands on
/// the next scan.
const GAP_RETRY_BASE_SECS: i64 = 240;

/// Longest span one request is asked to cover, in buckets. Leaves room for the
/// margin that reaches past the span start.
const GAP_SPAN_MAX_BUCKETS: usize = MAX_CANDLES_PER_REQUEST - CATCH_UP_MARGIN;

const REASON_EMPTY_ANSWER: &str = "source returned no candles";
const REASON_SERVER_REFRESHING: &str = "data server is refreshing the series";
const REASON_NOT_REACHED: &str = "source answer did not reach the span";

/// Adjacent unfilled gaps of one timeframe, filled with one request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapSpan {
    pub timeframe: Timeframe,
    /// First missing bucket.
    pub start: i64,
    /// Last missing bucket (inclusive).
    pub end: i64,
    /// Most attempts among the span's rows.
    pub attempts: u32,
    /// Latest attempt among the span's rows.
    pub last_attempt: Option<i64>,
}

impl From<GapRecord> for GapSpan {
    fn from(row: GapRecord) -> Self {
        Self {
            timeframe: row.timeframe,
            start: row.start_timestamp,
            end: row.end_timestamp,
            attempts: row.attempts,
            last_attempt: row.last_attempt,
        }
    }
}

/// How one span is requested from `fetch_multi_source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GapRequest {
    /// `None`: the newest `limit` candles ending now. `Some(ts)`: the newest
    /// `limit` candles strictly older than `ts`.
    pub before: Option<i64>,
    pub limit: usize,
}

/// What a source answer says about a span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapOutcome {
    /// The answer reached the span start, so every bucket in the span is
    /// answered. `candles` is the storable candles inside it; zero means the
    /// span is source-confirmed no-trade.
    Resolved { candles: usize },
    /// The answer reached into the span but not its start: `[covered_start,
    /// end]` is answered, the rest stays open.
    Partial { covered_start: i64 },
    /// Nothing conclusive; the reason is recorded on the span's rows.
    Inconclusive(&'static str),
}

/// Result of one span fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GapFill {
    pub outcome: GapOutcome,
    /// Rows inserted or changed.
    pub written: usize,
    /// First and last stored bucket written, if any.
    pub written_range: Option<(i64, i64)>,
}

fn buckets_between(start: i64, end: i64, timeframe: Timeframe) -> usize {
    let secs = timeframe.to_seconds().max(1);
    usize::try_from(((end - start) / secs).max(0))
        .unwrap_or(usize::MAX)
        .saturating_add(1)
}

/// Group gap rows into spans, per timeframe in start order. A row joins the
/// current span when it overlaps or touches it, or when the grown span still
/// fits one request (`GAP_SPAN_MAX_BUCKETS`). A single longer hole is its own
/// span and is covered over several attempts.
pub fn group_spans(mut rows: Vec<GapRecord>) -> Vec<GapSpan> {
    rows.sort_by_key(|row| {
        (
            row.timeframe.to_seconds(),
            row.start_timestamp,
            row.end_timestamp,
        )
    });

    let mut spans: Vec<GapSpan> = Vec::new();
    for row in rows {
        if let Some(span) = spans.last_mut() {
            if span.timeframe == row.timeframe {
                let end = span.end.max(row.end_timestamp);
                let touches = row.start_timestamp <= span.end + row.timeframe.to_seconds();
                let fits = buckets_between(span.start, end, row.timeframe) <= GAP_SPAN_MAX_BUCKETS;
                if touches || fits {
                    span.end = end;
                    span.attempts = span.attempts.max(row.attempts);
                    span.last_attempt = span.last_attempt.max(row.last_attempt);
                    continue;
                }
            }
        }
        spans.push(GapSpan::from(row));
    }
    spans
}

/// Request covering a span. When the span start is within one request of the
/// current bucket, the newest candles reach it; otherwise the request pages
/// back from just after the span end.
pub fn plan_request(span: &GapSpan, now: i64) -> GapRequest {
    let current = OhlcvDatabase::bucket_start(now, span.timeframe);
    let to_now = buckets_between(span.start, current, span.timeframe);
    let newest = to_now.saturating_add(CATCH_UP_MARGIN);
    if newest <= MAX_CANDLES_PER_REQUEST {
        return GapRequest {
            before: None,
            limit: newest,
        };
    }
    GapRequest {
        before: Some(span.end + span.timeframe.to_seconds()),
        limit: buckets_between(span.start, span.end, span.timeframe)
            .saturating_add(CATCH_UP_MARGIN)
            .min(MAX_CANDLES_PER_REQUEST),
    }
}

/// Retry delay before attempt `attempts + 1`.
fn gap_retry_backoff_secs(attempts: u32) -> i64 {
    match attempts {
        0 => 0,
        n => GAP_RETRY_BASE_SECS << (n - 1).min(MAX_GAP_ATTEMPTS),
    }
}

/// Whether a span with this bookkeeping may be attempted at `now`.
pub fn gap_retry_due(attempts: u32, last_attempt: Option<i64>, now: i64) -> bool {
    if attempts >= MAX_GAP_ATTEMPTS {
        return false;
    }
    last_attempt.map_or(true, |last| now - last >= gap_retry_backoff_secs(attempts))
}

/// Due spans, most recent first, at most `budget` of them.
pub fn select_due_spans(mut spans: Vec<GapSpan>, now: i64, budget: usize) -> Vec<GapSpan> {
    spans.retain(|span| gap_retry_due(span.attempts, span.last_attempt, now));
    spans.sort_by(|a, b| b.end.cmp(&a.end).then(b.start.cmp(&a.start)));
    spans.truncate(budget);
    spans
}

/// Judge a source answer for a span. Only storable candles count. A stale Data
/// Server answer (refreshing behind it) is never conclusive: its cached series
/// may hold the very hole being filled.
pub fn evaluate_answer(span: &GapSpan, candles: &[Candle], server_refreshing: bool) -> GapOutcome {
    let buckets: Vec<i64> = candles
        .iter()
        .filter(|candle| OhlcvDatabase::is_storable(candle))
        .map(|candle| OhlcvDatabase::bucket_start(candle.timestamp, span.timeframe))
        .collect();
    let Some(oldest) = buckets.iter().copied().min() else {
        return GapOutcome::Inconclusive(REASON_EMPTY_ANSWER);
    };
    if server_refreshing {
        return GapOutcome::Inconclusive(REASON_SERVER_REFRESHING);
    }
    let in_span = buckets
        .iter()
        .filter(|bucket| (span.start..=span.end).contains(*bucket))
        .count();
    if oldest <= span.start {
        GapOutcome::Resolved { candles: in_span }
    } else if in_span > 0 {
        GapOutcome::Partial {
            covered_start: oldest,
        }
    } else {
        GapOutcome::Inconclusive(REASON_NOT_REACHED)
    }
}

/// Store an answer's in-span candles as native rows, drop the timeframe's
/// hot-cache entry and record the outcome on the span's gap rows.
#[allow(clippy::too_many_arguments)]
fn apply_answer(
    db: &OhlcvDatabase,
    cache: &OhlcvCache,
    mint: &str,
    pool_address: &str,
    span: &GapSpan,
    candles: &[Candle],
    server_refreshing: bool,
    now: i64,
) -> OhlcvResult<GapFill> {
    let timeframe = span.timeframe;
    let outcome = evaluate_answer(span, candles, server_refreshing);

    let in_span: Vec<Candle> = candles
        .iter()
        .filter(|candle| {
            OhlcvDatabase::is_storable(candle)
                && (span.start..=span.end)
                    .contains(&OhlcvDatabase::bucket_start(candle.timestamp, timeframe))
        })
        .cloned()
        .collect();
    let written_range = in_span
        .iter()
        .map(|candle| OhlcvDatabase::bucket_start(candle.timestamp, timeframe))
        .fold(None, |range: Option<(i64, i64)>, bucket| {
            Some(range.map_or((bucket, bucket), |(lo, hi)| {
                (lo.min(bucket), hi.max(bucket))
            }))
        });
    let written = if in_span.is_empty() {
        0
    } else {
        let written = db.insert_candles_batch(
            mint,
            pool_address,
            timeframe,
            &in_span,
            OhlcvDatabase::NATIVE_SOURCE,
        )?;
        cache.invalidate(mint, Some(pool_address), Some(timeframe))?;
        written
    };

    match outcome {
        GapOutcome::Resolved { .. } => {
            db.resolve_gap_span(mint, pool_address, timeframe, span.start, span.end, now)?
        }
        GapOutcome::Partial { covered_start } => db.split_gap_span(
            mint,
            pool_address,
            timeframe,
            span.start,
            span.end,
            covered_start,
            covered_start - timeframe.to_seconds(),
            span.attempts,
            now,
        )?,
        GapOutcome::Inconclusive(reason) => {
            db.record_gap_attempt(
                mint,
                pool_address,
                timeframe,
                span.start,
                span.end,
                now,
                reason,
            )?;
        }
    }

    Ok(GapFill {
        outcome,
        written,
        written_range,
    })
}

pub struct GapManager {
    db: Arc<OhlcvDatabase>,
    fetcher: Arc<OhlcvFetcher>,
    cache: Arc<OhlcvCache>,
    pool_manager: Arc<PoolManager>,
}

impl GapManager {
    pub fn new(
        db: Arc<OhlcvDatabase>,
        fetcher: Arc<OhlcvFetcher>,
        cache: Arc<OhlcvCache>,
        pool_manager: Arc<PoolManager>,
    ) -> Self {
        Self {
            db,
            fetcher,
            cache,
            pool_manager,
        }
    }

    /// Detect gaps in stored data for a token
    pub async fn detect_gaps(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<Vec<(i64, i64)>> {
        let retention_days = with_config(|cfg| cfg.ohlcv.retention_days).max(1);
        let timeframe_seconds = timeframe.to_seconds().max(1);
        let window_seconds = (retention_days as i64).saturating_mul(86_400);
        let lookback_start =
            (Utc::now().timestamp() - window_seconds - timeframe_seconds * 2).max(0);

        let estimated_points = ((window_seconds / 60).max(1) as usize).saturating_add(512);
        let limit = estimated_points.min(200_000);

        // DEBUG: Record gap detection start
        record_ohlcv_event(
            "gap_detection_start",
            Severity::Debug,
            Some(mint),
            Some(pool_address),
            json!({
                "mint": mint,
                "pool_address": pool_address,
                "timeframe": timeframe.to_string(),
                "lookback_start": lookback_start,
                "limit": limit,
            }),
        )
        .await;

        // Get existing data sliced to the retention window in ascending order
        let data = self.db.get_candles(
            mint,
            Some(pool_address),
            Timeframe::Minute1,
            Some(lookback_start),
            None,
            Some(limit),
        )?;

        if data.is_empty() {
            return Ok(Vec::new());
        }

        // Normalize data for requested timeframe.
        let normalized = if timeframe == Timeframe::Minute1 {
            data
        } else {
            OhlcvAggregator::aggregate(&data, Timeframe::Minute1, timeframe)?
        };

        // Detect gaps using aggregator
        let gaps = OhlcvAggregator::detect_gaps(&normalized, timeframe);

        // Store detected gaps in database
        for (start, end) in &gaps {
            self.db
                .insert_gap(mint, pool_address, timeframe, *start, *end)?;
        }

        // INFO: Record gap detection completion
        record_ohlcv_event(
            "gap_detection_complete",
            Severity::Info,
            Some(mint),
            Some(pool_address),
            json!({
                "mint": mint,
                "pool_address": pool_address,
                "timeframe": timeframe.to_string(),
                "gaps_found": gaps.len(),
                "data_points_analyzed": normalized.len(),
            }),
        )
        .await;

        Ok(gaps)
    }

    /// Get unfilled gaps for a token
    pub async fn get_unfilled_gaps(
        &self,
        mint: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<Vec<Gap>> {
        let gap_tuples = self.db.get_unfilled_gaps(mint, timeframe)?;

        Ok(gap_tuples
            .into_iter()
            .map(|(pool_address, start, end)| Gap {
                mint: mint.to_string(),
                pool_address,
                timeframe,
                start_timestamp: start,
                end_timestamp: end,
            })
            .collect())
    }

    /// Check data quality and detect potential issues
    pub async fn check_data_quality(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<DataQualityReport> {
        let mut data = self.db.get_candles(
            mint,
            Some(pool_address),
            Timeframe::Minute1,
            None,
            None,
            Some(10000),
        )?;

        let total_candles = data.len();

        // Sort to ASC for accurate gap detection
        data.sort_by_key(|d| d.timestamp);

        let gaps = OhlcvAggregator::detect_gaps(&data, timeframe);
        let gap_count = gaps.len();

        let invalid_candles = data.iter().filter(|d| !d.is_valid()).count();

        // Calculate expected candles
        let expected = if let (Some(first), Some(last)) = (data.first(), data.last()) {
            OhlcvAggregator::expected_candles(first.timestamp, last.timestamp, timeframe)
        } else {
            0
        };

        let completeness = if expected > 0 {
            ((total_candles as f64) / (expected as f64)) * 100.0
        } else {
            100.0
        };

        Ok(DataQualityReport {
            total_candles,
            expected_candles: expected,
            gap_count,
            invalid_candles,
            completeness_percent: completeness,
            has_issues: gap_count > 0 || invalid_candles > 0,
        })
    }

    /// Due gap spans of one pool within the retention window, most recent
    /// first, at most `budget` of them.
    pub fn due_spans(
        &self,
        mint: &str,
        pool_address: &str,
        now: i64,
        budget: usize,
    ) -> OhlcvResult<Vec<GapSpan>> {
        if budget == 0 {
            return Ok(Vec::new());
        }
        let retention_days = with_config(|cfg| cfg.ohlcv.retention_days).max(1);
        let since = now.saturating_sub(retention_days.saturating_mul(86_400));
        let rows = self
            .db
            .get_open_gaps(mint, pool_address, since, MAX_GAP_ATTEMPTS)?;
        Ok(select_due_spans(group_spans(rows), now, budget))
    }

    /// Fill one span through `fetch_multi_source` for the span's own timeframe
    /// (Data Server first) and store the answer (`store_span_answer`). A failed
    /// request is recorded as an attempt and returned as the error.
    pub async fn fill_span(
        &self,
        mint: &str,
        pool_address: &str,
        pool_is_native: bool,
        span: &GapSpan,
    ) -> OhlcvResult<GapFill> {
        let request = plan_request(span, Utc::now().timestamp());
        let (api_endpoint, aggregate) = span.timeframe.to_api_params();
        let response = match self
            .fetcher
            .fetch_multi_source(
                mint,
                pool_address,
                api_endpoint,
                aggregate,
                request.limit,
                pool_is_native,
                request.before,
            )
            .await
        {
            Ok(response) => response,
            Err(e) => {
                self.db.record_gap_attempt(
                    mint,
                    pool_address,
                    span.timeframe,
                    span.start,
                    span.end,
                    Utc::now().timestamp(),
                    &e.to_string(),
                )?;
                return Err(e);
            }
        };

        self.store_span_answer(mint, pool_address, span, request, response)
            .await
    }

    /// Store the answer to `request` for `span` (`apply_answer`) and record the outcome. A Data
    /// Server page naming another series pool than `pool_address` moves the default
    /// (`PoolManager::page_series_pool`) and fills nothing: the span is a hole in the previous
    /// pool's series, whose rows the move removed. That answer returns
    /// `OhlcvError::SeriesPoolMoved`.
    async fn store_span_answer(
        &self,
        mint: &str,
        pool_address: &str,
        span: &GapSpan,
        request: GapRequest,
        response: FetchResponse,
    ) -> OhlcvResult<GapFill> {
        let page_pool = self
            .pool_manager
            .page_series_pool(mint, pool_address, response.series_pool.as_deref())
            .await?;
        if page_pool != pool_address {
            return Err(OhlcvError::SeriesPoolMoved {
                mint: mint.to_string(),
                pool: pool_address.to_string(),
            });
        }

        let fill = apply_answer(
            &self.db,
            &self.cache,
            mint,
            pool_address,
            span,
            &response.candles,
            response.server_refreshing,
            Utc::now().timestamp(),
        )?;

        let (event, severity) = match fill.outcome {
            GapOutcome::Resolved { candles: 0 } => ("gap_fill_confirmed_no_trade", Severity::Debug),
            GapOutcome::Resolved { .. } => ("gap_fill_complete", Severity::Info),
            GapOutcome::Partial { .. } => ("gap_fill_partial", Severity::Info),
            GapOutcome::Inconclusive(_) => ("gap_fill_inconclusive", Severity::Debug),
        };
        record_ohlcv_event(
            event,
            severity,
            Some(mint),
            Some(pool_address),
            json!({
                "timeframe": span.timeframe.as_str(),
                "start_timestamp": span.start,
                "end_timestamp": span.end,
                "attempt": span.attempts + 1,
                "before": request.before,
                "limit": request.limit,
                "outcome": format!("{:?}", fill.outcome),
                "rows_written": fill.written,
            }),
        )
        .await;

        Ok(fill)
    }

    /// Estimate time to fill all gaps
    pub async fn estimate_fill_time(&self, mint: &str) -> OhlcvResult<Duration> {
        let mut total_gaps = 0;

        for timeframe in Timeframe::all() {
            let gaps = self.get_unfilled_gaps(mint, timeframe).await?;
            total_gaps += gaps.len();
        }

        // Estimate: 2 seconds per gap (includes rate limiting)
        Ok(Duration::from_secs((total_gaps as u64) * 2))
    }
}

#[derive(Debug, Clone)]
pub struct Gap {
    pub mint: String,
    pub pool_address: String,
    pub timeframe: Timeframe,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
}

impl Gap {
    pub fn duration_seconds(&self) -> i64 {
        self.end_timestamp - self.start_timestamp
    }

    pub fn candle_count(&self) -> usize {
        let duration = self.duration_seconds();
        let candle_duration = self.timeframe.to_seconds();

        if candle_duration == 0 {
            return 0;
        }

        (duration / candle_duration) as usize
    }
}

#[derive(Debug, Clone, Default)]
pub struct GapFillStats {
    pub total_gaps: usize,
    pub filled_gaps: usize,
    pub failed_gaps: usize,
    pub data_points_added: usize,
}

#[derive(Debug, Clone)]
pub struct DataQualityReport {
    pub total_candles: usize,
    pub expected_candles: usize,
    pub gap_count: usize,
    pub invalid_candles: usize,
    pub completeness_percent: f64,
    pub has_issues: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainId;

    const MIN: i64 = 60;
    const HOUR: i64 = 3_600;
    /// Midnight, so every bucket boundary is obvious by inspection.
    const T0: i64 = 1_700_000_000 - 1_700_000_000 % 86_400;
    /// Two days after `T0`, 30 seconds into a minute bucket.
    const NOW: i64 = T0 + 2 * 86_400 + 30;

    fn row(timeframe: Timeframe, start: i64, end: i64) -> GapRecord {
        GapRecord {
            timeframe,
            start_timestamp: start,
            end_timestamp: end,
            attempts: 0,
            last_attempt: None,
        }
    }

    fn span(timeframe: Timeframe, start: i64, end: i64) -> GapSpan {
        GapSpan::from(row(timeframe, start, end))
    }

    fn candle(ts: i64, volume: f64) -> Candle {
        Candle::new(ts, 1.0, 2.0, 0.5, 1.5, volume)
    }

    #[test]
    fn adjacent_gaps_of_one_timeframe_group_into_spans_that_fit_one_request() {
        let m1 = Timeframe::Minute1;
        let mut first = row(m1, T0 + 2 * MIN, T0 + 3 * MIN);
        first.attempts = 2;
        first.last_attempt = Some(NOW - 100);
        let rows = vec![
            row(m1, T0 + 10 * MIN, T0 + 10 * MIN),
            first,
            row(m1, T0, T0),
            // Far beyond one request from the span start: a new span.
            row(m1, T0 + 2_000 * MIN, T0 + 2_001 * MIN),
            // Another timeframe never joins a 1m span.
            row(Timeframe::Hour1, T0 + HOUR, T0 + 2 * HOUR),
        ];

        let spans = group_spans(rows);
        assert_eq!(spans.len(), 3);
        assert_eq!(
            spans[0],
            GapSpan {
                timeframe: m1,
                start: T0,
                end: T0 + 10 * MIN,
                attempts: 2,
                last_attempt: Some(NOW - 100),
            }
        );
        assert_eq!(
            (spans[1].start, spans[1].end),
            (T0 + 2_000 * MIN, T0 + 2_001 * MIN)
        );
        assert_eq!(spans[2].timeframe, Timeframe::Hour1);

        // The span cap: the last bucket that still fits joins, the next does not.
        let cap = GAP_SPAN_MAX_BUCKETS as i64;
        let spans = group_spans(vec![
            row(m1, T0, T0),
            row(m1, T0 + (cap - 1) * MIN, T0 + (cap - 1) * MIN),
            row(m1, T0 + (cap + 5) * MIN, T0 + (cap + 5) * MIN),
        ]);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].end, T0 + (cap - 1) * MIN);

        // A hole longer than one request absorbs every row it overlaps or touches.
        let spans = group_spans(vec![
            row(m1, T0, T0 + 3_000 * MIN),
            row(m1, T0, T0 + 20 * MIN),
            row(m1, T0 + 3_001 * MIN, T0 + 3_001 * MIN),
        ]);
        assert_eq!(spans.len(), 1);
        assert_eq!((spans[0].start, spans[0].end), (T0, T0 + 3_001 * MIN));
    }

    #[test]
    fn a_recent_span_uses_the_newest_candles_and_an_old_one_pages_back() {
        let current = OhlcvDatabase::bucket_start(NOW, Timeframe::Minute1);

        // 100 buckets from the span start through the current bucket, + margin.
        let recent = span(Timeframe::Minute1, current - 99 * MIN, current - 90 * MIN);
        assert_eq!(
            plan_request(&recent, NOW),
            GapRequest {
                before: None,
                limit: 100 + CATCH_UP_MARGIN,
            }
        );

        // The largest newest-N request still fits.
        let edge_buckets = (MAX_CANDLES_PER_REQUEST - CATCH_UP_MARGIN) as i64;
        let edge = span(
            Timeframe::Minute1,
            current - (edge_buckets - 1) * MIN,
            current - 500 * MIN,
        );
        assert_eq!(plan_request(&edge, NOW).before, None);
        assert_eq!(plan_request(&edge, NOW).limit, MAX_CANDLES_PER_REQUEST);

        // One bucket older: paged back from just after the span end, sized to
        // the span's 9 buckets.
        let old = span(
            Timeframe::Minute1,
            current - edge_buckets * MIN,
            current - 990 * MIN,
        );
        assert_eq!(
            plan_request(&old, NOW),
            GapRequest {
                before: Some(current - 989 * MIN),
                limit: 9 + CATCH_UP_MARGIN,
            }
        );

        // A hole longer than one request asks for the request cap.
        let long = span(Timeframe::Minute1, T0, T0 + 3_000 * MIN);
        assert_eq!(
            plan_request(&long, NOW),
            GapRequest {
                before: Some(T0 + 3_001 * MIN),
                limit: MAX_CANDLES_PER_REQUEST,
            }
        );
    }

    #[test]
    fn answers_resolve_split_or_stay_inconclusive() {
        let gap = span(Timeframe::Minute1, T0 + 10 * MIN, T0 + 20 * MIN);

        // Reaching past the start with nothing inside: source-confirmed no-trade.
        let no_trade = [candle(T0 + 5 * MIN, 1.0), candle(T0 + 25 * MIN, 1.0)];
        assert_eq!(
            evaluate_answer(&gap, &no_trade, false),
            GapOutcome::Resolved { candles: 0 }
        );
        // Candles with no volume are not trades.
        let empty_volume = [
            candle(T0 + 5 * MIN, 1.0),
            candle(T0 + 12 * MIN, 0.0),
            candle(T0 + 13 * MIN, f64::NAN),
        ];
        assert_eq!(
            evaluate_answer(&gap, &empty_volume, false),
            GapOutcome::Resolved { candles: 0 }
        );
        // Reaching exactly the start counts.
        let filled = [candle(T0 + 10 * MIN, 1.0), candle(T0 + 15 * MIN + 7, 1.0)];
        assert_eq!(
            evaluate_answer(&gap, &filled, false),
            GapOutcome::Resolved { candles: 2 }
        );
        // Inside the span but not back to its start: the covered part.
        let partial = [candle(T0 + 14 * MIN, 1.0), candle(T0 + 30 * MIN, 1.0)];
        assert_eq!(
            evaluate_answer(&gap, &partial, false),
            GapOutcome::Partial {
                covered_start: T0 + 14 * MIN
            }
        );
        // Nothing answered, nothing reached, or a stale server answer.
        assert_eq!(
            evaluate_answer(&gap, &[], false),
            GapOutcome::Inconclusive(REASON_EMPTY_ANSWER)
        );
        assert_eq!(
            evaluate_answer(&gap, &[candle(T0 + 25 * MIN, 1.0)], false),
            GapOutcome::Inconclusive(REASON_NOT_REACHED)
        );
        assert_eq!(
            evaluate_answer(&gap, &no_trade, true),
            GapOutcome::Inconclusive(REASON_SERVER_REFRESHING)
        );
    }

    #[test]
    fn retries_back_off_and_stop_after_the_attempt_bound() {
        assert!(gap_retry_due(0, None, NOW));
        assert!(gap_retry_due(0, Some(NOW), NOW));

        let mut expected = GAP_RETRY_BASE_SECS;
        for attempts in 1..MAX_GAP_ATTEMPTS {
            assert!(!gap_retry_due(attempts, Some(NOW - expected + 1), NOW));
            assert!(gap_retry_due(attempts, Some(NOW - expected), NOW));
            expected *= 2;
        }
        // Exhausted: never due again.
        assert!(!gap_retry_due(MAX_GAP_ATTEMPTS, None, NOW));
        assert!(!gap_retry_due(
            MAX_GAP_ATTEMPTS,
            Some(NOW - 30 * 86_400),
            NOW
        ));
    }

    #[test]
    fn a_scan_takes_due_spans_newest_first_up_to_the_budget() {
        let spans: Vec<GapSpan> = (0..10)
            .map(|i| span(Timeframe::Minute1, T0 + i * HOUR, T0 + i * HOUR + MIN))
            .collect();
        let mut backing_off = span(Timeframe::Minute1, T0 + 20 * HOUR, T0 + 20 * HOUR);
        backing_off.attempts = 1;
        backing_off.last_attempt = Some(NOW - 10);
        let mut all = spans.clone();
        all.push(backing_off);

        let picked = select_due_spans(all.clone(), NOW, 3);
        assert_eq!(
            picked.iter().map(|s| s.start).collect::<Vec<_>>(),
            vec![T0 + 9 * HOUR, T0 + 8 * HOUR, T0 + 7 * HOUR]
        );
        assert!(select_due_spans(all.clone(), NOW, 0).is_empty());
        assert_eq!(select_due_spans(all, NOW, 100).len(), 10);
    }

    fn open_store(label: &str) -> (OhlcvDatabase, OhlcvCache, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "screenerbot-ohlcv-gapfill-{label}-{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        (
            OhlcvDatabase::new(&path, ChainId::Solana).unwrap(),
            OhlcvCache::new(ChainId::Solana),
            path,
        )
    }

    fn open_rows(db: &OhlcvDatabase) -> Vec<(i64, i64, u32)> {
        db.get_open_gaps("mint", "pool", 0, u32::MAX)
            .unwrap()
            .into_iter()
            .map(|g| (g.start_timestamp, g.end_timestamp, g.attempts))
            .collect()
    }

    #[test]
    fn an_empty_answer_resolves_the_span_and_detection_does_not_reopen_it() {
        let (db, cache, path) = open_store("no-trade");
        let m1 = Timeframe::Minute1;
        db.insert_gap("mint", "pool", m1, T0 + MIN, T0 + MIN)
            .unwrap();
        db.insert_gap("mint", "pool", m1, T0 + 3 * MIN, T0 + 4 * MIN)
            .unwrap();
        let spans = group_spans(
            db.get_open_gaps("mint", "pool", 0, MAX_GAP_ATTEMPTS)
                .unwrap(),
        );
        assert_eq!(spans.len(), 1);

        let answer = [
            candle(T0, 1.0),
            candle(T0 + 2 * MIN, 1.0),
            candle(T0 + 5 * MIN, 1.0),
        ];
        // Only the neighbour between the two holes is inside the span.
        let fill =
            apply_answer(&db, &cache, "mint", "pool", &spans[0], &answer, false, NOW).unwrap();
        assert_eq!(fill.outcome, GapOutcome::Resolved { candles: 1 });
        assert_eq!(fill.written_range, Some((T0 + 2 * MIN, T0 + 2 * MIN)));
        assert!(open_rows(&db).is_empty());

        db.insert_gap("mint", "pool", m1, T0 + MIN, T0 + MIN)
            .unwrap();
        db.insert_gap("mint", "pool", m1, T0 + 3 * MIN, T0 + 4 * MIN)
            .unwrap();
        assert!(open_rows(&db).is_empty());

        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn fills_write_native_rows_drop_the_cache_and_record_attempts() {
        let (db, cache, path) = open_store("fill");
        let m1 = Timeframe::Minute1;
        db.insert_gap("mint", "pool", m1, T0 + 10 * MIN, T0 + 20 * MIN)
            .unwrap();
        let gap = group_spans(
            db.get_open_gaps("mint", "pool", 0, MAX_GAP_ATTEMPTS)
                .unwrap(),
        )
        .remove(0);
        cache
            .put("mint", Some("pool"), m1, vec![candle(T0, 1.0)])
            .unwrap();

        // Inconclusive: one attempt recorded, the rows stay open.
        let fill = apply_answer(&db, &cache, "mint", "pool", &gap, &[], false, NOW).unwrap();
        assert_eq!(fill.outcome, GapOutcome::Inconclusive(REASON_EMPTY_ANSWER));
        assert_eq!(fill.written, 0);
        assert_eq!(open_rows(&db), vec![(T0 + 10 * MIN, T0 + 20 * MIN, 1)]);
        let retried = group_spans(
            db.get_open_gaps("mint", "pool", 0, MAX_GAP_ATTEMPTS)
                .unwrap(),
        )
        .remove(0);
        assert_eq!(retried.last_attempt, Some(NOW));
        assert!(!gap_retry_due(
            retried.attempts,
            retried.last_attempt,
            NOW + 1
        ));

        // Partial: in-span candles are stored natively, the remainder stays open
        // with the attempts it had.
        let answer = [candle(T0 + 15 * MIN, 1.0), candle(T0 + 18 * MIN, 1.0)];
        let fill = apply_answer(
            &db,
            &cache,
            "mint",
            "pool",
            &retried,
            &answer,
            false,
            NOW + 300,
        )
        .unwrap();
        assert_eq!(
            fill.outcome,
            GapOutcome::Partial {
                covered_start: T0 + 15 * MIN
            }
        );
        assert_eq!(fill.written, 2);
        assert_eq!(open_rows(&db), vec![(T0 + 10 * MIN, T0 + 14 * MIN, 1)]);
        assert!(cache.get("mint", Some("pool"), m1).unwrap().is_none());
        let stored = db
            .get_candles("mint", Some("pool"), m1, Some(T0), None, None)
            .unwrap();
        assert_eq!(
            stored.iter().map(|c| c.timestamp).collect::<Vec<_>>(),
            vec![T0 + 15 * MIN, T0 + 18 * MIN]
        );

        drop(db);
        let _ = std::fs::remove_file(path);
    }

    /// A span answer the Data Server served from another series pool moves the default and
    /// fills nothing, under either pool; an answer from the requested pool fills the span.
    #[tokio::test]
    async fn a_span_answer_from_another_series_pool_moves_the_default_and_fills_nothing() {
        use crate::ohlcvs::database::SeriesPoolPlan;
        use crate::ohlcvs::fetcher::CandleSource;
        use crate::ohlcvs::types::PoolConfig;
        crate::config::utils::install_default_config();
        let (db, cache, path) = open_store("series-pool");
        let (db, cache) = (Arc::new(db), Arc::new(cache));
        db.write_series_pools("mint", |_| {
            Some(SeriesPoolPlan {
                pools: vec![
                    PoolConfig::new("pool".to_string(), "dex".to_string(), 300.0),
                    PoolConfig::new("server".to_string(), "dex".to_string(), 100.0),
                ],
                series: "pool".to_string(),
            })
        })
        .unwrap();
        let pool_manager = Arc::new(PoolManager::new(Arc::clone(&db), Arc::clone(&cache)));
        let gaps = GapManager::new(
            Arc::clone(&db),
            Arc::new(OhlcvFetcher::new(ChainId::Solana)),
            Arc::clone(&cache),
            pool_manager,
        );
        let m1 = Timeframe::Minute1;
        db.insert_gap("mint", "pool", m1, T0 + 10 * MIN, T0 + 20 * MIN)
            .unwrap();
        let gap = group_spans(
            db.get_open_gaps("mint", "pool", 0, MAX_GAP_ATTEMPTS)
                .unwrap(),
        )
        .remove(0);
        let request = plan_request(&gap, NOW);
        let page = |series_pool: &str| FetchResponse {
            candles: (10..=20).map(|m| candle(T0 + m * MIN, 1.0)).collect(),
            server_refreshing: false,
            source: Some(CandleSource::DataServer),
            series_pool: Some(series_pool.to_string()),
        };
        let stored = |pool: &str| {
            db.get_candles("mint", Some(pool), m1, None, None, None)
                .unwrap()
                .len()
        };

        let fill = gaps
            .store_span_answer("mint", "pool", &gap, request, page("pool"))
            .await
            .unwrap();
        assert_eq!(fill.outcome, GapOutcome::Resolved { candles: 11 });
        assert_eq!(stored("pool"), 11);

        let moved = gaps
            .store_span_answer("mint", "pool", &gap, request, page("server"))
            .await;
        assert!(matches!(moved, Err(OhlcvError::SeriesPoolMoved { .. })));
        let default =
            PoolConfig::series_pool(&db.get_pools("mint").unwrap()).map(|p| p.address.clone());
        assert_eq!(default.as_deref(), Some("server"));
        assert_eq!(stored("pool"), 0);
        assert_eq!(stored("server"), 0);

        drop(gaps);
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
