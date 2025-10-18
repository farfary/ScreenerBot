/// DeFiLlama API client
///
/// API Documentation: https://defillama.com/docs/api
///
/// Endpoints implemented:
/// 1. /protocols - Get all DeFi protocols
/// 2. /prices/current/solana:{mint} - Get current token price
use super::client::HttpClient;
use super::defillama_types::*;
use super::stats::ApiStatsTracker;
use crate::tokens::types::ApiError;
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// API CONFIGURATION - Hardcoded for DeFiLlama API
// ============================================================================

const DEFILLAMA_BASE_URL: &str = "https://api.llama.fi";
const DEFILLAMA_PRICES_URL: &str = "https://coins.llama.fi/prices/current";

/// Request timeout - DeFiLlama protocols endpoint can be slow with 6k+ protocols, 25s recommended
const TIMEOUT_SECS: u64 = 25;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

pub struct DefiLlamaClient {
    http_client: HttpClient,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl DefiLlamaClient {
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

    /// Fetch all DeFi protocols
    pub async fn fetch_protocols(&self) -> Result<Vec<DefiLlamaProtocol>, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        let start = Instant::now();
        let url = format!("{}/protocols", DEFILLAMA_BASE_URL);

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

        let protocols: Vec<DefiLlamaProtocol> = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        Ok(protocols)
    }

    /// Fetch current price for a Solana token
    ///
    /// # Arguments
    /// * `mint` - Solana token mint address
    pub async fn fetch_token_price(&self, mint: &str) -> Result<f64, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        let start = Instant::now();
        let url = format!("{}/solana:{}", DEFILLAMA_PRICES_URL, mint);

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

        let price_response: DefiLlamaPriceResponse = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        // Extract price from response
        let price_key = format!("solana:{}", mint);
        price_response
            .coins
            .get(&price_key)
            .map(|p| p.price)
            .ok_or_else(|| ApiError::NotFound)
    }

    /// Extract Solana token addresses from protocols
    pub fn extract_solana_addresses(protocols: &[DefiLlamaProtocol]) -> Vec<String> {
        protocols
            .iter()
            .filter_map(|protocol| {
                // Check if protocol supports Solana
                let has_solana = protocol
                    .chains
                    .as_ref()
                    .map(|chains| {
                        chains
                            .iter()
                            .any(|chain| chain.to_lowercase().contains("solana"))
                    })
                    .unwrap_or(false);

                if has_solana {
                    protocol.address.as_ref().and_then(|addr| {
                        if !addr.is_empty() && addr.len() > 32 && addr.len() < 50 {
                            Some(addr.clone())
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Extract Solana token addresses with names
    pub fn extract_solana_addresses_with_names(
        protocols: &[DefiLlamaProtocol],
    ) -> Vec<(String, String)> {
        protocols
            .iter()
            .filter_map(|protocol| {
                // Check if protocol supports Solana
                let has_solana = protocol
                    .chains
                    .as_ref()
                    .map(|chains| {
                        chains
                            .iter()
                            .any(|chain| chain.to_lowercase().contains("solana"))
                    })
                    .unwrap_or(false);

                if has_solana {
                    protocol.address.as_ref().and_then(|addr| {
                        if !addr.is_empty() && addr.len() > 32 && addr.len() < 50 {
                            Some((protocol.name.clone(), addr.clone()))
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}
