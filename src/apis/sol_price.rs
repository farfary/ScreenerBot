//! SOL Price Service
//!
//! Provides real-time SOL/USD price for USD conversions and trading calculations.
//! Runs as a background task and maintains a cached price for the whole bot.
//!
//! **Source cascade (in priority order):**
//! 1. DexScreener — primary (highest-liquidity WSOL pair `priceUsd`)
//! 2. GeckoTerminal — secondary (`simple/.../token_price`)
//! 3. Jupiter — last-resort fallback (`lite-api.jup.ag/price/v3`)
//!
//! Keeping SOL price OFF Jupiter by default is deliberate: Jupiter's free
//! lite-api rate-limits per-IP across all endpoints, and that budget must be
//! reserved for swap quote/swap. Each source is tried in order so one provider's
//! outage or rate-limit never blocks trading.
//!
//! **Key Features:**
//! - Multi-source price fetching with automatic failover
//! - Automatic price caching and refresh cycles
//! - Graceful shutdown handling
//! - Thread-safe price access for concurrent operations

use crate::logger::{self, LogTag};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::RwLock as StdRwLock;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::time::{interval, sleep};

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// WSOL mint (SOL/USD is quoted as the price of wrapped SOL).
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// DexScreener token endpoint for WSOL — PRIMARY source.
const DEXSCREENER_SOL_URL: &str =
    "https://api.dexscreener.com/latest/dex/tokens/So11111111111111111111111111111111111111112";

/// GeckoTerminal simple token-price endpoint for WSOL — SECONDARY source.
const GECKOTERMINAL_SOL_URL: &str =
    "https://api.geckoterminal.com/api/v2/simple/networks/solana/token_price/So11111111111111111111111111111111111111112";

/// Jupiter price endpoint — LAST-RESORT fallback (shares the swap rate budget).
const JUPITER_PRICE_API: &str =
    "https://lite-api.jup.ag/price/v3?ids=So11111111111111111111111111111111111111112";

/// Price refresh interval in seconds
const PRICE_REFRESH_INTERVAL_SECS: u64 = 30;

/// Request timeout in seconds
const REQUEST_TIMEOUT_SECS: u64 = 10;

/// Cache expiry time in seconds (if service stops updating)
const CACHE_EXPIRY_SECS: u64 = 300; // 5 minutes

/// Maximum price change threshold for validation (50% change detection)
const MAX_PRICE_CHANGE_PERCENT: f64 = 50.0;

/// Maximum consecutive errors before marking cache as invalid
const MAX_CONSECUTIVE_ERRORS: u32 = 10;

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Jupiter API price response structure (direct mint address mapping)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JupiterPriceResponse {
    #[serde(rename = "So11111111111111111111111111111111111111112")]
    pub sol: JupiterTokenPrice,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JupiterTokenPrice {
    #[serde(rename = "usdPrice")]
    pub usd_price: f64,
    #[serde(rename = "blockId")]
    pub block_id: u64,
    pub decimals: u8,
    #[serde(rename = "priceChange24h")]
    pub price_change_24h: f64,
}

/// Cached SOL price data with metadata
#[derive(Debug, Clone)]
pub struct SolPriceData {
    pub price_usd: f64,
    pub last_updated: Instant,
    pub is_valid: bool,
    pub source: String,
    pub fetch_count: u64,
    pub error_count: u64,
}

impl Default for SolPriceData {
    fn default() -> Self {
        Self {
            price_usd: 0.0,
            last_updated: Instant::now(),
            is_valid: false,
            source: "uninitialized".to_owned(),
            fetch_count: 0,
            error_count: 0,
        }
    }
}

impl SolPriceData {
    /// Check if cached price is still fresh
    pub fn is_fresh(&self) -> bool {
        self.is_valid && self.last_updated.elapsed().as_secs() < CACHE_EXPIRY_SECS
    }

    /// Get age of cached price in seconds
    pub fn age_seconds(&self) -> u64 {
        self.last_updated.elapsed().as_secs()
    }
}

// =============================================================================
// GLOBAL STATE
// =============================================================================

/// Global SOL price cache with thread-safe access
static SOL_PRICE_CACHE: LazyLock<Arc<StdRwLock<SolPriceData>>> =
    LazyLock::new(|| Arc::new(StdRwLock::new(SolPriceData::default())));

/// Service status tracking
static SERVICE_RUNNING: LazyLock<Arc<std::sync::atomic::AtomicBool>> =
    LazyLock::new(|| Arc::new(std::sync::atomic::AtomicBool::new(false)));

// =============================================================================
// PUBLIC API
// =============================================================================

/// Get current SOL price in USD
/// Returns cached price if available and fresh, otherwise returns 0.0
pub fn get_sol_price() -> f64 {
    match SOL_PRICE_CACHE.read() {
        Ok(cache) => {
            if cache.is_fresh() {
                cache.price_usd
            } else {
                // Debug, not warning: this is a hot getter called by many price
                // consumers, and callers handle the 0.0 sentinel. The real cause
                // (fetch failure / offline) is surfaced on the fetch side, so a
                // per-read warning here is pure log spam.
                logger::debug(
                    LogTag::SolPrice,
                    &format!(
                        "SOL price cache stale (age: {}s), returning 0.0",
                        cache.age_seconds()
                    ),
                );
                0.0
            }
        }
        Err(e) => {
            logger::error(
                LogTag::SolPrice,
                &format!("Failed to read SOL price cache: {e}"),
            );
            0.0
        }
    }
}

/// Get detailed SOL price information including metadata
pub fn get_sol_price_info() -> Option<SolPriceData> {
    match SOL_PRICE_CACHE.read() {
        Ok(cache) => Some(cache.clone()),
        Err(e) => {
            logger::error(
                LogTag::SolPrice,
                &format!("Failed to read SOL price info: {e}"),
            );
            None
        }
    }
}

/// Check if SOL price service is running
pub fn is_sol_price_service_running() -> bool {
    SERVICE_RUNNING.load(std::sync::atomic::Ordering::SeqCst)
}

/// Manually fetch and cache SOL price (useful for debug tools)
/// Returns the fetched price on success
pub async fn fetch_and_cache_sol_price() -> Result<f64, String> {
    let (price, source) = fetch_sol_price().await?;

    // Update cache
    match SOL_PRICE_CACHE.write() {
        Ok(mut cache) => {
            *cache = SolPriceData {
                price_usd: price,
                last_updated: Instant::now(),
                source: format!("{source} (manual)"),
                is_valid: true,
                fetch_count: cache.fetch_count + 1,
                error_count: cache.error_count,
            };
            Ok(price)
        }
        Err(e) => Err(format!("Failed to update cache: {e}")),
    }
}

// =============================================================================
// SERVICE LIFECYCLE
// =============================================================================

/// Start the SOL price service
///
/// Returns JoinHandle so ServiceManager can wait for graceful shutdown.
pub async fn start_sol_price_service(
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> Result<tokio::task::JoinHandle<()>, String> {
    logger::info(LogTag::SolPrice, "Starting SOL price service");

    // Mark service as running
    SERVICE_RUNNING.store(true, std::sync::atomic::Ordering::SeqCst);

    // Spawn the background task and return handle
    let handle = tokio::spawn(monitor.instrument(async move {
        sol_price_task(shutdown).await;
    }));

    logger::info(LogTag::SolPrice, "SOL price service started (instrumented)");
    Ok(handle)
}

/// Stop the SOL price service
pub async fn stop_sol_price_service() {
    logger::warning(LogTag::SolPrice, "Stopping SOL price service");

    // Mark service as stopped
    SERVICE_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);

    logger::info(LogTag::SolPrice, "SOL price service stopped");
}

// =============================================================================
// BACKGROUND TASK
// =============================================================================

/// Main SOL price monitoring task
async fn sol_price_task(shutdown: Arc<Notify>) {
    logger::info(LogTag::SolPrice, "SOL price monitoring task started");

    let mut price_interval = interval(Duration::from_secs(PRICE_REFRESH_INTERVAL_SECS));
    let mut consecutive_errors = 0u32;

    // Initial price fetch
    fetch_and_update_sol_price(&mut consecutive_errors).await;

    loop {
        tokio::select! {
               _ = shutdown.notified() => {
        logger::info(LogTag::SolPrice, "SOL price task shutdown requested");
               break;
             }
             _ = price_interval.tick() => {
               // Check if service should still be running
               if !is_sol_price_service_running() {
        logger::info(LogTag::SolPrice, "SOL price service marked as stopped, exiting task");
                 break;
               }

               // Fetch new price data
               fetch_and_update_sol_price(&mut consecutive_errors).await;

               // If too many consecutive errors, increase interval
               if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                 logger::warning(
                   LogTag::SolPrice,
                   &format!(
        "{} consecutive errors, extending interval to reduce API pressure",
                     consecutive_errors
                   ),
                 );
                 sleep(Duration::from_secs(PRICE_REFRESH_INTERVAL_SECS * 2)).await;
               }
             }
           }
    }

    // Mark service as stopped
    SERVICE_RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
    logger::info(LogTag::SolPrice, "SOL price monitoring task completed");
}

// =============================================================================
// PRICE FETCHING LOGIC
// =============================================================================

/// Fetch SOL price from Jupiter API and update cache
async fn fetch_and_update_sol_price(consecutive_errors: &mut u32) {
    // Skip the refresh entirely while the internet is confirmed offline: all
    // three price sources are unreachable, so this would only time out on DNS.
    // The cached price stays (callers already handle staleness); we resume on
    // reconnect. Never triggers at startup (Unknown state).
    if crate::connectivity::is_network_offline() {
        return;
    }

    logger::debug(LogTag::SolPrice, "Fetching SOL price (DexScreener -> GeckoTerminal -> Jupiter)");

    match fetch_sol_price().await {
        Ok((price, source)) => {
            if validate_price_change(price) {
                update_price_cache(price, source.to_owned(), true).await;
                *consecutive_errors = 0; // Reset error counter on success
                logger::debug(
                    LogTag::SolPrice,
                    &format!("SOL price updated: ${:.4} (source: {})", price, source),
                );
            } else {
                logger::warning(
                    LogTag::SolPrice,
                    &format!(
                        "SOL price validation failed: ${:.4} (change >{}%)",
                        price, MAX_PRICE_CHANGE_PERCENT
                    ),
                );
                *consecutive_errors += 1;
            }
        }
        Err(e) => {
            *consecutive_errors += 1;
            update_error_count().await;

            logger::error(
                LogTag::SolPrice,
                &format!(
                    "Failed to fetch SOL price: {} (errors: {})",
                    e, consecutive_errors
                ),
            );

            // If too many errors, mark cache as invalid but keep last price
            if *consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                invalidate_cache().await;
            }
        }
    }
}

/// Fetch SOL/USD using the source cascade: DexScreener -> GeckoTerminal -> Jupiter.
/// Returns `(price_usd, source_label)`. Each source is tried in order; on failure
/// the next is used so a single provider's outage/rate-limit never blocks trading.
async fn fetch_sol_price() -> Result<(f64, &'static str), String> {
    let mut errors: Vec<String> = Vec::new();

    match fetch_from_dexscreener().await {
        Ok(price) => return Ok((price, "dexscreener")),
        Err(e) => {
            logger::debug(LogTag::SolPrice, &format!("DexScreener SOL price failed: {e}"));
            errors.push(format!("dexscreener: {e}"));
        }
    }

    match fetch_from_geckoterminal().await {
        Ok(price) => return Ok((price, "geckoterminal")),
        Err(e) => {
            logger::debug(
                LogTag::SolPrice,
                &format!("GeckoTerminal SOL price failed: {e}"),
            );
            errors.push(format!("geckoterminal: {e}"));
        }
    }

    match fetch_sol_price_from_jupiter().await {
        Ok(price) => {
            logger::warning(
                LogTag::SolPrice,
                "SOL price fell back to Jupiter (DexScreener + GeckoTerminal both failed)",
            );
            return Ok((price, "jupiter"));
        }
        Err(e) => errors.push(format!("jupiter: {e}")),
    }

    Err(format!("All SOL price sources failed: {}", errors.join("; ")))
}

/// PRIMARY: fetch SOL/USD from DexScreener — the `priceUsd` of the highest-liquidity
/// pair whose BASE token is WSOL (so the price is SOL's, not the quote token's).
async fn fetch_from_dexscreener() -> Result<f64, String> {
    let client = crate::net::client();
    let response = client
        .get(DEXSCREENER_SOL_URL)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("JSON parsing failed: {e}"))?;

    let pairs = json
        .get("pairs")
        .and_then(|p| p.as_array())
        .ok_or("no pairs in response")?;

    let mut best: Option<(f64, f64)> = None; // (price_usd, liquidity_usd)
    for pair in pairs {
        let base_is_wsol = pair
            .get("baseToken")
            .and_then(|b| b.get("address"))
            .and_then(|a| a.as_str())
            == Some(WSOL_MINT);
        if !base_is_wsol {
            continue;
        }
        let price = pair
            .get("priceUsd")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<f64>().ok());
        let liquidity = pair
            .get("liquidity")
            .and_then(|l| l.get("usd"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if let Some(price) = price {
            if price > 0.0 && price.is_finite() && best.map_or(true, |(_, l)| liquidity > l) {
                best = Some((price, liquidity));
            }
        }
    }

    match best {
        Some((price, _)) => Ok(price),
        None => Err("no valid WSOL pair price".to_owned()),
    }
}

/// SECONDARY: fetch SOL/USD from GeckoTerminal's simple token-price endpoint.
async fn fetch_from_geckoterminal() -> Result<f64, String> {
    let client = crate::net::client();
    let response = client
        .get(GECKOTERMINAL_SOL_URL)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("JSON parsing failed: {e}"))?;

    let price_str = json
        .pointer("/data/attributes/token_prices")
        .and_then(|m| m.as_object())
        .and_then(|m| m.values().next())
        .and_then(|v| v.as_str())
        .ok_or("no token price in response")?;

    let price: f64 = price_str
        .parse()
        .map_err(|e| format!("price parse failed: {e}"))?;

    if price > 0.0 && price.is_finite() {
        Ok(price)
    } else {
        Err(format!("invalid price: {price}"))
    }
}

/// LAST-RESORT: fetch SOL price from Jupiter API
async fn fetch_sol_price_from_jupiter() -> Result<f64, String> {
    // Yield to in-flight swaps and space against other background Jupiter calls
    // so price polling never starves the swap rate budget (lite-api is per-IP).
    crate::apis::jupiter::throttle::acquire_background().await;

    let client = crate::net::client();

    let response = client
        .get(JUPITER_PRICE_API)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let price_response: JupiterPriceResponse = response
        .json()
        .await
        .map_err(|e| format!("JSON parsing failed: {e}"))?;

    // Extract SOL price directly from the response
    let sol_price = price_response.sol.usd_price;

    if sol_price > 0.0 && sol_price.is_finite() {
        Ok(sol_price)
    } else {
        Err(format!("Invalid SOL price: {sol_price}"))
    }
}

/// Validate price change to detect anomalies
fn validate_price_change(new_price: f64) -> bool {
    if new_price <= 0.0 || !new_price.is_finite() {
        return false;
    }

    // Get current cached price for comparison
    if let Ok(cache) = SOL_PRICE_CACHE.read() {
        if cache.is_valid && cache.price_usd > 0.0 {
            let change_percent = ((new_price - cache.price_usd) / cache.price_usd).abs() * 100.0;
            if change_percent > MAX_PRICE_CHANGE_PERCENT {
                return false; // Price change too large, likely an error
            }
        }
    }

    true
}

/// Update the price cache with new data
async fn update_price_cache(price: f64, source: String, is_valid: bool) {
    if let Ok(mut cache) = SOL_PRICE_CACHE.write() {
        cache.price_usd = price;
        cache.last_updated = Instant::now();
        cache.is_valid = is_valid;
        cache.source = source;
        cache.fetch_count += 1;
        logger::debug(
            LogTag::SolPrice,
            &format!(
                "Price cache updated: ${:.4} from {} (fetch: {})",
                price, cache.source, cache.fetch_count
            ),
        );
    }
}

/// Increment error count in cache
async fn update_error_count() {
    if let Ok(mut cache) = SOL_PRICE_CACHE.write() {
        cache.error_count += 1;
    }
}

/// Mark cache as invalid (but preserve last price)
async fn invalidate_cache() {
    if let Ok(mut cache) = SOL_PRICE_CACHE.write() {
        cache.is_valid = false;
        logger::warning(
            LogTag::SolPrice,
            "SOL price cache marked as invalid due to consecutive errors",
        );
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Get SOL price service statistics for debugging
pub fn get_sol_price_stats() -> String {
    match SOL_PRICE_CACHE.read() {
        Ok(cache) => {
            format!(
        "SOL Price Stats: ${:.4} | Age: {}s | Valid: {} | Source: {} | Fetches: {} | Errors: {} | Running: {}",
        cache.price_usd,
        cache.age_seconds(),
        cache.is_valid,
        cache.source,
        cache.fetch_count,
        cache.error_count,
        is_sol_price_service_running()
      )
        }
        Err(_) => "SOL Price Stats: Cache lock error".to_owned(),
    }
}

/// Force refresh SOL price (for manual testing)
pub async fn force_refresh_sol_price() -> Result<f64, String> {
    logger::info(LogTag::SolPrice, "Force refreshing SOL price");

    let mut consecutive_errors = 0u32;
    fetch_and_update_sol_price(&mut consecutive_errors).await;

    let price = get_sol_price();
    if price > 0.0 {
        Ok(price)
    } else {
        Err("Failed to refresh SOL price".to_owned())
    }
}
