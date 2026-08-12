//! Featured card assembly — turn each source's tokens into enriched cards.
//!
//! Every category goes through [`build_cards`], which normalizes the source token
//! into a [`FeaturedCard`] and then fills in what our own database knows about it
//! (market cap, holders, security score, volume, 24h transactions, banner).
//!
//! The database pass is strictly LOCAL: `get_full_token_async` is cache-first and
//! DB-backed and never triggers a provider fetch, so rendering a featured surface
//! cannot hammer the APIs no matter how many tokens the sources return.

use super::identity::fill_identity;
use super::types::FeaturedCard;
use crate::tokens;

/// Build enriched cards from any featured source.
pub(super) async fn build_cards<T: Into<FeaturedCard>>(source: Vec<T>) -> Vec<FeaturedCard> {
    let mut cards: Vec<FeaturedCard> = source.into_iter().map(Into::into).collect();
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

    // Identity fallbacks (name/symbol/logo/banner from the DB, then a live
    // DexScreener batch for tokens we hold no market data for) and URL
    // normalization. A boosted mint arrives with NO identity at all, so this pass
    // is what makes a just-purchased boost renderable.
    fill_identity(&mut cards).await;

    cards
}

/// Everything the local database can tell us about a featured token.
struct DbStats {
    name: Option<String>,
    symbol: Option<String>,
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
        name: Some(token.name),
        symbol: Some(token.symbol),
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
fn apply_stats(card: &mut FeaturedCard, stats: DbStats) {
    card.is_in_database = true;

    // The logo is token IDENTITY and must be the SAME image everywhere the token
    // appears (token details, positions, featured surfaces). It comes from token
    // metadata (DexScreener/GeckoTerminal), so our stored image is authoritative
    // and OVERRIDES whatever icon a discovery provider happened to ship —
    // otherwise a featured card shows Jupiter's icon while token details shows our
    // DexScreener image for the same mint. A discovery provider's icon only
    // survives as a fallback for tokens we do not track (apply_stats never runs for
    // those; fill_identity handles them).
    if let Some(logo) = stats.logo.filter(|s| !s.trim().is_empty()) {
        card.logo = Some(logo);
    }
    if card.banner.is_none() {
        card.banner = stats.banner;
    }

    // Name and symbol are identity too, but the provider's value is kept when we
    // have one: a discovery board names the token the way its market does, and our
    // stored row can predate a rename. Only a blank is filled.
    if card.name.is_empty() {
        card.name = stats.name.unwrap_or_default().trim().to_owned();
    }
    if card.symbol.is_empty() {
        card.symbol = stats.symbol.unwrap_or_default().trim().to_owned();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webserver::routes::boosts::BoostStanding;

    fn stats() -> DbStats {
        DbStats {
            name: Some("Database Name".to_owned()),
            symbol: Some("DBN".to_owned()),
            logo: Some("https://db/logo.png".to_owned()),
            banner: Some("https://db/banner.png".to_owned()),
            price_usd: 2.0,
            price_sol: 0.01,
            price_change_24h: Some(5.0),
            market_cap: Some(1000.0),
            liquidity_usd: Some(500.0),
            volume_24h: Some(250.0),
            holders: Some(42),
            security_score: Some(80),
            txns_24h_buys: Some(7),
            txns_24h_sells: Some(3),
        }
    }

    fn boosted_card() -> FeaturedCard {
        FeaturedCard::from(BoostStanding {
            mint: "mint1".to_owned(),
            boosts: 500,
            golden: true,
        })
    }

    #[test]
    fn db_stats_never_change_a_cards_paid_standing() {
        let mut card = boosted_card();
        apply_stats(&mut card, stats());
        assert_eq!(card.boosts, 500);
        assert!(card.golden);
        assert!(card.is_in_database);
    }

    #[test]
    fn a_boost_feed_card_takes_its_whole_identity_from_the_database() {
        let mut card = boosted_card();
        apply_stats(&mut card, stats());
        assert_eq!(card.name, "Database Name");
        assert_eq!(card.symbol, "DBN");
        assert_eq!(card.logo.as_deref(), Some("https://db/logo.png"));
        assert_eq!(card.price_usd, Some(2.0));
    }

    #[test]
    fn our_stored_logo_overrides_a_discovery_providers_icon() {
        let mut card = FeaturedCard {
            name: "Provider".to_owned(),
            symbol: "PRV".to_owned(),
            logo: Some("https://jupiter/icon.png".to_owned()),
            ..boosted_card()
        };
        apply_stats(&mut card, stats());
        assert_eq!(card.logo.as_deref(), Some("https://db/logo.png"));
        // Name and symbol are the opposite: the provider's naming stands.
        assert_eq!(card.name, "Provider");
        assert_eq!(card.symbol, "PRV");
    }

    #[test]
    fn a_zero_price_row_does_not_erase_a_provider_price() {
        let mut card = FeaturedCard {
            price_usd: Some(9.0),
            ..boosted_card()
        };
        apply_stats(
            &mut card,
            DbStats {
                price_usd: 0.0,
                price_sol: 0.0,
                ..stats()
            },
        );
        assert_eq!(card.price_usd, Some(9.0));
        assert_eq!(card.price_sol, None);
    }
}
