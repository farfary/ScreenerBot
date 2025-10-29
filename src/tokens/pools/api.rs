/// API fetching functions - retrieve pool data from external sources
use crate::apis::dexscreener::types::DexScreenerPool;
use crate::apis::geckoterminal::types::GeckoTerminalPool;
use crate::apis::manager::get_api_manager;
use crate::logger::{self, LogTag};
use crate::sol_price::get_sol_price;
use crate::tokens::service::get_rate_coordinator;
use crate::tokens::types::{TokenError, TokenPoolInfo, TokenResult};
use crate::tokens::updates::RateLimitCoordinator;
use std::collections::HashMap;
use std::sync::Arc;

use super::conversion;
use super::operations::ingest_pool_entry;

/// Fetch pools from all enabled sources (DexScreener + GeckoTerminal)
pub async fn fetch_from_sources(
    mint: &str,
    coordinator: Arc<RateLimitCoordinator>,
) -> TokenResult<(HashMap<String, TokenPoolInfo>, usize)> {
    let api = get_api_manager();
    let sol_price = get_sol_price();

    let should_fetch_dex = api.dexscreener.is_enabled();
    let should_fetch_gecko = api.geckoterminal.is_enabled();

    let mint_owned = mint.to_string();

    let dex_future = {
        let api = api.clone();
        let coordinator = coordinator.clone();
        let mint = mint_owned.clone();
        async move {
            if should_fetch_dex {
                coordinator.acquire_dexscreener_pools().await?;
                api.dexscreener
                    .fetch_token_pools(&mint, None)
                    .await
                    .map_err(|e| TokenError::Api {
                        source: "DexScreener".to_string(),
                        message: e,
                    })
            } else {
                Ok(Vec::new())
            }
        }
    };

    let gecko_future = {
        let api = api.clone();
        let coordinator = coordinator.clone();
        let mint = mint_owned.clone();
        async move {
            if should_fetch_gecko {
                coordinator.acquire_geckoterminal().await?;
                api.geckoterminal
                    .fetch_pools(&mint)
                    .await
                    .map_err(|e| TokenError::Api {
                        source: "GeckoTerminal".to_string(),
                        message: e,
                    })
            } else {
                Ok(Vec::new())
            }
        }
    };

    let (dex_result, gecko_result) = tokio::join!(dex_future, gecko_future);

    let mut pools_map: HashMap<String, TokenPoolInfo> = HashMap::new();
    let mut success_sources = 0usize;
    let mut failures: Vec<String> = Vec::new();

    match dex_result {
        Ok(pools) => {
            if should_fetch_dex {
                success_sources += 1;
            }
            for pool in pools.iter() {
                if let Some(info) = conversion::from_dexscreener(pool) {
                    ingest_pool_entry(&mut pools_map, info);
                }
            }
        }
        Err(err) => {
            let message = err.to_string();
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "[TOKEN_POOLS] DexScreener fetch failed for mint={}: {}",
                    mint, message
                ),
            );
            failures.push(format!("DexScreener→{}", message));
        }
    }

    match gecko_result {
        Ok(pools) => {
            if should_fetch_gecko {
                success_sources += 1;
            }
            for pool in pools.iter() {
                if let Some(info) = conversion::from_geckoterminal(pool, sol_price) {
                    ingest_pool_entry(&mut pools_map, info);
                }
            }
        }
        Err(err) => {
            let message = err.to_string();
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "[TOKEN_POOLS] GeckoTerminal fetch failed for mint={}: {}",
                    mint, message
                ),
            );
            failures.push(format!("GeckoTerminal→{}", message));
        }
    }

    let attempted_sources = (should_fetch_dex as usize) + (should_fetch_gecko as usize);
    if attempted_sources > 0 && success_sources == 0 {
        let combined = if failures.is_empty() {
            "all pool sources failed without details".to_string()
        } else {
            failures.join(" | ")
        };
        return Err(TokenError::Api {
            source: "TokenPools".to_string(),
            message: combined,
        });
    }

    if attempted_sources == 0 {
        logger::warning(
            LogTag::Tokens,
            &format!(
                "[TOKEN_POOLS] No pool sources enabled for mint={} – returning empty snapshot",
                mint
            ),
        );
    }

    Ok((pools_map, success_sources))
}
