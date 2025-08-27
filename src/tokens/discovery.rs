/// Base token discovery system structure
use crate::logger::{ log, LogTag };
use crate::global::is_debug_discovery_enabled;
use crate::tokens::dexscreener::get_global_dexscreener_api;
use crate::tokens::cache::TokenDatabase;
use crate::tokens::is_token_excluded_from_trading;
use tokio::time::{ sleep, Duration };
use std::sync::{ Arc, OnceLock };
use tokio::sync::RwLock;
use chrono::{ Utc, DateTime };
use reqwest::Client;
use futures::FutureExt; // for now_or_never on shutdown future

// =============================================================================
// NETWORK CONSTANTS / HELPERS
// =============================================================================
/// Per-endpoint HTTP timeout for discovery API calls (seconds)
/// Increased to 15s to accommodate RugCheck new_tokens endpoint (~10s response time)
const DISCOVERY_HTTP_TIMEOUT_SECS: u64 = 15;

/// Build a reqwest client with a short timeout suitable for discovery endpoints.
fn build_discovery_client() -> Result<Client, String> {
    reqwest::Client
        ::builder()
        .timeout(std::time::Duration::from_secs(DISCOVERY_HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

// =============================================================================
// API FUNCTIONS
// =============================================================================

/// Fetch latest token profiles from DexScreener API and extract Solana mint addresses
pub async fn fetch_dexscreener_latest_token_profiles() -> Result<Vec<String>, String> {
    log(LogTag::Discovery, "DEBUG", "ENTERED fetch_dexscreener_latest_token_profiles function");

    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "API", "Fetching latest token profiles from DexScreener");
    }

    log(LogTag::Discovery, "DEBUG", "Building HTTP client...");
    let client = build_discovery_client()?;

    log(LogTag::Discovery, "DEBUG", "Making HTTP request to profiles API...");
    let response = client
        .get("https://api.dexscreener.com/token-profiles/latest/v1")
        .header("Accept", "*/*")
        .send().await
        .map_err(|e| format!("HTTP request failed (profiles): {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(array) = json.as_array() {
        for item in array {
            if
                let (Some(chain_id), Some(token_address)) = (
                    item.get("chainId").and_then(|v| v.as_str()),
                    item.get("tokenAddress").and_then(|v| v.as_str()),
                )
            {
                if chain_id == "solana" {
                    mints.push(token_address.to_string());
                }
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "EXTRACTED", &format!("Found {} Solana mints", mints.len()));
    }
    Ok(mints)
}

/// Fetch latest boosted tokens from DexScreener API and extract Solana mint addresses
pub async fn fetch_dexscreener_latest_boosted_tokens() -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "API", "Fetching latest boosted tokens from DexScreener");
    }

    let client = build_discovery_client()?;
    let response = client
        .get("https://api.dexscreener.com/token-boosts/latest/v1")
        .header("Accept", "*/*")
        .send().await
        .map_err(|e| format!("HTTP request failed (boosted): {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(array) = json.as_array() {
        for item in array {
            if
                let (Some(chain_id), Some(token_address)) = (
                    item.get("chainId").and_then(|v| v.as_str()),
                    item.get("tokenAddress").and_then(|v| v.as_str()),
                )
            {
                if chain_id == "solana" {
                    mints.push(token_address.to_string());
                }
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "EXTRACTED", &format!("Found {} Solana boosted mints", mints.len()));
    }
    Ok(mints)
}

/// Fetch tokens with most active boosts from DexScreener API and extract Solana mint addresses
pub async fn fetch_dexscreener_tokens_with_most_active_boosts() -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "API", "Fetching tokens with most active boosts from DexScreener");
    }

    let client = build_discovery_client()?;
    let response = client
        .get("https://api.dexscreener.com/token-boosts/top/v1")
        .header("Accept", "*/*")
        .send().await
        .map_err(|e| format!("HTTP request failed (top boosts): {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(array) = json.as_array() {
        for item in array {
            if
                let (Some(chain_id), Some(token_address)) = (
                    item.get("chainId").and_then(|v| v.as_str()),
                    item.get("tokenAddress").and_then(|v| v.as_str()),
                )
            {
                if chain_id == "solana" {
                    mints.push(token_address.to_string());
                }
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "EXTRACTED",
            &format!("Found {} Solana top boosted mints", mints.len())
        );
    }
    Ok(mints)
}

/// Fetch new tokens from RugCheck API and extract Solana mint addresses
pub async fn fetch_rugcheck_new_tokens() -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "API", "Fetching new tokens from RugCheck");
    }

    let client = build_discovery_client()?;
    let response = client
        .get("https://api.rugcheck.xyz/v1/stats/new_tokens")
        .header("accept", "application/json")
        .send().await
        .map_err(|e| format!("HTTP request failed (rug_new): {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(array) = json.as_array() {
        for item in array {
            if let Some(mint) = item.get("mint").and_then(|v| v.as_str()) {
                mints.push(mint.to_string());
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "EXTRACTED",
            &format!("Found {} Solana new token mints from RugCheck", mints.len())
        );
    }
    Ok(mints)
}

/// Fetch most viewed tokens from RugCheck API and extract Solana mint addresses
pub async fn fetch_rugcheck_most_viewed() -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "API", "Fetching most viewed tokens from RugCheck");
    }

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.rugcheck.xyz/v1/stats/recent")
        .header("accept", "application/json")
        .send().await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(array) = json.as_array() {
        for item in array {
            if let Some(mint) = item.get("mint").and_then(|v| v.as_str()) {
                mints.push(mint.to_string());
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "EXTRACTED",
            &format!("Found {} Solana most viewed token mints from RugCheck", mints.len())
        );
    }
    Ok(mints)
}

/// Fetch trending tokens from RugCheck API and extract Solana mint addresses
pub async fn fetch_rugcheck_trending() -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "API", "Fetching trending tokens from RugCheck");
    }

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.rugcheck.xyz/v1/stats/trending")
        .header("accept", "application/json")
        .send().await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(array) = json.as_array() {
        for item in array {
            if let Some(mint) = item.get("mint").and_then(|v| v.as_str()) {
                mints.push(mint.to_string());
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "EXTRACTED",
            &format!("Found {} Solana trending token mints from RugCheck", mints.len())
        );
    }
    Ok(mints)
}

/// Fetch verified tokens from RugCheck API and extract Solana mint addresses
pub async fn fetch_rugcheck_verified() -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "API", "Fetching verified tokens from RugCheck");
    }

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.rugcheck.xyz/v1/stats/verified")
        .header("accept", "application/json")
        .send().await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(array) = json.as_array() {
        for item in array {
            if let Some(mint) = item.get("mint").and_then(|v| v.as_str()) {
                mints.push(mint.to_string());
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "EXTRACTED",
            &format!("Found {} Solana verified token mints from RugCheck", mints.len())
        );
    }
    Ok(mints)
}

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Discovery cycle duration in seconds
pub const DISCOVERY_CYCLE_SECONDS: u64 = 10;

// =============================================================================
// TOKEN DISCOVERY MANAGER
// =============================================================================

/// Discovery statistics snapshot
#[derive(Debug, Clone, Default)]
pub struct DiscoverySourceCounts {
    pub profiles: usize,
    pub boosted: usize,
    pub top_boosts: usize,
    pub rug_new: usize,
    pub rug_viewed: usize,
    pub rug_trending: usize,
    pub rug_verified: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DiscoveryStats {
    pub total_cycles: u64,
    pub last_cycle_started: Option<DateTime<Utc>>,
    pub last_cycle_completed: Option<DateTime<Utc>>,
    pub last_processed: usize,
    pub last_added: usize,
    pub last_deduplicated_removed: usize,
    pub last_blacklist_removed: usize,
    pub total_processed: u64,
    pub total_added: u64,
    pub per_source: DiscoverySourceCounts,
    pub last_error: Option<String>,
}

static DISCOVERY_STATS: OnceLock<Arc<RwLock<DiscoveryStats>>> = OnceLock::new();

fn get_discovery_stats_handle() -> Arc<RwLock<DiscoveryStats>> {
    DISCOVERY_STATS.get_or_init(|| Arc::new(RwLock::new(DiscoveryStats::default()))).clone()
}

pub struct TokenDiscovery {
    database: TokenDatabase,
}

impl TokenDiscovery {
    /// Create new token discovery instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database = TokenDatabase::new()?;

        Ok(Self {
            database,
        })
    }

    /// Main discovery function - calls all APIs, combines mints, fetches decimals and token info
    pub async fn discover_new_tokens(
        &mut self,
        shutdown: Option<Arc<tokio::sync::Notify>>
    ) -> Result<(), String> {
        use crate::utils::check_shutdown_or_delay;
        use tokio::time::Duration;

        // Always log cycle start (visibility even without --debug-discovery)
        log(LogTag::Discovery, "START", "Starting comprehensive discovery cycle");

        // Mark stats: cycle start (non-blocking)
        if let Some(stats_handle) = DISCOVERY_STATS.get() {
            if let Ok(mut stats) = stats_handle.try_write() {
                stats.total_cycles = stats.total_cycles.saturating_add(1);
                stats.last_cycle_started = Some(Utc::now());
                stats.last_error = None; // reset at start
                // reset per-source for this cycle; will be overwritten below
                stats.per_source = DiscoverySourceCounts::default();
            } else {
                log(LogTag::Discovery, "WARN", "Stats lock busy, skipping stats update");
            }
        }

        let mut all_mints = Vec::new();
        // Per-cycle source counters
        let mut cycle_counts = DiscoverySourceCounts::default();

        log(LogTag::Discovery, "API_START", "About to fetch from profiles API");
        log(
            LogTag::Discovery,
            "DEBUG",
            "Calling fetch_dexscreener_latest_token_profiles() function..."
        );

        // Fetch latest token profiles
        match fetch_dexscreener_latest_token_profiles().await {
            Ok(mints) => {
                log(LogTag::Discovery, "SUCCESS", &format!("Profiles fetched: {}", mints.len()));
                cycle_counts.profiles = mints.len();
                all_mints.extend(mints);
            }
            Err(e) => {
                log(LogTag::Discovery, "ERROR", &format!("Profiles fetch failed: {}", e));
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("profiles: {}", e));
                    }
                }
            }
        }

        // Fetch latest boosted tokens
        match fetch_dexscreener_latest_boosted_tokens().await {
            Ok(mints) => {
                log(LogTag::Discovery, "SUCCESS", &format!("Boosted fetched: {}", mints.len()));
                cycle_counts.boosted = mints.len();
                all_mints.extend(mints);
            }
            Err(e) => {
                log(LogTag::Discovery, "ERROR", &format!("Boosted fetch failed: {}", e));
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("boosted: {}", e));
                    }
                }
            }
        }

        // Fetch tokens with most active boosts
        match fetch_dexscreener_tokens_with_most_active_boosts().await {
            Ok(mints) => {
                log(LogTag::Discovery, "SUCCESS", &format!("Top boosts fetched: {}", mints.len()));
                cycle_counts.top_boosts = mints.len();
                all_mints.extend(mints);
            }
            Err(e) => {
                log(LogTag::Discovery, "ERROR", &format!("Top boosts fetch failed: {}", e));
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("top_boosts: {}", e));
                    }
                }
            }
        }

        // Fetch new tokens from RugCheck
        match fetch_rugcheck_new_tokens().await {
            Ok(mints) => {
                log(
                    LogTag::Discovery,
                    "SUCCESS",
                    &format!("RugCheck new fetched: {}", mints.len())
                );
                cycle_counts.rug_new = mints.len();
                all_mints.extend(mints);
            }
            Err(e) => {
                log(LogTag::Discovery, "ERROR", &format!("RugCheck new fetch failed: {}", e));
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("rug_new: {}", e));
                    }
                }
            }
        }

        // Fetch most viewed tokens from RugCheck
        match fetch_rugcheck_most_viewed().await {
            Ok(mints) => {
                log(
                    LogTag::Discovery,
                    "SUCCESS",
                    &format!("RugCheck viewed fetched: {}", mints.len())
                );
                cycle_counts.rug_viewed = mints.len();
                all_mints.extend(mints);
            }
            Err(e) => {
                log(LogTag::Discovery, "ERROR", &format!("RugCheck viewed fetch failed: {}", e));
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("rug_viewed: {}", e));
                    }
                }
            }
        }

        // Fetch trending tokens from RugCheck
        match fetch_rugcheck_trending().await {
            Ok(mints) => {
                log(
                    LogTag::Discovery,
                    "SUCCESS",
                    &format!("RugCheck trending fetched: {}", mints.len())
                );
                cycle_counts.rug_trending = mints.len();
                all_mints.extend(mints);
            }
            Err(e) => {
                log(LogTag::Discovery, "ERROR", &format!("RugCheck trending fetch failed: {}", e));
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("rug_trending: {}", e));
                    }
                }
            }
        }

        // Fetch verified tokens from RugCheck
        match fetch_rugcheck_verified().await {
            Ok(mints) => {
                log(
                    LogTag::Discovery,
                    "SUCCESS",
                    &format!("RugCheck verified fetched: {}", mints.len())
                );
                cycle_counts.rug_verified = mints.len();
                all_mints.extend(mints);
            }
            Err(e) => {
                log(LogTag::Discovery, "ERROR", &format!("RugCheck verified fetch failed: {}", e));
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("rug_verified: {}", e));
                    }
                }
            }
        }

        // Deduplicate mints
        let original_count = all_mints.len();
        all_mints.sort();
        all_mints.dedup();
        let deduplicated_count = all_mints.len();
        let dedup_removed = original_count.saturating_sub(deduplicated_count);

        // Filter out blacklisted/excluded tokens
        let before_blacklist_count = all_mints.len();
        all_mints.retain(|mint| !is_token_excluded_from_trading(mint));
        let after_blacklist_count = all_mints.len();
        let blacklisted_count = before_blacklist_count - after_blacklist_count;

        log(
            LogTag::Discovery,
            "DEDUP",
            &format!(
                "Processed mints: {} → {} deduplicated → {} after blacklist filter (removed {} blacklisted)",
                original_count,
                deduplicated_count,
                after_blacklist_count,
                blacklisted_count
            )
        );

        // REMOVED: Price service seeding that was causing resource waste
        // Discovery tokens should not be added to priority monitoring
        // Only open positions should be priority per user requirements
        if !all_mints.is_empty() {
            log(
                LogTag::Discovery,
                "DISCOVERY_COMPLETE",
                &format!(
                    "📊 Discovery completed: {} tokens found (not added to priority monitoring)",
                    all_mints.len().min(50)
                )
            );
        }

        if all_mints.is_empty() {
            log(LogTag::Discovery, "COMPLETE", "No tokens to process");
            // Update stats and return (non-blocking)
            if let Some(stats_handle) = DISCOVERY_STATS.get() {
                if let Ok(mut stats) = stats_handle.try_write() {
                    stats.last_processed = 0;
                    stats.last_added = 0;
                    stats.last_deduplicated_removed = dedup_removed;
                    stats.last_blacklist_removed = blacklisted_count;
                    stats.per_source = cycle_counts;
                    stats.last_cycle_completed = Some(Utc::now());
                }
            }
            return Ok(());
        }

        // Process tokens in batches to avoid overwhelming APIs
        let batch_size = 30; // DexScreener API limit
        let mut total_processed = 0;
        let mut total_added = 0;

        for (batch_index, batch) in all_mints.chunks(batch_size).enumerate() {
            log(
                LogTag::Discovery,
                "BATCH",
                &format!(
                    "Processing batch {}/{} with {} tokens",
                    batch_index + 1,
                    (all_mints.len() + batch_size - 1) / batch_size,
                    batch.len()
                )
            );

            // Get token information from DexScreener API
            let tokens_result = {
                let api = match get_global_dexscreener_api().await {
                    Ok(api) => api,
                    Err(e) => {
                        log(
                            LogTag::Discovery,
                            "ERROR",
                            &format!("Failed to get global API client: {}", e)
                        );
                        continue;
                    }
                };
                let mut api_instance = api.lock().await;
                // CRITICAL: Only hold the lock for the API call, then release immediately
                api_instance.get_tokens_info(batch).await
            }; // Lock is released here automatically

            match tokens_result {
                Ok(tokens) => {
                    if !tokens.is_empty() {
                        // Fetch actual decimals from blockchain and ensure they're cached
                        // This is critical for P&L calculations - decimals must be in cache
                        let mints: Vec<String> = tokens
                            .iter()
                            .map(|t| t.mint.clone())
                            .collect();
                        let decimal_results = crate::tokens::decimals::batch_fetch_token_decimals(
                            &mints
                        ).await;

                        // Verify all decimals were successfully cached
                        let mut failed_tokens = Vec::new();
                        for (mint, decimal_result) in decimal_results.iter() {
                            match decimal_result {
                                Ok(_) => {
                                    if is_debug_discovery_enabled() {
                                        if
                                            let Some(token) = tokens
                                                .iter()
                                                .find(|t| t.mint == *mint)
                                        {
                                            log(
                                                LogTag::Discovery,
                                                "DECIMALS",
                                                &format!(
                                                    "Cached decimals for {} ({})",
                                                    token.symbol,
                                                    &token.mint[..8]
                                                )
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    // CRITICAL: Never proceed without cached decimals
                                    // If decimals fetch fails, we should not add this token to watch list
                                    if let Some(token) = tokens.iter().find(|t| t.mint == *mint) {
                                        log(
                                            LogTag::Discovery,
                                            "ERROR",
                                            &format!(
                                                "Failed to fetch decimals for {} ({}): {} - SKIPPING TOKEN",
                                                token.symbol,
                                                &token.mint[..8],
                                                e
                                            )
                                        );
                                        failed_tokens.push(mint.clone());
                                    }
                                }
                            }
                        }

                        // Filter out tokens with failed decimal fetching
                        let original_count = tokens.len();
                        let tokens: Vec<_> = tokens
                            .into_iter()
                            .filter(|token| !failed_tokens.contains(&token.mint))
                            .collect();
                        let filtered_count = tokens.len();

                        if original_count != filtered_count {
                            log(
                                LogTag::Discovery,
                                "FILTER",
                                &format!(
                                    "Removed {} tokens with failed decimal fetching (keeping {} tokens)",
                                    original_count - filtered_count,
                                    filtered_count
                                )
                            );
                        }

                        if tokens.is_empty() {
                            log(
                                LogTag::Discovery,
                                "SKIP",
                                "No tokens remaining after decimal validation"
                            );
                            continue;
                        }

                        // Check for new tokens before adding to database
                        let original_count = tokens.len();
                        let mut new_tokens = Vec::new();
                        let mut existing_count = 0;

                        for token in &tokens {
                            match self.database.get_token_by_mint(&token.mint) {
                                Ok(Some(_)) => {
                                    existing_count += 1;
                                }
                                Ok(None) => new_tokens.push(token.clone()),
                                Err(_) => new_tokens.push(token.clone()), // Assume new if check fails
                            }
                        }

                        if new_tokens.is_empty() {
                            if is_debug_discovery_enabled() {
                                log(
                                    LogTag::Discovery,
                                    "SKIP",
                                    &format!("All {} tokens already exist in database - skipping batch", original_count)
                                );
                            }
                            continue;
                        }

                        // Only add truly NEW tokens to database - let monitor handle updates
                        match self.database.add_tokens(&new_tokens).await {
                            Ok(_) => {
                                total_added += new_tokens.len();

                                if is_debug_discovery_enabled() {
                                    log(
                                        LogTag::Discovery,
                                        "DATABASE",
                                        &format!(
                                            "Added {} NEW tokens to database (skipped {} existing)",
                                            new_tokens.len(),
                                            existing_count
                                        )
                                    );

                                    // Log only first-seen tokens
                                    for token in &new_tokens {
                                        log(
                                            LogTag::Discovery,
                                            "NEW",
                                            &format!(
                                                "{} ({}) - Liquidity: ${:.0}",
                                                token.symbol,
                                                &token.mint[..8],
                                                token.liquidity
                                                    .as_ref()
                                                    .and_then(|l| l.usd)
                                                    .unwrap_or(0.0)
                                            )
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                log(
                                    LogTag::Discovery,
                                    "ERROR",
                                    &format!("Failed to add tokens to database: {}", e)
                                );
                            }
                        }
                    } else {
                        if is_debug_discovery_enabled() {
                            log(
                                LogTag::Discovery,
                                "WARN",
                                "No token data returned from API for batch"
                            );
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Discovery,
                        "ERROR",
                        &format!("Failed to get token info for batch: {}", e)
                    );
                }
            }

            total_processed += batch.len();

            // Small delay between batches to respect rate limits
            if batch_index < (all_mints.len() + batch_size - 1) / batch_size - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        log(
            LogTag::Discovery,
            "COMPLETE",
            &format!(
                "Discovery cycle completed: processed {} added {} | sources profiles={} boosted={} top_boosts={} rug_new={} rug_viewed={} rug_trending={} rug_verified={} dedup_removed={} blacklist_removed={}",
                total_processed,
                total_added,
                cycle_counts.profiles,
                cycle_counts.boosted,
                cycle_counts.top_boosts,
                cycle_counts.rug_new,
                cycle_counts.rug_viewed,
                cycle_counts.rug_trending,
                cycle_counts.rug_verified,
                dedup_removed,
                blacklisted_count
            )
        );

        // Persist stats (non-blocking)
        if let Some(stats_handle) = DISCOVERY_STATS.get() {
            if let Ok(mut stats) = stats_handle.try_write() {
                stats.last_processed = total_processed;
                stats.last_added = total_added;
                stats.last_deduplicated_removed = dedup_removed;
                stats.last_blacklist_removed = blacklisted_count;
                stats.total_processed = stats.total_processed.saturating_add(
                    total_processed as u64
                );
                stats.total_added = stats.total_added.saturating_add(total_added as u64);
                stats.per_source = cycle_counts;
                stats.last_cycle_completed = Some(Utc::now());
            } else {
                log(LogTag::Discovery, "WARN", "Stats lock busy, skipping final stats update");
            }
        }

        Ok(())
    }
    /// Start continuous discovery loop in background
    pub async fn start_discovery_loop(&mut self, shutdown: Arc<tokio::sync::Notify>) {
        log(LogTag::Discovery, "START", "Discovery loop started");

        // IMPORTANT: Create the shutdown future once to avoid missing notifications
        let mut shutdown_fut = Box::pin(shutdown.notified());

        // Immediate first cycle (non-blocking shutdown check first)
        // Check for shutdown before starting (non-blocking)
        if let Some(_) = shutdown_fut.as_mut().now_or_never() {
            log(LogTag::Discovery, "SHUTDOWN", "Discovery loop stopping before first cycle");
            return;
        }

        log(LogTag::Discovery, "START_FETCHING", "Beginning API data collection");
        if let Err(e) = self.discover_new_tokens(Some(shutdown.clone())).await {
            log(LogTag::Discovery, "ERROR", &format!("Discovery initial cycle failed: {}", e));
        }

        loop {
            tokio::select! {
        _ = shutdown_fut.as_mut() => {
                    log(LogTag::Discovery, "SHUTDOWN", "Discovery loop stopping");
                    break;
                }
                _ = sleep(Duration::from_secs(DISCOVERY_CYCLE_SECONDS)) => {
                    if let Err(e) = self.discover_new_tokens(Some(shutdown.clone())).await {
                        log(LogTag::Discovery, "ERROR", &format!("Discovery cycle failed: {}", e));
                    }
                }
            }
        }

        log(LogTag::Discovery, "STOP", "Discovery loop stopped");
    }
}

// =============================================================================
// PUBLIC HELPER FUNCTIONS
// =============================================================================

/// Start token discovery background task
pub async fn start_token_discovery(
    shutdown: Arc<tokio::sync::Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    log(LogTag::System, "START", "Starting token discovery background task");

    let handle = tokio::spawn(async move {
        let mut discovery = match TokenDiscovery::new() {
            Ok(discovery) => {
                log(LogTag::Discovery, "INIT", "Discovery instance created successfully");
                discovery
            }
            Err(e) => {
                log(LogTag::Discovery, "ERROR", &format!("Failed to initialize discovery: {}", e));
                return;
            }
        };

        log(LogTag::Discovery, "READY", "Starting discovery loop");
        discovery.start_discovery_loop(shutdown).await;
        log(LogTag::Discovery, "EXIT", "Discovery task ended");
    });

    Ok(handle)
}

/// Manual token discovery for testing
pub async fn discover_tokens_once() -> Result<(), String> {
    let mut discovery = TokenDiscovery::new().map_err(|e|
        format!("Failed to create discovery: {}", e)
    )?;
    discovery.discover_new_tokens(None).await
}

/// Get a snapshot of discovery stats for display
pub async fn get_discovery_stats() -> DiscoveryStats {
    let handle = get_discovery_stats_handle();
    let snapshot = {
        let guard = handle.read().await;
        guard.clone()
    };
    snapshot
}
