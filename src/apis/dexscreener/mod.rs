//! Complete DexScreener API client with ALL available endpoints
//!
//! API Documentation: https://docs.dexscreener.com/api/reference
//!
//! Endpoints implemented (verified working):
//! 1. /token-pairs/v1/{chainId}/{tokenAddress} - PRIMARY: Get all pools for a token
//! 2. /tokens/v1/{chainId}/{tokenAddresses} - Get pools for up to 30 tokens (batch)
//! 3. /latest/dex/pairs/{chainId}/{pairId} - Get single pair by chain/address
//! 4. /latest/dex/search?q={query} - Search pairs
//! 5. /token-profiles/latest/v1 - Get latest token profiles
//! 6. /token-boosts/latest/v1 - Get latest boosted tokens
//! 7. /token-boosts/top/v1 - Get top boosted tokens  
//! 8. /orders/v1/{chainId}/{tokenAddress} - Get orders for a token

mod endpoints;
pub mod types;

// Re-export types for external use
pub use self::types::{
    ChainInfo, DexScreenerPairRaw, DexScreenerPool, PairResponse, PairsResponse, TokenBoostLatest,
    TokenBoostTop, TokenInfo, TokenOrder, TokenProfile,
};

use crate::apis::client::RateLimiter;
use crate::apis::stats::ApiStatsTracker;
use crate::apis::Error;
use crate::errors::{DataError, NetworkError};
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// API CONFIGURATION - Hardcoded for DexScreener API
// ============================================================================

pub(crate) const DEXSCREENER_BASE_URL: &str = "https://api.dexscreener.com";

/// The active chain's identifier in DexScreener's API.
pub(crate) fn default_chain_id() -> &'static str {
    crate::chains::adapter().market_data_network()
}

/// Maximum tokens per batch request
pub(crate) const MAX_TOKENS_PER_REQUEST: usize = 30;

/// Request timeout in seconds - DexScreener is fast, 10s is sufficient
pub const TIMEOUT_SECS: u64 = 10;

/// Rate limits per endpoint (requests per minute)
pub const RATE_LIMIT_TOKEN_POOLS_PER_MINUTE: usize = 300;
pub const RATE_LIMIT_TOKEN_BATCH_PER_MINUTE: usize = 300;
pub const RATE_LIMIT_PAIR_LOOKUP_PER_MINUTE: usize = 300;
pub const RATE_LIMIT_SEARCH_PER_MINUTE: usize = 300;
pub const RATE_LIMIT_LATEST_PROFILES_PER_MINUTE: usize = 60;
pub const RATE_LIMIT_LATEST_BOOSTS_PER_MINUTE: usize = 60;
pub const RATE_LIMIT_TOP_BOOSTS_PER_MINUTE: usize = 60;
pub const RATE_LIMIT_TOKEN_ORDERS_PER_MINUTE: usize = 60;
pub const RATE_LIMIT_TOKEN_INFO_PER_MINUTE: usize = 60;
pub const RATE_LIMIT_SUPPORTED_CHAINS_PER_MINUTE: usize = 60;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// Complete DexScreener API client
pub struct DexScreenerClient {
    pub(crate) client: Client,
    pub(crate) stats: Arc<ApiStatsTracker>,
    timeout: Duration,
    enabled: bool,
    pub(crate) limiter_token_pools: RateLimiter,
    pub(crate) limiter_token_batch: RateLimiter,
    pub(crate) limiter_pair_lookup: RateLimiter,
    pub(crate) limiter_search: RateLimiter,
    pub(crate) limiter_latest_profiles: RateLimiter,
    pub(crate) limiter_latest_boosts: RateLimiter,
    pub(crate) limiter_top_boosts: RateLimiter,
    pub(crate) limiter_token_orders: RateLimiter,
    pub(crate) limiter_token_info: RateLimiter,
    pub(crate) limiter_supported_chains: RateLimiter,
}

impl DexScreenerClient {
    pub fn new(enabled: bool, timeout_seconds: u64) -> Result<Self, Error> {
        if timeout_seconds == 0 {
            return Err(DataError::ValidationError {
                field: "timeout_seconds".to_owned(),
                value: "0".to_owned(),
                reason: "must be greater than zero".to_owned(),
            }
            .into());
        }

        Ok(Self {
            client: crate::net::client(),
            stats: Arc::new(ApiStatsTracker::new()),
            timeout: Duration::from_secs(timeout_seconds),
            enabled,
            limiter_token_pools: RateLimiter::new(RATE_LIMIT_TOKEN_POOLS_PER_MINUTE),
            limiter_token_batch: RateLimiter::new(RATE_LIMIT_TOKEN_BATCH_PER_MINUTE),
            limiter_pair_lookup: RateLimiter::new(RATE_LIMIT_PAIR_LOOKUP_PER_MINUTE),
            limiter_search: RateLimiter::new(RATE_LIMIT_SEARCH_PER_MINUTE),
            limiter_latest_profiles: RateLimiter::new(RATE_LIMIT_LATEST_PROFILES_PER_MINUTE),
            limiter_latest_boosts: RateLimiter::new(RATE_LIMIT_LATEST_BOOSTS_PER_MINUTE),
            limiter_top_boosts: RateLimiter::new(RATE_LIMIT_TOP_BOOSTS_PER_MINUTE),
            limiter_token_orders: RateLimiter::new(RATE_LIMIT_TOKEN_ORDERS_PER_MINUTE),
            limiter_token_info: RateLimiter::new(RATE_LIMIT_TOKEN_INFO_PER_MINUTE),
            limiter_supported_chains: RateLimiter::new(RATE_LIMIT_SUPPORTED_CHAINS_PER_MINUTE),
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get API stats (placeholder - DexScreener uses direct HTTP without stats tracking)
    pub async fn get_stats(&self) -> crate::apis::stats::ApiStats {
        self.stats.get_stats().await
    }

    fn ensure_enabled(&self, _endpoint: &str) -> Result<(), Error> {
        if self.enabled {
            Ok(())
        } else {
            Err(Error::Disabled {
                provider: "DexScreener".to_owned(),
            })
        }
    }

    pub(crate) async fn execute_request(
        &self,
        endpoint: &str,
        builder: reqwest::RequestBuilder,
        limiter: &RateLimiter,
    ) -> Result<(reqwest::Response, f64), Error> {
        self.ensure_enabled(endpoint)?;

        let guard = limiter.acquire().await?;

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
                        "DexScreener",
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
        limiter: &RateLimiter,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let (response, elapsed) = self.execute_request(endpoint, builder, limiter).await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.stats.record_request(false, elapsed).await;
            self.stats
                .record_error_with_event("DexScreener", endpoint, format!("HTTP {status}: {body}"))
                .await;
            // Simple 429 backoff to avoid hammering when rate limited
            if status.as_u16() == 429 {
                tokio::time::sleep(Duration::from_secs(5)).await;
                return Err(NetworkError::RateLimited {
                    endpoint: endpoint.to_owned(),
                    retry_after_ms: Some(5000),
                }
                .into());
            }
            return Err(NetworkError::HttpStatus {
                endpoint: endpoint.to_owned(),
                status: status.as_u16(),
                body: Some(body),
            }
            .into());
        }

        match response.json::<T>().await {
            Ok(value) => {
                self.stats.record_request(true, elapsed).await;
                Ok(value)
            }
            Err(err) => {
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event("DexScreener", endpoint, format!("Parse error: {err}"))
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
