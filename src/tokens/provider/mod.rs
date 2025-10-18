// Provider module: High-level token data access interface
// Single entry point for all token data operations

pub mod fetcher;
pub mod query;
pub mod types;

use crate::tokens::api::ApiClients;
use crate::tokens::cache::CacheManager;
use crate::tokens::provider::fetcher::Fetcher;
use crate::tokens::provider::query::Query;
use crate::tokens::provider::types::{
    CompleteTokenData, FetchOptions, ProviderStats, TokenMetadata,
};
use crate::tokens::storage::Database;
use crate::tokens::types::DataSource;
use chrono::Utc;
use log::{error, info};
use std::sync::{Arc, Mutex};

const TOKENS_DB_PATH: &str = "data/tokens.db";

pub use types::{CacheStrategy, FetchResult};

/// Main provider for token data access
pub struct TokenDataProvider {
    fetcher: Arc<Fetcher>,
    query: Arc<Query>,
    stats: Arc<Mutex<ProviderStats>>,
}

impl TokenDataProvider {
    /// Create new provider instance
    pub async fn new() -> Result<Self, String> {
        info!("[TOKENS] Initializing TokenDataProvider...");

        // Get database path from config
        let db_path = TOKENS_DB_PATH;

        // Initialize database
        let database = Arc::new(Database::new(db_path)?);

        // Initialize store with database handle (single source of truth)
        crate::tokens::store::initialize_with_database(Arc::clone(&database))?;

        // Hydrate store from database (load existing tokens into memory)
        Self::hydrate_store_from_database(&database)?;

        // Initialize cache
        let cache_config = crate::tokens::cache::CacheConfig::from_global();
        let cache = Arc::new(CacheManager::new(cache_config));

        // Initialize API clients
        let api_clients = Arc::new(ApiClients::new()?);

        // Create fetcher and query
        let fetcher = Arc::new(Fetcher::new(
            Arc::clone(&api_clients),
            Arc::clone(&cache),
            Arc::clone(&database),
        ));
        let query = Arc::new(Query::new(Arc::clone(&database)));

        info!("[TOKENS] TokenDataProvider initialized successfully");

        Ok(Self {
            fetcher,
            query,
            stats: Arc::new(Mutex::new(ProviderStats::default())),
        })
    }

    /// Hydrate store from database on startup
    fn hydrate_store_from_database(db: &Arc<Database>) -> Result<(), String> {
        use std::time::Instant;
        
        info!("[TOKENS] Hydrating store from database...");
        let start = Instant::now();

        let conn = db.get_connection();
        let conn = conn
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT mint, symbol, name, decimals, updated_at FROM tokens ORDER BY updated_at DESC")
            .map_err(|e| format!("Failed to prepare hydration query: {}", e))?;

        let snapshots: Vec<crate::tokens::store::Snapshot> = stmt
            .query_map([], |row| {
                let updated_ts: i64 = row.get(4)?;
                
                Ok(crate::tokens::store::Snapshot {
                    mint: row.get(0)?,
                    symbol: row.get(1)?,
                    name: row.get(2)?,
                    decimals: row.get(3)?,
                    is_blacklisted: false,
                    best_pool: None,
                    sources: Vec::new(),
                    priority: crate::tokens::priorities::Priority::Medium,
                    fetched_at: None,
                    updated_at: chrono::DateTime::from_timestamp(updated_ts, 0)
                        .unwrap_or_else(|| Utc::now()),
                })
            })
            .map_err(|e| format!("Failed to query tokens: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let count = snapshots.len();

        // Batch load into store (direct memory access, skip DB write)
        crate::tokens::store::hydrate_from_snapshots(snapshots)?;

        info!(
            "[TOKENS] Store hydrated: {} tokens loaded in {}ms",
            count,
            start.elapsed().as_millis()
        );

        Ok(())
    }

    /// Fetch complete token data from all configured sources
    pub async fn fetch_complete_data(
        &self,
        mint: &str,
        options: Option<FetchOptions>,
    ) -> Result<CompleteTokenData, String> {
        let options = options.unwrap_or_default();
        let fetch_start = Utc::now();

        info!("[TOKENS] Fetching complete data for mint={}", mint);

        let mut dexscreener_pools = Vec::new();
        let mut geckoterminal_pools = Vec::new();
        let mut rugcheck_info = None;
        let mut sources_used = Vec::new();
        let mut cache_hits = Vec::new();
        let mut cache_misses = Vec::new();

        // Fetch DexScreener data
        if options.sources.contains(&DataSource::DexScreener) {
            match self.fetcher.fetch_dexscreener_pools(mint, &options).await {
                Ok(result) => {
                    dexscreener_pools = result.data;
                    sources_used.push(DataSource::DexScreener);
                    if result.from_cache {
                        cache_hits.push(DataSource::DexScreener);
                    } else {
                        cache_misses.push(DataSource::DexScreener);
                    }

                    // Update metadata from DexScreener
                    if let Some(pool) = dexscreener_pools.first() {
                        self.fetcher.update_metadata(
                            mint,
                            Some(&pool.base_token_symbol),
                            Some(&pool.base_token_name),
                            None,
                        );
                    }
                }
                Err(e) => {
                    error!("[TOKENS] Failed to fetch DexScreener data: {}", e);
                    self.increment_errors();
                }
            }
        }

        // Fetch GeckoTerminal data
        if options.sources.contains(&DataSource::GeckoTerminal) {
            match self.fetcher.fetch_geckoterminal_pools(mint, &options).await {
                Ok(result) => {
                    geckoterminal_pools = result.data;
                    sources_used.push(DataSource::GeckoTerminal);
                    if result.from_cache {
                        cache_hits.push(DataSource::GeckoTerminal);
                    } else {
                        cache_misses.push(DataSource::GeckoTerminal);
                    }

                    // Update metadata from GeckoTerminal - no name/symbol in pools
                    // Metadata will come from Rugcheck or DexScreener
                }
                Err(e) => {
                    error!("[TOKENS] Failed to fetch GeckoTerminal data: {}", e);
                    self.increment_errors();
                }
            }
        }

        // Fetch Rugcheck data
        if options.sources.contains(&DataSource::Rugcheck) {
            match self.fetcher.fetch_rugcheck_info(mint, &options).await {
                Ok(result) => {
                    rugcheck_info = Some(result.data.clone());
                    sources_used.push(DataSource::Rugcheck);
                    if result.from_cache {
                        cache_hits.push(DataSource::Rugcheck);
                    } else {
                        cache_misses.push(DataSource::Rugcheck);
                    }

                    // Update metadata from Rugcheck
                    self.fetcher.update_metadata(
                        mint,
                        result.data.token_symbol.as_deref(),
                        result.data.token_name.as_deref(),
                        result.data.token_decimals,
                    );
                }
                Err(e) => {
                    error!("[TOKENS] Failed to fetch Rugcheck data: {}", e);
                    self.increment_errors();
                }
            }
        }

        // Get unified metadata
        let metadata = self
            .query
            .get_token_metadata(mint)?
            .map(|m| TokenMetadata {
                mint: m.mint,
                symbol: m.symbol,
                name: m.name,
                decimals: m.decimals,
            })
            .unwrap_or_else(|| TokenMetadata {
                mint: mint.to_string(),
                symbol: None,
                name: None,
                decimals: None,
            });

        // Update stats
        self.increment_fetches();
        if !cache_hits.is_empty() {
            self.increment_cache_hits();
        }
        if !cache_misses.is_empty() {
            self.increment_cache_misses();
        }

        info!(
            "[TOKENS] Fetched complete data for mint={}: {} sources, {} cache hits, {} cache misses",
            mint,
            sources_used.len(),
            cache_hits.len(),
            cache_misses.len()
        );

        Ok(CompleteTokenData {
            mint: mint.to_string(),
            metadata,
            dexscreener_pools,
            geckoterminal_pools,
            rugcheck_info,
            sources_used,
            fetch_timestamp: fetch_start,
            cache_hits,
            cache_misses,
        })
    }

    /// Get token metadata from database (no API fetch)
    pub fn get_token_metadata(&self, mint: &str) -> Result<Option<query::TokenMetadata>, String> {
        self.query.get_token_metadata(mint)
    }

    /// Check if token exists in database
    pub fn token_exists(&self, mint: &str) -> bool {
        self.query.token_exists(mint)
    }

    /// Get all token mints in database
    pub fn get_all_mints(&self) -> Result<Vec<String>, String> {
        self.query.get_all_mints()
    }

    /// Get API clients bundle (read-only) for discovery flows
    pub fn api(&self) -> Arc<ApiClients> {
        self.fetcher.api_clients()
    }

    /// Upsert token metadata fields
    pub fn upsert_token_metadata(
        &self,
        mint: &str,
        symbol: Option<&str>,
        name: Option<&str>,
        decimals: Option<u8>,
    ) -> Result<(), String> {
        self.fetcher.upsert_metadata(mint, symbol, name, decimals)
    }

    /// Get provider statistics
    pub fn get_stats(&self) -> ProviderStats {
        self.stats.lock().unwrap().clone()
    }

    /// Expose database for auxiliary modules (e.g., blacklist hydrate)
    /// Internal: Get database reference for service initialization only
    /// External modules should NEVER access database directly - use store API
    pub(crate) fn database(&self) -> Arc<Database> {
        // Query holds Arc<Database>, fetcher also holds Arc<Database>
        // Prefer returning the fetcher's db to keep a single source
        // SAFETY: both point to same Arc in new()
        // We can extend Fetcher API to expose DB as needed; for now, clone from query (same Arc instance)
        self.query.database.clone()
    }

    // Stats helpers
    fn increment_fetches(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_fetches += 1;
        }
    }

    fn increment_cache_hits(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.cache_hits += 1;
        }
    }

    fn increment_cache_misses(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.cache_misses += 1;
        }
    }

    fn increment_errors(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.errors += 1;
        }
    }
}
