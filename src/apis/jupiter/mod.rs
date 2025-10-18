/// Jupiter API client for token discovery
///
/// API Documentation: https://station.jup.ag/docs/apis/general-api
///
/// Endpoints implemented:
/// 1. /tokens/v2/recent - Recent tokens
/// 2. /tokens/v2/toporganicscore/{interval} - Top organic score tokens
/// 3. /tokens/v2/toptraded/{interval} - Top traded tokens
/// 4. /tokens/v2/toptrending/{interval} - Top trending tokens

pub mod types;

use crate::apis::client::HttpClient;
use crate::apis::stats::ApiStatsTracker;
use self::types::JupiterToken;
use crate::tokens::types::ApiError;
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// API CONFIGURATION - Hardcoded for Jupiter API
// ============================================================================

const JUPITER_BASE_URL: &str = "https://lite-api.jup.ag/tokens/v2";

/// Request timeout - Jupiter API is fast, 15s is sufficient
const TIMEOUT_SECS: u64 = 15;

/// Default limit for paginated endpoints
const DEFAULT_LIMIT: usize = 100;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

pub struct JupiterClient {
    http_client: HttpClient,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl JupiterClient {
    pub fn new(enabled: bool) -> Result<Self, String> {
        let http_client = HttpClient::new(TIMEOUT_SECS)?;
        let stats = Arc::new(ApiStatsTracker::new());

        Ok(Self {
            http_client,
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

    /// Fetch recent tokens from /tokens/v2/recent
    pub async fn fetch_recent_tokens(&self) -> Result<Vec<JupiterToken>, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        let start = Instant::now();
        let url = format!("{}/recent", JUPITER_BASE_URL);

        let response = self
            .http_client
            .client()
            .get(&url)
            .header("Accept", "application/json")
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

        let tokens: Vec<JupiterToken> = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        Ok(tokens)
    }

    /// Fetch top organic score tokens for given interval
    ///
    /// # Arguments
    /// * `interval` - Time interval: "5m", "1h", "6h", "24h"
    /// * `limit` - Number of results (default: 100)
    pub async fn fetch_top_organic_score(
        &self,
        interval: &str,
        limit: Option<usize>,
    ) -> Result<Vec<JupiterToken>, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        let start = Instant::now();
        let limit = limit.unwrap_or(DEFAULT_LIMIT);
        let url = format!(
            "{}/toporganicscore/{}?limit={}",
            JUPITER_BASE_URL, interval, limit
        );

        let response = self
            .http_client
            .client()
            .get(&url)
            .header("Accept", "application/json")
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

        let tokens: Vec<JupiterToken> = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        Ok(tokens)
    }

    /// Fetch top traded tokens for given interval
    ///
    /// # Arguments
    /// * `interval` - Time interval: "5m", "1h", "6h", "24h"
    /// * `limit` - Number of results (default: 100)
    pub async fn fetch_top_traded(
        &self,
        interval: &str,
        limit: Option<usize>,
    ) -> Result<Vec<JupiterToken>, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        let start = Instant::now();
        let limit = limit.unwrap_or(DEFAULT_LIMIT);
        let url = format!(
            "{}/toptraded/{}?limit={}",
            JUPITER_BASE_URL, interval, limit
        );

        let response = self
            .http_client
            .client()
            .get(&url)
            .header("Accept", "application/json")
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

        let tokens: Vec<JupiterToken> = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        Ok(tokens)
    }

    /// Fetch top trending tokens for given interval
    ///
    /// # Arguments
    /// * `interval` - Time interval: "5m", "1h", "6h", "24h"
    /// * `limit` - Number of results (default: 100)
    pub async fn fetch_top_trending(
        &self,
        interval: &str,
        limit: Option<usize>,
    ) -> Result<Vec<JupiterToken>, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        let start = Instant::now();
        let limit = limit.unwrap_or(DEFAULT_LIMIT);
        let url = format!(
            "{}/toptrending/{}?limit={}",
            JUPITER_BASE_URL, interval, limit
        );

        let response = self
            .http_client
            .client()
            .get(&url)
            .header("Accept", "application/json")
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

        let tokens: Vec<JupiterToken> = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        Ok(tokens)
    }
}
