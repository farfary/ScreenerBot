use crate::prelude::*;
use crate::shutdown;
use crate::rate_limiter::{
    RateLimitedRequest,
    DEXSCREENER_LIMITER,
    RUGCHECK_LIMITER,
    GECKOTERMINAL_LIMITER,
};

use tokio::sync::RwLock;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::atomic::Ordering;
use reqwest::Client;
use colored::Colorize;
use serde::{ Serialize, Deserialize };
use tokio::{ fs, io::AsyncReadExt, io::AsyncWriteExt };
use std::collections::{ VecDeque, HashMap };

// ────────────── CONFIGURATION ──────────────
// Cache files
const TOKEN_CACHE_FILE: &str = ".tokens_cache.json";
const RUGCHECK_CACHE_FILE: &str = ".rugcheck_cache.json";
const DEXSCREENER_CACHE_FILE: &str = ".dexscreener_cache.json";

// Configuration constants - modify these to change behavior
const ENABLE_RUGCHECK: bool = false; // Set to false to disable rugcheck entirely
const RUGCHECK_CACHE_DAYS: u64 = 7; // Cache rugcheck data for N days (default: 7)
const DEXSCREENER_CACHE_MINUTES: u64 = 30; // Cache dexscreener data for N minutes (default: 30)

// Cache structure for rugcheck data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugCheckCache {
    pub data: HashMap<String, RugCheckData>,
    pub last_updated: u64,
}

// Cache structure for dexscreener data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexScreenerCache {
    pub data: HashMap<String, DexScreenerData>,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexScreenerData {
    pub dex_id: String,
    pub url: String,
    pub pair_address: String,
    pub labels: Vec<String>,
    pub quote_address: String,
    pub quote_name: String,
    pub quote_symbol: String,
    pub price_native: String,
    pub price_usd: String,
    pub volume_usd: String,
    pub fdv_usd: String,
    pub image_url: String,
    pub txns: Txns,
    pub volume: Volume,
    pub price_change: PriceChange,
    pub liquidity: Liquidity,
    pub pair_created_at: u64,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxnCount {
    pub buys: u64,
    pub sells: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Txns {
    pub m5: TxnCount,
    pub h1: TxnCount,
    pub h6: TxnCount,
    pub h24: TxnCount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub m5: f64,
    pub h1: f64,
    pub h6: f64,
    pub h24: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceChange {
    pub m5: f64,
    pub h1: f64,
    pub h6: f64,
    pub h24: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Liquidity {
    pub usd: f64,
    pub base: f64,
    pub quote: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugCheckRisk {
    pub name: String,
    pub description: String,
    pub level: String,
    pub score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugCheckData {
    pub score: i32,
    pub score_normalised: i32,
    pub rugged: bool,
    pub total_supply: u64,
    pub creator_balance: u64,
    pub total_holders: u64,
    pub total_market_liquidity: f64,
    pub risks: Vec<RugCheckRisk>,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub transfer_fee_pct: f64,
    pub checked_at: Option<u64>,
}

impl Default for RugCheckData {
    fn default() -> Self {
        Self {
            score: 0,
            score_normalised: 0,
            rugged: false,
            total_supply: 0,
            creator_balance: 0,
            total_holders: 0,
            total_market_liquidity: 0.0,
            risks: Vec::new(),
            mint_authority: None,
            freeze_authority: None,
            transfer_fee_pct: 0.0,
            checked_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub mint: String,
    pub balance: String,
    pub ata_pubkey: String,
    pub program_id: String,
    pub name: String,
    pub symbol: String,
    pub dex_id: String,
    pub url: String,
    pub pair_address: String,
    pub labels: Vec<String>,
    pub quote_address: String,
    pub quote_name: String,
    pub quote_symbol: String,
    pub price_native: String,
    pub price_usd: String,
    pub last_price_usd: String,
    pub volume_usd: String,
    pub fdv_usd: String,
    pub image_url: String,
    pub txns: Txns,
    pub volume: Volume,
    pub price_change: PriceChange,
    pub liquidity: Liquidity,
    pub pair_created_at: u64,
    pub rug_check: RugCheckData,
}

// ────────────── CACHE MANAGEMENT ──────────────

/// Load rugcheck cache from disk
async fn load_rugcheck_cache(debug: bool) -> RugCheckCache {
    if let Ok(data) = fs::read(RUGCHECK_CACHE_FILE).await {
        if let Ok(cache) = serde_json::from_slice::<RugCheckCache>(&data) {
            if debug {
                println!("📥 Loaded {} rugcheck entries from cache", cache.data.len());
            }
            return cache;
        }
    }

    if debug {
        println!("📥 No rugcheck cache found, starting fresh");
    }

    RugCheckCache {
        data: HashMap::new(),
        last_updated: 0,
    }
}

/// Save rugcheck cache to disk
async fn save_rugcheck_cache(cache: &RugCheckCache, debug: bool) {
    if let Ok(data) = serde_json::to_vec_pretty(cache) {
        if let Err(e) = fs::write(RUGCHECK_CACHE_FILE, data).await {
            if debug {
                println!("❌ Failed to save rugcheck cache: {}", e);
            }
        } else if debug {
            println!("💾 Saved {} rugcheck entries to cache", cache.data.len());
        }
    }
}

/// Load dexscreener cache from disk
async fn load_dexscreener_cache(debug: bool) -> DexScreenerCache {
    if let Ok(data) = fs::read(DEXSCREENER_CACHE_FILE).await {
        if let Ok(cache) = serde_json::from_slice::<DexScreenerCache>(&data) {
            if debug {
                println!("📥 Loaded {} dexscreener entries from cache", cache.data.len());
            }
            return cache;
        }
    }

    if debug {
        println!("📥 No dexscreener cache found, starting fresh");
    }

    DexScreenerCache {
        data: HashMap::new(),
        last_updated: 0,
    }
}

/// Save dexscreener cache to disk
async fn save_dexscreener_cache(cache: &DexScreenerCache, debug: bool) {
    if let Ok(data) = serde_json::to_vec_pretty(cache) {
        if let Err(e) = fs::write(DEXSCREENER_CACHE_FILE, data).await {
            if debug {
                println!("❌ Failed to save dexscreener cache: {}", e);
            }
        } else if debug {
            println!("💾 Saved {} dexscreener entries to cache", cache.data.len());
        }
    }
}

/// Check if rugcheck data is still valid (configurable cache period)
fn is_rugcheck_data_valid(data: &RugCheckData) -> bool {
    if let Some(checked_at) = data.checked_at {
        let now = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        return now - checked_at < RUGCHECK_CACHE_DAYS * 24 * 3600;
    }
    false
}

/// Check if dexscreener data is still valid (configurable cache period)
fn is_dexscreener_data_valid(data: &DexScreenerData) -> bool {
    let now = std::time::SystemTime
        ::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now - data.last_updated < DEXSCREENER_CACHE_MINUTES * 60
}

// ────────────── GLOBAL STATIC ──────────────
pub static TOKENS: Lazy<RwLock<Vec<Token>>> = Lazy::new(|| {
    RwLock::new(Vec::new()) // will load below
});

// ────────────── RUG CHECK FUNCTIONS ──────────────
async fn fetch_rug_check_data(
    client: &Client,
    mint: &str,
    debug: bool,
    rugcheck_cache: &mut RugCheckCache
) -> Option<RugCheckData> {
    // Check cache first
    if let Some(cached_data) = rugcheck_cache.data.get(mint) {
        if is_rugcheck_data_valid(cached_data) {
            if debug {
                println!("📋 [RugCheck] Using cached data for: {}", mint);
            }
            return Some(cached_data.clone());
        }
    }

    let url = format!("https://api.rugcheck.xyz/v1/tokens/{}/report", mint);

    if debug {
        println!("🔍 [RugCheck] Fetching fresh report for: {}", mint);
    }

    match client.get(&url).send().await {
        Ok(resp) => {
            match resp.json::<Value>().await {
                Ok(json) => {
                    let mut risks = Vec::new();

                    if let Some(risks_array) = json["risks"].as_array() {
                        for risk in risks_array {
                            risks.push(RugCheckRisk {
                                name: risk["name"].as_str().unwrap_or_default().to_string(),
                                description: risk["description"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                level: risk["level"].as_str().unwrap_or_default().to_string(),
                                score: risk["score"].as_i64().unwrap_or(0) as i32,
                            });
                        }
                    }

                    let rug_data = RugCheckData {
                        score: json["score"].as_i64().unwrap_or(0) as i32,
                        score_normalised: json["score_normalised"].as_i64().unwrap_or(0) as i32,
                        rugged: json["rugged"].as_bool().unwrap_or(false),
                        total_supply: json["token"]["supply"].as_u64().unwrap_or(0),
                        creator_balance: json["creatorBalance"].as_u64().unwrap_or(0),
                        total_holders: json["totalHolders"].as_u64().unwrap_or(0),
                        total_market_liquidity: json["totalMarketLiquidity"]
                            .as_f64()
                            .unwrap_or(0.0),
                        risks,
                        mint_authority: json["token"]["mintAuthority"].as_str().map(String::from),
                        freeze_authority: json["token"]["freezeAuthority"]
                            .as_str()
                            .map(String::from),
                        transfer_fee_pct: json["transferFee"]["pct"].as_f64().unwrap_or(0.0),
                        checked_at: Some(
                            std::time::SystemTime
                                ::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                        ),
                    };

                    // Cache the data immediately
                    rugcheck_cache.data.insert(mint.to_string(), rug_data.clone());
                    rugcheck_cache.last_updated = std::time::SystemTime
                        ::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    // Save to disk immediately
                    save_rugcheck_cache(rugcheck_cache, debug).await;

                    if debug {
                        println!("💾 [RugCheck] Cached and saved data for: {}", mint);
                    }

                    Some(rug_data)
                }
                Err(e) => {
                    if debug {
                        println!("❌ [RugCheck] Failed to parse JSON for {}: {}", mint, e);
                    }
                    None
                }
            }
        }
        Err(e) => {
            if debug {
                println!("❌ [RugCheck] Failed to fetch report for {}: {}", mint, e);
            }
            None
        }
    }
}

pub fn is_safe_to_trade(token: &Token, debug: bool) -> bool {
    // If rugcheck is disabled, always return true
    if !ENABLE_RUGCHECK {
        if debug {
            println!("🔓 [Safety] RugCheck disabled - allowing all tokens");
        }
        return true;
    }

    let rug_data = &token.rug_check;

    // Basic safety checks
    if rug_data.rugged {
        if debug {
            println!("🚨 [Safety] Token {} is marked as rugged", token.symbol);
        }
        return false;
    }

    // Score thresholds (lower is better for RugCheck)
    const MAX_ACCEPTABLE_SCORE: i32 = 500;
    const MAX_ACCEPTABLE_NORMALIZED_SCORE: i32 = 30;

    if rug_data.score > MAX_ACCEPTABLE_SCORE {
        if debug {
            println!("🚨 [Safety] Token {} has high risk score: {}", token.symbol, rug_data.score);
        }
        return false;
    }

    if rug_data.score_normalised > MAX_ACCEPTABLE_NORMALIZED_SCORE {
        if debug {
            println!(
                "🚨 [Safety] Token {} has high normalized score: {}",
                token.symbol,
                rug_data.score_normalised
            );
        }
        return false;
    }

    // Check for critical risks
    for risk in &rug_data.risks {
        if risk.level == "danger" && risk.score > 800 {
            if debug {
                println!("🚨 [Safety] Token {} has critical risk: {}", token.symbol, risk.name);
            }
            return false;
        }
    }

    // Check for mint/freeze authority (bad signs)
    if rug_data.mint_authority.is_some() {
        if debug {
            println!("⚠️ [Safety] Token {} has mint authority", token.symbol);
        }
        // Could return false here if you want to be very strict
    }

    if rug_data.freeze_authority.is_some() {
        if debug {
            println!("⚠️ [Safety] Token {} has freeze authority", token.symbol);
        }
        // Could return false here if you want to be very strict
    }

    // Check transfer fees (high fees are bad)
    if rug_data.transfer_fee_pct > 5.0 {
        if debug {
            println!(
                "🚨 [Safety] Token {} has high transfer fee: {}%",
                token.symbol,
                rug_data.transfer_fee_pct
            );
        }
        return false;
    }

    // Check minimum holders
    const MIN_HOLDERS: u64 = 50;
    if rug_data.total_holders < MIN_HOLDERS {
        if debug {
            println!(
                "⚠️ [Safety] Token {} has low holder count: {}",
                token.symbol,
                rug_data.total_holders
            );
        }
        return false;
    }

    // Check minimum liquidity
    const MIN_LIQUIDITY: f64 = 5000.0;
    if rug_data.total_market_liquidity < MIN_LIQUIDITY {
        if debug {
            println!(
                "⚠️ [Safety] Token {} has low liquidity: ${}",
                token.symbol,
                rug_data.total_market_liquidity
            );
        }
        return false;
    }

    if debug {
        println!(
            "✅ [Safety] Token {} passed safety checks (score: {}, normalized: {})",
            token.symbol,
            rug_data.score,
            rug_data.score_normalised
        );
    }

    true
}

/// Get a detailed rug check report for a specific token
pub async fn get_rug_check_report(mint: &str, debug: bool) -> Option<RugCheckData> {
    // Return None if rugcheck is disabled
    if !ENABLE_RUGCHECK {
        if debug {
            println!("🔓 [RugCheck] RugCheck is disabled - returning None");
        }
        return None;
    }

    let client = reqwest::Client::new();
    let mut rugcheck_cache = load_rugcheck_cache(debug).await;
    fetch_rug_check_data(&client, mint, debug, &mut rugcheck_cache).await
}

/// Ensure RugCheck data is cached for all tokens in the watch list
pub async fn ensure_watchlist_rugcheck_cached(debug: bool) {
    // Skip if rugcheck is disabled
    if !ENABLE_RUGCHECK {
        if debug {
            println!("🔓 [RugCheck] RugCheck is disabled - skipping watchlist update");
        }
        return;
    }

    let client = reqwest::Client::new();
    let mut rugcheck_cache = load_rugcheck_cache(debug).await;

    // Get list of tokens that need updates WITHOUT holding the write lock
    let tokens_to_update: Vec<(String, String)> = {
        let tokens = TOKENS.read().await;
        let now = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        tokens
            .iter()
            .filter_map(|token| {
                let needs_refresh = match token.rug_check.checked_at {
                    Some(checked_at) => now - checked_at >= 86400, // 24 hours
                    None => true, // No data at all
                };

                if needs_refresh {
                    Some((token.mint.clone(), token.symbol.clone()))
                } else {
                    if debug {
                        println!(
                            "� [RugCheck] {} has fresh cached data (age: {} hours)",
                            token.symbol,
                            (now - token.rug_check.checked_at.unwrap_or(0)) / 3600
                        );
                    }
                    None
                }
            })
            .collect()
    };

    if debug && !tokens_to_update.is_empty() {
        println!("🔍 [RugCheck] Need to update {} tokens", tokens_to_update.len());
    }

    // Process each token and update them individually to minimize lock time
    for (mint, symbol) in tokens_to_update {
        if debug {
            println!("🔍 [RugCheck] Refreshing data for watch list token: {}", symbol);
        }

        if
            let Some(rug_data) = fetch_rug_check_data(
                &client,
                &mint,
                debug,
                &mut rugcheck_cache
            ).await
        {
            // Only hold the write lock for the brief update
            {
                let mut tokens = TOKENS.write().await;
                if let Some(token) = tokens.iter_mut().find(|t| t.mint == mint) {
                    token.rug_check = rug_data.clone();

                    if debug {
                        println!(
                            "✅ [RugCheck] Updated {} ({}): score={}, normalized={}, rugged={}",
                            symbol,
                            mint,
                            rug_data.score,
                            rug_data.score_normalised,
                            rug_data.rugged
                        );
                    }
                }
            } // Write lock is released here
        }

        // Be respectful to the API - but don't hold the lock during sleep
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }
}

// ────────────── DEXSCREENER FUNCTIONS ──────────────

/// Update token with DexScreener data and cache it
async fn update_token_with_dexscreener_data(
    token: &mut Token,
    item: &Value,
    dexscreener_cache: &mut DexScreenerCache,
    debug: bool
) {
    let now = std::time::SystemTime
        ::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let dex_data = DexScreenerData {
        dex_id: item["dexId"].as_str().unwrap_or_default().into(),
        url: item["url"].as_str().unwrap_or_default().into(),
        pair_address: item["pairAddress"].as_str().unwrap_or_default().into(),
        labels: item["labels"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        quote_address: item["quoteToken"]["address"].as_str().unwrap_or_default().into(),
        quote_name: item["quoteToken"]["name"].as_str().unwrap_or_default().into(),
        quote_symbol: item["quoteToken"]["symbol"].as_str().unwrap_or_default().into(),
        price_native: item["priceNative"].as_str().unwrap_or_default().into(),
        price_usd: item["priceUsd"].as_str().unwrap_or_default().into(),
        volume_usd: item["volume"]["h24"].as_f64().unwrap_or(0.0).to_string(),
        fdv_usd: item["fdv"].as_f64().unwrap_or(0.0).to_string(),
        image_url: item["info"]["imageUrl"].as_str().unwrap_or_default().into(),
        txns: Txns {
            m5: TxnCount {
                buys: item["txns"]["m5"]["buys"].as_u64().unwrap_or(0),
                sells: item["txns"]["m5"]["sells"].as_u64().unwrap_or(0),
            },
            h1: TxnCount {
                buys: item["txns"]["h1"]["buys"].as_u64().unwrap_or(0),
                sells: item["txns"]["h1"]["sells"].as_u64().unwrap_or(0),
            },
            h6: TxnCount {
                buys: item["txns"]["h6"]["buys"].as_u64().unwrap_or(0),
                sells: item["txns"]["h6"]["sells"].as_u64().unwrap_or(0),
            },
            h24: TxnCount {
                buys: item["txns"]["h24"]["buys"].as_u64().unwrap_or(0),
                sells: item["txns"]["h24"]["sells"].as_u64().unwrap_or(0),
            },
        },
        volume: Volume {
            m5: item["volume"]["m5"].as_f64().unwrap_or(0.0),
            h1: item["volume"]["h1"].as_f64().unwrap_or(0.0),
            h6: item["volume"]["h6"].as_f64().unwrap_or(0.0),
            h24: item["volume"]["h24"].as_f64().unwrap_or(0.0),
        },
        price_change: PriceChange {
            m5: item["priceChange"]["m5"].as_f64().unwrap_or(0.0),
            h1: item["priceChange"]["h1"].as_f64().unwrap_or(0.0),
            h6: item["priceChange"]["h6"].as_f64().unwrap_or(0.0),
            h24: item["priceChange"]["h24"].as_f64().unwrap_or(0.0),
        },
        liquidity: Liquidity {
            usd: item["liquidity"]["usd"].as_f64().unwrap_or(0.0),
            base: item["liquidity"]["base"].as_f64().unwrap_or(0.0),
            quote: item["liquidity"]["quote"].as_f64().unwrap_or(0.0),
        },
        pair_created_at: item["pairCreatedAt"].as_u64().unwrap_or(0),
        last_updated: now,
    };

    // Update token with data
    token.dex_id = dex_data.dex_id.clone();
    token.url = dex_data.url.clone();
    token.pair_address = dex_data.pair_address.clone();
    token.labels = dex_data.labels.clone();
    token.name = item["baseToken"]["name"].as_str().unwrap_or_default().into();
    token.symbol = item["baseToken"]["symbol"].as_str().unwrap_or_default().into();
    token.quote_address = dex_data.quote_address.clone();
    token.quote_name = dex_data.quote_name.clone();
    token.quote_symbol = dex_data.quote_symbol.clone();
    token.price_native = dex_data.price_native.clone();
    token.price_usd = dex_data.price_usd.clone();
    token.volume_usd = dex_data.volume_usd.clone();
    token.fdv_usd = dex_data.fdv_usd.clone();
    token.image_url = dex_data.image_url.clone();
    token.txns = dex_data.txns.clone();
    token.volume = dex_data.volume.clone();
    token.price_change = dex_data.price_change.clone();
    token.liquidity = dex_data.liquidity.clone();
    token.pair_created_at = dex_data.pair_created_at;

    // Cache the data
    dexscreener_cache.data.insert(token.mint.clone(), dex_data);
    dexscreener_cache.last_updated = now;

    if debug {
        println!("💾 [DexScreener] Cached data for: {}", token.symbol);
    }
}

/// Apply cached DexScreener data to token if available and valid
fn apply_cached_dexscreener_data(
    token: &mut Token,
    dexscreener_cache: &DexScreenerCache,
    debug: bool
) -> bool {
    if let Some(cached_data) = dexscreener_cache.data.get(&token.mint) {
        if is_dexscreener_data_valid(cached_data) {
            token.dex_id = cached_data.dex_id.clone();
            token.url = cached_data.url.clone();
            token.pair_address = cached_data.pair_address.clone();
            token.labels = cached_data.labels.clone();
            token.quote_address = cached_data.quote_address.clone();
            token.quote_name = cached_data.quote_name.clone();
            token.quote_symbol = cached_data.quote_symbol.clone();
            token.price_native = cached_data.price_native.clone();
            token.price_usd = cached_data.price_usd.clone();
            token.volume_usd = cached_data.volume_usd.clone();
            token.fdv_usd = cached_data.fdv_usd.clone();
            token.image_url = cached_data.image_url.clone();
            token.txns = cached_data.txns.clone();
            token.volume = cached_data.volume.clone();
            token.price_change = cached_data.price_change.clone();
            token.liquidity = cached_data.liquidity.clone();
            token.pair_created_at = cached_data.pair_created_at;

            if debug {
                println!(
                    "📋 [DexScreener] Using cached data for {} (age: {} minutes)",
                    token.symbol,
                    (std::time::SystemTime
                        ::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() -
                        cached_data.last_updated) /
                        60
                );
            }
            return true;
        }
    }
    false
}

// ────────────── CHUNK HELPER ──────────────
fn chunked(vec: Vec<String>, chunk_size: usize) -> Vec<Vec<String>> {
    vec.chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

pub fn start_dexscreener_loop() {
    println!("🚀 Starting DexScreener loop...");

    let debug = ARGS.iter().any(|a| a == "--debug-dexscreener");
    let client = Client::new();
    let client_insert = client.clone();

    tokio::spawn(async move {
        // ───── Load from disk cache on start ──────────
        let mut cached_tokens: Vec<Token> = Vec::new();
        if let Ok(mut file) = fs::File::open(TOKEN_CACHE_FILE).await {
            let mut data = Vec::new();
            if file.read_to_end(&mut data).await.is_ok() {
                if let Ok(tokens) = serde_json::from_slice::<Vec<Token>>(&data) {
                    cached_tokens = tokens;
                    let mut lock = TOKENS.write().await;
                    lock.clear();
                    lock.extend(cached_tokens.clone());
                    if debug {
                        println!("📥 Loaded {} tokens from disk cache", lock.len());
                    }
                }
            }
        }

        // Load rugcheck and dexscreener caches
        let mut rugcheck_cache = load_rugcheck_cache(debug).await;
        let mut dexscreener_cache = load_dexscreener_cache(debug).await;

        loop {
            if shutdown::is_shutdown_requested() {
                break;
            }

            if debug {
                println!("\n🔄 [Screener] Fetching DexScreener token lists...");
            }

            let mut new_tokens: Vec<Token> = Vec::new();
            let endpoints = [
                "https://api.dexscreener.com/token-profiles/latest/v1",
                "https://api.dexscreener.com/token-boosts/latest/v1",
                "https://api.dexscreener.com/token-boosts/top/v1",
            ];

            // RugCheck API endpoints
            let rugcheck_endpoints = [
                "https://api.rugcheck.xyz/v1/stats/verified",
                "https://api.rugcheck.xyz/v1/stats/recent",
                "https://api.rugcheck.xyz/v1/stats/new_tokens",
            ];

            // GeckoTerminal API endpoints
            let geckoterminal_endpoints = [
                "https://api.geckoterminal.com/api/v2/tokens/info_recently_updated?include=network&network=solana",
            ];

            // Raydium API endpoints
            let raydium_endpoints = ["https://api-v3.raydium.io/mint/list"];

            // ── first-pass lists with rate limiting ────────────────────────────
            for url in endpoints {
                if debug {
                    println!("🌐 [Screener] Requesting: {}", url);
                }
                if
                    let Ok(resp) = client_insert.get_with_rate_limit(
                        url,
                        &DEXSCREENER_LIMITER
                    ).await
                {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(arr) = json.as_array() {
                            if debug {
                                println!("✅ {} tokens from {}", arr.len(), url);
                            }
                            for item in arr {
                                let mint = item["tokenAddress"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string();
                                if BLACKLIST.read().await.contains(&mint) {
                                    continue;
                                }
                                new_tokens.push(Token {
                                    mint,
                                    balance: "0".into(),
                                    ata_pubkey: item["url"].as_str().unwrap_or_default().into(),
                                    program_id: item["chainId"].as_str().unwrap_or_default().into(),
                                    name: "".into(),
                                    symbol: "".into(),
                                    dex_id: String::new(),
                                    url: String::new(),
                                    pair_address: String::new(),
                                    labels: Vec::new(),
                                    quote_address: String::new(),
                                    quote_name: String::new(),
                                    quote_symbol: String::new(),
                                    price_native: String::new(),
                                    price_usd: String::new(),
                                    last_price_usd: String::new(),
                                    volume_usd: String::new(),
                                    fdv_usd: String::new(),
                                    image_url: String::new(),
                                    txns: Txns {
                                        m5: TxnCount { buys: 0, sells: 0 },
                                        h1: TxnCount { buys: 0, sells: 0 },
                                        h6: TxnCount { buys: 0, sells: 0 },
                                        h24: TxnCount { buys: 0, sells: 0 },
                                    },
                                    volume: Volume { m5: 0.0, h1: 0.0, h6: 0.0, h24: 0.0 },
                                    price_change: PriceChange {
                                        m5: 0.0,
                                        h1: 0.0,
                                        h6: 0.0,
                                        h24: 0.0,
                                    },
                                    liquidity: Liquidity { usd: 0.0, base: 0.0, quote: 0.0 },
                                    pair_created_at: 0,
                                    rug_check: RugCheckData::default(),
                                });
                            }
                        }
                    }
                }
            }

            // ── RugCheck API endpoints with rate limiting ───────────────────
            for url in rugcheck_endpoints {
                if debug {
                    println!("🔍 [RugCheck] Requesting: {}", url);
                }
                if let Ok(resp) = client_insert.get_with_rate_limit(url, &RUGCHECK_LIMITER).await {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(arr) = json.as_array() {
                            if debug {
                                println!("✅ {} tokens from {}", arr.len(), url);
                            }
                            for item in arr {
                                let mint = item["mint"].as_str().unwrap_or_default().to_string();
                                if mint.is_empty() || BLACKLIST.read().await.contains(&mint) {
                                    continue;
                                }
                                new_tokens.push(Token {
                                    mint,
                                    balance: "0".into(),
                                    ata_pubkey: "".into(),
                                    program_id: "".into(),
                                    name: item["name"].as_str().unwrap_or_default().into(),
                                    symbol: item["symbol"].as_str().unwrap_or_default().into(),
                                    dex_id: String::new(),
                                    url: String::new(),
                                    pair_address: String::new(),
                                    labels: Vec::new(),
                                    quote_address: String::new(),
                                    quote_name: String::new(),
                                    quote_symbol: String::new(),
                                    price_native: String::new(),
                                    price_usd: String::new(),
                                    last_price_usd: String::new(),
                                    volume_usd: String::new(),
                                    fdv_usd: String::new(),
                                    image_url: String::new(),
                                    txns: Txns {
                                        m5: TxnCount { buys: 0, sells: 0 },
                                        h1: TxnCount { buys: 0, sells: 0 },
                                        h6: TxnCount { buys: 0, sells: 0 },
                                        h24: TxnCount { buys: 0, sells: 0 },
                                    },
                                    volume: Volume { m5: 0.0, h1: 0.0, h6: 0.0, h24: 0.0 },
                                    price_change: PriceChange {
                                        m5: 0.0,
                                        h1: 0.0,
                                        h6: 0.0,
                                        h24: 0.0,
                                    },
                                    liquidity: Liquidity { usd: 0.0, base: 0.0, quote: 0.0 },
                                    pair_created_at: 0,
                                    rug_check: RugCheckData::default(),
                                });
                            }
                        }
                    }
                }
            }

            // ── GeckoTerminal API endpoints with rate limiting ──────────────
            for url in geckoterminal_endpoints {
                if debug {
                    println!("🦎 [GeckoTerminal] Requesting: {}", url);
                }
                if
                    let Ok(resp) = client_insert.get_with_rate_limit(
                        url,
                        &GECKOTERMINAL_LIMITER
                    ).await
                {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(data_arr) = json["data"].as_array() {
                            if debug {
                                println!("✅ {} tokens from {}", data_arr.len(), url);
                            }
                            for item in data_arr {
                                let mint = item["attributes"]["address"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string();
                                if mint.is_empty() || BLACKLIST.read().await.contains(&mint) {
                                    continue;
                                }
                                new_tokens.push(Token {
                                    mint,
                                    balance: "0".into(),
                                    ata_pubkey: "".into(),
                                    program_id: "".into(),
                                    name: item["attributes"]["name"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .into(),
                                    symbol: item["attributes"]["symbol"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .into(),
                                    dex_id: String::new(),
                                    url: String::new(),
                                    pair_address: String::new(),
                                    labels: Vec::new(),
                                    quote_address: String::new(),
                                    quote_name: String::new(),
                                    quote_symbol: String::new(),
                                    price_native: String::new(),
                                    price_usd: String::new(),
                                    last_price_usd: String::new(),
                                    volume_usd: String::new(),
                                    fdv_usd: String::new(),
                                    image_url: item["attributes"]["image_url"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .into(),
                                    txns: Txns {
                                        m5: TxnCount { buys: 0, sells: 0 },
                                        h1: TxnCount { buys: 0, sells: 0 },
                                        h6: TxnCount { buys: 0, sells: 0 },
                                        h24: TxnCount { buys: 0, sells: 0 },
                                    },
                                    volume: Volume { m5: 0.0, h1: 0.0, h6: 0.0, h24: 0.0 },
                                    price_change: PriceChange {
                                        m5: 0.0,
                                        h1: 0.0,
                                        h6: 0.0,
                                        h24: 0.0,
                                    },
                                    liquidity: Liquidity { usd: 0.0, base: 0.0, quote: 0.0 },
                                    pair_created_at: 0,
                                    rug_check: RugCheckData::default(),
                                });
                            }
                        }
                    }
                }
            }

            // ── Raydium API endpoints ──────────────────────────────────────
            for url in raydium_endpoints {
                if debug {
                    println!("🌊 [Raydium] Requesting: {}", url);
                }
                if
                    let Ok(resp) = client_insert
                        .get(url)
                        .header(
                            "User-Agent",
                            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
                        )
                        .header("Accept", "application/json")
                        .send().await
                {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(mint_list) = json["data"]["mintList"].as_array() {
                            if debug {
                                println!("✅ {} tokens from {}", mint_list.len(), url);
                            }
                            for item in mint_list {
                                let mint = item["address"].as_str().unwrap_or_default().to_string();
                                if mint.is_empty() || BLACKLIST.read().await.contains(&mint) {
                                    continue;
                                }

                                // Filter out tokens with empty symbol/name if desired
                                let symbol = item["symbol"].as_str().unwrap_or_default();
                                let name = item["name"].as_str().unwrap_or_default();

                                if symbol.is_empty() && name.is_empty() {
                                    continue; // Skip tokens without basic info
                                }

                                new_tokens.push(Token {
                                    mint,
                                    balance: "0".into(),
                                    ata_pubkey: "".into(),
                                    program_id: item["programId"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .into(),
                                    name: name.into(),
                                    symbol: symbol.into(),
                                    dex_id: String::new(),
                                    url: String::new(),
                                    pair_address: String::new(),
                                    labels: item["tags"]
                                        .as_array()
                                        .unwrap_or(&vec![])
                                        .iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect(),
                                    quote_address: String::new(),
                                    quote_name: String::new(),
                                    quote_symbol: String::new(),
                                    price_native: String::new(),
                                    price_usd: String::new(),
                                    last_price_usd: String::new(),
                                    volume_usd: String::new(),
                                    fdv_usd: String::new(),
                                    image_url: item["logoURI"].as_str().unwrap_or_default().into(),
                                    txns: Txns {
                                        m5: TxnCount { buys: 0, sells: 0 },
                                        h1: TxnCount { buys: 0, sells: 0 },
                                        h6: TxnCount { buys: 0, sells: 0 },
                                        h24: TxnCount { buys: 0, sells: 0 },
                                    },
                                    volume: Volume { m5: 0.0, h1: 0.0, h6: 0.0, h24: 0.0 },
                                    price_change: PriceChange {
                                        m5: 0.0,
                                        h1: 0.0,
                                        h6: 0.0,
                                        h24: 0.0,
                                    },
                                    liquidity: Liquidity { usd: 0.0, base: 0.0, quote: 0.0 },
                                    pair_created_at: 0,
                                    rug_check: RugCheckData::default(),
                                });
                            }
                        }
                    }
                }
            }

            // ───────── ALWAYS include open positions' mints ─────────────
            // always use latest cached open positions
            let open_pos_mints: Vec<String> = {
                let open_pos = crate::persistence::OPEN_POSITIONS.read().await;
                open_pos.keys().cloned().collect()
            };

            // combine with new token mints and dedup
            let mut mints: Vec<String> = new_tokens
                .iter()
                .map(|t| t.mint.clone())
                .collect();
            mints.extend(open_pos_mints.iter().cloned());
            mints.sort();
            mints.dedup();

            // Load rugcheck cache for applying cached data
            let rugcheck_cache = load_rugcheck_cache(debug).await;

            // Apply cached RugCheck data to new tokens where available and still valid
            let now = std::time::SystemTime
                ::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            for token in &mut new_tokens {
                if let Some(cached_rug) = rugcheck_cache.data.get(&token.mint) {
                    if is_rugcheck_data_valid(cached_rug) {
                        token.rug_check = cached_rug.clone();
                        if debug {
                            println!(
                                "📋 [RugCheck] Using cached data for {} (age: {} hours)",
                                token.symbol,
                                (now - cached_rug.checked_at.unwrap_or(0)) / 3600
                            );
                        }
                    }
                }
            }

            // Load DexScreener cache
            let mut dexscreener_cache = load_dexscreener_cache(debug).await;

            // Apply cached DexScreener data where available
            for token in &mut new_tokens {
                apply_cached_dexscreener_data(token, &dexscreener_cache, debug);
            }

            let batches = chunked(mints, 30);

            for (i, batch) in batches.iter().enumerate() {
                let joined = batch.join(",");
                let url = format!("https://api.dexscreener.com/tokens/v1/solana/{}", joined);
                if debug {
                    println!("🔗 DexScreener info (batch {}/{})", i + 1, batches.len());
                }
                if let Ok(resp) = client_insert.get(&url).send().await {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(arr) = json.as_array() {
                            for item in arr {
                                let base_address = item["baseToken"]["address"]
                                    .as_str()
                                    .unwrap_or_default();
                                if
                                    let Some(tok) = new_tokens
                                        .iter_mut()
                                        .find(|t| t.mint == base_address)
                                {
                                    // Check if we need to update this token (if cache is stale or missing)
                                    if
                                        !is_dexscreener_data_valid(
                                            &dexscreener_cache.data.get(&tok.mint).unwrap_or(
                                                &(DexScreenerData {
                                                    dex_id: String::new(),
                                                    url: String::new(),
                                                    pair_address: String::new(),
                                                    labels: Vec::new(),
                                                    quote_address: String::new(),
                                                    quote_name: String::new(),
                                                    quote_symbol: String::new(),
                                                    price_native: String::new(),
                                                    price_usd: String::new(),
                                                    volume_usd: String::new(),
                                                    fdv_usd: String::new(),
                                                    image_url: String::new(),
                                                    txns: Txns {
                                                        m5: TxnCount { buys: 0, sells: 0 },
                                                        h1: TxnCount { buys: 0, sells: 0 },
                                                        h6: TxnCount { buys: 0, sells: 0 },
                                                        h24: TxnCount { buys: 0, sells: 0 },
                                                    },
                                                    volume: Volume {
                                                        m5: 0.0,
                                                        h1: 0.0,
                                                        h6: 0.0,
                                                        h24: 0.0,
                                                    },
                                                    price_change: PriceChange {
                                                        m5: 0.0,
                                                        h1: 0.0,
                                                        h6: 0.0,
                                                        h24: 0.0,
                                                    },
                                                    liquidity: Liquidity {
                                                        usd: 0.0,
                                                        base: 0.0,
                                                        quote: 0.0,
                                                    },
                                                    pair_created_at: 0,
                                                    last_updated: 0,
                                                })
                                            )
                                        )
                                    {
                                        update_token_with_dexscreener_data(
                                            tok,
                                            item,
                                            &mut dexscreener_cache,
                                            debug
                                        ).await;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Save DexScreener cache to disk
            save_dexscreener_cache(&dexscreener_cache, debug).await;

            // ── Fetch rug check data for all tokens ──────────────────────
            let mut rugcheck_cache = load_rugcheck_cache(debug).await;

            if ENABLE_RUGCHECK {
                if debug {
                    println!(
                        "🔍 [RugCheck] Fetching rug check data for {} tokens...",
                        new_tokens.len()
                    );
                }

                // Fetch rug check data for tokens in batches to avoid overwhelming the API
                let rug_check_batch_size = 5; // Conservative batch size
                let total_tokens = new_tokens.len();

                for batch_idx in 0..(total_tokens + rug_check_batch_size - 1) /
                    rug_check_batch_size {
                    let start_idx = batch_idx * rug_check_batch_size;
                    let end_idx = std::cmp::min(start_idx + rug_check_batch_size, total_tokens);

                    if debug {
                        println!(
                            "🔍 [RugCheck] Processing batch {}/{}",
                            batch_idx + 1,
                            (total_tokens + rug_check_batch_size - 1) / rug_check_batch_size
                        );
                    }

                    for token_idx in start_idx..end_idx {
                        let token = &mut new_tokens[token_idx];

                        // Skip if we already have recent rug check data
                        if let Some(checked_at) = token.rug_check.checked_at {
                            let now = std::time::SystemTime
                                ::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();

                            // Skip if checked within the configured cache period
                            if now - checked_at < RUGCHECK_CACHE_DAYS * 24 * 60 * 60 {
                                if debug {
                                    println!(
                                        "🔍 [RugCheck] Skipping {} - cached data is {} hours old",
                                        token.symbol,
                                        (now - checked_at) / 3600
                                    );
                                }
                                continue;
                            }
                        }

                        // Use cache-enabled fetch function
                        if
                            let Some(rug_data) = fetch_rug_check_data(
                                &client_insert,
                                &token.mint,
                                debug,
                                &mut rugcheck_cache
                            ).await
                        {
                            token.rug_check = rug_data;

                            if debug {
                                println!(
                                    "✅ [RugCheck] {} ({}): score={}, normalized={}, rugged={}",
                                    token.symbol,
                                    token.mint,
                                    token.rug_check.score,
                                    token.rug_check.score_normalised,
                                    token.rug_check.rugged
                                );
                            }
                        }

                        // Small delay between requests to be respectful to the API
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    }

                    // Longer delay between batches
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            } else if debug {
                println!("🔓 [RugCheck] RugCheck is disabled - skipping data fetch");
            }

            const MIN_PRICE_SOL: f64 = 0.000000001;
            const MAX_PRICE_SOL: f64 = 0.01;

            const MIN_VOLUME_USD: f64 = 5000.0;
            const MIN_FDV_USD: f64 = 20_000.0;
            const MAX_FDV_USD: f64 = 500_000_000.0;
            const MIN_LIQUIDITY_SOL: f64 = 10.0;

            const MAX_PRICE_CHANGE_M5: f64 = 10.0;
            const MAX_PRICE_CHANGE_H1: f64 = 80.0;
            const MAX_PRICE_CHANGE_H6: f64 = 100.0;
            const MAX_PRICE_CHANGE_H24: f64 = 180.0;

            const MIN_BUYS_24H: u64 = 50; // at least 10 buys in 24h
            const MAX_DUMP_24H: f64 = -50.0; // reject if -50% or worse in 24h

            // // Apply comprehensive filtering including rug check safety
            // new_tokens.retain(|t| {
            //     let price = t.price_native.parse::<f64>().unwrap_or(0.0);
            //     let vol = t.volume_usd.parse::<f64>().unwrap_or(0.0);
            //     let fdv = t.fdv_usd.parse::<f64>().unwrap_or(0.0);
            //     let pooled_sol = t.liquidity.base + t.liquidity.quote;
            //     let has_image = !t.image_url.trim().is_empty();

            //     let price_ok =
            //         t.price_change.m5.abs() <= MAX_PRICE_CHANGE_M5 &&
            //         t.price_change.h1.abs() <= MAX_PRICE_CHANGE_H1 &&
            //         t.price_change.h6.abs() <= MAX_PRICE_CHANGE_H6 &&
            //         t.price_change.h24.abs() <= MAX_PRICE_CHANGE_H24;

            //     let not_rugged = t.price_change.h24 > MAX_DUMP_24H;
            //     let not_dead = t.txns.h24.buys >= MIN_BUYS_24H;

            //     // Add rug check safety validation
            //     let rug_check_safe = is_safe_to_trade(t, debug);

            //     price >= MIN_PRICE_SOL &&
            //         price <= MAX_PRICE_SOL &&
            //         vol >= MIN_VOLUME_USD &&
            //         fdv >= MIN_FDV_USD &&
            //         fdv <= MAX_FDV_USD &&
            //         has_image &&
            //         pooled_sol >= MIN_LIQUIDITY_SOL &&
            //         !t.symbol.is_empty() &&
            //         !t.name.is_empty() &&
            //         !t.price_usd.is_empty() &&
            //         price_ok &&
            //         not_rugged &&
            //         not_dead &&
            //         rug_check_safe // Include rug check safety
            // });

            if debug {
                println!("✅ {} tokens remain after price filter", new_tokens.len());
            }

            new_tokens.sort_unstable_by(|a, b| {
                let va = a.volume_usd.parse::<f64>().unwrap_or(0.0);
                let vb = b.volume_usd.parse::<f64>().unwrap_or(0.0);
                vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
            });
            new_tokens.truncate(MAX_TOKENS);

            let existing_mints: Vec<String> = {
                let lock = TOKENS.read().await;
                lock.iter()
                    .map(|t| t.mint.clone())
                    .collect()
            };

            let mut filtered_tokens = Vec::with_capacity(MAX_TOKENS);
            for t in new_tokens {
                if BLACKLIST.read().await.contains(&t.mint) {
                    continue;
                }
                if !t.symbol.is_empty() && !t.price_usd.is_empty() && !t.name.is_empty() {
                    if !existing_mints.contains(&t.mint) {
                        if debug {
                            println!(
                                "{} {} {}\n  {} {}\n  {} {}\n  {} {} SOL\n  {} ${}\n  {} ${}\n  {} ${}\n  {} {}\n  {} {}\n  {} {}",
                                "🆕".bold(),
                                t.symbol.green().bold(),
                                "-".normal(),
                                "Name:".blue().bold(),
                                t.name.white(),
                                "Mint:".blue().bold(),
                                t.mint.dimmed(),
                                "Native:".blue().bold(),
                                t.price_native.cyan().bold(),
                                "USD:".blue().bold(),
                                t.price_usd.yellow().bold(),
                                "Volume24h:".blue().bold(),
                                t.volume_usd.magenta().bold(),
                                "FDV:".blue().bold(),
                                t.fdv_usd.blue().bold(),
                                "ATA:".blue().bold(),
                                t.ata_pubkey.dimmed(),
                                "ProgramID:".blue().bold(),
                                t.program_id.dimmed(),
                                "ImageURL:".blue().bold(),
                                t.image_url.underline().white()
                            );
                        } else {
                            println!("🆕 Added: {} ({})", t.symbol, t.mint);
                        }
                    }
                    filtered_tokens.push(t);
                }
            }

            {
                let mut lock = TOKENS.write().await;
                lock.clear();
                lock.extend(filtered_tokens);
            }

            // Ensure all watch list tokens have up-to-date RugCheck data
            ensure_watchlist_rugcheck_cached(debug).await;

            // ───── Save to disk cache ──────────
            if let Ok(data) = serde_json::to_vec(&*TOKENS.read().await) {
                let _ = fs::write(TOKEN_CACHE_FILE, data).await;
            }

            // Save rugcheck and dexscreener caches
            if ENABLE_RUGCHECK {
                if debug {
                    println!("💾 Saving rugcheck cache with {} entries", rugcheck_cache.data.len());
                }
                save_rugcheck_cache(&rugcheck_cache, debug).await;
            }
            if debug {
                println!(
                    "💾 Saving dexscreener cache with {} entries",
                    dexscreener_cache.data.len()
                );
            }
            save_dexscreener_cache(&dexscreener_cache, debug).await;

            println!("✅ TOKENS updated: {}", TOKENS.read().await.len());
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });
}
