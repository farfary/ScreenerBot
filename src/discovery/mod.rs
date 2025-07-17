pub mod sources;

use crate::config::DiscoveryConfig;
use crate::database::Database;
use crate::logger::Logger;
use crate::types::DiscoveryStats;
use anyhow::{ Context, Result };
use chrono::Utc;
use reqwest::Client;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

use sources::{ dexscreener::DexScreenerSource, rugcheck::RugCheckSource, SourceTrait };

pub struct Discovery {
    config: DiscoveryConfig,
    database: Arc<Database>,
    client: Client,
    known_mints: Arc<RwLock<HashSet<String>>>,
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
        let sources: Vec<Box<dyn SourceTrait>> = vec![
            Box::new(DexScreenerSource::new(client.clone())),
            Box::new(RugCheckSource::new(client.clone()))
        ];

        Self {
            config,
            database,
            client,
            known_mints: Arc::new(RwLock::new(HashSet::new())),
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

        Logger::discovery("Discovery module started");

        // Load existing mints from database
        self.load_existing_mints().await?;

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

    async fn load_existing_mints(&self) -> Result<()> {
        let mints = self.database.get_all_mints().context("Failed to load mints from database")?;

        let mut known_mints = self.known_mints.write().await;
        for mint in mints {
            known_mints.insert(mint);
        }

        Ok(())
    }

    async fn run_discovery_loop(&self) {
        let mut interval = time::interval(Duration::from_secs(self.config.interval_seconds));
        let start_time = Utc::now();
        let mut mints_discovered_this_session = 0u64;

        loop {
            interval.tick().await;

            let is_running = self.is_running.read().await;
            if !*is_running {
                break;
            }
            drop(is_running);

            match self.discover_mints().await {
                Ok(new_mints) => {
                    mints_discovered_this_session += new_mints;

                    // Update stats
                    let elapsed_hours = ((Utc::now() - start_time).num_minutes() as f64) / 60.0;
                    let rate = if elapsed_hours > 0.0 {
                        (mints_discovered_this_session as f64) / elapsed_hours
                    } else {
                        0.0
                    };

                    let (total_mints, _) = (self.database.get_mint_count().unwrap_or(0), 0);

                    let stats = DiscoveryStats {
                        total_tokens_discovered: total_mints,
                        active_tokens: total_mints,
                        last_discovery_run: Utc::now(),
                        discovery_rate_per_hour: rate,
                    };

                    *self.stats.write().await = stats.clone();

                    if let Err(e) = self.database.save_discovery_stats(&stats) {
                        Logger::error(&format!("Failed to save discovery stats: {}", e));
                    }

                    // Only print count of new tokens and total
                    println!("New tokens: {}, Total: {}", new_mints, total_mints);
                }
                Err(e) => {
                    Logger::error(&format!("Discovery failed: {}", e));
                }
            }
        }
    }

    async fn discover_mints(&self) -> Result<u64> {
        let mut new_mints_count = 0u64;

        // Run discovery from all enabled sources
        for source_name in &self.config.sources {
            if let Some(source) = self.find_source_by_name(source_name) {
                match source.discover_mints().await {
                    Ok(mints) => {
                        let processed = self.process_discovered_mints(mints).await?;
                        new_mints_count += processed;
                    }
                    Err(e) => {
                        Logger::error(&format!("Failed to discover from {}: {}", source.name(), e));
                    }
                }
            } else {
                Logger::warn(&format!("Unknown discovery source: {}", source_name));
            }
        }

        Ok(new_mints_count)
    }

    fn find_source_by_name(&self, name: &str) -> Option<&Box<dyn SourceTrait>> {
        self.sources
            .iter()
            .find(|source| {
                source.name().to_lowercase() == name.to_lowercase() ||
                    source.name().to_lowercase().contains(&name.to_lowercase())
            })
    }

    async fn process_discovered_mints(&self, mints: Vec<String>) -> Result<u64> {
        let mut new_mints_count = 0u64;
        let mut known_mints = self.known_mints.write().await;

        for mint in mints {
            // Skip if we already have this mint
            if !known_mints.contains(&mint) {
                // Save to database
                if let Err(e) = self.database.save_mint(&mint) {
                    Logger::error(&format!("Failed to save mint {}: {}", mint, e));
                    continue;
                }

                // Add to known mints
                known_mints.insert(mint);
                new_mints_count += 1;
            }
        }

        Ok(new_mints_count)
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
            known_mints: Arc::clone(&self.known_mints),
            is_running: Arc::clone(&self.is_running),
            stats: Arc::clone(&self.stats),
            sources,
        }
    }
}
