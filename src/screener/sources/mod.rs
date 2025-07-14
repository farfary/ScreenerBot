//! Token discovery sources module
//!
//! This module contains different sources for discovering new tokens
//! Each source implements the TokenSource trait to provide a standardized interface

use crate::core::types::{ TokenOpportunity, TokenInfo };
use crate::core::BotResult;
use async_trait::async_trait;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::str::FromStr;
use solana_sdk::pubkey::Pubkey;

pub mod dexscreener;
pub mod geckoterminal;
pub mod raydium;
pub mod rugcheck;

// Re-export all sources
pub use dexscreener::DexScreenerSource;
pub use geckoterminal::GeckoTerminalSource;
pub use raydium::RaydiumSource;
pub use rugcheck::RugCheckSource;

/// Standard interface for token discovery sources
#[async_trait::async_trait]
pub trait TokenSource: Send + Sync {
    /// Get the name of this source
    fn name(&self) -> &str;

    /// Initialize the source (setup connections, validate API keys, etc.)
    async fn initialize(&mut self) -> BotResult<()>;

    /// Get new tokens from this source
    async fn get_new_tokens(&self) -> BotResult<Vec<TokenOpportunity>>;

    /// Get specific token information (optional)
    async fn get_token_info(&self, mint: &Pubkey) -> BotResult<Option<TokenOpportunity>>;

    /// Health check for the source
    async fn health_check(&self) -> BotResult<bool>;
}

/// Base token discovery data that can be converted to TokenOpportunity
#[derive(Debug, Clone)]
pub struct BaseTokenDiscovery {
    pub token_address: String,
    pub chain_id: String,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub header: Option<String>,
    pub description: Option<String>,
    pub links: Vec<SocialLink>,
    pub boost_amount: Option<u64>, // For boosted tokens
    pub discovery_source: String,
}

#[derive(Debug, Clone)]
pub struct SocialLink {
    pub link_type: Option<String>,
    pub label: Option<String>,
    pub url: String,
}

impl BaseTokenDiscovery {
    /// Convert to TokenOpportunity with confidence scoring
    pub fn to_opportunity(&self) -> BotResult<TokenOpportunity> {
        // Parse token address
        let token = Pubkey::from_str(&self.token_address).map_err(|e|
            crate::core::BotError::Parsing(format!("Invalid token address: {}", e))
        )?;

        // Extract token name and symbol from URL or address
        let (symbol, name) = self.extract_token_info();

        // Calculate confidence based on available metadata
        let confidence = self.calculate_confidence();

        // Determine if has social links
        let has_socials = self.links.iter().any(|link| {
            if let Some(link_type) = &link.link_type {
                matches!(link_type.as_str(), "twitter" | "telegram" | "discord" | "website")
            } else {
                false
            }
        });

        // Create verification status
        let verification_status = crate::core::VerificationStatus {
            is_verified: self.description.is_some() &&
            !self.description.as_ref().unwrap().is_empty(),
            has_profile: true,
            is_boosted: self.boost_amount.is_some(),
            rugcheck_score: None,
            security_flags: Vec::new(),
            has_socials,
            contract_verified: false,
        };

        // Create basic token metrics
        let metrics = crate::core::TokenMetrics {
            price_usd: 0.0,
            volume_24h: 0.0,
            liquidity_usd: 0.0,
            market_cap: None,
            price_change_24h: None,
            age_hours: 0.0, // New discovery
            holder_count: None,
            top_10_holder_percentage: None,
        };

        Ok(TokenOpportunity {
            mint: token,
            token: crate::core::TokenInfo {
                mint: token,
                symbol: symbol.clone(),
                name: name.clone(),
            },
            symbol,
            name,
            metrics,
            source: match self.discovery_source.as_str() {
                "dexscreener_profiles" => crate::core::ScreenerSource::DexScreener,
                "dexscreener_boosts" => crate::core::ScreenerSource::DexScreener,
                _ => crate::core::ScreenerSource::DexScreener,
            },
            confidence_score: confidence,
            discovery_time: chrono::Utc::now(),
            liquidity_provider: crate::core::LiquidityProvider::Other("Unknown".to_string()),
            verification_status,
            risk_score: if self.boost_amount.is_some() {
                0.2
            } else {
                0.3
            }, // Boosted tokens might be lower risk
            social_metrics: None,
            risk_factors: Vec::new(),
        })
    }

    fn extract_token_info(&self) -> (String, String) {
        // Try to extract from URL first
        if let Some(url) = &self.url {
            if let Some(address_part) = url.split('/').last() {
                let symbol = address_part.chars().take(8).collect::<String>().to_uppercase();
                let name = format!("Token_{}", symbol);
                return (symbol, name);
            }
        }

        // Fallback to address
        let symbol = self.token_address.chars().take(8).collect::<String>().to_uppercase();
        let name = format!("Token_{}", symbol);
        (symbol, name)
    }

    fn calculate_confidence(&self) -> f64 {
        let mut confidence: f64 = 0.4; // Base confidence

        if self.description.is_some() {
            confidence += 0.1;
        }
        if self.icon.is_some() {
            confidence += 0.1;
        }
        if !self.links.is_empty() {
            confidence += 0.2;
        }
        if self.header.is_some() {
            confidence += 0.1;
        }
        if self.boost_amount.is_some() {
            confidence += 0.1; // Boosted tokens get extra confidence
        }

        confidence.min(1.0)
    }
}
