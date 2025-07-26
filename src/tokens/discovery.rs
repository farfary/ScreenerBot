/// Base token discovery system structure
use crate::logger::{ log, LogTag };
use crate::tokens::api::DexScreenerApi;
use crate::tokens::cache::TokenDatabase;
use tokio::time::{ sleep, Duration };
use std::sync::Arc;

// =============================================================================
// API FUNCTIONS
// =============================================================================

/// Fetch latest token profiles from DexScreener API and extract Solana mint addresses
pub async fn fetch_dexscreener_latest_token_profiles() -> Result<Vec<String>, String> {
    log(LogTag::Discovery, "API", "Fetching latest token profiles from DexScreener");

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.dexscreener.com/token-profiles/latest/v1")
        .header("Accept", "*/*")
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

    log(LogTag::Discovery, "EXTRACTED", &format!("Found {} Solana mints", mints.len()));
    Ok(mints)
}

/// Fetch latest boosted tokens from DexScreener API and extract Solana mint addresses
pub async fn fetch_dexscreener_latest_boosted_tokens() -> Result<Vec<String>, String> {
    log(LogTag::Discovery, "API", "Fetching latest boosted tokens from DexScreener");

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.dexscreener.com/token-boosts/latest/v1")
        .header("Accept", "*/*")
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

    log(LogTag::Discovery, "EXTRACTED", &format!("Found {} Solana boosted mints", mints.len()));
    Ok(mints)
}

/// Fetch tokens with most active boosts from DexScreener API and extract Solana mint addresses
pub async fn fetch_dexscreener_tokens_with_most_active_boosts() -> Result<Vec<String>, String> {
    log(LogTag::Discovery, "API", "Fetching tokens with most active boosts from DexScreener");

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.dexscreener.com/token-boosts/top/v1")
        .header("Accept", "*/*")
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

    log(LogTag::Discovery, "EXTRACTED", &format!("Found {} Solana top boosted mints", mints.len()));
    Ok(mints)
}

/// Fetch new tokens from RugCheck API and extract Solana mint addresses
pub async fn fetch_rugcheck_new_tokens() -> Result<Vec<String>, String> {
    log(LogTag::Discovery, "API", "Fetching new tokens from RugCheck");

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.rugcheck.xyz/v1/stats/new_tokens")
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

    log(
        LogTag::Discovery,
        "EXTRACTED",
        &format!("Found {} Solana new token mints from RugCheck", mints.len())
    );
    Ok(mints)
}

/// Fetch most viewed tokens from RugCheck API and extract Solana mint addresses
pub async fn fetch_rugcheck_most_viewed() -> Result<Vec<String>, String> {
    log(LogTag::Discovery, "API", "Fetching most viewed tokens from RugCheck");

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

    log(
        LogTag::Discovery,
        "EXTRACTED",
        &format!("Found {} Solana most viewed token mints from RugCheck", mints.len())
    );
    Ok(mints)
}

/// Fetch trending tokens from RugCheck API and extract Solana mint addresses
pub async fn fetch_rugcheck_trending() -> Result<Vec<String>, String> {
    log(LogTag::Discovery, "API", "Fetching trending tokens from RugCheck");

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

    log(
        LogTag::Discovery,
        "EXTRACTED",
        &format!("Found {} Solana trending token mints from RugCheck", mints.len())
    );
    Ok(mints)
}

/// Fetch verified tokens from RugCheck API and extract Solana mint addresses
pub async fn fetch_rugcheck_verified() -> Result<Vec<String>, String> {
    log(LogTag::Discovery, "API", "Fetching verified tokens from RugCheck");

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

    log(
        LogTag::Discovery,
        "EXTRACTED",
        &format!("Found {} Solana verified token mints from RugCheck", mints.len())
    );
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

pub struct TokenDiscovery {
    api: DexScreenerApi,
    database: TokenDatabase,
}

impl TokenDiscovery {
    /// Create new token discovery instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api = DexScreenerApi::new();
        let database = TokenDatabase::new()?;

        Ok(Self {
            api,
            database,
        })
    }

    /// Main discovery function - calls all APIs, combines mints, fetches decimals and token info
    pub async fn discover_new_tokens(&mut self) -> Result<(), String> {
        log(LogTag::Discovery, "START", "Starting comprehensive discovery cycle");

        let mut all_mints = Vec::new();

        // Fetch latest token profiles
        match fetch_dexscreener_latest_token_profiles().await {
            Ok(mints) => {
                log(
                    LogTag::Discovery,
                    "SUCCESS",
                    &format!("DexScreener profiles: {} mints", mints.len())
                );
                all_mints.extend(mints);
            }
            Err(e) => {
                log(LogTag::Discovery, "ERROR", &format!("Failed to fetch token profiles: {}", e));
            }
        }

        // Fetch latest boosted tokens
        match fetch_dexscreener_latest_boosted_tokens().await {
            Ok(mints) => {
                log(
                    LogTag::Discovery,
                    "SUCCESS",
                    &format!("DexScreener boosted: {} mints", mints.len())
                );
                all_mints.extend(mints);
            }
            Err(e) => {
                log(LogTag::Discovery, "ERROR", &format!("Failed to fetch boosted tokens: {}", e));
            }
        }

        // Fetch tokens with most active boosts
        match fetch_dexscreener_tokens_with_most_active_boosts().await {
            Ok(mints) => {
                log(
                    LogTag::Discovery,
                    "SUCCESS",
                    &format!("DexScreener top boosts: {} mints", mints.len())
                );
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("Failed to fetch top boosted tokens: {}", e)
                );
            }
        }

        // Fetch new tokens from RugCheck
        match fetch_rugcheck_new_tokens().await {
            Ok(mints) => {
                log(LogTag::Discovery, "SUCCESS", &format!("RugCheck new: {} mints", mints.len()));
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("Failed to fetch new tokens from RugCheck: {}", e)
                );
            }
        }

        // Fetch most viewed tokens from RugCheck
        match fetch_rugcheck_most_viewed().await {
            Ok(mints) => {
                log(
                    LogTag::Discovery,
                    "SUCCESS",
                    &format!("RugCheck viewed: {} mints", mints.len())
                );
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("Failed to fetch most viewed tokens from RugCheck: {}", e)
                );
            }
        }

        // Fetch trending tokens from RugCheck
        match fetch_rugcheck_trending().await {
            Ok(mints) => {
                log(
                    LogTag::Discovery,
                    "SUCCESS",
                    &format!("RugCheck trending: {} mints", mints.len())
                );
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("Failed to fetch trending tokens from RugCheck: {}", e)
                );
            }
        }

        // Fetch verified tokens from RugCheck
        match fetch_rugcheck_verified().await {
            Ok(mints) => {
                log(
                    LogTag::Discovery,
                    "SUCCESS",
                    &format!("RugCheck verified: {} mints", mints.len())
                );
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("Failed to fetch verified tokens from RugCheck: {}", e)
                );
            }
        }

        // Deduplicate mints
        let original_count = all_mints.len();
        all_mints.sort();
        all_mints.dedup();
        let deduplicated_count = all_mints.len();

        log(
            LogTag::Discovery,
            "DEDUP",
            &format!(
                "Deduplicated {} -> {} unique mints (removed {} duplicates)",
                original_count,
                deduplicated_count,
                original_count - deduplicated_count
            )
        );

        if all_mints.is_empty() {
            log(LogTag::Discovery, "COMPLETE", "No tokens to process");
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
            match self.api.get_tokens_info(batch).await {
                Ok(tokens) => {
                    if !tokens.is_empty() {
                        // Add tokens to database
                        match self.database.add_tokens(&tokens).await {
                            Ok(_) => {
                                total_added += tokens.len();
                                log(
                                    LogTag::Discovery,
                                    "DATABASE",
                                    &format!("Added {} tokens to database", tokens.len())
                                );

                                // Log some token details
                                for token in &tokens {
                                    log(
                                        LogTag::Discovery,
                                        "TOKEN",
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
                            Err(e) => {
                                log(
                                    LogTag::Discovery,
                                    "ERROR",
                                    &format!("Failed to add tokens to database: {}", e)
                                );
                            }
                        }
                    } else {
                        log(LogTag::Discovery, "WARN", "No token data returned from API for batch");
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
                "Discovery cycle completed: processed {} tokens, added {} to database",
                total_processed,
                total_added
            )
        );

        Ok(())
    }
    /// Start continuous discovery loop in background
    pub async fn start_discovery_loop(&mut self, shutdown: Arc<tokio::sync::Notify>) {
        log(LogTag::Discovery, "START", "Discovery loop started");

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Discovery, "SHUTDOWN", "Discovery loop stopping");
                    break;
                }
                
                _ = sleep(Duration::from_secs(DISCOVERY_CYCLE_SECONDS)) => {
                    if let Err(e) = self.discover_new_tokens().await {
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
            Ok(discovery) => discovery,
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Failed to initialize discovery: {}", e));
                return;
            }
        };

        discovery.start_discovery_loop(shutdown).await;
    });

    Ok(handle)
}

/// Manual token discovery for testing
pub async fn discover_tokens_once() -> Result<(), String> {
    let mut discovery = TokenDiscovery::new().map_err(|e|
        format!("Failed to create discovery: {}", e)
    )?;
    discovery.discover_new_tokens().await
}
