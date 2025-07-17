use serde::{ Deserialize, Serialize };

/// Token price information from various sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub address: String,
    pub price_usd: f64,
    pub price_sol: Option<f64>,
    pub market_cap: Option<f64>,
    pub volume_24h: f64,
    pub liquidity_usd: f64,
    pub timestamp: u64, // Unix timestamp in seconds
    pub source: PriceSource,
    pub is_cached: bool,
}

/// Token information including metadata and pricing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Option<u64>,
    pub pools: Vec<crate::market_data::models::PoolInfo>,
    pub price: Option<TokenPrice>,
    pub last_updated: u64, // Unix timestamp in seconds
}

/// Sources of price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriceSource {
    GeckoTerminal,
    PoolCalculation,
    Cache,
    DynamicPricing,
}

impl std::fmt::Display for PriceSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PriceSource::GeckoTerminal => write!(f, "GeckoTerminal"),
            PriceSource::PoolCalculation => write!(f, "PoolCalculation"),
            PriceSource::Cache => write!(f, "Cache"),
            PriceSource::DynamicPricing => write!(f, "DynamicPricing"),
        }
    }
}
