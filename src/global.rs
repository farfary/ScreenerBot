use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::Path;
use std::collections::HashSet;
use once_cell::sync::Lazy;
use std::sync::RwLock;

pub static LIST_MINTS: Lazy<RwLock<HashSet<String>>> = Lazy::new(|| RwLock::new(HashSet::new()));

pub static LIST_TOKENS: Lazy<RwLock<Vec<Token>>> = Lazy::new(|| RwLock::new(vec![]));

/// Represents the runtime configuration loaded from configs.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configs {
    pub main_wallet_private: String,
    pub rpc_url: String,
    pub rpc_fallbacks: Vec<String>,
}

/// Reads the configs.json file from the project root and returns a Configs object
pub fn read_configs<P: AsRef<Path>>(path: P) -> Result<Configs, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(path)?;
    let configs: Configs = serde_json::from_str(&data)?;
    Ok(configs)
}

/// Represents a liquidity pool for a token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pool {
    pub address: String,
    pub dex: String,
    pub base_token: String,
    pub quote_token: String,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub fee: Option<f64>,
    pub url: Option<String>,
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
}

/// Represents transaction data for different time periods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TxnPeriod {
    pub buys: Option<i64>,
    pub sells: Option<i64>,
}

/// Represents transaction statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TxnStats {
    pub m5: Option<TxnPeriod>,
    pub h1: Option<TxnPeriod>,
    pub h6: Option<TxnPeriod>,
    pub h24: Option<TxnPeriod>,
}

/// Represents volume data for different time periods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolumeStats {
    pub m5: Option<f64>,
    pub h1: Option<f64>,
    pub h6: Option<f64>,
    pub h24: Option<f64>,
}

/// Represents price change data for different time periods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceChangeStats {
    pub m5: Option<f64>,
    pub h1: Option<f64>,
    pub h6: Option<f64>,
    pub h24: Option<f64>,
}

/// Represents liquidity information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidityInfo {
    pub usd: Option<f64>,
    pub base: Option<f64>,
    pub quote: Option<f64>,
}

/// Represents social media links
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocialLink {
    pub link_type: String, // "twitter", "telegram", etc.
    pub url: String,
}

/// Represents website links
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebsiteLink {
    pub label: Option<String>,
    pub url: String,
}

/// Represents token info from DexScreener
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenInfo {
    pub image_url: Option<String>,
    pub header: Option<String>,
    pub open_graph: Option<String>,
    pub websites: Vec<WebsiteLink>,
    pub socials: Vec<SocialLink>,
}

/// Represents boost information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoostInfo {
    pub active: Option<i64>,
}

/// Represents a cryptocurrency token with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub chain: String,

    // Existing fields we need to keep
    pub logo_url: Option<String>,
    pub coingecko_id: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub is_verified: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,

    // Price data from various sources
    pub price_dexscreener_sol: Option<f64>,
    pub price_dexscreener_usd: Option<f64>,
    pub price_geckoterminal_sol: Option<f64>,
    pub price_geckoterminal_usd: Option<f64>,
    pub price_raydium_sol: Option<f64>,
    pub price_raydium_usd: Option<f64>,
    pub price_pool_sol: Option<f64>,
    pub price_pool_usd: Option<f64>,
    pub pools: Vec<Pool>,

    // New fields from DexScreener API
    pub dex_id: Option<String>,
    pub pair_address: Option<String>,
    pub pair_url: Option<String>,
    pub labels: Vec<String>,
    pub fdv: Option<f64>, // Fully Diluted Valuation
    pub market_cap: Option<f64>,
    pub txns: Option<TxnStats>,
    pub volume: Option<VolumeStats>,
    pub price_change: Option<PriceChangeStats>,
    pub liquidity: Option<LiquidityInfo>,
    pub info: Option<TokenInfo>,
    pub boosts: Option<BoostInfo>,
}
