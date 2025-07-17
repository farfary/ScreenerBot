use serde::{ Deserialize, Serialize };

/// Pool information from various DEX protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: String,
    pub pool_type: PoolType,
    pub reserve_0: u64,
    pub reserve_1: u64,
    pub token_0: String,
    pub token_1: String,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub fee_tier: Option<f64>,
    pub last_updated: u64, // Unix timestamp
}

/// Supported DEX pool types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PoolType {
    Raydium,
    PumpFun,
    Meteora,
    Orca,
    Serum,
    Unknown(String),
}

impl std::fmt::Display for PoolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolType::Raydium => write!(f, "Raydium"),
            PoolType::PumpFun => write!(f, "PumpFun"),
            PoolType::Meteora => write!(f, "Meteora"),
            PoolType::Orca => write!(f, "Orca"),
            PoolType::Serum => write!(f, "Serum"),
            PoolType::Unknown(name) => write!(f, "Unknown({})", name),
        }
    }
}

impl From<String> for PoolType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "raydium" => PoolType::Raydium,
            "pumpfun" => PoolType::PumpFun,
            "meteora" => PoolType::Meteora,
            "orca" => PoolType::Orca,
            "serum" => PoolType::Serum,
            _ => PoolType::Unknown(s),
        }
    }
}

impl From<&str> for PoolType {
    fn from(s: &str) -> Self {
        PoolType::from(s.to_string())
    }
}
