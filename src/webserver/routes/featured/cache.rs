// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Discovery-source caching — cached fetchers for the Jupiter and DexScreener
//! boards the featured surfaces show alongside our own boosted tokens.
//!
//! Our boosted tokens are NOT cached here: they come from
//! `routes::boosts::active_boosts`, which owns that feed for the whole app.

use super::types::ExternalToken;
use crate::apis::get_api_manager;
use crate::webserver::{Error, Result};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// The Jupiter featured boards, registered by the composition root;
/// The featured surfaces are re-pointed at chain-aware sources later.
type JupiterBoardFn = fn() -> Pin<Box<dyn Future<Output = Result<Vec<ExternalToken>>> + Send>>;

static JUPITER_BOARDS: std::sync::OnceLock<JupiterBoards> = std::sync::OnceLock::new();

struct JupiterBoards {
    organic: JupiterBoardFn,
    traded: JupiterBoardFn,
}

/// Install the chain-owned Jupiter boards (composition root only).
pub fn install_jupiter_boards(organic: JupiterBoardFn, traded: JupiterBoardFn) {
    let _ = JUPITER_BOARDS.set(JupiterBoards { organic, traded });
}

/// Cached external tokens
struct ExternalTokensCache {
    tokens: Vec<ExternalToken>,
    fetched_at: Instant,
}

static JUPITER_ORGANIC_CACHE: LazyLock<Arc<RwLock<Option<ExternalTokensCache>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

static JUPITER_TRADED_CACHE: LazyLock<Arc<RwLock<Option<ExternalTokensCache>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

static DEXSCREENER_TRENDING_CACHE: LazyLock<Arc<RwLock<Option<ExternalTokensCache>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

const EXTERNAL_CACHE_TTL: Duration = Duration::from_secs(120);
/// Discovery boards are optional UI content. A shared provider client's
/// rate-limit queue or retry backoff must never hold `/featured/all` open.
const EXTERNAL_FETCH_TIMEOUT: Duration = Duration::from_secs(3);

/// Fetch Jupiter top organic score tokens
async fn fetch_jupiter_organic() -> Result<Vec<ExternalToken>> {
    let Some(boards) = JUPITER_BOARDS.get() else {
        return Ok(vec![]); // unset reads as "board disabled", today's empty path
    };
    (boards.organic)().await
}

/// Fetch Jupiter top traded tokens
async fn fetch_jupiter_traded() -> Result<Vec<ExternalToken>> {
    let Some(boards) = JUPITER_BOARDS.get() else {
        return Ok(vec![]); // unset reads as "board disabled", today's empty path
    };
    (boards.traded)().await
}

/// Fetch DexScreener trending (top boosted) tokens.
///
/// This is DexScreener's OWN boost product and has nothing to do with a
/// ScreenerBot boost — it is a third-party discovery board, so its rows stay
/// `boosts: 0` and never earn our gold treatment.
async fn fetch_dexscreener_trending() -> Result<Vec<ExternalToken>> {
    let api = get_api_manager();

    if !api.dexscreener.is_enabled() {
        return Ok(vec![]);
    }

    let tokens = api
        .dexscreener
        .get_top_boosted_tokens(Some(crate::chains::adapter().market_data_network()))
        .await
        .map_err(|source| Error::Api {
            operation: "DexScreener trending fetch",
            source,
        })?;

    Ok(tokens
        .into_iter()
        .take(20)
        .map(|t| {
            // Parse links for social info
            let mut website = None;
            let mut twitter = None;
            let mut telegram = None;
            let mut discord = None;

            if let Some(links) = &t.links {
                for link in links {
                    if let Some(obj) = link.as_object() {
                        let link_type = obj.get("type").and_then(|v| v.as_str());
                        let link_url = obj
                            .get("url")
                            .or_else(|| obj.get("label"))
                            .and_then(|v| v.as_str());

                        if let (Some(lt), Some(url)) = (link_type, link_url) {
                            match lt {
                                "website" => website = Some(url.to_string()),
                                "twitter" => twitter = Some(url.to_string()),
                                "telegram" => telegram = Some(url.to_string()),
                                "discord" => discord = Some(url.to_string()),
                                _ => {}
                            }
                        }
                    }
                }
            }

            ExternalToken {
                // DexScreener's boost board ships a free-text `description`, not a
                // name/symbol. Leaving it empty lets the shared identity pass fill
                // the real ones instead of printing a sentence where a ticker goes.
                mint: t.token_address,
                name: String::new(),
                symbol: String::new(),
                logo: t.icon,
                website,
                twitter,
                telegram,
                discord,
                price_usd: None,
                volume_24h: None,
                liquidity: None,
                organic_score: None,
            }
        })
        .collect())
}

/// Read a cached board, refreshing it when the TTL has passed.
async fn cached_board<F, Fut>(
    slot: &LazyLock<Arc<RwLock<Option<ExternalTokensCache>>>>,
    fetch: F,
) -> Vec<ExternalToken>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Vec<ExternalToken>>>,
{
    let stale = {
        let cache = slot.read().await;
        if let Some(cached) = cache.as_ref() {
            if cached.fetched_at.elapsed() < EXTERNAL_CACHE_TTL {
                return cached.tokens.clone();
            }
            cached.tokens.clone()
        } else {
            Vec::new()
        }
    };

    let tokens = match tokio::time::timeout(EXTERNAL_FETCH_TIMEOUT, fetch()).await {
        Ok(Ok(tokens)) => tokens,
        Ok(Err(_)) | Err(_) => stale,
    };

    {
        let mut cache = slot.write().await;
        *cache = Some(ExternalTokensCache {
            tokens: tokens.clone(),
            fetched_at: Instant::now(),
        });
    }

    tokens
}

/// Get Jupiter organic tokens (with caching)
pub(super) async fn get_jupiter_organic() -> Vec<ExternalToken> {
    cached_board(&JUPITER_ORGANIC_CACHE, fetch_jupiter_organic).await
}

/// Get Jupiter traded tokens (with caching)
pub(super) async fn get_jupiter_traded() -> Vec<ExternalToken> {
    cached_board(&JUPITER_TRADED_CACHE, fetch_jupiter_traded).await
}

/// Get DexScreener trending tokens (with caching)
pub(super) async fn get_dexscreener_trending() -> Vec<ExternalToken> {
    cached_board(&DEXSCREENER_TRENDING_CACHE, fetch_dexscreener_trending).await
}
