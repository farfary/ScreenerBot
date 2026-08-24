//! API fetching functions - retrieve pool data from external sources

use crate::apis::manager::get_api_manager;
use crate::events::{record_token_event, Severity};
use crate::logger::{self, LogTag};
use crate::sol_price::get_sol_price;
use crate::tokens::types::{TokenError, TokenPoolInfo, TokenResult};
use crate::tokens::updates::RateLimitCoordinator;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use super::conversion;
use super::operations::ingest_pool_entry;
use serde_json::json;

/// Timeout for acquiring rate limit permits (prevents indefinite blocking)
const RATE_LIMIT_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// Fetch pools from all enabled sources (DexScreener + GeckoTerminal)
/// Uses timeouts on rate limit acquisition to prevent indefinite blocking
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
                // Use timeout to prevent indefinite blocking on rate limit
                match tokio::time::timeout(
                    RATE_LIMIT_ACQUIRE_TIMEOUT,
                    coordinator.acquire_dexscreener_pools(),
                )
                .await
                {
                    Ok(Ok(permit)) => {
                        // Got permit, proceed with fetch
                        let result = api
                            .dexscreener
                            .fetch_token_pools(&mint, None)
                            .await
                            .map_err(|e| TokenError::Api {
                                source: "DexScreener".to_owned(),
                                message: e.to_string(),
                            });
                        // Only forget permit if API call succeeded
                        if result.is_ok() {
                            permit.forget();
                        }
                        result
                    }
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(TokenError::RateLimit {
                        source: "DexScreener".to_owned(),
                        message: "Rate limit acquisition timed out".to_owned(),
                    }),
                }
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
                // Use timeout to prevent indefinite blocking on rate limit
                match tokio::time::timeout(
                    RATE_LIMIT_ACQUIRE_TIMEOUT,
                    coordinator.acquire_geckoterminal(),
                )
                .await
                {
                    Ok(Ok(permit)) => {
                        // Got permit, proceed with fetch
                        let result = api.geckoterminal.fetch_pools(&mint).await.map_err(|e| {
                            TokenError::Api {
                                source: "GeckoTerminal".to_owned(),
                                message: e.to_string(),
                            }
                        });
                        // Only forget permit if API call succeeded
                        if result.is_ok() {
                            permit.forget();
                        }
                        result
                    }
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(TokenError::RateLimit {
                        source: "GeckoTerminal".to_owned(),
                        message: "Rate limit acquisition timed out".to_owned(),
                    }),
                }
            } else {
                Ok(Vec::new())
            }
        }
    };

    // The self-hosted server is the PRIMARY, central pool registry: fetch it
    // alongside the direct providers and ingest it FIRST so its pools seed the
    // snapshot (DexScreener/GeckoTerminal then enrich price/volume by address).
    // Even if both direct providers fail, the server pools keep the snapshot
    // non-empty, so every consumer (OHLCV, pool service, dashboard) still gets a
    // usable pool set.
    let server_future = super::server::fetch_pools_from_server(&mint_owned);

    // Fetch the proxy-backed server (no direct rate limit) + DexScreener first.
    // Direct GeckoTerminal is a LAST-RESORT fallback below — it used to be fetched
    // in PARALLEL here for every token, which hammered the direct GeckoTerminal
    // API into constant 429s (marking the endpoint unhealthy and starving the
    // OHLCV fetcher's own Gecko fallback), even though the server + DexScreener
    // already cover ~every token's pools.
    let (server_pools, dex_result) = tokio::join!(server_future, dex_future);

    let mut pools_map: HashMap<String, TokenPoolInfo> = HashMap::new();
    let mut success_sources = 0usize;
    let mut failures: Vec<String> = Vec::new();

    let server_ok = match server_pools {
        Some(pools) if !pools.is_empty() => {
            for info in pools {
                ingest_pool_entry(&mut pools_map, info);
            }
            true
        }
        _ => false,
    };

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
            record_token_event(
                mint,
                "pool_source_fetch_failed",
                Severity::Warn,
                json!({
                    "source": "dexscreener",
                    "error": message.clone(),
                }),
            )
            .await;
            failures.push(format!("DexScreener→{message}"));
        }
    }

    // Direct GeckoTerminal only as a last resort: fetch it ONLY when the server
    // and DexScreener both yielded zero pools. This keeps direct Gecko usage rare
    // so its shared rate limit stays free for the OHLCV fetcher's Gecko fallback
    // instead of being burned (and 429'd) on redundant pool lookups.
    let gecko_attempted = should_fetch_gecko && pools_map.is_empty();
    let gecko_result = if gecko_attempted {
        gecko_future.await
    } else {
        Ok(Vec::new())
    };

    match gecko_result {
        Ok(pools) => {
            if gecko_attempted {
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
            record_token_event(
                mint,
                "pool_source_fetch_failed",
                Severity::Warn,
                json!({
                    "source": "geckoterminal",
                    "error": message.clone(),
                }),
            )
            .await;
            failures.push(format!("GeckoTerminal→{message}"));
        }
    }

    // The server counts as a successful source: when it supplied pools, the
    // snapshot is usable even if both direct providers failed.
    if server_ok {
        success_sources += 1;
    }

    let attempted_sources = (should_fetch_dex as usize) + (gecko_attempted as usize);
    if attempted_sources > 0 && success_sources == 0 {
        let combined = if failures.is_empty() {
            "all pool sources failed without details".to_owned()
        } else {
            failures.join(" | ")
        };
        record_token_event(
            mint,
            "pool_sources_failed",
            Severity::Error,
            json!({
                "failures": failures,
                "combined_error": combined.clone(),
            }),
        )
        .await;
        return Err(TokenError::Api {
            source: "TokenPools".to_owned(),
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
        record_token_event(
            mint,
            "pool_sources_unconfigured",
            Severity::Warn,
            json!({
                "dexscreener_enabled": should_fetch_dex,
                "geckoterminal_enabled": should_fetch_gecko,
            }),
        )
        .await;
    }

    Ok((pools_map, success_sources))
}
