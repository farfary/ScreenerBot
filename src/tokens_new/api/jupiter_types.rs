/// Jupiter API response types
use serde::{Deserialize, Serialize};

// ============================================================================
// JUPITER TOKEN RESPONSE (comprehensive)
// ============================================================================

/// Jupiter token from all discovery endpoints
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterToken {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub decimals: u8,
    #[serde(default)]
    pub dev: Option<String>,
    #[serde(default)]
    pub circ_supply: Option<f64>,
    #[serde(default)]
    pub total_supply: Option<f64>,
    #[serde(default)]
    pub token_program: Option<String>,
    #[serde(default)]
    pub first_pool: Option<JupiterFirstPool>,
    #[serde(default)]
    pub holder_count: Option<i64>,
    #[serde(default)]
    pub audit: Option<JupiterAudit>,
    #[serde(default)]
    pub apy: Option<JupiterApy>,
    #[serde(default)]
    pub organic_score: Option<f64>,
    #[serde(default)]
    pub organic_score_label: Option<String>,
    #[serde(default)]
    pub is_verified: Option<bool>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub fdv: Option<f64>,
    #[serde(default)]
    pub mcap: Option<f64>,
    #[serde(default)]
    pub usd_price: Option<f64>,
    #[serde(default)]
    pub price_block_id: Option<i64>,
    #[serde(default)]
    pub liquidity: Option<f64>,
    #[serde(default)]
    pub stats5m: Option<JupiterStats>,
    #[serde(default)]
    pub stats1h: Option<JupiterStats>,
    #[serde(default)]
    pub stats6h: Option<JupiterStats>,
    #[serde(default)]
    pub stats24h: Option<JupiterStats>,
    #[serde(default)]
    pub ct_likes: Option<i64>,
    #[serde(default)]
    pub smart_ct_likes: Option<i64>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterFirstPool {
    pub id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterAudit {
    #[serde(default)]
    pub is_sus: Option<bool>,
    #[serde(default)]
    pub mint_authority_disabled: Option<bool>,
    #[serde(default)]
    pub freeze_authority_disabled: Option<bool>,
    #[serde(default)]
    pub top_holders_percentage: Option<f64>,
    #[serde(default)]
    pub dev_balance_percentage: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterApy {
    #[serde(default)]
    pub jup_earn: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterStats {
    #[serde(default)]
    pub holder_change: Option<f64>, // Can be float like -95.76719576719576
    #[serde(default)]
    pub price_change: Option<f64>,
    #[serde(default)]
    pub liquidity_change: Option<f64>,
    #[serde(default)]
    pub volume_change: Option<f64>,
    #[serde(default)]
    pub buy_volume: Option<f64>,
    #[serde(default)]
    pub sell_volume: Option<f64>,
    #[serde(default)]
    pub buy_organic_volume: Option<f64>,
    #[serde(default)]
    pub sell_organic_volume: Option<f64>,
    #[serde(default)]
    pub num_buys: Option<i64>,
    #[serde(default)]
    pub num_sells: Option<i64>,
    #[serde(default)]
    pub num_traders: Option<i64>,
    #[serde(default)]
    pub num_organic_buyers: Option<i64>,
    #[serde(default)]
    pub num_net_buyers: Option<i64>,
}
