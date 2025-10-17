/// DeFiLlama API response types
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// DEFILLAMA PROTOCOLS RESPONSE
// ============================================================================

/// DeFiLlama protocol from /protocols
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefiLlamaProtocol {
    pub id: String,
    pub name: String,
    pub address: Option<String>,
    pub symbol: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub chain: Option<String>,  // Made optional - can be missing in some responses
    pub logo: Option<String>,
    #[serde(default)]
    pub chains: Option<Vec<String>>,
    pub category: Option<String>,
    #[serde(default)]
    pub tvl: Option<f64>,
}

// ============================================================================
// DEFILLAMA PRICE RESPONSE
// ============================================================================

/// DeFiLlama token price from /prices/current
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefiLlamaPriceResponse {
    pub coins: HashMap<String, DefiLlamaPrice>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefiLlamaPrice {
    pub decimals: u8,
    pub symbol: String,
    pub price: f64,
    pub timestamp: i64,
    pub confidence: f64,
}
