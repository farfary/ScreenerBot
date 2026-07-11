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

/// Resolve a logo and banner for every card that lacks one, in place.
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
        Ok(logos) => {
            let images = logos
                .into_iter()
                .map(|(mint, logo)| {
                    (
                        mint,
                        TokenImages {
                            logo: Some(logo),
                            banner: None,
                        },
                    )
                })
                .collect();
            apply_images(cards, &images);
        }
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
    // (freshly trending tokens are routinely newer than our database). The same
    // response carries the banner, so harvest both — no extra request.
    let images = fetch_dexscreener_images(&missing).await;
    apply_images(cards, &images);
}

/// Mints whose logo OR banner is still unresolved.
fn missing_mints(cards: &[BillboardCard]) -> Vec<String> {
    let mut mints: Vec<String> = cards
        .iter()
        .filter(|c| c.logo.is_none() || c.banner.is_none())
        .map(|c| c.mint.clone())
        .collect();
    mints.sort_unstable();
    mints.dedup();
    mints
}

/// A token's images as a provider returned them.
#[derive(Default)]
pub(super) struct TokenImages {
    pub logo: Option<String>,
    pub banner: Option<String>,
}

/// Assign resolved images back onto the cards that are still missing one.
fn apply_images(cards: &mut [BillboardCard], images: &HashMap<String, TokenImages>) {
    for card in cards.iter_mut() {
        let Some(found) = images.get(&card.mint) else {
            continue;
        };
        if card.logo.is_none() {
            card.logo = normalize_logo_url(found.logo.as_deref());
        }
        if card.banner.is_none() {
            card.banner = normalize_logo_url(found.banner.as_deref());
        }
    }
}

/// Look up logos AND banners for the given mints via DexScreener's batch endpoint.
async fn fetch_dexscreener_images(mints: &[String]) -> HashMap<String, TokenImages> {
    let mut images: HashMap<String, TokenImages> = HashMap::new();

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
                    if pool.info_image_url.is_none() && pool.info_header.is_none() {
                        continue;
                    }
                    let entry = images.entry(pool.base_token_address).or_default();
                    if entry.logo.is_none() {
                        entry.logo = pool.info_image_url;
                    }
                    if entry.banner.is_none() {
                        entry.banner = pool.info_header;
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
