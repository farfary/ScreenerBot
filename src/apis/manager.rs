/// Global API manager singleton - ensures single instance of all API clients across the bot
/// This provides centralized rate limiting and stats tracking per API
use std::sync::{Arc, LazyLock};

use crate::config::get_config_clone;
use crate::logger::{self, LogTag};

use super::coingecko::CoinGeckoClient;
use super::defillama::DefiLlamaClient;
use super::dexscreener::{
    DexScreenerClient, RATE_LIMIT_PER_MINUTE as DEX_RATE_LIMIT, TIMEOUT_SECS as DEX_TIMEOUT,
};
use super::geckoterminal::{
    GeckoTerminalClient, RATE_LIMIT_PER_MINUTE as GECKO_RATE_LIMIT, TIMEOUT_SECS as GECKO_TIMEOUT,
};
use super::jupiter::JupiterClient;
use super::rugcheck::{
    RugcheckClient, RATE_LIMIT_PER_MINUTE as RUG_RATE_LIMIT, TIMEOUT_SECS as RUG_TIMEOUT,
};
use super::stats::ApiStats;

/// Global API manager - holds all API clients with their individual rate limiters and stats
pub struct ApiManager {
    pub dexscreener: DexScreenerClient,
    pub geckoterminal: GeckoTerminalClient,
    pub rugcheck: RugcheckClient,
    pub jupiter: JupiterClient,
    pub coingecko: CoinGeckoClient,
    pub defillama: DefiLlamaClient,
}

impl ApiManager {
    fn new() -> Self {
        let cfg = get_config_clone();
        let sources_cfg = &cfg.tokens.sources;
        let discovery_cfg = &cfg.tokens.discovery;
        let discovery_enabled = discovery_cfg.enabled;

        let dexscreener_cfg = &sources_cfg.dexscreener;
        let geckoterminal_cfg = &sources_cfg.geckoterminal;

        let dexscreener_enabled =
            dexscreener_cfg.enabled && discovery_enabled && discovery_cfg.dexscreener.enabled;
        let geckoterminal_enabled =
            geckoterminal_cfg.enabled && discovery_enabled && discovery_cfg.geckoterminal.enabled;
        let rug_enabled =
            sources_cfg.rugcheck.enabled && discovery_enabled && discovery_cfg.rugcheck.enabled;

        let dex_rate_limit = if dexscreener_cfg.rate_limit_per_minute == 0 {
            DEX_RATE_LIMIT
        } else {
            dexscreener_cfg.rate_limit_per_minute as usize
        };
        let dex_timeout = if dexscreener_cfg.timeout_seconds == 0 {
            DEX_TIMEOUT
        } else {
            dexscreener_cfg.timeout_seconds
        };

        let gecko_rate_limit = if geckoterminal_cfg.rate_limit_per_minute == 0 {
            GECKO_RATE_LIMIT
        } else {
            geckoterminal_cfg.rate_limit_per_minute as usize
        };
        let gecko_timeout = if geckoterminal_cfg.timeout_seconds == 0 {
            GECKO_TIMEOUT
        } else {
            geckoterminal_cfg.timeout_seconds
        };

        let jup_enabled = discovery_enabled && discovery_cfg.jupiter.enabled;
        let coingecko_enabled = discovery_enabled
            && discovery_cfg.coingecko.enabled
            && discovery_cfg.coingecko.markets_enabled;
        let defillama_enabled = discovery_enabled
            && discovery_cfg.defillama.enabled
            && discovery_cfg.defillama.protocols_enabled;

        logger::info(LogTag::Api, "Initializing global API manager");

        Self {
            dexscreener: DexScreenerClient::new(dexscreener_enabled, dex_rate_limit, dex_timeout)
                .unwrap_or_else(|e| {
                    logger::warning(
                        LogTag::Api,
                        &format!(
                            "Failed to initialize DexScreener client: {} - using disabled client",
                            e
                        ),
                    );
                    DexScreenerClient::new(false, DEX_RATE_LIMIT, DEX_TIMEOUT)
                        .expect("Failed to create disabled DexScreener client")
                }),
            geckoterminal: GeckoTerminalClient::new(
                geckoterminal_enabled,
                gecko_rate_limit,
                gecko_timeout,
            )
            .unwrap_or_else(|e| {
                logger::warning(
                    LogTag::Api,
                    &format!(
                        "Failed to initialize GeckoTerminal client: {} - using disabled client",
                        e
                    ),
                );
                GeckoTerminalClient::new(false, GECKO_RATE_LIMIT, GECKO_TIMEOUT)
                    .expect("Failed to create disabled GeckoTerminal client")
            }),
            rugcheck: RugcheckClient::new(rug_enabled, RUG_RATE_LIMIT, RUG_TIMEOUT).unwrap_or_else(
                |e| {
                    logger::warning(
                        LogTag::Api,
                        &format!(
                            "Failed to initialize Rugcheck client: {} - using disabled client",
                            e
                        ),
                    );
                    RugcheckClient::new(false, RUG_RATE_LIMIT, RUG_TIMEOUT)
                        .expect("Failed to create disabled Rugcheck client")
                },
            ),
            jupiter: JupiterClient::new(jup_enabled).unwrap_or_else(|e| {
                logger::warning(
                    LogTag::Api,
                    &format!(
                        "Failed to initialize Jupiter client: {} - using disabled client",
                        e
                    ),
                );
                JupiterClient::new(false).expect("Failed to create disabled Jupiter client")
            }),
            coingecko: CoinGeckoClient::new(coingecko_enabled).unwrap_or_else(|e| {
                logger::warning(
                    LogTag::Api,
                    &format!(
                        "Failed to initialize CoinGecko client: {} - using disabled client",
                        e
                    ),
                );
                CoinGeckoClient::new(false).expect("Failed to create disabled CoinGecko client")
            }),
            defillama: DefiLlamaClient::new(defillama_enabled).unwrap_or_else(|e| {
                logger::warning(
                    LogTag::Api,
                    &format!(
                        "Failed to initialize DefiLlama client: {} - using disabled client",
                        e
                    ),
                );
                DefiLlamaClient::new(false).expect("Failed to create disabled DefiLlama client")
            }),
        }
    }

    /// Get aggregated stats from all API clients
    pub async fn get_all_stats(&self) -> ApiManagerStats {
        ApiManagerStats {
            dexscreener: self.dexscreener.get_stats().await,
            geckoterminal: self.geckoterminal.get_stats().await,
            rugcheck: self.rugcheck.get_stats().await,
            jupiter: self.jupiter.get_stats().await,
            coingecko: self.coingecko.get_stats().await,
            defillama: self.defillama.get_stats().await,
        }
    }
}

/// Aggregated stats from all API clients
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiManagerStats {
    pub dexscreener: ApiStats,
    pub geckoterminal: ApiStats,
    pub rugcheck: ApiStats,
    pub jupiter: ApiStats,
    pub coingecko: ApiStats,
    pub defillama: ApiStats,
}

/// Global singleton instance - lazy initialized on first access
/// This ensures only ONE instance of each API client exists across the entire bot,
/// providing true global rate limiting and centralized stats tracking
static GLOBAL_API_MANAGER: LazyLock<Arc<ApiManager>> =
    LazyLock::new(|| Arc::new(ApiManager::new()));

/// Get global API manager (creates singleton on first call, reuses on subsequent calls)
///
/// This is the ONLY way to access API clients in the bot - ensures proper rate limiting
/// and stats tracking across all usages
pub fn get_api_manager() -> Arc<ApiManager> {
    GLOBAL_API_MANAGER.clone()
}
