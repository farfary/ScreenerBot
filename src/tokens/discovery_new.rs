/// Token discovery system for finding new tokens from multiple sources
use crate::logger::{ log, LogTag };
use crate::tokens::api::DexScreenerApi;
use crate::tokens::cache::TokenDatabase;
use crate::tokens::types::*;
use std::collections::HashSet;
use tokio::time::{ sleep, Duration };

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
    pub async fn discover_new_tokens(
        &mut self
    ) -> Result<Vec<DiscoveryResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        log(LogTag::System, "DISCOVERY", "Starting token discovery cycle");

        for source in &self.sources.clone() {
            match self.discover_from_source(source).await {
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
                    } else {
                        log(
                            LogTag::System,
                            "WARN",
                            &format!(
                                "Discovery failed for {}: {}",
                                result.source,
                                result.error.unwrap_or_default()
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
        source: &DiscoverySource
    ) -> Result<DiscoveryResult, Box<dyn std::error::Error>> {
        let now = chrono::Utc::now();

        match source {
            DiscoverySource::DexScreener { chain } => {
                self.discover_from_dexscreener(chain, now).await
            }
            DiscoverySource::RugCheck => { self.discover_from_rugcheck(now).await }
            DiscoverySource::TrendingBots => { self.discover_from_trending_bots(now).await }
        }
    }

    /// Discover tokens from DexScreener trending/new listings
    async fn discover_from_dexscreener(
        &mut self,
        chain: &str,
        timestamp: chrono::DateTime<chrono::Utc>
    ) -> Result<DiscoveryResult, Box<dyn std::error::Error>> {
        let source = format!("DexScreener-{}", chain);

        match self.api.get_trending_tokens(chain, MAX_NEW_TOKENS_PER_CYCLE).await {
            Ok(tokens) => {
                // Filter out tokens we already have
                let existing_mints = self.get_existing_mints().await?;
                let new_tokens: Vec<ApiToken> = tokens
                    .into_iter()
                    .filter(|token| !existing_mints.contains(&token.mint))
                    .collect();

                // Save new tokens to database
                if !new_tokens.is_empty() {
                    self.database.add_tokens(&new_tokens).await?;
                    log(
                        LogTag::System,
                        "CACHE",
                        &format!("Saved {} new tokens to database", new_tokens.len())
                    );
                }

                Ok(DiscoveryResult {
                    source,
                    new_tokens,
                    timestamp,
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                Ok(DiscoveryResult {
                    source,
                    new_tokens: Vec::new(),
                    timestamp,
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Discover tokens from RugCheck (placeholder)
    async fn discover_from_rugcheck(
        &mut self,
        timestamp: chrono::DateTime<chrono::Utc>
    ) -> Result<DiscoveryResult, Box<dyn std::error::Error>> {
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
        timestamp: chrono::DateTime<chrono::Utc>
    ) -> Result<DiscoveryResult, Box<dyn std::error::Error>> {
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
    async fn get_existing_mints(&self) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
        let tokens = self.database.get_all_tokens().await?;
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
}

// =============================================================================
// DISCOVERY HELPER FUNCTIONS
// =============================================================================

/// Start token discovery in background task
pub async fn start_token_discovery(
    shutdown: std::sync::Arc<tokio::sync::Notify>
) -> Result<tokio::task::JoinHandle<()>, Box<dyn std::error::Error>> {
    let mut discovery = TokenDiscovery::new()?;

    let handle = tokio::spawn(async move {
        discovery.start_discovery_loop(shutdown).await;
    });

    Ok(handle)
}

/// Manual token discovery trigger (for testing)
pub async fn discover_tokens_once() -> Result<Vec<DiscoveryResult>, Box<dyn std::error::Error>> {
    let mut discovery = TokenDiscovery::new()?;
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
