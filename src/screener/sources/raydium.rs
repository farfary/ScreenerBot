//! Raydium API integration for token discovery

use super::TokenSource;
use crate::core::{
    BotResult,
    BotError,
    TokenOpportunity,
    TokenInfo,
    TokenMetrics,
    VerificationStatus,
    ScreenerSource,
    LiquidityProvider,
    SocialMetrics,
};
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use solana_sdk::pubkey::Pubkey;
use std::{ time::Duration, str::FromStr };
use chrono::Utc;

/// Raydium API response structures
#[derive(Debug, Deserialize)]
struct RaydiumResponse {
    success: bool,
    data: RaydiumData,
}

#[derive(Debug, Deserialize)]
struct RaydiumData {
    count: u32,
    data: Vec<RaydiumPool>,
}

#[derive(Debug, Deserialize)]
struct RaydiumPool {
    id: String,
    #[serde(rename = "mintA")]
    mint_a: RaydiumToken,
    #[serde(rename = "mintB")]
    mint_b: RaydiumToken,
    tvl: f64,
    price: f64,
    #[serde(rename = "mintAmountA")]
    mint_amount_a: f64,
    #[serde(rename = "mintAmountB")]
    mint_amount_b: f64,
    #[serde(rename = "feeRate")]
    fee_rate: f64,
    day: Option<RaydiumPeriodStats>,
    week: Option<RaydiumPeriodStats>,
    month: Option<RaydiumPeriodStats>,
}

#[derive(Debug, Deserialize)]
struct RaydiumToken {
    address: String,
    symbol: String,
    name: String,
    decimals: u8,
    #[serde(rename = "logoURI")]
    logo_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RaydiumPeriodStats {
    volume: f64,
    #[serde(rename = "volumeQuote")]
    volume_quote: f64,
    #[serde(rename = "priceMin")]
    price_min: f64,
    #[serde(rename = "priceMax")]
    price_max: f64,
}

/// Raydium API source
pub struct RaydiumSource {
    client: Client,
    base_url: String,
}

impl RaydiumSource {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ScreenerBot/1.0")
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://api-v3.raydium.io".to_string(),
        }
    }

    /// Check if a token is paired with SOL/WSOL
    fn is_sol_pool(&self, pool: &RaydiumPool) -> bool {
        const SOL_ADDRESS: &str = "So11111111111111111111111111111111111111112";
        pool.mint_a.address == SOL_ADDRESS || pool.mint_b.address == SOL_ADDRESS
    }

    /// Check if a token is a known stablecoin
    fn is_known_stablecoin(&self, symbol: &str) -> bool {
        matches!(symbol.to_uppercase().as_str(), "USDC" | "USDT" | "DAI" | "BUSD" | "FRAX" | "TUSD")
    }

    /// Extract token information from a pool that's paired with SOL
    fn extract_token_from_pool(&self, pool: &RaydiumPool) -> Option<TokenOpportunity> {
        const SOL_ADDRESS: &str = "So11111111111111111111111111111111111111112";
        const SOL_PRICE_USD: f64 = 159.0; // Approximate SOL price in USD

        // Determine which token is the non-SOL token and get reserves
        let (token_info, sol_reserve, token_reserve) = if pool.mint_a.address == SOL_ADDRESS {
            (&pool.mint_b, pool.mint_amount_a, pool.mint_amount_b)
        } else if pool.mint_b.address == SOL_ADDRESS {
            (&pool.mint_a, pool.mint_amount_b, pool.mint_amount_a)
        } else {
            return None; // Not a SOL pool
        };

        // Parse the token mint address
        let mint = match Pubkey::from_str(&token_info.address) {
            Ok(mint) => mint,
            Err(e) => {
                log::warn!("Failed to parse mint address {}: {}", token_info.address, e);
                return None;
            }
        };

        // Calculate token price in USD based on reserves
        let token_price_usd = if self.is_known_stablecoin(&token_info.symbol) {
            // For known stablecoins, price should be around $1
            1.0
        } else if token_reserve > 0.0 && sol_reserve > 0.0 {
            // For other tokens, calculate based on SOL reserves
            // Price per token = (SOL reserve / Token reserve) * SOL price in USD
            let sol_per_token = sol_reserve / token_reserve;
            
            // Adjust for decimals - convert both reserves to their actual amounts
            let sol_decimals = 9;
            let token_decimals = token_info.decimals as i32;
            
            let actual_sol_reserve = sol_reserve / 10_f64.powi(sol_decimals);
            let actual_token_reserve = token_reserve / 10_f64.powi(token_decimals);
            
            if actual_token_reserve > 0.0 {
                (actual_sol_reserve / actual_token_reserve) * SOL_PRICE_USD
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Calculate price change (using daily stats if available)
        let price_change_24h = pool.day.as_ref().map(|day| {
            if day.price_min > 0.0 {
                ((day.price_max - day.price_min) / day.price_min) * 100.0
            } else {
                0.0
            }
        });

        // Get 24h volume
        let volume_24h = pool.day
            .as_ref()
            .map(|day| day.volume_quote)
            .unwrap_or(0.0);

        // Calculate age in hours (using current time as we don't have creation time)
        let age_hours = 24.0; // Default to 24 hours as we don't have exact creation time

        let token_opportunity = TokenOpportunity {
            mint,
            token: TokenInfo {
                mint,
                symbol: token_info.symbol.clone(),
                name: token_info.name.clone(),
            },
            symbol: token_info.symbol.clone(),
            name: token_info.name.clone(),
            source: ScreenerSource::Raydium,
            discovery_time: Utc::now(),
            metrics: TokenMetrics {
                price_usd: token_price_usd,
                volume_24h,
                liquidity_usd: pool.tvl,
                market_cap: None, // Not provided by this API
                price_change_24h,
                age_hours,
                holder_count: None, // Not provided by this API
                top_10_holder_percentage: None, // Not provided by this API
            },
            verification_status: VerificationStatus {
                is_verified: false, // Default value
                has_profile: token_info.logo_uri.is_some(),
                is_boosted: false, // Default value
                rugcheck_score: None,
                security_flags: Vec::new(),
                has_socials: false, // Default value
                contract_verified: false, // Default value
            },
            risk_score: 0.5, // Default medium risk
            confidence_score: 0.7, // Default confidence
            liquidity_provider: LiquidityProvider::Raydium,
            social_metrics: Some(SocialMetrics {
                twitter_followers: None,
                telegram_members: None,
                website_url: None,
                twitter_url: None,
                telegram_url: None,
            }),
            risk_factors: Vec::new(),
        };

        Some(token_opportunity)
    }
}

#[async_trait::async_trait]
impl TokenSource for RaydiumSource {
    fn name(&self) -> &str {
        "Raydium"
    }

    async fn initialize(&mut self) -> BotResult<()> {
        log::info!("✅ Raydium source initialized");
        Ok(())
    }

    async fn get_new_tokens(&self) -> BotResult<Vec<TokenOpportunity>> {
        log::info!("🔍 Fetching new tokens from Raydium API...");

        let url = format!(
            "{}/pools/info/list?poolType=all&poolSortField=liquidity&sortType=desc&pageSize=100&page=1",
            self.base_url
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(
                BotError::Api(format!("Raydium API returned status: {}", response.status()))
            );
        }

        let raydium_response: RaydiumResponse = response.json().await?;

        if !raydium_response.success {
            return Err(BotError::Api("Raydium API returned success: false".to_string()));
        }

        let mut tokens = Vec::new();
        let mut sol_pools_found = 0;

        for pool in raydium_response.data.data {
            if self.is_sol_pool(&pool) {
                sol_pools_found += 1;
                if let Some(token) = self.extract_token_from_pool(&pool) {
                    tokens.push(token);
                }
            }
        }

        log::info!(
            "✅ Found {} SOL pools out of {} total pools, extracted {} tokens",
            sol_pools_found,
            raydium_response.data.count,
            tokens.len()
        );

        Ok(tokens)
    }

    async fn get_token_info(&self, mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        // For now, we'll return None as the pools API doesn't support single token lookup
        // This could be enhanced later with a different endpoint
        log::debug!("Token info lookup for {} not implemented in Raydium source", mint);
        Ok(None)
    }

    async fn health_check(&self) -> BotResult<bool> {
        let url = format!("{}/pools/info/list?pageSize=1&page=1", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                log::warn!("Raydium health check failed: {}", e);
                Ok(false)
            }
        }
    }
}
