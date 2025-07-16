use super::SourceTrait;
use crate::types::TokenInfo;
use crate::Logger;
use anyhow::{ Context, Result };
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::time;

#[derive(Debug, Clone)]
pub struct DexScreenerSource {
    client: Client,
    rate_limit_delay: Duration,
}

#[derive(Debug, Deserialize)]
struct DexScreenerTokenProfile {
    #[serde(rename = "chainId")]
    chain_id: String,
    #[serde(rename = "tokenAddress")]
    token_address: String,
    url: String,
    icon: Option<String>,
    header: Option<String>,
    #[serde(rename = "openGraph")]
    open_graph: Option<String>,
    description: Option<String>,
    links: Option<Vec<ProfileLink>>,
}

#[derive(Debug, Deserialize)]
struct ProfileLink {
    #[serde(rename = "type")]
    link_type: Option<String>,
    label: Option<String>,
    url: String,
}

#[derive(Debug, Deserialize)]
struct DexScreenerBoost {
    #[serde(rename = "tokenAddress")]
    token_address: String,
    #[serde(rename = "totalAmount")]
    total_amount: Option<f64>,
    description: Option<String>,
}

impl DexScreenerSource {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            rate_limit_delay: Duration::from_millis(1000), // 60 requests per minute = 1 per second
        }
    }

    async fn fetch_latest_profiles(&self) -> Result<Vec<TokenInfo>> {
        let url = "https://api.dexscreener.com/token-profiles/latest/v1";

        let response = self.client
            .get(url)
            .send().await
            .context("Failed to fetch DexScreener token profiles")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("DexScreener API returned status: {}", response.status()));
        }

        let profiles: Vec<DexScreenerTokenProfile> = response
            .json().await
            .context("Failed to parse DexScreener token profiles")?;

        let mut tokens = Vec::new();
        for profile in profiles {
            // Only process Solana tokens
            if profile.chain_id == "solana" {
                if let Some(token) = self.convert_profile_to_token(profile) {
                    tokens.push(token);
                }
            }
        }

        Ok(tokens)
    }

    async fn fetch_latest_boosts(&self) -> Result<Vec<TokenInfo>> {
        time::sleep(self.rate_limit_delay).await;

        let url = "https://api.dexscreener.com/token-boosts/latest/v1";

        let response = self.client
            .get(url)
            .send().await
            .context("Failed to fetch DexScreener token boosts")?;

        if !response.status().is_success() {
            return Err(
                anyhow::anyhow!("DexScreener boosts API returned status: {}", response.status())
            );
        }

        let boosts: Vec<DexScreenerBoost> = response
            .json().await
            .context("Failed to parse DexScreener token boosts")?;

        // For boosts, we only have token addresses, so we need to fetch additional data
        // For now, we'll create minimal token info
        let mut tokens = Vec::new();
        for boost in boosts {
            let token = TokenInfo {
                mint: boost.token_address,
                symbol: "UNKNOWN".to_string(),
                name: boost.description.unwrap_or_else(|| "Boosted Token".to_string()),
                decimals: 9, // Default for Solana
                supply: 0,
                market_cap: None,
                price: None,
                volume_24h: None,
                liquidity: boost.total_amount,
                pool_address: None,
                discovered_at: Utc::now(),
                last_updated: Utc::now(),
                is_active: true,
            };
            tokens.push(token);
        }

        Ok(tokens)
    }

    async fn fetch_top_boosts(&self) -> Result<Vec<TokenInfo>> {
        time::sleep(self.rate_limit_delay).await;

        let url = "https://api.dexscreener.com/token-boosts/top/v1";

        let response = self.client
            .get(url)
            .send().await
            .context("Failed to fetch DexScreener top boosts")?;

        if !response.status().is_success() {
            return Err(
                anyhow::anyhow!("DexScreener top boosts API returned status: {}", response.status())
            );
        }

        let boosts: Vec<DexScreenerBoost> = response
            .json().await
            .context("Failed to parse DexScreener top boosts")?;

        let mut tokens = Vec::new();
        for boost in boosts {
            let token = TokenInfo {
                mint: boost.token_address,
                symbol: "UNKNOWN".to_string(),
                name: boost.description.unwrap_or_else(|| "Top Boosted Token".to_string()),
                decimals: 9,
                supply: 0,
                market_cap: None,
                price: None,
                volume_24h: None,
                liquidity: boost.total_amount,
                pool_address: None,
                discovered_at: Utc::now(),
                last_updated: Utc::now(),
                is_active: true,
            };
            tokens.push(token);
        }

        Ok(tokens)
    }

    fn convert_profile_to_token(&self, profile: DexScreenerTokenProfile) -> Option<TokenInfo> {
        // Skip if not a proper Solana token address
        if profile.token_address.len() != 44 {
            return None;
        }

        // Only process Solana tokens
        if profile.chain_id != "solana" {
            return None;
        }

        Some(TokenInfo {
            mint: profile.token_address,
            symbol: "UNKNOWN".to_string(), // Profile API doesn't include symbol
            name: profile.description.unwrap_or_else(|| "Token Profile".to_string()),
            decimals: 9, // Default for Solana tokens
            supply: 0, // Not provided in this API
            market_cap: None,
            price: None,
            volume_24h: None,
            liquidity: None,
            pool_address: None,
            discovered_at: Utc::now(),
            last_updated: Utc::now(),
            is_active: true,
        })
    }
}

#[async_trait]
impl SourceTrait for DexScreenerSource {
    fn name(&self) -> &str {
        "DexScreener"
    }

    async fn discover(&self) -> Result<Vec<TokenInfo>> {
        let mut all_tokens = Vec::new();

        // Fetch from all DexScreener endpoints
        match self.fetch_latest_profiles().await {
            Ok(mut tokens) => {
                all_tokens.append(&mut tokens);
            }
            Err(e) => {
                Logger::warn(&format!("Failed to fetch DexScreener profiles: {}", e));
            }
        }

        match self.fetch_latest_boosts().await {
            Ok(mut tokens) => {
                all_tokens.append(&mut tokens);
            }
            Err(e) => {
                Logger::warn(&format!("Failed to fetch DexScreener latest boosts: {}", e));
            }
        }

        match self.fetch_top_boosts().await {
            Ok(mut tokens) => {
                all_tokens.append(&mut tokens);
            }
            Err(e) => {
                Logger::warn(&format!("Failed to fetch DexScreener top boosts: {}", e));
            }
        }

        // Remove duplicates by mint address
        all_tokens.sort_by(|a, b| a.mint.cmp(&b.mint));
        all_tokens.dedup_by(|a, b| a.mint == b.mint);

        Ok(all_tokens)
    }
}
