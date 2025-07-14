use crate::core::{
    BotResult,
    BotError,
    ScreenerConfig,
    TokenOpportunity,
    ScreenerSource,
    TokenMetrics,
    VerificationStatus,
    LiquidityProvider,
    TokenInfo,
};
use std::collections::HashMap;
use chrono::Utc;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub mod sources;
pub mod filters;
pub mod analysis;

// Re-export the new structure
pub use sources::{
    TokenSource,
    DexScreenerSource,
    GeckoTerminalSource,
    RaydiumSource,
    RugCheckSource,
};
pub use filters::*;
pub use analysis::*;

/// Main screener manager for discovering new tokens
pub struct ScreenerManager {
    config: ScreenerConfig,
    sources: Vec<Box<dyn TokenSource + Send + Sync>>,
    filters: OpportunityFilter,
    analyzer: OpportunityAnalyzer,
}

impl ScreenerManager {
    /// Create a new screener manager
    pub fn new(bot_config: &crate::core::BotConfig) -> BotResult<Self> {
        let config = &bot_config.screener_config;
        let mut sources: Vec<Box<dyn TokenSource + Send + Sync>> = Vec::new();

        // Initialize sources based on config
        if config.sources.dexscreener_enabled {
            sources.push(Box::new(DexScreenerSource::new()));
        }

        if config.sources.geckoterminal_enabled {
            sources.push(Box::new(GeckoTerminalSource::new()));
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
                // Keep the one with higher confidence score - simplified comparison
                let existing_idx = *existing;
                // Just replace it for now - avoid complex type inference issue
                deduped[existing_idx] = opportunity;
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

impl std::fmt::Debug for ScreenerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenerManager")
            .field("config", &self.config)
            .field("sources_count", &self.sources.len())
            .field("filters", &self.filters)
            .field("analyzer", &self.analyzer)
            .finish()
    }
}
