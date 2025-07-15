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
    cache: crate::cache::CacheManager,
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
        let cache = crate::cache::CacheManager::new(bot_config)?;

        Ok(Self {
            config: config.clone(),
            sources,
            filters,
            analyzer,
            cache,
        })
    }

    /// Initialize the screener
    pub async fn initialize(&mut self) -> BotResult<()> {
        log::info!("🔍 Initializing screener...");
        log::info!("📡 Active sources: {}", self.sources.len());

        // Initialize cache system
        self.cache.initialize().await?;

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
        let mut new_token_count = 0;

        // Collect opportunities from all sources
        for source in &self.sources {
            match source.get_new_tokens().await {
                Ok(tokens) => {
                    log::debug!("📊 Source {} found {} tokens", source.name(), tokens.len());
                    
                    // Process each token for caching and new discovery logging
                    for token in tokens {
                        // Cache the token and check if it's new
                        match self.cache.cache_token_opportunity(&token).await {
                            Ok(is_new) => {
                                if is_new {
                                    new_token_count += 1;
                                    log::info!(
                                        "🚀 NEW TOKEN DISCOVERED! {} ({}) from {} - Price: ${:.6} | Liquidity: ${:.2} | Volume: ${:.2}",
                                        token.symbol,
                                        token.name,
                                        format!("{:?}", token.source),
                                        token.metrics.price_usd,
                                        token.metrics.liquidity_usd,
                                        token.metrics.volume_24h
                                    );
                                    
                                    // Additional details for new high-value tokens
                                    if token.metrics.liquidity_usd > 100000.0 {
                                        log::info!(
                                            "💎 HIGH-VALUE NEW TOKEN: {} | Mint: {} | Risk Score: {:.2} | Confidence: {:.2}",
                                            token.symbol,
                                            token.mint,
                                            token.risk_score,
                                            token.confidence_score
                                        );
                                    }
                                } else {
                                    log::debug!("🔄 Updated existing token: {} from {}", token.symbol, format!("{:?}", token.source));
                                }
                                all_opportunities.push(token);
                            }
                            Err(e) => {
                                log::warn!("⚠️ Failed to cache token {}: {}", token.symbol, e);
                                all_opportunities.push(token); // Still include in opportunities
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("⚠️ Source {} failed: {}", source.name(), e);
                    continue;
                }
            }
        }

        log::info!("🎯 Found {} total opportunities ({} new tokens discovered)", all_opportunities.len(), new_token_count);

        // Log discovery statistics
        if new_token_count > 0 {
            match self.cache.get_discovery_stats().await {
                Ok(stats) => {
                    log::info!("📈 Discovery stats by source:");
                    for (source, count) in stats {
                        log::info!("  📊 {}: {} tokens", source, count);
                    }
                }
                Err(e) => log::warn!("Failed to get discovery stats: {}", e)
            }
        }

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

    /// Get recently discovered tokens from cache
    pub async fn get_recent_discoveries(&self, limit: Option<usize>) -> BotResult<Vec<(String, String, String)>> {
        self.cache.get_discovered_tokens(None, limit).await
    }

    /// Get discovery statistics by source
    pub async fn get_discovery_statistics(&self) -> BotResult<Vec<(String, usize)>> {
        self.cache.get_discovery_stats().await
    }

    /// Check if a token is already known
    pub async fn is_token_known(&self, mint: &Pubkey) -> BotResult<bool> {
        self.cache.is_token_known(mint).await
    }

    /// Display recent token discoveries
    pub async fn display_recent_discoveries(&self, limit: usize) -> BotResult<()> {
        let tokens = self.get_recent_discoveries(Some(limit)).await?;
        let stats = self.get_discovery_statistics().await?;
        
        log::info!("📚 Recent Token Discoveries (Last {}):", limit);
        log::info!("{}", "=".repeat(80));
        
        if tokens.is_empty() {
            log::info!("No tokens discovered yet.");
        } else {
            for (i, (mint, symbol, name)) in tokens.iter().enumerate() {
                log::info!("{}. {} ({}) - {}", i + 1, symbol, name, &mint[..8]);
            }
        }
        
        log::info!("");
        log::info!("📊 Discovery Statistics:");
        for (source, count) in stats {
            log::info!("  {} tokens from {}", count, source);
        }
        
        Ok(())
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
