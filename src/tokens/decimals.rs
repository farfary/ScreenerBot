/// Token decimals fetching from Solana blockchain
use crate::logger::{ log, LogTag };
use crate::global::{ is_debug_decimals_enabled, DECIMAL_CACHE as DECIMAL_CACHE_FILE };
use crate::tokens::is_system_or_stable_token;
use crate::rpc::get_rpc_client;
use crate::utils::safe_truncate;
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

const CACHE_FILE_NAME: &str = DECIMAL_CACHE_FILE;

// =============================================================================
// DECIMAL CONSTANTS
// =============================================================================

/// SOL token decimals constant - ALWAYS use this instead of hardcoding 9
pub const SOL_DECIMALS: u8 = 9;

/// SOL token lamports per SOL constant - ALWAYS use this instead of hardcoding 1_000_000_000
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

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
                        if is_debug_decimals_enabled() {
                            log(
                                LogTag::Decimals,
                                "CACHE_LOAD",
                                &format!(
                                    "Loaded {} decimal entries and {} failed entries from cache file",
                                    cache_data.decimals.len(),
                                    cache_data.failed_tokens.len()
                                )
                            );
                        }
                        return (cache_data.decimals, cache_data.failed_tokens);
                    }
                    Err(e) => {
                        // Try to parse old format (without failed_tokens)
                        if
                            let Ok(old_cache) = serde_json::from_str::<HashMap<String, u8>>(
                                &content
                            )
                        {
                            if is_debug_decimals_enabled() {
                                log(
                                    LogTag::Decimals,
                                    "CACHE_MIGRATE",
                                    &format!(
                                        "Migrated {} decimal entries from old format cache file",
                                        old_cache.len()
                                    )
                                );
                            }
                            return (old_cache, HashMap::new());
                        }

                        if is_debug_decimals_enabled() {
                            log(
                                LogTag::Decimals,
                                "CACHE_ERROR",
                                &format!("Failed to parse decimal cache file: {}", e)
                            );
                        }
                    }
                }
            }
            Err(e) => {
                if is_debug_decimals_enabled() {
                    log(
                        LogTag::Decimals,
                        "CACHE_READ_ERROR",
                        &format!("Failed to read decimal cache file: {}", e)
                    );
                }
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
                if is_debug_decimals_enabled() {
                    log(
                        LogTag::Decimals,
                        "SAVE_ERROR",
                        &format!("Failed to save decimal cache to disk: {}", e)
                    );
                }
            }
        }
        Err(e) => {
            if is_debug_decimals_enabled() {
                log(
                    LogTag::Decimals,
                    "SERIALIZE_ERROR",
                    &format!("Failed to serialize decimal cache: {}", e)
                );
            }
        }
    }
}

/// Get token decimals from Solana blockchain with caching
pub async fn get_token_decimals_from_chain(mint: &str) -> Result<u8, String> {
    // CRITICAL: SOL (native token) always has 9 decimals
    if mint == "So11111111111111111111111111111111111111112" {
        return Ok(9);
    }

    // Skip system/stable tokens that shouldn't be processed
    if is_system_or_stable_token(mint) {
        if is_debug_decimals_enabled() {
            log(
                LogTag::Decimals,
                "SKIP_SYSTEM",
                &format!("Skipping system/stable token: {}", mint)
            );
        }
        return Err("System or stable token excluded from processing".to_string());
    }

    // Check successful decimals cache first
    if let Ok(cache) = DECIMAL_CACHE.lock() {
        if let Some(&decimals) = cache.get(mint) {
            return Ok(decimals);
        }
    }

    // Check failed decimals cache - but allow retries for network/temporary errors
    if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        if let Some(error) = failed_cache.get(mint) {
            // Only skip if this was a real blockchain error, not a network issue
            if should_cache_as_failed(error) {
                if is_debug_decimals_enabled() {
                    log(
                        LogTag::Decimals,
                        "CACHED_FAIL",
                        &format!("Skipping previously failed token {}: {}", mint, error)
                    );
                }
                return Err(error.clone());
            } else {
                // Network/temporary error - allow retry but log it
                if is_debug_decimals_enabled() {
                    log(
                        LogTag::Decimals,
                        "RETRY_CACHED",
                        &format!(
                            "Retrying previously failed token {} (network error): {}",
                            mint,
                            error
                        )
                    );
                }
                // Continue to fetch - don't return early
            }
        }
    }

    // Use the batch function for single token (more efficient than separate implementation)
    let results = batch_fetch_token_decimals(&[mint.to_string()]).await;

    if let Some((_, result)) = results.first() {
        // If successful and was previously failed, the batch function already cleaned it up
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

/// Check if a token failed with a permanent error (not retryable)
fn is_token_failed_permanently(mint: &str) -> bool {
    if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        if let Some(error) = failed_cache.get(mint) {
            // Only permanently failed if it's a real blockchain error
            return should_cache_as_failed(error);
        }
    }
    false
}

/// Add a token to the failed cache
fn cache_failed_token(mint: &str, error: &str) {
    if let Ok(mut failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        failed_cache.insert(mint.to_string(), error.to_string());
        if is_debug_decimals_enabled() {
            log(
                LogTag::Decimals,
                "CACHE_FAIL",
                &format!("Cached failed lookup for {}: {}", mint, error)
            );
        }
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
        error_lower.contains("empty") ||
        error_lower.contains("account owner is not spl token program")
    {
        return true;
    }

    // Rate limiting and temporary issues - retry with different RPC
    if
        error_lower.contains("429") ||
        error_lower.contains("too many requests") ||
        error_lower.contains("rate limit") ||
        error_lower.contains("rate limited") ||
        error_lower.contains("timeout") ||
        error_lower.contains("connection") ||
        error_lower.contains("network") ||
        error_lower.contains("unavailable") ||
        error_lower.contains("error sending request") ||
        error_lower.contains("request failed") ||
        error_lower.contains("connection refused") ||
        error_lower.contains("connection reset") ||
        error_lower.contains("timed out") ||
        error_lower.contains("dns") ||
        error_lower.contains("ssl") ||
        error_lower.contains("tls") ||
        error_lower.contains("failed to get multiple accounts") ||
        error_lower.contains("batch fetch failed")
    {
        return false;
    }

    // Default to caching as failed for unknown errors
    true
}

/// Batch fetch token decimals using the centralized RPC client with automatic fallback
async fn batch_fetch_decimals_with_fallback(
    mint_pubkeys: &[Pubkey]
) -> Result<Vec<(Pubkey, Result<u8, String>)>, String> {
    let rpc_client = get_rpc_client();

    // Split into chunks of 100 (Solana RPC limit)
    const MAX_ACCOUNTS_PER_CALL: usize = 100;
    let mut all_results = Vec::new();

    for chunk in mint_pubkeys.chunks(MAX_ACCOUNTS_PER_CALL) {
        // Get multiple accounts in one RPC call using centralized client
        let accounts = rpc_client
            .get_multiple_accounts(chunk).await
            .map_err(|e| {
                // Improve error categorization
                if e.contains("429") || e.contains("rate limit") || e.contains("Too Many Requests") {
                    format!("Rate limited: {}", e)
                } else if e.contains("error sending request") || e.contains("connection") {
                    format!("Network error: {}", e)
                } else {
                    format!("Failed to get multiple accounts: {}", e)
                }
            })?;

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

        // Progressive delay between batches to avoid rate limiting
        if mint_pubkeys.len() > MAX_ACCOUNTS_PER_CALL {
            let delay_ms = if all_results.len() > 200 { 300 } else { 150 }; // Longer delay for large batches
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
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

    // Convert mint strings to Pubkeys, filtering out invalid ones and handling SOL
    let mut valid_mints = Vec::new();
    let mut invalid_results = Vec::new();
    let mut sol_results = Vec::new();

    for mint in mints {
        // CRITICAL: Handle SOL (native token) first
        if mint == "So11111111111111111111111111111111111111112" {
            sol_results.push((mint.clone(), Ok(9u8)));
            continue;
        }

        // Skip system/stable tokens that shouldn't be in watch lists
        if is_system_or_stable_token(mint) {
            if is_debug_decimals_enabled() {
                log(
                    LogTag::Decimals,
                    "SKIP_SYSTEM",
                    &format!("Skipping system/stable token: {}", mint)
                );
            }
            continue;
        }

        match Pubkey::from_str(mint) {
            Ok(pubkey) => valid_mints.push((mint.clone(), pubkey)),
            Err(e) => {
                if is_debug_decimals_enabled() {
                    log(
                        LogTag::Decimals,
                        "INVALID_MINT",
                        &format!("Invalid mint address {}: {}", mint, e)
                    );
                }
                invalid_results.push((mint.clone(), Err(format!("Invalid mint address: {}", e))));
            }
        }
    }

    if valid_mints.is_empty() {
        // Return SOL results + invalid results if no other valid mints
        let mut all_results = sol_results;
        all_results.extend(invalid_results);
        return all_results;
    }

    // Check which tokens are not in cache and not previously failed
    let mut uncached_mints = Vec::new();
    let mut cached_results = Vec::new();

    if let Ok(cache) = DECIMAL_CACHE.lock() {
        for (mint_str, pubkey) in &valid_mints {
            if let Some(&decimals) = cache.get(mint_str) {
                cached_results.push((mint_str.clone(), Ok(decimals)));
            } else if is_token_failed_permanently(mint_str) {
                // Token failed with permanent error (not network), skip
                if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
                    if let Some(error) = failed_cache.get(mint_str) {
                        cached_results.push((mint_str.clone(), Err(error.clone())));
                        if is_debug_decimals_enabled() {
                            log(
                                LogTag::Decimals,
                                "SKIP_FAILED",
                                &format!("Skipping permanently failed token {}", mint_str)
                            );
                        }
                    }
                }
            } else {
                // Either not in failed cache, or failed with retryable error
                if is_token_already_failed(mint_str) && is_debug_decimals_enabled() {
                    if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
                        if let Some(error) = failed_cache.get(mint_str) {
                            log(
                                LogTag::Decimals,
                                "RETRY_BATCH",
                                &format!("Retrying token {} (network error): {}", mint_str, error)
                            );
                        }
                    }
                }
                uncached_mints.push((mint_str.clone(), *pubkey));
            }
        }
    } else {
        // Filter out permanently failed tokens even if main cache is locked
        for (mint_str, pubkey) in &valid_mints {
            if !is_token_failed_permanently(mint_str) {
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

    // Only log and fetch if there are uncached tokens and debug is enabled
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

    // Only log batch operations if debug is enabled and significant batch size
    if is_debug_decimals_enabled() && uncached_mints.len() > 3 {
        log(
            LogTag::Decimals,
            "BATCH_FETCH",
            &format!(
                "Fetching decimals for {} tokens (batch operation, cached: {})",
                uncached_mints.len(),
                cached_results.len()
            )
        );
    }

    // Use centralized RPC client with automatic fallback handling
    let mut fetch_results = Vec::new();
    let mut new_cache_entries = HashMap::new();

    if !uncached_mints.is_empty() {
        let uncached_pubkeys: Vec<Pubkey> = uncached_mints
            .iter()
            .map(|(_, pubkey)| *pubkey)
            .collect();

        match batch_fetch_decimals_with_fallback(&uncached_pubkeys).await {
            Ok(batch_results) => {
                for (i, (_pubkey, decimals_result)) in batch_results.iter().enumerate() {
                    let mint_str = &uncached_mints[i].0;

                    match decimals_result {
                        Ok(decimals) => {
                            new_cache_entries.insert(mint_str.clone(), *decimals);
                            fetch_results.push((mint_str.clone(), Ok(*decimals)));

                            // Remove from failed cache if it was previously failed
                            if is_token_already_failed(mint_str) {
                                if let Ok(mut failed_cache) = FAILED_DECIMALS_CACHE.lock() {
                                    if let Some(old_error) = failed_cache.remove(mint_str) {
                                        if is_debug_decimals_enabled() {
                                            log(
                                                LogTag::Decimals,
                                                "RETRY_SUCCESS",
                                                &format!(
                                                    "Token {} succeeded on retry, removed from failed cache (was: {})",
                                                    mint_str,
                                                    old_error
                                                )
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // Cache failure for permanent errors
                            if should_cache_as_failed(e) {
                                cache_failed_token(mint_str, e);
                            }
                            fetch_results.push((mint_str.clone(), Err(e.clone())));

                            if is_debug_decimals_enabled() {
                                log(
                                    LogTag::Decimals,
                                    "FETCH_ERROR",
                                    &format!("Token {} failed: {}", mint_str, e)
                                );
                            }
                        }
                    }
                }

                if is_debug_decimals_enabled() && !fetch_results.is_empty() {
                    let success_count = fetch_results
                        .iter()
                        .filter(|(_, r)| r.is_ok())
                        .count();
                    log(
                        LogTag::Decimals,
                        "BATCH_SUCCESS",
                        &format!(
                            "Successfully fetched decimals for {}/{} tokens using centralized RPC client",
                            success_count,
                            fetch_results.len()
                        )
                    );
                }
            }
            Err(e) => {
                // If entire batch fails, mark all as failed with the batch error
                let error_msg = format!("Batch fetch failed: {}", e);
                let should_cache = should_cache_as_failed(&error_msg);
                
                if is_debug_decimals_enabled() {
                    log(
                        LogTag::Decimals,
                        "BATCH_ERROR",
                        &format!(
                            "Batch fetch failed for {} tokens: {} (caching: {})", 
                            uncached_mints.len(), 
                            e,
                            should_cache
                        )
                    );
                }
                
                for (mint_str, _) in &uncached_mints {
                    if should_cache {
                        cache_failed_token(mint_str, &error_msg);
                    } else {
                        // For network errors, just log but don't cache permanently
                        if is_debug_decimals_enabled() {
                            log(
                                LogTag::Decimals,
                                "BATCH_RETRY_LATER",
                                &format!("Network error for {}, will retry later: {}", mint_str, e)
                            );
                        }
                    }
                    fetch_results.push((mint_str.clone(), Err(error_msg.clone())));
                }
            }
        }
    }

    // Update cache and save to disk if we have new entries or removed failed entries
    let mut cache_updated = false;

    if !new_cache_entries.is_empty() {
        if let Ok(mut cache) = DECIMAL_CACHE.lock() {
            let old_size = cache.len();
            cache.extend(new_cache_entries.clone());
            let new_size = cache.len();
            cache_updated = true;

            // Only log significant cache updates or in debug mode
            if is_debug_decimals_enabled() || new_cache_entries.len() > 5 {
                log(
                    LogTag::Decimals,
                    "CACHE_UPDATE",
                    &format!(
                        "Updated decimal cache: {} → {} entries (+{} new: {})",
                        old_size,
                        new_size,
                        new_cache_entries.len(),
                        new_cache_entries.keys().take(3).cloned().collect::<Vec<_>>().join(", ")
                    )
                );
            }
        }
    }

    // Always save to disk if cache was updated (includes failed cache removals)
    if cache_updated {
        // Get current caches for saving
        let success_cache = if let Ok(cache) = DECIMAL_CACHE.lock() {
            cache.clone()
        } else {
            HashMap::new()
        };

        let failed_cache = if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
            failed_cache.clone()
        } else {
            HashMap::new()
        };

        // Save both caches to disk
        save_cache_to_disk(&success_cache, &failed_cache);
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

    // Add back the SOL results and invalid mint results
    all_results.extend(sol_results);
    all_results.extend(invalid_results);

    all_results
}

/// Get decimals from cache only (no RPC call)
pub fn get_cached_decimals(mint: &str) -> Option<u8> {
    // CRITICAL: SOL (native token) always has 9 decimals
    if mint == "So11111111111111111111111111111111111111112" {
        return Some(9);
    }

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
        if is_debug_decimals_enabled() {
            log(LogTag::Decimals, "CACHE_CLEAR", "Cleared decimal cache and saved to disk");
        }
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

/// Clean up temporary/network errors from failed cache, keeping only permanent blockchain errors
pub fn cleanup_retryable_failed_cache() {
    if let Ok(mut failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        let original_size = failed_cache.len();

        // Keep only permanent errors (blockchain state errors)
        failed_cache.retain(|_mint, error| should_cache_as_failed(error));

        let cleaned_size = failed_cache.len();
        let removed_count = original_size - cleaned_size;

        if removed_count > 0 {
            // Save cleaned cache to disk
            let success_cache = if let Ok(cache) = DECIMAL_CACHE.lock() {
                cache.clone()
            } else {
                HashMap::new()
            };
            save_cache_to_disk(&success_cache, &failed_cache);

            if is_debug_decimals_enabled() {
                log(
                    LogTag::Decimals,
                    "CACHE_CLEANUP",
                    &format!(
                        "Cleaned failed cache: removed {} retryable errors, kept {} permanent errors",
                        removed_count,
                        cleaned_size
                    )
                );
            }
        }
    }
}

/// Get failed cache statistics for debugging
pub fn get_failed_cache_stats() -> (usize, Vec<String>) {
    if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        let size = failed_cache.len();
        let sample_errors: Vec<String> = failed_cache
            .iter()
            .take(5)
            .map(|(mint, error)| format!("{}: {}", &mint[..8], error))
            .collect();
        (size, sample_errors)
    } else {
        (0, Vec::new())
    }
}

// =============================================================================
// LAMPORTS CONVERSION UTILITIES
// =============================================================================

/// Convert lamports to SOL using the proper SOL decimals constant
pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / LAMPORTS_PER_SOL as f64
}

/// Convert SOL to lamports using the proper SOL decimals constant
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * LAMPORTS_PER_SOL as f64) as u64
}

/// Convert token amount to UI amount using provided decimals
pub fn raw_to_ui_amount(raw_amount: u64, decimals: u8) -> f64 {
    raw_amount as f64 / 10f64.powi(decimals as i32)
}

/// Convert UI amount to raw token amount using provided decimals
pub fn ui_to_raw_amount(ui_amount: f64, decimals: u8) -> u64 {
    (ui_amount * 10f64.powi(decimals as i32)) as u64
}
