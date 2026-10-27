// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! API fetching functions - retrieve pool data from external sources.

use crate::apis::geckoterminal::RequestLane;
use crate::apis::manager::get_api_manager;
use crate::chains::{adapter_for, ChainId};
use crate::events::{record_token_event, Severity};
use crate::logger::{self, LogTag};
use crate::native_price::get_native_price_for;
use crate::tokens::types::{TokenPoolInfo, TokenResult};
use crate::tokens::updates::RateLimitCoordinator;
use crate::tokens::Error;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use super::conversion;
use super::operations::{
    dexscreener_list_is_incomplete, ingest_pool_entry, reliable_completion_pools,
};
use serde_json::json;

/// Timeout for acquiring rate limit permits (prevents indefinite blocking)
const RATE_LIMIT_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// The merged answer of every pool source for one mint.
pub struct FetchedPools {
    /// Pools by address, server pools first, enriched by the direct providers.
    pub pools: HashMap<String, TokenPoolInfo>,
    /// How many sources answered.
    pub success_sources: usize,
    /// The data server's canonical (series) pool, when the server answered.
    pub series_pool: Option<String>,
}

/// Fetch pools from all enabled sources (DexScreener + GeckoTerminal)
/// Uses timeouts on rate limit acquisition to prevent indefinite blocking
pub async fn fetch_from_sources(
    chain: ChainId,
    mint: &str,
    coordinator: Arc<RateLimitCoordinator>,
) -> TokenResult<FetchedPools> {
    let api = get_api_manager();
    let native_price = get_native_price_for(chain);

    let should_fetch_dex = api.dexscreener.is_enabled();
    let should_fetch_gecko = api.geckoterminal.is_enabled();

    let mint_owned = mint.to_string();
    let network = adapter_for(chain).market_data_network();

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
                            .fetch_token_pools(&mint, Some(network))
                            .await
                            .map_err(|e| Error::Api {
                                provider: "DexScreener".to_owned(),
                                message: e.to_string(),
                            });
                        // Only forget permit if API call succeeded
                        if result.is_ok() {
                            permit.forget();
                        }
                        result
                    }
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(Error::RateLimit {
                        provider: "DexScreener".to_owned(),
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
                        // Got permit, proceed with fetch. `Ok(None)`: the client
                        // deferred the request during its rate-limit penalty.
                        let result = match api
                            .geckoterminal
                            .fetch_pools_on_network(&mint, Some(network), RequestLane::Background)
                            .await
                        {
                            Ok(pools) => Ok(Some(pools)),
                            Err(crate::apis::Error::Deferred { .. }) => Ok(None),
                            Err(e) => Err(Error::Api {
                                provider: "GeckoTerminal".to_owned(),
                                message: e.to_string(),
                            }),
                        };
                        // Only forget permit if the API answered
                        if matches!(result, Ok(Some(_))) {
                            permit.forget();
                        }
                        result
                    }
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(Error::RateLimit {
                        provider: "GeckoTerminal".to_owned(),
                        message: "Rate limit acquisition timed out".to_owned(),
                    }),
                }
            } else {
                Ok(Some(Vec::new()))
            }
        }
    };

    // The self-hosted server is the PRIMARY, central pool registry: fetch it
    // alongside the direct providers and ingest it FIRST so its pools seed the
    // snapshot (DexScreener/GeckoTerminal then enrich price/volume by address).
    // Even if both direct providers fail, the server pools keep the snapshot
    // non-empty, so every consumer (OHLCV, pool service, dashboard) still gets a
    // usable pool set.
    let server_future = super::server::fetch_pools_from_server(chain, &mint_owned);

    // Fetch the proxy-backed server (no direct rate limit) + DexScreener first.
    // Direct GeckoTerminal is a fallback below, never a parallel source: fetched
    // for every token it hammered the direct GeckoTerminal API into constant 429s
    // (marking the endpoint unhealthy and starving the OHLCV fetcher's own Gecko
    // fallback), even though the server + DexScreener cover ~every token's pools.
    let (server_pools, dex_result) = tokio::join!(server_future, dex_future);

    let mut pools_map: HashMap<String, TokenPoolInfo> = HashMap::new();
    let mut success_sources = 0usize;
    let mut failures: Vec<String> = Vec::new();

    let mut series_pool = None;
    let server_ok = match server_pools {
        Some(server) if !server.pools.is_empty() => {
            for info in server.pools {
                ingest_pool_entry(&mut pools_map, info);
            }
            series_pool = server.series_pool;
            true
        }
        _ => false,
    };

    let mut dex_incomplete = false;
    match dex_result {
        Ok(pools) => {
            if should_fetch_dex {
                success_sources += 1;
            }
            let listed: Vec<TokenPoolInfo> = pools
                .iter()
                .filter_map(|pool| conversion::from_dexscreener(chain, mint, pool))
                .collect();
            dex_incomplete =
                should_fetch_dex && dexscreener_list_is_incomplete(&listed, pools.len());
            for info in listed {
                ingest_pool_entry(&mut pools_map, info);
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

    // Direct GeckoTerminal stays rare, at most one call per resolution, so its
    // shared rate limit stays free for the OHLCV fetcher's Gecko fallback instead
    // of being burned (and 429'd) on redundant pool lookups. It is fetched only
    // when the server and DexScreener yielded no pool (its whole list is taken),
    // or when the server did not answer and DexScreener's list holds no native
    // pair or was cut at its `token-pairs` cap (only its native- and
    // stable-countered pools DexScreener did not list are merged).
    let gecko_fills_empty = pools_map.is_empty();
    let gecko_attempted =
        should_fetch_gecko && (gecko_fills_empty || (!server_ok && dex_incomplete));
    let gecko_result = if gecko_attempted {
        gecko_future.await
    } else {
        Ok(Some(Vec::new()))
    };

    match gecko_result {
        Ok(Some(pools)) => {
            if gecko_attempted {
                success_sources += 1;
            }
            let fetched: Vec<TokenPoolInfo> = pools
                .iter()
                .filter_map(|pool| conversion::from_geckoterminal(chain, mint, pool, native_price))
                .collect();
            let merged = if gecko_fills_empty {
                fetched
            } else {
                reliable_completion_pools(chain, mint, &pools_map, fetched)
            };
            for info in merged {
                ingest_pool_entry(&mut pools_map, info);
            }
        }
        // No answer this round: counted as a failed source, retried on the next refresh.
        Ok(None) => {
            logger::debug(
                LogTag::Tokens,
                &format!(
                    "[TOKEN_POOLS] GeckoTerminal fetch deferred by its rate-limit penalty for mint={}",
                    mint
                ),
            );
            failures.push("GeckoTerminal→deferred by its rate-limit penalty".to_owned());
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
        return Err(Error::Api {
            provider: "TokenPools".to_owned(),
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

    Ok(FetchedPools {
        pools: pools_map,
        success_sources,
        series_pool,
    })
}
