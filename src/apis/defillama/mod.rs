//! DeFiLlama API client
//!
//! API Documentation: https://defillama.com/docs/api
//!
//! Endpoints implemented:
//! 1. /protocols - Get all DeFi protocols
//! 2. /prices/current/solana:{mint} - Get current token price

pub mod types;

use self::types::{DefiLlamaPriceResponse, DefiLlamaProtocol};
use crate::apis::client::HttpClient;
use crate::apis::stats::ApiStatsTracker;
use crate::apis::Error;
use crate::errors::{DataError, NetworkError};
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
    /// Create a new DefiLlama API client
    pub fn new(enabled: bool) -> Result<Self, Error> {
        let http_client = HttpClient::new(TIMEOUT_SECS)?;
        let stats = Arc::new(ApiStatsTracker::new());

        Ok(Self {
            http_client,
            stats,
            enabled,
        })
    }

    /// Whether this client is enabled in the current configuration
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Retrieve cumulative API usage statistics
    pub async fn get_stats(&self) -> super::stats::ApiStats {
        self.stats.get_stats().await
    }

    /// Fetch all DeFi protocols
    pub async fn fetch_protocols(&self) -> Result<Vec<DefiLlamaProtocol>, Error> {
        if !self.enabled {
            return Err(Error::Disabled {
                provider: "DeFiLlama".to_owned(),
            });
        }

        let start = Instant::now();
        let url = format!("{DEFILLAMA_BASE_URL}/protocols");

        let response = self
            .http_client
            .client()
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                let error: Error = NetworkError::RequestFailed {
                    endpoint: url.clone(),
                    detail: e.to_string(),
                }
                .into();
                self.stats.record_cache_miss();
                error
            })?;

        let elapsed = start.elapsed().as_millis() as f64;

        if !response.status().is_success() {
            self.stats.record_request(false, elapsed).await;
            return Err(NetworkError::HttpStatus {
                endpoint: url.clone(),
                status: response.status().as_u16(),
                body: None,
            }
            .into());
        }

        let protocols: Vec<DefiLlamaProtocol> = match response.json().await {
            Ok(parsed) => parsed,
            Err(e) => {
                self.stats.record_request(false, elapsed).await;
                return Err(DataError::ParseError {
                    data_type: url.clone(),
                    error: e.to_string(),
                }
                .into());
            }
        };

        self.stats.record_request(true, elapsed).await;

        Ok(protocols)
    }

    /// Fetch current price for a Solana token
    ///
    /// # Arguments
    /// * `mint` - Solana token mint address
    pub async fn fetch_token_price(&self, mint: &str) -> Result<f64, Error> {
        if !self.enabled {
            return Err(Error::Disabled {
                provider: "DeFiLlama".to_owned(),
            });
        }

        let start = Instant::now();
        let url = format!("{DEFILLAMA_PRICES_URL}/solana:{mint}");

        let response = self
            .http_client
            .client()
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                let error: Error = NetworkError::RequestFailed {
                    endpoint: url.clone(),
                    detail: e.to_string(),
                }
                .into();
                self.stats.record_cache_miss();
                error
            })?;

        let elapsed = start.elapsed().as_millis() as f64;

        if !response.status().is_success() {
            self.stats.record_request(false, elapsed).await;
            return Err(NetworkError::HttpStatus {
                endpoint: url.clone(),
                status: response.status().as_u16(),
                body: None,
            }
            .into());
        }

        let price_response: DefiLlamaPriceResponse = match response.json().await {
            Ok(parsed) => parsed,
            Err(e) => {
                self.stats.record_request(false, elapsed).await;
                return Err(DataError::ParseError {
                    data_type: url.clone(),
                    error: e.to_string(),
                }
                .into());
            }
        };

        self.stats.record_request(true, elapsed).await;

        // Extract price from response
        let price_key = format!("solana:{mint}");
        price_response
            .coins
            .get(&price_key)
            .map(|p| p.price)
            .ok_or_else(|| Error::NotFound {
                provider: "DeFiLlama".to_owned(),
                resource: price_key,
            })
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
                        chains.iter().any(|chain| {
                            chain
                                .to_lowercase()
                                .contains(crate::chains::adapter().market_data_network())
                        })
                    })
                    .unwrap_or_default();

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
                        chains.iter().any(|chain| {
                            chain
                                .to_lowercase()
                                .contains(crate::chains::adapter().market_data_network())
                        })
                    })
                    .unwrap_or_default();

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
