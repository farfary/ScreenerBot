/// GeckoTerminal API response types
use serde::{Deserialize, Serialize};

// ============================================================================
// API RESPONSE STRUCTURES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalResponse {
    pub data: Vec<GeckoTerminalPoolData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalPoolData {
    pub id: String,
    #[serde(rename = "type")]
    pub pool_type: String,
    pub attributes: GeckoTerminalAttributes,
    pub relationships: GeckoTerminalRelationships,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalAttributes {
    pub base_token_price_usd: String,
    pub base_token_price_native_currency: String,
    pub base_token_price_quote_token: String,
    pub quote_token_price_usd: String,
    pub quote_token_price_native_currency: String,
    pub quote_token_price_base_token: String,
    pub address: String,
    pub name: String,
    pub pool_created_at: Option<String>,
    pub token_price_usd: Option<String>,
    pub fdv_usd: Option<String>,
    pub market_cap_usd: Option<String>,
    pub price_change_percentage: Option<GeckoTerminalPriceChange>,
    pub transactions: Option<GeckoTerminalTransactions>,
    pub volume_usd: Option<GeckoTerminalVolume>,
    pub reserve_in_usd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalPriceChange {
    pub m5: Option<String>,
    pub m15: Option<String>,
    pub m30: Option<String>,
    pub h1: Option<String>,
    pub h6: Option<String>,
    pub h24: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalTransactions {
    pub m5: Option<GeckoTerminalTxnPeriod>,
    pub m15: Option<GeckoTerminalTxnPeriod>,
    pub m30: Option<GeckoTerminalTxnPeriod>,
    pub h1: Option<GeckoTerminalTxnPeriod>,
    pub h6: Option<GeckoTerminalTxnPeriod>,
    pub h24: Option<GeckoTerminalTxnPeriod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalTxnPeriod {
    pub buys: Option<i64>,
    pub sells: Option<i64>,
    pub buyers: Option<i64>,
    pub sellers: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalVolume {
    pub m5: Option<String>,
    pub m15: Option<String>,
    pub m30: Option<String>,
    pub h1: Option<String>,
    pub h6: Option<String>,
    pub h24: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalRelationships {
    pub base_token: GeckoTerminalTokenRef,
    pub quote_token: GeckoTerminalTokenRef,
    pub dex: GeckoTerminalDexRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalTokenRef {
    pub data: GeckoTerminalTokenData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalTokenData {
    pub id: String,
    #[serde(rename = "type")]
    pub token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalDexRef {
    pub data: GeckoTerminalDexData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalDexData {
    pub id: String,
    #[serde(rename = "type")]
    pub dex_type: String,
}

// ============================================================================
// CONVERSION IMPLEMENTATION - SAME PATTERN AS DEXSCREENER
// ============================================================================

impl GeckoTerminalPoolData {
    pub fn to_pool(&self, mint: &str) -> crate::tokens_new::types::GeckoTerminalPool {
        use chrono::Utc;
        
        let attrs = &self.attributes;
        let rels = &self.relationships;

        let parse_f64 = |s: &str| s.parse::<f64>().ok();

        let price_change = attrs.price_change_percentage.as_ref();
        let transactions = attrs.transactions.as_ref();
        let volume = attrs.volume_usd.as_ref();

        crate::tokens_new::types::GeckoTerminalPool {
            mint: mint.to_string(),
            pool_address: attrs.address.clone(),
            pool_name: attrs.name.clone(),
            dex_id: rels.dex.data.id.clone(),
            base_token_id: rels.base_token.data.id.clone(),
            quote_token_id: rels.quote_token.data.id.clone(),
            base_token_price_usd: attrs.base_token_price_usd.clone(),
            base_token_price_native: attrs.base_token_price_native_currency.clone(),
            base_token_price_quote: attrs.base_token_price_quote_token.clone(),
            quote_token_price_usd: attrs.quote_token_price_usd.clone(),
            quote_token_price_native: attrs.quote_token_price_native_currency.clone(),
            quote_token_price_base: attrs.quote_token_price_base_token.clone(),
            token_price_usd: attrs.token_price_usd.clone().unwrap_or_else(|| attrs.base_token_price_usd.clone()),
            fdv_usd: attrs.fdv_usd.as_ref().and_then(|s| parse_f64(s)),
            market_cap_usd: attrs.market_cap_usd.as_ref().and_then(|s| parse_f64(s)),
            reserve_usd: attrs.reserve_in_usd.as_ref().and_then(|s| parse_f64(s)),
            volume_m5: volume.and_then(|v| v.m5.as_ref().and_then(|s| parse_f64(s))),
            volume_m15: volume.and_then(|v| v.m15.as_ref().and_then(|s| parse_f64(s))),
            volume_m30: volume.and_then(|v| v.m30.as_ref().and_then(|s| parse_f64(s))),
            volume_h1: volume.and_then(|v| v.h1.as_ref().and_then(|s| parse_f64(s))),
            volume_h6: volume.and_then(|v| v.h6.as_ref().and_then(|s| parse_f64(s))),
            volume_h24: volume.and_then(|v| v.h24.as_ref().and_then(|s| parse_f64(s))),
            price_change_m5: price_change.and_then(|pc| pc.m5.as_ref().and_then(|s| parse_f64(s))),
            price_change_m15: price_change.and_then(|pc| pc.m15.as_ref().and_then(|s| parse_f64(s))),
            price_change_m30: price_change.and_then(|pc| pc.m30.as_ref().and_then(|s| parse_f64(s))),
            price_change_h1: price_change.and_then(|pc| pc.h1.as_ref().and_then(|s| parse_f64(s))),
            price_change_h6: price_change.and_then(|pc| pc.h6.as_ref().and_then(|s| parse_f64(s))),
            price_change_h24: price_change.and_then(|pc| pc.h24.as_ref().and_then(|s| parse_f64(s))),
            txns_m5_buys: transactions.and_then(|t| t.m5.as_ref().and_then(|p| p.buys)),
            txns_m5_sells: transactions.and_then(|t| t.m5.as_ref().and_then(|p| p.sells)),
            txns_m5_buyers: transactions.and_then(|t| t.m5.as_ref().and_then(|p| p.buyers)),
            txns_m5_sellers: transactions.and_then(|t| t.m5.as_ref().and_then(|p| p.sellers)),
            txns_m15_buys: transactions.and_then(|t| t.m15.as_ref().and_then(|p| p.buys)),
            txns_m15_sells: transactions.and_then(|t| t.m15.as_ref().and_then(|p| p.sells)),
            txns_m15_buyers: transactions.and_then(|t| t.m15.as_ref().and_then(|p| p.buyers)),
            txns_m15_sellers: transactions.and_then(|t| t.m15.as_ref().and_then(|p| p.sellers)),
            txns_m30_buys: transactions.and_then(|t| t.m30.as_ref().and_then(|p| p.buys)),
            txns_m30_sells: transactions.and_then(|t| t.m30.as_ref().and_then(|p| p.sells)),
            txns_m30_buyers: transactions.and_then(|t| t.m30.as_ref().and_then(|p| p.buyers)),
            txns_m30_sellers: transactions.and_then(|t| t.m30.as_ref().and_then(|p| p.sellers)),
            txns_h1_buys: transactions.and_then(|t| t.h1.as_ref().and_then(|p| p.buys)),
            txns_h1_sells: transactions.and_then(|t| t.h1.as_ref().and_then(|p| p.sells)),
            txns_h1_buyers: transactions.and_then(|t| t.h1.as_ref().and_then(|p| p.buyers)),
            txns_h1_sellers: transactions.and_then(|t| t.h1.as_ref().and_then(|p| p.sellers)),
            txns_h6_buys: transactions.and_then(|t| t.h6.as_ref().and_then(|p| p.buys)),
            txns_h6_sells: transactions.and_then(|t| t.h6.as_ref().and_then(|p| p.sells)),
            txns_h6_buyers: transactions.and_then(|t| t.h6.as_ref().and_then(|p| p.buyers)),
            txns_h6_sellers: transactions.and_then(|t| t.h6.as_ref().and_then(|p| p.sellers)),
            txns_h24_buys: transactions.and_then(|t| t.h24.as_ref().and_then(|p| p.buys)),
            txns_h24_sells: transactions.and_then(|t| t.h24.as_ref().and_then(|p| p.sells)),
            txns_h24_buyers: transactions.and_then(|t| t.h24.as_ref().and_then(|p| p.buyers)),
            txns_h24_sellers: transactions.and_then(|t| t.h24.as_ref().and_then(|p| p.sellers)),
            pool_created_at: attrs.pool_created_at.clone(),
            fetched_at: Utc::now(),
        }
    }
}
