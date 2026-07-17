//! Billboard card assembly — turn each provider's tokens into enriched cards.
//!
//! Every category goes through [`build_cards`], which normalizes the source token
//! into a [`BillboardCard`] and then fills in what our own database knows about it
//! (market cap, holders, security score, volume, 24h transactions, banner).
//!
//! The database pass is strictly LOCAL: `get_full_token_async` is cache-first and
//! DB-backed and never triggers a provider fetch, so rendering the billboard cannot
//! hammer the APIs no matter how many tokens the sources return.

use super::logos::fill_missing_logos;
use super::types::BillboardCard;
use crate::tokens;

/// Build enriched cards from any billboard source.
pub(super) async fn build_cards<T: Into<BillboardCard>>(source: Vec<T>) -> Vec<BillboardCard> {
    let mut cards: Vec<BillboardCard> = source.into_iter().map(Into::into).collect();
    if cards.is_empty() {
        return cards;
    }

    // Local DB enrichment, concurrently — each lookup is a cache hit or one query.
    let enriched = futures::future::join_all(cards.iter().map(|c| enrich_from_db(c.mint.clone())));

    for (card, stats) in cards.iter_mut().zip(enriched.await) {
        if let Some(stats) = stats {
            apply_stats(card, stats);
        }
    }

    // Logo fallbacks (DB image, then a live DexScreener batch for tokens we have
    // no market data for at all) and URL normalization.
    fill_missing_logos(&mut cards).await;

    cards
}

/// Everything the local database can tell us about a billboard token.
struct DbStats {
    logo: Option<String>,
    banner: Option<String>,
    price_usd: f64,
    price_sol: f64,
    price_change_24h: Option<f64>,
    market_cap: Option<f64>,
    liquidity_usd: Option<f64>,
    volume_24h: Option<f64>,
    holders: Option<i64>,
    security_score: Option<i32>,
    txns_24h_buys: Option<i64>,
    txns_24h_sells: Option<i64>,
}

/// Read a token's stats from our database. `None` when we do not track it.
async fn enrich_from_db(mint: String) -> Option<DbStats> {
    let token = tokens::get_full_token_async(&mint).await.ok()??;

    Some(DbStats {
        logo: token.image_url,
        banner: token.header_image_url,
        price_usd: token.price_usd,
        price_sol: token.price_sol,
        price_change_24h: token.price_change_h24,
        // FDV stands in for market cap when the token has no circulating-supply
        // figure, which is the norm for freshly launched tokens.
        market_cap: token.market_cap.or(token.fdv),
        liquidity_usd: token.liquidity_usd,
        volume_24h: token.volume_h24,
        holders: token.total_holders,
        security_score: token.security_score_normalised,
        txns_24h_buys: token.txns_h24_buys,
        txns_24h_sells: token.txns_h24_sells,
    })
}

/// Overlay our database's view onto the card.
///
/// Our data wins over the provider's for anything we actually hold (it is fresher
/// and consistent across categories), but a provider value is never wiped out by a
/// missing local one.
fn apply_stats(card: &mut BillboardCard, stats: DbStats) {
    card.is_in_database = true;

    // The logo is token IDENTITY and must be the SAME image everywhere the token
    // appears (token details, positions, billboard). It comes from token metadata
    // (DexScreener/GeckoTerminal), never a listing upload, so our stored image is
    // authoritative and OVERRIDES whatever icon a discovery provider happened to
    // ship — otherwise the billboard shows Jupiter's icon while token details shows
    // our DexScreener image for the same mint. A discovery provider's icon only
    // survives as a fallback for tokens we do not track (apply_stats never runs for
    // those; fill_missing_logos handles them).
    if let Some(logo) = stats.logo.filter(|s| !s.trim().is_empty()) {
        card.logo = Some(logo);
    }
    // The banner is a promotional image the listing owner MAY customize, so an
    // owner-uploaded banner (already on the card) wins; our DexScreener header image
    // only fills in when the listing has none.
    if card.banner.is_none() {
        card.banner = stats.banner;
    }

    if stats.price_usd > 0.0 {
        card.price_usd = Some(stats.price_usd);
    }
    if stats.price_sol > 0.0 {
        card.price_sol = Some(stats.price_sol);
    }

    card.price_change_24h = stats.price_change_24h.or(card.price_change_24h);
    card.market_cap = stats.market_cap.or(card.market_cap);
    card.liquidity_usd = stats.liquidity_usd.or(card.liquidity_usd);
    card.volume_24h = stats.volume_24h.or(card.volume_24h);
    card.holders = stats.holders.or(card.holders);
    card.security_score = stats.security_score.or(card.security_score);
    card.txns_24h_buys = stats.txns_24h_buys.or(card.txns_24h_buys);
    card.txns_24h_sells = stats.txns_24h_sells.or(card.txns_24h_sells);
}
