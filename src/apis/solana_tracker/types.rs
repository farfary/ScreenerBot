//! SolanaTracker API response types

use serde::Deserialize;

/// OHLCV response — note: API returns key "oclhv" (their typo, not "ohlcv")
#[derive(Debug, Clone, Deserialize)]
pub struct OhlcvResponse {
    /// Candle data (API key is "oclhv" — a typo in their API)
    #[serde(alias = "oclhv", alias = "ohlcv", default)]
    pub candles: Vec<OhlcvCandle>,
}

/// Single OHLCV candle from SolanaTracker
#[derive(Debug, Clone, Deserialize)]
pub struct OhlcvCandle {
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
    /// Unix timestamp in seconds
    pub time: i64,
}

/// Credits response
#[derive(Debug, Clone, Deserialize)]
pub struct CreditsResponse {
    pub credits: i64,
}

/// Token info response
#[derive(Debug, Clone, Deserialize)]
pub struct TokenInfoResponse {
    #[serde(default)]
    pub token: Option<TokenDetail>,
    #[serde(default)]
    pub pools: Vec<PoolInfo>,
    #[serde(default)]
    pub events: std::collections::HashMap<String, EventData>,
    #[serde(default)]
    pub risk: Option<RiskInfo>,
}

/// Token detail
#[derive(Debug, Clone, Deserialize)]
pub struct TokenDetail {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub mint: String,
    #[serde(default)]
    pub decimals: Option<u8>,
    #[serde(default)]
    pub image: Option<String>,
}

/// Pool info from token response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolInfo {
    #[serde(default)]
    pub pool_id: String,
    #[serde(default)]
    pub liquidity: Option<LiquidityInfo>,
    #[serde(default)]
    pub price: Option<PriceInfo>,
    #[serde(default)]
    pub token_supply: Option<f64>,
    #[serde(default)]
    pub lp_burn: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LiquidityInfo {
    #[serde(default)]
    pub quote: Option<f64>,
    #[serde(default)]
    pub usd: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriceInfo {
    #[serde(default)]
    pub quote: Option<f64>,
    #[serde(default)]
    pub usd: Option<f64>,
}

/// Event data for a timeframe
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventData {
    #[serde(default)]
    pub price_change_percentage: Option<f64>,
}

/// Risk assessment
#[derive(Debug, Clone, Deserialize)]
pub struct RiskInfo {
    #[serde(default)]
    pub score: Option<u32>,
    #[serde(default)]
    pub risks: Vec<RiskDetail>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RiskDetail {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub score: Option<i32>,
}

/// Search result
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub mint: String,
    #[serde(default)]
    pub image: Option<String>,
}
