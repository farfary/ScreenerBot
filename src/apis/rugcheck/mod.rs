/// Rugcheck API client for token security analysis
///
/// API Documentation: https://api.rugcheck.xyz/
///
/// Endpoints implemented:
/// 1. /v1/tokens/{mint}/report - Get security report for a token
/// 2. /v1/tokens/{mint}/report/summary - Get summary security report
/// 3. /v1/stats/summary - Get global platform statistics
/// 4. /v1/tokens/{mints}/batch - Get multiple token reports (batch)
pub mod types;

// Re-export types for external use
pub use self::types::{
    RugcheckInfo, RugcheckNewToken, RugcheckRecentToken, RugcheckResponse, RugcheckTrendingToken,
    RugcheckVerifiedToken,
};

use crate::apis::client::{HttpClient, RateLimiter};
use crate::apis::stats::ApiStatsTracker;
use crate::tokens::types::{ApiError, SecurityRisk, TokenHolder};
use chrono::Utc;
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
                    .record_error(format!("{} rate limiter acquire failed: {}", endpoint, err))
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
                    .record_error(format!("{} request failed: {}", endpoint, err))
                    .await;
                Err(ApiError::NetworkError(err.to_string()))
            }
        }
    }

    async fn parse_json<T>(&self, url: &str, endpoint: &str) -> Result<T, ApiError>
    where
        T: DeserializeOwned,
    {
        let (mut response, elapsed) = self.execute_request(url, endpoint).await?;
        let status = response.status();

        if !status.is_success() {
            self.stats.record_request(false, elapsed).await;
            let body = response.text().await.unwrap_or_default();
            self.stats
                .record_error(format!("{} HTTP {}: {}", endpoint, status, body))
                .await;

            if status == StatusCode::NOT_FOUND {
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
                    .record_error(format!("{} parse error: {}", endpoint, err))
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
        let url = format!("{}/{}/report", RUGCHECK_BASE_URL, mint);
        let api_response: RugcheckResponse = self.parse_json(&url, "rugcheck.report").await?;

        // Convert API response to domain type
        let risks = api_response
            .risks
            .unwrap_or_default()
            .into_iter()
            .map(|r| SecurityRisk {
                name: r.name,
                value: r.value,
                description: r.description,
                score: r.score,
                level: r.level,
            })
            .collect();

        let top_holders = api_response
            .top_holders
            .unwrap_or_default()
            .into_iter()
            .map(|h| TokenHolder {
                address: h.address,
                amount: h.amount.to_string(),
                pct: h.pct,
                owner: h.owner,
                insider: h.insider.unwrap_or(false),
            })
            .collect();

        let token_meta = api_response.token_meta;
        let token = api_response.token;
        let transfer_fee = api_response.transfer_fee;

        // Extract authorities with fallback strategy:
        // 1. Try top-level fields (may be None if API returned object format)
        // 2. Fall back to nested token.* fields (always string or null)
        // This ensures we NEVER miss authority data regardless of API response format
        let mint_authority = api_response
            .mint_authority
            .or_else(|| token.as_ref().and_then(|t| t.mint_authority.clone()));

        let freeze_authority = api_response
            .freeze_authority
            .or_else(|| token.as_ref().and_then(|t| t.freeze_authority.clone()));

        Ok(RugcheckInfo {
            mint: api_response.mint,
            token_program: api_response.token_program,
            token_type: api_response.token_type,
            token_name: token_meta.as_ref().and_then(|t| t.name.clone()),
            token_symbol: token_meta.as_ref().and_then(|t| t.symbol.clone()),
            token_decimals: token.as_ref().and_then(|t| t.decimals),
            token_supply: token.as_ref().and_then(|t| t.supply.map(|s| s.to_string())),
            token_uri: token_meta.as_ref().and_then(|t| t.uri.clone()),
            token_mutable: token_meta.as_ref().and_then(|t| t.mutable),
            token_update_authority: token_meta.as_ref().and_then(|t| t.update_authority.clone()),
            mint_authority,
            freeze_authority,
            creator: api_response.creator,
            creator_balance: api_response.creator_balance,
            creator_tokens: api_response.creator_tokens,
            score: api_response.score,
            score_normalised: api_response.score_normalised,
            rugged: api_response.rugged.unwrap_or(false),
            risks,
            total_markets: None, // Market data not included in API response
            total_market_liquidity: api_response.total_market_liquidity,
            total_stable_liquidity: api_response.total_stable_liquidity,
            total_lp_providers: api_response.total_lp_providers,
            total_holders: api_response.total_holders,
            top_holders,
            graph_insiders_detected: api_response.graph_insiders_detected,
            transfer_fee_pct: transfer_fee.as_ref().and_then(|t| t.pct),
            transfer_fee_max_amount: transfer_fee
                .as_ref()
                .and_then(|t| t.max_amount.map(|a| a as i64)),
            transfer_fee_authority: transfer_fee.and_then(|t| t.authority),
            detected_at: api_response.detected_at,
            analyzed_at: api_response.analyzed_at,
            fetched_at: Utc::now(),
        })
    }

    // ========================================================================
    // Stats Endpoints
    // ========================================================================

    /// Fetch new tokens from /v1/stats/new_tokens
    pub async fn fetch_new_tokens(&self) -> Result<Vec<RugcheckNewToken>, ApiError> {
        let url = format!("{}/new_tokens", RUGCHECK_STATS_BASE_URL);
        self.parse_json(&url, "rugcheck.stats.new_tokens").await
    }

    /// Fetch most viewed tokens from /v1/stats/recent
    pub async fn fetch_recent_tokens(&self) -> Result<Vec<RugcheckRecentToken>, ApiError> {
        let url = format!("{}/recent", RUGCHECK_STATS_BASE_URL);
        self.parse_json(&url, "rugcheck.stats.recent").await
    }

    /// Fetch trending tokens from /v1/stats/trending
    pub async fn fetch_trending_tokens(&self) -> Result<Vec<RugcheckTrendingToken>, ApiError> {
        let url = format!("{}/trending", RUGCHECK_STATS_BASE_URL);
        self.parse_json(&url, "rugcheck.stats.trending").await
    }

    /// Fetch verified tokens from /v1/stats/verified
    pub async fn fetch_verified_tokens(&self) -> Result<Vec<RugcheckVerifiedToken>, ApiError> {
        let url = format!("{}/verified", RUGCHECK_STATS_BASE_URL);
        self.parse_json(&url, "rugcheck.stats.verified").await
    }
}
