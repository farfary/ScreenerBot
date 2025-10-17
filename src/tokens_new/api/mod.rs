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
        let dex_enabled = with_config(|cfg| cfg.tokens.sources.dexscreener.enabled);
        let gecko_enabled = with_config(|cfg| cfg.tokens.sources.geckoterminal.enabled);
        let rug_enabled = with_config(|cfg| cfg.tokens.sources.rugcheck.enabled);

        // Jupiter, CoinGecko, DeFiLlama, DexScreener, GeckoTerminal, Rugcheck
        // all have hardcoded timing params optimized per API
        let jup_enabled = true;
        let coingecko_enabled = true;
        let defillama_enabled = true;

        Ok(Self {
            dexscreener: DexScreenerClient::new(
                dexscreener::RATE_LIMIT_PER_MINUTE,
                dexscreener::TIMEOUT_SECS
            ),
            geckoterminal: GeckoTerminalClient::new(
                geckoterminal::RATE_LIMIT_PER_MINUTE,
                geckoterminal::TIMEOUT_SECS
            ),
            rugcheck: RugcheckClient::new(
                rug_enabled,
                rugcheck::RATE_LIMIT_PER_MINUTE,
                rugcheck::TIMEOUT_SECS
            )?,
            jupiter: JupiterClient::new(jup_enabled)?,
            coingecko: CoinGeckoClient::new(coingecko_enabled)?,
            defillama: DefiLlamaClient::new(defillama_enabled)?,
        })
    }
}
