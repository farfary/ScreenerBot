/// DexScreener API integration
/// Handles all communication with DexScreener API including rate limiting and caching
use crate::logger::{ log, LogTag };
use crate::tokens::types::*;
use crate::global::read_configs;
use std::collections::HashMap;
use std::time::{ Duration, Instant };
use tokio::sync::Semaphore;
use std::sync::Arc;
use reqwest::StatusCode;
use serde_json;
use chrono::Utc;

/// DexScreener API client with rate limiting and statistics
pub struct DexScreenerApi {
    client: reqwest::Client,
    rate_limiter: Arc<Semaphore>,
    discovery_rate_limiter: Arc<Semaphore>,
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
            rate_limiter: Arc::new(Semaphore::new(300)), // 300 requests per minute
            discovery_rate_limiter: Arc::new(Semaphore::new(60)), // 60 requests per minute for discovery
            stats: ApiStats::new(),
            last_request_time: None,
        }
    }

    /// Initialize the API client
    pub async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Initializing DexScreener API client...");

        // Test API connectivity
        match self.test_connectivity().await {
            Ok(_) => {
                log(LogTag::System, "SUCCESS", "DexScreener API client initialized successfully");
                Ok(())
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to initialize DexScreener API: {}", e)
                );
                Err(e)
            }
        }
    }

    /// Test API connectivity
    async fn test_connectivity(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Testing DexScreener API connectivity...");

        let test_url = "https://api.dexscreener.com/token-boosts/latest/v1";
        let start_time = Instant::now();

        let permit = self.discovery_rate_limiter
            .clone()
            .acquire_owned().await
            .map_err(|e| format!("Failed to acquire rate limit permit: {}", e))?;

        let response = self.client
            .get(test_url)
            .send().await
            .map_err(|e| format!("Failed to connect to DexScreener API: {}", e))?;

        drop(permit);

        let response_time = start_time.elapsed().as_millis() as f64;
        let success = response.status().is_success();

        self.stats.record_request(success, response_time);

        if success {
            log(
                LogTag::System,
                "SUCCESS",
                &format!("DexScreener API test successful ({}ms)", response_time)
            );
            Ok(())
        } else {
            let error_msg = format!(
                "DexScreener API test failed with status: {}",
                response.status()
            );
            log(LogTag::System, "ERROR", &error_msg);
            Err(error_msg)
        }
    }

    /// Get token price for a single mint address
    pub async fn get_token_price(&mut self, mint: &str) -> Option<f64> {
        log(LogTag::Trader, "API", &format!("Fetching price for token: {}", mint));

        match self.get_token_data(mint).await {
            Ok(Some(token)) => {
                if let Some(price) = token.price_sol {
                    log(
                        LogTag::Trader,
                        "API",
                        &format!("Got price for {}: {:.12} SOL", mint, price)
                    );
                    Some(price)
                } else {
                    log(
                        LogTag::Trader,
                        "WARN",
                        &format!("No SOL price available for token: {}", mint)
                    );
                    None
                }
            }
            Ok(None) => {
                log(LogTag::Trader, "WARN", &format!("Token not found in DexScreener: {}", mint));
                None
            }
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("Failed to fetch token price for {}: {}", mint, e)
                );
                None
            }
        }
    }

    /// Get token prices for multiple mint addresses (batch)
    pub async fn get_multiple_token_prices(&mut self, mints: &[String]) -> HashMap<String, f64> {
        let mut prices = HashMap::new();

        // Process in chunks of 30 (DexScreener API limit)
        for chunk in mints.chunks(30) {
            match self.get_multiple_token_data(chunk).await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Some(price) = token.price_sol {
                            prices.insert(token.mint.clone(), price);
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Failed to fetch batch token prices: {}", e)
                    );
                }
            }

            // Small delay between batches to be API-friendly
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        log(
            LogTag::Trader,
            "API",
            &format!("Fetched prices for {}/{} tokens", prices.len(), mints.len())
        );
        prices
    }

    /// Get detailed token data for a single mint
    pub async fn get_token_data(&mut self, mint: &str) -> Result<Option<ApiToken>, String> {
        let url = format!("https://api.dexscreener.com/tokens/v1/solana/{}", mint);

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

        // Parse response
        if let Some(pairs_array) = data.as_array() {
            if let Some(pair_data) = pairs_array.first() {
                match self.parse_token_from_pair(pair_data) {
                    Ok(token) => Ok(Some(token)),
                    Err(e) => {
                        log(
                            LogTag::Trader,
                            "WARN",
                            &format!("Failed to parse token data for {}: {}", mint, e)
                        );
                        Ok(None)
                    }
                }
            } else {
                Ok(None) // No pairs found
            }
        } else {
            Err("Invalid API response format".to_string())
        }
    }

    /// Get detailed token data for multiple mints (batch)
    pub async fn get_multiple_token_data(
        &mut self,
        mints: &[String]
    ) -> Result<Vec<ApiToken>, String> {
        if mints.is_empty() {
            return Ok(vec![]);
        }

        if mints.len() > 30 {
            return Err("Too many tokens requested (max 30)".to_string());
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

        if let Some(pairs_array) = data.as_array() {
            for pair_data in pairs_array {
                match self.parse_token_from_pair(pair_data) {
                    Ok(token) => tokens.push(token),
                    Err(e) => {
                        log(
                            LogTag::Trader,
                            "WARN",
                            &format!("Failed to parse token from batch: {}", e)
                        );
                    }
                }
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

        // Calculate SOL price based on quote token
        let quote_token = pair_data.get("quoteToken");
        let price_sol = if let Some(qt) = quote_token {
            if let Some(quote_address) = qt.get("address").and_then(|v| v.as_str()) {
                // If quote is SOL, price_native is already in SOL
                if quote_address == "So11111111111111111111111111111111111111112" {
                    Some(price_native)
                } else {
                    // Calculate SOL price from USD price if available
                    // This is approximate and would need SOL/USD rate
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Parse additional fields
        let liquidity = self.parse_liquidity(pair_data.get("liquidity"));
        let volume = self.parse_volume(pair_data.get("volume"));
        let txns = self.parse_txns(pair_data.get("txns"));
        let price_change = self.parse_price_change(pair_data.get("priceChange"));
        let fdv = pair_data.get("fdv").and_then(|v| v.as_f64());
        let market_cap = pair_data.get("marketCap").and_then(|v| v.as_f64());
        let pair_created_at = pair_data.get("pairCreatedAt").and_then(|v| v.as_i64());
        let boosts = self.parse_boosts(pair_data.get("boosts"));
        let info = self.parse_info(pair_data.get("info"));
        let labels = self.parse_labels(pair_data.get("labels"));

        Ok(ApiToken {
            mint,
            symbol,
            name,
            decimals: 9, // Default - will be updated by TokenDiscovery system
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

    fn parse_info(&self, value: Option<&serde_json::Value>) -> Option<TokenInfo> {
        value.map(|v| TokenInfo {
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

    /// Get discovery endpoints for token finding
    pub async fn discover_tokens(
        &mut self,
        source: DiscoverySourceType
    ) -> Result<Vec<String>, String> {
        let url = match source {
            DiscoverySourceType::DexScreenerBoosts =>
                "https://api.dexscreener.com/token-boosts/latest/v1",
            DiscoverySourceType::DexScreenerBoostsTop =>
                "https://api.dexscreener.com/token-boosts/top/v1",
            DiscoverySourceType::DexScreenerProfiles =>
                "https://api.dexscreener.com/token-profiles/latest/v1",
            _ => {
                return Err("Unsupported discovery source".to_string());
            }
        };

        log(LogTag::Monitor, "DISCOVERY", &format!("Discovering tokens from: {:?}", source));

        let start_time = Instant::now();

        // Use discovery rate limiter (60 requests per minute)
        let permit = self.discovery_rate_limiter
            .clone()
            .acquire_owned().await
            .map_err(|e| format!("Failed to acquire discovery rate limit permit: {}", e))?;

        let response = self.client
            .get(url)
            .send().await
            .map_err(|e| format!("Discovery request failed: {}", e))?;

        drop(permit);

        let response_time = start_time.elapsed().as_millis() as f64;
        let success = response.status() == StatusCode::OK;

        self.stats.record_request(success, response_time);

        if !success {
            return Err(format!("Discovery API returned status: {}", response.status()));
        }

        let data: serde_json::Value = response
            .json().await
            .map_err(|e| format!("Failed to parse discovery response: {}", e))?;

        let mut mints = Vec::new();

        if let Some(items_array) = data.as_array() {
            for item in items_array {
                if let Some(token_address) = item.get("tokenAddress").and_then(|v| v.as_str()) {
                    if token_address.len() == 44 {
                        // Valid Solana address length
                        mints.push(token_address.to_string());
                    }
                }
            }
        }

        log(
            LogTag::Monitor,
            "DISCOVERY",
            &format!("Discovered {} tokens from {:?}", mints.len(), source)
        );
        Ok(mints)
    }

    /// Get trending tokens for discovery
    pub async fn get_trending_tokens(
        &mut self,
        chain: &str,
        limit: usize
    ) -> Result<Vec<ApiToken>, String> {
        let _permit = self.discovery_rate_limiter.acquire().await.unwrap();

        let url = format!("https://api.dexscreener.com/latest/dex/pairs/{}", chain);

        log(LogTag::System, "API", &format!("Fetching trending tokens for {}", chain));

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let text = response.text().await.map_err(|e| e.to_string())?;

                    // Parse response and extract tokens
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(pairs) = json.get("pairs").and_then(|v| v.as_array()) {
                            let mut tokens = Vec::new();

                            for pair in pairs.iter().take(limit) {
                                if let Ok(token) = self.parse_dexscreener_pair_to_token(pair) {
                                    tokens.push(token);
                                }
                            }

                            log(
                                LogTag::System,
                                "SUCCESS",
                                &format!("Found {} trending tokens", tokens.len())
                            );

                            return Ok(tokens);
                        }
                    }

                    Err("Failed to parse trending tokens response".to_string())
                } else {
                    Err(format!("API request failed with status: {}", response.status()))
                }
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Failed to fetch trending tokens: {}", e));
                Err(e.to_string())
            }
        }
    }

    /// Get detailed token information for multiple tokens
    pub async fn get_tokens_info(&mut self, mints: &[String]) -> Result<Vec<ApiToken>, String> {
        if mints.is_empty() {
            return Ok(Vec::new());
        }

        let _permit = self.rate_limiter.acquire().await.unwrap();

        // DexScreener supports up to 30 tokens per request
        let mints_str = mints.iter().take(30).cloned().collect::<Vec<String>>().join(",");

        let url = format!("https://api.dexscreener.com/latest/dex/tokens/{}", mints_str);

        log(LogTag::System, "API", &format!("Fetching info for {} tokens", mints.len()));

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let text = response.text().await.map_err(|e| e.to_string())?;

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(pairs) = json.get("pairs").and_then(|v| v.as_array()) {
                            let mut tokens = Vec::new();

                            for pair in pairs {
                                if let Ok(token) = self.parse_dexscreener_pair_to_token(pair) {
                                    tokens.push(token);
                                }
                            }

                            log(
                                LogTag::System,
                                "SUCCESS",
                                &format!("Retrieved info for {} tokens", tokens.len())
                            );

                            return Ok(tokens);
                        }
                    }

                    Err("Failed to parse token info response".to_string())
                } else {
                    Err(format!("API request failed with status: {}", response.status()))
                }
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Failed to fetch token info: {}", e));
                Err(e.to_string())
            }
        }
    }

    /// Parse DexScreener pair JSON to ApiToken
    fn parse_dexscreener_pair_to_token(
        &self,
        pair: &serde_json::Value
    ) -> Result<ApiToken, Box<dyn std::error::Error>> {
        let base_token = pair.get("baseToken");
        let quote_token = pair.get("quoteToken");

        // Prefer base token for analysis
        let token_data = if let Some(base) = base_token {
            if
                base
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .unwrap_or("") != "SOL"
            {
                base
            } else {
                quote_token.unwrap_or(base)
            }
        } else {
            return Err("Missing token data".into());
        };

        let mint = token_data
            .get("address")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let symbol = token_data
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = token_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if mint.is_empty() || symbol.is_empty() {
            return Err("Missing required token data".into());
        }

        let price_usd = pair
            .get("priceUsd")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let price_native = pair
            .get("priceNative")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let price_sol = if
            quote_token.and_then(|q| q.get("symbol")).and_then(|v| v.as_str()) == Some("SOL")
        {
            Some(price_native)
        } else {
            None
        };

        let liquidity = if
            let Some(liquidity_obj) = pair.get("liquidity").and_then(|v| v.as_object())
        {
            Some(LiquidityInfo {
                usd: liquidity_obj.get("usd").and_then(|v| v.as_f64()),
                base: liquidity_obj.get("base").and_then(|v| v.as_f64()),
                quote: liquidity_obj.get("quote").and_then(|v| v.as_f64()),
            })
        } else {
            None
        };

        let volume = if let Some(volume_obj) = pair.get("volume").and_then(|v| v.as_object()) {
            Some(VolumeStats {
                h24: volume_obj.get("h24").and_then(|v| v.as_f64()),
                h6: volume_obj.get("h6").and_then(|v| v.as_f64()),
                h1: volume_obj.get("h1").and_then(|v| v.as_f64()),
                m5: volume_obj.get("m5").and_then(|v| v.as_f64()),
            })
        } else {
            None
        };

        let txns = if let Some(txns_obj) = pair.get("txns").and_then(|v| v.as_object()) {
            Some(TxnStats {
                h24: txns_obj
                    .get("h24")
                    .and_then(|v| v.as_object())
                    .map(|h24| TxnPeriod {
                        buys: h24.get("buys").and_then(|v| v.as_i64()),
                        sells: h24.get("sells").and_then(|v| v.as_i64()),
                    }),
                h6: txns_obj
                    .get("h6")
                    .and_then(|v| v.as_object())
                    .map(|h6| TxnPeriod {
                        buys: h6.get("buys").and_then(|v| v.as_i64()),
                        sells: h6.get("sells").and_then(|v| v.as_i64()),
                    }),
                h1: txns_obj
                    .get("h1")
                    .and_then(|v| v.as_object())
                    .map(|h1| TxnPeriod {
                        buys: h1.get("buys").and_then(|v| v.as_i64()),
                        sells: h1.get("sells").and_then(|v| v.as_i64()),
                    }),
                m5: txns_obj
                    .get("m5")
                    .and_then(|v| v.as_object())
                    .map(|m5| TxnPeriod {
                        buys: m5.get("buys").and_then(|v| v.as_i64()),
                        sells: m5.get("sells").and_then(|v| v.as_i64()),
                    }),
            })
        } else {
            None
        };

        let price_change = if
            let Some(price_change_obj) = pair.get("priceChange").and_then(|v| v.as_object())
        {
            Some(PriceChangeStats {
                h24: price_change_obj.get("h24").and_then(|v| v.as_f64()),
                h6: price_change_obj.get("h6").and_then(|v| v.as_f64()),
                h1: price_change_obj.get("h1").and_then(|v| v.as_f64()),
                m5: price_change_obj.get("m5").and_then(|v| v.as_f64()),
            })
        } else {
            None
        };

        Ok(ApiToken {
            mint,
            symbol,
            name,
            decimals: 9, // Default - will be updated by TokenDiscovery system
            chain_id: pair["chainId"].as_str().unwrap_or("solana").to_string(),
            dex_id: pair["dexId"].as_str().unwrap_or("").to_string(),
            pair_address: pair["pairAddress"].as_str().unwrap_or("").to_string(),
            pair_url: pair["url"].as_str().map(|s| s.to_string()),
            price_native,
            price_usd,
            price_sol,
            liquidity,
            volume,
            txns,
            price_change,
            fdv: pair["fdv"].as_f64(),
            market_cap: pair["marketCap"].as_f64(),
            pair_created_at: pair["pairCreatedAt"].as_i64(),
            boosts: pair["boosts"].as_object().map(|b| BoostInfo {
                active: b["active"].as_i64(),
            }),
            info: pair["info"].as_object().map(|info| TokenInfo {
                image_url: info["imageUrl"].as_str().map(|s| s.to_string()),
                websites: info["websites"].as_array().map(|websites| {
                    websites
                        .iter()
                        .filter_map(|w| {
                            w["url"].as_str().map(|url| WebsiteInfo {
                                url: url.to_string(),
                            })
                        })
                        .collect()
                }),
                socials: info["socials"].as_array().map(|socials| {
                    socials
                        .iter()
                        .filter_map(|s| {
                            Some(SocialInfo {
                                platform: s["type"].as_str()?.to_string(),
                                handle: s["url"].as_str()?.to_string(),
                            })
                        })
                        .collect()
                }),
            }),
            labels: pair
                .get("labels")
                .and_then(|v| v.as_array())
                .map(|labels| {
                    labels
                        .iter()
                        .filter_map(|l| l.as_str().map(|s| s.to_string()))
                        .collect()
                }),
            last_updated: Utc::now(),
        })
    }

    /// Get top tokens from DexScreener with optional limit
    pub async fn get_top_tokens(&mut self, limit: usize) -> Result<Vec<String>, String> {
        log(LogTag::Monitor, "TOP_TOKENS", &format!("Fetching top {} tokens", limit));

        // DexScreener doesn't have a simple "top tokens" endpoint
        // We'll use the latest endpoint as the closest equivalent
        let url = "https://api.dexscreener.com/latest/dex/tokens";

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

        log(LogTag::Monitor, "TOP_TOKENS", &format!("Found {} top tokens", mints.len()));
        Ok(mints)
    }

    /// Discover tokens and fetch their detailed information in one call
    pub async fn discover_and_fetch_tokens(
        &mut self,
        source: DiscoverySourceType,
        limit: usize
    ) -> Result<Vec<ApiToken>, String> {
        // First discover the token mints
        let mints = self.discover_tokens(source).await?;

        // Take only the requested limit
        let limited_mints: Vec<String> = mints.into_iter().take(limit).collect();

        if limited_mints.is_empty() {
            return Ok(Vec::new());
        }

        // Then fetch detailed information for these tokens
        self.get_tokens_info(&limited_mints).await
    }
}

/// Standalone function to get token prices from API
pub async fn get_token_prices_from_api(mints: Vec<String>) -> HashMap<String, f64> {
    let mut api = DexScreenerApi::new();

    if let Err(e) = api.initialize().await {
        log(LogTag::Trader, "ERROR", &format!("Failed to initialize API: {}", e));
        return HashMap::new();
    }

    api.get_multiple_token_prices(&mints).await
}
