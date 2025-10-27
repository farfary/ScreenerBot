/// GeckoTerminal API client
///
/// API Documentation: https://www.geckoterminal.com/dex-api
///
/// Endpoints implemented:
/// 1. /networks/{network}/tokens/{token}/pools - Get all pools for a token (primary)
/// 2. /networks/{network}/trending_pools - Trending pools per network
/// 3. /networks/{network}/pools - Top pools per network
/// 4. /networks/{network}/pools/{address} - Pool details by address
/// 5. /networks/{network}/pools/multi/{addresses} - Multiple pools at once
/// 6. /networks/{network}/pools/{pool}/ohlcv/{timeframe} - OHLCV data
/// 7. /networks/{network}/dexes - Supported DEX list
/// 8. /networks/{network}/new_pools - Newly listed pools
/// 9. /networks/{network}/tokens/multi/{addresses} - Multiple token metadata
/// 10. /networks/{network}/tokens/{address}/info - Token metadata
/// 11. /tokens/info_recently_updated - Recent token updates (global)
/// 12. /networks/{network}/pools/{pool_address}/trades - Recent pool trades
pub mod types;

// Re-export types for external use
pub use self::types::{
    GeckoTerminalDexesResponse, GeckoTerminalPool, GeckoTerminalRecentlyUpdatedResponse,
    GeckoTerminalResponse, GeckoTerminalTokenInfoResponse, GeckoTerminalTokensMultiResponse,
    GeckoTerminalTradesResponse,
};

use crate::apis::client::RateLimiter;
use crate::apis::stats::ApiStatsTracker;
use crate::logger::{self, LogTag};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// API CONFIGURATION - Hardcoded for GeckoTerminal API
// ============================================================================

const GECKOTERMINAL_BASE_URL: &str = "https://api.geckoterminal.com/api/v2";

/// Default network for Solana operations
const DEFAULT_NETWORK: &str = "solana";

/// Maximum page number for trending pools pagination
const MAX_TRENDING_PAGE: u32 = 10;

/// Request timeout in seconds - GeckoTerminal can have latency spikes, 10s is safe
pub const TIMEOUT_SECS: u64 = 10;

/// Rate limit per minute - GeckoTerminal has strict limits, 30/min is safe
pub const RATE_LIMIT_PER_MINUTE: usize = 30;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// GeckoTerminal API client with rate limiting and stats tracking
pub struct GeckoTerminalClient {
    client: Client,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    timeout: Duration,
    enabled: bool,
}

impl GeckoTerminalClient {
    pub fn new(enabled: bool, rate_limit: usize, timeout_seconds: u64) -> Result<Self, String> {
        if timeout_seconds == 0 {
            return Err("Timeout must be greater than zero".to_string());
        }

        Ok(Self {
            client: Client::new(),
            rate_limiter: RateLimiter::new(rate_limit),
            stats: Arc::new(ApiStatsTracker::new()),
            timeout: Duration::from_secs(timeout_seconds),
            enabled,
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub async fn get_stats(&self) -> crate::apis::stats::ApiStats {
        self.stats.get_stats().await
    }

    fn ensure_enabled(&self, endpoint: &str) -> Result<(), String> {
        if self.enabled {
            Ok(())
        } else {
            Err(format!(
                "GeckoTerminal client disabled via configuration (endpoint={})",
                endpoint
            ))
        }
    }

    async fn execute_request(
        &self,
        endpoint: &str,
        builder: reqwest::RequestBuilder,
    ) -> Result<(reqwest::Response, f64), String> {
        self.ensure_enabled(endpoint)?;

        let guard = self
            .rate_limiter
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
                        "GeckoTerminal",
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
    ) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        let (mut response, elapsed) = self.execute_request(endpoint, builder).await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            self.stats.record_request(false, elapsed).await;
            self.stats
                .record_error_with_event(
                    "GeckoTerminal",
                    endpoint,
                    format!("HTTP {}: {}", status, body),
                )
                .await;
            // Simple 429 backoff to avoid hammering when provider clamps down
            if status.as_u16() == 429 {
                // Sleep briefly to cool down; tuneable if needed
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            return Err(format!("GeckoTerminal API error {}: {}", status, body));
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
                        "GeckoTerminal",
                        endpoint,
                        format!("Parse error: {}", err),
                    )
                    .await;
                Err(format!("Failed to parse response: {}", err))
            }
        }
    }

    /// Fetch all pools for a single token address
    pub async fn fetch_pools(&self, mint: &str) -> Result<Vec<GeckoTerminalPool>, String> {
        self.fetch_pools_on_network(mint, None).await
    }

    /// Fetch pools for a token on a specific network
    pub async fn fetch_pools_on_network(
        &self,
        mint: &str,
        network: Option<&str>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let endpoint = format!("networks/{}/tokens/{}/pools", network_id, mint);
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching pools: token={}, network={}",
                mint, network_id
            ),
        );

        let api_response: GeckoTerminalResponse =
            self.get_json(&endpoint, self.client.get(&url)).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool(mint))
            .collect())
    }

    /// Get top pools by token address with optional sorting/filtering
    pub async fn fetch_top_pools_by_token(
        &self,
        token_address: &str,
        network: &str,
        include: Option<&str>,
        page: Option<u32>,
        sort: Option<&str>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let endpoint = format!("networks/{}/tokens/{}/pools", network, token_address);
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(inc) = include {
            query_params.push(("include".to_string(), inc.to_string()));
        }
        if let Some(p) = page {
            query_params.push(("page".to_string(), p.to_string()));
        }
        if let Some(s) = sort {
            query_params.push(("sort".to_string(), s.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching top pools by token: token={}, network={}, page={:?}, sort={:?}",
                token_address, network, page, sort
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool(token_address))
            .collect())
    }

    /// Get trending pools by network
    pub async fn fetch_trending_pools_by_network(
        &self,
        network: Option<&str>,
        page: Option<u32>,
        duration: Option<&str>,
        include: Option<Vec<&str>>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let endpoint = format!("networks/{}/trending_pools", network_id);
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(p) = page {
            query_params.push(("page".to_string(), p.min(MAX_TRENDING_PAGE).to_string()));
        }
        if let Some(d) = duration {
            query_params.push(("duration".to_string(), d.to_string()));
        }
        if let Some(includes) = include {
            if !includes.is_empty() {
                query_params.push(("include".to_string(), includes.join(",")));
            }
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching trending pools: network={}, page={:?}, duration={:?}",
                network_id, page, duration
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool("trending"))
            .collect())
    }

    /// Get top pools by network
    pub async fn fetch_top_pools_by_network(
        &self,
        network: Option<&str>,
        include: Option<Vec<&str>>,
        page: Option<u32>,
        sort: Option<&str>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let endpoint = format!("networks/{}/pools", network_id);
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(p) = page {
            let page_num = p.min(10).max(1);
            query_params.push(("page".to_string(), page_num.to_string()));
        }
        if let Some(s) = sort {
            query_params.push(("sort".to_string(), s.to_string()));
        }
        if let Some(includes) = include {
            if !includes.is_empty() {
                query_params.push(("include".to_string(), includes.join(",")));
            }
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching top pools: network={}, page={:?}, sort={:?}",
                network_id, page, sort
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool("top_pools"))
            .collect())
    }

    /// Get specific pool data by address
    pub async fn fetch_pool_by_address(
        &self,
        network: Option<&str>,
        pool_address: &str,
        include: Option<Vec<&str>>,
        include_volume_breakdown: bool,
        include_composition: bool,
    ) -> Result<GeckoTerminalPool, String> {
        let network_id = network.unwrap_or(DEFAULT_NETWORK);
        let endpoint = format!("networks/{}/pools/{}", network_id, pool_address);
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(includes) = include {
            if !includes.is_empty() {
                query_params.push(("include".to_string(), includes.join(",")));
            }
        }
        if include_volume_breakdown {
            query_params.push(("include_volume_breakdown".to_string(), "true".to_string()));
        }
        if include_composition {
            query_params.push(("include_composition".to_string(), "true".to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching pool: network={}, address={}",
                network_id, pool_address
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        api_response
            .data
            .into_iter()
            .next()
            .map(|p| p.to_pool(pool_address))
            .ok_or_else(|| "No pool data returned".to_string())
    }

    /// Fetch multiple pools in one call (max 30 pool addresses)
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
        let address_count = addresses.len();
        let addresses_str = addresses.join(",");
        let endpoint = format!("networks/{}/pools/multi/{}", network_id, addresses_str);
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(includes) = include {
            if !includes.is_empty() {
                query_params.push(("include".to_string(), includes.join(",")));
            }
        }
        if include_volume_breakdown {
            query_params.push(("include_volume_breakdown".to_string(), "true".to_string()));
        }
        if include_composition {
            query_params.push(("include_composition".to_string(), "true".to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching multi pools: network={}, count={}",
                network_id, address_count
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool("multi"))
            .collect())
    }

    /// Fetch OHLCV candlestick data for a pool
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
        let endpoint = format!(
            "networks/{}/pools/{}/ohlcv/{}",
            network, pool_address, timeframe
        );
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(agg) = aggregate {
            query_params.push(("aggregate".to_string(), agg.to_string()));
        }
        if let Some(lim) = limit {
            query_params.push(("limit".to_string(), lim.min(1000).to_string()));
        }
        if let Some(curr) = currency {
            query_params.push(("currency".to_string(), curr.to_string()));
        }
        if let Some(ts) = before_timestamp {
            query_params.push(("before_timestamp".to_string(), ts.to_string()));
        }
        if let Some(tok) = token {
            query_params.push(("token".to_string(), tok.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching OHLCV: network={}, pool={}, timeframe={}, aggregate={:?}, limit={:?}",
                network, pool_address, timeframe, aggregate, limit
            ),
        );

        let ohlcv_response: OhlcvResponseRaw = self.get_json(&endpoint, builder).await?;

        Ok(OhlcvResponse {
            ohlcv_list: ohlcv_response.data.attributes.ohlcv_list,
            base_token: ohlcv_response.meta.base,
            quote_token: ohlcv_response.meta.quote,
        })
    }

    /// Get supported DEX list for a network
    pub async fn fetch_dexes_by_network(
        &self,
        network: &str,
        page: Option<u32>,
    ) -> Result<Vec<(String, String)>, String> {
        let endpoint = format!("networks/{}/dexes", network);
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        let builder = if let Some(p) = page {
            self.client
                .get(&url)
                .query(&[("page".to_string(), p.to_string())])
        } else {
            self.client.get(&url)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching DEXes: network={}, page={:?}",
                network, page
            ),
        );

        let dex_response: GeckoTerminalDexesResponse = self.get_json(&endpoint, builder).await?;

        Ok(dex_response
            .data
            .into_iter()
            .map(|d| (d.id, d.attributes.name))
            .collect())
    }

    /// Fetch latest newly created pools on a network
    pub async fn fetch_new_pools_by_network(
        &self,
        network: &str,
        include: Option<&str>,
        page: Option<u32>,
    ) -> Result<Vec<GeckoTerminalPool>, String> {
        let endpoint = format!("networks/{}/new_pools", network);
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(inc) = include {
            query_params.push(("include".to_string(), inc.to_string()));
        }
        if let Some(p) = page {
            query_params.push(("page".to_string(), p.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching new pools: network={}, page={:?}",
                network, page
            ),
        );

        let api_response: GeckoTerminalResponse = self.get_json(&endpoint, builder).await?;

        Ok(api_response
            .data
            .into_iter()
            .map(|p| p.to_pool("new_pools"))
            .collect())
    }

    /// Fetch multiple token metadata entries
    pub async fn fetch_tokens_multi(
        &self,
        network: &str,
        addresses: &str,
        include: Option<&str>,
        include_composition: Option<bool>,
    ) -> Result<GeckoTerminalTokensMultiResponse, String> {
        let endpoint = format!("networks/{}/tokens/multi/{}", network, addresses);
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(inc) = include {
            query_params.push(("include".to_string(), inc.to_string()));
        }
        if let Some(comp) = include_composition {
            query_params.push(("include_composition".to_string(), comp.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching tokens multi: network={}, addresses_count={}",
                network,
                addresses.split(',').count()
            ),
        );

        self.get_json(&endpoint, builder).await
    }

    /// Fetch token metadata for a single address
    pub async fn fetch_token_info(
        &self,
        network: &str,
        address: &str,
    ) -> Result<GeckoTerminalTokenInfoResponse, String> {
        let endpoint = format!("networks/{}/tokens/{}/info", network, address);
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching token info: network={}, address={}",
                network, address
            ),
        );

        self.get_json(&endpoint, self.client.get(&url)).await
    }

    /// Fetch recently updated tokens (global endpoint)
    pub async fn fetch_recently_updated_tokens(
        &self,
        include: Option<&str>,
        network: Option<&str>,
    ) -> Result<GeckoTerminalRecentlyUpdatedResponse, String> {
        let endpoint = "tokens/info_recently_updated";
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(inc) = include {
            query_params.push(("include".to_string(), inc.to_string()));
        }
        if let Some(net) = network {
            query_params.push(("network".to_string(), net.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching recently updated tokens: network={:?}",
                network
            ),
        );

        self.get_json(endpoint, builder).await
    }

    /// Fetch trades for a pool in the last 24 hours
    pub async fn fetch_pool_trades(
        &self,
        network: &str,
        pool_address: &str,
        trade_volume_in_usd_greater_than: Option<f64>,
        token: Option<&str>,
    ) -> Result<GeckoTerminalTradesResponse, String> {
        let endpoint = format!("networks/{}/pools/{}/trades", network, pool_address);
        let url = format!("{}/{}", GECKOTERMINAL_BASE_URL, endpoint);

        let mut query_params: Vec<(String, String)> = Vec::new();
        if let Some(min_volume) = trade_volume_in_usd_greater_than {
            query_params.push((
                "trade_volume_in_usd_greater_than".to_string(),
                min_volume.to_string(),
            ));
        }
        if let Some(tok) = token {
            query_params.push(("token".to_string(), tok.to_string()));
        }

        let builder = if query_params.is_empty() {
            self.client.get(&url)
        } else {
            self.client.get(&url).query(&query_params)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "[GECKOTERMINAL] Fetching pool trades: network={}, pool={}, min_volume={:?}",
                network, pool_address, trade_volume_in_usd_greater_than
            ),
        );

        self.get_json(&endpoint, builder).await
    }
}

// ============================================================================
// OHLCV Response Types
// ============================================================================

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
    ohlcv_list: Vec<[f64; 6]>,
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
    pub ohlcv_list: Vec<[f64; 6]>,
    pub base_token: TokenInfo,
    pub quote_token: TokenInfo,
}

impl OhlcvResponse {
    pub fn len(&self) -> usize {
        self.ohlcv_list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ohlcv_list.is_empty()
    }

    pub fn get_candle(&self, index: usize) -> Option<&[f64; 6]> {
        self.ohlcv_list.get(index)
    }

    pub fn latest(&self) -> Option<&[f64; 6]> {
        self.ohlcv_list.first()
    }

    pub fn timestamps(&self) -> Vec<i64> {
        self.ohlcv_list.iter().map(|c| c[0] as i64).collect()
    }

    pub fn close_prices(&self) -> Vec<f64> {
        self.ohlcv_list.iter().map(|c| c[4]).collect()
    }

    pub fn volumes(&self) -> Vec<f64> {
        self.ohlcv_list.iter().map(|c| c[5]).collect()
    }
}
