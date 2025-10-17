/// API clients for external token data sources
pub mod client;
pub mod coingecko;
pub mod coingecko_types;
pub mod defillama;
pub mod defillama_types;
pub mod dexscreener;
pub mod dexscreener_types;
pub mod geckoterminal;
pub mod geckoterminal_types;
pub mod jupiter;
pub mod jupiter_types;
pub mod rugcheck;
pub mod rugcheck_types;
pub mod stats;

pub use client::{HttpClient, RateLimiter};
pub use coingecko::CoinGeckoClient;
pub use defillama::DefiLlamaClient;
pub use dexscreener::DexScreenerClient;
pub use geckoterminal::GeckoTerminalClient;
pub use jupiter::JupiterClient;
pub use rugcheck::RugcheckClient;
pub use stats::{ApiStats, ApiStatsTracker};

use crate::config::with_config;

/// All API clients bundled together
pub struct ApiClients {
    pub dexscreener: DexScreenerClient,
    pub geckoterminal: GeckoTerminalClient,
    pub rugcheck: RugcheckClient,
    pub jupiter: JupiterClient,
    pub coingecko: CoinGeckoClient,
    pub defillama: DefiLlamaClient,
}

impl ApiClients {
    pub fn new() -> Result<Self, String> {
        let (dex_enabled, dex_rate_limit, dex_timeout) = with_config(|cfg| {
            (
                cfg.tokens.sources.dexscreener.enabled,
                cfg.tokens.sources.dexscreener.rate_limit_per_minute,
                cfg.tokens.sources.dexscreener.timeout_seconds,
            )
        });

        let (gecko_enabled, gecko_rate_limit, gecko_timeout) = with_config(|cfg| {
            (
                cfg.tokens.sources.geckoterminal.enabled,
                cfg.tokens.sources.geckoterminal.rate_limit_per_minute,
                cfg.tokens.sources.geckoterminal.timeout_seconds,
            )
        });

        let (rug_enabled, rug_rate_limit, rug_timeout) = with_config(|cfg| {
            (
                cfg.tokens.sources.rugcheck.enabled,
                cfg.tokens.sources.rugcheck.rate_limit_per_minute,
                cfg.tokens.sources.rugcheck.timeout_seconds,
            )
        });

        // Jupiter, CoinGecko, DeFiLlama have hardcoded timing params optimized per API
        // These APIs have their own timeout constants defined in their respective files
        let jup_enabled = true;
        let coingecko_enabled = true;
        let defillama_enabled = true;

        Ok(Self {
            dexscreener: DexScreenerClient::new(dex_rate_limit as usize, dex_timeout),
            geckoterminal: GeckoTerminalClient::new(gecko_rate_limit as usize, gecko_timeout),
            rugcheck: RugcheckClient::new(rug_enabled, rug_rate_limit as usize, rug_timeout)?,
            jupiter: JupiterClient::new(jup_enabled)?,
            coingecko: CoinGeckoClient::new(coingecko_enabled)?,
            defillama: DefiLlamaClient::new(defillama_enabled)?,
        })
    }
}
