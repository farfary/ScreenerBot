// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! GeckoTerminal API client
//!
//! API Documentation: https://www.geckoterminal.com/dex-api
//!
//! Endpoints implemented:
//! 1. /networks/{network}/tokens/{token}/pools - Get all pools for a token (primary)
//! 2. /networks/{network}/trending_pools - Trending pools per network
//! 3. /networks/{network}/pools - Top pools per network
//! 4. /networks/{network}/pools/{address} - Pool details by address
//! 5. /networks/{network}/pools/multi/{addresses} - Multiple pools at once
//! 6. /networks/{network}/pools/{pool}/ohlcv/{timeframe} - OHLCV data
//! 7. /networks/{network}/dexes - Supported DEX list
//! 8. /networks/{network}/new_pools - Newly listed pools
//! 9. /networks/{network}/tokens/multi/{addresses} - Multiple token metadata
//! 10. /networks/{network}/tokens/{address}/info - Token metadata
//! 11. /tokens/info_recently_updated - Recent token updates (global)

mod endpoints;
pub mod types;

// Re-export types for external use
pub use self::types::{
    GeckoTerminalDexesResponse, GeckoTerminalPool, GeckoTerminalRecentlyUpdatedResponse,
    GeckoTerminalResponse, GeckoTerminalTokenInfoResponse, GeckoTerminalTokensMultiResponse,
};

use crate::apis::client::RateLimiter;
use crate::apis::stats::ApiStatsTracker;
use crate::apis::Error;
use crate::errors::{DataError, NetworkError};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ============================================================================
// API CONFIGURATION - Hardcoded for GeckoTerminal API
// ============================================================================

pub(crate) const GECKOTERMINAL_BASE_URL: &str = "https://api.geckoterminal.com/api/v2";

/// The active chain's network slug in GeckoTerminal's API.
pub(crate) fn default_network() -> &'static str {
    crate::chains::adapter().market_data_network()
}

/// Maximum page number for trending pools pagination
pub(crate) const MAX_TRENDING_PAGE: u32 = 10;

/// Request timeout in seconds - GeckoTerminal can have latency spikes, 10s is safe
pub const TIMEOUT_SECS: u64 = 10;

/// Rate limit per minute - GeckoTerminal has strict limits, 30/min is safe
pub const RATE_LIMIT_PER_MINUTE: usize = 30;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// GeckoTerminal API client with rate limiting and stats tracking
pub struct GeckoTerminalClient {
    pub(crate) client: Client,
    pub(crate) base_url: String,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    timeout: Duration,
    enabled: bool,
    /// Consecutive 429 error count for exponential backoff
    consecutive_429s: AtomicU32,
    /// When set, no request may be SENT before this instant. Set on a 429 and
    /// enforced inside `execute_request` while holding the rate-limiter guard, so
    /// the backoff slows the outbound rate for every caller rather than only the
    /// one that saw the 429. A plain mutex: it is never held across an await.
    penalty_until: Mutex<Option<Instant>>,
}

impl GeckoTerminalClient {
    pub fn new(enabled: bool, rate_limit: usize, timeout_seconds: u64) -> Result<Self, Error> {
        Self::with_base_url(
            enabled,
            rate_limit,
            timeout_seconds,
            GECKOTERMINAL_BASE_URL.to_owned(),
        )
    }

    /// Construct a client with an explicit API base URL (used when an
    /// OHLCV-specific or tokens-specific endpoint override is configured).
    pub fn with_base_url(
        enabled: bool,
        rate_limit: usize,
        timeout_seconds: u64,
        base_url: String,
    ) -> Result<Self, Error> {
        if timeout_seconds == 0 {
            return Err(DataError::ValidationError {
                field: "timeout_seconds".to_owned(),
                value: "0".to_owned(),
                reason: "must be greater than zero".to_owned(),
            }
            .into());
        }

        let url = if base_url.is_empty() {
            GECKOTERMINAL_BASE_URL.to_owned()
        } else {
            base_url.trim_end_matches('/').to_owned()
        };

        Ok(Self {
            client: crate::net::client(),
            base_url: url,
            rate_limiter: RateLimiter::new(rate_limit),
            stats: Arc::new(ApiStatsTracker::new()),
            timeout: Duration::from_secs(timeout_seconds),
            enabled,
            consecutive_429s: AtomicU32::new(0),
            penalty_until: Mutex::new(None),
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub async fn get_stats(&self) -> crate::apis::stats::ApiStats {
        self.stats.get_stats().await
    }

    /// Time left in the 429 penalty window, `None` when no penalty is active. A caller that
    /// can defer its request (background candle fallback) checks this instead of queueing
    /// behind the penalty on the single request permit.
    pub fn penalty_remaining(&self) -> Option<Duration> {
        let until = (*self.penalty())?;
        until
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
    }

    /// Open or extend the shared penalty window after a 429: 10 s, 20 s, 40 s, then 60 s for
    /// consecutive 429s. An active window is never shortened.
    pub(crate) fn record_rate_limited(&self) {
        let count = self.consecutive_429s.fetch_add(1, Ordering::Relaxed) + 1;
        let backoff_secs = (10u64 * (1u64 << (count - 1).min(3))).min(60);
        let until = Instant::now() + Duration::from_secs(backoff_secs);
        let mut penalty = self.penalty();
        if penalty.map_or(true, |existing| until > existing) {
            *penalty = Some(until);
        }
    }

    /// Close the penalty window and reset the 429 streak after a successful answer.
    fn record_success(&self) {
        self.consecutive_429s.store(0, Ordering::Relaxed);
        *self.penalty() = None;
    }

    fn penalty(&self) -> std::sync::MutexGuard<'_, Option<Instant>> {
        self.penalty_until
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn ensure_enabled(&self, _endpoint: &str) -> Result<(), Error> {
        if self.enabled {
            Ok(())
        } else {
            Err(Error::Disabled {
                provider: "GeckoTerminal".to_owned(),
            })
        }
    }

    async fn execute_request(
        &self,
        endpoint: &str,
        builder: reqwest::RequestBuilder,
    ) -> Result<(reqwest::Response, f64), Error> {
        self.ensure_enabled(endpoint)?;

        let guard = self.rate_limiter.acquire().await?;

        // Honor any active 429 penalty BEFORE sending, while still holding the
        // limiter guard so every caller serializes behind the backoff window.
        if let Some(left) = self.penalty_remaining() {
            tokio::time::sleep(left).await;
        }

        let start = Instant::now();
        let response_result = builder.timeout(self.timeout).send().await;
        drop(guard);
        let elapsed = start.elapsed().as_millis() as f64;

        match response_result {
            Ok(response) => Ok((response, elapsed)),
            Err(err) => {
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event(
                        "GeckoTerminal",
                        endpoint,
                        format!("Request failed: {err}"),
                    )
                    .await;
                Err(NetworkError::RequestFailed {
                    endpoint: endpoint.to_owned(),
                    detail: err.to_string(),
                }
                .into())
            }
        }
    }

    pub(crate) async fn get_json<T>(
        &self,
        endpoint: &str,
        builder: reqwest::RequestBuilder,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let (response, elapsed) = self.execute_request(endpoint, builder).await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.stats.record_request(false, elapsed).await;
            self.stats
                .record_error_with_event(
                    "GeckoTerminal",
                    endpoint,
                    format!(
                        "HTTP {status}: {}",
                        crate::errors::summarize_http_body(&body)
                    ),
                )
                .await;
            // A 429 opens the shared penalty window enforced in `execute_request`, so
            // concurrent callers stop firing every min_interval until the limit recovers.
            if status.as_u16() == 429 {
                self.record_rate_limited();
                return Err(NetworkError::RateLimited {
                    endpoint: endpoint.to_owned(),
                    retry_after_ms: None,
                }
                .into());
            }
            return Err(NetworkError::http_status(
                endpoint.to_owned(),
                status.as_u16(),
                Some(body),
            )
            .into());
        }

        match response.json::<T>().await {
            Ok(value) => {
                self.stats.record_request(true, elapsed).await;
                self.record_success();
                Ok(value)
            }
            Err(err) => {
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event(
                        "GeckoTerminal",
                        endpoint,
                        format!("Parse error: {err}"),
                    )
                    .await;
                Err(DataError::ParseError {
                    data_type: endpoint.to_owned(),
                    error: err.to_string(),
                }
                .into())
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub coingecko_coin_id: Option<String>,
}

/// OHLCV response containing candle data
#[derive(Debug, Clone)]
pub struct OhlcvResponse {
    pub ohlcv_list: Vec<[f64; 6]>,
    pub base_token: TokenInfo,
    pub quote_token: TokenInfo,
}

impl OhlcvResponse {
    pub fn len(&self) -> usize {
        self.ohlcv_list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ohlcv_list.is_empty()
    }

    pub fn get_candle(&self, index: usize) -> Option<&[f64; 6]> {
        self.ohlcv_list.get(index)
    }

    pub fn latest(&self) -> Option<&[f64; 6]> {
        self.ohlcv_list.first()
    }

    pub fn timestamps(&self) -> Vec<i64> {
        self.ohlcv_list.iter().map(|c| c[0] as i64).collect()
    }

    pub fn close_prices(&self) -> Vec<f64> {
        self.ohlcv_list.iter().map(|c| c[4]).collect()
    }

    pub fn volumes(&self) -> Vec<f64> {
        self.ohlcv_list.iter().map(|c| c[5]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_429_opens_a_penalty_window_and_a_success_closes_it() {
        let client = GeckoTerminalClient::new(true, RATE_LIMIT_PER_MINUTE, TIMEOUT_SECS).unwrap();
        assert_eq!(client.penalty_remaining(), None);

        client.record_rate_limited();
        let first = client.penalty_remaining().expect("a 429 opens the window");
        assert!(first > Duration::from_secs(9) && first <= Duration::from_secs(10));

        // A second 429 in a row doubles the window; it is never shortened.
        client.record_rate_limited();
        let second = client.penalty_remaining().expect("the window stays open");
        assert!(second > Duration::from_secs(19) && second <= Duration::from_secs(20));

        client.record_success();
        assert_eq!(client.penalty_remaining(), None);
        // The streak restarts: the next 429 opens the shortest window again.
        client.record_rate_limited();
        let restarted = client
            .penalty_remaining()
            .expect("a new 429 opens the window");
        assert!(restarted <= Duration::from_secs(10));
    }
}
