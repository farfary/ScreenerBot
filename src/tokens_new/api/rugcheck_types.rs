/// Rugcheck API response types
use serde::{Deserialize, Serialize};

// ============================================================================
// API RESPONSE STRUCTURES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckResponse {
    pub mint: String,
    #[serde(rename = "tokenProgram")]
    pub token_program: Option<String>,
    #[serde(rename = "tokenType")]
    pub token_type: Option<String>,
    pub token: Option<RugcheckToken>,
    #[serde(rename = "tokenMeta")]
    pub token_meta: Option<RugcheckTokenMeta>,
    #[serde(rename = "mintAuthority")]
    pub mint_authority: Option<String>,
    #[serde(rename = "freezeAuthority")]
    pub freeze_authority: Option<String>,
    pub creator: Option<String>,
    #[serde(rename = "creatorBalance")]
    pub creator_balance: Option<i64>,
    #[serde(rename = "creatorTokens")]
    pub creator_tokens: Option<String>,
    pub score: Option<i32>,
    #[serde(rename = "score_normalised")]
    pub score_normalised: Option<i32>,
    pub rugged: Option<bool>,
    pub risks: Option<Vec<RugcheckRiskItem>>,
    pub markets: Option<Vec<RugcheckMarket>>,
    #[serde(rename = "totalMarketLiquidity")]
    pub total_market_liquidity: Option<f64>,
    #[serde(rename = "totalStableLiquidity")]
    pub total_stable_liquidity: Option<f64>,
    #[serde(rename = "totalLPProviders")]
    pub total_lp_providers: Option<i64>,
    #[serde(rename = "totalHolders")]
    pub total_holders: Option<i64>,
    #[serde(rename = "topHolders")]
    pub top_holders: Option<Vec<RugcheckTopHolder>>,
    #[serde(rename = "graphInsidersDetected")]
    pub graph_insiders_detected: Option<i64>,
    #[serde(rename = "transferFee")]
    pub transfer_fee: Option<RugcheckTransferFee>,
    #[serde(rename = "detectedAt")]
    pub detected_at: Option<String>,
    #[serde(rename = "analyzedAt")]
    pub analyzed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckToken {
    #[serde(rename = "mintAuthority")]
    pub mint_authority: Option<String>,
    pub supply: Option<u64>,
    pub decimals: Option<u8>,
    #[serde(rename = "isInitialized")]
    pub is_initialized: Option<bool>,
    #[serde(rename = "freezeAuthority")]
    pub freeze_authority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckTokenMeta {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub uri: Option<String>,
    pub mutable: Option<bool>,
    #[serde(rename = "updateAuthority")]
    pub update_authority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckRiskItem {
    pub name: String,
    pub value: String,
    pub description: String,
    pub score: i32,
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckMarket {
    pub pubkey: String,
    #[serde(rename = "marketType")]
    pub market_type: String,
    pub lp: Option<RugcheckLpInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckLpInfo {
    #[serde(rename = "lpLockedPct")]
    pub lp_locked_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckTopHolder {
    pub address: String,
    pub amount: u64,
    pub pct: f64,
    pub owner: Option<String>,
    pub insider: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckTransferFee {
    pub pct: Option<f64>,
    #[serde(rename = "maxAmount")]
    pub max_amount: Option<u64>,
    pub authority: Option<String>,
}
