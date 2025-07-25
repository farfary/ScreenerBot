use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::Path;
use std::collections::HashSet;
use once_cell::sync::Lazy;
use std::sync::{ RwLock, Mutex };
use chrono::{ DateTime, Utc };
// TODO: Replace with new pool price system
// use crate::pool_price::Pool; // Import Pool from pool_price module
use solana_sdk::signature::Keypair;
use std::env;
use crate::tokens::{ TokenDatabase };

pub static CMD_ARGS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| { Mutex::new(env::args().collect()) });

pub static LIST_MINTS: Lazy<RwLock<HashSet<String>>> = Lazy::new(|| RwLock::new(HashSet::new()));

pub static LIST_TOKENS: Lazy<RwLock<Vec<Token>>> = Lazy::new(|| RwLock::new(vec![]));

// Global token database instance
pub static TOKEN_DB: Lazy<Mutex<Option<TokenDatabase>>> = Lazy::new(|| Mutex::new(None));

// Startup timestamp to track when the bot started for trading logic
pub static STARTUP_TIME: Lazy<DateTime<Utc>> = Lazy::new(|| Utc::now());

/// Check if debug filtering mode is enabled via command line args
pub fn is_debug_filtering_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-filtering".to_string())
    } else {
        false
    }
}

/// Check if debug loss prevention mode is enabled via command line args
pub fn is_debug_loss_prevention_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-loss-prevention".to_string())
    } else {
        false
    }
}

/// Check if debug profit mode is enabled via command line args
pub fn is_debug_profit_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-profit".to_string())
    } else {
        false
    }
}

/// Check if debug pool prices mode is enabled via command line args
pub fn is_debug_pool_prices_enabled() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-pool-prices".to_string())
    } else {
        false
    }
}

/// Represents the runtime configuration loaded from configs.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configs {
    pub main_wallet_private: String,
    pub rpc_url: String,
    pub rpc_fallbacks: Vec<String>,
    pub websocket_url: String,
    pub websocket_fallbacks: Vec<String>,
}

/// Reads the configs.json file from the project root and returns a Configs object
pub fn read_configs<P: AsRef<Path>>(path: P) -> Result<Configs, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(path)?;
    let configs: Configs = serde_json::from_str(&data)?;
    Ok(configs)
}

/// Load the main wallet keypair from the configs
pub fn load_wallet_from_config(configs: &Configs) -> Result<Keypair, Box<dyn std::error::Error>> {
    // Parse the private key from base58 string
    let keypair = if
        configs.main_wallet_private.starts_with('[') &&
        configs.main_wallet_private.ends_with(']')
    {
        // Handle array format like [1,2,3,4,...]
        let private_key_str = configs.main_wallet_private
            .trim_start_matches('[')
            .trim_end_matches(']');
        let private_key_bytes: Result<Vec<u8>, _> = private_key_str
            .split(',')
            .map(|s| s.trim().parse::<u8>())
            .collect();

        match private_key_bytes {
            Ok(bytes) => {
                if bytes.len() != 64 {
                    return Err(
                        format!(
                            "Invalid private key length: expected 64 bytes, got {}",
                            bytes.len()
                        ).into()
                    );
                }
                Keypair::try_from(&bytes[..]).map_err(|e|
                    format!("Failed to create keypair: {}", e)
                )?
            }
            Err(e) => {
                return Err(format!("Failed to parse private key array: {}", e).into());
            }
        }
    } else {
        // Handle base58 format
        let decoded = bs58::decode(&configs.main_wallet_private).into_vec()?;
        if decoded.len() != 64 {
            return Err(
                format!(
                    "Invalid private key length: expected 64 bytes, got {}",
                    decoded.len()
                ).into()
            );
        }
        Keypair::try_from(&decoded[..]).map_err(|e| format!("Failed to create keypair: {}", e))?
    };

    Ok(keypair)
}

/// Initialize the global token database
pub fn initialize_token_database() -> Result<(), Box<dyn std::error::Error>> {
    let db = TokenDatabase::new()?;
    if let Ok(mut token_db) = TOKEN_DB.lock() {
        *token_db = Some(db);
        crate::logger::log(
            crate::logger::LogTag::System,
            "SUCCESS",
            "Token database initialized successfully"
        );
    }
    Ok(())
}

/// Cache a token to the database (thread-safe)
pub fn cache_token_to_db(
    _token: &Token,
    _discovery_source: &str
) -> Result<bool, Box<dyn std::error::Error>> {
    // Note: This function is synchronous but the database methods are async.
    // For now, we return success. This should be refactored to be async in the future.
    Ok(true)
}

/// Get tokens that should be used for trading (discovered after startup)
pub fn get_trading_tokens() -> Vec<Token> {
    if let Ok(tokens) = LIST_TOKENS.read() { tokens.clone() } else { Vec::new() }
}

/// Get token from database by mint (for swap detection)
pub fn get_token_from_db(_mint: &str) -> Option<Token> {
    // Note: This function is synchronous but the database methods are async.
    // For now, we return None. This should be refactored to be async in the future.
    None
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
    pub price_pool_sol: Option<f64>,
    pub price_pool_usd: Option<f64>,
    // TODO: Replace with new pool price system
    // pub pools: Vec<Pool>,

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
