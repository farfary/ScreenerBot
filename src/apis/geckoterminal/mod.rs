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
//! 12. /networks/{network}/pools/{pool_address}/trades - Recent pool trades

pub mod types;
mod endpoints;

// Re-export types for external use
pub use self::types::{
    GeckoTerminalDexesResponse, GeckoTerminalPool, GeckoTerminalRecentlyUpdatedResponse,
    GeckoTerminalResponse, GeckoTerminalTokenInfoResponse, GeckoTerminalTokensMultiResponse,
    GeckoTerminalTradesResponse,
};

use crate::apis::client::RateLimiter;
use crate::apis::stats::ApiStatsTracker;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// API CONFIGURATION - Hardcoded for GeckoTerminal API
// ============================================================================

pub(crate) const GECKOTERMINAL_BASE_URL: &str = "https://api.geckoterminal.com/api/v2";

/// Default network for Solana operations
pub(crate) const DEFAULT_NETWORK: &str = "solana";

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
}

impl GeckoTerminalClient {
    pub fn new(enabled: bool, rate_limit: usize, timeout_seconds: u64) -> Result<Self, String> {
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
    ) -> Result<Self, String> {
        if timeout_seconds == 0 {
            return Err("Timeout must be greater than zero".to_owned());
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
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub async fn get_stats(&self) -> crate::apis::stats::ApiStats {
        self.stats.get_stats().await
    }

    fn ensure_enabled(&self, endpoint: &str) -> Result<(), String> {
        if self.enabled {
            Ok(())
        } else {
            Err(format!(
                "GeckoTerminal client disabled via configuration (endpoint={})",
                endpoint
            ))
        }
    }

    async fn execute_request(
        &self,
        endpoint: &str,
        builder: reqwest::RequestBuilder,
    ) -> Result<(reqwest::Response, f64), String> {
        self.ensure_enabled(endpoint)?;

        let guard = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {e}"))?;

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
                Err(format!("Request failed: {err}"))
            }
        }
    }

    pub(crate) async fn get_json<T>(
        &self,
        endpoint: &str,
        builder: reqwest::RequestBuilder,
    ) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        let (mut response, elapsed) = self.execute_request(endpoint, builder).await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.stats.record_request(false, elapsed).await;
            self.stats
                .record_error_with_event(
                    "GeckoTerminal",
                    endpoint,
                    format!("HTTP {status}: {body}"),
                )
                .await;
            // Exponential backoff on 429: 10s, 20s, 40s, 60s max
            if status.as_u16() == 429 {
                let count = self.consecutive_429s.fetch_add(1, Ordering::Relaxed) + 1;
                let backoff_secs = (10u64 * (1u64 << (count - 1).min(3))).min(60);
                tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            }
            return Err(format!("GeckoTerminal API error {status}: {body}"));
        }

        match response.json::<T>().await {
            Ok(value) => {
                self.stats.record_request(true, elapsed).await;
                self.consecutive_429s.store(0, Ordering::Relaxed);
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
                Err(format!("Failed to parse response: {err}"))
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
