/// GeckoTerminal API client
use super::client::{HttpClient, RateLimiter};
use super::stats::ApiStatsTracker;
use super::types::GeckoTerminalResponse;
use crate::tokens_new::types::{ApiError, GeckoTerminalPool};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;

const GECKOTERMINAL_BASE_URL: &str = "https://api.geckoterminal.com/api/v2";

pub struct GeckoTerminalClient {
    http_client: HttpClient,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl GeckoTerminalClient {
    pub fn new(enabled: bool, rate_limit_per_minute: usize, timeout_secs: u64) -> Result<Self, String> {
        let http_client = HttpClient::new(timeout_secs)?;
        let rate_limiter = RateLimiter::new(rate_limit_per_minute);
        let stats = Arc::new(ApiStatsTracker::new());

        Ok(Self {
            http_client,
            rate_limiter,
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

    /// Fetch all pools for a token
    pub async fn fetch_pools(&self, mint: &str) -> Result<Vec<GeckoTerminalPool>, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        self.rate_limiter.acquire().await;
        let start = Instant::now();

        let url = format!(
            "{}/networks/solana/tokens/{}/pools",
            GECKOTERMINAL_BASE_URL, mint
        );

        let response = self
            .http_client
            .client()
            .get(&url)
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
            if response.status() == 404 {
                return Err(ApiError::NotFound);
            }
            return Err(ApiError::InvalidResponse(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let api_response: GeckoTerminalResponse = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        // Convert API response to domain types
        let pools = api_response
            .data
            .into_iter()
            .map(|pool_data| {
                let attrs = pool_data.attributes;
                let rels = pool_data.relationships;

                let parse_f64 = |s: &str| s.parse::<f64>().ok();
                let parse_i64 = |s: &str| s.parse::<i64>().ok();

                let price_change = attrs.price_change_percentage.as_ref();
                let transactions = attrs.transactions.as_ref();
                let volume = attrs.volume_usd.as_ref();

                GeckoTerminalPool {
                    mint: mint.to_string(),
                    pool_address: attrs.address,
                    pool_name: attrs.name,
                    dex_id: rels.dex.data.id,
                    base_token_id: rels.base_token.data.id,
                    quote_token_id: rels.quote_token.data.id,
                    base_token_price_usd: attrs.base_token_price_usd,
                    base_token_price_native: attrs.base_token_price_native_currency,
                    base_token_price_quote: attrs.base_token_price_quote_token,
                    quote_token_price_usd: attrs.quote_token_price_usd,
                    quote_token_price_native: attrs.quote_token_price_native_currency,
                    quote_token_price_base: attrs.quote_token_price_base_token,
                    token_price_usd: attrs.token_price_usd,
                    fdv_usd: attrs.fdv_usd.and_then(|s| parse_f64(&s)),
                    market_cap_usd: attrs.market_cap_usd.and_then(|s| parse_f64(&s)),
                    reserve_usd: attrs.reserve_in_usd.and_then(|s| parse_f64(&s)),
                    volume_m5: volume.and_then(|v| v.m5.as_ref().and_then(|s| parse_f64(s))),
                    volume_m15: volume.and_then(|v| v.m15.as_ref().and_then(|s| parse_f64(s))),
                    volume_m30: volume.and_then(|v| v.m30.as_ref().and_then(|s| parse_f64(s))),
                    volume_h1: volume.and_then(|v| v.h1.as_ref().and_then(|s| parse_f64(s))),
                    volume_h6: volume.and_then(|v| v.h6.as_ref().and_then(|s| parse_f64(s))),
                    volume_h24: volume.and_then(|v| v.h24.as_ref().and_then(|s| parse_f64(s))),
                    price_change_m5: price_change.and_then(|pc| pc.m5.as_ref().and_then(|s| parse_f64(s))),
                    price_change_m15: price_change.and_then(|pc| pc.m15.as_ref().and_then(|s| parse_f64(s))),
                    price_change_m30: price_change.and_then(|pc| pc.m30.as_ref().and_then(|s| parse_f64(s))),
                    price_change_h1: price_change.and_then(|pc| pc.h1.as_ref().and_then(|s| parse_f64(s))),
                    price_change_h6: price_change.and_then(|pc| pc.h6.as_ref().and_then(|s| parse_f64(s))),
                    price_change_h24: price_change.and_then(|pc| pc.h24.as_ref().and_then(|s| parse_f64(s))),
                    txns_m5_buys: transactions.and_then(|t| t.m5.as_ref().and_then(|p| p.buys)),
                    txns_m5_sells: transactions.and_then(|t| t.m5.as_ref().and_then(|p| p.sells)),
                    txns_m5_buyers: transactions.and_then(|t| t.m5.as_ref().and_then(|p| p.buyers)),
                    txns_m5_sellers: transactions.and_then(|t| t.m5.as_ref().and_then(|p| p.sellers)),
                    txns_m15_buys: transactions.and_then(|t| t.m15.as_ref().and_then(|p| p.buys)),
                    txns_m15_sells: transactions.and_then(|t| t.m15.as_ref().and_then(|p| p.sells)),
                    txns_m15_buyers: transactions.and_then(|t| t.m15.as_ref().and_then(|p| p.buyers)),
                    txns_m15_sellers: transactions.and_then(|t| t.m15.as_ref().and_then(|p| p.sellers)),
                    txns_m30_buys: transactions.and_then(|t| t.m30.as_ref().and_then(|p| p.buys)),
                    txns_m30_sells: transactions.and_then(|t| t.m30.as_ref().and_then(|p| p.sells)),
                    txns_m30_buyers: transactions.and_then(|t| t.m30.as_ref().and_then(|p| p.buyers)),
                    txns_m30_sellers: transactions.and_then(|t| t.m30.as_ref().and_then(|p| p.sellers)),
                    txns_h1_buys: transactions.and_then(|t| t.h1.as_ref().and_then(|p| p.buys)),
                    txns_h1_sells: transactions.and_then(|t| t.h1.as_ref().and_then(|p| p.sells)),
                    txns_h1_buyers: transactions.and_then(|t| t.h1.as_ref().and_then(|p| p.buyers)),
                    txns_h1_sellers: transactions.and_then(|t| t.h1.as_ref().and_then(|p| p.sellers)),
                    txns_h6_buys: transactions.and_then(|t| t.h6.as_ref().and_then(|p| p.buys)),
                    txns_h6_sells: transactions.and_then(|t| t.h6.as_ref().and_then(|p| p.sells)),
                    txns_h6_buyers: transactions.and_then(|t| t.h6.as_ref().and_then(|p| p.buyers)),
                    txns_h6_sellers: transactions.and_then(|t| t.h6.as_ref().and_then(|p| p.sellers)),
                    txns_h24_buys: transactions.and_then(|t| t.h24.as_ref().and_then(|p| p.buys)),
                    txns_h24_sells: transactions.and_then(|t| t.h24.as_ref().and_then(|p| p.sells)),
                    txns_h24_buyers: transactions.and_then(|t| t.h24.as_ref().and_then(|p| p.buyers)),
                    txns_h24_sellers: transactions.and_then(|t| t.h24.as_ref().and_then(|p| p.sellers)),
                    pool_created_at: attrs.pool_created_at,
                    fetched_at: Utc::now(),
                }
            })
            .collect();

        Ok(pools)
    }

    /// Fetch OHLCV (Open, High, Low, Close, Volume) data for a pool
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
    /// OhlcvResponse with list of [timestamp, open, high, low, close, volume]
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
    ) -> Result<OhlcvResponse, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        self.rate_limiter.acquire().await;
        let start = Instant::now();

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

        let response = self
            .http_client
            .client()
            .get(&url)
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
            if response.status() == 404 {
                return Err(ApiError::NotFound);
            }
            return Err(ApiError::InvalidResponse(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let ohlcv_response: OhlcvResponseRaw = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        Ok(OhlcvResponse {
            ohlcv_list: ohlcv_response.data.attributes.ohlcv_list,
            base_token: ohlcv_response.meta.base,
            quote_token: ohlcv_response.meta.quote,
        })
    }
}

// ===== OHLCV Response Types =====

use serde::Deserialize;

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
