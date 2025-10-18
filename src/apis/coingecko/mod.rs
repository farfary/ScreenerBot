/// CoinGecko API client
///
/// API Documentation: https://docs.coingecko.com/reference/introduction
///
/// Endpoints implemented:
/// 1. /api/v3/coins/list?include_platform=true - Get all coins with platform addresses

pub mod types;

use crate::apis::client::HttpClient;
use crate::apis::stats::ApiStatsTracker;
use self::types::CoinGeckoCoin;
use crate::tokens::types::ApiError;
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// API CONFIGURATION - Hardcoded for CoinGecko API
// ============================================================================

const COINGECKO_BASE_URL: &str = "https://api.coingecko.com/api/v3";

/// API key for CoinGecko demo tier
const COINGECKO_API_KEY: &str = "COINGECKO_KEY_REMOVED";

/// Request timeout - CoinGecko can be slow with large datasets, 20s recommended
const TIMEOUT_SECS: u64 = 20;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

pub struct CoinGeckoClient {
    http_client: HttpClient,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl CoinGeckoClient {
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

    /// Fetch all coins with platform addresses
    /// Returns coins that have Solana addresses in their platforms
    pub async fn fetch_coins_list(&self) -> Result<Vec<CoinGeckoCoin>, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        let start = Instant::now();
        let url = format!("{}/coins/list?include_platform=true", COINGECKO_BASE_URL);

        let response = self
            .http_client
            .client()
            .get(&url)
            .header("Accept", "application/json")
            .header("x-cg-demo-api-key", COINGECKO_API_KEY)
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

        let coins: Vec<CoinGeckoCoin> = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        Ok(coins)
    }

    /// Extract Solana token addresses from coins list
    pub fn extract_solana_addresses(coins: &[CoinGeckoCoin]) -> Vec<String> {
        coins
            .iter()
            .filter_map(|coin| {
                coin.platforms.as_ref().and_then(|platforms| {
                    platforms.get("solana").and_then(|addr| {
                        if !addr.is_empty() && addr.len() > 32 && addr.len() < 50 {
                            Some(addr.clone())
                        } else {
                            None
                        }
                    })
                })
            })
            .collect()
    }

    /// Extract Solana token addresses with names
    pub fn extract_solana_addresses_with_names(coins: &[CoinGeckoCoin]) -> Vec<(String, String)> {
        coins
            .iter()
            .filter_map(|coin| {
                coin.platforms.as_ref().and_then(|platforms| {
                    platforms.get("solana").and_then(|addr| {
                        if !addr.is_empty() && addr.len() > 32 && addr.len() < 50 {
                            Some((coin.name.clone(), addr.clone()))
                        } else {
                            None
                        }
                    })
                })
            })
            .collect()
    }
}
