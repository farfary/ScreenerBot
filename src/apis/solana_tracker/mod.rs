//! SolanaTracker API client
//!
//! API Documentation: https://docs.solanatracker.io
//!
//! Endpoints implemented:
//! 1. /chart/{token} - OHLCV candlestick data
//! 2. /tokens/{token} - Token information (pools, events, risk)
//! 3. /credits - Check remaining API credits
//! 4. /search - Search tokens

pub mod types;

use crate::apis::client::RateLimiter;
use crate::apis::stats::ApiStatsTracker;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const BASE_URL: &str = "https://data.solanatracker.io";
pub const TIMEOUT_SECS: u64 = 15;
/// Credit-based API, but keep a reasonable request ceiling
pub const RATE_LIMIT_PER_MINUTE: usize = 60;

pub struct SolanaTrackerClient {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    timeout: Duration,
    enabled: bool,
    api_key: String,
    /// Cached remaining API credits (-1 = unknown)
    remaining_credits: AtomicI64,
}

impl SolanaTrackerClient {
    pub fn new(
        enabled: bool,
        api_key: String,
        rate_limit: usize,
        timeout_seconds: u64,
    ) -> Result<Self, String> {
        Self::with_base_url(
            enabled,
            api_key,
            rate_limit,
            timeout_seconds,
            BASE_URL.to_owned(),
        )
    }

    /// Construct a client with an explicit API base URL (used when an
    /// OHLCV-specific endpoint override is configured).
    pub fn with_base_url(
        enabled: bool,
        api_key: String,
        rate_limit: usize,
        timeout_seconds: u64,
        base_url: String,
    ) -> Result<Self, String> {
        if timeout_seconds == 0 {
            return Err("Timeout must be greater than zero".to_owned());
        }

        let url = if base_url.is_empty() {
            BASE_URL.to_owned()
        } else {
            base_url.trim_end_matches('/').to_owned()
        };

        Ok(Self {
            client: crate::net::client(),
            base_url: url,
            rate_limiter: RateLimiter::new(rate_limit),
            stats: Arc::new(ApiStatsTracker::new()),
            timeout: Duration::from_secs(timeout_seconds),
            enabled: enabled && !api_key.is_empty(),
            api_key,
            remaining_credits: AtomicI64::new(-1),
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn remaining_credits(&self) -> i64 {
        self.remaining_credits.load(Ordering::Relaxed)
    }

    pub async fn get_stats(&self) -> crate::apis::stats::ApiStats {
        self.stats.get_stats().await
    }

    fn ensure_enabled(&self, endpoint: &str) -> Result<(), String> {
        if !self.enabled {
            return Err(format!(
                "SolanaTracker client disabled (endpoint={})",
                endpoint
            ));
        }
        // Check if we know credits are exhausted
        let credits = self.remaining_credits.load(Ordering::Relaxed);
        if credits == 0 {
            return Err(format!(
                "SolanaTracker credits exhausted (endpoint={})",
                endpoint
            ));
        }
        Ok(())
    }

    async fn get_json<T: DeserializeOwned>(&self, endpoint: &str, path: &str) -> Result<T, String> {
        self.ensure_enabled(endpoint)?;

        let guard = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {e}"))?;

        let url = format!("{}{}", self.base_url, path);
        let start = Instant::now();

        let response_result = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .timeout(self.timeout)
            .send()
            .await;

        drop(guard);
        let elapsed = start.elapsed().as_millis() as f64;

        let response = match response_result {
            Ok(r) => r,
            Err(err) => {
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event(
                        "SolanaTracker",
                        endpoint,
                        format!("Request failed: {err}"),
                    )
                    .await;
                return Err(format!("Request failed: {err}"));
            }
        };

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.stats.record_request(false, elapsed).await;

            // A 403 "insufficient credits" is an account-level, persistent
            // condition — not a per-token failure. Latch remaining_credits to 0 so
            // `ensure_enabled` short-circuits every subsequent call instead of
            // hammering the API (and spamming OHLCV logs) with the same 403 for
            // the thousands of pool-less tokens that fall back to SolanaTracker.
            // Credits reset on the next successful `fetch_credits()` (e.g. after a
            // top-up) or on restart.
            if status == reqwest::StatusCode::FORBIDDEN
                && body.to_lowercase().contains("credit")
                && self.remaining_credits.swap(0, Ordering::Relaxed) != 0
            {
                crate::logger::warning(
                    crate::logger::LogTag::Tokens,
                    "SolanaTracker credits exhausted (HTTP 403) - pausing SolanaTracker \
                     calls until credits are refreshed",
                );
            }

            self.stats
                .record_error_with_event(
                    "SolanaTracker",
                    endpoint,
                    format!("HTTP {status}: {body}"),
                )
                .await;
            return Err(format!("SolanaTracker API error {status}: {body}"));
        }

        match response.json::<T>().await {
            Ok(value) => {
                self.stats.record_request(true, elapsed).await;
                Ok(value)
            }
            Err(err) => {
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event(
                        "SolanaTracker",
                        endpoint,
                        format!("Parse error: {err}"),
                    )
                    .await;
                Err(format!("Failed to parse response: {err}"))
            }
        }
    }

    /// Fetch OHLCV candlestick data for a token.
    ///
    /// `interval` — one of: 1s, 5s, 15s, 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1mn
    /// `currency` — one of: usd, sol, eur
    pub async fn fetch_ohlcv(
        &self,
        token_address: &str,
        interval: &str,
        currency: &str,
        time_from: Option<i64>,
        time_to: Option<i64>,
    ) -> Result<types::OhlcvResponse, String> {
        let mut path = format!(
            "/chart/{}?type={}&currency={}",
            token_address, interval, currency
        );
        if let Some(from) = time_from {
            path.push_str(&format!("&time_from={}", from));
        }
        if let Some(to) = time_to {
            path.push_str(&format!("&time_to={}", to));
        }

        self.get_json("ohlcv", &path).await
    }

    /// Fetch token information (pools, events, risk).
    pub async fn fetch_token_info(
        &self,
        token_address: &str,
    ) -> Result<types::TokenInfoResponse, String> {
        let path = format!("/tokens/{}", token_address);
        self.get_json("token_info", &path).await
    }

    /// Check remaining API credits and cache the result.
    pub async fn fetch_credits(&self) -> Result<types::CreditsResponse, String> {
        let result: Result<types::CreditsResponse, String> =
            self.get_json("credits", "/credits").await;
        if let Ok(ref credits) = result {
            self.remaining_credits
                .store(credits.credits, Ordering::Relaxed);
        }
        result
    }

    /// Search tokens by name, symbol, or mint address.
    pub async fn search_tokens(&self, query: &str) -> Result<Vec<types::SearchResult>, String> {
        // Build URL via reqwest::Url to get proper query-parameter encoding
        let base = format!("{}/search", self.base_url);
        let url = reqwest::Url::parse_with_params(&base, &[("query", query)])
            .map_err(|e| format!("Failed to build search URL: {e}"))?;
        // get_json expects a path (with query string), so extract it from the parsed URL
        let path = format!("{}?{}", url.path(), url.query().unwrap_or_default());
        self.get_json("search", &path).await
    }
}
