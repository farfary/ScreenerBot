//! Featured card identity — normalize provider images and backfill what is missing.
//!
//! Featured cards arrive from three independent sources and each ships a different
//! subset of identity. Our own boost feed is the extreme case: it carries a MINT AND
//! NOTHING ELSE, because the website owns payments, not token metadata. DexScreener's
//! trending board ships an icon but no usable name or symbol. So every category is
//! passed through [`fill_identity`], which resolves each missing field from the first
//! source that has it:
//!
//! 1. the provider's own value (URLs normalized),
//! 2. our local token database,
//! 3. the shared Data Server market cache,
//! 4. live DexScreener and GeckoTerminal batch lookups for whatever is still missing.
//!
//! Every network stage is batched and runs only for mints that are still incomplete.
//! The Data Server goes first because it is our shared provider cache; direct provider
//! calls are the last resort for a freshly boosted token this installation has never seen.

use super::types::FeaturedCard;
use crate::apis::dexscreener::MAX_TOKENS_PER_REQUEST;
use crate::apis::get_api_manager;
use crate::connectivity;
use crate::logger::{self, LogTag};
use crate::tokens;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

/// Direct providers are a last-resort presentation fallback. Their internal
/// rate-limit queues must not delay the entire Featured surface.
const DIRECT_PROVIDER_TIMEOUT: Duration = Duration::from_secs(2);

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

/// Resolve name, symbol, logo and banner for every card that lacks one, in place.
///
/// Best-effort: any failing stage simply leaves the field unset and the frontend
/// placeholder stands.
pub(super) async fn fill_identity(cards: &mut [FeaturedCard]) {
    if cards.is_empty() {
        return;
    }

    for card in cards.iter_mut() {
        card.logo = normalize_logo_url(card.logo.as_deref());
        card.banner = normalize_logo_url(card.banner.as_deref());
        card.name = card.name.trim().to_owned();
        card.symbol = card.symbol.trim().to_owned();
    }

    let mut missing = incomplete_mints(cards);
    if missing.is_empty() {
        return;
    }

    // Stage 1: our own database — free and authoritative for identity already
    // persisted by an earlier featured request, even when the token has no market
    // row yet. Read name/symbol and artwork together in one batch.
    match tokens::database::get_token_info_batch_async(missing.clone()).await {
        Ok(info) => {
            let identities = info
                .into_iter()
                .map(|(mint, (symbol, name, logo))| {
                    (
                        mint,
                        TokenIdentity {
                            name,
                            symbol,
                            logo,
                            banner: None,
                        },
                    )
                })
                .collect();
            apply_identity(cards, &identities);
        }
        Err(e) => logger::debug(
            LogTag::Webserver,
            &format!("[FEATURED] DB identity lookup failed: {e}"),
        ),
    }

    // Optional artwork must never hold the featured endpoint hostage. Network
    // fallbacks exist to make a card readable, so only a missing name or symbol
    // proceeds beyond the local database. A token with no banner is still fully
    // usable and should render immediately.
    missing = unreadable_mints(cards);
    if missing.is_empty() {
        return;
    }

    // Stage 2: our shared Data Server cache. The website Terminal reads this same
    // normalized market identity, so a token must not be named on the website while
    // the desktop app renders "???" merely because DexScreener has not indexed it.
    let resolved = fetch_server_identity(&missing).await;
    apply_identity(cards, &resolved);

    missing = unreadable_mints(cards);
    if missing.is_empty() {
        return;
    }

    // Stage 3: live DexScreener lookup for anything the shared cache still lacks.
    let resolved = fetch_dexscreener_identity(&missing).await;
    apply_identity(cards, &resolved);

    missing = unreadable_mints(cards);
    if missing.is_empty() {
        return;
    }

    // Stage 4: GeckoTerminal is the final identity fallback. Newly launched pools
    // can appear here before DexScreener, which is the exact gap boosted tokens hit.
    let resolved = fetch_geckoterminal_identity(&missing).await;
    apply_identity(cards, &resolved);
}

/// Mints still missing any identity field.
fn incomplete_mints(cards: &[FeaturedCard]) -> Vec<String> {
    let mut mints: Vec<String> = cards
        .iter()
        .filter(|c| {
            c.logo.is_none() || c.banner.is_none() || c.name.is_empty() || c.symbol.is_empty()
        })
        .map(|c| c.mint.clone())
        .collect();
    mints.sort_unstable();
    mints.dedup();
    mints
}

/// Mints that still cannot be labeled for a user. Optional artwork does not count.
fn unreadable_mints(cards: &[FeaturedCard]) -> Vec<String> {
    let mut mints: Vec<String> = cards
        .iter()
        .filter(|card| card.name.is_empty() || card.symbol.is_empty())
        .map(|card| card.mint.clone())
        .collect();
    mints.sort_unstable();
    mints.dedup();
    mints
}

/// A token's identity as a provider returned it.
#[derive(Default)]
pub(super) struct TokenIdentity {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub logo: Option<String>,
    pub banner: Option<String>,
}

#[derive(Deserialize)]
struct ServerMarketResponse {
    markets: HashMap<String, ServerMarketIdentity>,
}

#[derive(Deserialize)]
struct ServerMarketIdentity {
    name: Option<String>,
    symbol: Option<String>,
    image_url: Option<String>,
    banner_url: Option<String>,
}

impl From<ServerMarketIdentity> for TokenIdentity {
    fn from(value: ServerMarketIdentity) -> Self {
        Self {
            name: value.name,
            symbol: value.symbol,
            logo: value.image_url,
            banner: value.banner_url,
        }
    }
}

/// Assign resolved identity back onto the cards that are still missing a field.
fn apply_identity(cards: &mut [FeaturedCard], resolved: &HashMap<String, TokenIdentity>) {
    for card in cards.iter_mut() {
        let Some(found) = resolved.get(&card.mint) else {
            continue;
        };
        if card.logo.is_none() {
            card.logo = normalize_logo_url(found.logo.as_deref());
        }
        if card.banner.is_none() {
            card.banner = normalize_logo_url(found.banner.as_deref());
        }
        if card.name.is_empty() {
            card.name = found.name.clone().unwrap_or_default().trim().to_owned();
        }
        if card.symbol.is_empty() {
            card.symbol = found.symbol.clone().unwrap_or_default().trim().to_owned();
        }
    }
}

/// Look up identity in the Data Server's normalized market cache.
async fn fetch_server_identity(mints: &[String]) -> HashMap<String, TokenIdentity> {
    let mut resolved = HashMap::new();
    // One question before a loop over chunks: without access every chunk would
    // be refused identically, and the reason is already published once.
    if !crate::data_server::is_usable(crate::data_server::Surface::Tokens) {
        return resolved;
    }

    for chunk in mints.chunks(MAX_TOKENS_PER_REQUEST) {
        let Some(payload) = crate::data_server::get_json::<ServerMarketResponse>(
            crate::data_server::Surface::Tokens,
            "/v1/market",
            &[("mints", chunk.join(","))],
        )
        .await
        else {
            continue;
        };
        resolved.extend(
            payload
                .markets
                .into_iter()
                .map(|(mint, identity)| (mint, identity.into())),
        );
    }

    resolved
}

/// Look up identity for the given mints via DexScreener's batch endpoint.
async fn fetch_dexscreener_identity(mints: &[String]) -> HashMap<String, TokenIdentity> {
    let mut resolved: HashMap<String, TokenIdentity> = HashMap::new();

    if connectivity::is_network_offline() {
        return resolved;
    }

    let api = get_api_manager();
    if !api.dexscreener.is_enabled() {
        return resolved;
    }

    for chunk in mints.chunks(MAX_TOKENS_PER_REQUEST) {
        match tokio::time::timeout(
            DIRECT_PROVIDER_TIMEOUT,
            api.dexscreener
                .fetch_token_batch(chunk, Some(crate::chains::adapter().market_data_network())),
        )
        .await
        {
            Ok(Ok(pools)) => {
                for pool in pools {
                    let entry = resolved.entry(pool.base_token_address).or_default();
                    if entry.logo.is_none() {
                        entry.logo = pool.info_image_url;
                    }
                    if entry.banner.is_none() {
                        entry.banner = pool.info_header;
                    }
                    if entry.name.is_none() && !pool.base_token_name.is_empty() {
                        entry.name = Some(pool.base_token_name);
                    }
                    if entry.symbol.is_none() && !pool.base_token_symbol.is_empty() {
                        entry.symbol = Some(pool.base_token_symbol);
                    }
                }
            }
            Ok(Err(e)) => {
                logger::debug(
                    LogTag::Webserver,
                    &format!("[FEATURED] DexScreener identity lookup failed: {e}"),
                );
                break;
            }
            Err(_) => {
                logger::debug(
                    LogTag::Webserver,
                    "[FEATURED] DexScreener identity lookup timed out",
                );
                break;
            }
        }
    }

    resolved
}

/// Look up identity via GeckoTerminal's token batch endpoint.
async fn fetch_geckoterminal_identity(mints: &[String]) -> HashMap<String, TokenIdentity> {
    let mut resolved = HashMap::new();

    if connectivity::is_network_offline() {
        return resolved;
    }

    let api = get_api_manager();
    if !api.geckoterminal.is_enabled() {
        return resolved;
    }

    for chunk in mints.chunks(MAX_TOKENS_PER_REQUEST) {
        let addresses = chunk.join(",");
        match tokio::time::timeout(
            DIRECT_PROVIDER_TIMEOUT,
            api.geckoterminal.fetch_tokens_multi(
                crate::chains::adapter().market_data_network(),
                &addresses,
                None,
                None,
            ),
        )
        .await
        {
            Ok(Ok(response)) => {
                for token in response.data {
                    let attrs = token.attributes;
                    resolved.insert(
                        attrs.address,
                        TokenIdentity {
                            name: Some(attrs.name),
                            symbol: Some(attrs.symbol),
                            logo: attrs.image_url,
                            banner: None,
                        },
                    );
                }
            }
            Ok(Err(e)) => {
                logger::debug(
                    LogTag::Webserver,
                    &format!("[FEATURED] GeckoTerminal identity lookup failed: {e}"),
                );
                break;
            }
            Err(_) => {
                logger::debug(
                    LogTag::Webserver,
                    "[FEATURED] GeckoTerminal identity lookup timed out",
                );
                break;
            }
        }
    }

    resolved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webserver::routes::boosts::BoostStanding;

    fn boosted(mint: &str) -> FeaturedCard {
        FeaturedCard::from(BoostStanding {
            mint: mint.to_owned(),
            boosts: 10,
            golden: false,
        })
    }

    #[test]
    fn a_capitalized_scheme_survives_normalization_with_its_path_intact() {
        assert_eq!(
            normalize_logo_url(Some("Https://cdn.example.com/A/b.PNG")),
            Some("https://cdn.example.com/A/b.PNG".to_owned())
        );
    }

    #[test]
    fn an_ipfs_uri_is_routed_through_a_gateway_and_junk_is_dropped() {
        assert_eq!(
            normalize_logo_url(Some("ipfs://ipfs/QmCid")),
            Some("https://ipfs.io/ipfs/QmCid".to_owned())
        );
        assert_eq!(normalize_logo_url(Some("   ")), None);
        assert_eq!(normalize_logo_url(Some("ipfs://")), None);
        assert_eq!(normalize_logo_url(None), None);
    }

    #[test]
    fn a_boost_feed_card_is_incomplete_until_every_identity_field_lands() {
        let mut cards = vec![boosted("mint1")];
        assert_eq!(incomplete_mints(&cards), vec!["mint1".to_owned()]);

        let mut resolved = HashMap::new();
        resolved.insert(
            "mint1".to_owned(),
            TokenIdentity {
                name: Some("  Boosted Token ".to_owned()),
                symbol: Some("BOOST".to_owned()),
                logo: Some("https://cdn/logo.png".to_owned()),
                banner: Some("https://cdn/banner.png".to_owned()),
            },
        );
        apply_identity(&mut cards, &resolved);

        assert_eq!(cards[0].name, "Boosted Token");
        assert_eq!(cards[0].symbol, "BOOST");
        assert!(incomplete_mints(&cards).is_empty());
        assert!(unreadable_mints(&cards).is_empty());
    }

    #[test]
    fn missing_optional_artwork_does_not_trigger_network_identity_fallbacks() {
        let cards = vec![FeaturedCard {
            name: "Readable Token".to_owned(),
            symbol: "READ".to_owned(),
            logo: None,
            banner: None,
            ..boosted("mint1")
        }];

        assert_eq!(incomplete_mints(&cards), vec!["mint1".to_owned()]);
        assert!(unreadable_mints(&cards).is_empty());
    }

    #[test]
    fn resolved_identity_never_overwrites_what_the_provider_already_supplied() {
        let mut cards = vec![FeaturedCard {
            name: "Provider Name".to_owned(),
            symbol: "PRV".to_owned(),
            logo: Some("https://cdn/provider.png".to_owned()),
            ..boosted("mint1")
        }];

        let mut resolved = HashMap::new();
        resolved.insert(
            "mint1".to_owned(),
            TokenIdentity {
                name: Some("Other".to_owned()),
                symbol: Some("OTH".to_owned()),
                logo: Some("https://cdn/other.png".to_owned()),
                banner: Some("https://cdn/banner.png".to_owned()),
            },
        );
        apply_identity(&mut cards, &resolved);

        assert_eq!(cards[0].name, "Provider Name");
        assert_eq!(cards[0].symbol, "PRV");
        assert_eq!(cards[0].logo.as_deref(), Some("https://cdn/provider.png"));
        // The one field it DID lack is filled.
        assert_eq!(cards[0].banner.as_deref(), Some("https://cdn/banner.png"));
    }

    #[test]
    fn data_server_market_identity_maps_to_the_featured_shape() {
        let payload: ServerMarketResponse = serde_json::from_value(serde_json::json!({
            "markets": {
                "mint1": {
                    "name": "  Resila Bio Tech ",
                    "symbol": "RBT",
                    "image_url": null,
                    "banner_url": "https://cdn/banner.png"
                }
            }
        }))
        .unwrap();
        let resolved = payload
            .markets
            .into_iter()
            .map(|(mint, identity)| (mint, identity.into()))
            .collect();
        let mut cards = vec![boosted("mint1")];

        apply_identity(&mut cards, &resolved);

        assert_eq!(cards[0].name, "Resila Bio Tech");
        assert_eq!(cards[0].symbol, "RBT");
        assert_eq!(cards[0].banner.as_deref(), Some("https://cdn/banner.png"));
    }
}
