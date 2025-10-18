// Provider fetcher: Orchestrates data fetching from API → Cache → DB

use crate::tokens::api::ApiClients;
use crate::tokens::cache::{CacheKey, CacheManager, DataType};
use crate::tokens::provider::types::{CacheStrategy, FetchOptions, FetchResult};
use crate::tokens::storage::{
    log_api_fetch, save_dexscreener_pools, save_geckoterminal_pools, save_rugcheck_info,
    upsert_token_metadata, Database,
};
use crate::tokens::types::{DataSource, DexScreenerPool, GeckoTerminalPool, RugcheckInfo};
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::time::Instant;

/// Fetcher orchestrates data retrieval from multiple sources
pub struct Fetcher {
    api_clients: Arc<ApiClients>,
    cache: Arc<CacheManager>,
    database: Arc<Database>,
}

impl Fetcher {
    pub fn new(
        api_clients: Arc<ApiClients>,
        cache: Arc<CacheManager>,
        database: Arc<Database>,
    ) -> Self {
        Self {
            api_clients,
            cache,
            database,
        }
    }

    /// Expose database reference for provider-level helpers
    pub fn database(&self) -> &Arc<Database> {
        &self.database
    }

    /// Expose API clients for read-only auxiliary operations (e.g., discovery)
    pub fn api_clients(&self) -> Arc<ApiClients> {
        Arc::clone(&self.api_clients)
    }

    /// Upsert token metadata fields conveniently
    pub fn upsert_metadata(
        &self,
        mint: &str,
        symbol: Option<&str>,
        name: Option<&str>,
        decimals: Option<u8>,
    ) -> Result<(), String> {
        upsert_token_metadata(&self.database, mint, symbol, name, decimals)
    }

    /// Fetch DexScreener pools for a token
    pub async fn fetch_dexscreener_pools(
        &self,
        mint: &str,
        options: &FetchOptions,
    ) -> Result<FetchResult<Vec<DexScreenerPool>>, String> {
        let start = Instant::now();
        let cache_key = CacheKey {
            source: DataSource::DexScreener,
            data_type: DataType::Pools,
            identifier: mint.to_string(),
        };

        // Try cache first if strategy allows
        if options.cache_strategy == CacheStrategy::CacheFirst
            || options.cache_strategy == CacheStrategy::CacheOnly
        {
            if let Some(cached) = self.cache.get::<Vec<DexScreenerPool>>(&cache_key) {
                debug!("[TOKENS] DexScreener pools cache HIT: mint={}", mint);
                return Ok(FetchResult {
                    data: cached,
                    source: DataSource::DexScreener,
                    from_cache: true,
                    fetch_duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }

        // Return error if cache-only and miss
        if options.cache_strategy == CacheStrategy::CacheOnly {
            return Err(format!("DexScreener pools not in cache: {}", mint));
        }

        // Fetch from API
        debug!(
            "[TOKENS] Fetching DexScreener pools from API: mint={}",
            mint
        );
        let pools = self.api_clients.dexscreener.fetch_pools(mint).await?;

        // Save to cache
        self.cache.set(cache_key, &pools)?;

        // Save to database if persist enabled
        if options.persist {
            if let Err(e) = save_dexscreener_pools(&self.database, mint, &pools) {
                error!("[TOKENS] Failed to save DexScreener pools to DB: {}", e);
            }
        }

        // Log fetch
        let _ = log_api_fetch(
            &self.database,
            mint,
            DataSource::DexScreener,
            true,
            None,
            Some(pools.len()),
        );

        info!(
            "[TOKENS] Fetched {} DexScreener pools for mint={} in {}ms",
            pools.len(),
            mint,
            start.elapsed().as_millis()
        );

        Ok(FetchResult {
            data: pools,
            source: DataSource::DexScreener,
            from_cache: false,
            fetch_duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Fetch GeckoTerminal pools for a token
    pub async fn fetch_geckoterminal_pools(
        &self,
        mint: &str,
        options: &FetchOptions,
    ) -> Result<FetchResult<Vec<GeckoTerminalPool>>, String> {
        let start = Instant::now();
        let cache_key = CacheKey {
            source: DataSource::GeckoTerminal,
            data_type: DataType::Pools,
            identifier: mint.to_string(),
        };

        // Try cache first if strategy allows
        if options.cache_strategy == CacheStrategy::CacheFirst
            || options.cache_strategy == CacheStrategy::CacheOnly
        {
            if let Some(cached) = self.cache.get::<Vec<GeckoTerminalPool>>(&cache_key) {
                debug!("[TOKENS] GeckoTerminal pools cache HIT: mint={}", mint);
                return Ok(FetchResult {
                    data: cached,
                    source: DataSource::GeckoTerminal,
                    from_cache: true,
                    fetch_duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }

        // Return error if cache-only and miss
        if options.cache_strategy == CacheStrategy::CacheOnly {
            return Err(format!("GeckoTerminal pools not in cache: {}", mint));
        }

        // Fetch from API
        debug!(
            "[TOKENS] Fetching GeckoTerminal pools from API: mint={}",
            mint
        );
        let pools = self.api_clients.geckoterminal.fetch_pools(mint).await?;

        // Save to cache
        self.cache.set(cache_key, &pools)?;

        // Save to database if persist enabled
        if options.persist {
            if let Err(e) = save_geckoterminal_pools(&self.database, mint, &pools) {
                error!(
                    "[TOKENS] Failed to save GeckoTerminal pools to DB: {}",
                    e
                );
            }
        }

        // Log fetch
        let _ = log_api_fetch(
            &self.database,
            mint,
            DataSource::GeckoTerminal,
            true,
            None,
            Some(pools.len()),
        );

        info!(
            "[TOKENS] Fetched {} GeckoTerminal pools for mint={} in {}ms",
            pools.len(),
            mint,
            start.elapsed().as_millis()
        );

        Ok(FetchResult {
            data: pools,
            source: DataSource::GeckoTerminal,
            from_cache: false,
            fetch_duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Fetch Rugcheck info for a token
    pub async fn fetch_rugcheck_info(
        &self,
        mint: &str,
        options: &FetchOptions,
    ) -> Result<FetchResult<RugcheckInfo>, String> {
        let start = Instant::now();
        let cache_key = CacheKey {
            source: DataSource::Rugcheck,
            data_type: DataType::Info,
            identifier: mint.to_string(),
        };

        // Try cache first if strategy allows
        if options.cache_strategy == CacheStrategy::CacheFirst
            || options.cache_strategy == CacheStrategy::CacheOnly
        {
            if let Some(cached) = self.cache.get::<RugcheckInfo>(&cache_key) {
                debug!("[TOKENS] Rugcheck info cache HIT: mint={}", mint);
                return Ok(FetchResult {
                    data: cached,
                    source: DataSource::Rugcheck,
                    from_cache: true,
                    fetch_duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }

        // Return error if cache-only and miss
        if options.cache_strategy == CacheStrategy::CacheOnly {
            return Err(format!("Rugcheck info not in cache: {}", mint));
        }

        // Fetch from API
        debug!(
            "[TOKENS] Fetching Rugcheck info from API: mint={}",
            mint
        );
        let info = self.api_clients.rugcheck.fetch_report(mint).await?;

        // Save to cache
        self.cache.set(cache_key, &info)?;

        // Save to database if persist enabled
        if options.persist {
            if let Err(e) = save_rugcheck_info(&self.database, mint, &info) {
                error!("[TOKENS] Failed to save Rugcheck info to DB: {}", e);
            }
        }

        // Log fetch
        let _ = log_api_fetch(
            &self.database,
            mint,
            DataSource::Rugcheck,
            true,
            None,
            Some(1),
        );

        info!(
            "[TOKENS] Fetched Rugcheck info for mint={} in {}ms",
            mint,
            start.elapsed().as_millis()
        );

        Ok(FetchResult {
            data: info,
            source: DataSource::Rugcheck,
            from_cache: false,
            fetch_duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Update token metadata from fetched data
    pub fn update_metadata(
        &self,
        mint: &str,
        symbol: Option<&str>,
        name: Option<&str>,
        decimals: Option<u8>,
    ) {
        if let Err(e) = upsert_token_metadata(&self.database, mint, symbol, name, decimals) {
            warn!("[TOKENS] Failed to update token metadata: {}", e);
        }
    }
}
