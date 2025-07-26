/// Token discovery system for finding new tokens from multiple sources
use crate::logger::{ log, LogTag };
use crate::tokens::api::DexScreenerApi;
use crate::tokens::cache::TokenDatabase;
use crate::tokens::types::*;
use crate::tokens::decimals::get_token_decimals_from_chain;
use std::collections::HashSet;
use tokio::time::{ sleep, Duration };
use std::sync::Arc;

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Discovery cycle duration in minutes
pub const DISCOVERY_CYCLE_MINUTES: u64 = 3;

/// Maximum new tokens to discover per cycle
pub const MAX_NEW_TOKENS_PER_CYCLE: usize = 50;

/// Rate limit for discovery API calls (per minute)
pub const DISCOVERY_RATE_LIMIT_PER_MINUTE: usize = 30;

// =============================================================================
// TOKEN DISCOVERY SOURCES
// =============================================================================

/// Token discovery source configuration
#[derive(Debug, Clone)]
pub enum DiscoverySource {
    DexScreener {
        chain: String,
    },
    RugCheck,
    TrendingBots,
}

/// Discovery result containing new tokens
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub source: String,
    pub new_tokens: Vec<ApiToken>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub success: bool,
    pub error: Option<String>,
}

// =============================================================================
// TOKEN DISCOVERY MANAGER
// =============================================================================

pub struct TokenDiscovery {
    api: DexScreenerApi,
    database: TokenDatabase,
    sources: Vec<DiscoverySource>,
}

impl TokenDiscovery {
    /// Create new token discovery instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api = DexScreenerApi::new();
        let database = TokenDatabase::new()?;

        let sources = vec![
            DiscoverySource::DexScreener { chain: "solana".to_string() }
            // Add more sources as needed
        ];

        Ok(Self {
            api,
            database,
            sources,
        })
    }

    /// Discover new tokens from all configured sources
    pub async fn discover_new_tokens(&mut self) -> Result<Vec<DiscoveryResult>, String> {
        let mut results = Vec::new();

        log(LogTag::System, "DISCOVERY", "Starting token discovery cycle");

        // Get existing token mints from database to avoid duplicates
        let existing_mints = self.get_existing_mints().await?;
        log(
            LogTag::System,
            "DISCOVERY",
            &format!("Found {} existing tokens in database", existing_mints.len())
        );

        for source in &self.sources.clone() {
            match self.discover_from_source(source, &existing_mints).await {
                Ok(result) => {
                    if result.success {
                        log(
                            LogTag::System,
                            "SUCCESS",
                            &format!(
                                "Discovered {} new tokens from {}",
                                result.new_tokens.len(),
                                result.source
                            )
                        );

                        // Save new tokens to database
                        if !result.new_tokens.is_empty() {
                            // First, fetch decimals for the new tokens
                            let tokens_with_decimals = self.fetch_decimals_for_tokens(
                                result.new_tokens.clone()
                            ).await;

                            if let Err(e) = self.database.add_tokens(&tokens_with_decimals).await {
                                log(
                                    LogTag::System,
                                    "ERROR",
                                    &format!("Failed to save tokens to database: {}", e)
                                );
                            } else {
                                log(
                                    LogTag::System,
                                    "SUCCESS",
                                    &format!(
                                        "Saved {} new tokens to database with accurate decimals",
                                        tokens_with_decimals.len()
                                    )
                                );
                            }
                        }
                    } else {
                        log(
                            LogTag::System,
                            "WARN",
                            &format!(
                                "Discovery failed for {}: {}",
                                result.source,
                                result.error.clone().unwrap_or_default()
                            )
                        );
                    }
                    results.push(result);
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Discovery error for {:?}: {}", source, e)
                    );
                }
            }

            // Rate limiting delay between sources
            sleep(Duration::from_secs(2)).await;
        }

        log(
            LogTag::System,
            "DISCOVERY",
            &format!("Completed discovery cycle with {} sources", results.len())
        );

        Ok(results)
    }

    /// Discover tokens from a specific source
    async fn discover_from_source(
        &mut self,
        source: &DiscoverySource,
        existing_mints: &HashSet<String>
    ) -> Result<DiscoveryResult, String> {
        let now = chrono::Utc::now();

        match source {
            DiscoverySource::DexScreener { chain } => {
                self.discover_from_dexscreener(chain, now, existing_mints).await
            }
            DiscoverySource::RugCheck => { self.discover_from_rugcheck(now, existing_mints).await }
            DiscoverySource::TrendingBots => {
                self.discover_from_trending_bots(now, existing_mints).await
            }
        }
    }

    /// Discover tokens from DexScreener using multiple endpoints
    async fn discover_from_dexscreener(
        &mut self,
        chain: &str,
        timestamp: chrono::DateTime<chrono::Utc>,
        existing_mints: &HashSet<String>
    ) -> Result<DiscoveryResult, String> {
        let source = format!("DexScreener-{}", chain);

        log(LogTag::System, "DISCOVERY", &format!("Starting DexScreener discovery for {}", chain));

        let mut all_new_tokens = Vec::new();

        // Method 1: Token boosts (trending/promoted tokens)
        match
            self.api.discover_and_fetch_tokens(
                DiscoverySourceType::DexScreenerBoosts,
                MAX_NEW_TOKENS_PER_CYCLE / 2
            ).await
        {
            Ok(boost_tokens) => {
                let new_boost_tokens: Vec<ApiToken> = boost_tokens
                    .into_iter()
                    .filter(|token| !existing_mints.contains(&token.mint))
                    .collect();

                log(
                    LogTag::System,
                    "DISCOVERY",
                    &format!("Found {} new tokens from boosts", new_boost_tokens.len())
                );
                all_new_tokens.extend(new_boost_tokens);
            }
            Err(e) => {
                log(LogTag::System, "WARN", &format!("Boost discovery failed: {}", e));
            }
        }

        // Method 2: Token profiles (recently created profiles)
        match
            self.api.discover_and_fetch_tokens(
                DiscoverySourceType::DexScreenerProfiles,
                MAX_NEW_TOKENS_PER_CYCLE / 2
            ).await
        {
            Ok(profile_tokens) => {
                let new_profile_tokens: Vec<ApiToken> = profile_tokens
                    .into_iter()
                    .filter(|token| !existing_mints.contains(&token.mint))
                    .collect();

                log(
                    LogTag::System,
                    "DISCOVERY",
                    &format!("Found {} new tokens from profiles", new_profile_tokens.len())
                );
                all_new_tokens.extend(new_profile_tokens);
            }
            Err(e) => {
                log(LogTag::System, "WARN", &format!("Profile discovery failed: {}", e));
            }
        }

        // Method 3: Top tokens by volume
        match self.api.get_top_tokens(MAX_NEW_TOKENS_PER_CYCLE / 2).await {
            Ok(top_mints) => {
                let new_top_mints: Vec<String> = top_mints
                    .into_iter()
                    .filter(|mint| !existing_mints.contains(mint))
                    .collect();

                if !new_top_mints.is_empty() {
                    log(
                        LogTag::System,
                        "DISCOVERY",
                        &format!(
                            "Found {} new top tokens, fetching details...",
                            new_top_mints.len()
                        )
                    );

                    // Fetch detailed info for top tokens
                    match self.api.get_multiple_token_data(&new_top_mints).await {
                        Ok(top_tokens) => {
                            log(
                                LogTag::System,
                                "SUCCESS",
                                &format!("Fetched details for {} top tokens", top_tokens.len())
                            );
                            all_new_tokens.extend(top_tokens);
                        }
                        Err(e) => {
                            log(
                                LogTag::System,
                                "WARN",
                                &format!("Failed to fetch top token details: {}", e)
                            );
                        }
                    }
                }
            }
            Err(e) => {
                log(LogTag::System, "WARN", &format!("Top tokens discovery failed: {}", e));
            }
        }

        // Remove duplicates within discovered tokens
        let mut unique_tokens = Vec::new();
        let mut seen_mints = HashSet::new();

        for token in all_new_tokens {
            if seen_mints.insert(token.mint.clone()) {
                unique_tokens.push(token);
            }
        }

        log(
            LogTag::System,
            "DISCOVERY",
            &format!(
                "DexScreener discovery completed: {} unique new tokens found",
                unique_tokens.len()
            )
        );

        Ok(DiscoveryResult {
            source,
            new_tokens: unique_tokens,
            timestamp,
            success: true,
            error: None,
        })
    }

    /// Discover tokens from RugCheck (placeholder)
    async fn discover_from_rugcheck(
        &mut self,
        timestamp: chrono::DateTime<chrono::Utc>,
        _existing_mints: &HashSet<String>
    ) -> Result<DiscoveryResult, String> {
        // TODO: Implement RugCheck API integration
        Ok(DiscoveryResult {
            source: "RugCheck".to_string(),
            new_tokens: Vec::new(),
            timestamp,
            success: false,
            error: Some("Not implemented yet".to_string()),
        })
    }

    /// Discover tokens from trending bots (placeholder)
    async fn discover_from_trending_bots(
        &mut self,
        timestamp: chrono::DateTime<chrono::Utc>,
        _existing_mints: &HashSet<String>
    ) -> Result<DiscoveryResult, String> {
        // TODO: Implement trending bot integration
        Ok(DiscoveryResult {
            source: "TrendingBots".to_string(),
            new_tokens: Vec::new(),
            timestamp,
            success: false,
            error: Some("Not implemented yet".to_string()),
        })
    }

    /// Get set of existing token mints from database
    async fn get_existing_mints(&self) -> Result<HashSet<String>, String> {
        let tokens = self.database
            .get_all_tokens().await
            .map_err(|e| format!("Failed to get tokens from database: {}", e))?;
        Ok(
            tokens
                .into_iter()
                .map(|t| t.mint)
                .collect()
        )
    }

    /// Start continuous discovery loop
    pub async fn start_discovery_loop(&mut self, shutdown: std::sync::Arc<tokio::sync::Notify>) {
        log(LogTag::System, "START", "Token discovery manager started");

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::System, "SHUTDOWN", "Token discovery manager stopping");
                    break;
                }
                
                _ = sleep(Duration::from_secs(DISCOVERY_CYCLE_MINUTES * 60)) => {
                    if let Err(e) = self.discover_new_tokens().await {
                        log(LogTag::System, "ERROR", 
                            &format!("Discovery cycle failed: {}", e));
                    }
                }
            }
        }

        log(LogTag::System, "STOP", "Token discovery manager stopped");
    }

    /// Fetch decimals for new tokens before adding to database
    async fn fetch_decimals_for_tokens(&self, mut tokens: Vec<ApiToken>) -> Vec<ApiToken> {
        log(
            LogTag::System,
            "DECIMALS",
            &format!("Fetching decimals for {} tokens...", tokens.len())
        );

        let mut updated_count = 0;

        for token in &mut tokens {
            match get_token_decimals_from_chain(&token.mint).await {
                Ok(decimals) => {
                    if decimals != token.decimals {
                        log(
                            LogTag::System,
                            "DECIMALS",
                            &format!(
                                "Updated {} decimals: {} -> {}",
                                token.symbol,
                                token.decimals,
                                decimals
                            )
                        );
                        token.decimals = decimals;
                        updated_count += 1;
                    }
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "WARN",
                        &format!(
                            "Failed to fetch decimals for {}: {}, keeping default ({})",
                            token.symbol,
                            e,
                            token.decimals
                        )
                    );
                }
            }

            // Small delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        log(
            LogTag::System,
            "SUCCESS",
            &format!("Updated decimals for {}/{} tokens", updated_count, tokens.len())
        );

        tokens
    }
}

// =============================================================================
// DISCOVERY HELPER FUNCTIONS
// =============================================================================

/// Start token discovery in background task
pub async fn start_token_discovery(
    shutdown: std::sync::Arc<tokio::sync::Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    log(crate::logger::LogTag::System, "START", "Token discovery background task started");

    let handle = tokio::spawn(async move {
        let mut discovery = match TokenDiscovery::new() {
            Ok(discovery) => discovery,
            Err(e) => {
                log(
                    crate::logger::LogTag::System,
                    "ERROR",
                    &format!("Failed to initialize token discovery: {}", e)
                );
                return;
            }
        };

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(
                        crate::logger::LogTag::System,
                        "SHUTDOWN",
                        "Token discovery background task stopping"
                    );
                    break;
                }
                
                _ = tokio::time::sleep(std::time::Duration::from_secs(300)) => { // 5 minutes
                    log(
                        crate::logger::LogTag::System,
                        "DISCOVERY",
                        "Running token discovery cycle"
                    );
                    
                    if let Err(e) = discovery.discover_new_tokens().await {
                        log(
                            crate::logger::LogTag::System,
                            "ERROR",
                            &format!("Token discovery failed: {}", e)
                        );
                    }
                }
            }
        }
    });

    Ok(handle)
}

/// Manual token discovery trigger (for testing)
pub async fn discover_tokens_once() -> Result<Vec<DiscoveryResult>, String> {
    let mut discovery = TokenDiscovery::new().map_err(|e|
        format!("Failed to create discovery: {}", e)
    )?;
    discovery.discover_new_tokens().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_discovery_creation() {
        let discovery = TokenDiscovery::new();
        assert!(discovery.is_ok());
    }

    #[tokio::test]
    async fn test_manual_discovery() {
        let result = discover_tokens_once().await;
        // Should not fail even if no tokens found
        assert!(result.is_ok());
    }
}
