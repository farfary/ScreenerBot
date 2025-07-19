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

/// Represents a cryptocurrency token with full details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub chain: String,
    pub logo_url: Option<String>,
    pub coingecko_id: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub is_verified: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub price_dexscreener_sol: Option<f64>,
    pub price_dexscreener_usd: Option<f64>,
    pub price_geckoterminal_sol: Option<f64>,
    pub price_geckoterminal_usd: Option<f64>,
    pub price_raydium_sol: Option<f64>,
    pub price_raydium_usd: Option<f64>,
    pub price_pool_sol: Option<f64>,
    pub price_pool_usd: Option<f64>,
    pub pools: Vec<Pool>,
}
