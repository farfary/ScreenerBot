use crate::core::{
    BotResult,
    BotError,
    TokenOpportunity,
    ScreenerSource,
    TokenMetrics,
    VerificationStatus,
    LiquidityProvider,
    TokenInfo,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use chrono::{ Utc, DateTime };
use std::time::Duration;

// DexScreener API Response Types
#[derive(Debug, Deserialize)]
struct DexScreenerResponse {
    pairs: Vec<DexScreenerPair>,
}

#[derive(Debug, Deserialize)]
struct DexScreenerPair {
    #[serde(rename = "chainId")]
    chain_id: String,
    #[serde(rename = "dexId")]
    dex_id: String,
    url: String,
    #[serde(rename = "pairAddress")]
    pair_address: String,
    #[serde(rename = "baseToken")]
    base_token: DexScreenerToken,
    #[serde(rename = "quoteToken")]
    quote_token: DexScreenerToken,
    #[serde(rename = "priceNative")]
    price_native: String,
    #[serde(rename = "priceUsd")]
    price_usd: Option<String>,
    txns: DexScreenerTxns,
    volume: DexScreenerVolume,
    #[serde(rename = "priceChange")]
    price_change: DexScreenerPriceChange,
    liquidity: Option<DexScreenerLiquidity>,
    #[serde(rename = "pairCreatedAt")]
    pair_created_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct DexScreenerToken {
    address: String,
    name: String,
    symbol: String,
}

#[derive(Debug, Deserialize)]
struct DexScreenerTxns {
    #[serde(rename = "m5")]
    m5: DexScreenerTxnData,
    #[serde(rename = "h1")]
    h1: DexScreenerTxnData,
    #[serde(rename = "h6")]
    h6: DexScreenerTxnData,
    #[serde(rename = "h24")]
    h24: DexScreenerTxnData,
}

#[derive(Debug, Deserialize)]
struct DexScreenerTxnData {
    buys: i32,
    sells: i32,
}

#[derive(Debug, Deserialize)]
struct DexScreenerVolume {
    #[serde(rename = "h24")]
    h24: f64,
    #[serde(rename = "h6")]
    h6: f64,
    #[serde(rename = "h1")]
    h1: f64,
    #[serde(rename = "m5")]
    m5: f64,
}

#[derive(Debug, Deserialize)]
struct DexScreenerPriceChange {
    #[serde(rename = "m5")]
    m5: Option<f64>,
    #[serde(rename = "h1")]
    h1: Option<f64>,
    #[serde(rename = "h6")]
    h6: Option<f64>,
    #[serde(rename = "h24")]
    h24: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct DexScreenerLiquidity {
    usd: Option<f64>,
    base: Option<f64>,
    quote: Option<f64>,
}

use crate::screener::TokenSource;

/// DexScreener API source for token discovery
pub struct DexScreenerSource {
    client: Client,
    base_url: String,
}

impl DexScreenerSource {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ScreenerBot/1.0")
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://api.dexscreener.com".to_string(),
        }
    }

    /// Get latest token profiles from DexScreener
    async fn fetch_latest_tokens(&self) -> BotResult<Vec<DexScreenerPair>> {
        let url = format!("{}/latest/dex/tokens", self.base_url);

        log::info!("Fetching latest tokens from DexScreener: {}", url);

        let response = self.client
            .get(&url)
            .header("Accept", "*/*")
            .send().await
            .map_err(|e| BotError::Network(format!("DexScreener API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(
                BotError::Api(format!("DexScreener API returned status: {}", response.status()))
            );
        }

        let response_text = response
            .text().await
            .map_err(|e| BotError::Network(format!("Failed to read response: {}", e)))?;

        log::debug!("DexScreener response: {}", response_text);

        let dex_response: DexScreenerResponse = serde_json
            ::from_str(&response_text)
            .map_err(|e|
                BotError::Parsing(format!("Failed to parse DexScreener response: {}", e))
            )?;

        Ok(dex_response.pairs)
    }

    /// Convert DexScreener pair to TokenOpportunity
    fn pair_to_opportunity(&self, pair: &DexScreenerPair) -> BotResult<TokenOpportunity> {
        // Parse token address
        let token = Pubkey::from_str(&pair.base_token.address).map_err(|e|
            BotError::Parsing(format!("Invalid token address: {}", e))
        )?;

        // Parse price
        let price_usd = pair.price_usd
            .as_ref()
            .and_then(|p| p.parse::<f64>().ok())
            .unwrap_or(0.0);

        // Calculate age in hours
        let age_hours = if let Some(created_at) = pair.pair_created_at {
            let created_time = DateTime::from_timestamp(created_at / 1000, 0).unwrap_or_else(||
                Utc::now()
            );
            (Utc::now() - created_time).num_hours() as f64
        } else {
            24.0 // Default to 24 hours if no creation time
        };

        // Create token metrics
        let metrics = TokenMetrics {
            price_usd,
            volume_24h: pair.volume.h24,
            liquidity_usd: pair.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0),
            market_cap: None, // DexScreener doesn't provide market cap directly
            price_change_24h: pair.price_change.h24,
            age_hours,
            holder_count: None,
            top_10_holder_percentage: None,
        };

        // Determine liquidity provider
        let liquidity_provider = match pair.dex_id.as_str() {
            "raydium" => LiquidityProvider::Raydium,
            "orca" => LiquidityProvider::Orca,
            "jupiter" => LiquidityProvider::Jupiter,
            _ => LiquidityProvider::Other(pair.dex_id.clone()),
        };

        // Create verification status (basic checks)
        let verification_status = VerificationStatus {
            is_verified: !pair.base_token.name.is_empty() && !pair.base_token.symbol.is_empty(),
            has_profile: true, // Coming from DexScreener means it has some profile
            is_boosted: false, // No boost information available
            rugcheck_score: None, // No rugcheck data available
            security_flags: Vec::new(), // No security flags from this endpoint
            has_socials: false, // We don't have social data from this endpoint
            contract_verified: false, // Would need additional verification
        };

        // Calculate basic confidence score
        let mut confidence: f64 = 0.3; // Base confidence

        // Increase confidence based on volume
        if pair.volume.h24 > 1000.0 {
            confidence += 0.2;
        }
        if pair.volume.h24 > 10000.0 {
            confidence += 0.2;
        }

        // Increase confidence based on liquidity
        if let Some(liquidity) = &pair.liquidity {
            if let Some(usd_liquidity) = liquidity.usd {
                if usd_liquidity > 10000.0 {
                    confidence += 0.1;
                }
                if usd_liquidity > 50000.0 {
                    confidence += 0.1;
                }
            }
        }

        // Increase confidence based on transaction activity
        if pair.txns.h24.buys + pair.txns.h24.sells > 50 {
            confidence += 0.1;
        }

        confidence = confidence.min(1.0);

        Ok(TokenOpportunity {
            mint: token,
            token: TokenInfo {
                mint: token,
                symbol: pair.base_token.symbol.clone(),
                name: if pair.base_token.name.is_empty() {
                    "Unknown".to_string()
                } else {
                    pair.base_token.name.clone()
                },
            },
            symbol: pair.base_token.symbol.clone(),
            name: if pair.base_token.name.is_empty() {
                "Unknown".to_string()
            } else {
                pair.base_token.name.clone()
            },
            metrics,
            source: ScreenerSource::DexScreener,
            confidence_score: confidence,
            discovery_time: Utc::now(),
            liquidity_provider,
            verification_status,
            risk_score: 0.5, // Default risk score
            social_metrics: None, // Not available from this endpoint
            risk_factors: Vec::new(), // Will be calculated by analyzer
        })
    }

    /// Filter pairs to only include Solana tokens
    fn filter_solana_pairs(&self, pairs: Vec<DexScreenerPair>) -> Vec<DexScreenerPair> {
        pairs
            .into_iter()
            .filter(|pair| {
                pair.chain_id == "solana" &&
                    // Filter out obvious scams or low-quality tokens
                    pair.volume.h24 > 100.0 && // Minimum 24h volume
                    pair.base_token.symbol.len() <= 10 && // Reasonable symbol length
                    !pair.base_token.symbol.chars().any(|c| !c.is_ascii()) // ASCII only
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl TokenSource for DexScreenerSource {
    fn name(&self) -> &str {
        "DexScreener"
    }

    async fn initialize(&mut self) -> BotResult<()> {
        log::info!("✅ DexScreener source initialized");
        Ok(())
    }

    async fn get_new_tokens(&self) -> BotResult<Vec<TokenOpportunity>> {
        log::info!("Fetching opportunities from DexScreener...");

        let pairs = self.fetch_latest_tokens().await?;
        log::info!("Fetched {} pairs from DexScreener", pairs.len());

        // Filter to Solana pairs only
        let solana_pairs = self.filter_solana_pairs(pairs);
        log::info!("Found {} Solana pairs", solana_pairs.len());

        let mut opportunities = Vec::new();

        for pair in solana_pairs {
            match self.pair_to_opportunity(&pair) {
                Ok(opportunity) => {
                    log::debug!(
                        "Created opportunity for token: {} ({:?})",
                        opportunity.symbol,
                        opportunity.token
                    );
                    opportunities.push(opportunity);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to create opportunity for pair {}: {}",
                        pair.base_token.symbol,
                        e
                    );
                }
            }
        }

        log::info!("Created {} opportunities from DexScreener", opportunities.len());
        Ok(opportunities)
    }

    async fn get_token_info(&self, _mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        // For specific token info, we could query DexScreener's token endpoint
        Ok(None)
    }

    async fn health_check(&self) -> BotResult<bool> {
        Ok(true)
    }
}
/// GeckoTerminal API source
pub struct GeckoTerminalSource {
    client: Client,
    base_url: String,
}

impl GeckoTerminalSource {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ScreenerBot/1.0")
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://api.geckoterminal.com".to_string(),
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
        // Placeholder implementation - would call GeckoTerminal API
        log::info!("GeckoTerminal source not yet implemented");
        Ok(Vec::new())
    }

    async fn get_token_info(&self, _mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        Ok(None)
    }

    async fn health_check(&self) -> BotResult<bool> {
        Ok(true)
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
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ScreenerBot/1.0")
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://api.raydium.io".to_string(),
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
        // Placeholder implementation - would call Raydium API
        log::info!("Raydium source not yet implemented");
        Ok(Vec::new())
    }

    async fn get_token_info(&self, _mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        Ok(None)
    }

    async fn health_check(&self) -> BotResult<bool> {
        Ok(true)
    }
}

/// RugCheck API source for token verification
pub struct RugCheckSource {
    client: Client,
    base_url: String,
}

impl RugCheckSource {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ScreenerBot/1.0")
                .build()
                .expect("Failed to create HTTP client"),
            base_url: "https://api.rugcheck.xyz".to_string(),
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
        // Placeholder implementation - would call RugCheck API
        log::info!("RugCheck source not yet implemented");
        Ok(Vec::new())
    }

    async fn get_token_info(&self, _mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        Ok(None)
    }

    async fn health_check(&self) -> BotResult<bool> {
        Ok(true)
    }
}
