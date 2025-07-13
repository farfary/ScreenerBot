use crate::core::{
    BotResult,
    BotError,
    TokenOpportunity,
    ScreenerSource,
    TokenMetrics,
    VerificationStatus,
};
use crate::screener::{ TokenSource, BaseTokenOpportunity };
use reqwest::Client;
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use chrono::Utc;
use std::time::Duration;

/// DexScreener API source for token discovery
pub struct DexScreenerSource {
    client: Client,
    base_url: String,
}

impl DexScreenerSource {
    pub fn new() -> Self {
        Self {
            client: Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
            base_url: crate::core::DEXSCREENER_API_BASE.to_string(),
        }
    }

    async fn get_trending_tokens(&self) -> BotResult<Vec<Value>> {
        let url = format!("{}/dex/tokens/trending", self.base_url);

        let response = self.client
            .get(&url)
            .header("User-Agent", "ScreenerBot/1.0")
            .send().await
            .map_err(|e| BotError::Network(format!("DexScreener request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(BotError::Network(format!("DexScreener API error: {}", response.status())));
        }

        let data: Value = response
            .json().await
            .map_err(|e| BotError::Parse(format!("Failed to parse DexScreener response: {}", e)))?;

        Ok(
            data["tokens"]
                .as_array()
                .unwrap_or(&vec![])
                .clone()
        )
    }

    async fn parse_dexscreener_token(
        &self,
        token_data: &Value
    ) -> BotResult<Option<TokenOpportunity>> {
        let mint_str = token_data["address"]
            .as_str()
            .ok_or_else(|| BotError::Parse("Missing token address".to_string()))?;

        let symbol = token_data["symbol"].as_str().unwrap_or("UNKNOWN").to_string();
        let name = token_data["name"].as_str().unwrap_or("Unknown Token").to_string();

        // Parse metrics
        let price_usd = token_data["priceUsd"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let volume_24h = token_data["volume"]["h24"].as_f64().unwrap_or(0.0);
        let liquidity = token_data["liquidity"]["usd"].as_f64().unwrap_or(0.0);
        let market_cap = token_data["fdv"].as_f64();
        let price_change_24h = token_data["priceChange"]["h24"].as_f64();

        let metrics = TokenMetrics {
            price_usd,
            volume_24h,
            liquidity_usd: liquidity,
            market_cap,
            price_change_24h,
            age_hours: 1.0, // Assume new for trending
            holder_count: None,
            top_10_holder_percentage: None,
        };

        // Basic verification (DexScreener doesn't provide detailed verification)
        let verification = VerificationStatus {
            is_verified: false,
            has_profile: token_data["info"].is_object(),
            is_boosted: false,
            rugcheck_score: None,
            security_flags: Vec::new(),
        };

        let base_opportunity = BaseTokenOpportunity {
            mint: mint_str.to_string(),
            symbol,
            name,
            source: ScreenerSource::DexScreener,
        };

        base_opportunity.to_opportunity(metrics, verification).await.map(Some)
    }
}

#[async_trait::async_trait]
impl TokenSource for DexScreenerSource {
    fn name(&self) -> &str {
        "DexScreener"
    }

    async fn initialize(&mut self) -> BotResult<()> {
        // Test API connection
        self.health_check().await?;
        log::info!("✅ DexScreener source initialized");
        Ok(())
    }

    async fn get_new_tokens(&self) -> BotResult<Vec<TokenOpportunity>> {
        let trending_data = self.get_trending_tokens().await?;
        let mut opportunities = Vec::new();

        for token_data in trending_data {
            match self.parse_dexscreener_token(&token_data).await {
                Ok(Some(opportunity)) => opportunities.push(opportunity),
                Ok(None) => {
                    continue;
                }
                Err(e) => {
                    log::warn!("Failed to parse DexScreener token: {}", e);
                    continue;
                }
            }
        }

        Ok(opportunities)
    }

    async fn get_token_info(&self, mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        let url = format!("{}/dex/tokens/{}", self.base_url, mint);

        let response = self.client
            .get(&url)
            .header("User-Agent", "ScreenerBot/1.0")
            .send().await
            .map_err(|e| BotError::Network(format!("DexScreener request failed: {}", e)))?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(BotError::Network(format!("DexScreener API error: {}", response.status())));
        }

        let data: Value = response
            .json().await
            .map_err(|e| BotError::Parse(format!("Failed to parse DexScreener response: {}", e)))?;

        if let Some(token_data) = data["pairs"].as_array().and_then(|pairs| pairs.first()) {
            self.parse_dexscreener_token(&token_data["baseToken"]).await
        } else {
            Ok(None)
        }
    }

    async fn health_check(&self) -> BotResult<bool> {
        let url = format!("{}/dex/tokens/trending", self.base_url);

        match
            self.client
                .get(&url)
                .header("User-Agent", "ScreenerBot/1.0")
                .timeout(Duration::from_secs(10))
                .send().await
        {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

/// GeckoTerminal API source
pub struct GeckoTerminalSource {
    client: Client,
    base_url: String,
}

impl GeckoTerminalSource {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl TokenSource for GeckoTerminalSource {
    fn name(&self) -> &str {
        "GeckoTerminal"
    }

    async fn initialize(&mut self) -> BotResult<()> {
        log::info!("✅ GeckoTerminal source initialized");
        Ok(())
    }

    async fn get_new_tokens(&self) -> BotResult<Vec<TokenOpportunity>> {
        // Implementation for GeckoTerminal new tokens
        // For now, return empty - can be implemented based on their API
        Ok(Vec::new())
    }

    async fn get_token_info(&self, _mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        // Implementation for specific token info from GeckoTerminal
        Ok(None)
    }

    async fn health_check(&self) -> BotResult<bool> {
        Ok(true) // Simplified for now
    }
}

/// Raydium API source
pub struct RaydiumSource {
    client: Client,
    base_url: String,
}

impl RaydiumSource {
    pub fn new() -> Self {
        Self {
            client: Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
            base_url: crate::core::RAYDIUM_API_BASE.to_string(),
        }
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
        // Implementation for Raydium new pools/tokens
        Ok(Vec::new())
    }

    async fn get_token_info(&self, _mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        Ok(None)
    }

    async fn health_check(&self) -> BotResult<bool> {
        Ok(true)
    }
}

/// RugCheck API source for security verification
pub struct RugCheckSource {
    client: Client,
    base_url: String,
}

impl RugCheckSource {
    pub fn new() -> Self {
        Self {
            client: Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
            base_url: crate::core::RUGCHECK_API_BASE.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl TokenSource for RugCheckSource {
    fn name(&self) -> &str {
        "RugCheck"
    }

    async fn initialize(&mut self) -> BotResult<()> {
        log::info!("✅ RugCheck source initialized");
        Ok(())
    }

    async fn get_new_tokens(&self) -> BotResult<Vec<TokenOpportunity>> {
        // RugCheck is primarily for verification, not discovery
        Ok(Vec::new())
    }

    async fn get_token_info(&self, mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        // Get rugcheck data for a specific token
        let url = format!("{}/tokens/{}/report", self.base_url, mint);

        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                // Parse rugcheck response and create verification status
                // This would need to be implemented based on actual RugCheck API
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    async fn health_check(&self) -> BotResult<bool> {
        Ok(true)
    }
}
