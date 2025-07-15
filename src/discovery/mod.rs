pub mod sources;

use crate::config::DiscoveryConfig;
use crate::database::Database;
use crate::logger::Logger;
use crate::types::{ TokenInfo, DiscoveryStats };
use anyhow::{ Context, Result };
use chrono::Utc;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

use sources::{ dexscreener::DexScreenerSource, rugcheck::RugCheckSource, SourceTrait };

pub struct Discovery {
    config: DiscoveryConfig,
    database: Arc<Database>,
    client: Client,
    token_cache: Arc<RwLock<HashMap<String, TokenInfo>>>,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<DiscoveryStats>>,
    sources: Vec<Box<dyn SourceTrait>>,
}

impl Discovery {
    pub fn new(config: DiscoveryConfig, database: Arc<Database>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("ScreenerBot/1.0")
            .build()
            .expect("Failed to create HTTP client");

        let stats = DiscoveryStats {
            total_tokens_discovered: 0,
            active_tokens: 0,
            last_discovery_run: Utc::now(),
            discovery_rate_per_hour: 0.0,
        };

        // Initialize all discovery sources
        let mut sources: Vec<Box<dyn SourceTrait>> = Vec::new();
        sources.push(Box::new(DexScreenerSource::new(client.clone())));
        sources.push(Box::new(RugCheckSource::new(client.clone())));

        Self {
            config,
            database,
            client,
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
            sources,
        }
    }

    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            Logger::warn("Discovery module is disabled in config");
            return Ok(());
        }

        let mut is_running = self.is_running.write().await;
        if *is_running {
            Logger::warn("Discovery is already running");
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        Logger::success("Discovery module started");

        // Load existing tokens from database
        self.load_existing_tokens().await?;

        // Start discovery loop
        let discovery = self.clone();
        tokio::spawn(async move {
            discovery.run_discovery_loop().await;
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        Logger::info("Discovery module stopped");
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    pub async fn get_stats(&self) -> DiscoveryStats {
        self.stats.read().await.clone()
    }

    pub async fn get_cached_tokens(&self) -> HashMap<String, TokenInfo> {
        self.token_cache.read().await.clone()
    }

    pub async fn get_token(&self, mint: &str) -> Option<TokenInfo> {
        self.token_cache.read().await.get(mint).cloned()
    }

    async fn load_existing_tokens(&self) -> Result<()> {
        Logger::discovery("Loading existing tokens from database...");

        let tokens = self.database
            .get_active_tokens(None)
            .context("Failed to load tokens from database")?;

        let mut cache = self.token_cache.write().await;
        for token in tokens {
            cache.insert(token.mint.clone(), token);
        }

        Logger::discovery(&format!("Loaded {} tokens from database", cache.len()));
        Ok(())
    }

    async fn run_discovery_loop(&self) {
        Logger::discovery("Starting discovery loop...");

        let mut interval = time::interval(Duration::from_secs(self.config.interval_seconds));
        let start_time = Utc::now();
        let mut tokens_discovered_this_session = 0u64;

        loop {
            interval.tick().await;

            let is_running = self.is_running.read().await;
            if !*is_running {
                break;
            }
            drop(is_running);

            Logger::discovery("Running token discovery...");

            match self.discover_tokens().await {
                Ok(new_tokens) => {
                    tokens_discovered_this_session += new_tokens;

                    // Update stats
                    let elapsed_hours = ((Utc::now() - start_time).num_minutes() as f64) / 60.0;
                    let rate = if elapsed_hours > 0.0 {
                        (tokens_discovered_this_session as f64) / elapsed_hours
                    } else {
                        0.0
                    };

                    let (total_tokens, active_tokens) = self.database
                        .get_token_count()
                        .unwrap_or((0, 0));

                    let stats = DiscoveryStats {
                        total_tokens_discovered: total_tokens,
                        active_tokens,
                        last_discovery_run: Utc::now(),
                        discovery_rate_per_hour: rate,
                    };

                    *self.stats.write().await = stats.clone();

                    if let Err(e) = self.database.save_discovery_stats(&stats) {
                        Logger::error(&format!("Failed to save discovery stats: {}", e));
                    }

                    if new_tokens > 0 {
                        Logger::discovery(
                            &format!(
                                "🎯 Discovery complete! Found {} NEW tokens this run! Total: {}, Active: {}, Rate: {:.2}/hour",
                                new_tokens,
                                total_tokens,
                                active_tokens,
                                rate
                            )
                        );
                    }
                }
                Err(e) => {
                    Logger::error(&format!("Discovery failed: {}", e));
                }
            }
        }

        Logger::discovery("Discovery loop stopped");
    }

    async fn discover_tokens(&self) -> Result<u64> {
        let mut new_tokens_count = 0u64;

        // Run discovery from all enabled sources
        for source_name in &self.config.sources {
            if let Some(source) = self.find_source_by_name(source_name) {
                Logger::discovery(&format!("🔍 Discovering from {}...", source.name()));

                match source.discover().await {
                    Ok(tokens) => {
                        let processed = self.process_discovered_tokens(tokens, source_name).await?;
                        new_tokens_count += processed;

                        if processed > 0 {
                            Logger::discovery(
                                &format!(
                                    "✅ {} found {} new tokens from {}",
                                    "DISCOVERY",
                                    processed,
                                    source.name()
                                )
                            );
                        } else {
                            Logger::discovery(
                                &format!(
                                    "📊 {} checked {} - no new tokens",
                                    "DISCOVERY",
                                    source.name()
                                )
                            );
                        }
                    }
                    Err(e) => {
                        Logger::error(
                            &format!("❌ Failed to discover from {}: {}", source.name(), e)
                        );
                    }
                }
            } else {
                Logger::warn(&format!("Unknown discovery source: {}", source_name));
            }
        }

        Ok(new_tokens_count)
    }

    fn find_source_by_name(&self, name: &str) -> Option<&Box<dyn SourceTrait>> {
        self.sources
            .iter()
            .find(|source| {
                source.name().to_lowercase() == name.to_lowercase() ||
                    source.name().to_lowercase().contains(&name.to_lowercase())
            })
    }

    fn meets_criteria(&self, token: &TokenInfo) -> bool {
        // Check against blacklist
        if self.config.blacklisted_tokens.contains(&token.mint) {
            return false;
        }

        // Check liquidity
        if let Some(liquidity) = token.liquidity {
            if liquidity < self.config.min_liquidity {
                return false;
            }
        }

        // Check volume
        if let Some(volume) = token.volume_24h {
            if volume < self.config.min_volume_24h {
                return false;
            }
        }

        // Check market cap bounds
        if let Some(market_cap) = token.market_cap {
            if let Some(min_cap) = self.config.min_market_cap {
                if market_cap < min_cap {
                    return false;
                }
            }
            if let Some(max_cap) = self.config.max_market_cap {
                if market_cap > max_cap {
                    return false;
                }
            }
        }

        true
    }

    async fn process_discovered_tokens(&self, tokens: Vec<TokenInfo>, source: &str) -> Result<u64> {
        let mut new_tokens_count = 0u64;
        let mut cache = self.token_cache.write().await;

        for token in tokens {
            // Apply filtering criteria
            if !self.meets_criteria(&token) {
                continue;
            }

            // Check if we already have this token
            if !cache.contains_key(&token.mint) {
                // Save to database
                if let Err(e) = self.database.save_token(&token) {
                    Logger::error(&format!("Failed to save token {}: {}", token.symbol, e));
                    continue;
                }

                // Add to cache
                cache.insert(token.mint.clone(), token.clone());
                new_tokens_count += 1;

                // Show new token discovery with detailed info
                Logger::discovery(
                    &format!(
                        "🆕 NEW TOKEN FOUND via {}: {} ({}) | 💰 MC: ${:.0} | 💧 Liq: ${:.0} | 📊 Vol: ${:.0} | 🔗 {}",
                        source.to_uppercase(),
                        token.symbol,
                        token.name,
                        token.market_cap.unwrap_or(0.0),
                        token.liquidity.unwrap_or(0.0),
                        token.volume_24h.unwrap_or(0.0),
                        &token.mint[..8]
                    )
                );
            } else {
                // Update existing token data
                let mut existing_token = token.clone();
                existing_token.last_updated = Utc::now();

                if let Err(e) = self.database.save_token(&existing_token) {
                    Logger::error(&format!("Failed to update token {}: {}", token.symbol, e));
                    continue;
                }

                cache.insert(token.mint.clone(), existing_token);
            }
        }

        Ok(new_tokens_count)
    }
}

// Implement Clone for Discovery (needed for tokio::spawn)
impl Clone for Discovery {
    fn clone(&self) -> Self {
        // Note: We can't clone the sources easily due to trait objects
        // For the clone, we'll recreate them
        let mut sources: Vec<Box<dyn SourceTrait>> = Vec::new();
        sources.push(Box::new(DexScreenerSource::new(self.client.clone())));
        sources.push(Box::new(RugCheckSource::new(self.client.clone())));

        Self {
            config: self.config.clone(),
            database: Arc::clone(&self.database),
            client: self.client.clone(),
            token_cache: Arc::clone(&self.token_cache),
            is_running: Arc::clone(&self.is_running),
            stats: Arc::clone(&self.stats),
            sources,
        }
    }
}
