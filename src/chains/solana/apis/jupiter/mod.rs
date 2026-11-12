// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Jupiter API client for token discovery
//!
//! API Documentation: https://station.jup.ag/docs/apis/general-api
//!
//! Endpoints implemented:
//! 1. /tokens/v2/recent - Recent tokens
//! 2. /tokens/v2/toporganicscore/{interval} - Top organic score tokens
//! 3. /tokens/v2/toptraded/{interval} - Top traded tokens
//! 4. /tokens/v2/toptrending/{interval} - Top trending tokens

pub mod sources;
#[cfg(test)]
mod tests;
pub mod throttle;
pub mod types;

use self::types::JupiterToken;
use crate::apis::client::HttpClient;
use crate::apis::stats::ApiStatsTracker;
use crate::apis::Error;
use crate::errors::{source_chain, DataError, NetworkError};
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
    pub fn new(enabled: bool) -> Result<Self, Error> {
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

    pub async fn get_stats(&self) -> crate::apis::stats::ApiStats {
        self.stats.get_stats().await
    }

    /// Fetch recent tokens from /tokens/v2/recent
    pub async fn fetch_recent_tokens(&self) -> Result<Vec<JupiterToken>, Error> {
        self.fetch_list(format!("{JUPITER_BASE_URL}/recent")).await
    }

    /// Fetch top organic score tokens for `interval` ("5m", "1h", "6h", "24h"),
    /// `limit` results (default 100)
    pub async fn fetch_top_organic_score(
        &self,
        interval: &str,
        limit: Option<usize>,
    ) -> Result<Vec<JupiterToken>, Error> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT);
        self.fetch_list(format!(
            "{JUPITER_BASE_URL}/toporganicscore/{interval}?limit={limit}"
        ))
        .await
    }

    /// Fetch top traded tokens for `interval` ("5m", "1h", "6h", "24h"),
    /// `limit` results (default 100)
    pub async fn fetch_top_traded(
        &self,
        interval: &str,
        limit: Option<usize>,
    ) -> Result<Vec<JupiterToken>, Error> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT);
        self.fetch_list(format!(
            "{JUPITER_BASE_URL}/toptraded/{interval}?limit={limit}"
        ))
        .await
    }

    /// Fetch top trending tokens for `interval` ("5m", "1h", "6h", "24h"),
    /// `limit` results (default 100)
    pub async fn fetch_top_trending(
        &self,
        interval: &str,
        limit: Option<usize>,
    ) -> Result<Vec<JupiterToken>, Error> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT);
        self.fetch_list(format!(
            "{JUPITER_BASE_URL}/toptrending/{interval}?limit={limit}"
        ))
        .await
    }

    /// One token-list request behind the background throttle. A transport failure carries
    /// its whole cause chain: reqwest's own message ("error sending request") names only the
    /// URL, never whether the connect, the TLS handshake, the body or the timeout failed.
    async fn fetch_list(&self, url: String) -> Result<Vec<JupiterToken>, Error> {
        if !self.enabled {
            return Err(Error::Disabled {
                provider: "Jupiter".to_owned(),
            });
        }

        crate::chains::solana::apis::jupiter::throttle::acquire_background().await;
        let start = Instant::now();

        let response = match self
            .http_client
            .client()
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => {
                self.stats
                    .record_request(false, start.elapsed().as_millis() as f64)
                    .await;
                return Err(NetworkError::RequestFailed {
                    endpoint: url,
                    detail: source_chain(&e).join(": "),
                }
                .into());
            }
        };

        let elapsed = start.elapsed().as_millis() as f64;

        if !response.status().is_success() {
            self.stats.record_request(false, elapsed).await;
            return Err(NetworkError::http_status(url, response.status().as_u16(), None).into());
        }

        match response.json::<Vec<JupiterToken>>().await {
            Ok(tokens) => {
                self.stats.record_request(true, elapsed).await;
                Ok(tokens)
            }
            Err(e) => {
                self.stats.record_request(false, elapsed).await;
                Err(DataError::ParseError {
                    data_type: url,
                    error: source_chain(&e).join(": "),
                }
                .into())
            }
        }
    }
}
