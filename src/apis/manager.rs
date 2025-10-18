/// Global API manager singleton - ensures single instance of all API clients across the bot
/// This provides centralized rate limiting and stats tracking per API
use std::sync::{Arc, LazyLock};

use crate::config::get_config_clone;
use crate::logger::{log, LogTag};

use super::coingecko::CoinGeckoClient;
use super::defillama::DefiLlamaClient;
use super::dexscreener::{DexScreenerClient, RATE_LIMIT_PER_MINUTE as DEX_RATE_LIMIT, TIMEOUT_SECS as DEX_TIMEOUT};
use super::geckoterminal::{GeckoTerminalClient, RATE_LIMIT_PER_MINUTE as GECKO_RATE_LIMIT, TIMEOUT_SECS as GECKO_TIMEOUT};
use super::jupiter::JupiterClient;
use super::rugcheck::{RugcheckClient, RATE_LIMIT_PER_MINUTE as RUG_RATE_LIMIT, TIMEOUT_SECS as RUG_TIMEOUT};
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

        let rug_enabled =
            sources_cfg.rugcheck.enabled && discovery_enabled && discovery_cfg.rugcheck.enabled;

        let jup_enabled = discovery_enabled && discovery_cfg.jupiter.enabled;
        let coingecko_enabled = discovery_enabled
            && discovery_cfg.coingecko.enabled
            && discovery_cfg.coingecko.markets_enabled;
        let defillama_enabled = discovery_enabled
            && discovery_cfg.defillama.enabled
            && discovery_cfg.defillama.protocols_enabled;

        log(LogTag::Api, "INIT", "Initializing global API manager");

        Self {
            dexscreener: DexScreenerClient::new(DEX_RATE_LIMIT, DEX_TIMEOUT),
            geckoterminal: GeckoTerminalClient::new(GECKO_RATE_LIMIT, GECKO_TIMEOUT),
            rugcheck: RugcheckClient::new(rug_enabled, RUG_RATE_LIMIT, RUG_TIMEOUT)
                .unwrap_or_else(|e| {
                    log(
                        LogTag::Api,
                        "WARN",
                        &format!("Failed to initialize Rugcheck client: {} - using disabled client", e),
                    );
                    RugcheckClient::new(false, RUG_RATE_LIMIT, RUG_TIMEOUT)
                        .expect("Failed to create disabled Rugcheck client")
                }),
            jupiter: JupiterClient::new(jup_enabled).unwrap_or_else(|e| {
                log(
                    LogTag::Api,
                    "WARN",
                    &format!("Failed to initialize Jupiter client: {} - using disabled client", e),
                );
                JupiterClient::new(false).expect("Failed to create disabled Jupiter client")
            }),
            coingecko: CoinGeckoClient::new(coingecko_enabled).unwrap_or_else(|e| {
                log(
                    LogTag::Api,
                    "WARN",
                    &format!("Failed to initialize CoinGecko client: {} - using disabled client", e),
                );
                CoinGeckoClient::new(false).expect("Failed to create disabled CoinGecko client")
            }),
            defillama: DefiLlamaClient::new(defillama_enabled).unwrap_or_else(|e| {
                log(
                    LogTag::Api,
                    "WARN",
                    &format!("Failed to initialize DefiLlama client: {} - using disabled client", e),
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
static GLOBAL_API_MANAGER: LazyLock<Arc<ApiManager>> = LazyLock::new(|| {
    Arc::new(ApiManager::new())
});

/// Get global API manager (creates singleton on first call, reuses on subsequent calls)
/// 
/// This is the ONLY way to access API clients in the bot - ensures proper rate limiting
/// and stats tracking across all usages
pub fn get_api_manager() -> Arc<ApiManager> {
    GLOBAL_API_MANAGER.clone()
}
