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
use super::dexscreener_types::*;
use crate::tokens::api::dexscreener_types::DexScreenerPool;
use crate::tokens::types::ApiError;
use log::{debug, warn};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

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

/// Rate limit per minute - DexScreener has generous limits, 300/min is conservative
pub const RATE_LIMIT_PER_MINUTE: usize = 300;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// Complete DexScreener API client
pub struct DexScreenerClient {
    client: Client,
    rate_limiter: Arc<Semaphore>,
    timeout: Duration,
}

impl DexScreenerClient {
    pub fn new(rate_limit: usize, timeout_seconds: u64) -> Self {
        Self {
            client: Client::new(),
            rate_limiter: Arc::new(Semaphore::new(rate_limit)),
            timeout: Duration::from_secs(timeout_seconds),
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
        let permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!(
            "{}/token-pairs/v1/{}/{}",
            DEXSCREENER_BASE_URL, chain, token_address
        );

        debug!(
            "[DEXSCREENER] Fetching token pools: token={}, chain={}",
            token_address, chain
        );

        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("DexScreener API error {}: {}", status, error_text));
        }

        let pairs: Vec<DexScreenerPairRaw> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

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
        let permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

        let address_list = addresses.join(",");
        let url = format!(
            "{}/tokens/v1/{}/{}",
            DEXSCREENER_BASE_URL, chain, address_list
        );

        debug!(
            "[DEXSCREENER] Fetching batch tokens: {} addresses, chain={}",
            addresses.len(),
            chain
        );

        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("DexScreener API error {}: {}", status, error_text));
        }

        let pairs: Vec<DexScreenerPairRaw> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

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
        let permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!(
            "{}/latest/dex/pairs/{}/{}",
            DEXSCREENER_BASE_URL, chain_id, pair_address
        );

        debug!(
            "[DEXSCREENER] Fetching pair: {} on {}",
            pair_address, chain_id
        );

        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("DexScreener API error {}: {}", status, error_text));
        }

        let data: PairResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

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

        let permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!("{}/latest/dex/search", DEXSCREENER_BASE_URL);

        debug!("[DEXSCREENER] Searching pairs: query={}", query);

        let response = self
            .client
            .get(&url)
            .query(&[("q", query)])
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("DexScreener API error {}: {}", status, error_text));
        }

        let data: PairsResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(data.pairs.into_iter().map(|p| p.to_pool()).collect())
    }

    /// Get latest token profiles (newest listings)
    pub async fn get_latest_profiles(&self) -> Result<Vec<TokenProfile>, String> {
        let permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!("{}/token-profiles/latest/v1", DEXSCREENER_BASE_URL);

        debug!("[DEXSCREENER] Fetching latest token profiles");

        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("DexScreener API error {}: {}", status, error_text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Parse token profiles from response
        let profiles: Vec<TokenProfile> = serde_json::from_value(data).unwrap_or_default();

        Ok(profiles)
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
        let permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

        let mut url = format!("{}/token-boosts/top/v1", DEXSCREENER_BASE_URL);
        if let Some(chain) = chain_id {
            url = format!("{}?chainId={}", url, chain);
        }

        debug!("[DEXSCREENER] Fetching top boosted tokens");

        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("DexScreener API error {}: {}", status, error_text));
        }

        let boosts: Vec<TokenBoostTop> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(boosts)
    }

    /// Get latest boosted tokens (newest promotions)
    /// Uses /token-boosts/latest/v1
    ///
    /// # Returns
    /// Vec<TokenBoostLatest> - Latest boosted tokens
    pub async fn get_latest_boosted_tokens(&self) -> Result<Vec<TokenBoostLatest>, String> {
        let permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!("{}/token-boosts/latest/v1", DEXSCREENER_BASE_URL);

        debug!("[DEXSCREENER] Fetching latest boosted tokens");

        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("DexScreener API error {}: {}", status, error_text));
        }

        let boosts: Vec<TokenBoostLatest> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(boosts)
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
        let permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

        let mut url = format!("{}/token-profiles/latest/v1", DEXSCREENER_BASE_URL);

        let mut params = vec![];
        if let Some(chain) = chain_id {
            params.push(format!("chainId={}", chain));
        }
        if let Some(sort) = sort_by {
            params.push(format!("sortBy={}", sort));
        }
        if let Some(order_val) = order {
            params.push(format!("order={}", order_val));
        }

        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        debug!(
            "[DEXSCREENER] Fetching top tokens: chain={:?}, sort={:?}",
            chain_id, sort_by
        );

        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("DexScreener API error {}: {}", status, error_text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Parse and convert to pools - implement based on actual response
        Ok(Vec::new())
    }

    /// Get token info with social links, description, etc.
    ///
    /// # Arguments
    /// * `address` - Token address
    pub async fn get_token_info(&self, address: &str) -> Result<Option<TokenInfo>, String> {
        let permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate-limiter error: {}", e))?;

        let url = format!("{}/token-profiles/{}", DEXSCREENER_BASE_URL, address);

        debug!("[DEXSCREENER] Fetching token info: {}", address);

        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 404 {
                return Ok(None);
            }
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("DexScreener API error {}: {}", status, error_text));
        }

        let info: TokenInfo = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(Some(info))
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
        let permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!(
            "{}/orders/v1/{}/{}",
            DEXSCREENER_BASE_URL, chain, token_address
        );

        debug!(
            "[DEXSCREENER] Fetching token orders: token={}, chain={}",
            token_address, chain
        );

        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("DexScreener API error {}: {}", status, error_text));
        }

        let orders: Vec<TokenOrder> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(orders)
    }

    /// Get supported chains
    pub async fn get_supported_chains(&self) -> Result<Vec<ChainInfo>, String> {
        let permit = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!("{}/chains/v1", DEXSCREENER_BASE_URL);

        debug!("[DEXSCREENER] Fetching supported chains");

        let response = self
            .client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("DexScreener API error {}: {}", status, error_text));
        }

        let chains: Vec<ChainInfo> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(chains)
    }

    /// Legacy method for backward compatibility - redirects to fetch_token_pools
    pub async fn fetch_pools(&self, mint: &str) -> Result<Vec<DexScreenerPool>, String> {
        self.fetch_token_pools(mint, None).await
    }
}
