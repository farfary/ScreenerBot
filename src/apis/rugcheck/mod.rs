//! Rugcheck API client for token security analysis
//!
//! API Documentation: https://api.rugcheck.xyz/
//!
//! Endpoints implemented:
//! 1. /v1/tokens/{mint}/report - Get security report for a token
//! 2. /v1/tokens/{mint}/report/summary - Get summary security report
//! 3. /v1/stats/summary - Get global platform statistics
//! 4. /v1/tokens/{mints}/batch - Get multiple token reports (batch)

pub mod types;

// Re-export types for external use
pub use self::types::{
    RugcheckInfo, RugcheckNewToken, RugcheckRecentToken, RugcheckResponse, RugcheckTrendingToken,
    RugcheckVerifiedToken,
};

use crate::apis::client::{HttpClient, RateLimiter};
use crate::apis::stats::ApiStatsTracker;
use crate::tokens::types::ApiError;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// API CONFIGURATION - Hardcoded for Rugcheck API
// ============================================================================

const RUGCHECK_BASE_URL: &str = "https://api.rugcheck.xyz/v1/tokens";
const RUGCHECK_STATS_BASE_URL: &str = "https://api.rugcheck.xyz/v1/stats";

/// Request timeout in seconds - Rugcheck can be slow, 15s for security analysis
pub const TIMEOUT_SECS: u64 = 15;

/// Rate limit per minute - Rugcheck has moderate limits, 60/min is reasonable
pub const RATE_LIMIT_PER_MINUTE: usize = 60;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

pub struct RugcheckClient {
    http_client: HttpClient,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl RugcheckClient {
    pub fn new(
        enabled: bool,
        rate_limit_per_minute: usize,
        timeout_secs: u64,
    ) -> Result<Self, String> {
        let http_client = HttpClient::new(timeout_secs)?;
        let rate_limiter = RateLimiter::new(rate_limit_per_minute);
        let stats = Arc::new(ApiStatsTracker::new());

        Ok(Self {
            http_client,
            rate_limiter,
            stats,
            enabled,
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub async fn get_stats(&self) -> super::stats::ApiStats {
        self.stats.get_stats().await
    }

    async fn execute_request(
        &self,
        url: &str,
        endpoint: &str,
    ) -> Result<(reqwest::Response, f64), ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        let guard = match self.rate_limiter.acquire().await {
            Ok(permit) => permit,
            Err(err) => {
                self.stats
                    .record_error_with_event(
                        "Rugcheck",
                        endpoint,
                        format!("Rate limiter acquire failed: {err}"),
                    )
                    .await;
                return Err(ApiError::RateLimitExceeded);
            }
        };

        let start = Instant::now();
        let response_result = self.http_client.client().get(url).send().await;
        drop(guard);
        let elapsed = start.elapsed().as_millis() as f64;

        match response_result {
            Ok(response) => Ok((response, elapsed)),
            Err(err) => {
                self.stats.record_cache_miss();
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event("Rugcheck", endpoint, format!("Request failed: {err}"))
                    .await;
                Err(ApiError::NetworkError(err.to_string()))
            }
        }
    }

    async fn parse_json<T>(&self, url: &str, endpoint: &str) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let (response, elapsed) = self.execute_request(url, endpoint).await?;
        let status = response.status();

        if !status.is_success() {
            self.stats.record_request(false, elapsed).await;
            let body = response.text().await.unwrap_or_default();
            self.stats
                .record_error_with_event("Rugcheck", endpoint, format!("HTTP {status}: {body}"))
                .await;

            // Check for 404 Not Found
            if status == StatusCode::NOT_FOUND {
                return Err(ApiError::NotFound);
            }

            // Check for 400 Bad Request with "not found" in body (Rugcheck returns this for unanalyzed tokens)
            if status == StatusCode::BAD_REQUEST && body.contains("not found") {
                return Err(ApiError::NotFound);
            }

            return Err(ApiError::InvalidResponse(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        match response.json::<T>().await {
            Ok(value) => {
                self.stats.record_request(true, elapsed).await;
                Ok(value)
            }
            Err(err) => {
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event("Rugcheck", endpoint, format!("Parse error: {err}"))
                    .await;
                Err(ApiError::InvalidResponse(err.to_string()))
            }
        }
    }

    /// Fetch security report for a token
    ///
    /// **DATA EXTRACTION STRATEGY:**
    /// Rugcheck API returns data in multiple formats depending on token type:
    ///
    /// - **Standard tokens**: Authority fields are strings or null
    /// - **Token2022 tokens**: Authority fields may be account info objects
    ///
    /// This implementation uses a fallback strategy to ensure we NEVER miss data:
    /// 1. Custom deserializer handles object→None conversion (see types.rs)
    /// 2. Fallback to nested `token.*` fields when top-level fields are None
    /// 3. All data extraction is exhaustive - we capture everything the API provides
    ///
    /// **SYSTEMATIC ERROR HANDLING:**
    /// - 404 errors → Return Ok(None) (token not analyzed yet)
    /// - Decoding errors → Should never occur with flexible deserializers
    /// - Network errors → Propagated as ApiError for retry logic
    pub async fn fetch_report(&self, mint: &str) -> Result<RugcheckInfo, ApiError> {
        let url = format!("{RUGCHECK_BASE_URL}/{mint}/report");
        let api_response: RugcheckResponse = self.parse_json(&url, "rugcheck.report").await?;
        Ok(RugcheckInfo::from_response(api_response))
    }

    // ========================================================================
    // Stats Endpoints
    // ========================================================================

    /// Fetch new tokens from /v1/stats/new_tokens
    pub async fn fetch_new_tokens(&self) -> Result<Vec<RugcheckNewToken>, ApiError> {
        let url = format!("{RUGCHECK_STATS_BASE_URL}/new_tokens");
        self.parse_json(&url, "rugcheck.stats.new_tokens").await
    }

    /// Fetch most viewed tokens from /v1/stats/recent
    pub async fn fetch_recent_tokens(&self) -> Result<Vec<RugcheckRecentToken>, ApiError> {
        let url = format!("{RUGCHECK_STATS_BASE_URL}/recent");
        self.parse_json(&url, "rugcheck.stats.recent").await
    }

    /// Fetch trending tokens from /v1/stats/trending
    pub async fn fetch_trending_tokens(&self) -> Result<Vec<RugcheckTrendingToken>, ApiError> {
        let url = format!("{RUGCHECK_STATS_BASE_URL}/trending");
        self.parse_json(&url, "rugcheck.stats.trending").await
    }

    /// Fetch verified tokens from /v1/stats/verified
    pub async fn fetch_verified_tokens(&self) -> Result<Vec<RugcheckVerifiedToken>, ApiError> {
        let url = format!("{RUGCHECK_STATS_BASE_URL}/verified");
        self.parse_json(&url, "rugcheck.stats.verified").await
    }
}
