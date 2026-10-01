//! OHLCV fetcher — retrieves candlestick data from multiple sources.
//!
//! Sources (in priority order):
//! 0. ScreenerBot self-hosted OHLCV server — fast shared cache, tried FIRST
//! 1. SolanaTracker — uses token address, credit-based, high quality
//! 2. GeckoTerminal — uses pool address, rate-limited 30/min, free

use crate::apis::{get_api_manager, ApiManager, Error as ApiError};
use crate::errors::NetworkError;
use crate::events::{record_ohlcv_event, Severity};
use crate::ohlcvs::types::{Candle, OhlcvError, OhlcvResult, Priority, Timeframe};
use serde::Deserialize;
use serde_json::json;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
pub(crate) const MAX_CANDLES_PER_REQUEST: usize = 1000;

/// Candles returned by [`OhlcvFetcher::fetch_multi_source`].
#[derive(Debug, Clone, Default)]
pub struct FetchResponse {
    pub candles: Vec<Candle>,
    /// The Data Server answered from a cached series it is refreshing behind the
    /// response (`refreshing` or `pending`), so a newer page is expected shortly.
    /// Always false for provider answers.
    pub server_refreshing: bool,
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
    api_manager: Arc<ApiManager>,
    request_history: Arc<Mutex<VecDeque<Instant>>>,
    request_queue: Arc<Mutex<BinaryHeap<FetchRequest>>>,
    api_calls_count: Arc<Mutex<u64>>,
    total_latency_ms: Arc<Mutex<u64>>,
}

impl OhlcvFetcher {
    pub fn new() -> Self {
        Self {
            api_manager: get_api_manager(),
            request_history: Arc::new(Mutex::new(VecDeque::new())),
            request_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            api_calls_count: Arc::new(Mutex::new(0)),
            total_latency_ms: Arc::new(Mutex::new(0)),
        }
    }

    /// Check if SolanaTracker source is available (enabled with API key)
    pub fn has_solana_tracker(&self) -> bool {
        self.api_manager.solana_tracker.is_enabled()
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
                crate::chains::adapter().market_data_network(),
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

    /// Map GeckoTerminal API params to SolanaTracker interval string
    fn gt_to_st_interval(api_endpoint: &str, aggregate: u32) -> Option<&'static str> {
        match (api_endpoint, aggregate) {
            ("minute", 1) => Some("1m"),
            ("minute", 5) => Some("5m"),
            ("minute", 15) => Some("15m"),
            ("minute", 30) => Some("30m"),
            ("hour", 1) => Some("1h"),
            ("hour", 4) => Some("4h"),
            ("hour", 12) => Some("12h"),
            ("day", 1) => Some("1d"),
            _ => None,
        }
    }

    /// Fetch OHLCV from SolanaTracker using token mint address
    pub async fn fetch_from_solana_tracker(
        &self,
        mint: &str,
        interval: &str,
        limit: usize,
    ) -> OhlcvResult<Vec<Candle>> {
        if !self.api_manager.solana_tracker.is_enabled() {
            return Err(OhlcvError::ApiError("SolanaTracker not enabled".to_owned()));
        }

        let start = Instant::now();

        record_ohlcv_event(
            "solanatracker_fetch_attempt",
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

        let response = self
            .api_manager
            .solana_tracker
            .fetch_ohlcv(mint, interval, "sol", None, None)
            .await;

        match response {
            Ok(ohlcv) => {
                let mut data_points: Vec<Candle> = ohlcv
                    .candles
                    .into_iter()
                    .map(|c| Candle {
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

                // Limit results
                if data_points.len() > limit {
                    let skip = data_points.len() - limit;
                    data_points = data_points.into_iter().skip(skip).collect();
                }

                let latency = start.elapsed().as_millis() as u64;
                self.record_api_call(latency);

                record_ohlcv_event(
                    "solanatracker_fetch_complete",
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
                    "solanatracker_fetch_error",
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

    /// Map the GeckoTerminal (endpoint, aggregate) pair back to the canonical
    /// timeframe string the ScreenerBot server expects.
    fn server_timeframe(api_endpoint: &str, aggregate: u32) -> Option<&'static str> {
        match (api_endpoint, aggregate) {
            ("minute", 1) => Some("1m"),
            ("minute", 5) => Some("5m"),
            ("minute", 15) => Some("15m"),
            ("hour", 1) => Some("1h"),
            ("hour", 4) => Some("4h"),
            ("hour", 12) => Some("12h"),
            ("day", 1) => Some("1d"),
            _ => None,
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
        let tf = Self::server_timeframe(api_endpoint, aggregate)?;
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
            "/v1/ohlcv",
            &query,
        )
        .await?;
        Some(FetchResponse {
            server_refreshing: server_state_is_refreshing(&body.state),
            candles: body.candles,
        })
    }

    /// Fetch one timeframe with multi-source fallback: the Data Server, then
    /// SolanaTracker, then GeckoTerminal (SOL-quoted pools only). Returns the
    /// newest `limit` candles ending now, or with `before` (unix secs, exclusive)
    /// the newest `limit` candles strictly older than it. SolanaTracker serves
    /// only the newest candles, so a `before` request skips it.
    pub async fn fetch_multi_source(
        &self,
        mint: &str,
        pool_address: &str,
        api_endpoint: &str,
        aggregate: u32,
        limit: usize,
        pool_is_sol: bool,
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

        // Try SolanaTracker first (uses token address, better data)
        if before.is_none() && self.api_manager.solana_tracker.is_enabled() {
            if let Some(interval) = Self::gt_to_st_interval(api_endpoint, aggregate) {
                match self.fetch_from_solana_tracker(mint, interval, limit).await {
                    Ok(candles) if !candles.is_empty() => {
                        return Ok(FetchResponse {
                            candles,
                            server_refreshing: false,
                        })
                    }
                    Ok(_) => {
                        // Empty result, fall through to GeckoTerminal
                    }
                    Err(e) => {
                        record_ohlcv_event(
                            "solanatracker_fallback",
                            Severity::Warn,
                            Some(mint),
                            Some(pool_address),
                            json!({
                                "reason": "SolanaTracker failed, falling back to GeckoTerminal",
                                "error": e.to_string(),
                            }),
                        )
                        .await;
                    }
                }
            }
        }

        // Fallback to GeckoTerminal (uses pool address). GeckoTerminal returns
        // candles in the pool's QUOTE token (`currency=token`), so it is SOL only
        // for a wSOL-quoted pool. On a USD-quoted pool it returns USD candles that
        // would poison the SOL-denominated series (candles are keyed by
        // (mint,timeframe,ts) with no pool, so one USD candle corrupts the chart).
        // Skip Gecko entirely for non-SOL pools; the SOL-forcing sources above
        // (data server, SolanaTracker) are the only valid path there.
        if !pool_is_sol {
            record_ohlcv_event(
                "gecko_skipped_non_sol_pool",
                Severity::Debug,
                Some(mint),
                Some(pool_address),
                json!({
                    "reason": "USD-quoted pool; GeckoTerminal would return USD candles",
                }),
            )
            .await;
            return Ok(FetchResponse::default());
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
        })
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
                crate::chains::adapter().market_data_network(),
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

impl Default for OhlcvFetcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
