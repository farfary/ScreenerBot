/// Cache configuration and TTL strategies
use crate::config::with_config;
use crate::tokens::types::DataSource;
use std::time::Duration;

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub dexscreener_pools_ttl: Duration,
    pub geckoterminal_pools_ttl: Duration,
    pub rugcheck_info_ttl: Duration,
}

impl CacheConfig {
    /// Load configuration from global config
    pub fn from_global() -> Self {
        let (dex_ttl, gecko_ttl, rug_ttl) = with_config(|cfg| {
            (
                cfg.tokens.sources.dexscreener.cache_ttl_seconds,
                cfg.tokens.sources.geckoterminal.cache_ttl_seconds,
                cfg.tokens.sources.rugcheck.cache_ttl_seconds,
            )
        });

        Self {
            dexscreener_pools_ttl: Duration::from_secs(dex_ttl),
            geckoterminal_pools_ttl: Duration::from_secs(gecko_ttl),
            rugcheck_info_ttl: Duration::from_secs(rug_ttl),
        }
    }

    /// Get TTL for a specific source
    pub fn get_ttl(&self, source: DataSource) -> Duration {
        match source {
            DataSource::DexScreener => self.dexscreener_pools_ttl,
            DataSource::GeckoTerminal => self.geckoterminal_pools_ttl,
            DataSource::Rugcheck => self.rugcheck_info_ttl,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            dexscreener_pools_ttl: Duration::from_secs(60),
            geckoterminal_pools_ttl: Duration::from_secs(300),
            rugcheck_info_ttl: Duration::from_secs(86400),
        }
    }
}
