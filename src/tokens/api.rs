/// DexScreener API integration
/// Handles token information retrieval with rate limiting and caching
use crate::logger::{ log, LogTag };
use crate::global::is_debug_api_enabled;
use crate::swaps::config::SOL_MINT;

// =============================================================================
// DEXSCREENER API CONFIGURATION CONSTANTS
// =============================================================================

/// DexScreener API rate limit (requests per minute)
pub const DEXSCREENER_RATE_LIMIT_PER_MINUTE: usize = 300;

/// DexScreener discovery API rate limit (requests per minute)
pub const DEXSCREENER_DISCOVERY_RATE_LIMIT: usize = 60;

/// Maximum tokens per API call (DexScreener API constraint)
pub const MAX_TOKENS_PER_API_CALL: usize = 30;

/// API calls per monitoring cycle (based on rate limits)
pub const API_CALLS_PER_MONITORING_CYCLE: usize = 30;
use crate::tokens::types::{
    TokenInfo,
    VolumeStats,
    PriceChangeStats,
    TxnPeriod,
    ApiToken,
    ApiStats,
    LiquidityInfo,
    TxnStats,
    BoostInfo,
    WebsiteInfo,
    SocialInfo,
    DiscoverySourceType,
    Token,
};
use std::collections::HashMap;
use std::time::{ Duration, Instant };
use std::sync::Arc;
use tokio::sync::Semaphore;
use reqwest::StatusCode;
use serde_json;
use chrono::Utc;

/// DexScreener API client with rate limiting and statistics
pub struct DexScreenerApi {
    client: reqwest::Client,
    rate_limiter: Arc<Semaphore>,
    stats: ApiStats,
    last_request_time: Option<Instant>,
}

impl DexScreenerApi {
    /// Create new DexScreener API client
    pub fn new() -> Self {
        Self {
            client: reqwest::Client
                ::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("ScreenerBot/1.0")
                .build()
                .expect("Failed to create HTTP client"),
            rate_limiter: Arc::new(Semaphore::new(DEXSCREENER_RATE_LIMIT_PER_MINUTE)),
            stats: ApiStats::new(),
            last_request_time: None,
        }
    }

    /// Initialize the API client
    pub async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::Api, "INIT", "Initializing DexScreener API client...");
        log(LogTag::Api, "SUCCESS", "DexScreener API client initialized successfully");
        Ok(())
    }

    /// Get token price for a single mint address
    pub async fn get_token_price(&mut self, mint: &str) -> Option<f64> {
        match self.get_token_data(mint).await {
            Ok(Some(token)) => {
                if let Some(price) = token.price_sol { Some(price) } else { None }
            }
            Ok(None) => None,
            Err(e) => {
                log(LogTag::Api, "ERROR", &format!("Failed to fetch price for {}: {}", mint, e));
                None
            }
        }
    }

    /// Get token prices for multiple mint addresses (batch)
    pub async fn get_multiple_token_prices(&mut self, mints: &[String]) -> HashMap<String, f64> {
        let mut prices = HashMap::new();
        let start_time = Instant::now();
        let mut total_errors = 0;

        // Process in chunks of MAX_TOKENS_PER_API_CALL (DexScreener API limit)
        for (chunk_idx, chunk) in mints.chunks(MAX_TOKENS_PER_API_CALL).enumerate() {
            match self.get_tokens_info(chunk).await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Some(price) = token.price_sol {
                            prices.insert(token.mint.clone(), price);
                        }
                    }
                }
                Err(e) => {
                    total_errors += 1;
                    if total_errors <= 3 {
                        // Only log first 3 errors to avoid spam
                        log(
                            LogTag::Api,
                            "ERROR",
                            &format!(
                                "Batch {} failed (tokens {}-{}): {}",
                                chunk_idx + 1,
                                chunk_idx * 30 + 1,
                                chunk_idx * MAX_TOKENS_PER_API_CALL + chunk.len(),
                                e
                            )
                        );
                    }
                }
            }

            // Small delay between batches to be API-friendly
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let elapsed = start_time.elapsed().as_millis();

        if total_errors > 0 {
            log(
                LogTag::Api,
                "WARN",
                &format!(
                    "Price batch completed with {} errors: {}/{} tokens in {}ms",
                    total_errors,
                    prices.len(),
                    mints.len(),
                    elapsed
                )
            );
        } else {
            log(
                LogTag::Api,
                "SUCCESS",
                &format!(
                    "Price batch completed: {}/{} tokens in {}ms",
                    prices.len(),
                    mints.len(),
                    elapsed
                )
            );
        }

        prices
    }

    /// Get detailed token data for a single mint
    pub async fn get_token_data(&mut self, mint: &str) -> Result<Option<ApiToken>, String> {
        let tokens = self.get_tokens_info(&[mint.to_string()]).await?;
        Ok(tokens.into_iter().next())
    }

    /// Get Token object from mint address (converts ApiToken to Token)
    pub async fn get_token_from_mint(&mut self, mint: &str) -> Result<Option<Token>, String> {
        let api_tokens = self.get_tokens_info(&[mint.to_string()]).await?;

        if let Some(api_token) = api_tokens.into_iter().next() {
            let token = Token::from(api_token);
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    /// Get token information for multiple mint addresses (main function)
    pub async fn get_tokens_info(&mut self, mints: &[String]) -> Result<Vec<ApiToken>, String> {
        if mints.is_empty() {
            return Ok(Vec::new());
        }

        if mints.len() > MAX_TOKENS_PER_API_CALL {
            return Err(
                format!(
                    "Too many tokens requested: {}. Maximum is {}",
                    mints.len(),
                    MAX_TOKENS_PER_API_CALL
                )
            );
        }

        let mint_list = mints.join(",");
        let url = format!("https://api.dexscreener.com/tokens/v1/solana/{}", mint_list);

        let start_time = Instant::now();

        // Rate limiting
        let permit = self.rate_limiter
            .clone()
            .acquire_owned().await
            .map_err(|e| format!("Failed to acquire rate limit permit: {}", e))?;

        let response = self.client
            .get(&url)
            .send().await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        drop(permit);

        let response_time = start_time.elapsed().as_millis() as f64;
        let success = response.status() == StatusCode::OK;

        self.stats.record_request(success, response_time);
        self.last_request_time = Some(start_time);

        if !success {
            return Err(format!("API returned status: {}", response.status()));
        }

        let data: serde_json::Value = response
            .json().await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

        let mut tokens = Vec::new();
        let mut rejected_non_sol_pairs = 0;
        let mut parsing_errors = 0;

        if let Some(pairs_array) = data.as_array() {
            for pair_data in pairs_array {
                match self.parse_token_from_pair(pair_data) {
                    Ok(token) => tokens.push(token),
                    Err(e) => {
                        if e.contains("not paired with SOL") || e.contains("not a SOL pair") {
                            rejected_non_sol_pairs += 1;
                            if is_debug_api_enabled() {
                                log(LogTag::Api, "SOL_FILTER", &format!("Rejected: {}", e));
                            }
                        } else {
                            parsing_errors += 1;
                            log(
                                LogTag::Api,
                                "WARN",
                                &format!("Failed to parse token from batch: {}", e)
                            );
                        }
                    }
                }
            }
        }

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "SUCCESS",
                &format!(
                    "Retrieved info for {}/{} tokens in {}ms (rejected {} non-SOL pairs, {} parsing errors)",
                    tokens.len(),
                    mints.len(),
                    response_time as u64,
                    rejected_non_sol_pairs,
                    parsing_errors
                )
            );
        } else if rejected_non_sol_pairs > 0 {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "SOL_FILTER",
                    &format!("Filtered out {} non-SOL pairs from batch", rejected_non_sol_pairs)
                );
            }
        }

        Ok(tokens)
    }

    /// Parse token data from DexScreener pair response
    fn parse_token_from_pair(&self, pair_data: &serde_json::Value) -> Result<ApiToken, String> {
        let base_token = pair_data.get("baseToken").ok_or("Missing baseToken field")?;

        let mint = base_token
            .get("address")
            .and_then(|v| v.as_str())
            .ok_or("Missing token address")?
            .to_string();

        let symbol = base_token
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let name = base_token
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let chain_id = pair_data
            .get("chainId")
            .and_then(|v| v.as_str())
            .unwrap_or("solana")
            .to_string();

        let dex_id = pair_data
            .get("dexId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let pair_address = pair_data
            .get("pairAddress")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let pair_url = pair_data
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Parse prices
        let price_native_str = pair_data
            .get("priceNative")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        let price_native = price_native_str
            .parse::<f64>()
            .map_err(|_| format!("Invalid price_native: {}", price_native_str))?;

        let price_usd = if let Some(usd_str) = pair_data.get("priceUsd").and_then(|v| v.as_str()) {
            usd_str.parse::<f64>().unwrap_or(0.0)
        } else {
            0.0
        };

        // CRITICAL: Only accept tokens paired with SOL
        let quote_token = pair_data.get("quoteToken");
        let (price_sol, is_sol_pair) = if let Some(qt) = quote_token {
            if let Some(quote_address) = qt.get("address").and_then(|v| v.as_str()) {
                // Check if quote is SOL
                if quote_address == SOL_MINT {
                    (Some(price_native), true)
                } else {
                    // Reject non-SOL pairs
                    return Err(
                        format!("Token {} is not paired with SOL (quote: {})", mint, quote_address)
                    );
                }
            } else {
                return Err(format!("Token {} has no quote address", mint));
            }
        } else {
            return Err(format!("Token {} has no quote token", mint));
        };

        // Only proceed if this is a SOL pair
        if !is_sol_pair {
            return Err(format!("Token {} is not a SOL pair", mint));
        }

        // Parse additional fields
        let liquidity = self.parse_liquidity(pair_data.get("liquidity"));
        let volume = self.parse_volume(pair_data.get("volume"));
        let txns = self.parse_txns(pair_data.get("txns"));
        let price_change = self.parse_price_change(pair_data.get("priceChange"));
        let fdv = pair_data.get("fdv").and_then(|v| v.as_f64());
        let market_cap = pair_data.get("marketCap").and_then(|v| v.as_f64());
        let pair_created_at = pair_data.get("pairCreatedAt").and_then(|v| v.as_i64());
        let boosts = self.parse_boosts(pair_data.get("boosts"));
        let info = self.parse_info(pair_data.get("info"), &mint, &name, &symbol);
        let labels = self.parse_labels(pair_data.get("labels"));

        Ok(ApiToken {
            mint,
            symbol,
            name,
            // decimals removed - only use decimal_cache.json
            chain_id,
            dex_id,
            pair_address,
            pair_url,
            price_native,
            price_usd,
            price_sol,
            liquidity,
            volume,
            txns,
            price_change,
            fdv,
            market_cap,
            pair_created_at,
            boosts,
            info,
            labels,
            last_updated: Utc::now(),
        })
    }

    // Helper methods for parsing complex fields
    fn parse_liquidity(&self, value: Option<&serde_json::Value>) -> Option<LiquidityInfo> {
        value.map(|v| LiquidityInfo {
            usd: v.get("usd").and_then(|f| f.as_f64()),
            base: v.get("base").and_then(|f| f.as_f64()),
            quote: v.get("quote").and_then(|f| f.as_f64()),
        })
    }

    fn parse_volume(&self, value: Option<&serde_json::Value>) -> Option<VolumeStats> {
        value.map(|v| VolumeStats {
            h24: v.get("h24").and_then(|f| f.as_f64()),
            h6: v.get("h6").and_then(|f| f.as_f64()),
            h1: v.get("h1").and_then(|f| f.as_f64()),
            m5: v.get("m5").and_then(|f| f.as_f64()),
        })
    }

    fn parse_txns(&self, value: Option<&serde_json::Value>) -> Option<TxnStats> {
        value.map(|v| TxnStats {
            h24: self.parse_txn_period(v.get("h24")),
            h6: self.parse_txn_period(v.get("h6")),
            h1: self.parse_txn_period(v.get("h1")),
            m5: self.parse_txn_period(v.get("m5")),
        })
    }

    fn parse_txn_period(&self, value: Option<&serde_json::Value>) -> Option<TxnPeriod> {
        value.map(|v| TxnPeriod {
            buys: v.get("buys").and_then(|i| i.as_i64()),
            sells: v.get("sells").and_then(|i| i.as_i64()),
        })
    }

    fn parse_price_change(&self, value: Option<&serde_json::Value>) -> Option<PriceChangeStats> {
        value.map(|v| PriceChangeStats {
            h24: v.get("h24").and_then(|f| f.as_f64()),
            h6: v.get("h6").and_then(|f| f.as_f64()),
            h1: v.get("h1").and_then(|f| f.as_f64()),
            m5: v.get("m5").and_then(|f| f.as_f64()),
        })
    }

    fn parse_boosts(&self, value: Option<&serde_json::Value>) -> Option<BoostInfo> {
        value.map(|v| BoostInfo {
            active: v.get("active").and_then(|i| i.as_i64()),
        })
    }

    fn parse_info(
        &self,
        value: Option<&serde_json::Value>,
        address: &str,
        name: &str,
        symbol: &str
    ) -> Option<TokenInfo> {
        value.map(|v| TokenInfo {
            address: address.to_string(),
            name: name.to_string(),
            symbol: symbol.to_string(),
            image_url: v
                .get("imageUrl")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string()),
            websites: self.parse_websites(v.get("websites")),
            socials: self.parse_socials(v.get("socials")),
        })
    }

    fn parse_websites(&self, value: Option<&serde_json::Value>) -> Option<Vec<WebsiteInfo>> {
        value
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.get("url")
                            .and_then(|url| url.as_str())
                            .map(|url| WebsiteInfo { url: url.to_string() })
                    })
                    .collect()
            })
    }

    fn parse_socials(&self, value: Option<&serde_json::Value>) -> Option<Vec<SocialInfo>> {
        value
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let platform = item.get("platform")?.as_str()?.to_string();
                        let handle = item.get("handle")?.as_str()?.to_string();
                        Some(SocialInfo { platform, handle })
                    })
                    .collect()
            })
    }

    fn parse_labels(&self, value: Option<&serde_json::Value>) -> Option<Vec<String>> {
        value
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect()
            })
    }

    /// Get API statistics
    pub fn get_stats(&self) -> ApiStats {
        self.stats.clone()
    }

    /// Get token information from specific mints (batch processing for discovery.rs)
    pub async fn get_multiple_token_data(
        &mut self,
        mints: &[String]
    ) -> Result<Vec<ApiToken>, String> {
        self.get_tokens_info(mints).await
    }

    /// Simple discovery endpoint access for discovery.rs (boosts)
    pub async fn discover_and_fetch_tokens(
        &mut self,
        source: DiscoverySourceType,
        limit: usize
    ) -> Result<Vec<ApiToken>, String> {
        let url = match source {
            DiscoverySourceType::DexScreenerBoosts =>
                "https://api.dexscreener.com/token-boosts/latest/v1",
            DiscoverySourceType::DexScreenerProfiles =>
                "https://api.dexscreener.com/token-profiles/latest/v1",
            _ => {
                return Err("Unsupported discovery source".to_string());
            }
        };

        let response = self.client
            .get(url)
            .send().await
            .map_err(|e| format!("Discovery request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Discovery API returned status: {}", response.status()));
        }

        let data: serde_json::Value = response
            .json().await
            .map_err(|e| format!("Failed to parse discovery response: {}", e))?;

        let mut mints = Vec::new();
        if let Some(items_array) = data.as_array() {
            for item in items_array.iter().take(limit) {
                if let Some(token_address) = item.get("tokenAddress").and_then(|v| v.as_str()) {
                    if token_address.len() == 44 {
                        mints.push(token_address.to_string());
                    }
                }
            }
        }

        if mints.is_empty() {
            return Ok(Vec::new());
        }

        self.get_tokens_info(&mints).await
    }

    /// Simple top tokens access for discovery.rs
    pub async fn get_top_tokens(&mut self, limit: usize) -> Result<Vec<String>, String> {
        let url = "https://api.dexscreener.com/latest/dex/pairs/solana";

        let response = self.client
            .get(url)
            .send().await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

        let json: serde_json::Value = serde_json
            ::from_str(&text)
            .map_err(|e| format!("JSON parsing failed: {}", e))?;

        let mut mints = Vec::new();
        if let Some(pairs) = json.get("pairs").and_then(|v| v.as_array()) {
            for pair in pairs.iter().take(limit) {
                if let Some(base_token) = pair.get("baseToken") {
                    if let Some(mint) = base_token.get("address").and_then(|v| v.as_str()) {
                        if
                            !mint.is_empty() &&
                            base_token
                                .get("symbol")
                                .and_then(|v| v.as_str())
                                .unwrap_or("") != "SOL"
                        {
                            mints.push(mint.to_string());
                        }
                    }
                }
            }
        }

        Ok(mints)
    }
}

/// Pool pair information from DexScreener API
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TokenPair {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    #[serde(rename = "dexId")]
    pub dex_id: String,
    pub url: String,
    #[serde(rename = "pairAddress")]
    pub pair_address: String,
    pub labels: Option<Vec<String>>,
    #[serde(rename = "baseToken")]
    pub base_token: TokenInfo,
    #[serde(rename = "quoteToken")]
    pub quote_token: TokenInfo,
    #[serde(rename = "priceNative")]
    pub price_native: String,
    #[serde(rename = "priceUsd")]
    pub price_usd: Option<String>,
    pub txns: TxnStats,
    pub volume: VolumeStats,
    #[serde(rename = "priceChange")]
    pub price_change: PriceChangeStats,
    pub liquidity: Option<LiquidityStats>,
    #[serde(rename = "pairCreatedAt")]
    pub pair_created_at: Option<u64>, // Made optional since some pairs don't have this field
    pub fdv: Option<f64>,
    #[serde(rename = "marketCap")]
    pub market_cap: Option<f64>,
}

// Remove the duplicate struct definitions since we import from types
// All structs are now imported from tokens::types module

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LiquidityStats {
    pub usd: f64,
    pub base: f64,
    pub quote: f64,
}

impl DexScreenerApi {
    /// Get token pairs (pools) for a specific token from DexScreener API
    pub async fn get_token_pairs(
        &mut self,
        chain_id: &str,
        token_address: &str
    ) -> Result<Vec<TokenPair>, String> {
        let url = format!(
            "https://api.dexscreener.com/token-pairs/v1/{}/{}",
            chain_id,
            token_address
        );

        if is_debug_api_enabled() {
            log(LogTag::Api, "REQUEST", &format!("Fetching pools for token: {}", token_address));
        }

        let start_time = Instant::now();

        // Rate limiting
        let permit = self.rate_limiter
            .clone()
            .acquire_owned().await
            .map_err(|e| format!("Failed to acquire rate limit permit: {}", e))?;

        // Make HTTP request
        let response = self.client
            .get(&url)
            .send().await
            .map_err(|e| format!("Failed to fetch token pairs: {}", e))?;

        drop(permit);

        let response_time = start_time.elapsed().as_millis() as f64;
        let success = response.status().is_success();

        self.stats.record_request(success, response_time);

        if !success {
            let error_msg = format!("API request failed with status: {}", response.status());
            log(LogTag::Api, "ERROR", &error_msg);
            return Err(error_msg);
        }

        // Parse response
        let response_text = response
            .text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let pairs: Vec<TokenPair> = serde_json
            ::from_str(&response_text)
            .map_err(|e| format!("Failed to parse token pairs response: {}", e))?;

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "SUCCESS",
                &format!(
                    "Found {} pools for token {} ({}ms)",
                    pairs.len(),
                    token_address,
                    response_time
                )
            );
        }

        // Update last request time
        self.last_request_time = Some(Instant::now());

        Ok(pairs)
    }

    /// Get token pairs for Solana specifically
    pub async fn get_solana_token_pairs(
        &mut self,
        token_address: &str
    ) -> Result<Vec<TokenPair>, String> {
        self.get_token_pairs("solana", token_address).await
    }
}

/// Standalone function to get token prices from API
pub async fn get_token_prices_from_api(mints: Vec<String>) -> HashMap<String, f64> {
    get_multiple_token_prices_from_global_api(&mints).await
}

/// Standalone function to get token pairs from API
pub async fn get_token_pairs_from_api(token_address: &str) -> Result<Vec<TokenPair>, String> {
    let api = get_global_dexscreener_api().await?;
    let mut api_instance = api.lock().await;
    api_instance.get_solana_token_pairs(token_address).await
}

// =============================================================================
// GLOBAL DEXSCREENER API SINGLETON (TRUE SINGLETON)
// =============================================================================

use tokio::sync::{ OnceCell, Mutex };

static GLOBAL_DEXSCREENER_API: OnceCell<Arc<Mutex<DexScreenerApi>>> = OnceCell::const_new();

/// Initialize the global DexScreener API client (creates single instance)
pub async fn init_dexscreener_api() -> Result<(), String> {
    // Check if already initialized
    if GLOBAL_DEXSCREENER_API.get().is_some() {
        log(LogTag::Api, "INIT", "DexScreener API already initialized, skipping");
        return Ok(());
    }

    let api = Arc::new(Mutex::new(DexScreenerApi::new()));

    // Initialize the API instance once
    {
        let mut api_instance = api.lock().await;
        api_instance.initialize().await?;
    }

    GLOBAL_DEXSCREENER_API.set(api).map_err(
        |_| "Failed to initialize global DexScreener API state"
    )?;

    log(LogTag::Api, "SUCCESS", "DexScreener API client initialized successfully");
    Ok(())
}

/// Get reference to the global DexScreener API client
pub async fn get_global_dexscreener_api() -> Result<Arc<Mutex<DexScreenerApi>>, String> {
    GLOBAL_DEXSCREENER_API.get()
        .ok_or_else(||
            "DexScreener API not initialized. Call init_dexscreener_api() first.".to_string()
        )
        .map(|api| api.clone())
}

/// Helper function to get token price using global API
pub async fn get_token_price_from_global_api(mint: &str) -> Option<f64> {
    match get_global_dexscreener_api().await {
        Ok(api) => {
            let mut api_instance = api.lock().await;
            api_instance.get_token_price(mint).await
        }
        Err(e) => {
            log(LogTag::Api, "ERROR", &format!("Failed to get global API client: {}", e));
            None
        }
    }
}

/// Helper function to get Token object from mint using global API
pub async fn get_token_from_mint_global_api(mint: &str) -> Result<Option<Token>, String> {
    match get_global_dexscreener_api().await {
        Ok(api) => {
            let mut api_instance = api.lock().await;
            api_instance.get_token_from_mint(mint).await
        }
        Err(e) => {
            log(LogTag::Api, "ERROR", &format!("Failed to get global API client: {}", e));
            Err(e)
        }
    }
}

/// Helper function to get multiple token prices using global API
pub async fn get_multiple_token_prices_from_global_api(mints: &[String]) -> HashMap<String, f64> {
    match get_global_dexscreener_api().await {
        Ok(api) => {
            let mut api_instance = api.lock().await;
            api_instance.get_multiple_token_prices(mints).await
        }
        Err(e) => {
            log(LogTag::Api, "ERROR", &format!("Failed to get global API client: {}", e));
            HashMap::new()
        }
    }
}
