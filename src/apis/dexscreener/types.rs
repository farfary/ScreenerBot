/// DexScreener API response types
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::tokens::types::{SocialLink, WebsiteLink};

// ============================================================================
// POOL DATA STRUCTURE
// ============================================================================

/// DexScreener pool data - Used for API response parsing only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexScreenerPool {
    pub mint: String,
    pub pair_address: String,
    pub chain_id: String,
    pub dex_id: String,
    pub url: Option<String>,

    // Token info
    pub base_token_address: String,
    pub base_token_name: String,
    pub base_token_symbol: String,
    pub quote_token_address: String,
    pub quote_token_name: String,
    pub quote_token_symbol: String,

    // Prices (high precision as strings)
    pub price_native: String,
    pub price_usd: String,

    // Liquidity
    pub liquidity_usd: Option<f64>,
    pub liquidity_base: Option<f64>,
    pub liquidity_quote: Option<f64>,

    // Volume
    pub volume_m5: Option<f64>,
    pub volume_h1: Option<f64>,
    pub volume_h6: Option<f64>,
    pub volume_h24: Option<f64>,

    // Transactions
    pub txns_m5_buys: Option<i64>,
    pub txns_m5_sells: Option<i64>,
    pub txns_h1_buys: Option<i64>,
    pub txns_h1_sells: Option<i64>,
    pub txns_h6_buys: Option<i64>,
    pub txns_h6_sells: Option<i64>,
    pub txns_h24_buys: Option<i64>,
    pub txns_h24_sells: Option<i64>,

    // Price changes
    pub price_change_m5: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub price_change_h6: Option<f64>,
    pub price_change_h24: Option<f64>,

    // Market metrics
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,

    // Metadata
    pub pair_created_at: Option<i64>,
    pub labels: Vec<String>,

    // Info
    pub info_image_url: Option<String>,
    pub info_header: Option<String>,
    pub info_open_graph: Option<String>,
    pub info_websites: Vec<WebsiteLink>,
    pub info_socials: Vec<SocialLink>,

    pub fetched_at: DateTime<Utc>,
}

impl Default for DexScreenerPool {
    fn default() -> Self {
        Self {
            mint: String::new(),
            pair_address: String::new(),
            chain_id: String::new(),
            dex_id: String::new(),
            url: None,
            base_token_address: String::new(),
            base_token_name: String::new(),
            base_token_symbol: String::new(),
            quote_token_address: String::new(),
            quote_token_name: String::new(),
            quote_token_symbol: String::new(),
            price_native: String::new(),
            price_usd: String::new(),
            liquidity_usd: None,
            liquidity_base: None,
            liquidity_quote: None,
            volume_m5: None,
            volume_h1: None,
            volume_h6: None,
            volume_h24: None,
            txns_m5_buys: None,
            txns_m5_sells: None,
            txns_h1_buys: None,
            txns_h1_sells: None,
            txns_h6_buys: None,
            txns_h6_sells: None,
            txns_h24_buys: None,
            txns_h24_sells: None,
            price_change_m5: None,
            price_change_h1: None,
            price_change_h6: None,
            price_change_h24: None,
            fdv: None,
            market_cap: None,
            pair_created_at: None,
            labels: Vec::new(),
            info_image_url: None,
            info_header: None,
            info_open_graph: None,
            info_websites: Vec::new(),
            info_socials: Vec::new(),
            fetched_at: Utc::now(),
        }
    }
}

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
pub struct TransactionPeriod {
    pub buys: Option<i64>,
    pub sells: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct Transactions {
    pub m5: Option<TransactionPeriod>,
    pub h1: Option<TransactionPeriod>,
    pub h6: Option<TransactionPeriod>,
    pub h24: Option<TransactionPeriod>,
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
    #[serde(rename = "tokenAddress")]
    pub token_address: String,
    #[serde(rename = "chainId")]
    pub chain_id: Option<String>,
    pub url: Option<String>,
    pub icon: Option<String>,
    pub header: Option<String>,
    #[serde(rename = "openGraph")]
    pub open_graph: Option<String>,
    pub description: Option<String>,
    pub links: Option<Vec<serde_json::Value>>,
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
            if let Some(ref m5) = txns.m5 {
                pool.txns_m5_buys = m5.buys;
                pool.txns_m5_sells = m5.sells;
            }
            if let Some(ref h1) = txns.h1 {
                pool.txns_h1_buys = h1.buys;
                pool.txns_h1_sells = h1.sells;
            }
            if let Some(ref h6) = txns.h6 {
                pool.txns_h6_buys = h6.buys;
                pool.txns_h6_sells = h6.sells;
            }
            if let Some(ref h24) = txns.h24 {
                pool.txns_h24_buys = h24.buys;
                pool.txns_h24_sells = h24.sells;
            }
        }

        if let Some(ref pc) = self.price_change {
            pool.price_change_m5 = pc.m5;
            pool.price_change_h1 = pc.h1;
            pool.price_change_h6 = pc.h6;
            pool.price_change_h24 = pc.h24;
        }

        pool.fdv = self.fdv;
        pool.market_cap = self.market_cap;
        // Convert milliseconds to seconds for pair_created_at
        pool.pair_created_at = self.pair_created_at.map(|ms| ms / 1000);

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
