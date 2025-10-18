/// Rugcheck API client for token security analysis
///
/// API Documentation: https://api.rugcheck.xyz/
///
/// Endpoints implemented:
/// 1. /v1/tokens/{mint}/report - Get security report for a token
/// 2. /v1/tokens/{mint}/report/summary - Get summary security report
/// 3. /v1/stats/summary - Get global platform statistics
/// 4. /v1/tokens/{mints}/batch - Get multiple token reports (batch)
use super::client::{HttpClient, RateLimiter};
use super::rugcheck_types::*;
use super::stats::ApiStatsTracker;
use crate::tokens_new::types::{ApiError, RugcheckHolder, RugcheckInfo, RugcheckRisk};
use chrono::Utc;
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

    /// Fetch security report for a token
    pub async fn fetch_report(&self, mint: &str) -> Result<RugcheckInfo, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        self.rate_limiter.acquire().await;
        let start = Instant::now();

        let url = format!("{}/{}/report", RUGCHECK_BASE_URL, mint);

        let response = self
            .http_client
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                let error = ApiError::NetworkError(e.to_string());
                self.stats.record_cache_miss();
                error
            })?;

        let elapsed = start.elapsed().as_millis() as f64;

        if !response.status().is_success() {
            self.stats.record_request(false, elapsed).await;
            if response.status() == 404 {
                return Err(ApiError::NotFound);
            }
            return Err(ApiError::InvalidResponse(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let api_response: RugcheckResponse = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        // Convert API response to domain type
        let risks = api_response
            .risks
            .unwrap_or_default()
            .into_iter()
            .map(|r| RugcheckRisk {
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
            .map(|h| RugcheckHolder {
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
            mint_authority: api_response.mint_authority,
            freeze_authority: api_response.freeze_authority,
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
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        self.rate_limiter.acquire().await;
        let start = Instant::now();

        let url = format!("{}/new_tokens", RUGCHECK_STATS_BASE_URL);

        let response = self
            .http_client
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                let error = ApiError::NetworkError(e.to_string());
                self.stats.record_cache_miss();
                error
            })?;

        let elapsed = start.elapsed().as_millis() as f64;

        if !response.status().is_success() {
            self.stats.record_request(false, elapsed).await;
            return Err(ApiError::InvalidResponse(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let tokens: Vec<RugcheckNewToken> = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        Ok(tokens)
    }

    /// Fetch most viewed tokens from /v1/stats/recent
    pub async fn fetch_recent_tokens(&self) -> Result<Vec<RugcheckRecentToken>, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        self.rate_limiter.acquire().await;
        let start = Instant::now();

        let url = format!("{}/recent", RUGCHECK_STATS_BASE_URL);

        let response = self
            .http_client
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                let error = ApiError::NetworkError(e.to_string());
                self.stats.record_cache_miss();
                error
            })?;

        let elapsed = start.elapsed().as_millis() as f64;

        if !response.status().is_success() {
            self.stats.record_request(false, elapsed).await;
            return Err(ApiError::InvalidResponse(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let tokens: Vec<RugcheckRecentToken> = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        Ok(tokens)
    }

    /// Fetch trending tokens from /v1/stats/trending
    pub async fn fetch_trending_tokens(&self) -> Result<Vec<RugcheckTrendingToken>, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        self.rate_limiter.acquire().await;
        let start = Instant::now();

        let url = format!("{}/trending", RUGCHECK_STATS_BASE_URL);

        let response = self
            .http_client
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                let error = ApiError::NetworkError(e.to_string());
                self.stats.record_cache_miss();
                error
            })?;

        let elapsed = start.elapsed().as_millis() as f64;

        if !response.status().is_success() {
            self.stats.record_request(false, elapsed).await;
            return Err(ApiError::InvalidResponse(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let tokens: Vec<RugcheckTrendingToken> = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        Ok(tokens)
    }

    /// Fetch verified tokens from /v1/stats/verified
    pub async fn fetch_verified_tokens(&self) -> Result<Vec<RugcheckVerifiedToken>, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        self.rate_limiter.acquire().await;
        let start = Instant::now();

        let url = format!("{}/verified", RUGCHECK_STATS_BASE_URL);

        let response = self
            .http_client
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                let error = ApiError::NetworkError(e.to_string());
                self.stats.record_cache_miss();
                error
            })?;

        let elapsed = start.elapsed().as_millis() as f64;

        if !response.status().is_success() {
            self.stats.record_request(false, elapsed).await;
            return Err(ApiError::InvalidResponse(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let tokens: Vec<RugcheckVerifiedToken> = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        Ok(tokens)
    }
}
