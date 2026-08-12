//! Featured route types — the provider shapes plus the one card every featured
//! endpoint returns.

use crate::webserver::routes::boosts::BoostStanding;
use serde::{Deserialize, Serialize};

/// External token (Jupiter/DexScreener) as the provider returns it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalToken {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub logo: Option<String>,
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub discord: Option<String>,
    pub price_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub liquidity: Option<f64>,
    pub organic_score: Option<f64>,
}

/// The single shape every featured category returns.
///
/// The three sources (our boost feed, Jupiter x2, DexScreener) each carry a
/// different subset of fields, so everything is normalized into this one card and
/// the frontend has exactly one renderer.
///
/// All stats are filled from our LOCAL DATABASE only (never a live API call), so a
/// token we have no market data for simply has `None` and the card omits that stat.
#[derive(Debug, Clone, Serialize)]
pub struct FeaturedCard {
    pub mint: String,
    pub name: String,
    pub symbol: String,

    /// Active ScreenerBot boosts on this mint. `0` for an organic discovery row —
    /// this is the ONE field that decides whether a surface treats the token as
    /// paid, so it is always serialized, never skipped.
    pub boosts: u32,
    /// True once the boosts reach the website's Golden threshold.
    pub golden: bool,

    /// Square token logo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    /// Wide header/banner image (DexScreener, 1500x500). Most tokens have none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_sol: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_change_24h: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidity_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_24h: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holders: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txns_24h_buys: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txns_24h_sells: Option<i64>,

    /// Whether we hold market data for this token locally.
    pub is_in_database: bool,
}

impl FeaturedCard {
    /// A card carrying nothing but its mint. Identity and stats are filled in by
    /// the enrichment pass; a mint alone is all the boost feed gives us.
    fn bare(mint: String) -> Self {
        Self {
            mint,
            name: String::new(),
            symbol: String::new(),
            boosts: 0,
            golden: false,
            logo: None,
            banner: None,
            website: None,
            twitter: None,
            telegram: None,
            discord: None,
            price_usd: None,
            price_sol: None,
            price_change_24h: None,
            market_cap: None,
            liquidity_usd: None,
            volume_24h: None,
            holders: None,
            security_score: None,
            txns_24h_buys: None,
            txns_24h_sells: None,
            is_in_database: false,
        }
    }
}

/// The boost feed carries a mint and its paid standing and nothing else — no name,
/// no symbol, no logo. Identity is resolved afterwards from our own database and,
/// for a mint we do not track yet, from DexScreener (`identity::fill_identity`).
impl From<BoostStanding> for FeaturedCard {
    fn from(standing: BoostStanding) -> Self {
        Self {
            boosts: standing.boosts,
            golden: standing.golden,
            ..Self::bare(standing.mint)
        }
    }
}

impl From<ExternalToken> for FeaturedCard {
    fn from(t: ExternalToken) -> Self {
        Self {
            name: t.name,
            symbol: t.symbol,
            logo: t.logo,
            website: t.website,
            twitter: t.twitter,
            telegram: t.telegram,
            discord: t.discord,
            // Provider-supplied values; the DB pass overrides them when we hold our
            // own (fresher, SOL-denominated) market data.
            price_usd: t.price_usd,
            liquidity_usd: t.liquidity,
            volume_24h: t.volume_24h,
            ..Self::bare(t.mint)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_boosted_card_keeps_its_paid_standing_and_starts_without_identity() {
        let card = FeaturedCard::from(BoostStanding {
            mint: "mint1".to_owned(),
            boosts: 500,
            golden: true,
        });
        assert_eq!(card.mint, "mint1");
        assert_eq!(card.boosts, 500);
        assert!(card.golden);
        assert!(card.name.is_empty());
        assert!(card.symbol.is_empty());
    }

    #[test]
    fn a_discovery_card_is_never_boosted() {
        let card = FeaturedCard::from(ExternalToken {
            mint: "mint2".to_owned(),
            name: "Token".to_owned(),
            symbol: "TKN".to_owned(),
            logo: None,
            website: None,
            twitter: None,
            telegram: None,
            discord: None,
            price_usd: Some(1.5),
            volume_24h: Some(10.0),
            liquidity: Some(20.0),
            organic_score: Some(30.0),
        });
        assert_eq!(card.boosts, 0);
        assert!(!card.golden);
        assert_eq!(card.price_usd, Some(1.5));
        assert_eq!(card.liquidity_usd, Some(20.0));
    }

    #[test]
    fn boosts_are_always_serialized_so_a_surface_can_always_tell_paid_from_organic() {
        let json = serde_json::to_value(FeaturedCard::bare("mint3".to_owned())).unwrap();
        assert_eq!(json["boosts"], 0);
        assert_eq!(json["golden"], false);
        assert!(json.get("logo").is_none());
    }
}
