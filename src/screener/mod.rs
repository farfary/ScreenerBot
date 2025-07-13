use crate::core::{
    BotResult,
    BotError,
    ScreenerConfig,
    TokenOpportunity,
    ScreenerSource,
    TokenMetrics,
    VerificationStatus,
};
use serde_json::Value;
use std::collections::HashMap;
use chrono::Utc;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub mod sources;
pub mod filters;
pub mod analysis;

pub use sources::*;
pub use filters::*;
pub use analysis::*;

/// Main screener manager for discovering new tokens
#[derive(Debug)]
pub struct ScreenerManager {
    config: ScreenerConfig,
    sources: Vec<Box<dyn TokenSource + Send + Sync>>,
    filters: OpportunityFilter,
    analyzer: OpportunityAnalyzer,
}

impl ScreenerManager {
    /// Create a new screener manager
    pub fn new(config: &ScreenerConfig) -> BotResult<Self> {
        let mut sources: Vec<Box<dyn TokenSource + Send + Sync>> = Vec::new();

        // Initialize sources based on config
        if config.sources.dexscreener_enabled {
            sources.push(Box::new(DexScreenerSource::new()));
        }

        if config.sources.geckoterminal_enabled {
            sources.push(Box::new(GeckoTerminalSource::new(&config.gecko_api_base)));
        }

        if config.sources.raydium_enabled {
            sources.push(Box::new(RaydiumSource::new()));
        }

        if config.sources.rugcheck_enabled {
            sources.push(Box::new(RugCheckSource::new()));
        }

        let filters = OpportunityFilter::new(&config.filters);
        let analyzer = OpportunityAnalyzer::new();

        Ok(Self {
            config: config.clone(),
            sources,
            filters,
            analyzer,
        })
    }

    /// Initialize the screener
    pub async fn initialize(&mut self) -> BotResult<()> {
        log::info!("🔍 Initializing screener...");
        log::info!("📡 Active sources: {}", self.sources.len());

        // Initialize all sources
        for source in &mut self.sources {
            source.initialize().await?;
        }

        log::info!("✅ Screener initialized successfully");
        Ok(())
    }

    /// Scan for new token opportunities
    pub async fn scan_opportunities(&self) -> BotResult<Vec<TokenOpportunity>> {
        log::info!("🔎 Scanning for new opportunities...");

        let mut all_opportunities = Vec::new();

        // Collect opportunities from all sources
        for source in &self.sources {
            match source.get_new_tokens().await {
                Ok(mut tokens) => {
                    log::debug!("📊 Source {} found {} tokens", source.name(), tokens.len());
                    all_opportunities.append(&mut tokens);
                }
                Err(e) => {
                    log::warn!("⚠️ Source {} failed: {}", source.name(), e);
                    continue;
                }
            }
        }

        log::info!("🎯 Found {} raw opportunities", all_opportunities.len());

        // Remove duplicates based on mint address
        all_opportunities = self.deduplicate_opportunities(all_opportunities);

        // Apply filters
        let filtered_opportunities = self.filters.apply_filters(all_opportunities).await?;
        log::info!("✅ {} opportunities passed filters", filtered_opportunities.len());

        // Analyze remaining opportunities
        let analyzed_opportunities =
            self.analyzer.analyze_opportunities(filtered_opportunities).await?;
        log::info!("📈 Analysis complete: {} opportunities ready", analyzed_opportunities.len());

        Ok(analyzed_opportunities)
    }

    /// Remove duplicate opportunities based on mint address
    fn deduplicate_opportunities(
        &self,
        opportunities: Vec<TokenOpportunity>
    ) -> Vec<TokenOpportunity> {
        let mut seen_mints = HashMap::new();
        let mut deduped = Vec::new();

        for opportunity in opportunities {
            let mint_str = opportunity.mint.to_string();

            if let Some(existing) = seen_mints.get(&mint_str) {
                // Keep the one with higher confidence score
                let existing_idx = *existing;
                if opportunity.confidence_score > deduped[existing_idx].confidence_score {
                    deduped[existing_idx] = opportunity;
                }
            } else {
                seen_mints.insert(mint_str, deduped.len());
                deduped.push(opportunity);
            }
        }

        log::debug!("🔄 Deduplicated {} opportunities", deduped.len());
        deduped
    }

    /// Get detailed information about a specific token
    pub async fn get_token_details(&self, mint: &Pubkey) -> BotResult<Option<TokenOpportunity>> {
        for source in &self.sources {
            if let Ok(Some(token)) = source.get_token_info(mint).await {
                return Ok(Some(token));
            }
        }
        Ok(None)
    }

    /// Update screener configuration
    pub fn update_config(&mut self, config: ScreenerConfig) {
        self.config = config;
        self.filters = OpportunityFilter::new(&self.config.filters);
    }
}

/// Trait for token data sources
#[async_trait::async_trait]
pub trait TokenSource {
    /// Get the name of this source
    fn name(&self) -> &str;

    /// Initialize the source
    async fn initialize(&mut self) -> BotResult<()>;

    /// Get new tokens from this source
    async fn get_new_tokens(&self) -> BotResult<Vec<TokenOpportunity>>;

    /// Get detailed info about a specific token
    async fn get_token_info(&self, mint: &Pubkey) -> BotResult<Option<TokenOpportunity>>;

    /// Check if the source is healthy/available
    async fn health_check(&self) -> BotResult<bool>;
}

/// Base token opportunity that can be extended by sources
pub struct BaseTokenOpportunity {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub source: ScreenerSource,
}

impl BaseTokenOpportunity {
    /// Convert to full TokenOpportunity with metrics and verification
    pub async fn to_opportunity(
        self,
        metrics: TokenMetrics,
        verification: VerificationStatus
    ) -> BotResult<TokenOpportunity> {
        let mint = Pubkey::from_str(&self.mint).map_err(|e|
            BotError::Parse(format!("Invalid mint address: {}", e))
        )?;

        // Calculate basic scores
        let risk_score = self.calculate_risk_score(&metrics, &verification);
        let confidence_score = self.calculate_confidence_score(&metrics, &verification);

        Ok(TokenOpportunity {
            mint,
            symbol: self.symbol,
            name: self.name,
            source: self.source,
            discovery_time: Utc::now(),
            metrics,
            verification_status: verification,
            risk_score,
            confidence_score,
        })
    }

    /// Calculate risk score based on metrics and verification
    fn calculate_risk_score(
        &self,
        metrics: &TokenMetrics,
        verification: &VerificationStatus
    ) -> f64 {
        let mut risk = 0.5; // Base risk

        // Age factor (newer = higher risk)
        if metrics.age_hours < 1.0 {
            risk += 0.3;
        } else if metrics.age_hours < 24.0 {
            risk += 0.2;
        }

        // Liquidity factor
        if metrics.liquidity_usd < 10000.0 {
            risk += 0.2;
        }

        // Verification factors
        if verification.is_verified {
            risk -= 0.2;
        }

        if verification.has_profile {
            risk -= 0.1;
        }

        // Security flags
        risk += (verification.security_flags.len() as f64) * 0.1;

        risk.clamp(0.0, 1.0)
    }

    /// Calculate confidence score
    fn calculate_confidence_score(
        &self,
        metrics: &TokenMetrics,
        verification: &VerificationStatus
    ) -> f64 {
        let mut confidence = 0.3; // Base confidence

        // Volume factor
        if metrics.volume_24h > 100000.0 {
            confidence += 0.3;
        } else if metrics.volume_24h > 10000.0 {
            confidence += 0.2;
        } else if metrics.volume_24h > 1000.0 {
            confidence += 0.1;
        }

        // Liquidity factor
        if metrics.liquidity_usd > 50000.0 {
            confidence += 0.2;
        } else if metrics.liquidity_usd > 10000.0 {
            confidence += 0.1;
        }

        // Verification bonus
        if verification.is_verified {
            confidence += 0.2;
        }

        if verification.has_profile {
            confidence += 0.1;
        }

        confidence.clamp(0.0, 1.0)
    }
}
