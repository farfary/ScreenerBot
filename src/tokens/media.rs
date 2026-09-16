//! Token logos and banners, resolved the same way everywhere in the ecosystem.
//!
//! The ScreenerBot data service owns the resolution order:
//! - **logo:** the token's own on-chain metadata image → a published Token Profile
//!   icon → a market provider's logo;
//! - **banner:** a published Token Profile banner → a market provider's header.
//!
//! This app already stores the market providers' pictures itself, so it keeps
//! only what OUTRANKS them: a metadata logo, or published profile media. Every
//! place a token's `image_url`/`header_image_url` is assembled passes its provider
//! value through [`resolve_logo`]/[`resolve_banner`], so a provider picture can
//! never win over the token's own.
//!
//! Signed out or offline the data service answers nothing; the provider pictures
//! then stand as before, and the last fetched overrides survive a restart.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::errors::InternalError;
use crate::logger::{self, LogTag};
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::TokenResult;
use crate::tokens::Error;
use crate::utils::{check_shutdown_or_delay, run_or_shutdown};

/// Mints per data-service request; the service accepts up to 100.
const MINTS_PER_REQUEST: usize = 100;
/// Requests per sync pass, so a large token store spreads over several passes.
const REQUESTS_PER_PASS: usize = 5;
/// A mint the service has not resolved yet is asked about again this soon.
const PENDING_RETRY_SECS: i64 = 15 * 60;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaOverride {
    pub logo_url: Option<String>,
    pub banner_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaUpdate {
    pub mint: String,
    pub media: MediaOverride,
    pub fetched_at: i64,
    pub next_fetch_at: i64,
}

static OVERRIDES: LazyLock<RwLock<HashMap<String, MediaOverride>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn override_for(mint: &str) -> Option<MediaOverride> {
    OVERRIDES.read().ok()?.get(mint).cloned()
}

/// The logo that outranks the market providers (on-chain metadata or a published
/// profile), when the data service reported one.
pub fn override_logo(mint: &str) -> Option<String> {
    override_for(mint)?.logo_url
}

/// A published profile banner, when the data service reported one.
pub fn override_banner(mint: &str) -> Option<String> {
    override_for(mint)?.banner_url
}

/// The token's logo: an override first, then the provider's.
pub fn resolve_logo(mint: &str, provider: Option<String>) -> Option<String> {
    override_logo(mint).or(provider)
}

/// The token's banner: an override first, then the provider's.
pub fn resolve_banner(mint: &str, provider: Option<String>) -> Option<String> {
    override_banner(mint).or(provider)
}

#[derive(Debug, Deserialize)]
struct MediaResponse {
    media: HashMap<String, ServiceMedia>,
}

#[derive(Debug, Deserialize)]
struct ServiceMedia {
    logo_url: Option<String>,
    logo_source: Option<String>,
    banner_url: Option<String>,
    banner_source: Option<String>,
    chain_status: String,
}

/// Keep only what outranks this app's own providers; a `market` source is the
/// service's copy of a picture this app already fetched itself.
fn override_from(media: &ServiceMedia) -> MediaOverride {
    let outranks =
        |source: &Option<String>| matches!(source.as_deref(), Some("chain") | Some("profile"));
    MediaOverride {
        logo_url: media
            .logo_url
            .clone()
            .filter(|_| outranks(&media.logo_source)),
        banner_url: media
            .banner_url
            .clone()
            .filter(|_| media.banner_source.as_deref() == Some("profile")),
    }
}

/// Turn one service answer into stored updates for every requested mint. A mint
/// absent from the answer, or still pending its first on-chain check, is retried
/// soon; a resolved one waits for the refresh window.
fn updates_from(
    requested: &[String],
    response: &MediaResponse,
    now: i64,
    refresh_secs: i64,
) -> Vec<MediaUpdate> {
    requested
        .iter()
        .map(|mint| {
            let answer = response.media.get(mint);
            let pending = answer.is_none_or(|media| media.chain_status == "pending");
            MediaUpdate {
                mint: mint.clone(),
                media: answer.map(override_from).unwrap_or_default(),
                fetched_at: now,
                next_fetch_at: now
                    + if pending {
                        PENDING_RETRY_SECS
                    } else {
                        refresh_secs
                    },
            }
        })
        .collect()
}

fn apply(updates: &[MediaUpdate]) {
    let Ok(mut overrides) = OVERRIDES.write() else {
        return;
    };
    for update in updates {
        if update.media == MediaOverride::default() {
            overrides.remove(&update.mint);
        } else {
            overrides.insert(update.mint.clone(), update.media.clone());
        }
    }
}

/// One pass: fetch media for the tokens due, store it, publish it in memory.
/// Stops at the first request the service does not answer, marking nothing, so
/// a signed-out or offline install keeps its tokens due for when it can ask.
async fn sync_pass(db: &Arc<TokenDatabase>) -> TokenResult<usize> {
    let refresh_secs = crate::config::with_config(|config| {
        config.tokens.sources.screenerbot_server.media_refresh_hours
    })
    .max(1) as i64
        * 3_600;
    let now = chrono::Utc::now().timestamp();
    let due_db = db.clone();
    let due = tokio::task::spawn_blocking(move || {
        due_db.mints_due_for_media(now, MINTS_PER_REQUEST * REQUESTS_PER_PASS)
    })
    .await
    .map_err(|error| Error::Internal(InternalError::from(error)))??;

    let mut stored = 0;
    for chunk in due.chunks(MINTS_PER_REQUEST) {
        let Some(response) = crate::data_server::get_json::<MediaResponse>(
            crate::data_server::Surface::Tokens,
            "/v1/tokens/media",
            &[("mints", chunk.join(","))],
        )
        .await
        else {
            break;
        };
        let updates = updates_from(chunk, &response, now, refresh_secs);
        let store_db = db.clone();
        let to_store = updates.clone();
        tokio::task::spawn_blocking(move || store_db.store_media_updates(&to_store))
            .await
            .map_err(|error| Error::Internal(InternalError::from(error)))??;
        apply(&updates);
        stored += updates.len();
    }
    Ok(stored)
}

/// Load persisted overrides, then keep them current from the data service.
pub fn start_media_sync_loop(db: Arc<TokenDatabase>, shutdown: Arc<Notify>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let load_db = db.clone();
        match tokio::task::spawn_blocking(move || load_db.load_media_overrides()).await {
            Ok(Ok(overrides)) => {
                if let Ok(mut current) = OVERRIDES.write() {
                    *current = overrides;
                }
            }
            Ok(Err(error)) => logger::warning(
                LogTag::Tokens,
                &format!("[MEDIA] Failed to load stored token media: {error}"),
            ),
            Err(error) => logger::warning(
                LogTag::Tokens,
                &format!("[MEDIA] Token media load task failed: {error}"),
            ),
        }
        loop {
            let interval = crate::config::with_config(|config| {
                config.tokens.sources.screenerbot_server.media_sync_seconds
            })
            .max(30);
            if check_shutdown_or_delay(&shutdown, Duration::from_secs(interval)).await {
                break;
            }
            match run_or_shutdown(&shutdown, sync_pass(&db)).await {
                None => break,
                Some(Ok(0)) => {}
                Some(Ok(stored)) => logger::debug(
                    LogTag::Tokens,
                    &format!("[MEDIA] Stored media for {stored} tokens"),
                ),
                Some(Err(error)) => logger::warning(
                    LogTag::Tokens,
                    &format!("[MEDIA] Token media sync failed: {error}"),
                ),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(logo: (&str, &str), banner: Option<(&str, &str)>, status: &str) -> ServiceMedia {
        ServiceMedia {
            logo_url: Some(logo.0.to_owned()),
            logo_source: Some(logo.1.to_owned()),
            banner_url: banner.map(|(url, _)| url.to_owned()),
            banner_source: banner.map(|(_, source)| source.to_owned()),
            chain_status: status.to_owned(),
        }
    }

    #[test]
    fn only_media_that_outranks_the_providers_is_kept() {
        let chain = service(
            ("https://chain/a.png", "chain"),
            Some(("https://site/b.png", "profile")),
            "found",
        );
        assert_eq!(
            override_from(&chain),
            MediaOverride {
                logo_url: Some("https://chain/a.png".to_owned()),
                banner_url: Some("https://site/b.png".to_owned()),
            }
        );
        let market = service(
            ("https://dex/a.png", "market"),
            Some(("https://dex/b.png", "market")),
            "no_image",
        );
        assert_eq!(override_from(&market), MediaOverride::default());
    }

    #[test]
    fn pending_and_unanswered_mints_retry_soon() {
        let mut media = HashMap::new();
        media.insert(
            "Done".to_owned(),
            service(("https://chain/a.png", "chain"), None, "found"),
        );
        media.insert(
            "Queued".to_owned(),
            service(("https://dex/a.png", "market"), None, "pending"),
        );
        let response = MediaResponse { media };
        let requested = ["Done".to_owned(), "Queued".to_owned(), "Unknown".to_owned()];
        let updates = updates_from(&requested, &response, 1_000, 86_400);
        assert_eq!(updates[0].next_fetch_at, 1_000 + 86_400);
        assert_eq!(updates[1].next_fetch_at, 1_000 + PENDING_RETRY_SECS);
        assert_eq!(updates[2].next_fetch_at, 1_000 + PENDING_RETRY_SECS);
        assert_eq!(updates[2].media, MediaOverride::default());
    }
}
