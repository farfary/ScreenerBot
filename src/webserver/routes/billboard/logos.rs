//! Billboard logo resolution — normalize provider logos and backfill missing ones.
//!
//! Billboard tokens arrive from four independent sources (our website, Jupiter x2,
//! DexScreener) and each may ship a missing, blank or malformed logo URL. Rather
//! than let the frontend render a letter placeholder whenever one provider happens
//! to lack an icon, every category is passed through [`fill_missing_logos`], which
//! resolves a logo from the first source that has one:
//!
//! 1. the provider's own URL (normalized),
//! 2. our local token database (DexScreener image, then GeckoTerminal image),
//! 3. a live DexScreener batch lookup for whatever is still missing.

use super::types::BillboardCard;
use crate::apis::dexscreener::MAX_TOKENS_PER_REQUEST;
use crate::apis::get_api_manager;
use crate::connectivity;
use crate::logger::{self, LogTag};
use crate::tokens;
use std::collections::HashMap;

/// Normalize a provider-supplied logo URL, or return `None` if it is unusable.
///
/// Providers are inconsistent: Jupiter serves some icons with a capitalized scheme
/// (`Https://www.blackbullsol.com/...` for ANSEM), others ship blank strings or
/// bare `ipfs://` URIs that a browser `<img>` cannot load. A case-sensitive
/// `https://` prefix check silently discarded all of those.
pub(super) fn normalize_logo_url(raw: Option<&str>) -> Option<String> {
    let url = raw?.trim();
    if url.is_empty() {
        return None;
    }

    let lower = url.to_ascii_lowercase();

    // Rewrite the scheme in place so the casing is normalized but the path — which
    // IS case-sensitive on most CDNs — is preserved exactly as the provider sent it.
    for scheme in ["https://", "http://"] {
        if lower.starts_with(scheme) {
            return Some(format!("{scheme}{}", &url[scheme.len()..]));
        }
    }

    // `ipfs://<cid>` is not loadable by an <img>; route it through a public gateway.
    if let Some(cid) = lower
        .strip_prefix("ipfs://")
        .map(|_| &url["ipfs://".len()..])
    {
        let cid = cid.trim_start_matches("ipfs/");
        if !cid.is_empty() {
            return Some(format!("https://ipfs.io/ipfs/{cid}"));
        }
    }

    None
}

/// Resolve a logo for every card that lacks one, in place.
///
/// Normalizes what the provider gave us, then backfills the remainder from the
/// local database and finally from a live DexScreener lookup. Best-effort: any
/// failing stage simply leaves the logo unset and the frontend placeholder stands.
pub(super) async fn fill_missing_logos(cards: &mut [BillboardCard]) {
    if cards.is_empty() {
        return;
    }

    for card in cards.iter_mut() {
        card.logo = normalize_logo_url(card.logo.as_deref());
        card.banner = normalize_logo_url(card.banner.as_deref());
    }

    let mut missing = missing_mints(cards);
    if missing.is_empty() {
        return;
    }

    // Stage 1: our own database — free, already holds DexScreener + GeckoTerminal
    // images for every token we have ever fetched market data for.
    match tokens::database::get_token_images_batch_async(missing.clone()).await {
        Ok(images) => apply_logos(cards, &images),
        Err(e) => logger::debug(
            LogTag::Webserver,
            &format!("[BILLBOARD] DB logo lookup failed: {e}"),
        ),
    }

    missing = missing_mints(cards);
    if missing.is_empty() {
        return;
    }

    // Stage 2: live DexScreener lookup for tokens we have no market data for yet
    // (freshly trending tokens are routinely newer than our database).
    let images = fetch_dexscreener_logos(&missing).await;
    apply_logos(cards, &images);
}

/// Mints whose logo is still unresolved.
fn missing_mints(cards: &[BillboardCard]) -> Vec<String> {
    let mut mints: Vec<String> = cards
        .iter()
        .filter(|c| c.logo.is_none())
        .map(|c| c.mint.clone())
        .collect();
    mints.sort_unstable();
    mints.dedup();
    mints
}

/// Assign resolved logos back onto the tokens that are still missing one.
fn apply_logos(cards: &mut [BillboardCard], images: &HashMap<String, String>) {
    for card in cards.iter_mut() {
        if card.logo.is_some() {
            continue;
        }
        if let Some(url) = images.get(&card.mint) {
            card.logo = normalize_logo_url(Some(url));
        }
    }
}

/// Look up logos for the given mints via DexScreener's batch token endpoint.
async fn fetch_dexscreener_logos(mints: &[String]) -> HashMap<String, String> {
    let mut images = HashMap::new();

    if connectivity::is_network_offline() {
        return images;
    }

    let api = get_api_manager();
    if !api.dexscreener.is_enabled() {
        return images;
    }

    for chunk in mints.chunks(MAX_TOKENS_PER_REQUEST) {
        match api
            .dexscreener
            .fetch_token_batch(chunk, Some("solana"))
            .await
        {
            Ok(pools) => {
                for pool in pools {
                    if let Some(url) = pool.info_image_url {
                        images.entry(pool.base_token_address).or_insert(url);
                    }
                }
            }
            Err(e) => {
                logger::debug(
                    LogTag::Webserver,
                    &format!("[BILLBOARD] DexScreener logo lookup failed: {e}"),
                );
                break;
            }
        }
    }

    images
}
