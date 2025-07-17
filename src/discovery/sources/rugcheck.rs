use super::SourceTrait;
use anyhow::{ Context, Result };
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::time;

#[derive(Debug, Clone)]
pub struct RugCheckSource {
    client: Client,
    rate_limit_delay: Duration,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RugCheckToken {
    mint: String,
    decimals: Option<u8>,
    symbol: Option<String>,
    creator: Option<String>,
    #[serde(rename = "mintAuthority")]
    mint_authority: Option<String>,
    #[serde(rename = "freezeAuthority")]
    freeze_authority: Option<String>,
    program: Option<String>,
    #[serde(rename = "createAt")]
    create_at: Option<String>,
    #[serde(rename = "updatedAt")]
    updated_at: Option<String>,
    events: Option<serde_json::Value>,
}

impl RugCheckSource {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            rate_limit_delay: Duration::from_millis(500), // Conservative rate limiting
        }
    }

    async fn fetch_new_tokens(&self) -> Result<Vec<String>> {
        let url = "https://api.rugcheck.xyz/v1/stats/new_tokens";

        let response = self.client
            .get(url)
            .header("accept", "application/json")
            .send().await
            .context("Failed to fetch RugCheck new tokens")?;

        if !response.status().is_success() {
            return Err(
                anyhow::anyhow!("RugCheck new tokens API returned status: {}", response.status())
            );
        }

        let tokens: Vec<RugCheckToken> = response
            .json().await
            .context("Failed to parse RugCheck new tokens response")?;

        let mut mints = Vec::new();
        for token in tokens {
            // Only process valid Solana token addresses
            if token.mint.len() == 44 {
                mints.push(token.mint);
            }
        }

        Ok(mints)
    }

    async fn fetch_recent_tokens(&self) -> Result<Vec<String>> {
        time::sleep(self.rate_limit_delay).await;

        let url = "https://api.rugcheck.xyz/v1/stats/recent";

        let response = self.client
            .get(url)
            .header("accept", "application/json")
            .send().await
            .context("Failed to fetch RugCheck recent tokens")?;

        if !response.status().is_success() {
            return Err(
                anyhow::anyhow!("RugCheck recent tokens API returned status: {}", response.status())
            );
        }

        let tokens: Vec<RugCheckToken> = response
            .json().await
            .context("Failed to parse RugCheck recent tokens response")?;

        let mut mints = Vec::new();
        for token in tokens {
            // Only process valid Solana token addresses
            if token.mint.len() == 44 {
                mints.push(token.mint);
            }
        }

        Ok(mints)
    }

    async fn fetch_trending_tokens(&self) -> Result<Vec<String>> {
        time::sleep(self.rate_limit_delay).await;

        let url = "https://api.rugcheck.xyz/v1/stats/trending";

        let response = self.client
            .get(url)
            .header("accept", "application/json")
            .send().await
            .context("Failed to fetch RugCheck trending tokens")?;

        if !response.status().is_success() {
            return Err(
                anyhow::anyhow!(
                    "RugCheck trending tokens API returned status: {}",
                    response.status()
                )
            );
        }

        let tokens: Vec<RugCheckToken> = response
            .json().await
            .context("Failed to parse RugCheck trending tokens response")?;

        let mut mints = Vec::new();
        for token in tokens {
            // Only process valid Solana token addresses
            if token.mint.len() == 44 {
                mints.push(token.mint);
            }
        }

        Ok(mints)
    }

    async fn fetch_verified_tokens(&self) -> Result<Vec<String>> {
        time::sleep(self.rate_limit_delay).await;

        let url = "https://api.rugcheck.xyz/v1/stats/verified";

        let response = self.client
            .get(url)
            .header("accept", "application/json")
            .send().await
            .context("Failed to fetch RugCheck verified tokens")?;

        if !response.status().is_success() {
            return Err(
                anyhow::anyhow!(
                    "RugCheck verified tokens API returned status: {}",
                    response.status()
                )
            );
        }

        let tokens: Vec<RugCheckToken> = response
            .json().await
            .context("Failed to parse RugCheck verified tokens response")?;

        let mut mints = Vec::new();
        for token in tokens {
            // Only process valid Solana token addresses
            if token.mint.len() == 44 {
                mints.push(token.mint);
            }
        }

        Ok(mints)
    }
}

#[async_trait]
impl SourceTrait for RugCheckSource {
    fn name(&self) -> &str {
        "RugCheck"
    }

    async fn discover_mints(&self) -> Result<Vec<String>> {
        let mut all_mints = Vec::new();

        // Fetch from all RugCheck endpoints
        match self.fetch_new_tokens().await {
            Ok(mut mints) => {
                all_mints.append(&mut mints);
            }
            Err(_) => {
                // Silently continue on error
            }
        }

        match self.fetch_recent_tokens().await {
            Ok(mut mints) => {
                all_mints.append(&mut mints);
            }
            Err(_) => {
                // Silently continue on error
            }
        }

        match self.fetch_trending_tokens().await {
            Ok(mut mints) => {
                all_mints.append(&mut mints);
            }
            Err(_) => {
                // Silently continue on error
            }
        }

        match self.fetch_verified_tokens().await {
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
