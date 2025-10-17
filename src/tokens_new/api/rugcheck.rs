/// Rugcheck API client
use super::client::{HttpClient, RateLimiter};
use super::stats::ApiStatsTracker;
use super::types::RugcheckResponse;
use crate::tokens_new::types::{ApiError, RugcheckHolder, RugcheckInfo, RugcheckRisk};
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;

const RUGCHECK_BASE_URL: &str = "https://api.rugcheck.xyz/v1/tokens";

pub struct RugcheckClient {
    http_client: HttpClient,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl RugcheckClient {
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

    /// Fetch security report for a token
    pub async fn fetch_report(&self, mint: &str) -> Result<RugcheckInfo, ApiError> {
        if !self.enabled {
            return Err(ApiError::Disabled);
        }

        self.rate_limiter.acquire().await;
        let start = Instant::now();

        let url = format!("{}/{}/report", RUGCHECK_BASE_URL, mint);

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

        let api_response: RugcheckResponse = response.json().await.map_err(|e| {
            self.stats.record_request(false, elapsed);
            ApiError::InvalidResponse(e.to_string())
        })?;

        self.stats.record_request(true, elapsed).await;

        // Convert API response to domain type
        let risks = api_response
            .risks
            .unwrap_or_default()
            .into_iter()
            .map(|r| RugcheckRisk {
                name: r.name,
                value: r.value,
                description: r.description,
                score: r.score,
                level: r.level,
            })
            .collect();

        let top_holders = api_response
            .top_holders
            .unwrap_or_default()
            .into_iter()
            .map(|h| RugcheckHolder {
                address: h.address,
                amount: h.amount.to_string(),
                pct: h.pct,
                owner: h.owner,
                insider: h.insider.unwrap_or(false),
            })
            .collect();

        let token_meta = api_response.token_meta;
        let token = api_response.token;
        let transfer_fee = api_response.transfer_fee;

        Ok(RugcheckInfo {
            mint: api_response.mint,
            token_program: api_response.token_program,
            token_type: api_response.token_type,
            token_name: token_meta.as_ref().and_then(|t| t.name.clone()),
            token_symbol: token_meta.as_ref().and_then(|t| t.symbol.clone()),
            token_decimals: token.as_ref().and_then(|t| t.decimals),
            token_supply: token
                .as_ref()
                .and_then(|t| t.supply.map(|s| s.to_string())),
            token_uri: token_meta.as_ref().and_then(|t| t.uri.clone()),
            token_mutable: token_meta.as_ref().and_then(|t| t.mutable),
            token_update_authority: token_meta
                .as_ref()
                .and_then(|t| t.update_authority.clone()),
            mint_authority: api_response.mint_authority,
            freeze_authority: api_response.freeze_authority,
            creator: api_response.creator,
            creator_balance: api_response.creator_balance,
            creator_tokens: api_response.creator_tokens,
            score: api_response.score,
            score_normalised: api_response.score_normalised,
            rugged: api_response.rugged.unwrap_or(false),
            risks,
            total_markets: api_response.markets.as_ref().map(|m| m.len() as i64),
            total_market_liquidity: api_response.total_market_liquidity,
            total_stable_liquidity: api_response.total_stable_liquidity,
            total_lp_providers: api_response.total_lp_providers,
            total_holders: api_response.total_holders,
            top_holders,
            graph_insiders_detected: api_response.graph_insiders_detected,
            transfer_fee_pct: transfer_fee.as_ref().and_then(|t| t.pct),
            transfer_fee_max_amount: transfer_fee.as_ref().and_then(|t| t.max_amount.map(|a| a as i64)),
            transfer_fee_authority: transfer_fee.and_then(|t| t.authority),
            detected_at: api_response.detected_at,
            analyzed_at: api_response.analyzed_at,
            fetched_at: Utc::now(),
        })
    }
}
