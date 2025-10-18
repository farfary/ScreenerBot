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

use crate::config::get_config_clone;

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
        let cfg = get_config_clone();
        let sources_cfg = &cfg.tokens.sources;
        let discovery_cfg = &cfg.tokens.discovery;
        let discovery_enabled = discovery_cfg.enabled;

    let _dex_enabled = sources_cfg.dexscreener.enabled;
    let _gecko_enabled = sources_cfg.geckoterminal.enabled;
        let rug_enabled = sources_cfg.rugcheck.enabled && discovery_enabled && discovery_cfg.rugcheck.enabled;

        let jup_enabled = discovery_enabled && discovery_cfg.jupiter.enabled;
        let coingecko_enabled = discovery_enabled && discovery_cfg.coingecko.enabled && discovery_cfg.coingecko.markets_enabled;
        let defillama_enabled = discovery_enabled && discovery_cfg.defillama.enabled && discovery_cfg.defillama.protocols_enabled;

        Ok(Self {
            dexscreener: DexScreenerClient::new(
                dexscreener::RATE_LIMIT_PER_MINUTE,
                dexscreener::TIMEOUT_SECS,
            ),
            geckoterminal: GeckoTerminalClient::new(
                geckoterminal::RATE_LIMIT_PER_MINUTE,
                geckoterminal::TIMEOUT_SECS,
            ),
            rugcheck: RugcheckClient::new(
                rug_enabled,
                rugcheck::RATE_LIMIT_PER_MINUTE,
                rugcheck::TIMEOUT_SECS,
            )?,
            jupiter: JupiterClient::new(jup_enabled)?,
            coingecko: CoinGeckoClient::new(coingecko_enabled)?,
            defillama: DefiLlamaClient::new(defillama_enabled)?,
        })
    }
}
