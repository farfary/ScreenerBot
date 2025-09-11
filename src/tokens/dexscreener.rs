use crate::global::is_debug_api_enabled;
/// DexScreener API integration with SIMPLE 1-MINUTE TTL CACHING
///
/// Simplified design: we keep a single in-memory cache with a fixed 60s TTL
/// for every token. No position-aware branching, no extended TTL paths.
/// Always attempt to return cached data if still fresh; otherwise fetch,
/// cache and return. This reduces complexity and duplicate logic.
use crate::logger::{ log, LogTag };
use crate::swaps::config::SOL_MINT;
use crate::tokens::types::{
    ApiStats,
    ApiToken,
    BoostInfo,
    DiscoverySourceType,
    LiquidityInfo,
    PriceChangeStats,
    SocialInfo,
    Token,
    TokenInfo,
    TxnPeriod,
    TxnStats,
    VolumeStats,
    WebsiteInfo,
};
use chrono::{ DateTime, Utc };
use reqwest::StatusCode;
use serde_json;
use std::collections::HashMap;
use std::sync::{ Arc, LazyLock };
use std::time::{ Duration, Instant };
use tokio::sync::{ Mutex, OnceCell, RwLock, Semaphore };
use tokio::time::timeout;

// (Removed internal FetchMode enum – not needed after simplification)

// =============================================================================
// DEXSCREENER API CONFIGURATION CONSTANTS
// =============================================================================

/// DexScreener API rate limit (requests per minute)
pub const DEXSCREENER_RATE_LIMIT_PER_MINUTE: usize = 100;

/// DexScreener discovery API rate limit (requests per minute)
pub const DEXSCREENER_DISCOVERY_RATE_LIMIT: usize = 60;

/// Maximum tokens per API call (DexScreener API constraint)
pub const MAX_TOKENS_PER_API_CALL: usize = 30;

/// API calls per monitoring cycle (based on rate limits)
pub const API_CALLS_PER_MONITORING_CYCLE: usize = 90;

// =============================================================================
// SIMPLE CACHING SYSTEM (GLOBAL 60s TTL)
// =============================================================================

/// Cache entry for storing token data with timestamp
#[derive(Debug, Clone)]
pub struct CachedTokenData {
    pub token: ApiToken,
    pub cached_at: DateTime<Utc>,
}

/// Global cache for token data (separate from the API client instance)
static TOKEN_CACHE: LazyLock<RwLock<HashMap<String, CachedTokenData>>> = LazyLock::new(||
    RwLock::new(HashMap::new())
);

/// Cache TTL in seconds (1 minute maximum for price data)
const PRICE_CACHE_TTL_SECS: i64 = 60; // 1 minute TTL for all token data

// (Removed has_open_position – no position-aware logic)

/// Get cached token data if available and not expired
async fn get_cached_token_data(mint: &str) -> Option<ApiToken> {
    let cache = TOKEN_CACHE.read().await;

    if let Some(cached_data) = cache.get(mint) {
        let now = Utc::now();
        let age_seconds = (now - cached_data.cached_at).num_seconds();

        // For price data, always use 1 minute maximum TTL regardless of position status
        let ttl_seconds = PRICE_CACHE_TTL_SECS; // 1 minute for all price data

        if age_seconds < ttl_seconds {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "CACHE_HIT",
                    &format!(
                        "Using cached data for {} (age: {}s, TTL: {}s)",
                        mint,
                        age_seconds,
                        ttl_seconds
                    )
                );
            }
            return Some(cached_data.token.clone());
        } else {
            // Cache is expired, don't use it for price data
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "CACHE_EXPIRED",
                    &format!(
                        "Cache expired for {} (age: {}s > TTL: {}s) - will fetch fresh data",
                        mint,
                        age_seconds,
                        ttl_seconds
                    )
                );
            }
        }
    }

    None
}

/// Store token data in cache
async fn cache_token_data(mint: &str, token: &ApiToken) {
    let mut cache = TOKEN_CACHE.write().await;
    cache.insert(mint.to_string(), CachedTokenData {
        token: token.clone(),
        cached_at: Utc::now(),
    });

    if is_debug_api_enabled() {
        log(LogTag::Api, "CACHE_STORE", &format!("Cached data for token {}", mint));
    }
}

/// Get cache statistics
pub async fn get_cache_stats() -> (usize, usize) {
    let cache = TOKEN_CACHE.read().await;
    let total_entries = cache.len();
    let now = Utc::now();

    let valid_entries = cache
        .values()
        .filter(|entry| {
            let age_seconds = (now - entry.cached_at).num_seconds();
            age_seconds < PRICE_CACHE_TTL_SECS // 1 minute validity for price data
        })
        .count();

    (total_entries, valid_entries)
}

/// Clean up expired cache entries (call periodically to prevent memory leaks)
/// Note: This should be called periodically by a background task to prevent unbounded cache growth
pub async fn cleanup_expired_cache_entries() {
    let mut cache = TOKEN_CACHE.write().await;
    let now = Utc::now();
    let mut removed_count = 0;

    // Remove entries older than 1 minute for price data
    cache.retain(|_mint, entry| {
        let age_seconds = (now - entry.cached_at).num_seconds();
        let should_keep = age_seconds < PRICE_CACHE_TTL_SECS; // 1 minute cleanup
        if !should_keep {
            removed_count += 1;
        }
        should_keep
    });

    if removed_count > 0 && is_debug_api_enabled() {
        log(
            LogTag::Api,
            "CACHE_CLEANUP",
            &format!("Cleaned up {} expired cache entries (older than 1 minute)", removed_count)
        );
    }
}

/// Check if position-aware caching is enabled
// (Removed is_position_aware_caching_enabled + logging – obsolete)

/// Get a summary of cache effectiveness (useful for debugging and monitoring)
pub async fn get_cache_effectiveness_summary() -> String {
    let (total_entries, valid_entries) = get_cache_stats().await;
    format!(
        "DexScreener Cache | Entries: {} total, {} valid (TTL {}s)",
        total_entries,
        valid_entries,
        PRICE_CACHE_TTL_SECS
    )
}

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
        if is_debug_api_enabled() {
            log(LogTag::Api, "INIT", "Initializing DexScreener API client...");
        }

        // Simplified caching – no config status to log

        if is_debug_api_enabled() {
            log(LogTag::Api, "SUCCESS", "DexScreener API client initialized successfully");
        }
        Ok(())
    }

    /// Get token price for a single mint address
    pub async fn get_price(&mut self, mint: &str) -> Option<f64> {
        match self.fetch_and_cache_token(mint).await {
            Ok(Some(t)) => t.price_sol,
            _ => None,
        }
    }

    /// Get multiple token prices for multiple mint addresses (batch)
    pub async fn get_prices(&mut self, mints: &[String]) -> HashMap<String, f64> {
        let mut prices = HashMap::new();
        let start_time = Instant::now();
        let mut total_errors = 0;
        let mut cached_count = 0;
        let mut position_skipped_count = 0;
        let mut api_call_mints = Vec::new();

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "DEBUG",
                &format!("🚀 Starting batch price fetch for {} tokens", mints.len())
            );
        }

        // First pass: Check positions and cache for all mints
        for mint in mints {
            if
                is_debug_api_enabled() &&
                cached_count + position_skipped_count + api_call_mints.len() < 5
            {
                log(LogTag::Api, "DEBUG", &format!("🔍 Checking token {}", &mint[..8]));
            }

            if let Some(cached_token) = get_cached_token_data(mint).await {
                if let Some(price) = cached_token.price_sol {
                    prices.insert(mint.clone(), price);
                    cached_count += 1;
                    if is_debug_api_enabled() && cached_count <= 3 {
                        log(
                            LogTag::Api,
                            "DEBUG",
                            &format!("💾 Cache hit for {}: ${:.8}", &mint[..8], price)
                        );
                    }
                }
                continue;
            }

            // Add to list for API calls
            api_call_mints.push(mint.clone());
        }

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "DEBUG",
                &format!(
                    "📊 Batch analysis: {} cached, {} need API calls",
                    cached_count,
                    api_call_mints.len()
                )
            );
        }

        // Second pass: Make API calls only for tokens without open positions
        if !api_call_mints.is_empty() {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "DEBUG",
                    &format!("🌐 Making API calls for {} tokens", api_call_mints.len())
                );
            }

            // Process in chunks of MAX_TOKENS_PER_API_CALL (DexScreener API limit)
            for (chunk_idx, chunk) in api_call_mints.chunks(MAX_TOKENS_PER_API_CALL).enumerate() {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Api,
                        "DEBUG",
                        &format!(
                            "📦 Processing chunk {} with {} tokens",
                            chunk_idx + 1,
                            chunk.len()
                        )
                    );
                }

                match self.get_tokens_info(chunk).await {
                    Ok(tokens) => {
                        if is_debug_api_enabled() {
                            log(
                                LogTag::Api,
                                "DEBUG",
                                &format!(
                                    "✅ Chunk {} returned {} tokens",
                                    chunk_idx + 1,
                                    tokens.len()
                                )
                            );
                        }

                        for token in tokens {
                            // Cache the result
                            cache_token_data(&token.mint, &token).await;

                            if let Some(price) = token.price_sol {
                                prices.insert(token.mint.clone(), price);
                                if is_debug_api_enabled() && prices.len() <= 3 {
                                    log(
                                        LogTag::Api,
                                        "DEBUG",
                                        &format!(
                                            "💰 Got price for {}: ${:.8}",
                                            &token.mint[..8],
                                            price
                                        )
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        total_errors += 1;
                        if total_errors <= 3 && is_debug_api_enabled() {
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
        } else {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "DEBUG",
                    "🚫 No API calls needed - all tokens cached or skipped due to positions"
                );
            }
        }

        let elapsed = start_time.elapsed().as_millis();

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "BATCH_COMPLETE",
                &format!(
                    "Price batch: {}/{} tokens in {}ms (cached: {}, api_calls: {}, errors: {})",
                    prices.len(),
                    mints.len(),
                    elapsed,
                    cached_count,
                    api_call_mints.len(),
                    total_errors
                )
            );
        }

        prices
    }

    /// Get detailed token data for a single mint
    pub async fn get_token_data(&mut self, mint: &str) -> Result<Option<ApiToken>, String> {
        self.fetch_and_cache_token(mint).await
    }

    /// Get Token object from mint address (converts ApiToken to Token)
    pub async fn get_token_from_mint(&mut self, mint: &str) -> Result<Option<Token>, String> {
        match self.fetch_and_cache_token(mint).await? {
            Some(api_token) => Ok(Some(Token::from(api_token))),
            None => Ok(None),
        }
    }

    async fn fetch_and_cache_token(&mut self, mint: &str) -> Result<Option<ApiToken>, String> {
        if let Some(cached) = get_cached_token_data(mint).await {
            return Ok(Some(cached));
        }
        let tokens = self.get_tokens_info(&[mint.to_string()]).await?;
        if let Some(token) = tokens.into_iter().next() {
            cache_token_data(mint, &token).await;
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    /// Get token information for multiple mint addresses (main function)
    pub async fn get_tokens_info(&mut self, mints: &[String]) -> Result<Vec<ApiToken>, String> {
        if mints.is_empty() {
            if is_debug_api_enabled() {
                log(LogTag::Api, "DEBUG", "get_tokens_info called with empty mints array");
            }
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

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "DEBUG",
                &format!("🔍 DexScreener API request: {} tokens, URL: {}", mints.len(), if
                    url.len() > 100
                {
                    format!("{}...", &url[..100])
                } else {
                    url.clone()
                })
            );
            log(LogTag::Api, "DEBUG", &format!("📋 Mint addresses: {:?}", mints));
        }

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

        if is_debug_api_enabled() {
            // Log response structure without full content to avoid spam
            let response_info = if let Some(arr) = data.as_array() {
                format!("📥 API response: array with {} items", arr.len())
            } else if data.is_object() {
                format!(
                    "📥 API response: object with keys: {:?}",
                    data
                        .as_object()
                        .map(|obj| obj.keys().collect::<Vec<_>>())
                        .unwrap_or_default()
                )
            } else if data.is_null() {
                "📥 API response: null".to_string()
            } else {
                format!("📥 API response: {} type", if data.is_string() {
                    "string"
                } else if data.is_number() {
                    "number"
                } else if data.is_boolean() {
                    "boolean"
                } else {
                    "unknown"
                })
            };
            log(LogTag::Api, "DEBUG", &response_info);
        }

        let mut tokens = Vec::new();
        let mut rejected_non_sol_pairs = 0;
        let mut parsing_errors = 0;

        if let Some(pairs_array) = data.as_array() {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "DEBUG",
                    &format!("🔄 Processing {} pairs from API response", pairs_array.len())
                );

                if pairs_array.is_empty() {
                    log(
                        LogTag::Api,
                        "WARN",
                        &format!(
                            "⚠️ API returned empty array for {} tokens - these tokens may not exist on DexScreener",
                            mints.len()
                        )
                    );
                }
            }

            for (idx, pair_data) in pairs_array.iter().enumerate() {
                if is_debug_api_enabled() && idx < 3 {
                    // Log first few pairs for debugging
                    if let Some(base_token) = pair_data.get("baseToken") {
                        let mint = base_token
                            .get("address")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        let symbol = base_token
                            .get("symbol")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        log(
                            LogTag::Api,
                            "DEBUG",
                            &format!("🪙 Pair {}: {} ({})", idx + 1, symbol, &mint[..8])
                        );
                    }
                }

                match self.parse_token_from_pair(pair_data) {
                    Ok(token) => {
                        if is_debug_api_enabled() {
                            log(
                                LogTag::Api,
                                "DEBUG",
                                &format!(
                                    "✅ Successfully parsed token: {} ({})",
                                    token.symbol,
                                    &token.mint[..8]
                                )
                            );
                        }
                        tokens.push(token);
                    }
                    Err(e) => {
                        if e.contains("not paired with SOL") || e.contains("not a SOL pair") {
                            rejected_non_sol_pairs += 1;
                            if is_debug_api_enabled() {
                                log(LogTag::Api, "SOL_FILTER", &format!("Rejected: {}", e));
                            }
                        } else {
                            parsing_errors += 1;
                            if is_debug_api_enabled() {
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
        } else {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "WARN",
                    "⚠️ API response is not an array - this might be the issue!"
                );
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

        if is_debug_api_enabled() {
            log(LogTag::Api, "DEBUG", &format!("🔍 Parsing token: {}", &mint[..8]));
        }

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

        // CRITICAL: Only accept tokens paired with SOL
        let quote_token = pair_data.get("quoteToken");
        let (price_sol, is_sol_pair) = if let Some(qt) = quote_token {
            if let Some(quote_address) = qt.get("address").and_then(|v| v.as_str()) {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Api,
                        "DEBUG",
                        &format!("🔗 Token {} quote address: {}", &mint[..8], quote_address)
                    );
                }

                // Check if quote is SOL
                if quote_address == SOL_MINT {
                    let price_native_str = pair_data
                        .get("priceNative")
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");

                    let price_native = price_native_str
                        .parse::<f64>()
                        .map_err(|_| format!("Invalid price_native: {}", price_native_str))?;

                    if is_debug_api_enabled() {
                        log(
                            LogTag::Api,
                            "DEBUG",
                            &format!(
                                "✅ Token {} is SOL pair with price: {}",
                                &mint[..8],
                                price_native
                            )
                        );
                    }

                    (Some(price_native), true)
                } else {
                    // Reject non-SOL pairs
                    if is_debug_api_enabled() {
                        log(
                            LogTag::Api,
                            "DEBUG",
                            &format!(
                                "❌ Token {} rejected - not SOL pair (quote: {})",
                                &mint[..8],
                                quote_address
                            )
                        );
                    }
                    return Err(
                        format!("Token {} is not paired with SOL (quote: {})", mint, quote_address)
                    );
                }
            } else {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Api,
                        "DEBUG",
                        &format!("❌ Token {} rejected - no quote address", &mint[..8])
                    );
                }
                return Err(format!("Token {} has no quote address", mint));
            }
        } else {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "DEBUG",
                    &format!("❌ Token {} rejected - no quote token", &mint[..8])
                );
            }
            return Err(format!("Token {} has no quote token", mint));
        };

        // Only proceed if this is a SOL pair
        if !is_sol_pair {
            return Err(format!("Token {} is not a SOL pair", mint));
        }

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

        let price_usd = if let Some(usd_str) = pair_data.get("priceUsd").and_then(|v| v.as_str()) {
            usd_str.parse::<f64>().unwrap_or(0.0)
        } else {
            0.0
        };

        let price_native_str = pair_data
            .get("priceNative")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        let price_native = price_native_str
            .parse::<f64>()
            .map_err(|_| format!("Invalid price_native: {}", price_native_str))?;

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
                            .map(|url| WebsiteInfo {
                                url: url.to_string(),
                            })
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
            DiscoverySourceType::DexScreenerBoosts => {
                "https://api.dexscreener.com/token-boosts/latest/v1"
            }
            DiscoverySourceType::DexScreenerProfiles => {
                "https://api.dexscreener.com/token-profiles/latest/v1"
            }
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
            if is_debug_api_enabled() {
                log(LogTag::Api, "ERROR", &error_msg);
            }
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

    /// Get token pairs for multiple Solana tokens using batch endpoint (up to 30 tokens)
    pub async fn get_batch_solana_token_pairs(
        &mut self,
        token_addresses: &[String]
    ) -> Result<Vec<TokenPair>, String> {
        if token_addresses.is_empty() {
            return Ok(Vec::new());
        }

        if token_addresses.len() > MAX_TOKENS_PER_API_CALL {
            return Err(
                format!(
                    "Too many tokens for batch request: {}. Maximum is {}",
                    token_addresses.len(),
                    MAX_TOKENS_PER_API_CALL
                )
            );
        }

        // Join token addresses with commas for batch endpoint
        let token_list = token_addresses.join(",");
        let url = format!("https://api.dexscreener.com/tokens/v1/solana/{}", token_list);

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "REQUEST",
                &format!("Batch fetching pools for {} tokens", token_addresses.len())
            );
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
            .map_err(|e| format!("Failed to fetch batch token pairs: {}", e))?;

        drop(permit);

        let response_time = start_time.elapsed().as_millis() as f64;
        let success = response.status().is_success();

        self.stats.record_request(success, response_time);

        if !success {
            let error_msg = format!("Batch API request failed with status: {}", response.status());
            if is_debug_api_enabled() {
                log(LogTag::Api, "ERROR", &error_msg);
            }
            return Err(error_msg);
        }

        // Parse response - the batch endpoint returns an array of pairs directly
        let response_text = response
            .text().await
            .map_err(|e| format!("Failed to read batch response: {}", e))?;

        let pairs: Vec<TokenPair> = serde_json
            ::from_str(&response_text)
            .map_err(|e| format!("Failed to parse batch token pairs response: {}", e))?;

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "SUCCESS",
                &format!(
                    "Batch found {} pools for {} tokens ({:.0}ms)",
                    pairs.len(),
                    token_addresses.len(),
                    response_time
                )
            );
        }

        // Update last request time
        self.last_request_time = Some(Instant::now());

        Ok(pairs)
    }
}

/// Standalone function to get token pairs from API (improved timeout handling)
pub async fn get_token_pairs_from_api(token_address: &str) -> Result<Vec<TokenPair>, String> {
    let api = get_global_dexscreener_api().await?;

    // Use longer timeout to reduce timeout errors during system stress
    let result = timeout(Duration::from_secs(15), api.lock()).await;
    match result {
        Ok(mut api_instance) => api_instance.get_solana_token_pairs(token_address).await,
        Err(_) => {
            // Reduce log level to INFO since timeouts can be normal during shutdown
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "INFO",
                    "DexScreener API lock timeout in get_token_pairs_from_api (system may be shutting down)"
                );
            }
            Err("API lock timeout".to_string())
        }
    }
}

/// Get token pools from DexScreener API (consistent naming with GeckoTerminal and Raydium)
pub async fn get_token_pools_from_dexscreener(
    token_address: &str
) -> Result<Vec<TokenPair>, String> {
    get_token_pairs_from_api(token_address).await
}

/// Get token pairs for multiple tokens using the batch API endpoint
async fn get_batch_token_pairs_from_api(
    token_addresses: &[String]
) -> Result<Vec<TokenPair>, String> {
    let api = get_global_dexscreener_api().await?;

    // Use longer timeout to reduce timeout errors during system stress
    let result = timeout(Duration::from_secs(15), api.lock()).await;
    match result {
        Ok(mut api_instance) => api_instance.get_batch_solana_token_pairs(token_addresses).await,
        Err(_) => {
            // Reduce log level to INFO since timeouts can be normal during shutdown
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "INFO",
                    "DexScreener API lock timeout in get_batch_token_pairs_from_api (system may be shutting down)"
                );
            }
            Err("API lock timeout".to_string())
        }
    }
}

/// Batch result for multiple tokens from DexScreener
pub struct DexScreenerBatchResult {
    pub pools: HashMap<String, Vec<TokenPair>>,
    pub errors: HashMap<String, String>,
    pub successful_tokens: usize,
    pub failed_tokens: usize,
}

/// Get pools for multiple tokens in batch from DexScreener API using proper batch endpoint
pub async fn get_batch_token_pools_from_dexscreener(
    token_addresses: &[String]
) -> DexScreenerBatchResult {
    let start_time = std::time::Instant::now();

    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "DEXSCREENER_BATCH_START",
            &format!(
                "🟡 Starting DexScreener batch pool fetch for {} tokens using batch endpoint",
                token_addresses.len()
            )
        );
    }

    let mut pools = HashMap::new();
    let mut errors = HashMap::new();
    let mut successful_tokens = 0;
    let mut failed_tokens = 0;

    // Process tokens in chunks of MAX_TOKENS_PER_API_CALL (30) to use batch endpoint efficiently
    for (chunk_idx, chunk) in token_addresses.chunks(MAX_TOKENS_PER_API_CALL).enumerate() {
        // Rate limiting: delay between chunks (not individual tokens)
        if chunk_idx > 0 {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "DEXSCREENER_BATCH_CHUNK",
                &format!("📦 Processing chunk {}: {} tokens", chunk_idx + 1, chunk.len())
            );
        }

        match get_batch_token_pairs_from_api(chunk).await {
            Ok(batch_pairs) => {
                // Group pairs by token address
                let mut chunk_pools: HashMap<String, Vec<TokenPair>> = HashMap::new();

                for pair in batch_pairs {
                    // Determine which token this pair belongs to
                    let base_token_addr = &pair.base_token.address;
                    let quote_token_addr = &pair.quote_token.address;

                    // Find which of our requested tokens this pair represents
                    for token_addr in chunk {
                        if base_token_addr == token_addr || quote_token_addr == token_addr {
                            chunk_pools
                                .entry(token_addr.clone())
                                .or_insert_with(Vec::new)
                                .push(pair.clone());
                            break;
                        }
                    }
                }

                // Update results
                for token_addr in chunk {
                    if let Some(token_pools) = chunk_pools.remove(token_addr) {
                        if is_debug_api_enabled() {
                            log(
                                LogTag::Api,
                                "DEXSCREENER_BATCH_SUCCESS",
                                &format!(
                                    "✅ DexScreener batch: {} found {} pools",
                                    &token_addr[..8],
                                    token_pools.len()
                                )
                            );
                        }
                        pools.insert(token_addr.clone(), token_pools);
                        successful_tokens += 1;
                    } else {
                        // No pools found for this token
                        pools.insert(token_addr.clone(), Vec::new());
                        successful_tokens += 1;
                    }
                }
            }
            Err(e) => {
                // Mark all tokens in this chunk as failed
                for token_addr in chunk {
                    if is_debug_api_enabled() {
                        log(
                            LogTag::Api,
                            "DEXSCREENER_BATCH_ERROR",
                            &format!("❌ DexScreener batch: {} failed: {}", &token_addr[..8], e)
                        );
                    }
                    errors.insert(token_addr.clone(), e.clone());
                    failed_tokens += 1;
                }
            }
        }
    }

    let elapsed = start_time.elapsed();

    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "DEXSCREENER_BATCH_COMPLETE",
            &format!(
                "✅ DexScreener batch complete: {}/{} successful in {:.2}s ({} chunks)",
                successful_tokens,
                token_addresses.len(),
                elapsed.as_secs_f64(),
                (token_addresses.len() + MAX_TOKENS_PER_API_CALL - 1) / MAX_TOKENS_PER_API_CALL
            )
        );
    }

    DexScreenerBatchResult {
        pools,
        errors,
        successful_tokens,
        failed_tokens,
    }
}

// =============================================================================
// GLOBAL DEXSCREENER API SINGLETON (TRUE SINGLETON)
// =============================================================================

static GLOBAL_DEXSCREENER_API: OnceCell<Arc<Mutex<DexScreenerApi>>> = OnceCell::const_new();

/// Initialize the global DexScreener API client (creates single instance)
pub async fn init_dexscreener_api() -> Result<(), String> {
    // Check if already initialized
    if GLOBAL_DEXSCREENER_API.get().is_some() {
        return Ok(());
    }

    let api = Arc::new(Mutex::new(DexScreenerApi::new()));

    // Initialize the API instance once
    {
        let result = timeout(Duration::from_secs(20), api.lock()).await;
        match result {
            Ok(mut api_instance) => {
                api_instance.initialize().await?;
            }
            Err(_) => {
                if is_debug_api_enabled() {
                    log(LogTag::Api, "ERROR", "DexScreener API lock timeout during initialization");
                }
                return Err("API initialization lock timeout".to_string());
            }
        }
    }

    GLOBAL_DEXSCREENER_API.set(api).map_err(
        |_| "Failed to initialize global DexScreener API state"
    )?;

    // Initialization already logged inside DexScreenerApi::initialize(); avoid duplicate success log here
    Ok(())
}

/// Get reference to the global DexScreener API client
pub async fn get_global_dexscreener_api() -> Result<Arc<Mutex<DexScreenerApi>>, String> {
    GLOBAL_DEXSCREENER_API.get()
        .ok_or_else(|| {
            "DexScreener API not initialized. Call init_dexscreener_api() first.".to_string()
        })
        .map(|api| api.clone())
}

// (global helper wrappers removed – callers must lock global API and call methods directly)
