/// DexScreener API response types
use crate::tokens_new::types::DexScreenerPool;
use serde::{Deserialize, Serialize};

// ============================================================================
// API RESPONSE STRUCTURES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PairResponse {
    pub pair: Option<DexScreenerPairRaw>,
}

#[derive(Debug, Deserialize)]
pub struct PairsResponse {
    pub pairs: Vec<DexScreenerPairRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DexScreenerPairRaw {
    pub chain_id: Option<String>,
    pub dex_id: Option<String>,
    pub url: Option<String>,
    pub pair_address: Option<String>,
    pub base_token: Option<TokenInfo>,
    pub quote_token: Option<TokenInfo>,
    pub price_native: Option<String>,
    pub price_usd: Option<String>,
    pub txns: Option<Transactions>,
    pub volume: Option<VolumeData>,
    pub price_change: Option<PriceChanges>,
    pub liquidity: Option<LiquidityData>,
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,
    pub pair_created_at: Option<i64>,
    pub info: Option<PairInfo>,
    pub boosts: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenInfo {
    pub address: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transactions {
    pub m5: Option<serde_json::Value>,
    pub h1: Option<serde_json::Value>,
    pub h24: Option<serde_json::Value>,
    pub m5_buys: Option<i64>,
    pub m5_sells: Option<i64>,
    pub h1_buys: Option<i64>,
    pub h1_sells: Option<i64>,
    pub h6_buys: Option<i64>,
    pub h6_sells: Option<i64>,
    pub h24_buys: Option<i64>,
    pub h24_sells: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeData {
    pub m5: Option<f64>,
    pub h1: Option<f64>,
    pub h6: Option<f64>,
    pub h24: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceChanges {
    pub m5: Option<f64>,
    pub h1: Option<f64>,
    pub h6: Option<f64>,
    pub h24: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityData {
    pub usd: Option<f64>,
    pub base: Option<f64>,
    pub quote: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairInfo {
    pub image_url: Option<String>,
    pub header: Option<String>,
    pub open_graph: Option<String>,
    pub websites: Option<Vec<serde_json::Value>>,
    pub socials: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenProfile {
    pub address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub header_url: Option<String>,
    pub chain_id: Option<String>,
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub discord: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenOrder {
    pub token_address: String,
    pub order_type: String,
    pub status: String,
    pub amount: f64,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChainInfo {
    pub id: String,
    pub name: String,
}

// ============================================================================
// BOOST/DISCOVERY RESPONSE TYPES
// ============================================================================

/// Token boost from /token-boosts/latest/v1
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBoostLatest {
    pub url: String,
    pub chain_id: String,
    pub token_address: String,
    pub open_graph: Option<String>,
    pub total_amount: i64,
    pub amount: i64,
}

/// Token boost from /token-boosts/top/v1
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenBoostTop {
    pub url: String,
    pub chain_id: String,
    pub token_address: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub header: Option<String>,
    pub open_graph: Option<String>,
    pub links: Option<Vec<serde_json::Value>>,
    pub total_amount: i64,
}

// ============================================================================
// CONVERSION IMPLEMENTATION - SAME PATTERN AS GECKOTERMINAL
// ============================================================================

impl DexScreenerPairRaw {
    pub fn to_pool(&self) -> DexScreenerPool {
        let mut pool = DexScreenerPool::default();

        if let Some(ref base) = self.base_token {
            pool.base_token_address = base.address.clone().unwrap_or_default();
            pool.base_token_name = base.name.clone().unwrap_or_default();
            pool.base_token_symbol = base.symbol.clone().unwrap_or_default();
        }

        if let Some(ref quote) = self.quote_token {
            pool.quote_token_address = quote.address.clone().unwrap_or_default();
            pool.quote_token_name = quote.name.clone().unwrap_or_default();
            pool.quote_token_symbol = quote.symbol.clone().unwrap_or_default();
        }

        pool.chain_id = self.chain_id.clone().unwrap_or_default();
        pool.dex_id = self.dex_id.clone().unwrap_or_default();
        pool.pair_address = self.pair_address.clone().unwrap_or_default();
        pool.url = self.url.clone();
        pool.price_native = self.price_native.clone().unwrap_or_default();
        pool.price_usd = self.price_usd.clone().unwrap_or_default();

        if let Some(ref liquidity) = self.liquidity {
            pool.liquidity_usd = liquidity.usd;
            pool.liquidity_base = liquidity.base;
            pool.liquidity_quote = liquidity.quote;
        }

        if let Some(ref volume) = self.volume {
            pool.volume_m5 = volume.m5;
            pool.volume_h1 = volume.h1;
            pool.volume_h6 = volume.h6;
            pool.volume_h24 = volume.h24;
        }

        if let Some(ref txns) = self.txns {
            pool.txns_m5_buys = txns.m5_buys;
            pool.txns_m5_sells = txns.m5_sells;
            pool.txns_h1_buys = txns.h1_buys;
            pool.txns_h1_sells = txns.h1_sells;
            pool.txns_h6_buys = txns.h6_buys;
            pool.txns_h6_sells = txns.h6_sells;
            pool.txns_h24_buys = txns.h24_buys;
            pool.txns_h24_sells = txns.h24_sells;
        }

        if let Some(ref pc) = self.price_change {
            pool.price_change_m5 = pc.m5;
            pool.price_change_h1 = pc.h1;
            pool.price_change_h6 = pc.h6;
            pool.price_change_h24 = pc.h24;
        }

        pool.fdv = self.fdv;
        pool.market_cap = self.market_cap;
        pool.pair_created_at = self.pair_created_at;

        if let Some(ref info) = self.info {
            pool.info_image_url = info.image_url.clone();
            pool.info_header = info.header.clone();
            pool.info_open_graph = info.open_graph.clone();

            pool.info_websites = info
                .websites
                .as_ref()
                .and_then(|v| serde_json::from_value(serde_json::Value::Array(v.clone())).ok())
                .unwrap_or_default();
            pool.info_socials = info
                .socials
                .as_ref()
                .and_then(|v| serde_json::from_value(serde_json::Value::Array(v.clone())).ok())
                .unwrap_or_default();
        }

        pool
    }
}
