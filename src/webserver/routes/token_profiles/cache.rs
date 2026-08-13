use super::types::{PublishedTokenProfile, WebsiteProfileResponse};
use crate::connectivity;
use crate::logger::{self, LogTag};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const PROFILE_FEED_URL: &str = "https://screenerbot.io/api/token-profile/public";
const CACHE_TTL: Duration = Duration::from_secs(60);
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

struct ProfileCache {
    profiles: HashMap<String, PublishedTokenProfile>,
    fetched_at: Instant,
}

static CACHE: LazyLock<Arc<RwLock<Option<ProfileCache>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

async fn fetch_feed() -> Result<HashMap<String, PublishedTokenProfile>, String> {
    if connectivity::is_network_offline() {
        return Err("network offline".to_owned());
    }
    let response = crate::net::client()
        .get(PROFILE_FEED_URL)
        .timeout(FETCH_TIMEOUT)
        .send()
        .await
        .map_err(|error| format!("token profile feed request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "token profile feed returned status {}",
            response.status()
        ));
    }
    let payload: WebsiteProfileResponse = response
        .json()
        .await
        .map_err(|error| format!("token profile feed parse failed: {error}"))?;
    Ok(payload
        .profiles
        .into_iter()
        .filter(|profile| !profile.mint.trim().is_empty())
        .map(|profile| (profile.mint.clone(), profile))
        .collect())
}

async fn refresh() -> Result<HashMap<String, PublishedTokenProfile>, String> {
    let profiles = fetch_feed().await?;
    *CACHE.write().await = Some(ProfileCache {
        profiles: profiles.clone(),
        fetched_at: Instant::now(),
    });
    Ok(profiles)
}

pub async fn all() -> Vec<PublishedTokenProfile> {
    let profiles = {
        let cache = CACHE.read().await;
        cache.as_ref().map(|value| {
            let stale = value.fetched_at.elapsed() >= CACHE_TTL;
            (value.profiles.clone(), stale)
        })
    };
    if let Some((profiles, stale)) = profiles {
        if stale {
            tokio::spawn(async {
                if let Err(error) = refresh().await {
                    logger::debug(
                        LogTag::Webserver,
                        &format!("[TOKEN_PROFILES] refresh failed: {error}"),
                    );
                }
            });
        }
        return profiles.into_values().collect();
    }
    match refresh().await {
        Ok(profiles) => profiles.into_values().collect(),
        Err(error) => {
            logger::debug(
                LogTag::Webserver,
                &format!("[TOKEN_PROFILES] fetch failed: {error}"),
            );
            Vec::new()
        }
    }
}

pub async fn get(mint: &str) -> Option<PublishedTokenProfile> {
    all().await.into_iter().find(|profile| profile.mint == mint)
}

pub fn spawn_prewarm() {
    tokio::spawn(async {
        if let Err(error) = refresh().await {
            logger::debug(
                LogTag::Webserver,
                &format!("[TOKEN_PROFILES] prewarm failed: {error}"),
            );
        }
    });
}
