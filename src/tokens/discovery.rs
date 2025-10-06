use crate::global::is_debug_discovery_enabled;
/// Base token discovery system structure
use crate::logger::{ log, LogTag };
use crate::tokens::cache::TokenDatabase;
use crate::tokens::dexscreener::get_global_dexscreener_api;
use crate::tokens::is_token_excluded_from_trading;
use chrono::{ DateTime, Utc };
use futures::FutureExt;
use reqwest::Client;
use std::sync::{ Arc, OnceLock };
use tokio::sync::RwLock;
use tokio::time::{ sleep, Duration }; // for now_or_never on shutdown future

// =============================================================================
// NETWORK CONSTANTS / HELPERS
// =============================================================================
// =============================================================================
/// Per-endpoint HTTP timeout for discovery API calls (seconds)
/// Increased to 15s to accommodate RugCheck new_tokens endpoint (~10s response time)
const DISCOVERY_HTTP_TIMEOUT_SECS: u64 = 15;

/// Delay between API calls in discovery cycle (seconds)
/// Prevents overwhelming external APIs with rapid consecutive requests
const DISCOVERY_API_DELAY_SECS: u64 = 3;

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
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "DEBUG", "ENTERED fetch_dexscreener_latest_token_profiles function");
        log(LogTag::Discovery, "API", "Fetching latest token profiles from DexScreener");
        log(LogTag::Discovery, "DEBUG", "Building HTTP client...");
    }
    let client = build_discovery_client()?;

    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "DEBUG", "Making HTTP request to profiles API...");
    }
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

/// Fetch recently updated tokens from GeckoTerminal API and extract Solana mint addresses
pub async fn fetch_geckoterminal_recently_updated() -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "API", "Fetching recently updated tokens from GeckoTerminal");
    }

    let client = build_discovery_client()?;
    let response = client
        .get(
            "https://api.geckoterminal.com/api/v2/tokens/info_recently_updated?include=network&network=solana"
        )
        .header("accept", "application/json")
        .send().await
        .map_err(|e| format!("HTTP request failed (gecko_updated): {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(data) = json.get("data").and_then(|v| v.as_array()) {
        for item in data {
            if let Some(attributes) = item.get("attributes") {
                if let Some(address) = attributes.get("address").and_then(|v| v.as_str()) {
                    mints.push(address.to_string());
                }
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "EXTRACTED",
            &format!("Found {} Solana recently updated token mints from GeckoTerminal", mints.len())
        );
    }
    Ok(mints)
}

/// Fetch tokens from GeckoTerminal trending pools API across all pages and durations
pub async fn fetch_geckoterminal_trending_pools() -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "API",
            "Fetching trending pools tokens from GeckoTerminal (all pages & durations)"
        );
    }

    let client = build_discovery_client()?;
    let mut all_mints = Vec::new();

    // Available durations: 5m, 1h, 6h, 24h
    let durations = ["5m", "1h", "6h", "24h"];

    for duration in durations.iter() {
        if is_debug_discovery_enabled() {
            log(
                LogTag::Discovery,
                "DURATION",
                &format!("Fetching trending pools for duration: {}", duration)
            );
        }

        // Fetch all pages (1-10) for this duration
        for page in 1..=10 {
            let url = format!(
                "https://api.geckoterminal.com/api/v2/networks/solana/trending_pools?include=base_token%2Cquote_token%2Cdex&page={}&duration={}",
                page,
                duration
            );

            if is_debug_discovery_enabled() {
                log(
                    LogTag::Discovery,
                    "PAGE",
                    &format!("Fetching page {} for duration {}", page, duration)
                );
            }

            let response = client
                .get(&url)
                .header("accept", "application/json")
                .send().await
                .map_err(|e| {
                    format!(
                        "HTTP request failed (gecko_trending page {} duration {}): {}",
                        page,
                        duration,
                        e
                    )
                })?;

            if !response.status().is_success() {
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "WARN",
                        &format!(
                            "Page {} duration {} returned status: {} - skipping",
                            page,
                            duration,
                            response.status()
                        )
                    );
                }
                continue; // Skip this page but continue with others
            }

            let text = response
                .text().await
                .map_err(|e| format!("Failed to read response: {}", e))?;

            let json: serde_json::Value = serde_json
                ::from_str(&text)
                .map_err(|e| format!("Failed to parse JSON: {}", e))?;

            let mut page_mints = Vec::new();

            // Extract token mints from pools data
            if let Some(data) = json.get("data").and_then(|v| v.as_array()) {
                if data.is_empty() {
                    if is_debug_discovery_enabled() {
                        log(
                            LogTag::Discovery,
                            "END",
                            &format!(
                                "No more data at page {} for duration {} - stopping pagination",
                                page,
                                duration
                            )
                        );
                    }
                    break; // No more data for this duration
                }

                for pool in data {
                    if let Some(relationships) = pool.get("relationships") {
                        // Extract base token address
                        if
                            let Some(base_token) = relationships
                                .get("base_token")
                                .and_then(|bt| bt.get("data"))
                                .and_then(|data| data.get("id"))
                                .and_then(|id| id.as_str())
                        {
                            // Extract the actual address from the ID (format: "solana_ADDRESS")
                            if let Some(address) = base_token.strip_prefix("solana_") {
                                page_mints.push(address.to_string());
                            }
                        }

                        // Extract quote token address
                        if
                            let Some(quote_token) = relationships
                                .get("quote_token")
                                .and_then(|qt| qt.get("data"))
                                .and_then(|data| data.get("id"))
                                .and_then(|id| id.as_str())
                        {
                            // Extract the actual address from the ID (format: "solana_ADDRESS")
                            if let Some(address) = quote_token.strip_prefix("solana_") {
                                page_mints.push(address.to_string());
                            }
                        }
                    }
                }
            }

            if is_debug_discovery_enabled() {
                log(
                    LogTag::Discovery,
                    "EXTRACTED",
                    &format!(
                        "Page {} duration {}: found {} token mints",
                        page,
                        duration,
                        page_mints.len()
                    )
                );
            }

            all_mints.extend(page_mints);
        }
    }

    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "EXTRACTED",
            &format!(
                "Found {} total token mints from GeckoTerminal trending pools (all durations & pages)",
                all_mints.len()
            )
        );
    }
    Ok(all_mints)
}

/// Fetch recent tokens from Jupiter API and extract Solana mint addresses
pub async fn fetch_jupiter_recent_tokens() -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "DEBUG", "ENTERED fetch_jupiter_recent_tokens function");
        log(LogTag::Discovery, "API", "Fetching recent tokens from Jupiter");
    }
    let client = build_discovery_client()?;

    let response = client
        .get("https://lite-api.jup.ag/tokens/v2/recent")
        .header("Accept", "application/json")
        .send().await
        .map_err(|e| format!("HTTP request failed (Jupiter recent): {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Jupiter recent API returned status: {}", response.status()));
    }

    let text = response
        .text().await
        .map_err(|e| format!("Failed to read Jupiter recent response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(array) = json.as_array() {
        for token in array {
            if let Some(address) = token.get("id").and_then(|v| v.as_str()) {
                // Verify it's a valid Solana address format (base58, ~44 chars)
                if address.len() > 32 && address.len() < 50 {
                    mints.push(address.to_string());
                }
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "EXTRACTED",
            &format!("Found {} recent token mints from Jupiter", mints.len())
        );
    }

    Ok(mints)
}

/// Fetch top organic score tokens from Jupiter API and extract Solana mint addresses
pub async fn fetch_jupiter_top_organic_score() -> Result<Vec<String>, String> {
    fetch_jupiter_category_tokens("toporganicscore", "24h").await
}

/// Fetch top traded tokens from Jupiter API and extract Solana mint addresses
pub async fn fetch_jupiter_top_traded() -> Result<Vec<String>, String> {
    fetch_jupiter_category_tokens("toptraded", "24h").await
}

/// Fetch top trending tokens from Jupiter API and extract Solana mint addresses
pub async fn fetch_jupiter_top_trending() -> Result<Vec<String>, String> {
    fetch_jupiter_category_tokens("toptrending", "24h").await
}

/// Generic function to fetch tokens from Jupiter category API
async fn fetch_jupiter_category_tokens(
    category: &str,
    interval: &str
) -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "DEBUG",
            &format!(
                "ENTERED fetch_jupiter_category_tokens function for category: {} interval: {}",
                category,
                interval
            )
        );
        log(
            LogTag::Discovery,
            "API",
            &format!("Fetching {} tokens from Jupiter for {} interval", category, interval)
        );
    }
    let client = build_discovery_client()?;

    let url = format!("https://lite-api.jup.ag/tokens/v2/{}/{}?limit=100", category, interval);
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send().await
        .map_err(|e| format!("HTTP request failed (Jupiter {}): {}", category, e))?;

    if !response.status().is_success() {
        return Err(format!("Jupiter {} API returned status: {}", category, response.status()));
    }

    let text = response
        .text().await
        .map_err(|e| format!("Failed to read Jupiter {} response: {}", category, e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(array) = json.as_array() {
        for token in array {
            if let Some(address) = token.get("id").and_then(|v| v.as_str()) {
                // Verify it's a valid Solana address format (base58, ~44 chars)
                if address.len() > 32 && address.len() < 50 {
                    mints.push(address.to_string());
                }
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "EXTRACTED",
            &format!(
                "Found {} {} token mints from Jupiter for {} interval",
                mints.len(),
                category,
                interval
            )
        );
    }

    Ok(mints)
}

/// CoinGecko API key for authenticated requests
const COINGECKO_API_KEY: &str = "COINGECKO_KEY_REMOVED";

/// Fetch Solana ecosystem tokens from CoinGecko API and extract mint addresses
pub async fn fetch_coingecko_solana_markets() -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "DEBUG", "ENTERED fetch_coingecko_solana_markets function");
        log(LogTag::Discovery, "API", "Fetching Solana ecosystem tokens from CoinGecko");
    }
    let client = build_discovery_client()?;

    // Use coins list endpoint with platform filter to get Solana tokens
    let response = client
        .get("https://api.coingecko.com/api/v3/coins/list?include_platform=true")
        .header("Accept", "application/json")
        .header("x-cg-demo-api-key", COINGECKO_API_KEY)
        .send().await
        .map_err(|e| format!("HTTP request failed (CoinGecko): {}", e))?;

    if !response.status().is_success() {
        return Err(format!("CoinGecko API returned status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(array) = json.as_array() {
        for token in array {
            // Try to extract Solana contract address from platforms
            if let Some(platforms) = token.get("platforms").and_then(|p| p.as_object()) {
                if let Some(solana_address) = platforms.get("solana").and_then(|v| v.as_str()) {
                    if
                        !solana_address.is_empty() &&
                        solana_address.len() > 32 &&
                        solana_address.len() < 50
                    {
                        mints.push(solana_address.to_string());
                    }
                }
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "EXTRACTED",
            &format!("Found {} Solana token mints from CoinGecko", mints.len())
        );
    }

    Ok(mints)
}

/// Fetch DeFi protocols from DeFiLlama API and extract Solana token addresses
pub async fn fetch_defillama_protocols() -> Result<Vec<String>, String> {
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "DEBUG", "ENTERED fetch_defillama_protocols function");
        log(LogTag::Discovery, "API", "Fetching protocols from DeFiLlama");
    }
    let client = build_discovery_client()?;

    let response = client
        .get("https://api.llama.fi/protocols")
        .header("Accept", "application/json")
        .send().await
        .map_err(|e| format!("HTTP request failed (DeFiLlama): {}", e))?;

    if !response.status().is_success() {
        return Err(format!("DeFiLlama API returned status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut mints = Vec::new();

    if let Some(array) = json.as_array() {
        for protocol in array {
            // Check if protocol supports Solana
            if let Some(chains) = protocol.get("chains").and_then(|c| c.as_array()) {
                let has_solana = chains
                    .iter()
                    .any(|chain| {
                        chain.as_str().map_or(false, |s| s.to_lowercase().contains("solana"))
                    });

                if has_solana {
                    // Try to extract token address from various fields
                    if let Some(address) = protocol.get("address").and_then(|v| v.as_str()) {
                        if address.len() > 32 && address.len() < 50 {
                            mints.push(address.to_string());
                        }
                    }
                }
            }
        }
    }

    if is_debug_discovery_enabled() {
        log(
            LogTag::Discovery,
            "EXTRACTED",
            &format!("Found {} Solana protocol token mints from DeFiLlama", mints.len())
        );
    }

    Ok(mints)
}

/// Fetch current price for a specific Solana token from DeFiLlama API
pub async fn fetch_defillama_token_price(mint: &str) -> Result<f64, String> {
    if is_debug_discovery_enabled() {
        log(LogTag::Discovery, "DEBUG", &format!("Fetching DeFiLlama price for mint: {}", mint));
    }
    let client = build_discovery_client()?;

    let url = format!("https://coins.llama.fi/prices/current/solana:{}", mint);
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send().await
        .map_err(|e| format!("HTTP request failed (DeFiLlama price): {}", e))?;

    if !response.status().is_success() {
        return Err(format!("DeFiLlama price API returned status: {}", response.status()));
    }

    let text = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

    let json: serde_json::Value = serde_json
        ::from_str(&text)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Extract price from coins.{chain:address}.price
    let price_key = format!("solana:{}", mint);
    if
        let Some(price) = json
            .get("coins")
            .and_then(|coins| coins.get(&price_key))
            .and_then(|token| token.get("price"))
            .and_then(|p| p.as_f64())
    {
        if is_debug_discovery_enabled() {
            log(
                LogTag::Discovery,
                "EXTRACTED",
                &format!("DeFiLlama price for {}: ${:.6}", mint, price)
            );
        }
        Ok(price)
    } else {
        Err(format!("Price not found for token: {}", mint))
    }
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
    pub gecko_updated: usize,
    pub gecko_trending: usize,
    pub jupiter_tokens: usize,
    pub jupiter_top_organic: usize,
    pub jupiter_top_traded: usize,
    pub jupiter_top_trending: usize,
    pub coingecko_markets: usize,
    pub defillama_protocols: usize,
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

        Ok(Self { database })
    }

    /// Main discovery function - calls all APIs, combines mints, fetches decimals and token info
    pub async fn discover_new_tokens(
        &mut self,
        shutdown: Option<Arc<tokio::sync::Notify>>
    ) -> Result<(), String> {
        use crate::utils::check_shutdown_or_delay;
        use tokio::time::Duration;

        // Always log cycle start (visibility even without --debug-discovery)
        if is_debug_discovery_enabled() {
            log(LogTag::Discovery, "START", "Starting comprehensive discovery cycle");
        }

        // Mark stats: cycle start (non-blocking)
        if let Some(stats_handle) = DISCOVERY_STATS.get() {
            if let Ok(mut stats) = stats_handle.try_write() {
                stats.total_cycles = stats.total_cycles.saturating_add(1);
                stats.last_cycle_started = Some(Utc::now());
                stats.last_error = None; // reset at start
                // reset per-source for this cycle; will be overwritten below
                stats.per_source = DiscoverySourceCounts::default();
            } else {
                if is_debug_discovery_enabled() {
                    log(LogTag::Discovery, "WARN", "Stats lock busy, skipping stats update");
                }
            }
        }

        let mut all_mints = Vec::new();
        // Per-cycle source counters
        let mut cycle_counts = DiscoverySourceCounts::default();

        if is_debug_discovery_enabled() {
            log(LogTag::Discovery, "API_START", "About to fetch from profiles API");

            log(
                LogTag::Discovery,
                "DEBUG",
                "Calling fetch_dexscreener_latest_token_profiles() function..."
            );
        }

        // Fetch latest token profiles
        match fetch_dexscreener_latest_token_profiles().await {
            Ok(mints) => {
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("Profiles fetched: {}", mints.len())
                    );
                }
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

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch latest boosted tokens
        match fetch_dexscreener_latest_boosted_tokens().await {
            Ok(mints) => {
                if is_debug_discovery_enabled() {
                    log(LogTag::Discovery, "SUCCESS", &format!("Boosted fetched: {}", mints.len()));
                }
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

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch tokens with most active boosts
        match fetch_dexscreener_tokens_with_most_active_boosts().await {
            Ok(mints) => {
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("Top boosts fetched: {}", mints.len())
                    );
                }
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

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await; // Fetch new tokens from RugCheck
        match fetch_rugcheck_new_tokens().await {
            Ok(mints) => {
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("RugCheck new fetched: {}", mints.len())
                    );
                }
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

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch most viewed tokens from RugCheck
        match fetch_rugcheck_most_viewed().await {
            Ok(mints) => {
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("RugCheck viewed fetched: {}", mints.len())
                    );
                }
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

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch trending tokens from RugCheck
        match fetch_rugcheck_trending().await {
            Ok(mints) => {
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("RugCheck trending fetched: {}", mints.len())
                    );
                }
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

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch verified tokens from RugCheck
        match fetch_rugcheck_verified().await {
            Ok(mints) => {
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("RugCheck verified fetched: {}", mints.len())
                    );
                }
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

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch recently updated tokens from GeckoTerminal
        match fetch_geckoterminal_recently_updated().await {
            Ok(mints) => {
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("GeckoTerminal updated fetched: {}", mints.len())
                    );
                }
                cycle_counts.gecko_updated = mints.len();
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("GeckoTerminal updated fetch failed: {}", e)
                );
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("gecko_updated: {}", e));
                    }
                }
            }
        }

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch trending pools tokens from GeckoTerminal
        match fetch_geckoterminal_trending_pools().await {
            Ok(mints) => {
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("GeckoTerminal trending pools fetched: {}", mints.len())
                    );
                }
                cycle_counts.gecko_trending = mints.len();
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("GeckoTerminal trending pools fetch failed: {}", e)
                );
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("gecko_trending: {}", e));
                    }
                }
            }
        }

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch recent tokens from Jupiter
        match fetch_jupiter_recent_tokens().await {
            Ok(mints) => {
                let mints_count = mints.len();
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("Jupiter recent tokens fetched: {} tokens", mints_count)
                    );
                }
                cycle_counts.jupiter_tokens = mints_count;
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("Jupiter recent tokens fetch failed: {}", e)
                );
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("jupiter_recent: {}", e));
                    }
                }
            }
        }

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch top organic score tokens from Jupiter
        match fetch_jupiter_top_organic_score().await {
            Ok(mints) => {
                let mints_count = mints.len();
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("Jupiter top organic score tokens fetched: {} tokens", mints_count)
                    );
                }
                cycle_counts.jupiter_top_organic = mints_count;
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("Jupiter top organic score tokens fetch failed: {}", e)
                );
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("jupiter_top_organic: {}", e));
                    }
                }
            }
        }

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch top traded tokens from Jupiter
        match fetch_jupiter_top_traded().await {
            Ok(mints) => {
                let mints_count = mints.len();
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("Jupiter top traded tokens fetched: {} tokens", mints_count)
                    );
                }
                cycle_counts.jupiter_top_traded = mints_count;
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("Jupiter top traded tokens fetch failed: {}", e)
                );
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("jupiter_top_traded: {}", e));
                    }
                }
            }
        }

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch top trending tokens from Jupiter
        match fetch_jupiter_top_trending().await {
            Ok(mints) => {
                let mints_count = mints.len();
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("Jupiter top trending tokens fetched: {} tokens", mints_count)
                    );
                }
                cycle_counts.jupiter_top_trending = mints_count;
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("Jupiter top trending tokens fetch failed: {}", e)
                );
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("jupiter_top_trending: {}", e));
                    }
                }
            }
        }

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch Solana ecosystem markets from CoinGecko
        match fetch_coingecko_solana_markets().await {
            Ok(mints) => {
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("CoinGecko Solana markets fetched: {}", mints.len())
                    );
                }
                cycle_counts.coingecko_markets = mints.len();
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("CoinGecko Solana markets fetch failed: {}", e)
                );
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("coingecko_markets: {}", e));
                    }
                }
            }
        }

        // Delay between API calls
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

        // Fetch DeFi protocols from DeFiLlama
        match fetch_defillama_protocols().await {
            Ok(mints) => {
                if is_debug_discovery_enabled() {
                    log(
                        LogTag::Discovery,
                        "SUCCESS",
                        &format!("DeFiLlama protocols fetched: {}", mints.len())
                    );
                }
                cycle_counts.defillama_protocols = mints.len();
                all_mints.extend(mints);
            }
            Err(e) => {
                log(
                    LogTag::Discovery,
                    "ERROR",
                    &format!("DeFiLlama protocols fetch failed: {}", e)
                );
                if let Some(stats_handle) = DISCOVERY_STATS.get() {
                    if let Ok(mut stats) = stats_handle.try_write() {
                        stats.last_error = Some(format!("defillama_protocols: {}", e));
                    }
                }
            }
        }

        // Final delay after last API call
        sleep(Duration::from_secs(DISCOVERY_API_DELAY_SECS)).await;

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

        if is_debug_discovery_enabled() {
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
        }

        // REMOVED: Price service seeding that was causing resource waste
        // Discovery tokens should not be added to priority monitoring
        // Only open positions should be priority per user requirements
        if !all_mints.is_empty() {
            if is_debug_discovery_enabled() {
                log(
                    LogTag::Discovery,
                    "DISCOVERY_COMPLETE",
                    &format!(
                        "📊 Discovery completed: {} tokens found (not added to priority monitoring)",
                        all_mints.len().min(50)
                    )
                );
            }
        }

        if all_mints.is_empty() {
            if is_debug_discovery_enabled() {
                log(LogTag::Discovery, "COMPLETE", "No tokens to process");
            }
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
            if is_debug_discovery_enabled() {
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
            }

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
                            if is_debug_discovery_enabled() {
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
                        }

                        if tokens.is_empty() {
                            if is_debug_discovery_enabled() {
                                log(
                                    LogTag::Discovery,
                                    "SKIP",
                                    "No tokens remaining after decimal validation"
                                );
                            }
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

        if is_debug_discovery_enabled() {
            log(
                LogTag::Discovery,
                "COMPLETE",
                &format!(
                    "Discovery cycle completed: processed {} added {} | sources profiles={} boosted={} top_boosts={} rug_new={} rug_viewed={} rug_trending={} rug_verified={} gecko_updated={} gecko_trending={} dedup_removed={} blacklist_removed={}",
                    total_processed,
                    total_added,
                    cycle_counts.profiles,
                    cycle_counts.boosted,
                    cycle_counts.top_boosts,
                    cycle_counts.rug_new,
                    cycle_counts.rug_viewed,
                    cycle_counts.rug_trending,
                    cycle_counts.rug_verified,
                    cycle_counts.gecko_updated,
                    cycle_counts.gecko_trending,
                    dedup_removed,
                    blacklisted_count
                )
            );
        }

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
                stats.per_source = cycle_counts.clone();
                stats.last_cycle_completed = Some(Utc::now());
            } else {
                if is_debug_discovery_enabled() {
                    log(LogTag::Discovery, "WARN", "Stats lock busy, skipping final stats update");
                }
            }
        }

        // Print detailed discovery cycle summary
        print_discovery_cycle_summary(
            total_processed,
            total_added,
            dedup_removed,
            blacklisted_count,
            &cycle_counts
        ).await;

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

        if is_debug_discovery_enabled() {
            log(LogTag::Discovery, "START_FETCHING", "Beginning API data collection");
        }
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
// DISCOVERY CYCLE SUMMARY
// =============================================================================

/// Print a detailed, colorful summary of the discovery cycle results
async fn print_discovery_cycle_summary(
    processed: usize,
    added: usize,
    dedup_removed: usize,
    blacklist_removed: usize,
    cycle_counts: &DiscoverySourceCounts
) {
    // Get current stats for context
    let stats = get_discovery_stats().await;

    // Calculate cycle duration
    let cycle_duration = if
        let (Some(start), Some(end)) = (stats.last_cycle_started, stats.last_cycle_completed)
    {
        let duration = end.signed_duration_since(start);
        if duration.num_seconds() > 0 {
            format!("{}s", duration.num_seconds())
        } else {
            format!("{}ms", duration.num_milliseconds())
        }
    } else {
        "N/A".to_string()
    };

    let total_fetched =
        cycle_counts.profiles +
        cycle_counts.boosted +
        cycle_counts.top_boosts +
        cycle_counts.rug_new +
        cycle_counts.rug_viewed +
        cycle_counts.rug_trending +
        cycle_counts.rug_verified +
        cycle_counts.gecko_updated +
        cycle_counts.gecko_trending;

    let dex_total = cycle_counts.profiles + cycle_counts.boosted + cycle_counts.top_boosts;
    let rug_total =
        cycle_counts.rug_new +
        cycle_counts.rug_viewed +
        cycle_counts.rug_trending +
        cycle_counts.rug_verified;
    let gecko_total = cycle_counts.gecko_updated + cycle_counts.gecko_trending;

    let success_rate = if stats.total_processed > 0 {
        ((stats.total_added as f64) / (stats.total_processed as f64)) * 100.0
    } else {
        0.0
    };

    let error_status = if let Some(error) = &stats.last_error {
        format!("⚠️  LAST ERROR: {}", error)
    } else {
        "✅ STATUS: All API endpoints successful - no errors".to_string()
    };

    let timing_info = if let Some(completed) = stats.last_cycle_completed {
        let now = Utc::now();
        let time_since = now.signed_duration_since(completed);
        format!(
            "⏰ TIMING: Completed {} | Next cycle in ~{}s",
            completed.format("%H:%M:%S UTC"),
            DISCOVERY_CYCLE_SECONDS.saturating_sub(time_since.num_seconds() as u64)
        )
    } else {
        "⏰ TIMING: N/A".to_string()
    };

    let filtering_info = if dedup_removed > 0 || blacklist_removed > 0 {
        let mut filter_parts = Vec::new();
        if dedup_removed > 0 {
            filter_parts.push(format!("♻️  {} duplicates", dedup_removed));
        }
        if blacklist_removed > 0 {
            filter_parts.push(format!("🚫 {} blacklisted", blacklist_removed));
        }
        format!("🧹 FILTERING: Removed {}", filter_parts.join(" + "))
    } else {
        "🧹 FILTERING: No tokens filtered".to_string()
    };

    let results_emoji = if added > 0 { "🎉" } else { "ℹ️" };
    let results_text = if added > 0 {
        format!("DISCOVERED {} NEW VALID TOKENS (no duplicates/blacklisted)!", added)
    } else {
        "NO NEW TOKENS: All fetched tokens already exist in database".to_string()
    };

    let header_line = "════════════════════════════════════════════════════════════";
    let title = "🔍 DISCOVERY SUMMARY - Comprehensive Token Sweep";
    let cycle_line = format!(
        "  • Cycle     📊  #{:<3} | Duration: {} | Total Lifetime: {} cycles",
        stats.total_cycles,
        cycle_duration,
        stats.total_cycles
    );
    let results_line = format!(
        "  • Results   🎯  Processed {} | 🆕 NEW VALID: {} | Filtered out: {}",
        processed,
        added,
        dedup_removed + blacklist_removed
    );
    let status_line = format!("  • Status    {} {}", results_emoji, results_text);

    let api_breakdown_line = format!("  • API Calls 📚  {} total from 9 endpoints:", total_fetched);
    let dex_line = format!(
        "    🔸 DexScreener: Profiles({}) + Boosted({}) + TopBoosts({}) = {}",
        cycle_counts.profiles,
        cycle_counts.boosted,
        cycle_counts.top_boosts,
        dex_total
    );
    let rug_line = format!(
        "    🔸 RugCheck: New({}) + Viewed({}) + Trending({}) + Verified({}) = {}",
        cycle_counts.rug_new,
        cycle_counts.rug_viewed,
        cycle_counts.rug_trending,
        cycle_counts.rug_verified,
        rug_total
    );
    let gecko_line = format!(
        "    🔸 GeckoTerminal: Updated({}) + TrendingPools({}) = {}",
        cycle_counts.gecko_updated,
        cycle_counts.gecko_trending,
        gecko_total
    );

    let filtering_line = format!(
        "  • Filtering {}",
        filtering_info.replace("🧹 FILTERING: ", "🧹  ")
    );
    let error_line = format!("  • Status    {}", error_status);
    let lifetime_line = format!(
        "  • Lifetime  📈  Processed {} | 🆕 Total Valid Added {} | Success Rate {:.1}%",
        stats.total_processed,
        stats.total_added,
        success_rate
    );
    let timing_line = format!("  • Timing    {}", timing_info.replace("⏰ TIMING: ", "⏰  "));

    let body = format!(
        "\n{header}\n{title}\n{header}\n{cycle}\n{results}\n{status}\n\n{api_breakdown}\n{dex}\n{rug}\n{gecko}\n\n{filtering}\n{error}\n{lifetime}\n{timing}\n{header}",
        header = header_line,
        title = title,
        cycle = cycle_line,
        results = results_line,
        status = status_line,
        api_breakdown = api_breakdown_line,
        dex = dex_line,
        rug = rug_line,
        gecko = gecko_line,
        filtering = filtering_line,
        error = error_line,
        lifetime = lifetime_line,
        timing = timing_line
    );

    log(LogTag::Discovery, "SUMMARY", &body);
}

// =============================================================================
// PUBLIC HELPER FUNCTIONS
// =============================================================================

/// Start token discovery background task
pub async fn start_token_discovery(
    shutdown: Arc<tokio::sync::Notify>,
    monitor: tokio_metrics::TaskMonitor
) -> Result<tokio::task::JoinHandle<()>, String> {
    log(LogTag::System, "START", "Starting token discovery background task (instrumented)");

    let handle = tokio::spawn(
        monitor.instrument(async move {
            let mut discovery = match TokenDiscovery::new() {
                Ok(discovery) => {
                    if is_debug_discovery_enabled() {
                        log(LogTag::Discovery, "INIT", "Discovery instance created successfully");
                    }
                    discovery
                }
                Err(e) => {
                    log(
                        LogTag::Discovery,
                        "ERROR",
                        &format!("Failed to initialize discovery: {}", e)
                    );
                    return;
                }
            };

            // Wait for Transactions system to be ready before starting discovery
            let mut last_log = std::time::Instant::now();
            loop {
                let tx_ready = crate::global::TRANSACTIONS_SYSTEM_READY.load(
                    std::sync::atomic::Ordering::SeqCst
                );

                if tx_ready {
                    if is_debug_discovery_enabled() {
                        log(
                            LogTag::Discovery,
                            "READY",
                            "✅ Transactions ready. Starting discovery loop"
                        );
                    }
                    break;
                }

                // Log only every 15 seconds
                if last_log.elapsed() >= std::time::Duration::from_secs(15) {
                    log(
                        LogTag::Discovery,
                        "READY",
                        "⏳ Waiting for Transactions system to be ready..."
                    );
                    last_log = std::time::Instant::now();
                }

                tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Discovery, "EXIT", "Discovery exiting during dependency wait");
                    return;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
            }
            }

            discovery.start_discovery_loop(shutdown).await;
            if is_debug_discovery_enabled() {
                log(LogTag::Discovery, "EXIT", "Discovery task ended");
            }
        })
    );

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
