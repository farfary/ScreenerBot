/// DexScreener API client
use super::client::{HttpClient, RateLimiter};
use super::stats::ApiStatsTracker;
use super::types::DexScreenerResponse;
use crate::tokens_new::types::{ApiError, DexScreenerPool, SocialLink, WebsiteLink};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;

const DEXSCREENER_BASE_URL: &str = "https://api.dexscreener.com/latest/dex";

pub struct DexScreenerClient {
    http_client: HttpClient,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl DexScreenerClient {
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

    /// Fetch all pools/pairs for a token
    pub async fn fetch_pools(&self, mint: &str) -> Result<Vec<DexScreenerPool>, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        self.rate_limiter.acquire().await;
        let start = Instant::now();

        let url = format!("{}/tokens/{}", DEXSCREENER_BASE_URL, mint);

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

        let api_response: DexScreenerResponse = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        // Convert API response to domain types
        let pools = api_response
            .pairs
            .into_iter()
            .map(|pair| {
                let labels = pair.labels.unwrap_or_default();
                let info = pair.info.unwrap_or_else(|| super::types::DexScreenerInfo {
                    image_url: None,
                    header: None,
                    open_graph: None,
                    websites: None,
                    socials: None,
                });

                let websites = info
                    .websites
                    .unwrap_or_default()
                    .into_iter()
                    .map(|w| WebsiteLink {
                        label: w.label,
                        url: w.url,
                    })
                    .collect();

                let socials = info
                    .socials
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| SocialLink {
                        link_type: s.social_type,
                        url: s.url,
                    })
                    .collect();

                let txns = pair.txns.as_ref();
                let volume = pair.volume.as_ref();
                let price_change = pair.price_change.as_ref();
                let liquidity = pair.liquidity.as_ref();

                DexScreenerPool {
                    mint: mint.to_string(),
                    pair_address: pair.pair_address,
                    chain_id: pair.chain_id,
                    dex_id: pair.dex_id,
                    url: pair.url,
                    base_token_address: pair.base_token.address,
                    base_token_name: pair.base_token.name,
                    base_token_symbol: pair.base_token.symbol,
                    quote_token_address: pair.quote_token.address,
                    quote_token_name: pair.quote_token.name,
                    quote_token_symbol: pair.quote_token.symbol,
                    price_native: pair.price_native,
                    price_usd: pair.price_usd.unwrap_or_else(|| "0".to_string()),
                    liquidity_usd: liquidity.and_then(|l| l.usd),
                    liquidity_base: liquidity.and_then(|l| l.base),
                    liquidity_quote: liquidity.and_then(|l| l.quote),
                    volume_m5: volume.and_then(|v| v.m5),
                    volume_h1: volume.and_then(|v| v.h1),
                    volume_h6: volume.and_then(|v| v.h6),
                    volume_h24: volume.and_then(|v| v.h24),
                    txns_m5_buys: txns.and_then(|t| t.m5.as_ref().and_then(|m| m.buys)),
                    txns_m5_sells: txns.and_then(|t| t.m5.as_ref().and_then(|m| m.sells)),
                    txns_h1_buys: txns.and_then(|t| t.h1.as_ref().and_then(|m| m.buys)),
                    txns_h1_sells: txns.and_then(|t| t.h1.as_ref().and_then(|m| m.sells)),
                    txns_h6_buys: txns.and_then(|t| t.h6.as_ref().and_then(|m| m.buys)),
                    txns_h6_sells: txns.and_then(|t| t.h6.as_ref().and_then(|m| m.sells)),
                    txns_h24_buys: txns.and_then(|t| t.h24.as_ref().and_then(|m| m.buys)),
                    txns_h24_sells: txns.and_then(|t| t.h24.as_ref().and_then(|m| m.sells)),
                    price_change_m5: price_change.and_then(|pc| pc.m5),
                    price_change_h1: price_change.and_then(|pc| pc.h1),
                    price_change_h6: price_change.and_then(|pc| pc.h6),
                    price_change_h24: price_change.and_then(|pc| pc.h24),
                    fdv: pair.fdv,
                    market_cap: pair.market_cap,
                    pair_created_at: pair.pair_created_at,
                    labels,
                    info_image_url: info.image_url,
                    info_header: info.header,
                    info_open_graph: info.open_graph,
                    info_websites: websites,
                    info_socials: socials,
                    fetched_at: Utc::now(),
                }
            })
            .collect();

        Ok(pools)
    }
}
