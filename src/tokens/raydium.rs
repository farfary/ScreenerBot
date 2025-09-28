/// Raydium API integration for enhanced pool discovery
///
/// This module integrates with Raydium's v3 API to fetch pool information
/// for Solana tokens. Raydium is one of the largest DEXes on Solana and provides
/// comprehensive pool data including:
/// - Standard AMM pools
/// - Concentrated liquidity pools (CLMM)
/// - Farm and reward information
/// - Real-time pricing and volume data
/// - TVL and liquidity metrics
use crate::global::is_debug_api_enabled;
use crate::logger::{log, LogTag};
use chrono::Utc;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

// =============================================================================
// RAYDIUM API CONFIGURATION
// =============================================================================

/// Raydium API base URL
const RAYDIUM_BASE_URL: &str = "https://api-v3.raydium.io";

/// Rate limit: Conservative estimate (not specified in docs)
const RAYDIUM_RATE_LIMIT_PER_MINUTE: usize = 120;

/// Maximum tokens per batch request
const MAX_TOKENS_PER_BATCH: usize = 10;

/// Default timeout for API requests
const REQUEST_TIMEOUT_SECONDS: u64 = 10;

// =============================================================================
// RAYDIUM API RESPONSE STRUCTURES
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
struct RaydiumApiResponse {
    id: String,
    success: bool,
    data: RaydiumPoolsData,
}

#[derive(Debug, Clone, Deserialize)]
struct RaydiumPoolsData {
    count: u32,
    data: Vec<RaydiumPoolInfo>,
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct RaydiumPoolInfo {
    #[serde(rename = "type")]
    pool_type: String,
    #[serde(rename = "programId")]
    program_id: String,
    id: String,
    #[serde(rename = "mintA")]
    mint_a: RaydiumMintInfo,
    #[serde(rename = "mintB")]
    mint_b: RaydiumMintInfo,
    price: f64,
    #[serde(rename = "mintAmountA")]
    mint_amount_a: f64,
    #[serde(rename = "mintAmountB")]
    mint_amount_b: f64,
    #[serde(rename = "feeRate")]
    fee_rate: f64,
    #[serde(rename = "openTime")]
    open_time: String,
    tvl: f64,
    day: Option<RaydiumTimeStats>,
    week: Option<RaydiumTimeStats>,
    month: Option<RaydiumTimeStats>,
    pooltype: Vec<String>,
    #[serde(rename = "farmUpcomingCount")]
    farm_upcoming_count: u32,
    #[serde(rename = "farmOngoingCount")]
    farm_ongoing_count: u32,
    #[serde(rename = "farmFinishedCount")]
    farm_finished_count: u32,
    #[serde(rename = "burnPercent")]
    burn_percent: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct RaydiumMintInfo {
    #[serde(rename = "chainId")]
    chain_id: u32,
    address: String,
    #[serde(rename = "programId")]
    program_id: String,
    #[serde(rename = "logoURI")]
    logo_uri: Option<String>,
    symbol: String,
    name: String,
    decimals: u8,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RaydiumTimeStats {
    volume: f64,
    #[serde(rename = "volumeQuote")]
    volume_quote: f64,
    #[serde(rename = "volumeFee")]
    volume_fee: f64,
    apr: f64,
    #[serde(rename = "feeApr")]
    fee_apr: f64,
    #[serde(rename = "priceMin")]
    price_min: f64,
    #[serde(rename = "priceMax")]
    price_max: f64,
}

/// Normalized pool structure compatible with other APIs
pub struct RaydiumPool {
    pub pool_address: String,
    pub dex_id: String,
    pub pool_type: String,
    pub base_token: String,
    pub quote_token: String,
    pub price_native: f64,
    pub price_usd: f64,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub fee_rate: f64,
    pub apr: f64,
    pub pool_name: Option<String>,
}

/// Batch result for multiple tokens
pub struct RaydiumBatchResult {
    pub pools: HashMap<String, Vec<RaydiumPool>>,
    pub errors: HashMap<String, String>,
    pub successful_tokens: usize,
    pub failed_tokens: usize,
}

// =============================================================================
// MAIN API FUNCTIONS
// =============================================================================

/// Get pools for a specific token from Raydium API
/// This searches for pools where the token is either mintA or mintB
pub async fn get_token_pools_from_raydium(token_mint: &str) -> Result<Vec<RaydiumPool>, String> {
    let start_time = std::time::Instant::now();

    if is_debug_api_enabled() {
        log(
            LogTag::Pool,
            "RAYDIUM_API_START",
            &format!(
                "🟡 Fetching pools for token {} from Raydium API",
                &token_mint[..8]
            ),
        );
    }

    // Create HTTP client
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
        .user_agent("ScreenerBot/1.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Build the API URL - search with SOL as the other pair to get most pools
    let sol_mint = "So11111111111111111111111111111111111111112";
    let url = format!(
        "{}/pools/info/mint?mint1={}&mint2={}&poolType=all&poolSortField=liquidity&sortType=desc&pageSize=10&page=1",
        RAYDIUM_BASE_URL,
        token_mint,
        sol_mint
    );

    // Make the API request
    let response = timeout(
        Duration::from_secs(REQUEST_TIMEOUT_SECONDS),
        client.get(&url).send(),
    )
    .await
    .map_err(|_| "Raydium API request timed out".to_string())?
    .map_err(|e| format!("HTTP request failed: {}", e))?;

    // Check response status
    if !response.status().is_success() {
        return Err(format!(
            "Raydium API returned status: {}",
            response.status()
        ));
    }

    // Parse the response
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let api_response: RaydiumApiResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

    if !api_response.success {
        return Err("Raydium API returned success=false".to_string());
    }

    // Convert to normalized pool format
    let pools: Vec<RaydiumPool> = api_response
        .data
        .data
        .into_iter()
        .map(|pool_info| parse_raydium_pool(pool_info, token_mint))
        .collect();

    let elapsed = start_time.elapsed();

    if is_debug_api_enabled() {
        log(
            LogTag::Pool,
            "RAYDIUM_API_SUCCESS",
            &format!(
                "✅ Raydium API: Found {} pools for {} in {:.2}s",
                pools.len(),
                &token_mint[..8],
                elapsed.as_secs_f64()
            ),
        );
    }

    Ok(pools)
}

/// Get pools for multiple tokens in batch
pub async fn get_batch_token_pools_from_raydium(token_mints: &[String]) -> RaydiumBatchResult {
    let start_time = std::time::Instant::now();

    if is_debug_api_enabled() {
        log(
            LogTag::Pool,
            "RAYDIUM_BATCH_START",
            &format!(
                "🟡 Starting Raydium batch pool fetch for {} tokens",
                token_mints.len()
            ),
        );
    }

    let mut pools = HashMap::new();
    let mut errors = HashMap::new();
    let mut successful_tokens = 0;
    let mut failed_tokens = 0;

    // Process tokens with rate limiting
    for (i, token_mint) in token_mints.iter().enumerate() {
        // Rate limiting: small delay between requests
        if i > 0 {
            tokio::time::sleep(Duration::from_millis(600)).await; // ~100 req/min
        }

        match get_token_pools_from_raydium(token_mint).await {
            Ok(token_pools) => {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Pool,
                        "RAYDIUM_BATCH_TOKEN_SUCCESS",
                        &format!(
                            "✅ {}: {} pools from Raydium",
                            &token_mint[..8],
                            token_pools.len()
                        ),
                    );
                }
                pools.insert(token_mint.clone(), token_pools);
                successful_tokens += 1;
            }
            Err(e) => {
                if is_debug_api_enabled() {
                    log(
                        LogTag::Pool,
                        "RAYDIUM_BATCH_TOKEN_ERROR",
                        &format!("❌ {}: Raydium error - {}", &token_mint[..8], e),
                    );
                }
                errors.insert(token_mint.clone(), e);
                failed_tokens += 1;
            }
        }
    }

    let elapsed = start_time.elapsed();

    if is_debug_api_enabled() {
        log(
            LogTag::Pool,
            "RAYDIUM_BATCH_COMPLETE",
            &format!(
                "🟡 Raydium batch complete: {}/{} successful in {:.2}s",
                successful_tokens,
                token_mints.len(),
                elapsed.as_secs_f64()
            ),
        );
    }

    RaydiumBatchResult {
        pools,
        errors,
        successful_tokens,
        failed_tokens,
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Parse Raydium pool info into normalized format
fn parse_raydium_pool(pool_info: RaydiumPoolInfo, target_token: &str) -> RaydiumPool {
    // SOL mint address for price calculations
    let sol_mint = "So11111111111111111111111111111111111111112";

    // Determine which mint is the target token and calculate USD price
    let (base_token, quote_token, price_usd) = if pool_info.mint_a.address == target_token {
        // Target token is mintA, price is in terms of mintB
        let price_in_quote = pool_info.price;

        // If mintB is SOL, we need to convert to USD (assuming SOL ≈ $207)
        let price_usd = if pool_info.mint_b.address == sol_mint {
            price_in_quote * 207.0 // Convert SOL price to USD
        } else {
            price_in_quote // Assume other quote tokens are already in USD terms
        };

        (
            pool_info.mint_a.address.clone(),
            pool_info.mint_b.address.clone(),
            price_usd,
        )
    } else {
        // Target token is mintB, price is mintA/mintB, so we need mintB/mintA
        let price_in_quote = 1.0 / pool_info.price;

        // If mintA is SOL, we need to convert to USD
        let price_usd = if pool_info.mint_a.address == sol_mint {
            price_in_quote * 207.0 // Convert SOL price to USD
        } else {
            price_in_quote // Assume other quote tokens are already in USD terms
        };

        (
            pool_info.mint_b.address.clone(),
            pool_info.mint_a.address.clone(),
            price_usd,
        )
    };

    // Calculate 24h volume
    let volume_24h = pool_info.day.as_ref().map(|day| day.volume).unwrap_or(0.0);

    // Calculate APR
    let apr = pool_info.day.as_ref().map(|day| day.apr).unwrap_or(0.0);

    // Create pool name
    let pool_name = Some(format!(
        "{}-{} ({})",
        pool_info.mint_a.symbol, pool_info.mint_b.symbol, pool_info.pool_type
    ));

    RaydiumPool {
        pool_address: pool_info.id,
        dex_id: "raydium".to_string(),
        pool_type: pool_info.pool_type,
        base_token,
        quote_token,
        price_native: pool_info.price,
        price_usd,
        liquidity_usd: pool_info.tvl,
        volume_24h,
        fee_rate: pool_info.fee_rate,
        apr,
        pool_name,
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Get Raydium API rate limit information
pub fn get_raydium_rate_limit() -> usize {
    RAYDIUM_RATE_LIMIT_PER_MINUTE
}

/// Get maximum tokens per batch for Raydium API
pub fn get_raydium_max_batch_size() -> usize {
    MAX_TOKENS_PER_BATCH
}
