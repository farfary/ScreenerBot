//! Billboard route types — source shapes from each provider plus the unified card
//! that every billboard endpoint returns.

use serde::{Deserialize, Serialize};

/// Billboard token from website API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillboardToken {
    pub id: String,
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub discord: Option<String>,
    pub github: Option<String>,
    pub logo_url: Option<String>,
    pub description: Option<String>,
    pub featured: bool,
    pub created_at: String,
}

/// Website billboard API response wrapper
#[derive(Debug, Deserialize)]
pub(super) struct WebsiteBillboardResponse {
    pub success: bool,
    pub tokens: Vec<BillboardToken>,
}

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

/// The single shape every billboard category returns.
///
/// The four sources (our website, Jupiter x2, DexScreener) each carry a different
/// subset of fields, and previously only the `featured` category was enriched from
/// our database while the external ones shipped whatever the provider happened to
/// send. Everything is normalized into this one card instead, so the frontend has
/// exactly one renderer and every category shows the same metrics.
///
/// All stats are filled from our LOCAL DATABASE only (never a live API call), so a
/// token we have no market data for simply has `None` and the card omits that stat.
#[derive(Debug, Clone, Serialize)]
pub struct BillboardCard {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub featured: bool,

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

impl From<BillboardToken> for BillboardCard {
    fn from(t: BillboardToken) -> Self {
        Self {
            mint: t.mint,
            name: t.name,
            symbol: t.symbol,
            featured: t.featured,
            logo: t.logo_url,
            banner: None,
            website: t.website,
            twitter: t.twitter,
            telegram: t.telegram,
            discord: t.discord,
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

impl From<ExternalToken> for BillboardCard {
    fn from(t: ExternalToken) -> Self {
        Self {
            mint: t.mint,
            name: t.name,
            symbol: t.symbol,
            featured: false,
            logo: t.logo,
            banner: None,
            website: t.website,
            twitter: t.twitter,
            telegram: t.telegram,
            discord: t.discord,
            // Provider-supplied values; the DB pass below overrides them when we
            // hold our own (fresher, SOL-denominated) market data.
            price_usd: t.price_usd,
            price_sol: None,
            price_change_24h: None,
            market_cap: None,
            liquidity_usd: t.liquidity,
            volume_24h: t.volume_24h,
            holders: None,
            security_score: None,
            txns_24h_buys: None,
            txns_24h_sells: None,
            is_in_database: false,
        }
    }
}
