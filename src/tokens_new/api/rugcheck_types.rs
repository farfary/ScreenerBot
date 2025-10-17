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
pub struct RugcheckTopHolder {
    pub address: String,
    pub amount: u64,
    pub decimals: Option<u8>,
    pub pct: f64,
    #[serde(rename = "uiAmount")]
    pub ui_amount: Option<f64>,
    #[serde(rename = "uiAmountString")]
    pub ui_amount_string: Option<String>,
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

// ============================================================================
// Stats Endpoints Response Types
// ============================================================================

/// New token from /v1/stats/new_tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckNewToken {
    pub mint: String,
    pub decimals: u8,
    pub symbol: String,
    pub creator: String,
    #[serde(rename = "mintAuthority")]
    pub mint_authority: String,
    #[serde(rename = "freezeAuthority")]
    pub freeze_authority: String,
    pub program: String,
    #[serde(rename = "createAt")]
    pub create_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(default)]
    pub events: Option<serde_json::Value>,
}

/// Token metadata for recent tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckTokenMetadata {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mutable: bool,
    #[serde(rename = "updateAuthority")]
    pub update_authority: String,
}

/// Recent token from /v1/stats/recent (most viewed)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckRecentToken {
    pub mint: String,
    pub metadata: RugcheckTokenMetadata,
    pub user_visits: u64,
    pub visits: u64,
    pub score: i32,
}

/// Trending token from /v1/stats/trending
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckTrendingToken {
    pub mint: String,
    pub vote_count: u64,
    pub up_count: u64,
}

/// Verified token from /v1/stats/verified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckVerifiedToken {
    pub mint: String,
    pub payer: String,
    pub name: String,
    pub symbol: String,
    pub description: String,
    pub jup_verified: bool,
    pub jup_strict: bool,
    #[serde(default)]
    pub links: Option<serde_json::Value>,
}
