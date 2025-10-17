/// Complete GeckoTerminal API client with ALL available endpoints
/// 
/// API Documentation: https://www.geckoterminal.com/dex-api
/// 
/// Endpoints implemented (verified working):
/// 1. /networks/{network}/tokens/{token}/pools - PRIMARY: Get all pools for a token (with advanced params)
/// 2. /networks/{network}/trending_pools - Get trending pools by network
/// 3. /networks/{network}/pools - Get top pools by network
/// 4. /networks/{network}/pools/{address} - Get specific pool data by pool address
/// 5. /networks/{network}/pools/multi/{addresses} - Get multiple pools data (up to 30 addresses)
/// 6. /networks/{network}/pools/{pool}/ohlcv/{timeframe} - Get OHLCV candlestick data
/// 7. /networks/{network}/dexes - Get supported DEXes list by network
/// 8. /networks/{network}/new_pools - Get latest new pools by network
/// 9. /networks/{network}/tokens/multi/{addresses} - Get multiple tokens data (up to 30 addresses)
/// 10. /networks/{network}/tokens/{address}/info - Get token metadata (name, symbol, socials, etc.)
/// 11. /tokens/info_recently_updated - Get 100 most recently updated tokens (global endpoint)

use super::geckoterminal_types::*;
use crate::tokens_new::types::GeckoTerminalPool;
use log::debug;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

const GECKOTERMINAL_BASE_URL: &str = "https://api.geckoterminal.com/api/v2";
const DEFAULT_NETWORK: &str = "solana";
const MAX_TRENDING_PAGE: u32 = 10;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Complete GeckoTerminal API client
pub struct GeckoTerminalClient {
    client: Client,
    rate_limiter: Arc<Semaphore>,
    timeout: Duration,
}

impl GeckoTerminalClient {
    pub fn new(rate_limit: usize, timeout_seconds: u64) -> Self {
        Self {
            client: Client::new(),
            rate_limiter: Arc::new(Semaphore::new(rate_limit)),
            timeout: Duration::from_secs(timeout_seconds),
        }
    }

    /// PRIMARY METHOD: Fetch ALL pools for a single token address
    /// Uses /networks/{network}/tokens/{token}/pools
    /// 
    /// Returns ALL liquidity pools (typically 20+) for the token across all DEXes on the network.
    /// 
    /// # Arguments
    /// * `mint` - Token mint address
    /// * `network` - Network identifier (defaults to "solana")
    /// 
    /// # Returns
    /// Vec<GeckoTerminalPool> - ALL pools for this token
    pub async fn fetch_pools(&self, mint: &str) -> Result<Vec<GeckoTerminalPool>, String> {
        self.fetch_pools_on_network(mint, None).await
    }

    /// Fetch pools for a token on a specific network
    /// 
    /// # Arguments
    /// * `mint` - Token mint address
    /// * `network` - Network identifier (defaults to "solana")
    /// 
    /// # Returns
    /// Vec<GeckoTerminalPool> - ALL pools for this token on the network
    pub async fn fetch_pools_on_network(
        &self,
        mint: &str,
        network: Option<&str>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!(
            "{}/networks/{}/tokens/{}/pools",
            GECKOTERMINAL_BASE_URL, network_id, mint
        );

        debug!("[GECKOTERMINAL] Fetching pools: token={}, network={}", mint, network_id);

        let response = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let api_response: GeckoTerminalResponse = response.json().await.map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(api_response.data.into_iter().map(|p| p.to_pool(mint)).collect())
    }

    /// Get top pools by token address with advanced filtering
    /// Uses /networks/{network}/tokens/{token_address}/pools with query parameters
    /// 
    /// Returns top pools for a token with sorting and filtering options.
    /// Same endpoint as fetch_pools_on_network but with additional query parameters.
    /// 
    /// # Arguments
    /// * `token_address` - Token contract address
    /// * `network` - Network identifier (e.g., "solana", "eth", "bsc")
    /// * `include` - Optional comma-separated attributes to include (base_token, quote_token, dex)
    /// * `page` - Optional page number for pagination (max: 10, default: 1)
    /// * `sort` - Optional sort field (h24_volume_usd_desc, h24_tx_count_desc, h24_volume_usd_liquidity_desc)
    /// 
    /// # Returns
    /// Vector of pools sorted by specified criteria
    /// 
    /// # Example
    /// ```no_run
    /// let pools = client.fetch_top_pools_by_token(
    ///     "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    ///     "solana",
    ///     Some("base_token,quote_token"),
    ///     Some(1),
    ///     Some("h24_volume_usd_desc")
    /// ).await?;
    /// ```
    pub async fn fetch_top_pools_by_token(
        &self,
        token_address: &str,
        network: &str,
        include: Option<&str>,
        page: Option<u32>,
        sort: Option<&str>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        let mut url = format!(
            "{}/networks/{}/tokens/{}/pools",
            GECKOTERMINAL_BASE_URL, network, token_address
        );

        let mut params = Vec::new();

        if let Some(inc) = include {
            params.push(format!("include={}", inc));
        }
        if let Some(p) = page {
            params.push(format!("page={}", p));
        }
        if let Some(s) = sort {
            params.push(format!("sort={}", s));
        }

        if !params.is_empty() {
            url.push_str(&format!("?{}", params.join("&")));
        }

        debug!("[GECKOTERMINAL] Fetching top pools by token: token={}, network={}, page={:?}, sort={:?}", 
               token_address, network, page, sort);

        let response = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let api_response: GeckoTerminalResponse = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(api_response.data.into_iter().map(|p| p.to_pool(token_address)).collect())
    }

    /// Get trending pools by network
    /// Uses /networks/{network}/trending_pools
    /// 
    /// Returns trending pools for a specific network with optional filtering by duration.
    /// 
    /// # Arguments
    /// * `network` - Network ID (e.g., "solana", "eth") - defaults to "solana"
    /// * `page` - Page number (1-10, default 1)
    /// * `duration` - Trending duration: "5m", "1h", "6h", "24h" (default "24h")
    /// * `include` - Attributes to include: Vec of "base_token", "quote_token", "dex"
    /// 
    /// # Returns
    /// Vec<GeckoTerminalPool> - 20 trending pools per page
    pub async fn fetch_trending_pools_by_network(
        &self,
        network: Option<&str>,
        page: Option<u32>,
        duration: Option<&str>,
        include: Option<Vec<&str>>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!("{}/networks/{}/trending_pools", GECKOTERMINAL_BASE_URL, network_id);
        
        let mut params = Vec::new();
        
        if let Some(p) = page {
            params.push(format!("page={}", p.min(MAX_TRENDING_PAGE)));
        }
        
        if let Some(d) = duration {
            params.push(format!("duration={}", d));
        }
        
        if let Some(includes) = include {
            if !includes.is_empty() {
                params.push(format!("include={}", includes.join(",")));
            }
        }

        let final_url = if !params.is_empty() {
            format!("{}?{}", url, params.join("&"))
        } else {
            url
        };

        debug!("[GECKOTERMINAL] Fetching trending pools: network={}, page={:?}, duration={:?}", network_id, page, duration);

        let response = self.client
            .get(&final_url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let api_response: GeckoTerminalResponse = response.json().await.map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(api_response.data.into_iter().map(|p| p.to_pool("trending")).collect())
    }

    /// Get top pools by network
    /// Uses /networks/{network}/pools
    /// 
    /// Returns top pools on the network sorted by volume or transaction count.
    /// 
    /// # Arguments
    /// * `network` - Network ID (e.g., "solana", "eth") - defaults to "solana"
    /// * `include` - Attributes to include: Vec of "base_token", "quote_token", "dex"
    /// * `page` - Page number (1-10, default 1)
    /// * `sort` - Sort field: "h24_volume_usd_desc" or "h24_tx_count_desc" (default)
    /// 
    /// # Returns
    /// Vec<GeckoTerminalPool> - Top pools on the network
    pub async fn fetch_top_pools_by_network(
        &self,
        network: Option<&str>,
        include: Option<Vec<&str>>,
        page: Option<u32>,
        sort: Option<&str>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!("{}/networks/{}/pools", GECKOTERMINAL_BASE_URL, network_id);
        
        let mut params = Vec::new();
        
        if let Some(p) = page {
            let page_num = p.min(10).max(1);
            params.push(format!("page={}", page_num));
        }
        
        if let Some(s) = sort {
            params.push(format!("sort={}", s));
        }
        
        if let Some(includes) = include {
            if !includes.is_empty() {
                params.push(format!("include={}", includes.join(",")));
            }
        }

        let final_url = if !params.is_empty() {
            format!("{}?{}", url, params.join("&"))
        } else {
            url
        };

        debug!("[GECKOTERMINAL] Fetching top pools: network={}, page={:?}, sort={:?}", network_id, page, sort);

        let response = self.client
            .get(&final_url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let api_response: GeckoTerminalResponse = response.json().await.map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(api_response.data.into_iter().map(|p| p.to_pool("top_pools")).collect())
    }

    /// Get specific pool data by pool address
    /// Uses /networks/{network}/pools/{address}
    /// 
    /// Returns detailed data for a single pool including liquidity, volume, and price information.
    /// 
    /// # Arguments
    /// * `network` - Network ID (e.g., "solana", "eth") - defaults to "solana"
    /// * `pool_address` - Pool contract address
    /// * `include` - Attributes to include: Vec of "base_token", "quote_token", "dex"
    /// * `include_volume_breakdown` - Include volume breakdown (default false)
    /// * `include_composition` - Include pool composition (default false)
    /// 
    /// # Returns
    /// GeckoTerminalPool - Single pool data
    pub async fn fetch_pool_by_address(
        &self,
        network: Option<&str>,
        pool_address: &str,
        include: Option<Vec<&str>>,
        include_volume_breakdown: bool,
        include_composition: bool,
    ) -> Result<GeckoTerminalPool, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!("{}/networks/{}/pools/{}", GECKOTERMINAL_BASE_URL, network_id, pool_address);
        
        let mut params = Vec::new();
        
        if let Some(includes) = include {
            if !includes.is_empty() {
                params.push(format!("include={}", includes.join(",")));
            }
        }
        
        if include_volume_breakdown {
            params.push("include_volume_breakdown=true".to_string());
        }
        
        if include_composition {
            params.push("include_composition=true".to_string());
        }

        let final_url = if !params.is_empty() {
            format!("{}?{}", url, params.join("&"))
        } else {
            url
        };

        debug!("[GECKOTERMINAL] Fetching pool: network={}, address={}", network_id, pool_address);

        let response = self.client
            .get(&final_url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let api_response: GeckoTerminalResponse = response.json().await.map_err(|e| format!("JSON parse error: {}", e))?;

        // Single pool endpoint returns data array with one item
        api_response.data.into_iter()
            .next()
            .map(|p| p.to_pool(pool_address))
            .ok_or_else(|| "No pool data returned".to_string())
    }

    /// Get multiple pools data by pool addresses
    /// Uses /networks/{network}/pools/multi/{addresses}
    /// 
    /// Returns detailed data for multiple pools (up to 30 addresses).
    /// 
    /// # Arguments
    /// * `network` - Network ID (e.g., "solana", "eth") - defaults to "solana"
    /// * `addresses` - Pool contract addresses (up to 30, comma-separated)
    /// * `include` - Attributes to include: Vec of "base_token", "quote_token", "dex"
    /// * `include_volume_breakdown` - Include volume breakdown (default false)
    /// * `include_composition` - Include pool composition (default false)
    /// 
    /// # Returns
    /// Vec<GeckoTerminalPool> - Multiple pool data
    pub async fn fetch_pools_multi(
        &self,
        network: Option<&str>,
        addresses: Vec<&str>,
        include: Option<Vec<&str>>,
        include_volume_breakdown: bool,
        include_composition: bool,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        if addresses.is_empty() {
            return Err("At least one address is required".to_string());
        }
        
        if addresses.len() > 30 {
            return Err("Maximum 30 addresses allowed".to_string());
        }

        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        let addresses_str = addresses.join(",");
        let url = format!("{}/networks/{}/pools/multi/{}", GECKOTERMINAL_BASE_URL, network_id, addresses_str);
        
        let mut params = Vec::new();
        
        if let Some(includes) = include {
            if !includes.is_empty() {
                params.push(format!("include={}", includes.join(",")));
            }
        }
        
        if include_volume_breakdown {
            params.push("include_volume_breakdown=true".to_string());
        }
        
        if include_composition {
            params.push("include_composition=true".to_string());
        }

        let final_url = if !params.is_empty() {
            format!("{}?{}", url, params.join("&"))
        } else {
            url
        };

        debug!("[GECKOTERMINAL] Fetching multi pools: network={}, count={}", network_id, addresses.len());

        let response = self.client
            .get(&final_url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let api_response: GeckoTerminalResponse = response.json().await.map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(api_response.data.into_iter().map(|p| p.to_pool("multi")).collect())
    }

    /// Get OHLCV (Open, High, Low, Close, Volume) candlestick data for a pool
    /// Uses /networks/{network}/pools/{pool}/ohlcv/{timeframe}
    /// 
    /// Returns candlestick chart data with configurable timeframe and aggregation.
    /// 
    /// # Arguments
    /// * `network` - Network ID (e.g., "solana", "eth")
    /// * `pool_address` - Pool contract address
    /// * `timeframe` - Timeframe: "day", "hour", or "minute"
    /// * `aggregate` - Time period to aggregate (day: 1, hour: 1/4/12, minute: 1/5/15)
    /// * `limit` - Number of results (max 1000, default 100)
    /// * `currency` - "usd" or "token"
    /// * `before_timestamp` - Optional: return data before this timestamp
    /// * `token` - Optional: "base", "quote", or token address to invert chart
    /// 
    /// # Returns
    /// OhlcvResponse - Contains list of [timestamp, open, high, low, close, volume] candles
    pub async fn fetch_ohlcv(
        &self,
        network: &str,
        pool_address: &str,
        timeframe: &str,
        aggregate: Option<u32>,
        limit: Option<u32>,
        currency: Option<&str>,
        before_timestamp: Option<i64>,
        token: Option<&str>,
    ) -> Result<OhlcvResponse, String> {
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        debug!(
            "[GECKOTERMINAL] Fetching OHLCV: network={}, pool={}, timeframe={}, aggregate={:?}, limit={:?}",
            network, pool_address, timeframe, aggregate, limit
        );

        let mut url = format!(
            "{}/networks/{}/pools/{}/ohlcv/{}",
            GECKOTERMINAL_BASE_URL, network, pool_address, timeframe
        );

        // Build query parameters
        let mut params = Vec::new();
        
        if let Some(agg) = aggregate {
            params.push(format!("aggregate={}", agg));
        }
        
        if let Some(lim) = limit {
            params.push(format!("limit={}", lim.min(1000))); // Max 1000
        }
        
        if let Some(curr) = currency {
            params.push(format!("currency={}", curr));
        }
        
        if let Some(ts) = before_timestamp {
            params.push(format!("before_timestamp={}", ts));
        }
        
        if let Some(tok) = token {
            params.push(format!("token={}", tok));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let response = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let ohlcv_response: OhlcvResponseRaw = response.json().await.map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(OhlcvResponse {
            ohlcv_list: ohlcv_response.data.attributes.ohlcv_list,
            base_token: ohlcv_response.meta.base,
            quote_token: ohlcv_response.meta.quote,
        })
    }

    /// Get supported DEXes list by network
    /// Uses /networks/{network}/dexes
    /// 
    /// Returns list of all supported decentralized exchanges (DEXs) on the network.
    /// 
    /// # Arguments
    /// * `network` - Network ID (e.g., "solana", "eth")
    /// * `page` - Page number (default 1)
    /// 
    /// # Returns
    /// Vec<(String, String)> - List of (dex_id, dex_name) tuples
    pub async fn fetch_dexes_by_network(
        &self,
        network: &str,
        page: Option<u32>,
    ) -> Result<Vec<(String, String)>, String> {
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!("{}/networks/{}/dexes", GECKOTERMINAL_BASE_URL, network);
        
        let final_url = if let Some(p) = page {
            format!("{}?page={}", url, p)
        } else {
            url
        };

        debug!("[GECKOTERMINAL] Fetching DEXes: network={}, page={:?}", network, page);

        let response = self.client
            .get(&final_url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let dex_response: GeckoTerminalDexesResponse = response.json().await.map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(dex_response.data.into_iter().map(|d| (d.id, d.attributes.name)).collect())
    }

    /// Get latest new pools by network
    /// 
    /// # Arguments
    /// * `network` - Network identifier (e.g., "solana", "eth", "bsc")
    /// * `include` - Optional comma-separated attributes to include (base_token, quote_token, dex)
    /// * `page` - Optional page number for pagination (max: 10, default: 1)
    /// 
    /// # Returns
    /// Vector of pools sorted by creation time (newest first)
    /// 
    /// # Example
    /// ```no_run
    /// let pools = client.fetch_new_pools_by_network("solana", Some("base_token,quote_token"), None).await?;
    /// ```
    pub async fn fetch_new_pools_by_network(
        &self,
        network: &str,
        include: Option<&str>,
        page: Option<u32>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let network_id = network;
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        let mut url = format!("{}/networks/{}/new_pools", GECKOTERMINAL_BASE_URL, network_id);
        let mut params = Vec::new();

        if let Some(inc) = include {
            params.push(format!("include={}", inc));
        }
        if let Some(p) = page {
            params.push(format!("page={}", p));
        }

        if !params.is_empty() {
            url.push_str(&format!("?{}", params.join("&")));
        }

        debug!("[GECKOTERMINAL] Fetching new pools: network={}, page={:?}", network_id, page);

        let response = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let api_response: GeckoTerminalResponse = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(api_response.data.into_iter().map(|p| p.to_pool("new_pools")).collect())
    }

    /// Get multiple tokens data by addresses
    /// Uses /networks/{network}/tokens/multi/{addresses}
    /// 
    /// Returns data for multiple tokens including optional top pools and composition.
    /// 
    /// # Arguments
    /// * `network` - Network identifier (e.g., "solana", "eth", "bsc")
    /// * `addresses` - Comma-separated token addresses (up to 30)
    /// * `include` - Optional attributes to include (top_pools)
    /// * `include_composition` - Optional flag to include pool composition
    /// 
    /// # Returns
    /// Result with tokens data
    /// 
    /// # Example
    /// ```no_run
    /// let tokens = client.fetch_tokens_multi(
    ///     "eth",
    ///     "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2,0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
    ///     Some("top_pools"),
    ///     Some(true)
    /// ).await?;
    /// ```
    pub async fn fetch_tokens_multi(
        &self,
        network: &str,
        addresses: &str,
        include: Option<&str>,
        include_composition: Option<bool>,
    ) -> Result<GeckoTerminalTokensMultiResponse, String> {
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        let mut url = format!(
            "{}/networks/{}/tokens/multi/{}",
            GECKOTERMINAL_BASE_URL, network, addresses
        );

        let mut params = Vec::new();

        if let Some(inc) = include {
            params.push(format!("include={}", inc));
        }
        if let Some(comp) = include_composition {
            params.push(format!("include_composition={}", comp));
        }

        if !params.is_empty() {
            url.push_str(&format!("?{}", params.join("&")));
        }

        debug!("[GECKOTERMINAL] Fetching tokens multi: network={}, addresses_count={}", 
               network, addresses.split(',').count());

        let response = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let tokens_response: GeckoTerminalTokensMultiResponse = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(tokens_response)
    }

    /// Get token info/metadata by address
    /// Uses /networks/{network}/tokens/{address}/info
    /// 
    /// Returns detailed token metadata including name, symbol, CoinGecko ID, image,
    /// socials, websites, description, and other metadata.
    /// 
    /// # Arguments
    /// * `network` - Network identifier (e.g., "solana", "eth", "bsc")
    /// * `address` - Token contract address
    /// 
    /// # Returns
    /// Result with token info/metadata
    /// 
    /// # Example
    /// ```no_run
    /// let token_info = client.fetch_token_info("eth", "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2").await?;
    /// ```
    pub async fn fetch_token_info(
        &self,
        network: &str,
        address: &str,
    ) -> Result<GeckoTerminalTokenInfoResponse, String> {
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        let url = format!(
            "{}/networks/{}/tokens/{}/info",
            GECKOTERMINAL_BASE_URL, network, address
        );

        debug!("[GECKOTERMINAL] Fetching token info: network={}, address={}", network, address);

        let response = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let token_info_response: GeckoTerminalTokenInfoResponse = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(token_info_response)
    }

    /// Get most recently updated tokens list
    /// Uses /tokens/info_recently_updated
    /// 
    /// Returns 100 most recently updated tokens info, either across all networks or filtered by network.
    /// This is a global endpoint (no network in path).
    /// 
    /// # Arguments
    /// * `include` - Optional attributes to include (network)
    /// * `network` - Optional network filter (e.g., "solana", "eth", "bsc")
    /// 
    /// # Returns
    /// Result with recently updated tokens list
    /// 
    /// # Example
    /// ```no_run
    /// let recent = client.fetch_recently_updated_tokens(Some("network"), Some("eth")).await?;
    /// ```
    pub async fn fetch_recently_updated_tokens(
        &self,
        include: Option<&str>,
        network: Option<&str>,
    ) -> Result<GeckoTerminalRecentlyUpdatedResponse, String> {
        let permit = self.rate_limiter.acquire().await.map_err(|e| format!("Rate limiter error: {}", e))?;

        let mut url = format!("{}/tokens/info_recently_updated", GECKOTERMINAL_BASE_URL);
        let mut params = Vec::new();

        if let Some(inc) = include {
            params.push(format!("include={}", inc));
        }
        if let Some(net) = network {
            params.push(format!("network={}", net));
        }

        if !params.is_empty() {
            url.push_str(&format!("?{}", params.join("&")));
        }

        debug!("[GECKOTERMINAL] Fetching recently updated tokens: network={:?}", network);

        let response = self.client
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        drop(permit);

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("HTTP {}: {}", status, error_text));
        }

        let recently_updated_response: GeckoTerminalRecentlyUpdatedResponse = response.json().await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        Ok(recently_updated_response)
    }
}

// ===== OHLCV Response Types =====

#[derive(Debug, Deserialize)]
struct OhlcvResponseRaw {
    data: OhlcvData,
    meta: OhlcvMeta,
}

#[derive(Debug, Deserialize)]
struct OhlcvData {
    attributes: OhlcvAttributes,
}

#[derive(Debug, Deserialize)]
struct OhlcvAttributes {
    ohlcv_list: Vec<[f64; 6]>, // [timestamp, open, high, low, close, volume]
}

#[derive(Debug, Deserialize)]
struct OhlcvMeta {
    base: TokenInfo,
    quote: TokenInfo,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub coingecko_coin_id: Option<String>,
}

/// OHLCV response containing candle data
#[derive(Debug, Clone)]
pub struct OhlcvResponse {
    /// List of OHLCV candles: [timestamp, open, high, low, close, volume]
    pub ohlcv_list: Vec<[f64; 6]>,
    pub base_token: TokenInfo,
    pub quote_token: TokenInfo,
}

impl OhlcvResponse {
    /// Get the number of candles
    pub fn len(&self) -> usize {
        self.ohlcv_list.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.ohlcv_list.is_empty()
    }

    /// Get a specific candle by index
    pub fn get_candle(&self, index: usize) -> Option<&[f64; 6]> {
        self.ohlcv_list.get(index)
    }

    /// Get the latest candle
    pub fn latest(&self) -> Option<&[f64; 6]> {
        self.ohlcv_list.first()
    }

    /// Get all timestamps
    pub fn timestamps(&self) -> Vec<i64> {
        self.ohlcv_list.iter().map(|c| c[0] as i64).collect()
    }

    /// Get all close prices
    pub fn close_prices(&self) -> Vec<f64> {
        self.ohlcv_list.iter().map(|c| c[4]).collect()
    }

    /// Get all volumes
    pub fn volumes(&self) -> Vec<f64> {
        self.ohlcv_list.iter().map(|c| c[5]).collect()
    }
}
