//! Rugcheck API response types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use crate::tokens::types::{SecurityRisk, TokenHolder};

// ============================================================================
// CUSTOM DESERIALIZERS - Handle API inconsistencies
// ============================================================================

/// Deserialize authority field that can be null, string, or account object
///
/// Rugcheck API returns authority fields in three formats:
/// 1. null - No authority
/// 2. "address_string" - Authority address (standard)
/// 3. {"lamports": ..., "owner": ..., ...} - Account info object (Token2022 tokens)
///
/// We extract the string address from all formats, falling back to the nested
/// token.mintAuthority/token.freezeAuthority fields when needed.
fn deserialize_optional_authority<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use serde_json::Value;

    let value = Option::<Value>::deserialize(deserializer)?;

    match value {
        None => Ok(None),
        Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(Value::Object(map)) => {
            // For account objects, we can't extract a meaningful address
            // The object contains lamports, owner, data, etc. but not the authority address
            // Return None and rely on fallback to token.* fields in the conversion logic
            if map.contains_key("lamports") && map.contains_key("owner") {
                Ok(None)
            } else {
                Err(Error::custom(format!(
                    "Unexpected object format for authority field: {:?}",
                    map.keys().collect::<Vec<_>>()
                )))
            }
        }
        Some(other) => Err(Error::custom(format!(
            "Expected null, string, or object for authority field, got: {}",
            other
        ))),
    }
}

// ============================================================================
// RUGCHECK DATA STRUCTURE
// ============================================================================

/// Rugcheck security data - Used for API response parsing only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugcheckInfo {
    pub mint: String,

    // Token program info
    pub token_program: Option<String>,
    pub token_type: Option<String>,

    // Token metadata
    pub token_name: Option<String>,
    pub token_symbol: Option<String>,
    pub token_decimals: Option<u8>,
    pub token_supply: Option<String>,
    pub token_uri: Option<String>,
    pub token_mutable: Option<bool>,
    pub token_update_authority: Option<String>,

    // Authorities
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,

    // Creator
    pub creator: Option<String>,
    pub creator_balance: Option<i64>,
    pub creator_tokens: Option<serde_json::Value>,

    // Scoring
    pub score: Option<i32>,
    pub score_normalised: Option<i32>,
    pub rugged: bool,

    // Risks
    pub risks: Vec<SecurityRisk>,

    // Market data
    pub total_markets: Option<i64>,
    pub total_market_liquidity: Option<f64>,
    pub total_stable_liquidity: Option<f64>,
    pub total_lp_providers: Option<i64>,

    // Holders
    pub total_holders: Option<i64>,
    pub top_holders: Vec<TokenHolder>,
    pub graph_insiders_detected: Option<i64>,

    // Transfer fee
    pub transfer_fee_pct: Option<f64>,
    pub transfer_fee_max_amount: Option<i64>,
    pub transfer_fee_authority: Option<String>,

    // Metadata
    pub detected_at: Option<String>,
    pub analyzed_at: Option<String>,

    pub fetched_at: DateTime<Utc>,
}

impl Default for RugcheckInfo {
    fn default() -> Self {
        Self {
            mint: String::new(),
            token_program: None,
            token_type: None,
            token_name: None,
            token_symbol: None,
            token_decimals: None,
            token_supply: None,
            token_uri: None,
            token_mutable: None,
            token_update_authority: None,
            mint_authority: None,
            freeze_authority: None,
            creator: None,
            creator_balance: None,
            creator_tokens: None,
            score: None,
            score_normalised: None,
            rugged: false,
            risks: Vec::new(),
            total_markets: None,
            total_market_liquidity: None,
            total_stable_liquidity: None,
            total_lp_providers: None,
            total_holders: None,
            top_holders: Vec::new(),
            graph_insiders_detected: None,
            transfer_fee_pct: None,
            transfer_fee_max_amount: None,
            transfer_fee_authority: None,
            detected_at: None,
            analyzed_at: None,
            fetched_at: Utc::now(),
        }
    }
}

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
    #[serde(
        rename = "mintAuthority",
        deserialize_with = "deserialize_optional_authority"
    )]
    pub mint_authority: Option<String>,
    #[serde(
        rename = "freezeAuthority",
        deserialize_with = "deserialize_optional_authority"
    )]
    pub freeze_authority: Option<String>,
    pub creator: Option<String>,
    #[serde(rename = "creatorBalance")]
    pub creator_balance: Option<i64>,
    #[serde(rename = "creatorTokens")]
    pub creator_tokens: Option<serde_json::Value>,
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

impl RugcheckInfo {
    /// Convert a raw Rugcheck API `RugcheckResponse` into the domain `RugcheckInfo`.
    ///
    /// This is a pure data mapping (no I/O, no rate limiting) so it can be reused
    /// by every source that yields the same raw report shape: the direct Rugcheck
    /// client AND the self-hosted screenerbot-data server (which caches and returns
    /// the byte-identical upstream JSON under its `report` field).
    ///
    /// Authority fields use a fallback strategy: prefer the top-level field, fall
    /// back to the nested `token.*` field, so we never miss authority data
    /// regardless of which response format the API used.
    pub fn from_response(api_response: RugcheckResponse) -> Self {
        let risks = api_response
            .risks
            .unwrap_or_default()
            .into_iter()
            .map(|r| SecurityRisk {
                name: r.name,
                value: r.value,
                description: r.description,
                score: r.score,
                level: r.level,
            })
            .collect();

        let top_holders = api_response
            .top_holders
            .unwrap_or_default()
            .into_iter()
            .map(|h| TokenHolder {
                address: h.address,
                amount: h.amount.to_string(),
                pct: h.pct,
                owner: h.owner,
                insider: h.insider.unwrap_or_default(),
            })
            .collect();

        let token_meta = api_response.token_meta;
        let token = api_response.token;
        let transfer_fee = api_response.transfer_fee;

        let mint_authority = api_response
            .mint_authority
            .or_else(|| token.as_ref().and_then(|t| t.mint_authority.clone()));

        let freeze_authority = api_response
            .freeze_authority
            .or_else(|| token.as_ref().and_then(|t| t.freeze_authority.clone()));

        RugcheckInfo {
            mint: api_response.mint,
            token_program: api_response.token_program,
            token_type: api_response.token_type,
            token_name: token_meta.as_ref().and_then(|t| t.name.clone()),
            token_symbol: token_meta.as_ref().and_then(|t| t.symbol.clone()),
            token_decimals: token.as_ref().and_then(|t| t.decimals),
            token_supply: token.as_ref().and_then(|t| t.supply.map(|s| s.to_string())),
            token_uri: token_meta.as_ref().and_then(|t| t.uri.clone()),
            token_mutable: token_meta.as_ref().and_then(|t| t.mutable),
            token_update_authority: token_meta.as_ref().and_then(|t| t.update_authority.clone()),
            mint_authority,
            freeze_authority,
            creator: api_response.creator,
            creator_balance: api_response.creator_balance,
            creator_tokens: api_response.creator_tokens,
            score: api_response.score,
            score_normalised: api_response.score_normalised,
            rugged: api_response.rugged.unwrap_or_default(),
            risks,
            total_markets: None, // Market data not included in API response
            total_market_liquidity: api_response.total_market_liquidity,
            total_stable_liquidity: api_response.total_stable_liquidity,
            total_lp_providers: api_response.total_lp_providers,
            total_holders: api_response.total_holders,
            top_holders,
            graph_insiders_detected: api_response.graph_insiders_detected,
            transfer_fee_pct: transfer_fee.as_ref().and_then(|t| t.pct),
            transfer_fee_max_amount: transfer_fee
                .as_ref()
                .and_then(|t| t.max_amount.map(|a| a as i64)),
            transfer_fee_authority: transfer_fee.and_then(|t| t.authority),
            detected_at: api_response.detected_at,
            analyzed_at: api_response.analyzed_at,
            fetched_at: Utc::now(),
        }
    }
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
    pub metadata: Option<RugcheckTokenMetadata>,
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
