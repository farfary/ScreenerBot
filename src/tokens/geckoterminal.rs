/// GeckoTerminal API integration for pool discovery
///
/// This module provides pool fetching capabilities from GeckoTerminal API
/// to complement DexScreener data. Many tokens have pools on one platform
/// but not the other, so using both sources significantly improves coverage.
///
/// CRITICAL RATE LIMITING IMPLEMENTATION:
/// - MAXIMUM 30 calls per minute (conservative limit)
/// - MAXIMUM 1 concurrent call at any time (enforced by semaphore)
/// - MINIMUM 2 seconds between calls
/// - All functions use unified rate limiting to prevent conflicts
/// - Rate limit tracking includes both time-based and count-based limits
///
/// Key features:
/// - Batch token pool fetching (serialized to respect rate limits)
/// - Strict concurrency control (semaphore ensures single call)
/// - Pool data normalization to match DexScreener format
/// - Error handling and timeout management
/// - Debug logging for troubleshooting and rate limit monitoring
use crate::config::with_config;
use crate::global::is_debug_api_enabled;
use crate::logger::{log, LogTag};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

// =============================================================================
// GECKOTERMINAL API CONFIGURATION
// =============================================================================

/// GeckoTerminal API base URL
const GECKOTERMINAL_BASE_URL: &str = "https://api.geckoterminal.com/api/v2";

/// Default rate limit: 30 requests per minute (configurable)
const DEFAULT_GECKOTERMINAL_RATE_LIMIT_PER_MINUTE: usize = 30;

/// Rate limiting delay between requests (2000ms to ensure no concurrent calls)
const RATE_LIMIT_DELAY_MS: u64 = 2000;

/// Default maximum tokens per batch request (configurable)
const DEFAULT_MAX_TOKENS_PER_BATCH: usize = 30;

/// Request timeout in seconds
const REQUEST_TIMEOUT_SECS: u64 = 10;

/// API version header value for OHLCV requests
const API_VERSION: &str = "20230302";

/// Solana network identifier for GeckoTerminal
const SOLANA_NETWORK: &str = "solana";

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// GeckoTerminal API response for token pools
#[derive(Debug, Deserialize)]
struct GeckoTerminalResponse {
    data: Option<GeckoTerminalTokenData>,
    included: Option<Vec<GeckoTerminalIncluded>>,
}

/// Token data from GeckoTerminal
#[derive(Debug, Deserialize)]
struct GeckoTerminalTokenData {
    id: String,
    #[serde(rename = "type")]
    data_type: String,
    attributes: Option<GeckoTerminalTokenAttributes>,
    relationships: Option<GeckoTerminalRelationships>,
}

/// Token attributes
#[derive(Debug, Deserialize)]
struct GeckoTerminalTokenAttributes {
    address: Option<String>,
    name: Option<String>,
    symbol: Option<String>,
    decimals: Option<u8>,
}

/// Token relationships (links to pools)
#[derive(Debug, Deserialize)]
struct GeckoTerminalRelationships {
    top_pools: Option<GeckoTerminalTopPools>,
}

/// Top pools relationship
#[derive(Debug, Deserialize)]
struct GeckoTerminalTopPools {
    data: Option<Vec<GeckoTerminalPoolRef>>,
}

/// Pool reference
#[derive(Debug, Deserialize)]
struct GeckoTerminalPoolRef {
    id: String,
    #[serde(rename = "type")]
    ref_type: String,
}

/// Included data (pools, networks, etc.)
#[derive(Debug, Deserialize)]
struct GeckoTerminalIncluded {
    id: String,
    #[serde(rename = "type")]
    data_type: String,
    attributes: Option<serde_json::Value>,
    relationships: Option<serde_json::Value>,
}

/// Pool attributes from included data
#[derive(Debug, Deserialize)]
struct GeckoTerminalPoolAttributes {
    address: Option<String>,
    name: Option<String>,
    pool_created_at: Option<String>,
    base_token_price_usd: Option<String>,
    quote_token_price_usd: Option<String>,
    base_token_price_native_currency: Option<String>,
    quote_token_price_native_currency: Option<String>,
    pool_created_at_block_number: Option<u64>,
    fdv_usd: Option<String>,
    market_cap_usd: Option<String>,
    price_change_percentage: Option<HashMap<String, String>>,
    transactions: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
    volume_usd: Option<HashMap<String, String>>,
    reserve_in_usd: Option<String>,
}

/// Normalized pool information for compatibility with DexScreener format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoTerminalPool {
    pub pool_address: String,
    pub dex_id: String,
    pub base_token: String,
    pub quote_token: String,
    pub price_native: f64,
    pub price_usd: f64,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub created_at: u64,
    pub pool_name: Option<String>,
}

/// Batch result for multiple tokens
pub struct GeckoTerminalBatchResult {
    pub pools: HashMap<String, Vec<GeckoTerminalPool>>,
    pub errors: HashMap<String, String>,
    pub successful_tokens: usize,
    pub failed_tokens: usize,
}

/// OHLCV data point (SOL-denominated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcvDataPoint {
    /// Timestamp (Unix seconds)
    pub timestamp: i64,
    /// Open price in SOL
    pub open: f64,
    /// High price in SOL
    pub high: f64,
    /// Low price in SOL
    pub low: f64,
    /// Close price in SOL
    pub close: f64,
    /// Volume in SOL
    pub volume: f64,
}

/// GeckoTerminal OHLCV API response structures
#[derive(Debug, Deserialize)]
struct GeckoTerminalOhlcvResponse {
    data: GeckoTerminalOhlcvData,
}

#[derive(Debug, Deserialize)]
struct GeckoTerminalOhlcvData {
    id: String,
    #[serde(rename = "type")]
    data_type: String,
    attributes: GeckoTerminalOhlcvAttributes,
}

#[derive(Debug, Deserialize)]
struct GeckoTerminalOhlcvAttributes {
    ohlcv_list: Vec<Vec<f64>>, // [timestamp, open, high, low, close, volume]
}

// =============================================================================
// RATE LIMITING AND CONCURRENCY CONTROL
// =============================================================================

/// Rate limiting state to track both time and call count
#[derive(Debug)]
struct RateLimitState {
    last_request: Instant,
    call_timestamps: Vec<Instant>,
}

impl RateLimitState {
    fn new() -> Self {
        Self {
            last_request: Instant::now() - Duration::from_secs(60), // Initialize in the past
            call_timestamps: Vec::new(),
        }
    }

    fn cleanup_old_calls(&mut self) {
        let one_minute_ago = Instant::now() - Duration::from_secs(60);
        self.call_timestamps
            .retain(|&timestamp| timestamp > one_minute_ago);
    }

    fn can_make_request(&mut self, max_calls_per_minute: usize) -> bool {
        self.cleanup_old_calls();

        // Check if we've hit the rate limit (30 calls per minute)
        if self.call_timestamps.len() >= max_calls_per_minute {
            return false;
        }

        // Check if enough time has passed since last request (2 seconds minimum)
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_request);
        elapsed >= Duration::from_millis(RATE_LIMIT_DELAY_MS)
    }

    fn record_request(&mut self) {
        let now = Instant::now();
        self.last_request = now;
        self.call_timestamps.push(now);
        self.cleanup_old_calls();
    }

    fn time_until_next_request(&self) -> Option<Duration> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_request);

        if elapsed < Duration::from_millis(RATE_LIMIT_DELAY_MS) {
            Some(Duration::from_millis(RATE_LIMIT_DELAY_MS) - elapsed)
        } else {
            None
        }
    }

    fn time_until_rate_limit_reset(&self, max_calls_per_minute: usize) -> Option<Duration> {
        if self.call_timestamps.len() < max_calls_per_minute {
            return None;
        }

        if let Some(&oldest_call) = self.call_timestamps.first() {
            let one_minute_from_oldest = oldest_call + Duration::from_secs(60);
            let now = Instant::now();

            if one_minute_from_oldest > now {
                Some(one_minute_from_oldest - now)
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Global rate limiter and concurrency control for GeckoTerminal API
/// This ensures only ONE call at a time and tracks rate limits properly
static GECKO_API_SEMAPHORE: tokio::sync::OnceCell<Arc<tokio::sync::Semaphore>> =
    tokio::sync::OnceCell::const_new();
static GECKO_RATE_LIMITER: tokio::sync::OnceCell<Arc<Mutex<RateLimitState>>> =
    tokio::sync::OnceCell::const_new();

/// Initialize the semaphore for single concurrent call
async fn get_api_semaphore() -> Arc<tokio::sync::Semaphore> {
    GECKO_API_SEMAPHORE
        .get_or_init(|| async {
            Arc::new(tokio::sync::Semaphore::new(1)) // Only 1 concurrent call allowed
        })
        .await
        .clone()
}

/// Initialize the rate limiter
async fn get_rate_limiter() -> Arc<Mutex<RateLimitState>> {
    GECKO_RATE_LIMITER
        .get_or_init(|| async { Arc::new(Mutex::new(RateLimitState::new())) })
        .await
        .clone()
}

/// Acquire a GeckoTerminal permit with optional timeout handling so callers can
/// respect their own latency budgets while participating in the shared rate limiter.
async fn acquire_gecko_permit_inner(
    max_wait: Option<Duration>,
    context: Option<&str>,
) -> Result<Option<tokio::sync::OwnedSemaphorePermit>, String> {
    let context_label = context.unwrap_or("GeckoTerminal request");
    let start = Instant::now();

    let semaphore = get_api_semaphore().await;
    let permit = if let Some(wait) = max_wait {
        match timeout(wait, semaphore.clone().acquire_owned()).await {
            Ok(result) => result.map_err(|e| format!("Failed to acquire semaphore: {}", e))?,
            Err(_) => {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Api,
                        "GECKO_RATE_LIMIT_TIMEOUT",
                        &format!(
                            "🦎 {} timed out after {}ms waiting for GeckoTerminal semaphore",
                            context_label,
                            wait.as_millis()
                        ),
                    );
                }
                return Ok(None);
            }
        }
    } else {
        semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("Failed to acquire semaphore: {}", e))?
    };

    let rate_limiter = get_rate_limiter().await;

    loop {
        if let Some(wait) = max_wait {
            if wait <= start.elapsed() {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Api,
                        "GECKO_RATE_LIMIT_SKIPPED",
                        &format!(
                            "🦎 {} skipped: exhausted {}ms wait budget before permit was available",
                            context_label,
                            wait.as_millis()
                        ),
                    );
                }
                drop(permit);
                return Ok(None);
            }
        }

        let max_calls_per_minute = geckoterminal_rate_limit_per_minute().max(1);
        let mut state = rate_limiter.lock().await;

        if state.can_make_request(max_calls_per_minute) {
            state.record_request();
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_RATE_LIMIT",
                    &format!(
                        "🦎 {} allowed ({}/{}) calls in last minute",
                        context_label,
                        state.call_timestamps.len(),
                        max_calls_per_minute
                    ),
                );
            }
            drop(state);
            return Ok(Some(permit));
        }

        let mut delay = state
            .time_until_rate_limit_reset(max_calls_per_minute)
            .or_else(|| state.time_until_next_request())
            .unwrap_or(Duration::from_millis(RATE_LIMIT_DELAY_MS));

        if let Some(wait) = max_wait {
            match wait.checked_sub(start.elapsed()) {
                Some(remaining) if remaining.is_zero() => {
                    if is_debug_api_enabled() {
                        log(
                            LogTag::Api,
                            "GECKO_RATE_LIMIT_SKIPPED",
                            &format!(
                                "🦎 {} skipped: no remaining wait budget for GeckoTerminal request",
                                context_label
                            ),
                        );
                    }
                    drop(state);
                    drop(permit);
                    return Ok(None);
                }
                Some(remaining) => {
                    if remaining <= delay {
                        if is_debug_api_enabled() {
                            log(
                                LogTag::Api,
                                "GECKO_RATE_LIMIT_SKIPPED",
                                &format!(
                                    "🦎 {} skipped: delay {}ms exceeds remaining {}ms budget",
                                    context_label,
                                    delay.as_millis(),
                                    remaining.as_millis()
                                ),
                            );
                        }
                        drop(state);
                        drop(permit);
                        return Ok(None);
                    }
                    delay = delay.min(remaining);
                }
                None => {
                    if is_debug_api_enabled() {
                        log(
                            LogTag::Api,
                            "GECKO_RATE_LIMIT_SKIPPED",
                            &format!(
                                "🦎 {} skipped: GeckoTerminal wait budget already exhausted",
                                context_label
                            ),
                        );
                    }
                    drop(state);
                    drop(permit);
                    return Ok(None);
                }
            }
        }

        drop(state);
        sleep(delay).await;
    }
}

/// Apply strict rate limiting and concurrency control before making API requests
/// Returns a guard that must be held for the duration of the API call
async fn apply_rate_limit_and_concurrency_control(
) -> Result<tokio::sync::OwnedSemaphorePermit, String> {
    acquire_gecko_permit_inner(None, None)
        .await?
        .ok_or_else(|| "Failed to acquire GeckoTerminal permit".to_string())
}

// =============================================================================
// CORE FUNCTIONS
// =============================================================================

/// Fetch pools for a single token from GeckoTerminal
pub async fn get_token_pools_from_geckoterminal(
    token_address: &str,
) -> Result<Vec<GeckoTerminalPool>, String> {
    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "GECKO_START",
            &format!(
                "🦎 Fetching pools for {} from GeckoTerminal",
                &token_address[..8]
            ),
        );
    }

    // Apply strict rate limiting and get exclusive access
    let _permit = apply_rate_limit_and_concurrency_control().await?;

    let url = format!(
        "{}/networks/solana/tokens/{}?include=top_pools",
        GECKOTERMINAL_BASE_URL, token_address
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = match timeout(
        Duration::from_secs(REQUEST_TIMEOUT_SECS),
        client.get(&url).send(),
    )
    .await
    {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_ERROR",
                    &format!("HTTP error for {}: {}", &token_address[..8], e),
                );
            }
            return Err(format!("HTTP request failed: {}", e));
        }
        Err(_) => {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_TIMEOUT",
                    &format!("Request timeout for {}", &token_address[..8]),
                );
            }
            return Err("Request timeout".to_string());
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_RATE_LIMIT",
                    &format!("Rate limited for {}, waiting 5s", &token_address[..8]),
                );
            }
            // Wait longer for rate limit recovery
            sleep(Duration::from_secs(5)).await;
            return Err("Rate limited".to_string());
        }

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "GECKO_STATUS_ERROR",
                &format!("HTTP {} for {}", status, &token_address[..8]),
            );
        }
        return Err(format!("HTTP {}", status));
    }

    let body = match response.text().await {
        Ok(body) => body,
        Err(e) => {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_BODY_ERROR",
                    &format!(
                        "Failed to read response body for {}: {}",
                        &token_address[..8],
                        e
                    ),
                );
            }
            return Err(format!("Failed to read response: {}", e));
        }
    };

    let gecko_response: GeckoTerminalResponse = match serde_json::from_str(&body) {
        Ok(response) => response,
        Err(e) => {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_PARSE_ERROR",
                    &format!(
                        "Failed to parse JSON for {}: {} (body preview: {})",
                        &token_address[..8],
                        e,
                        &body[..std::cmp::min(200, body.len())]
                    ),
                );
            }
            return Err(format!("Failed to parse JSON: {}", e));
        }
    };

    let pools = parse_geckoterminal_pools(&gecko_response, token_address)?;

    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "GECKO_SUCCESS",
            &format!(
                "🦎 Found {} pools for {} from GeckoTerminal",
                pools.len(),
                &token_address[..8]
            ),
        );
    }

    Ok(pools)
}

/// Fetch pools for multiple tokens in a single batch request
/// Note: GeckoTerminal doesn't support true batch requests, so we serialize individual calls
/// with strict rate limiting to ensure no concurrent calls and respect 30 calls/minute limit
pub async fn get_batch_token_pools_from_geckoterminal(
    token_addresses: &[String],
) -> GeckoTerminalBatchResult {
    let mut result = GeckoTerminalBatchResult {
        pools: HashMap::new(),
        errors: HashMap::new(),
        successful_tokens: 0,
        failed_tokens: 0,
    };

    if token_addresses.is_empty() {
        return result;
    }

    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "GECKO_BATCH_START",
            &format!(
                "🦎 Batch fetching pools for {} tokens from GeckoTerminal (SERIALIZED)",
                token_addresses.len()
            ),
        );
    }

    // Process tokens one by one to ensure no concurrent calls
    // This is required by GeckoTerminal's strict rate limiting
    let max_batch = max_tokens_per_batch_config();
    for token_address in token_addresses.iter().take(max_batch) {
        match get_token_pools_from_geckoterminal(token_address).await {
            Ok(pools) => {
                if !pools.is_empty() {
                    result.pools.insert(token_address.clone(), pools);
                    result.successful_tokens += 1;

                    if is_debug_api_enabled() {
                        log(
                            LogTag::Api,
                            "GECKO_BATCH_SUCCESS",
                            &format!(
                                "🦎 Success for {}: {} pools found",
                                &token_address[..8],
                                result.pools.get(token_address).unwrap().len()
                            ),
                        );
                    }
                } else {
                    if is_debug_api_enabled() {
                        log(
                            LogTag::Api,
                            "GECKO_BATCH_NO_POOLS",
                            &format!("🦎 No pools found for {}", &token_address[..8]),
                        );
                    }
                }
            }
            Err(error) => {
                result.errors.insert(token_address.clone(), error.clone());
                result.failed_tokens += 1;

                if is_debug_api_enabled() {
                    log(
                        LogTag::Api,
                        "GECKO_BATCH_ERROR",
                        &format!("🦎 Error for {}: {}", &token_address[..8], error),
                    );
                }
            }
        }
    }

    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "GECKO_BATCH_COMPLETE",
            &format!(
                "🦎 GeckoTerminal batch complete: {}/{} successful, {} errors",
                result.successful_tokens,
                result.successful_tokens + result.failed_tokens,
                result.failed_tokens
            ),
        );
    }

    result
}

/// Parse GeckoTerminal API response into normalized pool format
fn parse_geckoterminal_pools(
    response: &GeckoTerminalResponse,
    token_address: &str,
) -> Result<Vec<GeckoTerminalPool>, String> {
    let mut pools = Vec::new();

    // Get token data
    let token_data = response.data.as_ref().ok_or("No token data in response")?;

    // Get pool references from relationships
    let pool_refs = if let Some(relationships) = &token_data.relationships {
        if let Some(top_pools) = &relationships.top_pools {
            if let Some(data) = &top_pools.data {
                data.clone()
            } else {
                return Ok(pools); // No pools found
            }
        } else {
            return Ok(pools); // No top_pools relationship
        }
    } else {
        return Ok(pools); // No relationships
    };

    // Get included data (pool details)
    let included = response
        .included
        .as_ref()
        .ok_or("No included data in response")?;

    // Match pool references with included pool data
    for pool_ref in pool_refs {
        if pool_ref.ref_type != "pool" {
            continue;
        }

        // Find matching pool in included data
        if let Some(pool_data) = included
            .iter()
            .find(|item| item.id == pool_ref.id && item.data_type == "pool")
        {
            if let Some(pool) = parse_single_pool(pool_data, token_address)? {
                pools.push(pool);
            }
        }
    }

    // Sort by liquidity (highest first)
    pools.sort_by(|a, b| {
        b.liquidity_usd
            .partial_cmp(&a.liquidity_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(pools)
}

/// Parse a single pool from included data
fn parse_single_pool(
    pool_data: &GeckoTerminalIncluded,
    token_address: &str,
) -> Result<Option<GeckoTerminalPool>, String> {
    let attributes = pool_data.attributes.as_ref().ok_or("No pool attributes")?;

    // Parse attributes as GeckoTerminalPoolAttributes
    let pool_attrs: GeckoTerminalPoolAttributes = serde_json::from_value(attributes.clone())
        .map_err(|e| format!("Failed to parse pool attributes: {}", e))?;

    let pool_address = pool_attrs.address.ok_or("Missing pool address")?;

    // Parse price and liquidity data
    let price_usd = pool_attrs
        .base_token_price_usd
        .and_then(|p| p.parse::<f64>().ok())
        .unwrap_or(0.0);

    let price_native = pool_attrs
        .base_token_price_native_currency
        .and_then(|p| p.parse::<f64>().ok())
        .unwrap_or(0.0);

    let liquidity_usd = pool_attrs
        .reserve_in_usd
        .and_then(|l| l.parse::<f64>().ok())
        .unwrap_or(0.0);

    let volume_24h = pool_attrs
        .volume_usd
        .as_ref()
        .and_then(|v| v.get("h24"))
        .and_then(|v| Some(v.as_str()))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let created_at = pool_attrs.pool_created_at_block_number.unwrap_or(0);

    let pool = GeckoTerminalPool {
        pool_address,
        dex_id: "geckoterminal".to_string(),
        base_token: token_address.to_string(),
        quote_token: "So11111111111111111111111111111111111111112".to_string(), // Assume SOL for now
        price_native,
        price_usd,
        liquidity_usd,
        volume_24h,
        created_at,
        pool_name: pool_attrs.name,
    };

    Ok(Some(pool))
}

/// Fetch 1-minute OHLCV data from GeckoTerminal API (SOL-denominated)
pub async fn get_ohlcv_data_from_geckoterminal(
    pool_address: &str,
    target_mint: &str,
    limit: u32,
) -> Result<Vec<OhlcvDataPoint>, String> {
    // Apply strict rate limiting and get exclusive access
    let _permit = apply_rate_limit_and_concurrency_control().await?;

    // GeckoTerminal enforces a maximum limit (typically 1000)
    const GECKO_OHLCV_MAX_LIMIT: u32 = 1000;
    let effective_limit = if limit > GECKO_OHLCV_MAX_LIMIT {
        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "GECKO_OHLCV_LIMIT_CLAMP",
                &format!(
                    "Clamping requested OHLCV limit from {} to {} for pool {}",
                    limit, GECKO_OHLCV_MAX_LIMIT, pool_address
                ),
            );
        }
        GECKO_OHLCV_MAX_LIMIT
    } else {
        limit
    };

    let url = format!(
        "{}/networks/{}/pools/{}/ohlcv/minute",
        GECKOTERMINAL_BASE_URL, SOLANA_NETWORK, pool_address
    );

    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "GECKO_OHLCV_START",
            &format!(
                "🦎 Fetching 1m SOL-denominated OHLCV for pool {} (limit: {})",
                pool_address, effective_limit
            ),
        );
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = match timeout(
        Duration::from_secs(REQUEST_TIMEOUT_SECS),
        client
            .get(&url)
            .header(
                "Accept",
                format!("application/json;version={}", API_VERSION),
            )
            .query(&[
                ("aggregate", "1".to_string()),
                ("limit", effective_limit.to_string()),
                ("currency", "token".to_string()),
                ("token", target_mint.to_string()),
            ])
            .send(),
    )
    .await
    {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_OHLCV_ERROR",
                    &format!("HTTP error for pool {}: {}", pool_address, e),
                );
            }
            return Err(format!("HTTP request failed: {}", e));
        }
        Err(_) => {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_OHLCV_TIMEOUT",
                    &format!("Request timeout for pool {}", pool_address),
                );
            }
            return Err("Request timeout".to_string());
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_OHLCV_RATE_LIMIT",
                    &format!("Rate limited for pool {}, waiting 10s", pool_address),
                );
            }
            sleep(Duration::from_secs(10)).await;
            return Err("Rate limited".to_string());
        }

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "GECKO_OHLCV_STATUS_ERROR",
                &format!("HTTP {} for pool {}", status, pool_address),
            );
        }

        match status.as_u16() {
            404 => {
                return Err(format!("Pool not found: {}", pool_address));
            }
            400 => {
                return Err("Bad request - invalid parameters".to_string());
            }
            500..=599 => {
                return Err(format!("Server error ({})", status));
            }
            _ => {
                return Err(format!("HTTP {}", status));
            }
        }
    }

    let body = match response.text().await {
        Ok(body) => body,
        Err(e) => {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_OHLCV_BODY_ERROR",
                    &format!(
                        "Failed to read response body for pool {}: {}",
                        &pool_address[..8],
                        e
                    ),
                );
            }
            return Err(format!("Failed to read response: {}", e));
        }
    };

    let gecko_response: GeckoTerminalOhlcvResponse = match serde_json::from_str(&body) {
        Ok(response) => response,
        Err(e) => {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_OHLCV_PARSE_ERROR",
                    &format!(
                        "Failed to parse JSON for pool {}: {} (body preview: {})",
                        pool_address,
                        e,
                        &body[..std::cmp::min(200, body.len())]
                    ),
                );
            }
            return Err(format!("Failed to parse JSON: {}", e));
        }
    };

    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "GECKO_OHLCV_RESPONSE",
            &format!(
                "✅ GeckoTerminal OHLCV response: type={}, id={}, {} data points",
                gecko_response.data.data_type,
                gecko_response.data.id,
                gecko_response.data.attributes.ohlcv_list.len()
            ),
        );
    }

    let data_points: Result<Vec<OhlcvDataPoint>, String> = gecko_response
        .data
        .attributes
        .ohlcv_list
        .into_iter()
        .map(|ohlcv| {
            if ohlcv.len() != 6 {
                return Err(format!(
                    "Invalid OHLCV data format: expected 6 values, got {}",
                    ohlcv.len()
                ));
            }

            let timestamp = ohlcv[0] as i64;
            let open = ohlcv[1];
            let high = ohlcv[2];
            let low = ohlcv[3];
            let close = ohlcv[4];
            let volume = ohlcv[5];

            // Validate data integrity
            if timestamp <= 0 {
                return Err(format!("Invalid timestamp: {}", timestamp));
            }

            if open <= 0.0 || high <= 0.0 || low <= 0.0 || close <= 0.0 {
                return Err(format!(
                    "Invalid price data: open={}, high={}, low={}, close={}",
                    open, high, low, close
                ));
            }

            if volume < 0.0 {
                return Err(format!("Invalid volume: {}", volume));
            }

            if high < low {
                return Err(format!(
                    "Invalid OHLC relationship: high ({}) < low ({})",
                    high, low
                ));
            }

            if open > high || open < low || close > high || close < low {
                return Err(format!(
                    "OHLC values out of range: open={}, high={}, low={}, close={}",
                    open, high, low, close
                ));
            }

            if !open.is_finite()
                || !high.is_finite()
                || !low.is_finite()
                || !close.is_finite()
                || !volume.is_finite()
            {
                return Err("Non-finite values in OHLCV data".to_string());
            }

            Ok(OhlcvDataPoint {
                timestamp,
                open,
                high,
                low,
                close,
                volume,
            })
        })
        .collect();

    let result = data_points?;

    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "GECKO_OHLCV_SUCCESS",
            &format!(
                "🦎 Retrieved {} SOL-denominated OHLCV data points for pool {}",
                result.len(),
                pool_address
            ),
        );
    }

    Ok(result)
}

/// Fetch 1-minute SOL-denominated OHLCV data for a pool within an optional time window by paging backwards.
/// If start_timestamp and end_timestamp are provided, this function fetches pages using
/// `before_timestamp` until the window is covered or the page limit is reached.
/// If only end_timestamp is provided, it fetches up to `limit` points strictly before that time.
/// If neither is provided, it behaves like `get_ohlcv_data_from_geckoterminal` with the given limit.
pub async fn get_ohlcv_data_from_geckoterminal_range(
    pool_address: &str,
    target_mint: &str,
    start_timestamp: Option<i64>,
    end_timestamp: Option<i64>,
    limit: u32,
) -> Result<Vec<OhlcvDataPoint>, String> {
    // Fast path: no window provided -> fall back to simple fetch
    if start_timestamp.is_none() && end_timestamp.is_none() {
        return get_ohlcv_data_from_geckoterminal(pool_address, target_mint, limit).await;
    }

    // GeckoTerminal maximum page size
    const GECKO_OHLCV_MAX_LIMIT: u32 = 1000;
    let mut remaining = limit.min(GECKO_OHLCV_MAX_LIMIT * 20); // hard safety cap ~20k points

    // Start from the inclusive end bound; if not provided, use "now"
    let mut current_before = end_timestamp.unwrap_or_else(|| chrono::Utc::now().timestamp());
    let start_bound = start_timestamp.unwrap_or(0);

    let mut all_points: Vec<OhlcvDataPoint> = Vec::new();

    loop {
        if remaining == 0 {
            break;
        }

        // Page size is min(remaining, 1000)
        let page_limit = remaining.min(GECKO_OHLCV_MAX_LIMIT);

        // Apply strict rate limiting and get exclusive access
        let _permit = apply_rate_limit_and_concurrency_control().await?;

        let url = format!(
            "{}/networks/{}/pools/{}/ohlcv/minute",
            GECKOTERMINAL_BASE_URL, SOLANA_NETWORK, pool_address
        );

        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "GECKO_OHLCV_RANGE_START",
                &format!(
                    "🦎 Paging 1m OHLCV for pool {} (page_limit: {}, before: {}), start_bound: {}",
                    pool_address, page_limit, current_before, start_bound
                ),
            );
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = match timeout(
            Duration::from_secs(REQUEST_TIMEOUT_SECS),
            client
                .get(&url)
                .header(
                    "Accept",
                    format!("application/json;version={}", API_VERSION),
                )
                .query(&[
                    ("aggregate", "1".to_string()),
                    ("limit", page_limit.to_string()),
                    ("before_timestamp", current_before.to_string()),
                    ("currency", "token".to_string()),
                    ("token", target_mint.to_string()),
                ])
                .send(),
        )
        .await
        {
            Ok(Ok(response)) => response,
            Ok(Err(e)) => {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Api,
                        "GECKO_OHLCV_RANGE_ERROR",
                        &format!("HTTP error for pool {}: {}", pool_address, e),
                    );
                }
                return Err(format!("HTTP request failed: {}", e));
            }
            Err(_) => {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Api,
                        "GECKO_OHLCV_RANGE_TIMEOUT",
                        &format!("Request timeout for pool {}", pool_address),
                    );
                }
                return Err("Request timeout".to_string());
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            if status == StatusCode::TOO_MANY_REQUESTS {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Api,
                        "GECKO_OHLCV_RANGE_RATE_LIMIT",
                        &format!("Rate limited for pool {}, waiting 10s", pool_address),
                    );
                }
                sleep(Duration::from_secs(10)).await;
                continue; // try next loop after backoff
            }

            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_OHLCV_RANGE_STATUS_ERROR",
                    &format!("HTTP {} for pool {}", status, pool_address),
                );
            }
            // On non-429 errors break to avoid infinite loop
            return Err(format!("HTTP {}", status));
        }

        let body = match response.text().await {
            Ok(body) => body,
            Err(e) => {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Api,
                        "GECKO_OHLCV_RANGE_BODY_ERROR",
                        &format!(
                            "Failed to read response body for pool {}: {}",
                            &pool_address[..8],
                            e
                        ),
                    );
                }
                return Err(format!("Failed to read response: {}", e));
            }
        };

        let gecko_response: GeckoTerminalOhlcvResponse = match serde_json::from_str(&body) {
            Ok(response) => response,
            Err(e) => {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Api,
                        "GECKO_OHLCV_RANGE_PARSE_ERROR",
                        &format!(
                            "Failed to parse JSON for pool {}: {} (body preview: {})",
                            pool_address,
                            e,
                            &body[..std::cmp::min(200, body.len())]
                        ),
                    );
                }
                return Err(format!("Failed to parse JSON: {}", e));
            }
        };

        let mut page_points: Vec<OhlcvDataPoint> = Vec::new();
        let mut oldest_ts_in_page: Option<i64> = None;

        for ohlcv in gecko_response.data.attributes.ohlcv_list.into_iter() {
            if ohlcv.len() != 6 {
                continue;
            }
            let ts = ohlcv[0] as i64;
            let open = ohlcv[1];
            let high = ohlcv[2];
            let low = ohlcv[3];
            let close = ohlcv[4];
            let volume = ohlcv[5];

            if ts < start_bound {
                continue;
            }
            if ts > current_before {
                continue;
            }

            page_points.push(OhlcvDataPoint {
                timestamp: ts,
                open,
                high,
                low,
                close,
                volume,
            });
            oldest_ts_in_page = Some(oldest_ts_in_page.map(|o| o.min(ts)).unwrap_or(ts));
        }

        if page_points.is_empty() {
            // No more data or nothing in range
            break;
        }

        // Append and update counters
        remaining = remaining.saturating_sub(page_points.len() as u32);
        all_points.extend(page_points);

        // Move the window back by 60s before the oldest ts
        if let Some(oldest) = oldest_ts_in_page {
            if oldest <= start_bound {
                break;
            }
            current_before = oldest - 60;
        } else {
            break;
        }
    }

    // Sort chronologically ascending for consumers that expect ordered data
    all_points.sort_by_key(|p| p.timestamp);

    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "GECKO_OHLCV_RANGE_SUCCESS",
            &format!(
                "🦎 Retrieved {} OHLCV points in range for pool {} (start: {}, end: {})",
                all_points.len(),
                pool_address,
                start_bound,
                end_timestamp.unwrap_or(0)
            ),
        );
    }

    Ok(all_points)
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Get rate limit information
pub fn get_rate_limit_info() -> (usize, usize) {
    (
        geckoterminal_rate_limit_per_minute(),
        max_tokens_per_batch_config(),
    )
}

/// Get current rate limit status (async because it needs to check the state)
pub async fn get_current_rate_limit_status() -> (usize, usize, Option<u64>) {
    let rate_limiter = get_rate_limiter().await;
    let mut state = rate_limiter.lock().await;
    state.cleanup_old_calls();

    let current_calls = state.call_timestamps.len();
    let max_calls = geckoterminal_rate_limit_per_minute();
    let reset_in_ms = state
        .time_until_rate_limit_reset(max_calls)
        .map(|d| d.as_millis() as u64);

    (current_calls, max_calls, reset_in_ms)
}

/// Acquire a GeckoTerminal API rate limit permit for external callers (e.g. OHLCV module)
pub async fn acquire_gecko_api_permit() -> Result<tokio::sync::OwnedSemaphorePermit, String> {
    apply_rate_limit_and_concurrency_control().await
}

/// Attempt to acquire a permit but give up if the wait would exceed the caller's budget.
/// Returns `Ok(None)` when the caller should defer the request for a future cycle.
pub async fn try_acquire_gecko_api_permit(
    max_wait: Duration,
    context: &str,
) -> Result<Option<tokio::sync::OwnedSemaphorePermit>, String> {
    acquire_gecko_permit_inner(Some(max_wait), Some(context)).await
}

fn geckoterminal_rate_limit_per_minute() -> usize {
    let configured = with_config(|cfg| cfg.tokens.geckoterminal_rate_limit_per_minute);
    if configured == 0 {
        DEFAULT_GECKOTERMINAL_RATE_LIMIT_PER_MINUTE
    } else {
        configured
    }
}

fn max_tokens_per_batch_config() -> usize {
    let configured = with_config(|cfg| cfg.tokens.max_tokens_per_batch);
    if configured == 0 {
        DEFAULT_MAX_TOKENS_PER_BATCH
    } else {
        configured
    }
}
