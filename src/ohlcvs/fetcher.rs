// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV fetcher — retrieves candlestick data from multiple sources.
//!
//! Sources (in priority order):
//! 0. ScreenerBot self-hosted OHLCV server — fast shared cache, tried FIRST
//! 1. The chain's candle feeds (`ChainRuntime::candle_feeds`) — by token address
//! 2. GeckoTerminal — uses pool address, rate-limited 30/min, free

use crate::apis::{get_api_manager, ApiManager, Error as ApiError};
use crate::chains::ChainId;
use crate::errors::NetworkError;
use crate::events::{record_ohlcv_event, Severity};
use crate::ohlcvs::feeds::CandleFeed;
use crate::ohlcvs::types::{Candle, OhlcvError, OhlcvResult, Priority, Timeframe};
use serde::Deserialize;
use serde_json::json;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
pub(crate) const MAX_CANDLES_PER_REQUEST: usize = 1000;

/// Upstream that answered a [`OhlcvFetcher::fetch_multi_source`] request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleSource {
    DataServer,
    /// A chain-owned candle feed, by its label.
    Feed(&'static str),
    GeckoTerminal,
}

impl CandleSource {
    /// A provider the fetch fell back to after the Data Server returned nothing.
    pub fn is_fallback(self) -> bool {
        !matches!(self, CandleSource::DataServer)
    }
}

/// Candles returned by [`OhlcvFetcher::fetch_multi_source`].
#[derive(Debug, Clone, Default)]
pub struct FetchResponse {
    pub candles: Vec<Candle>,
    /// The Data Server answered from a cached series it is refreshing behind the
    /// response (`refreshing` or `pending`), so a newer page is expected shortly.
    /// Always false for provider answers.
    pub server_refreshing: bool,
    /// The upstream that served `candles`; `None` when no source was asked.
    pub source: Option<CandleSource>,
}

/// `GET /v1/ohlcv?stateful=true` body.
#[derive(Debug, Deserialize)]
struct StatefulOhlcv {
    candles: Vec<Candle>,
    #[serde(default)]
    state: String,
}

/// Whether a Data Server OHLCV state announces a refresh in progress. Unknown
/// states read as `ready`.
fn server_state_is_refreshing(state: &str) -> bool {
    matches!(state, "refreshing" | "pending")
}

#[derive(Clone, Debug)]
struct FetchRequest {
    mint: String,
    pool_address: String,
    timeframe: Timeframe,
    priority: Priority,
    before_timestamp: Option<i64>,
    limit: usize,
    requested_at: Instant,
}

impl PartialEq for FetchRequest {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.requested_at == other.requested_at
    }
}

impl Eq for FetchRequest {}

impl PartialOrd for FetchRequest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FetchRequest {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then earlier requests
        match self.priority.cmp(&other.priority) {
            std::cmp::Ordering::Equal => other.requested_at.cmp(&self.requested_at),
            other => other,
        }
    }
}

pub struct OhlcvFetcher {
    chain: ChainId,
    api_manager: Arc<ApiManager>,
    request_history: Arc<Mutex<VecDeque<Instant>>>,
    request_queue: Arc<Mutex<BinaryHeap<FetchRequest>>>,
    api_calls_count: Arc<Mutex<u64>>,
    total_latency_ms: Arc<Mutex<u64>>,
}

impl OhlcvFetcher {
    pub fn new(chain: ChainId) -> Self {
        Self {
            chain,
            api_manager: get_api_manager(),
            request_history: Arc::new(Mutex::new(VecDeque::new())),
            request_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            api_calls_count: Arc::new(Mutex::new(0)),
            total_latency_ms: Arc::new(Mutex::new(0)),
        }
    }

    /// The candle feeds this fetcher's chain contributes, enabled per current
    /// config and in fetch order.
    pub(super) fn candle_feeds(&self) -> Vec<CandleFeed> {
        crate::chains::runtime_for(self.chain)
            .map(|runtime| runtime.candle_feeds())
            .unwrap_or_default()
    }

    /// Fetch OHLCV data for a pool with priority
    pub async fn fetch_ohlcv(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        priority: Priority,
        before_timestamp: Option<i64>,
        limit: usize,
    ) -> OhlcvResult<Vec<Candle>> {
        // Queue the request
        self.queue_request(
            mint.to_string(),
            pool_address.to_string(),
            timeframe,
            priority,
            before_timestamp,
            limit,
        )?;

        // Process queue (this will respect rate limits)
        self.process_queue().await
    }

    /// Fetch with explicit aggregate parameter (NEW - native timeframe support)
    /// Uses GeckoTerminal's native aggregate feature instead of client-side aggregation
    pub async fn fetch_with_aggregate(
        &self,
        pool_address: &str,
        api_endpoint: &str,
        aggregate: u32,
        before_timestamp: Option<i64>,
        limit: usize,
    ) -> OhlcvResult<Vec<Candle>> {
        self.record_attempt();
        let start = Instant::now();
        let limit_clamped = limit.min(1000) as u32;

        // DEBUG: Record fetch attempt with aggregate
        record_ohlcv_event(
            "fetch_aggregate_attempt",
            Severity::Debug,
            None,
            Some(pool_address),
            json!({
                "pool_address": pool_address,
                "api_endpoint": api_endpoint,
                "aggregate": aggregate,
                "before_timestamp": before_timestamp,
                "limit": limit_clamped,
            }),
        )
        .await;

        let response = self
            .api_manager
            .geckoterminal
            .fetch_ohlcv(
                crate::chains::adapter_for(self.chain).market_data_network(),
                pool_address,
                api_endpoint,
                Some(aggregate),
                Some(limit_clamped),
                Some("token"),
                before_timestamp,
                None,
            )
            .await;

        match response {
            Ok(ohlcv) => {
                let data_points: Vec<Candle> = ohlcv
                    .ohlcv_list
                    .into_iter()
                    .map(|candle| Candle {
                        timestamp: candle[0] as i64,
                        open: candle[1],
                        high: candle[2],
                        low: candle[3],
                        close: candle[4],
                        volume: candle[5],
                    })
                    .collect();

                let latency = start.elapsed().as_millis() as u64;
                self.record_api_call(latency);

                // DEBUG: Record successful fetch (higher-level fetch_success recorded by monitor after storage)
                record_ohlcv_event(
                    "fetch_aggregate_complete",
                    Severity::Debug,
                    None,
                    Some(pool_address),
                    json!({
                        "pool_address": pool_address,
                        "api_endpoint": api_endpoint,
                        "aggregate": aggregate,
                        "data_points": data_points.len(),
                        "latency_ms": latency,
                    }),
                )
                .await;

                Ok(data_points)
            }
            Err(err) => {
                let is_rate_limited =
                    matches!(&err, ApiError::Network(NetworkError::RateLimited { .. }));
                let is_not_found = matches!(&err, ApiError::NotFound { .. })
                    || matches!(&err, ApiError::Network(NetworkError::HttpStatus { status, .. }) if *status == 404);
                let error_type = if is_rate_limited {
                    "rate_limit"
                } else if is_not_found {
                    "pool_not_found"
                } else {
                    "api_error"
                };
                let err_str = err.to_string();

                record_ohlcv_event(
                    "fetch_aggregate_error",
                    Severity::Error,
                    None,
                    Some(pool_address),
                    json!({
                        "pool_address": pool_address,
                        "api_endpoint": api_endpoint,
                        "aggregate": aggregate,
                        "error_type": error_type,
                        "error": err_str,
                    }),
                )
                .await;

                if is_rate_limited {
                    Err(OhlcvError::RateLimitExceeded)
                } else if is_not_found {
                    Err(OhlcvError::PoolNotFound(pool_address.to_string()))
                } else {
                    Err(OhlcvError::ApiError(err_str))
                }
            }
        }
    }

    /// Fetch the newest `limit` candles of `timeframe` from one chain-owned feed
    /// by token address.
    pub async fn fetch_from_feed(
        &self,
        feed: &CandleFeed,
        mint: &str,
        timeframe: Timeframe,
        limit: usize,
    ) -> OhlcvResult<Vec<Candle>> {
        let start = Instant::now();
        let interval = timeframe.as_str();

        record_ohlcv_event(
            &format!("{}_fetch_attempt", feed.label),
            Severity::Debug,
            Some(mint),
            None,
            json!({
                "mint": mint,
                "interval": interval,
                "limit": limit,
            }),
        )
        .await;

        let response = (feed.fetch)(mint.to_owned(), timeframe).await;
        match response {
            Ok(mut data_points) => {
                // Limit results
                if data_points.len() > limit {
                    let skip = data_points.len() - limit;
                    data_points = data_points.into_iter().skip(skip).collect();
                }

                let latency = start.elapsed().as_millis() as u64;
                self.record_api_call(latency);

                record_ohlcv_event(
                    &format!("{}_fetch_complete", feed.label),
                    Severity::Debug,
                    Some(mint),
                    None,
                    json!({
                        "mint": mint,
                        "interval": interval,
                        "data_points": data_points.len(),
                        "latency_ms": latency,
                    }),
                )
                .await;

                Ok(data_points)
            }
            Err(err) => {
                let latency = start.elapsed().as_millis() as u64;

                record_ohlcv_event(
                    &format!("{}_fetch_error", feed.label),
                    Severity::Error,
                    Some(mint),
                    None,
                    json!({
                        "mint": mint,
                        "interval": interval,
                        "error": err.to_string(),
                        "latency_ms": latency,
                    }),
                )
                .await;

                Err(OhlcvError::ApiError(err.to_string()))
            }
        }
    }

    /// Try the ScreenerBot data service. `None` on anything at all — switched
    /// off, signed out, refused, missed or timed out — so the caller falls back
    /// to the providers. The reason is published once by `data_server::access`.
    /// `before` (unix secs, exclusive) asks for the newest `limit` stored candles
    /// strictly older than it instead of the newest candles ending now.
    async fn fetch_from_screenerbot_server(
        &self,
        mint: &str,
        pool_address: &str,
        api_endpoint: &str,
        aggregate: u32,
        limit: usize,
        before: Option<i64>,
    ) -> Option<FetchResponse> {
        let tf = Timeframe::from_api_params(api_endpoint, aggregate)?.as_str();
        // `stateful=true` wraps the candle array (identical field names) with the
        // series state, which says whether the server is refreshing behind a stale
        // cached answer.
        let mut query = vec![
            ("mint", mint.to_string()),
            ("pool", pool_address.to_string()),
            ("timeframe", tf.to_string()),
            ("limit", limit.min(MAX_CANDLES_PER_REQUEST).to_string()),
            ("stateful", "true".to_string()),
        ];
        if let Some(before) = before {
            query.push(("before", before.to_string()));
        }
        let body = crate::data_server::get_json::<StatefulOhlcv>(
            crate::data_server::Surface::Ohlcv,
            self.chain,
            "/v1/ohlcv",
            &query,
        )
        .await?;
        Some(FetchResponse {
            server_refreshing: server_state_is_refreshing(&body.state),
            candles: body.candles,
            source: Some(CandleSource::DataServer),
        })
    }

    /// Fetch one timeframe with multi-source fallback: the Data Server, then, for a
    /// native-quoted pool only, the chain's candle feeds in order and GeckoTerminal.
    /// Returns the newest `limit` candles ending now, or with `before`
    /// (unix secs, exclusive) the newest `limit` candles strictly older than it.
    /// A candle feed serves only the newest candles, so a `before` request skips
    /// the feeds.
    pub async fn fetch_multi_source(
        &self,
        mint: &str,
        pool_address: &str,
        api_endpoint: &str,
        aggregate: u32,
        limit: usize,
        pool_is_native: bool,
        before: Option<i64>,
    ) -> OhlcvResult<FetchResponse> {
        // Try the self-hosted ScreenerBot OHLCV server first: it serves a shared
        // cache fast and warms itself, sparing the external providers' budgets. On
        // any miss/timeout/error we fall straight through to the providers below,
        // so this is purely an accelerator — never a hard dependency.
        if let Some(response) = self
            .fetch_from_screenerbot_server(
                mint,
                pool_address,
                api_endpoint,
                aggregate,
                limit,
                before,
            )
            .await
        {
            if !response.candles.is_empty() {
                return Ok(response);
            }
        }

        let Some(feeds) = fallback_feeds(pool_is_native, || self.candle_feeds(), before) else {
            record_ohlcv_event(
                "fallback_skipped_non_native_pool",
                Severity::Debug,
                Some(mint),
                Some(pool_address),
                json!({
                    "reason": "non-native pool; only the data server serves SOL candles",
                }),
            )
            .await;
            return Ok(FetchResponse::default());
        };

        if let Some(response) = self
            .fetch_feeds(&feeds, mint, pool_address, api_endpoint, aggregate, limit)
            .await
        {
            return Ok(response);
        }

        // GeckoTerminal's `before_timestamp` may include a candle stamped exactly
        // at the bound; one second earlier keeps the bound exclusive.
        let gecko_before = before.map(|ts| ts.saturating_sub(1));
        let candles = self
            .fetch_with_aggregate(pool_address, api_endpoint, aggregate, gecko_before, limit)
            .await?;
        Ok(FetchResponse {
            candles,
            server_refreshing: false,
            source: Some(CandleSource::GeckoTerminal),
        })
    }

    /// Try each candle feed in order and return the first non-empty answer. A
    /// feed that fails is recorded and the next one is tried; `None` when no
    /// feed answered with candles.
    async fn fetch_feeds(
        &self,
        feeds: &[CandleFeed],
        mint: &str,
        pool_address: &str,
        api_endpoint: &str,
        aggregate: u32,
        limit: usize,
    ) -> Option<FetchResponse> {
        let timeframe = Timeframe::from_api_params(api_endpoint, aggregate)?;
        for feed in feeds {
            match self.fetch_from_feed(feed, mint, timeframe, limit).await {
                Ok(candles) if !candles.is_empty() => {
                    return Some(FetchResponse {
                        candles,
                        server_refreshing: false,
                        source: Some(CandleSource::Feed(feed.label)),
                    })
                }
                Ok(_) => {
                    // Empty result, try the next source
                }
                Err(e) => {
                    record_ohlcv_event(
                        &format!("{}_fallback", feed.label),
                        Severity::Warn,
                        Some(mint),
                        Some(pool_address),
                        json!({
                            "reason": format!("{} failed, falling back to GeckoTerminal", feed.label),
                            "error": e.to_string(),
                        }),
                    )
                    .await;
                }
            }
        }
        None
    }

    /// Fetch OHLCV data immediately (bypasses queue, use for critical requests only)
    pub async fn fetch_immediate(
        &self,
        pool_address: &str,
        timeframe: Timeframe,
        before_timestamp: Option<i64>,
        limit: usize,
    ) -> OhlcvResult<Vec<Candle>> {
        // Record request attempt for local metrics
        self.record_attempt();

        let start = Instant::now();
        let limit_clamped = limit.min(MAX_CANDLES_PER_REQUEST) as u32;

        // DEBUG: Record fetch attempt
        record_ohlcv_event(
            "fetch_attempt",
            Severity::Debug,
            None,
            Some(pool_address),
            json!({
                "pool_address": pool_address,
                "timeframe": timeframe.to_string(),
                "before_timestamp": before_timestamp,
                "limit": limit_clamped,
            }),
        )
        .await;

        let response = self
            .api_manager
            .geckoterminal
            .fetch_ohlcv(
                crate::chains::adapter_for(self.chain).market_data_network(),
                pool_address,
                timeframe.to_api_param(),
                None,
                Some(limit_clamped),
                Some("token"),
                before_timestamp,
                None,
            )
            .await;

        match response {
            Ok(ohlcv) => {
                let data_points: Vec<Candle> = ohlcv
                    .ohlcv_list
                    .into_iter()
                    .map(|candle| Candle {
                        timestamp: candle[0] as i64,
                        open: candle[1],
                        high: candle[2],
                        low: candle[3],
                        close: candle[4],
                        volume: candle[5],
                    })
                    .collect();

                let latency = start.elapsed().as_millis() as u64;
                self.record_api_call(latency);

                // DEBUG: Record successful fetch (higher-level events recorded by monitor after storage)
                record_ohlcv_event(
                    "fetch_immediate_complete",
                    Severity::Debug,
                    None,
                    Some(pool_address),
                    json!({
                        "pool_address": pool_address,
                        "timeframe": timeframe.to_string(),
                        "data_points": data_points.len(),
                        "latency_ms": latency,
                        "before_timestamp": before_timestamp,
                    }),
                )
                .await;

                Ok(data_points)
            }
            Err(err) => {
                let is_rate_limited =
                    matches!(&err, ApiError::Network(NetworkError::RateLimited { .. }));
                let is_not_found = matches!(&err, ApiError::NotFound { .. })
                    || matches!(&err, ApiError::Network(NetworkError::HttpStatus { status, .. }) if *status == 404);
                let error_type = if is_rate_limited {
                    "rate_limit"
                } else if is_not_found {
                    "pool_not_found"
                } else {
                    "api_error"
                };
                let err_str = err.to_string();

                // ERROR: Record fetch failure
                record_ohlcv_event(
                    "fetch_error",
                    Severity::Error,
                    None,
                    Some(pool_address),
                    json!({
                        "pool_address": pool_address,
                        "timeframe": timeframe.to_string(),
                        "error_type": error_type,
                        "error": err_str,
                        "before_timestamp": before_timestamp,
                    }),
                )
                .await;

                if is_rate_limited {
                    Err(OhlcvError::RateLimitExceeded)
                } else if is_not_found {
                    Err(OhlcvError::PoolNotFound(pool_address.to_string()))
                } else {
                    Err(OhlcvError::ApiError(err_str))
                }
            }
        }
    }

    /// Fetch calls recorded so far, the denominator of [`Self::average_latency_ms`].
    pub(super) fn calls_recorded(&self) -> u64 {
        *self
            .api_calls_count
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// Get average latency in milliseconds
    pub fn average_latency_ms(&self) -> f64 {
        let total_latency = *self
            .total_latency_ms
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let api_calls = *self
            .api_calls_count
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        if api_calls == 0 {
            return 0.0;
        }

        (total_latency as f64) / (api_calls as f64)
    }

    /// Get API calls per minute
    pub fn calls_per_minute(&self) -> f64 {
        let mut history = self
            .request_history
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        Self::prune_history(&mut history, now);

        history.len() as f64
    }

    /// Get queue size
    pub fn queue_size(&self) -> usize {
        self.request_queue
            .lock()
            .map(|queue| queue.len())
            .unwrap_or_default()
    }

    // ==================== Private Methods ====================

    fn prune_history(history: &mut VecDeque<Instant>, now: Instant) {
        while let Some(&front) = history.front() {
            if now.duration_since(front) >= RATE_LIMIT_WINDOW {
                history.pop_front();
            } else {
                break;
            }
        }
    }

    fn queue_request(
        &self,
        mint: String,
        pool_address: String,
        timeframe: Timeframe,
        priority: Priority,
        before_timestamp: Option<i64>,
        limit: usize,
    ) -> OhlcvResult<()> {
        let mut queue = self
            .request_queue
            .lock()
            .map_err(|e| OhlcvError::ApiError(format!("Lock error: {e}")))?;

        queue.push(FetchRequest {
            mint,
            pool_address,
            timeframe,
            priority,
            before_timestamp,
            limit,
            requested_at: Instant::now(),
        });

        Ok(())
    }

    async fn process_queue(&self) -> OhlcvResult<Vec<Candle>> {
        // Get next request from queue
        let request = {
            let mut queue = self
                .request_queue
                .lock()
                .map_err(|e| OhlcvError::ApiError(format!("Lock error: {e}")))?;

            queue.pop()
        };

        if let Some(req) = request {
            self.fetch_immediate(
                &req.pool_address,
                req.timeframe,
                req.before_timestamp,
                req.limit,
            )
            .await
        } else {
            Ok(Vec::new())
        }
    }

    fn record_attempt(&self) {
        if let Ok(mut history) = self.request_history.lock() {
            let now = Instant::now();
            history.push_back(now);
            Self::prune_history(&mut history, now);
        }
    }

    fn record_api_call(&self, latency_ms: u64) {
        if let Ok(mut count) = self.api_calls_count.lock() {
            *count += 1;
        }

        if let Ok(mut total_latency) = self.total_latency_ms.lock() {
            *total_latency += latency_ms;
        }
    }
}

/// The candle feeds that can serve a request. A feed serves only the newest
/// candles, so a `before` request has none.
/// The candle feeds a fetch may fall back to after the Data Server, or `None` when every
/// fallback is skipped. Only the data server converts a non-native pool's candles to SOL.
/// GeckoTerminal answers in the pool's quote token (`currency=token`) and a candle feed
/// answers by token address for whatever pool it picks, so on a USD-quoted or unknown-quote
/// series pool either would write foreign-unit candles into the SOL series. `feeds` is
/// consulted only for a native pool.
fn fallback_feeds(
    pool_is_native: bool,
    feeds: impl FnOnce() -> Vec<CandleFeed>,
    before: Option<i64>,
) -> Option<Vec<CandleFeed>> {
    pool_is_native.then(|| feeds_serving(feeds(), before))
}

fn feeds_serving(feeds: Vec<CandleFeed>, before: Option<i64>) -> Vec<CandleFeed> {
    if before.is_some() {
        Vec::new()
    } else {
        feeds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ohlcvs::feeds::CandleFeedFn;
    use std::sync::atomic::{AtomicUsize, Ordering};

    type FeedFuture = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<Candle>, crate::apis::Error>> + Send>,
    >;

    static FAILING_CALLS: AtomicUsize = AtomicUsize::new(0);
    static EMPTY_CALLS: AtomicUsize = AtomicUsize::new(0);
    static ANSWERING_CALLS: AtomicUsize = AtomicUsize::new(0);
    static UNREACHED_CALLS: AtomicUsize = AtomicUsize::new(0);

    fn failing_feed(_mint: String, _timeframe: Timeframe) -> FeedFuture {
        FAILING_CALLS.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Err(crate::apis::Error::Disabled {
                provider: "failing".to_string(),
            })
        })
    }

    fn empty_feed(_mint: String, _timeframe: Timeframe) -> FeedFuture {
        EMPTY_CALLS.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(Vec::new()) })
    }

    fn answering_feed(_mint: String, timeframe: Timeframe) -> FeedFuture {
        ANSWERING_CALLS.fetch_add(1, Ordering::SeqCst);
        let timestamp = timeframe.to_seconds();
        Box::pin(async move {
            Ok(vec![Candle {
                timestamp,
                open: 1.0,
                high: 2.0,
                low: 0.5,
                close: 1.5,
                volume: 3.0,
            }])
        })
    }

    fn unreached_feed(_mint: String, _timeframe: Timeframe) -> FeedFuture {
        UNREACHED_CALLS.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(Vec::new()) })
    }

    fn feed(label: &'static str, fetch: CandleFeedFn) -> CandleFeed {
        CandleFeed { label, fetch }
    }

    fn test_fetcher() -> OhlcvFetcher {
        let _ = crate::config::utils::CONFIG
            .get_or_init(|| std::sync::RwLock::new(crate::config::Config::default()));
        OhlcvFetcher::new(ChainId::Solana)
    }

    #[test]
    fn a_before_request_is_served_by_no_feed() {
        let feeds = vec![feed("answering", answering_feed)];
        assert_eq!(feeds_serving(feeds.clone(), None).len(), 1);
        assert!(feeds_serving(feeds, Some(1_700_000_000)).is_empty());
    }

    #[tokio::test]
    async fn feeds_are_tried_in_order_until_one_answers_with_candles() {
        let fetcher = test_fetcher();
        let feeds = [
            feed("failing", failing_feed),
            feed("empty", empty_feed),
            feed("answering", answering_feed),
            feed("unreached", unreached_feed),
        ];

        let response = fetcher
            .fetch_feeds(&feeds, "mint", "pool", "minute", 5, 10)
            .await
            .expect("the answering feed serves the request");

        assert_eq!(response.candles.len(), 1);
        assert_eq!(
            response.candles[0].timestamp,
            Timeframe::Minute5.to_seconds()
        );
        assert_eq!(response.source, Some(CandleSource::Feed("answering")));
        assert!(!response.server_refreshing);
        assert_eq!(FAILING_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(EMPTY_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(ANSWERING_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(UNREACHED_CALLS.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn no_feed_or_an_unknown_timeframe_answers_nothing() {
        let fetcher = test_fetcher();
        assert!(fetcher
            .fetch_feeds(&[], "mint", "pool", "minute", 5, 10)
            .await
            .is_none());
        assert!(fetcher
            .fetch_feeds(
                &[feed("unreached", unreached_feed)],
                "mint",
                "pool",
                "minute",
                30,
                10
            )
            .await
            .is_none());
        assert_eq!(UNREACHED_CALLS.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_non_native_pool_is_never_served_by_a_fallback() {
        let fetcher = test_fetcher();
        let response = fetcher
            .fetch_multi_source("mint", "usd-pool", "minute", 5, 10, false, None)
            .await
            .expect("a skipped fallback is an empty answer, not an error");
        assert!(response.candles.is_empty());
        assert_eq!(response.source, None);

        // The feed list is never consulted for a non-native pool.
        assert!(fallback_feeds(
            false,
            || panic!("feeds consulted for a non-native pool"),
            None
        )
        .is_none());
        let native = fallback_feeds(true, || vec![feed("answering", answering_feed)], None);
        assert_eq!(native.map(|feeds| feeds.len()), Some(1));
        let before = fallback_feeds(true, || vec![feed("answering", answering_feed)], Some(1));
        assert_eq!(before.map(|feeds| feeds.len()), Some(0));
    }

    #[test]
    fn stateful_body_parses_and_only_refresh_states_mark_refreshing() {
        let body: StatefulOhlcv = serde_json::from_str(
            r#"{"candles":[{"timestamp":60,"open":1.0,"high":2.0,"low":0.5,"close":1.5,"volume":3.0}],"state":"refreshing"}"#,
        )
        .unwrap();
        assert_eq!(body.candles.len(), 1);
        assert_eq!(body.candles[0].timestamp, 60);
        assert!(server_state_is_refreshing(&body.state));
        assert!(server_state_is_refreshing("pending"));
        for state in ["ready", "empty", "unavailable", "", "something_new"] {
            assert!(!server_state_is_refreshing(state), "{state}");
        }
        let without_state: StatefulOhlcv = serde_json::from_str(r#"{"candles":[]}"#).unwrap();
        assert!(!server_state_is_refreshing(&without_state.state));
    }
}
