// Provider module: High-level token data access interface
// Single entry point for all token data operations

pub mod fetcher;
pub mod query;
pub mod types;

use crate::tokens::api::ApiClients;
use crate::tokens::cache::CacheManager;
use crate::tokens::provider::fetcher::Fetcher;
use crate::tokens::provider::query::Query;
use crate::tokens::provider::types::{CompleteTokenData, FetchOptions, ProviderStats};
use crate::logger::{log, LogTag};
use crate::tokens::storage::Database;
use crate::tokens::types::{DataSource, TokenMetadata};
use chrono::Utc;
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
        log(LogTag::Tokens, "INFO", "Initializing TokenDataProvider...");

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

        log(LogTag::Tokens, "INFO", "TokenDataProvider initialized successfully");

        Ok(Self {
            fetcher,
            query,
            stats: Arc::new(Mutex::new(ProviderStats::default())),
        })
    }

    /// Hydrate store from database on startup
    fn hydrate_store_from_database(db: &Arc<Database>) -> Result<(), String> {
        use std::time::Instant;
        
        log(LogTag::Tokens, "INFO", "Hydrating store from database...");
        let start = Instant::now();

        let conn = db.get_connection();
        let conn = conn
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT mint, COALESCE(symbol, ''), COALESCE(name, ''), COALESCE(decimals, 6), updated_at FROM tokens ORDER BY updated_at DESC")
            .map_err(|e| format!("Failed to prepare hydration query: {}", e))?;

        let tokens: Vec<crate::tokens::types::Token> = stmt
            .query_map([], |row| {
                let updated_ts: i64 = row.get(4)?;
                let updated_dt = chrono::DateTime::from_timestamp(updated_ts, 0)
                    .unwrap_or_else(|| Utc::now());
                let mint: String = row.get(0)?;
                let symbol: String = row.get(1)?;
                let name: String = row.get(2)?;
                let decimals: u8 = row.get::<_, i64>(3)? as u8;

                Ok(crate::tokens::types::Token {
                    mint,
                    symbol,
                    name,
                    decimals,
                    description: None,
                    image_url: None,
                    header_image_url: None,
                    supply: None,
                    data_source: crate::tokens::types::DataSource::DexScreener,
                    fetched_at: updated_dt,
                    updated_at: updated_dt,
                    price_usd: 0.0,
                    price_sol: 0.0,
                    price_native: "0".to_string(),
                    price_change_m5: None,
                    price_change_h1: None,
                    price_change_h6: None,
                    price_change_h24: None,
                    market_cap: None,
                    fdv: None,
                    liquidity_usd: None,
                    volume_m5: None,
                    volume_h1: None,
                    volume_h6: None,
                    volume_h24: None,
                    txns_m5_buys: None,
                    txns_m5_sells: None,
                    txns_h1_buys: None,
                    txns_h1_sells: None,
                    txns_h6_buys: None,
                    txns_h6_sells: None,
                    txns_h24_buys: None,
                    txns_h24_sells: None,
                    websites: Vec::new(),
                    socials: Vec::new(),
                    mint_authority: None,
                    freeze_authority: None,
                    security_score: None,
                    is_rugged: false,
                    security_risks: Vec::new(),
                    total_holders: None,
                    top_holders: Vec::new(),
                    creator_balance_pct: None,
                    transfer_fee_pct: None,
                    is_blacklisted: false,
                    priority: crate::tokens::priorities::Priority::Medium,
                    first_seen_at: updated_dt,
                    last_price_update: updated_dt,
                })
            })
            .map_err(|e| format!("Failed to query tokens: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        let count = tokens.len();

        // Batch load into store (direct memory access, skip DB write)
        crate::tokens::store::hydrate_from_tokens(tokens)?;

        log(LogTag::Tokens, "INFO", &format!("Store hydrated: {} tokens loaded in {}ms", count, start.elapsed().as_millis()));

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

        log(LogTag::Tokens, "INFO", &format!("Fetching complete data for mint={}", mint));

        let mut rugcheck_info = None;
        let mut sources_used = Vec::new();
        let mut cache_hits = Vec::new();
        let mut cache_misses = Vec::new();

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
                    log(LogTag::Tokens, "ERROR", &format!("Failed to fetch Rugcheck data: {}", e));
                    self.increment_errors();
                }
            }
        }

        // Get unified metadata
        let metadata = self
            .query
            .get_token_metadata(mint)?
            .unwrap_or_else(|| {
                let now = Utc::now().timestamp();
                TokenMetadata {
                    mint: mint.to_string(),
                    symbol: None,
                    name: None,
                    decimals: None,
                    created_at: now,
                    updated_at: now,
                }
            });

        // Update stats
        self.increment_fetches();
        if !cache_hits.is_empty() {
            self.increment_cache_hits();
        }
        if !cache_misses.is_empty() {
            self.increment_cache_misses();
        }

        log(LogTag::Tokens, "INFO", &format!("Fetched complete data for mint={}: {} sources, {} cache hits, {} cache misses", mint, sources_used.len(), cache_hits.len(), cache_misses.len()));

        Ok(CompleteTokenData {
            mint: mint.to_string(),
            metadata,
            rugcheck_info,
            sources_used,
            fetch_timestamp: fetch_start,
            cache_hits,
            cache_misses,
        })
    }

    /// Get token metadata from database (no API fetch)
    pub fn get_token_metadata(&self, mint: &str) -> Result<Option<TokenMetadata>, String> {
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
