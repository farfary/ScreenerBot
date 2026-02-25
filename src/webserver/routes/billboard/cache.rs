//! Billboard caching — cached fetchers for billboard, Jupiter, and DexScreener data.

use super::types::*;
use crate::apis::get_api_manager;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cached billboard data
struct BillboardCache {
    tokens: Vec<BillboardToken>,
    fetched_at: Instant,
}

/// Cached external tokens
struct ExternalTokensCache {
    tokens: Vec<ExternalToken>,
    fetched_at: Instant,
}

static BILLBOARD_CACHE: LazyLock<Arc<RwLock<Option<BillboardCache>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

static JUPITER_ORGANIC_CACHE: LazyLock<Arc<RwLock<Option<ExternalTokensCache>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

static JUPITER_TRADED_CACHE: LazyLock<Arc<RwLock<Option<ExternalTokensCache>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

static DEXSCREENER_TRENDING_CACHE: LazyLock<Arc<RwLock<Option<ExternalTokensCache>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes
const EXTERNAL_CACHE_TTL: Duration = Duration::from_secs(120); // 2 minutes for external APIs
const WEBSITE_BILLBOARD_URL: &str = "https://screenerbot.io/api/billboard";

/// Fetch billboard tokens from website
async fn fetch_from_website() -> Result<Vec<BillboardToken>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(WEBSITE_BILLBOARD_URL)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch billboard: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Billboard API returned status: {}",
            response.status()
        ));
    }

    let wrapper: WebsiteBillboardResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse billboard: {e}"))?;

    if !wrapper.success {
        return Err("Billboard API returned success=false".to_owned());
    }

    Ok(wrapper.tokens)
}

/// Fetch Jupiter top organic score tokens
async fn fetch_jupiter_organic() -> Result<Vec<ExternalToken>, String> {
    let api = get_api_manager();

    if !api.jupiter.is_enabled() {
        return Ok(vec![]);
    }

    let tokens = api
        .jupiter
        .fetch_top_organic_score("24h", Some(20))
        .await
        .map_err(|e| format!("Jupiter organic fetch failed: {:?}", e))?;

    Ok(tokens
        .into_iter()
        .map(|t| ExternalToken {
            mint: t.id,
            name: t.name,
            symbol: t.symbol,
            logo: t.icon,
            website: None,
            twitter: None,
            telegram: None,
            discord: None,
            price_usd: t.usd_price,
            volume_24h: t.stats24h.as_ref().and_then(|s| {
                let buy = s.buy_volume.unwrap_or_default();
                let sell = s.sell_volume.unwrap_or_default();
                Some(buy + sell)
            }),
            liquidity: t.liquidity,
            organic_score: t.organic_score,
        })
        .collect())
}

/// Fetch Jupiter top traded tokens
async fn fetch_jupiter_traded() -> Result<Vec<ExternalToken>, String> {
    let api = get_api_manager();

    if !api.jupiter.is_enabled() {
        return Ok(vec![]);
    }

    let tokens = api
        .jupiter
        .fetch_top_traded("24h", Some(20))
        .await
        .map_err(|e| format!("Jupiter traded fetch failed: {:?}", e))?;

    Ok(tokens
        .into_iter()
        .map(|t| ExternalToken {
            mint: t.id,
            name: t.name,
            symbol: t.symbol,
            logo: t.icon,
            website: None,
            twitter: None,
            telegram: None,
            discord: None,
            price_usd: t.usd_price,
            volume_24h: t.stats24h.as_ref().and_then(|s| {
                let buy = s.buy_volume.unwrap_or_default();
                let sell = s.sell_volume.unwrap_or_default();
                Some(buy + sell)
            }),
            liquidity: t.liquidity,
            organic_score: t.organic_score,
        })
        .collect())
}

/// Fetch DexScreener trending (top boosted) tokens
async fn fetch_dexscreener_trending() -> Result<Vec<ExternalToken>, String> {
    let api = get_api_manager();

    if !api.dexscreener.is_enabled() {
        return Ok(vec![]);
    }

    let tokens = api
        .dexscreener
        .get_top_boosted_tokens(Some("solana"))
        .await
        .map_err(|e| format!("DexScreener trending fetch failed: {e}"))?;

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
                mint: t.token_address,
                name: t
                    .description
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_owned()),
                symbol: t
                    .description
                    .as_ref()
                    .and_then(|d| d.split_whitespace().next())
                    .unwrap_or("???")
                    .to_string(),
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

/// Get billboard tokens (with caching)
pub(super) async fn get_billboard() -> Result<Vec<BillboardToken>, String> {
    // Check cache
    {
        let cache = BILLBOARD_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < CACHE_TTL {
                return Ok(cached.tokens.clone());
            }
        }
    }

    // Fetch fresh data
    let tokens = fetch_from_website().await?;

    // Update cache
    {
        let mut cache = BILLBOARD_CACHE.write().await;
        *cache = Some(BillboardCache {
            tokens: tokens.clone(),
            fetched_at: Instant::now(),
        });
    }

    Ok(tokens)
}

/// Get Jupiter organic tokens (with caching)
pub(super) async fn get_jupiter_organic() -> Vec<ExternalToken> {
    // Check cache
    {
        let cache = JUPITER_ORGANIC_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < EXTERNAL_CACHE_TTL {
                return cached.tokens.clone();
            }
        }
    }

    // Fetch fresh data
    let tokens = fetch_jupiter_organic().await.unwrap_or_default();

    // Update cache
    {
        let mut cache = JUPITER_ORGANIC_CACHE.write().await;
        *cache = Some(ExternalTokensCache {
            tokens: tokens.clone(),
            fetched_at: Instant::now(),
        });
    }

    tokens
}

/// Get Jupiter traded tokens (with caching)
pub(super) async fn get_jupiter_traded() -> Vec<ExternalToken> {
    // Check cache
    {
        let cache = JUPITER_TRADED_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < EXTERNAL_CACHE_TTL {
                return cached.tokens.clone();
            }
        }
    }

    // Fetch fresh data
    let tokens = fetch_jupiter_traded().await.unwrap_or_default();

    // Update cache
    {
        let mut cache = JUPITER_TRADED_CACHE.write().await;
        *cache = Some(ExternalTokensCache {
            tokens: tokens.clone(),
            fetched_at: Instant::now(),
        });
    }

    tokens
}

/// Get DexScreener trending tokens (with caching)
pub(super) async fn get_dexscreener_trending() -> Vec<ExternalToken> {
    // Check cache
    {
        let cache = DEXSCREENER_TRENDING_CACHE.read().await;
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < EXTERNAL_CACHE_TTL {
                return cached.tokens.clone();
            }
        }
    }

    // Fetch fresh data
    let tokens = fetch_dexscreener_trending().await.unwrap_or_default();

    // Update cache
    {
        let mut cache = DEXSCREENER_TRENDING_CACHE.write().await;
        *cache = Some(ExternalTokensCache {
            tokens: tokens.clone(),
            fetched_at: Instant::now(),
        });
    }

    tokens
}
