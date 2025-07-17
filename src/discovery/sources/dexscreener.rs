use super::SourceTrait;
use anyhow::{ Context, Result };
use async_trait::async_trait;
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

    async fn fetch_latest_profiles(&self) -> Result<Vec<String>> {
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

        let mut mints = Vec::new();
        for profile in profiles {
            // Only process Solana tokens with valid mint addresses
            if profile.chain_id == "solana" && profile.token_address.len() == 44 {
                mints.push(profile.token_address);
            }
        }

        Ok(mints)
    }

    async fn fetch_latest_boosts(&self) -> Result<Vec<String>> {
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

        let mut mints = Vec::new();
        for boost in boosts {
            // Only process valid Solana token addresses
            if boost.token_address.len() == 44 {
                mints.push(boost.token_address);
            }
        }

        Ok(mints)
    }

    async fn fetch_top_boosts(&self) -> Result<Vec<String>> {
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

        let mut mints = Vec::new();
        for boost in boosts {
            // Only process valid Solana token addresses
            if boost.token_address.len() == 44 {
                mints.push(boost.token_address);
            }
        }

        Ok(mints)
    }
}

#[async_trait]
impl SourceTrait for DexScreenerSource {
    fn name(&self) -> &str {
        "DexScreener"
    }

    async fn discover_mints(&self) -> Result<Vec<String>> {
        let mut all_mints = Vec::new();

        // Fetch from all DexScreener endpoints
        match self.fetch_latest_profiles().await {
            Ok(mut mints) => {
                all_mints.append(&mut mints);
            }
            Err(_) => {
                // Silently continue on error
            }
        }

        match self.fetch_latest_boosts().await {
            Ok(mut mints) => {
                all_mints.append(&mut mints);
            }
            Err(_) => {
                // Silently continue on error
            }
        }

        match self.fetch_top_boosts().await {
            Ok(mut mints) => {
                all_mints.append(&mut mints);
            }
            Err(_) => {
                // Silently continue on error
            }
        }

        // Remove duplicates
        all_mints.sort();
        all_mints.dedup();

        Ok(all_mints)
    }
}
