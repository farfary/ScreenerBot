/// Token decimals fetching from Solana blockchain
use crate::logger::{ log, LogTag };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_program::program_pack::Pack;
use spl_token::state::Mint;
use std::str::FromStr;
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;
use serde::{ Serialize, Deserialize };

const CACHE_FILE_NAME: &str = "decimal_cache.json";

#[derive(Serialize, Deserialize)]
struct DecimalCacheData {
    decimals: HashMap<String, u8>,
    failed_tokens: HashMap<String, String>, // mint -> error message
}

// Cache for token decimals to avoid repeated RPC calls
static DECIMAL_CACHE: Lazy<Arc<Mutex<HashMap<String, u8>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(load_cache_from_disk().0))
});

// Cache for failed token lookups to avoid repeated failures
static FAILED_DECIMALS_CACHE: Lazy<Arc<Mutex<HashMap<String, String>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(load_cache_from_disk().1))
});

/// Load decimal cache from disk
fn load_cache_from_disk() -> (HashMap<String, u8>, HashMap<String, String>) {
    if Path::new(CACHE_FILE_NAME).exists() {
        match fs::read_to_string(CACHE_FILE_NAME) {
            Ok(content) => {
                match serde_json::from_str::<DecimalCacheData>(&content) {
                    Ok(cache_data) => {
                        log(
                            LogTag::Decimals,
                            "CACHE",
                            &format!(
                                "Loaded {} decimal entries and {} failed entries from cache file",
                                cache_data.decimals.len(),
                                cache_data.failed_tokens.len()
                            )
                        );
                        return (cache_data.decimals, cache_data.failed_tokens);
                    }
                    Err(e) => {
                        // Try to parse old format (without failed_tokens)
                        if
                            let Ok(old_cache) = serde_json::from_str::<HashMap<String, u8>>(
                                &content
                            )
                        {
                            log(
                                LogTag::Decimals,
                                "CACHE",
                                &format!(
                                    "Loaded {} decimal entries from old format cache file",
                                    old_cache.len()
                                )
                            );
                            return (old_cache, HashMap::new());
                        }

                        log(
                            LogTag::Decimals,
                            "WARN",
                            &format!("Failed to parse decimal cache file: {}", e)
                        );
                    }
                }
            }
            Err(e) => {
                log(LogTag::Decimals, "WARN", &format!("Failed to read decimal cache file: {}", e));
            }
        }
    }

    (HashMap::new(), HashMap::new())
}

/// Save decimal cache to disk
fn save_cache_to_disk(cache: &HashMap<String, u8>, failed_cache: &HashMap<String, String>) {
    let cache_data = DecimalCacheData {
        decimals: cache.clone(),
        failed_tokens: failed_cache.clone(),
    };

    match serde_json::to_string_pretty(&cache_data) {
        Ok(json) => {
            if let Err(e) = fs::write(CACHE_FILE_NAME, json) {
                log(
                    LogTag::Decimals,
                    "ERROR",
                    &format!("Failed to save decimal cache to disk: {}", e)
                );
            }
        }
        Err(e) => {
            log(LogTag::Decimals, "ERROR", &format!("Failed to serialize decimal cache: {}", e));
        }
    }
}

/// Get token decimals from Solana blockchain with caching
pub async fn get_token_decimals_from_chain(mint: &str) -> Result<u8, String> {
    // Check successful decimals cache first
    if let Ok(cache) = DECIMAL_CACHE.lock() {
        if let Some(&decimals) = cache.get(mint) {
            return Ok(decimals);
        }
    }

    // Check failed decimals cache
    if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        if let Some(error) = failed_cache.get(mint) {
            log(
                LogTag::Decimals,
                "CACHED_FAIL",
                &format!("Skipping previously failed token {}: {}", mint, error)
            );
            return Err(error.clone());
        }
    }

    // Use the batch function for single token (more efficient than separate implementation)
    let results = batch_fetch_token_decimals(&[mint.to_string()]).await;

    if let Some((_, result)) = results.first() {
        result.clone()
    } else {
        Err("No results returned from batch fetch".to_string())
    }
}

/// Check if a token has already failed decimal lookup
fn is_token_already_failed(mint: &str) -> bool {
    if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        failed_cache.contains_key(mint)
    } else {
        false
    }
}

/// Add a token to the failed cache
fn cache_failed_token(mint: &str, error: &str) {
    if let Ok(mut failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        failed_cache.insert(mint.to_string(), error.to_string());
        log(
            LogTag::Decimals,
            "CACHE_FAIL",
            &format!("Cached failed lookup for {}: {}", mint, error)
        );
    }
}

/// Check if error should be cached as failed (real errors) vs retried (rate limits)
fn should_cache_as_failed(error: &str) -> bool {
    let error_lower = error.to_lowercase();

    // Real blockchain state errors - cache as failed
    if
        error_lower.contains("account not found") ||
        error_lower.contains("invalid account") ||
        error_lower.contains("account does not exist") ||
        error_lower.contains("invalid mint") ||
        error_lower.contains("empty")
    {
        return true;
    }

    // Rate limiting and temporary issues - retry with different RPC
    if
        error_lower.contains("429") ||
        error_lower.contains("too many requests") ||
        error_lower.contains("rate limit") ||
        error_lower.contains("timeout") ||
        error_lower.contains("connection") ||
        error_lower.contains("network") ||
        error_lower.contains("unavailable")
    {
        return false;
    }

    // Default to caching as failed for unknown errors
    true
}

/// Batch fetch token decimals from a specific RPC endpoint using get_multiple_accounts
async fn batch_fetch_decimals_from_rpc(
    rpc_url: &str,
    mint_pubkeys: &[Pubkey]
) -> Result<Vec<(Pubkey, Result<u8, String>)>, String> {
    let client = RpcClient::new(rpc_url);

    // Split into chunks of 100 (Solana RPC limit)
    const MAX_ACCOUNTS_PER_CALL: usize = 100;
    let mut all_results = Vec::new();

    for chunk in mint_pubkeys.chunks(MAX_ACCOUNTS_PER_CALL) {
        // Get multiple accounts in one RPC call
        let accounts = client
            .get_multiple_accounts(chunk)
            .map_err(|e| format!("Failed to get multiple accounts: {}", e))?;

        // Process each account result
        for (i, account_option) in accounts.iter().enumerate() {
            let mint_pubkey = chunk[i];

            let decimals_result = match account_option {
                Some(account) => {
                    // Check if account exists and has data
                    if account.data.is_empty() {
                        Err("Account not found or empty".to_string())
                    } else if
                        account.owner != spl_token::id() &&
                        account.owner != spl_token_2022::id()
                    {
                        Err(format!("Account owner is not SPL Token program: {}", account.owner))
                    } else {
                        // Parse mint data based on program type
                        if account.owner == spl_token::id() {
                            // Standard SPL Token
                            parse_spl_token_mint(&account.data)
                        } else {
                            // SPL Token-2022 (Token Extensions)
                            parse_token_2022_mint(&account.data)
                        }
                    }
                }
                None => Err("Account not found".to_string()),
            };

            all_results.push((mint_pubkey, decimals_result));
        }

        // Small delay between batches to avoid rate limiting
        if mint_pubkeys.len() > MAX_ACCOUNTS_PER_CALL {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    Ok(all_results)
}

/// Parse SPL Token mint data to extract decimals
fn parse_spl_token_mint(data: &[u8]) -> Result<u8, String> {
    if data.len() < Mint::LEN {
        return Err(format!("Invalid mint data length: expected {}, got {}", Mint::LEN, data.len()));
    }

    // Parse using SPL Token library
    let mint = Mint::unpack(data).map_err(|e| format!("Failed to unpack SPL Token mint: {}", e))?;

    Ok(mint.decimals)
}

/// Parse SPL Token-2022 mint data to extract decimals
fn parse_token_2022_mint(data: &[u8]) -> Result<u8, String> {
    // For Token-2022, the decimals are at the same position as in standard SPL Token
    // The first 9 bytes are the same structure for both token programs
    if data.len() < 44 {
        return Err(
            format!("Invalid Token-2022 mint data length: expected at least 44, got {}", data.len())
        );
    }

    // Decimals are at offset 44 in both SPL Token and SPL Token-2022
    Ok(data[44])
}

/// Batch fetch decimals for multiple tokens using efficient batch RPC calls
pub async fn batch_fetch_token_decimals(mints: &[String]) -> Vec<(String, Result<u8, String>)> {
    if mints.is_empty() {
        return Vec::new();
    }

    // Convert mint strings to Pubkeys, filtering out invalid ones
    let mut valid_mints = Vec::new();
    let mut invalid_results = Vec::new();

    for mint in mints {
        match Pubkey::from_str(mint) {
            Ok(pubkey) => valid_mints.push((mint.clone(), pubkey)),
            Err(e) =>
                invalid_results.push((mint.clone(), Err(format!("Invalid mint address: {}", e)))),
        }
    }

    if valid_mints.is_empty() {
        return invalid_results;
    }

    // Check which tokens are not in cache and not previously failed
    let mut uncached_mints = Vec::new();
    let mut cached_results = Vec::new();

    if let Ok(cache) = DECIMAL_CACHE.lock() {
        for (mint_str, pubkey) in &valid_mints {
            if let Some(&decimals) = cache.get(mint_str) {
                cached_results.push((mint_str.clone(), Ok(decimals)));
            } else if is_token_already_failed(mint_str) {
                // Token already failed, skip but report as failed
                if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
                    if let Some(error) = failed_cache.get(mint_str) {
                        cached_results.push((mint_str.clone(), Err(error.clone())));
                        log(
                            LogTag::Decimals,
                            "SKIP_FAILED",
                            &format!("Skipping previously failed token {}", mint_str)
                        );
                    }
                }
            } else {
                uncached_mints.push((mint_str.clone(), *pubkey));
            }
        }
    } else {
        // Filter out already failed tokens even if main cache is locked
        for (mint_str, pubkey) in &valid_mints {
            if !is_token_already_failed(mint_str) {
                uncached_mints.push((mint_str.clone(), *pubkey));
            } else {
                if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
                    if let Some(error) = failed_cache.get(mint_str) {
                        cached_results.push((mint_str.clone(), Err(error.clone())));
                    }
                }
            }
        }
    }

    // Only log and fetch if there are uncached tokens
    if uncached_mints.is_empty() {
        // Return all cached results in original order
        let mut all_results = Vec::new();
        for (mint_str, _) in &valid_mints {
            if let Some(cached_result) = cached_results.iter().find(|(m, _)| m == mint_str) {
                all_results.push(cached_result.clone());
            }
        }
        all_results.extend(invalid_results);
        return all_results;
    }

    log(
        LogTag::Decimals,
        "FETCH",
        &format!(
            "Fetching decimals for {} new tokens (cached: {})",
            uncached_mints.len(),
            cached_results.len()
        )
    );

    // Use only mainnet RPC for decimal fetching (no premium, no fallbacks unless rate limited)
    let mainnet_rpc = "https://api.mainnet-beta.solana.com";
    let fallback_rpcs = vec!["https://rpc.ankr.com/solana", "https://solana-api.projectserum.com"];

    let mut fetch_results = Vec::new();
    let mut remaining_mints = uncached_mints.clone();
    let mut new_cache_entries = HashMap::new();

    // Start with mainnet, use fallbacks only for rate limits/network issues
    let mut rpc_urls = vec![mainnet_rpc.to_string()];
    rpc_urls.extend(fallback_rpcs.iter().map(|s| s.to_string()));

    // Try each RPC endpoint until we get results
    for rpc_url in &rpc_urls {
        if remaining_mints.is_empty() {
            break;
        }

        let remaining_pubkeys: Vec<Pubkey> = remaining_mints
            .iter()
            .map(|(_, pubkey)| *pubkey)
            .collect();

        match batch_fetch_decimals_from_rpc(rpc_url, &remaining_pubkeys).await {
            Ok(batch_results) => {
                let mut successful_indices = Vec::new();

                for (i, (_pubkey, decimals_result)) in batch_results.iter().enumerate() {
                    let mint_str = &remaining_mints[i].0;

                    match decimals_result {
                        Ok(decimals) => {
                            new_cache_entries.insert(mint_str.clone(), *decimals);
                            fetch_results.push((mint_str.clone(), Ok(*decimals)));
                            successful_indices.push(i);
                        }
                        Err(e) => {
                            // Check if this is a real error (account not found) vs rate limit
                            if should_cache_as_failed(e) {
                                // Real error - cache failure and don't retry on other RPCs
                                cache_failed_token(mint_str, e);
                                fetch_results.push((mint_str.clone(), Err(e.clone())));

                                log(
                                    LogTag::Decimals,
                                    "REAL_ERROR",
                                    &format!("Token {} failed with real error: {}", mint_str, e)
                                );
                            } else {
                                // Rate limit / network issue - will retry on next RPC
                                log(
                                    LogTag::Decimals,
                                    "RETRY_ERROR",
                                    &format!(
                                        "Token {} failed with retryable error from {}: {}",
                                        mint_str,
                                        rpc_url,
                                        e
                                    )
                                );
                            }
                        }
                    }
                }

                // Collect indices to remove (successful + real failures)
                let mut indices_to_remove = Vec::new();
                for (i, (_pubkey, decimals_result)) in batch_results.iter().enumerate() {
                    let mint_str = &remaining_mints[i].0;
                    match decimals_result {
                        Ok(_) => indices_to_remove.push(i), // Success
                        Err(e) if should_cache_as_failed(e) => indices_to_remove.push(i), // Real failure, don't retry
                        Err(_) => {} // Retryable error, keep in remaining list
                    }
                }

                // Remove processed mints from remaining list (in reverse order to maintain indices)
                indices_to_remove.sort_by(|a, b| b.cmp(a));
                for &index in &indices_to_remove {
                    remaining_mints.remove(index);
                }

                if !successful_indices.is_empty() {
                    log(
                        LogTag::Decimals,
                        "SUCCESS",
                        &format!(
                            "Fetched {} new decimal entries from {}",
                            successful_indices.len(),
                            rpc_url
                        )
                    );
                }
            }
            Err(e) => {
                log(LogTag::Decimals, "RPC_ERROR", &format!("RPC {} failed: {}", rpc_url, e));

                // If this is a connection/rate limit error, continue to next RPC
                // If it's a systemic error, fail all remaining tokens
                if should_cache_as_failed(&e) {
                    // Systemic error, cache all remaining as failed
                    for (mint_str, _) in remaining_mints.drain(..) {
                        cache_failed_token(&mint_str, &e);
                        fetch_results.push((mint_str, Err(e.clone())));
                    }
                    break;
                }
            }
        }
    }

    // Handle any remaining mints that weren't processed (timeout/network issues across all RPCs)
    for (mint_str, _) in remaining_mints {
        let error_msg = "All RPC endpoints failed";
        log(
            LogTag::Decimals,
            "TIMEOUT",
            &format!("All RPC endpoints failed for token {}, caching as failed", mint_str)
        );

        cache_failed_token(&mint_str, error_msg);
        fetch_results.push((mint_str, Err(error_msg.to_string())));
    }

    // Update cache and save to disk if we have new entries
    if !new_cache_entries.is_empty() {
        if let Ok(mut cache) = DECIMAL_CACHE.lock() {
            let old_size = cache.len();
            cache.extend(new_cache_entries);
            let new_size = cache.len();

            // Get current failed cache for saving
            let failed_cache = if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
                failed_cache.clone()
            } else {
                HashMap::new()
            };

            // Save to disk
            save_cache_to_disk(&cache, &failed_cache);

            log(
                LogTag::Decimals,
                "CACHE_UPDATE",
                &format!(
                    "Updated decimal cache: {} → {} entries (+{})",
                    old_size,
                    new_size,
                    new_size - old_size
                )
            );
        }
    }

    // Combine cached and fetched results in original order
    let mut all_results = Vec::new();
    for (mint_str, _) in &valid_mints {
        // Check if this mint was cached
        if let Some(cached_result) = cached_results.iter().find(|(m, _)| m == mint_str) {
            all_results.push(cached_result.clone());
        } else {
            // Find in fetch results
            if let Some(fetch_result) = fetch_results.iter().find(|(m, _)| m == mint_str) {
                all_results.push(fetch_result.clone());
            } else {
                // This shouldn't happen, but handle gracefully
                all_results.push((mint_str.clone(), Err("Failed to fetch decimals".to_string())));
            }
        }
    }

    // Add back the invalid mint results
    all_results.extend(invalid_results);

    all_results
}

/// Get decimals from cache only (no RPC call)
pub fn get_cached_decimals(mint: &str) -> Option<u8> {
    DECIMAL_CACHE.lock().ok()?.get(mint).copied()
}

/// Batch get token decimals from blockchain with caching - efficient for multiple tokens
pub async fn get_multiple_token_decimals_from_chain(
    mints: &[String]
) -> Vec<(String, Result<u8, String>)> {
    if mints.is_empty() {
        return Vec::new();
    }

    // Check cache for all mints first
    let mut cached_results = Vec::new();
    let mut uncached_mints = Vec::new();

    if let Ok(cache) = DECIMAL_CACHE.lock() {
        for mint in mints {
            if let Some(&decimals) = cache.get(mint) {
                cached_results.push((mint.clone(), Ok(decimals)));
            } else {
                uncached_mints.push(mint.clone());
            }
        }
    } else {
        uncached_mints = mints.to_vec();
    }

    // If some mints are not cached, fetch them in batch
    let mut batch_results = Vec::new();
    if !uncached_mints.is_empty() {
        batch_results = batch_fetch_token_decimals(&uncached_mints).await;
    }

    // Combine cached and fetched results in original order
    let mut all_results = Vec::new();

    for mint in mints {
        // Check if this mint was cached
        if let Some(cached_result) = cached_results.iter().find(|(m, _)| m == mint) {
            all_results.push(cached_result.clone());
        } else {
            // Find in batch results
            if let Some(batch_result) = batch_results.iter().find(|(m, _)| m == mint) {
                all_results.push(batch_result.clone());
            } else {
                // This shouldn't happen, but handle gracefully
                all_results.push((mint.clone(), Err("Failed to fetch decimals".to_string())));
            }
        }
    }

    all_results
}

/// Clear decimals cache
pub fn clear_decimals_cache() {
    if let Ok(mut cache) = DECIMAL_CACHE.lock() {
        cache.clear();
        let failed_cache = if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
            failed_cache.clone()
        } else {
            HashMap::new()
        };
        save_cache_to_disk(&cache, &failed_cache);
        log(LogTag::Decimals, "CACHE", "Cleared decimal cache and saved to disk");
    }
}

/// Get cache statistics
pub fn get_cache_stats() -> (usize, usize) {
    if let Ok(cache) = DECIMAL_CACHE.lock() {
        let size = cache.len();
        let capacity = cache.capacity();
        (size, capacity)
    } else {
        (0, 0)
    }
}

/// Force save current cache to disk (useful for shutdown)
pub fn save_decimal_cache() {
    if let Ok(cache) = DECIMAL_CACHE.lock() {
        let failed_cache = if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
            failed_cache.clone()
        } else {
            HashMap::new()
        };
        save_cache_to_disk(&cache, &failed_cache);
    }
}
