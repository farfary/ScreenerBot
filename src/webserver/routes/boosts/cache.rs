//! Boost feed cache — one stale-while-revalidate read of the website's boost feed.

use super::types::{rank_boosts, retain_active, BoostStanding, WebsiteBoostResponse};
use crate::connectivity;
use crate::logger::{self, LogTag};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// The website's public boost feed. Boosts are bought there; the app only reads.
const BOOST_FEED_URL: &str = "https://screenerbot.io/api/boost";

/// The website serves the feed with a 30s cache; a minute here keeps the desktop
/// app well inside that without ever making a paid boost take long to appear.
const CACHE_TTL: Duration = Duration::from_secs(60);

const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

struct BoostCache {
    standings: Vec<BoostStanding>,
    fetched_at: Instant,
}

static BOOST_CACHE: LazyLock<Arc<RwLock<Option<BoostCache>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

/// Fetch, validate and rank the website's boost feed.
async fn fetch_feed() -> Result<Vec<BoostStanding>, String> {
    if connectivity::is_network_offline() {
        return Err("network offline".to_owned());
    }

    let response = crate::net::client()
        .get(BOOST_FEED_URL)
        .timeout(FETCH_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("boost feed request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("boost feed returned status {}", response.status()));
    }

    let payload: WebsiteBoostResponse = response
        .json()
        .await
        .map_err(|e| format!("boost feed parse failed: {e}"))?;

    if !payload.success {
        return Err("boost feed returned success=false".to_owned());
    }

    let mut standings = payload.tokens;
    // Rank BEFORE deduping so a duplicated mint collapses onto its strongest row.
    rank_boosts(&mut standings);
    retain_active(&mut standings);
    Ok(standings)
}

/// Fetch and store. Returns the fresh standings.
async fn refresh() -> Result<Vec<BoostStanding>, String> {
    let standings = fetch_feed().await?;
    {
        let mut cache = BOOST_CACHE.write().await;
        *cache = Some(BoostCache {
            standings: standings.clone(),
            fetched_at: Instant::now(),
        });
    }
    Ok(standings)
}

/// Every mint with an active boost, ranked (Golden first, then most-boosted).
///
/// Stale-while-revalidate: a warm cache answers instantly and refreshes in the
/// background, so no dashboard request ever waits on the remote fetch. A cold
/// cache is the only blocking path, and `prewarm()` runs it at startup.
///
/// A failing feed is NOT an error the user should see — an offline app, or a
/// website hiccup, simply means "no boosts right now". The last good standings are
/// served if we have them; otherwise the list is empty.
pub async fn active_boosts() -> Vec<BoostStanding> {
    {
        let cache = BOOST_CACHE.read().await;
        if let Some(cached) = cache.as_ref() {
            let stale = cached.fetched_at.elapsed() >= CACHE_TTL;
            let standings = cached.standings.clone();
            drop(cache);
            if stale {
                tokio::spawn(async {
                    if let Err(e) = refresh().await {
                        logger::debug(LogTag::Webserver, &format!("[BOOSTS] refresh failed: {e}"));
                    }
                });
            }
            return standings;
        }
    }

    match refresh().await {
        Ok(standings) => standings,
        Err(e) => {
            logger::debug(LogTag::Webserver, &format!("[BOOSTS] fetch failed: {e}"));
            Vec::new()
        }
    }
}

/// Warm the cache at startup so the first dashboard paint already knows which
/// tokens are boosted — otherwise the tokens table renders once plain and then
/// re-marks itself gold a moment later.
pub fn spawn_prewarm() {
    tokio::spawn(async {
        if let Err(e) = refresh().await {
            logger::debug(LogTag::Webserver, &format!("[BOOSTS] prewarm failed: {e}"));
        }
    });
}
