use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub supply: u64,
    pub market_cap: Option<f64>,
    pub price: Option<f64>,
    pub volume_24h: Option<f64>,
    pub liquidity: Option<f64>,
    pub pool_address: Option<String>,
    pub discovered_at: chrono::DateTime<chrono::Utc>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryStats {
    pub total_tokens_discovered: u64,
    pub active_tokens: u64,
    pub last_discovery_run: chrono::DateTime<chrono::Utc>,
    pub discovery_rate_per_hour: f64,
}
