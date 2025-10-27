/// Complete DexScreener API client with ALL available endpoints
///
/// API Documentation: https://docs.dexscreener.com/api/reference
///
/// Endpoints implemented (verified working):
/// 1. /token-pairs/v1/{chainId}/{tokenAddress} - PRIMARY: Get all pools for a token
/// 2. /tokens/v1/{chainId}/{tokenAddresses} - Get pools for up to 30 tokens (batch)
/// 3. /latest/dex/pairs/{chainId}/{pairId} - Get single pair by chain/address
/// 4. /latest/dex/search?q={query} - Search pairs
/// 5. /token-profiles/latest/v1 - Get latest token profiles
/// 6. /token-boosts/latest/v1 - Get latest boosted tokens
/// 7. /token-boosts/top/v1 - Get top boosted tokens  
/// 8. /orders/v1/{chainId}/{tokenAddress} - Get orders for a token
pub mod types;

// Re-export types for external use
pub use self::types::{
    ChainInfo, DexScreenerPairRaw, DexScreenerPool, PairResponse, PairsResponse, TokenBoostLatest,
    TokenBoostTop, TokenInfo, TokenOrder, TokenProfile,
};

use crate::apis::client::RateLimiter;
use crate::apis::stats::ApiStatsTracker;
use crate::logger::{self, LogTag};
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// API CONFIGURATION - Hardcoded for DexScreener API
// ============================================================================

const DEXSCREENER_BASE_URL: &str = "https://api.dexscreener.com";

/// Default chain for Solana operations
const DEFAULT_CHAIN_ID: &str = "solana";

/// Maximum tokens per batch request
const MAX_TOKENS_PER_REQUEST: usize = 30;

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
    client: Client,
    stats: Arc<ApiStatsTracker>,
    timeout: Duration,
    enabled: bool,
    limiter_token_pools: RateLimiter,
    limiter_token_batch: RateLimiter,
    limiter_pair_lookup: RateLimiter,
    limiter_search: RateLimiter,
    limiter_latest_profiles: RateLimiter,
    limiter_latest_boosts: RateLimiter,
    limiter_top_boosts: RateLimiter,
    limiter_token_orders: RateLimiter,
    limiter_token_info: RateLimiter,
    limiter_supported_chains: RateLimiter,
}

impl DexScreenerClient {
    pub fn new(enabled: bool, timeout_seconds: u64) -> Result<Self, String> {
        if timeout_seconds == 0 {
            return Err("Timeout must be greater than zero".to_string());
        }

        Ok(Self {
            client: Client::new(),
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

    /// Get API stats (placeholder - DexScreener uses direct HTTP without stats tracking)
    pub async fn get_stats(&self) -> crate::apis::stats::ApiStats {
        self.stats.get_stats().await
    }

    fn ensure_enabled(&self, endpoint: &str) -> Result<(), String> {
        if self.enabled {
            Ok(())
        } else {
            Err(format!(
                "DexScreener client disabled via configuration (endpoint={})",
                endpoint
            ))
        }
    }

    async fn execute_request(
        &self,
        endpoint: &str,
        builder: reqwest::RequestBuilder,
        limiter: &RateLimiter,
    ) -> Result<(reqwest::Response, f64), String> {
        self.ensure_enabled(endpoint)?;

        let guard = limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

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
                        format!("Request failed: {}", err),
                    )
                    .await;
                Err(format!("Request failed: {}", err))
            }
        }
    }

    async fn get_json<T>(
        &self,
        endpoint: &str,
        builder: reqwest::RequestBuilder,
        limiter: &RateLimiter,
    ) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        let (mut response, elapsed) = self.execute_request(endpoint, builder, limiter).await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.stats.record_request(false, elapsed).await;
            self.stats
                .record_error_with_event(
                    "DexScreener",
                    endpoint,
                    format!("HTTP {}: {}", status, body),
                )
                .await;
            return Err(format!("DexScreener API error {}: {}", status, body));
        }

        match response.json::<T>().await {
            Ok(value) => {
                self.stats.record_request(true, elapsed).await;
                Ok(value)
            }
            Err(err) => {
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event(
                        "DexScreener",
                        endpoint,
                        format!("Parse error: {}", err),
                    )
                    .await;
                Err(format!("Failed to parse response: {}", err))
            }
        }
    }

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
    ) -> Result<Vec<DexScreenerPool>, String> {
        let chain = chain_id.unwrap_or(DEFAULT_CHAIN_ID);
        let endpoint = format!("token-pairs/v1/{}/{}", chain, token_address);
        let url = format!("{}/{}", DEXSCREENER_BASE_URL, endpoint);

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
    ) -> Result<Vec<DexScreenerPool>, String> {
        if addresses.is_empty() {
            return Ok(Vec::new());
        }

        if addresses.len() > MAX_TOKENS_PER_REQUEST {
            return Err(format!(
                "Too many addresses: {} (max {})",
                addresses.len(),
                MAX_TOKENS_PER_REQUEST
            ));
        }

        let chain = chain_id.unwrap_or(DEFAULT_CHAIN_ID);
        let address_list = addresses.join(",");
        let endpoint = format!("tokens/v1/{}/{}", chain, address_list);
        let url = format!("{}/{}", DEXSCREENER_BASE_URL, endpoint);

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
    ) -> Result<Option<DexScreenerPool>, String> {
        let endpoint = format!("latest/dex/pairs/{}/{}", chain_id, pair_address);
        let url = format!("{}/{}", DEXSCREENER_BASE_URL, endpoint);

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
    pub async fn search(&self, query: &str) -> Result<Vec<DexScreenerPool>, String> {
        if query.trim().is_empty() {
            return Err("Query cannot be empty".to_string());
        }

        let endpoint = "latest/dex/search";
        let url = format!("{}/{}", DEXSCREENER_BASE_URL, endpoint);

        logger::debug(
            LogTag::Api,
            &format!("[DEXSCREENER] Searching pairs: query={}", query),
        );
        let builder = self.client.get(&url).query(&[("q", query)]);

        let data: PairsResponse = self
            .get_json(endpoint, builder, &self.limiter_search)
            .await?;

        Ok(data.pairs.into_iter().map(|p| p.to_pool()).collect())
    }

    /// Get latest token profiles (newest listings)
    pub async fn get_latest_profiles(&self) -> Result<Vec<TokenProfile>, String> {
        let endpoint = "token-profiles/latest/v1";
        let url = format!("{}/{}", DEXSCREENER_BASE_URL, endpoint);

        logger::debug(LogTag::Api, "[DEXSCREENER] Fetching latest token profiles");

        let (mut response, elapsed) = self
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
                .record_error_with_event(
                    "DexScreener",
                    endpoint,
                    format!("HTTP {}: {}", status, body),
                )
                .await;
            return Err(format!("DexScreener API error {}: {}", status, body));
        }

        let raw: serde_json::Value = match response.json().await {
            Ok(val) => val,
            Err(err) => {
                self.stats.record_request(false, elapsed).await;
                self.stats
                    .record_error_with_event(
                        "DexScreener",
                        endpoint,
                        format!("Parse error: {}", err),
                    )
                    .await;
                return Err(format!("Failed to parse response: {}", err));
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
                        format!("Conversion error: {}", err),
                    )
                    .await;
                Err(format!("Failed to decode token profiles: {}", err))
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
    ) -> Result<Vec<TokenBoostTop>, String> {
        let endpoint = "token-boosts/top/v1";
        let url = format!("{}/{}", DEXSCREENER_BASE_URL, endpoint);
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
    pub async fn get_latest_boosted_tokens(&self) -> Result<Vec<TokenBoostLatest>, String> {
        let endpoint = "token-boosts/latest/v1";
        let url = format!("{}/{}", DEXSCREENER_BASE_URL, endpoint);

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
    ) -> Result<Vec<DexScreenerPool>, String> {
        let endpoint = "token-profiles/latest/v1";
        let url = format!("{}/{}", DEXSCREENER_BASE_URL, endpoint);
        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(chain) = chain_id {
            query_params.push(("chainId".to_string(), chain.to_string()));
        }
        if let Some(sort) = sort_by {
            query_params.push(("sortBy".to_string(), sort.to_string()));
        }
        if let Some(order_val) = order {
            query_params.push(("order".to_string(), order_val.to_string()));
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

        let (mut response, elapsed) = self
            .execute_request(endpoint, builder, &self.limiter_latest_profiles)
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.stats.record_request(false, elapsed).await;
            self.stats
                .record_error_with_event(
                    "DexScreener",
                    endpoint,
                    format!("HTTP {}: {}", status, body),
                )
                .await;
            return Err(format!("DexScreener API error {}: {}", status, body));
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
                    .record_error_with_event(
                        "DexScreener",
                        endpoint,
                        format!("Parse error: {}", err),
                    )
                    .await;
                Err(format!("Failed to parse response: {}", err))
            }
        }
    }

    /// Get token info with social links, description, etc.
    ///
    /// # Arguments
    /// * `address` - Token address
    pub async fn get_token_info(&self, address: &str) -> Result<Option<TokenInfo>, String> {
        let endpoint = format!("token-profiles/{}", address);
        let url = format!("{}/{}", DEXSCREENER_BASE_URL, endpoint);

        logger::debug(
            LogTag::Api,
            &format!("[DEXSCREENER] Fetching token info: {}", address),
        );
        let (mut response, elapsed) = self
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
                .record_error_with_event(
                    "DexScreener",
                    &endpoint,
                    format!("HTTP {}: {}", status, body),
                )
                .await;
            return Err(format!("DexScreener API error {}: {}", status, body));
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
                        format!("Parse error: {}", err),
                    )
                    .await;
                Err(format!("Failed to parse response: {}", err))
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
    ) -> Result<Vec<TokenOrder>, String> {
        let chain = chain_id.unwrap_or(DEFAULT_CHAIN_ID);
        let endpoint = format!("orders/v1/{}/{}", chain, token_address);
        let url = format!("{}/{}", DEXSCREENER_BASE_URL, endpoint);

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
    pub async fn get_supported_chains(&self) -> Result<Vec<ChainInfo>, String> {
        let endpoint = "chains/v1";
        let url = format!("{}/{}", DEXSCREENER_BASE_URL, endpoint);

        logger::debug(LogTag::Api, "[DEXSCREENER] Fetching supported chains");

        self.get_json(
            endpoint,
            self.client.get(&url),
            &self.limiter_supported_chains,
        )
        .await
    }

    /// Legacy method for backward compatibility - redirects to fetch_token_pools
    pub async fn fetch_pools(&self, mint: &str) -> Result<Vec<DexScreenerPool>, String> {
        self.fetch_token_pools(mint, None).await
    }
}
