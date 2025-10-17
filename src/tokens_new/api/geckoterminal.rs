/// Complete GeckoTerminal API client with ALL available endpoints
/// 
/// API Documentation: https://www.geckoterminal.com/dex-api
/// 
/// Endpoints implemented (verified working):
/// 1. /networks/{network}/tokens/{token}/pools - PRIMARY: Get all pools for a token
/// 2. /networks/{network}/trending_pools - Get trending pools by network
/// 3. /networks/{network}/pools/{address} - Get specific pool data by pool address
/// 4. /networks/{network}/pools/{pool}/ohlcv/{timeframe} - Get OHLCV candlestick data

use super::types::GeckoTerminalResponse;
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
