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

use super::types::{BillboardToken, ExternalToken};
use crate::apis::dexscreener::MAX_TOKENS_PER_REQUEST;
use crate::apis::get_api_manager;
use crate::connectivity;
use crate::logger::{self, LogTag};
use crate::tokens;
use std::collections::HashMap;

/// A billboard token that carries a logo URL, regardless of the field name the
/// source struct uses (`logo_url` on our featured tokens, `logo` on external ones).
pub(super) trait HasLogo {
    fn mint(&self) -> &str;
    fn logo(&self) -> Option<&str>;
    fn set_logo(&mut self, url: Option<String>);
}

impl HasLogo for BillboardToken {
    fn mint(&self) -> &str {
        &self.mint
    }

    fn logo(&self) -> Option<&str> {
        self.logo_url.as_deref()
    }

    fn set_logo(&mut self, url: Option<String>) {
        self.logo_url = url;
    }
}

impl HasLogo for ExternalToken {
    fn mint(&self) -> &str {
        &self.mint
    }

    fn logo(&self) -> Option<&str> {
        self.logo.as_deref()
    }

    fn set_logo(&mut self, url: Option<String>) {
        self.logo = url;
    }
}

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

/// Resolve a logo for every token that lacks one, in place.
///
/// Normalizes what the provider gave us, then backfills the remainder from the
/// local database and finally from a live DexScreener lookup. Best-effort: any
/// failing stage simply leaves the logo unset and the frontend placeholder stands.
pub(super) async fn fill_missing_logos<T: HasLogo>(tokens: &mut [T]) {
    if tokens.is_empty() {
        return;
    }

    for token in tokens.iter_mut() {
        let normalized = normalize_logo_url(token.logo());
        token.set_logo(normalized);
    }

    let mut missing = missing_mints(tokens);
    if missing.is_empty() {
        return;
    }

    // Stage 1: our own database — free, already holds DexScreener + GeckoTerminal
    // images for every token we have ever fetched market data for.
    match tokens::database::get_token_images_batch_async(missing.clone()).await {
        Ok(images) => apply_logos(tokens, &images),
        Err(e) => logger::debug(
            LogTag::Webserver,
            &format!("[BILLBOARD] DB logo lookup failed: {e}"),
        ),
    }

    missing = missing_mints(tokens);
    if missing.is_empty() {
        return;
    }

    // Stage 2: live DexScreener lookup for tokens we have no market data for yet
    // (freshly trending tokens are routinely newer than our database).
    let images = fetch_dexscreener_logos(&missing).await;
    apply_logos(tokens, &images);
}

/// Mints whose logo is still unresolved.
fn missing_mints<T: HasLogo>(tokens: &[T]) -> Vec<String> {
    let mut mints: Vec<String> = tokens
        .iter()
        .filter(|t| t.logo().is_none())
        .map(|t| t.mint().to_owned())
        .collect();
    mints.sort_unstable();
    mints.dedup();
    mints
}

/// Assign resolved logos back onto the tokens that are still missing one.
fn apply_logos<T: HasLogo>(tokens: &mut [T], images: &HashMap<String, String>) {
    for token in tokens.iter_mut() {
        if token.logo().is_some() {
            continue;
        }
        if let Some(url) = images.get(token.mint()) {
            token.set_logo(normalize_logo_url(Some(url)));
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
