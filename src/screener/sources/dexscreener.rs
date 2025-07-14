//! DexScreener API integration for token discovery
//! 
//! This module provides access to multiple DexScreener endpoints:
//! - Token Profiles: Latest tokens with profile data
//! - Token Boosts: Promoted/boosted tokens with higher visibility

use super::{ TokenSource, BaseTokenDiscovery, SocialLink };
use crate::core::{ BotResult, BotError, TokenOpportunity };
use reqwest::Client;
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;
use std::collections::HashSet;

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

// DexScreener Token Boosts API Response Types
#[derive(Debug, Deserialize)]
struct DexScreenerTokenBoost {
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
    #[serde(rename = "totalAmount")]
    total_amount: u64,
    amount: u64,
}

#[derive(Debug, Deserialize)]
struct DexScreenerLink {
    #[serde(rename = "type")]
    link_type: Option<String>,
    label: Option<String>,
    url: String,
}

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
    async fn fetch_token_profiles(&self) -> BotResult<Vec<DexScreenerTokenProfile>> {
        let url = format!("{}/token-profiles/latest/v1", self.base_url);

        log::info!("Fetching token profiles from DexScreener: {}", url);

        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
            .send().await
            .map_err(|e| BotError::Network(format!("DexScreener profiles API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(
                BotError::Api(format!("DexScreener profiles API returned status: {}", response.status()))
            );
        }

        let response_text = response
            .text().await
            .map_err(|e| BotError::Network(format!("Failed to read profiles response: {}", e)))?;

        log::debug!("DexScreener profiles response length: {} chars", response_text.len());

        let token_profiles: Vec<DexScreenerTokenProfile> = serde_json
            ::from_str(&response_text)
            .map_err(|e|
                BotError::Parsing(format!("Failed to parse DexScreener profiles response: {}", e))
            )?;

        Ok(token_profiles)
    }

    /// Get latest token boosts from DexScreener
    async fn fetch_token_boosts(&self) -> BotResult<Vec<DexScreenerTokenBoost>> {
        let url = format!("{}/token-boosts/latest/v1", self.base_url);

        log::info!("Fetching token boosts from DexScreener: {}", url);

        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
            .send().await
            .map_err(|e| BotError::Network(format!("DexScreener boosts API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(
                BotError::Api(format!("DexScreener boosts API returned status: {}", response.status()))
            );
        }

        let response_text = response
            .text().await
            .map_err(|e| BotError::Network(format!("Failed to read boosts response: {}", e)))?;

        log::debug!("DexScreener boosts response length: {} chars", response_text.len());

        let token_boosts: Vec<DexScreenerTokenBoost> = serde_json
            ::from_str(&response_text)
            .map_err(|e|
                BotError::Parsing(format!("Failed to parse DexScreener boosts response: {}", e))
            )?;

        Ok(token_boosts)
    }

    /// Convert DexScreener links to SocialLink
    fn convert_links(&self, links: &Option<Vec<DexScreenerLink>>) -> Vec<SocialLink> {
        links.as_ref().map(|links| {
            links.iter().map(|link| SocialLink {
                link_type: link.link_type.clone(),
                label: link.label.clone(),
                url: link.url.clone(),
            }).collect()
        }).unwrap_or_default()
    }

    /// Convert token profile to BaseTokenDiscovery
    fn profile_to_discovery(&self, profile: &DexScreenerTokenProfile) -> BaseTokenDiscovery {
        BaseTokenDiscovery {
            token_address: profile.token_address.clone(),
            chain_id: profile.chain_id.clone(),
            url: Some(profile.url.clone()),
            icon: profile.icon.clone(),
            header: profile.header.clone(),
            description: profile.description.clone(),
            links: self.convert_links(&profile.links),
            boost_amount: None,
            discovery_source: "dexscreener_profiles".to_string(),
        }
    }

    /// Convert token boost to BaseTokenDiscovery
    fn boost_to_discovery(&self, boost: &DexScreenerTokenBoost) -> BaseTokenDiscovery {
        BaseTokenDiscovery {
            token_address: boost.token_address.clone(),
            chain_id: boost.chain_id.clone(),
            url: Some(boost.url.clone()),
            icon: boost.icon.clone(),
            header: boost.header.clone(),
            description: boost.description.clone(),
            links: self.convert_links(&boost.links),
            boost_amount: Some(boost.amount),
            discovery_source: "dexscreener_boosts".to_string(),
        }
    }

    /// Filter to only include Solana tokens and remove duplicates
    fn filter_and_dedupe_discoveries(&self, discoveries: Vec<BaseTokenDiscovery>) -> Vec<BaseTokenDiscovery> {
        let mut seen_addresses = HashSet::new();
        let mut filtered = Vec::new();

        for discovery in discoveries {
            // Only include Solana tokens
            if discovery.chain_id != "solana" {
                continue;
            }

            // Skip if we've already seen this token address
            if seen_addresses.contains(&discovery.token_address) {
                continue;
            }

            // Must have a valid token address
            if discovery.token_address.is_empty() {
                continue;
            }

            // Must have some metadata
            if discovery.description.is_none() && discovery.links.is_empty() {
                continue;
            }

            seen_addresses.insert(discovery.token_address.clone());
            filtered.push(discovery);
        }

        filtered
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

        // Fetch both profiles and boosts concurrently
        let (profiles_result, boosts_result) = tokio::join!(
            self.fetch_token_profiles(),
            self.fetch_token_boosts()
        );

        let mut discoveries = Vec::new();

        // Process profiles
        match profiles_result {
            Ok(profiles) => {
                log::info!("Fetched {} token profiles from DexScreener", profiles.len());
                for profile in profiles {
                    discoveries.push(self.profile_to_discovery(&profile));
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch DexScreener profiles: {}", e);
            }
        }

        // Process boosts
        match boosts_result {
            Ok(boosts) => {
                log::info!("Fetched {} token boosts from DexScreener", boosts.len());
                for boost in boosts {
                    discoveries.push(self.boost_to_discovery(&boost));
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch DexScreener boosts: {}", e);
            }
        }

        // Filter and deduplicate
        let filtered_discoveries = self.filter_and_dedupe_discoveries(discoveries);
        log::info!("Found {} unique Solana token discoveries", filtered_discoveries.len());

        // Convert to opportunities
        let mut opportunities = Vec::new();
        for discovery in filtered_discoveries {
            match discovery.to_opportunity() {
                Ok(opportunity) => {
                    log::debug!(
                        "Created opportunity for token: {} ({})",
                        opportunity.symbol,
                        if discovery.boost_amount.is_some() { "boosted" } else { "profile" }
                    );
                    opportunities.push(opportunity);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to create opportunity for token {}: {}",
                        discovery.token_address,
                        e
                    );
                }
            }
        }

        log::info!("Created {} opportunities from DexScreener", opportunities.len());
        Ok(opportunities)
    }

    async fn get_token_info(&self, _mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        // Could implement specific token lookup here
        Ok(None)
    }

    async fn health_check(&self) -> BotResult<bool> {
        // Simple health check by testing one endpoint
        match self.fetch_token_profiles().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
