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
use reqwest::Client;
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use chrono::{ Utc, DateTime };
use std::time::Duration;

// DexScreener Token Profiles API Response Types
#[derive(Debug, Deserialize)]
struct DexScreenerTokenProfile {
    url: String,
    #[serde(rename = "chainId")]
    chain_id: String,
    #[serde(rename = "tokenAddress")]
    token_address: String,
    icon: Option<String>,
    header: Option<String>,
    #[serde(rename = "openGraph")]
    open_graph: Option<String>,
    description: Option<String>,
    links: Option<Vec<DexScreenerLink>>,
}

#[derive(Debug, Deserialize)]
struct DexScreenerLink {
    #[serde(rename = "type")]
    link_type: Option<String>,
    label: Option<String>,
    url: String,
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
    async fn fetch_latest_tokens(&self) -> BotResult<Vec<DexScreenerTokenProfile>> {
        let url = format!("{}/token-profiles/latest/v1", self.base_url);

        log::info!("Fetching latest tokens from DexScreener: {}", url);

        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
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

        let token_profiles: Vec<DexScreenerTokenProfile> = serde_json
            ::from_str(&response_text)
            .map_err(|e|
                BotError::Parsing(format!("Failed to parse DexScreener response: {}", e))
            )?;

        Ok(token_profiles)
    }

    /// Convert DexScreener token profile to TokenOpportunity
    fn profile_to_opportunity(
        &self,
        profile: &DexScreenerTokenProfile
    ) -> BotResult<TokenOpportunity> {
        // Parse token address
        let token = Pubkey::from_str(&profile.token_address).map_err(|e|
            BotError::Parsing(format!("Invalid token address: {}", e))
        )?;

        // Extract token name and symbol from URL
        let (symbol, name) = self.extract_token_info_from_url(&profile.url);

        // Create basic token metrics (limited data available from token profiles)
        let metrics = TokenMetrics {
            price_usd: 0.0, // Not available in token profiles
            volume_24h: 0.0, // Not available in token profiles
            liquidity_usd: 0.0, // Not available in token profiles
            market_cap: None,
            price_change_24h: None,
            age_hours: 0.0, // New token, recent profile
            holder_count: None,
            top_10_holder_percentage: None,
        };

        // Determine if has social links
        let has_socials = profile.links
            .as_ref()
            .map(|links| {
                links.iter().any(|link| {
                    if let Some(link_type) = &link.link_type {
                        matches!(link_type.as_str(), "twitter" | "telegram" | "discord" | "website")
                    } else {
                        false
                    }
                })
            })
            .unwrap_or(false);

        // Create verification status
        let verification_status = VerificationStatus {
            is_verified: profile.description.is_some() &&
            !profile.description.as_ref().unwrap().is_empty(),
            has_profile: true, // Coming from DexScreener token profiles
            is_boosted: false,
            rugcheck_score: None,
            security_flags: Vec::new(),
            has_socials,
            contract_verified: false,
        };

        // Calculate confidence based on available metadata
        let mut confidence: f64 = 0.4; // Base confidence for having a profile

        if profile.description.is_some() {
            confidence += 0.1;
        }
        if profile.icon.is_some() {
            confidence += 0.1;
        }
        if has_socials {
            confidence += 0.2;
        }
        if profile.header.is_some() {
            confidence += 0.1;
        }

        confidence = confidence.min(1.0);

        Ok(TokenOpportunity {
            mint: token,
            token: TokenInfo {
                mint: token,
                symbol: symbol.clone(),
                name: name.clone(),
            },
            symbol,
            name,
            metrics,
            source: ScreenerSource::DexScreener,
            confidence_score: confidence,
            discovery_time: Utc::now(),
            liquidity_provider: LiquidityProvider::Other("Unknown".to_string()), // Not available in profiles
            verification_status,
            risk_score: 0.3, // Lower risk for tokens with profiles
            social_metrics: None,
            risk_factors: Vec::new(),
        })
    }

    /// Extract token info from DexScreener URL
    fn extract_token_info_from_url(&self, url: &str) -> (String, String) {
        // Extract from URL like: https://dexscreener.com/solana/2n6kbsxo4tbpe9kwndsu4su6agwjhpxv2tuzacmjbonk
        if let Some(address_part) = url.split('/').last() {
            // Use first 8 characters as symbol and full address as name for now
            let symbol = address_part.chars().take(8).collect::<String>().to_uppercase();
            let name = format!("Token_{}", symbol);
            (symbol, name)
        } else {
            ("UNKNOWN".to_string(), "Unknown Token".to_string())
        }
    }

    /// Filter profiles to only include Solana tokens
    fn filter_solana_profiles(
        &self,
        profiles: Vec<DexScreenerTokenProfile>
    ) -> Vec<DexScreenerTokenProfile> {
        profiles
            .into_iter()
            .filter(|profile| {
                profile.chain_id == "solana" &&
                    // Must have a token address
                    !profile.token_address.is_empty() &&
                    // Must have some description or links
                    (profile.description.is_some() || profile.links.is_some())
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

        let profiles = self.fetch_latest_tokens().await?;
        log::info!("Fetched {} token profiles from DexScreener", profiles.len());

        // Filter to Solana profiles only
        let solana_profiles = self.filter_solana_profiles(profiles);
        log::info!("Found {} Solana token profiles", solana_profiles.len());

        let mut opportunities = Vec::new();

        for profile in solana_profiles {
            match self.profile_to_opportunity(&profile) {
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
                        "Failed to create opportunity for profile {}: {}",
                        profile.token_address,
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
