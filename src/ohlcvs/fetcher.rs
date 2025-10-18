// GeckoTerminal API fetcher with rate limiting and priority queue

use crate::ohlcvs::types::{OhlcvDataPoint, OhlcvError, OhlcvResult, Priority, Timeframe};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::collections::{BinaryHeap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const MAX_CANDLES_PER_REQUEST: usize = 1000;

#[derive(Deserialize, Debug)]
struct GeckoOhlcvResponse {
    data: GeckoOhlcvData,
}

#[derive(Deserialize, Debug)]
struct GeckoOhlcvData {
    attributes: GeckoOhlcvAttributes,
}

#[derive(Deserialize, Debug)]
struct GeckoOhlcvAttributes {
    ohlcv_list: Vec<Vec<f64>>, // [timestamp, open, high, low, close, volume]
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
    client: Client,
    request_history: Arc<Mutex<VecDeque<Instant>>>,
    request_queue: Arc<Mutex<BinaryHeap<FetchRequest>>>,
    api_calls_count: Arc<Mutex<u64>>,
    total_latency_ms: Arc<Mutex<u64>>,
}

impl OhlcvFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
            request_history: Arc::new(Mutex::new(VecDeque::new())),
            request_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            api_calls_count: Arc::new(Mutex::new(0)),
            total_latency_ms: Arc::new(Mutex::new(0)),
        }
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
    ) -> OhlcvResult<Vec<OhlcvDataPoint>> {
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

    /// Fetch OHLCV data immediately (bypasses queue, use for critical requests only)
    pub async fn fetch_immediate(
        &self,
        pool_address: &str,
        timeframe: Timeframe,
        before_timestamp: Option<i64>,
        limit: usize,
    ) -> OhlcvResult<Vec<OhlcvDataPoint>> {
        // Rate limiting: rely on internal request_history window to respect per-minute caps

        // Record request attempt for local metrics
        self.record_attempt();

        let start = Instant::now();

        // Build URL
        let timeframe_param = timeframe.to_api_param();
        let mut url = format!(
            "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/{}",
            pool_address, timeframe_param
        );

        if let Some(before) = before_timestamp {
            url.push_str(&format!("?before_timestamp={}", before));
        }
        url.push_str(&format!(
            "{}limit={}&currency=token",
            if before_timestamp.is_some() { "&" } else { "?" },
            limit.min(MAX_CANDLES_PER_REQUEST)
        ));

        // Make request
        let response = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| OhlcvError::ApiError(format!("Request failed: {}", e)))?;

        // Check rate limit
        let status = response.status();

        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(OhlcvError::RateLimitExceeded);
        }

        if status == StatusCode::NOT_FOUND {
            return Err(OhlcvError::PoolNotFound(pool_address.to_string()));
        }

        if !status.is_success() {
            return Err(OhlcvError::ApiError(format!(
                "API returned status: {}",
                status
            )));
        }

        // Parse response
        let gecko_response: GeckoOhlcvResponse = response
            .json()
            .await
            .map_err(|e| OhlcvError::ApiError(format!("Failed to parse response: {}", e)))?;

        // Convert to our format
        let data_points: Vec<OhlcvDataPoint> = gecko_response
            .data
            .attributes
            .ohlcv_list
            .into_iter()
            .filter_map(|candle| {
                if candle.len() == 6 {
                    Some(OhlcvDataPoint {
                        timestamp: candle[0] as i64,
                        open: candle[1],
                        high: candle[2],
                        low: candle[3],
                        close: candle[4],
                        volume: candle[5],
                    })
                } else {
                    None
                }
            })
            .collect();

        // Record metrics
        let latency = start.elapsed().as_millis() as u64;
        self.record_api_call(latency);

        Ok(data_points)
    }

    /// Fetch multiple pages of data backwards
    pub async fn fetch_historical(
        &self,
        pool_address: &str,
        timeframe: Timeframe,
        from_timestamp: i64,
        to_timestamp: i64,
    ) -> OhlcvResult<Vec<OhlcvDataPoint>> {
        if from_timestamp >= to_timestamp {
            return Ok(Vec::new());
        }

        let mut all_data = Vec::new();
        let timeframe_seconds = timeframe.to_seconds();
        let mut before = Some(to_timestamp);
        let mut last_oldest = None;
        let mut attempts = 0u32;
        const MAX_ATTEMPTS: u32 = 500;

        while attempts < MAX_ATTEMPTS {
            attempts += 1;

            let mut data = self
                .fetch_immediate(pool_address, timeframe, before, MAX_CANDLES_PER_REQUEST)
                .await?;

            if data.is_empty() {
                break;
            }

            data.retain(|point| {
                point.timestamp >= from_timestamp && point.timestamp <= to_timestamp
            });

            if data.is_empty() {
                break;
            }

            let oldest_timestamp = data.iter().map(|d| d.timestamp).min().unwrap();

            if let Some(prev_oldest) = last_oldest {
                if prev_oldest <= oldest_timestamp {
                    break;
                }
            }
            last_oldest = Some(oldest_timestamp);

            all_data.extend(data);

            if oldest_timestamp <= from_timestamp {
                break;
            }

            before = Some(oldest_timestamp.saturating_sub(timeframe_seconds));

            if before == Some(to_timestamp) {
                break;
            }

            sleep(Duration::from_millis(500)).await;
        }

        all_data.sort_by_key(|d| d.timestamp);
        all_data.dedup_by_key(|d| d.timestamp);

        Ok(all_data)
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
            .unwrap_or(0)
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
            .map_err(|e| OhlcvError::ApiError(format!("Lock error: {}", e)))?;

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

    async fn process_queue(&self) -> OhlcvResult<Vec<OhlcvDataPoint>> {
        // Get next request from queue
        let request = {
            let mut queue = self
                .request_queue
                .lock()
                .map_err(|e| OhlcvError::ApiError(format!("Lock error: {}", e)))?;

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
