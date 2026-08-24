//! DexScreener API endpoint methods

use super::types::{
    ChainInfo, DexScreenerPairRaw, DexScreenerPool, PairResponse, PairsResponse, TokenBoostLatest,
    TokenBoostTop, TokenInfo, TokenOrder, TokenProfile,
};
use super::{default_chain_id, DexScreenerClient, DEXSCREENER_BASE_URL, MAX_TOKENS_PER_REQUEST};
use crate::apis::Error;
use crate::errors::{DataError, NetworkError};
use crate::logger::{self, LogTag};
use reqwest::StatusCode;
use std::time::Duration;

impl DexScreenerClient {
    /// PRIMARY METHOD: Fetch ALL pools for a single token address
    /// Uses /token-pairs/v1/{chainId}/{tokenAddress}
    ///
    /// Returns ALL liquidity pools (can be 30+) for the token across all DEXes.
    /// For batch operations with multiple tokens, use fetch_token_batch() instead.
    ///
    /// # Arguments
    /// * `token_address` - Token mint address
    /// * `chain_id` - Chain identifier (defaults to "solana")
    ///
    /// # Returns
    /// Vec<DexScreenerPool> - ALL pools for this token (typically 10-30 pools)
    pub async fn fetch_token_pools(
        &self,
        token_address: &str,
        chain_id: Option<&str>,
    ) -> Result<Vec<DexScreenerPool>, Error> {
        let chain = chain_id.unwrap_or_else(|| default_chain_id());
        let endpoint = format!("token-pairs/v1/{chain}/{token_address}");
        let url = format!("{DEXSCREENER_BASE_URL}/{endpoint}");

        logger::debug(
            LogTag::Api,
            &format!(
                "[DEXSCREENER] Fetching token pools: token={}, chain={}",
                token_address, chain
            ),
        );

        let pairs: Vec<DexScreenerPairRaw> = self
            .get_json(&endpoint, self.client.get(&url), &self.limiter_token_pools)
            .await?;

        Ok(pairs.into_iter().map(|p| p.to_pool()).collect())
    }

    /// Batch fetch the BEST/MOST LIQUID pair for up to 30 tokens in ONE call
    /// Uses /tokens/v1/{chainId}/{tokenAddresses}
    ///
    /// **IMPORTANT**: This returns ONE pair per token (the most liquid/popular one),
    /// not all pools. Use fetch_token_pools() if you need all pools for a token.
    ///
    /// # Arguments
    /// * `addresses` - Token mint addresses (max 30)
    /// * `chain_id` - Chain identifier (defaults to "solana")
    ///
    /// # Returns
    /// Vec<DexScreenerPool> - ONE best pair for each token in the batch
    pub async fn fetch_token_batch(
        &self,
        addresses: &[String],
        chain_id: Option<&str>,
    ) -> Result<Vec<DexScreenerPool>, Error> {
        if addresses.is_empty() {
            return Ok(Vec::new());
        }

        if addresses.len() > MAX_TOKENS_PER_REQUEST {
            return Err(DataError::ValidationError {
                field: "addresses".to_owned(),
                value: addresses.len().to_string(),
                reason: format!("max {MAX_TOKENS_PER_REQUEST}"),
            }
            .into());
        }

        let chain = chain_id.unwrap_or_else(|| default_chain_id());
        let address_list = addresses.join(",");
        let endpoint = format!("tokens/v1/{chain}/{address_list}");
        let url = format!("{DEXSCREENER_BASE_URL}/{endpoint}");

        logger::debug(
            LogTag::Api,
            &format!(
                "[DEXSCREENER] Fetching batch tokens: {} addresses, chain={}",
                addresses.len(),
                chain
            ),
        );
        let pairs: Vec<DexScreenerPairRaw> = self
            .get_json(&endpoint, self.client.get(&url), &self.limiter_token_batch)
            .await?;

        Ok(pairs.into_iter().map(|p| p.to_pool()).collect())
    }

    /// Get a single pair by chain and address
    ///
    /// # Arguments
    /// * `chain_id` - Chain identifier (e.g., "solana", "ethereum")
    /// * `pair_address` - Pair contract address
    pub async fn get_pair(
        &self,
        chain_id: &str,
        pair_address: &str,
    ) -> Result<Option<DexScreenerPool>, Error> {
        let endpoint = format!("latest/dex/pairs/{chain_id}/{pair_address}");
        let url = format!("{DEXSCREENER_BASE_URL}/{endpoint}");

        logger::debug(
            LogTag::Api,
            &format!(
                "[DEXSCREENER] Fetching pair: pair={}, chain={}",
                pair_address, chain_id
            ),
        );
        let data: PairResponse = self
            .get_json(&endpoint, self.client.get(&url), &self.limiter_pair_lookup)
            .await?;

        Ok(data.pair.map(|p| p.to_pool()))
    }

    /// Search for pairs by query
    ///
    /// # Arguments
    /// * `query` - Search query (token name, symbol, address)
    ///
    /// # Returns
    /// Vec of matching pairs
    pub async fn search(&self, query: &str) -> Result<Vec<DexScreenerPool>, Error> {
        if query.trim().is_empty() {
            return Err(DataError::ValidationError {
                field: "query".to_owned(),
                value: String::new(),
                reason: "cannot be empty".to_owned(),
            }
            .into());
        }

        let endpoint = "latest/dex/search";
        let url = format!("{DEXSCREENER_BASE_URL}/{endpoint}");

        logger::debug(
            LogTag::Api,
            &format!("[DEXSCREENER] Searching pairs: query={query}"),
        );
        let builder = self.client.get(&url).query(&[("q", query)]);

        let data: PairsResponse = self
            .get_json(endpoint, builder, &self.limiter_search)
            .await?;

        Ok(data.pairs.into_iter().map(|p| p.to_pool()).collect())
    }

    /// Get latest token profiles (newest listings)
    pub async fn get_latest_profiles(&self) -> Result<Vec<TokenProfile>, Error> {
        let endpoint = "token-profiles/latest/v1";
        let url = format!("{DEXSCREENER_BASE_URL}/{endpoint}");

        logger::debug(LogTag::Api, "[DEXSCREENER] Fetching latest token profiles");

        let (response, elapsed) = self
            .execute_request(
                endpoint,
                self.client.get(&url),
                &self.limiter_latest_profiles,
            )
            .await?;

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

        let raw: serde_json::Value = match response.json().await {
            Ok(val) => val,
            Err(err) => {
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event("DexScreener", endpoint, format!("Parse error: {err}"))
                    .await;
                return Err(DataError::ParseError {
                    data_type: endpoint.to_owned(),
                    error: err.to_string(),
                }
                .into());
            }
        };

        match serde_json::from_value::<Vec<TokenProfile>>(raw) {
            Ok(profiles) => {
                self.stats.record_request(true, elapsed).await;
                Ok(profiles)
            }
            Err(err) => {
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event(
                        "DexScreener",
                        endpoint,
                        format!("Conversion error: {err}"),
                    )
                    .await;
                Err(DataError::ParseError {
                    data_type: "token profiles".to_owned(),
                    error: err.to_string(),
                }
                .into())
            }
        }
    }

    /// Get top boosted tokens (most promoted)
    /// Uses /token-boosts/top/v1
    ///
    /// # Arguments
    /// * `chain_id` - Optional chain filter (e.g., "solana")
    ///
    /// # Returns
    /// Vec<TokenBoostTop> - Top boosted tokens with promotion details
    pub async fn get_top_boosted_tokens(
        &self,
        chain_id: Option<&str>,
    ) -> Result<Vec<TokenBoostTop>, Error> {
        let endpoint = "token-boosts/top/v1";
        let url = format!("{DEXSCREENER_BASE_URL}/{endpoint}");
        let builder = if let Some(chain) = chain_id {
            self.client.get(&url).query(&[("chainId", chain)])
        } else {
            self.client.get(&url)
        };

        logger::debug(LogTag::Api, "[DEXSCREENER] Fetching top boosted tokens");

        self.get_json(endpoint, builder, &self.limiter_top_boosts)
            .await
    }

    /// Get latest boosted tokens (newest promotions)
    /// Uses /token-boosts/latest/v1
    ///
    /// # Returns
    /// Vec<TokenBoostLatest> - Latest boosted tokens
    pub async fn get_latest_boosted_tokens(&self) -> Result<Vec<TokenBoostLatest>, Error> {
        let endpoint = "token-boosts/latest/v1";
        let url = format!("{DEXSCREENER_BASE_URL}/{endpoint}");

        logger::debug(LogTag::Api, "[DEXSCREENER] Fetching latest boosted tokens");

        self.get_json(endpoint, self.client.get(&url), &self.limiter_latest_boosts)
            .await
    }

    /// Get top tokens by volume in a specific time window
    ///
    /// # Arguments
    /// * `chain_id` - Optional chain filter
    /// * `sort_by` - Sort criterion ("volume", "liquidity", "marketCap")
    /// * `order` - Sort order ("desc", "asc")
    pub async fn get_top_tokens(
        &self,
        chain_id: Option<&str>,
        sort_by: Option<&str>,
        order: Option<&str>,
    ) -> Result<Vec<DexScreenerPool>, Error> {
        let endpoint = "token-profiles/latest/v1";
        let url = format!("{DEXSCREENER_BASE_URL}/{endpoint}");
        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(chain) = chain_id {
            query_params.push(("chainId".to_owned(), chain.to_string()));
        }
        if let Some(sort) = sort_by {
            query_params.push(("sortBy".to_owned(), sort.to_string()));
        }
        if let Some(order_val) = order {
            query_params.push(("order".to_owned(), order_val.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[DEXSCREENER] Fetching top tokens: chain={:?}, sort={:?}",
                chain_id, sort_by
            ),
        );

        let (response, elapsed) = self
            .execute_request(endpoint, builder, &self.limiter_latest_profiles)
            .await?;

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

        match response.json::<serde_json::Value>().await {
            Ok(value) => {
                // Placeholder until this endpoint is wired into pool conversion logic
                let _ = value;
                self.stats.record_request(true, elapsed).await;
                Ok(Vec::new())
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

    /// Get token info with social links, description, etc.
    ///
    /// # Arguments
    /// * `address` - Token address
    pub async fn get_token_info(&self, address: &str) -> Result<Option<TokenInfo>, Error> {
        let endpoint = format!("token-profiles/{address}");
        let url = format!("{DEXSCREENER_BASE_URL}/{endpoint}");

        logger::debug(
            LogTag::Api,
            &format!("[DEXSCREENER] Fetching token info: {address}"),
        );
        let (response, elapsed) = self
            .execute_request(&endpoint, self.client.get(&url), &self.limiter_token_info)
            .await?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            self.stats.record_request(true, elapsed).await;
            return Ok(None);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.stats.record_request(false, elapsed).await;
            self.stats
                .record_error_with_event("DexScreener", &endpoint, format!("HTTP {status}: {body}"))
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

        match response.json::<TokenInfo>().await {
            Ok(info) => {
                self.stats.record_request(true, elapsed).await;
                Ok(Some(info))
            }
            Err(err) => {
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event(
                        "DexScreener",
                        &endpoint,
                        format!("Parse error: {err}"),
                    )
                    .await;
                Err(DataError::ParseError {
                    data_type: endpoint.to_owned(),
                    error: err.to_string(),
                }
                .into())
            }
        }
    }

    /// Get token orders (paid promotions, ads)
    /// Uses /orders/v1/{chainId}/{tokenAddress}
    ///
    /// # Arguments  
    /// * `token_address` - Token address
    /// * `chain_id` - Chain identifier (defaults to "solana")
    pub async fn get_token_orders(
        &self,
        token_address: &str,
        chain_id: Option<&str>,
    ) -> Result<Vec<TokenOrder>, Error> {
        let chain = chain_id.unwrap_or_else(|| default_chain_id());
        let endpoint = format!("orders/v1/{chain}/{token_address}");
        let url = format!("{DEXSCREENER_BASE_URL}/{endpoint}");

        logger::debug(
            LogTag::Api,
            &format!(
                "[DEXSCREENER] Fetching token orders: token={}, chain={}",
                token_address, chain
            ),
        );

        self.get_json(&endpoint, self.client.get(&url), &self.limiter_token_orders)
            .await
    }

    /// Get supported chains
    pub async fn get_supported_chains(&self) -> Result<Vec<ChainInfo>, Error> {
        let endpoint = "chains/v1";
        let url = format!("{DEXSCREENER_BASE_URL}/{endpoint}");

        logger::debug(LogTag::Api, "[DEXSCREENER] Fetching supported chains");

        self.get_json(
            endpoint,
            self.client.get(&url),
            &self.limiter_supported_chains,
        )
        .await
    }

    /// Legacy method for backward compatibility - redirects to fetch_token_pools
    pub async fn fetch_pools(&self, mint: &str) -> Result<Vec<DexScreenerPool>, Error> {
        self.fetch_token_pools(mint, None).await
    }
}
