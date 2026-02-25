//! Billboard route types — request/response structs for billboard endpoints.

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

/// External token (Jupiter/DexScreener) — unified format for frontend
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_24h: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organic_score: Option<f64>,
}

/// Combined billboard response with all categories
#[derive(Debug, Clone, Serialize)]
pub struct BillboardAllResponse {
    pub featured: Vec<BillboardToken>,
    pub jupiter_organic: Vec<ExternalToken>,
    pub jupiter_traded: Vec<ExternalToken>,
    pub dexscreener_trending: Vec<ExternalToken>,
}

/// Enriched billboard token with data from local token database
#[derive(Debug, Clone, Serialize)]
pub struct EnrichedBillboardToken {
    #[serde(flatten)]
    pub base: BillboardToken,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_sol: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fdv_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidity_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_24h: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_change_1h: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_change_24h: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_score: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_score_normalised: Option<i32>,
    pub is_in_database: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<String>,
}
