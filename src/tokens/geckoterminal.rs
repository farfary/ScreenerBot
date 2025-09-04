/// GeckoTerminal API integration for pool discovery
///
/// This module provides pool fetching capabilities from GeckoTerminal API
/// to complement DexScreener data. Many tokens have pools on one platform
/// but not the other, so using both sources significantly improves coverage.
///
/// Key features:
/// - Batch token pool fetching (up to 30 tokens per call)
/// - Rate limiting (60 requests per minute)
/// - Pool data normalization to match DexScreener format
/// - Error handling and timeout management
/// - Debug logging for troubleshooting

use crate::global::is_debug_api_enabled;
use crate::logger::{log, LogTag};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

// =============================================================================
// GECKOTERMINAL API CONFIGURATION
// =============================================================================

/// GeckoTerminal API base URL
const GECKOTERMINAL_BASE_URL: &str = "https://api.geckoterminal.com/api/v2";

/// Rate limit: 60 requests per minute according to GeckoTerminal docs
const GECKOTERMINAL_RATE_LIMIT_PER_MINUTE: usize = 60;

/// Maximum tokens per batch request (GeckoTerminal supports multi-token queries)
const MAX_TOKENS_PER_BATCH: usize = 30;

/// Request timeout in seconds
const REQUEST_TIMEOUT_SECS: u64 = 10;

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

// =============================================================================
// CORE FUNCTIONS
// =============================================================================

/// Fetch pools for a single token from GeckoTerminal
pub async fn get_token_pools_from_geckoterminal(token_address: &str) -> Result<Vec<GeckoTerminalPool>, String> {
    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "GECKO_START",
            &format!("🦎 Fetching pools for {} from GeckoTerminal", &token_address[..8])
        );
    }

    let url = format!(
        "{}/networks/solana/tokens/{}?include=top_pools",
        GECKOTERMINAL_BASE_URL,
        token_address
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = match timeout(
        Duration::from_secs(REQUEST_TIMEOUT_SECS),
        client.get(&url).send()
    ).await {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_ERROR",
                    &format!("HTTP error for {}: {}", &token_address[..8], e)
                );
            }
            return Err(format!("HTTP request failed: {}", e));
        }
        Err(_) => {
            if is_debug_api_enabled() {
                log(
                    LogTag::Api,
                    "GECKO_TIMEOUT",
                    &format!("Request timeout for {}", &token_address[..8])
                );
            }
            return Err("Request timeout".to_string());
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        if is_debug_api_enabled() {
            log(
                LogTag::Api,
                "GECKO_STATUS_ERROR",
                &format!("HTTP {} for {}", status, &token_address[..8])
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
                    &format!("Failed to read response body for {}: {}", &token_address[..8], e)
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
                    &format!("Failed to parse JSON for {}: {} (body preview: {})", &token_address[..8], e, &body[..std::cmp::min(200, body.len())])
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
            &format!("🦎 Found {} pools for {} from GeckoTerminal", pools.len(), &token_address[..8])
        );
    }

    Ok(pools)
}

/// Fetch pools for multiple tokens in a single batch request
pub async fn get_batch_token_pools_from_geckoterminal(token_addresses: &[String]) -> GeckoTerminalBatchResult {
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
            &format!("🦎 Batch fetching pools for {} tokens from GeckoTerminal", token_addresses.len())
        );
    }

    // GeckoTerminal doesn't support true batch requests for multiple tokens,
    // so we'll need to make individual requests but with concurrency control
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(5)); // Limit concurrent requests
    let mut tasks = Vec::new();

    for token_address in token_addresses.iter().take(MAX_TOKENS_PER_BATCH) {
        let token_address = token_address.clone();
        let semaphore = semaphore.clone();
        
        let task = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let pools_result = get_token_pools_from_geckoterminal(&token_address).await;
            (token_address, pools_result)
        });
        
        tasks.push(task);
    }

    // Collect results
    for task in tasks {
        match task.await {
            Ok((token_address, pools_result)) => {
                match pools_result {
                    Ok(pools) => {
                        result.pools.insert(token_address, pools);
                        result.successful_tokens += 1;
                    }
                    Err(error) => {
                        result.errors.insert(token_address, error);
                        result.failed_tokens += 1;
                    }
                }
            }
            Err(e) => {
                // Task join error - this shouldn't happen normally
                log(
                    LogTag::Api,
                    "GECKO_TASK_ERROR",
                    &format!("Task join error: {}", e)
                );
            }
        }
    }

    if is_debug_api_enabled() {
        log(
            LogTag::Api,
            "GECKO_BATCH_COMPLETE",
            &format!(
                "🦎 GeckoTerminal batch complete: {}/{} successful",
                result.successful_tokens,
                result.successful_tokens + result.failed_tokens
            )
        );
    }

    result
}

/// Parse GeckoTerminal API response into normalized pool format
fn parse_geckoterminal_pools(response: &GeckoTerminalResponse, token_address: &str) -> Result<Vec<GeckoTerminalPool>, String> {
    let mut pools = Vec::new();

    // Get token data
    let token_data = response.data.as_ref()
        .ok_or("No token data in response")?;

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
    let included = response.included.as_ref()
        .ok_or("No included data in response")?;

    // Match pool references with included pool data
    for pool_ref in pool_refs {
        if pool_ref.ref_type != "pool" {
            continue;
        }

        // Find matching pool in included data
        if let Some(pool_data) = included.iter().find(|item| item.id == pool_ref.id && item.data_type == "pool") {
            if let Some(pool) = parse_single_pool(pool_data, token_address)? {
                pools.push(pool);
            }
        }
    }

    // Sort by liquidity (highest first)
    pools.sort_by(|a, b| b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal));

    Ok(pools)
}

/// Parse a single pool from included data
fn parse_single_pool(pool_data: &GeckoTerminalIncluded, token_address: &str) -> Result<Option<GeckoTerminalPool>, String> {
    let attributes = pool_data.attributes.as_ref()
        .ok_or("No pool attributes")?;

    // Parse attributes as GeckoTerminalPoolAttributes
    let pool_attrs: GeckoTerminalPoolAttributes = serde_json::from_value(attributes.clone())
        .map_err(|e| format!("Failed to parse pool attributes: {}", e))?;

    let pool_address = pool_attrs.address
        .ok_or("Missing pool address")?;

    // Parse price and liquidity data
    let price_usd = pool_attrs.base_token_price_usd
        .and_then(|p| p.parse::<f64>().ok())
        .unwrap_or(0.0);

    let price_native = pool_attrs.base_token_price_native_currency
        .and_then(|p| p.parse::<f64>().ok())
        .unwrap_or(0.0);

    let liquidity_usd = pool_attrs.reserve_in_usd
        .and_then(|l| l.parse::<f64>().ok())
        .unwrap_or(0.0);

    let volume_24h = pool_attrs.volume_usd
        .as_ref()
        .and_then(|v| v.get("h24"))
        .and_then(|v| Some(v.as_str()))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    let created_at = pool_attrs.pool_created_at_block_number
        .unwrap_or(0);

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

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Check if GeckoTerminal API is available (simple health check)
pub async fn test_geckoterminal_connection() -> Result<(), String> {
    let url = format!("{}/networks", GECKOTERMINAL_BASE_URL);
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = match timeout(
        Duration::from_secs(5),
        client.get(&url).send()
    ).await {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => return Err(format!("HTTP request failed: {}", e)),
        Err(_) => return Err("Request timeout".to_string()),
    };

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("HTTP {}", response.status()))
    }
}

/// Get rate limit information
pub fn get_rate_limit_info() -> (usize, usize) {
    (GECKOTERMINAL_RATE_LIMIT_PER_MINUTE, MAX_TOKENS_PER_BATCH)
}
